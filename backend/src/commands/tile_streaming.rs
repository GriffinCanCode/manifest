//! Tile streaming commands for frontend-backend communication
//! 
//! Handles efficient streaming of tile data from the backend ECS world
//! to the frontend rendering system via Tauri IPC.

use serde::{Deserialize, Serialize};
use tauri::State;
use tracing::{debug, instrument};

use crate::{
    core::zig_ffi::HexCoord,
    utils::lod::{calculate_lod_level, should_render_at_lod, LODLevel},
    world::tiles::{
        components::core::{Tile, TerrainType},
        chunks::{TileId},
    },
};

use super::AppState;

/// Request for streaming tiles based on camera position
#[derive(Debug, Clone, Deserialize)]
pub struct TileStreamingRequest {
    pub camera_position: [f32; 3],
    pub view_radius: f32,
    pub max_tiles: usize,
    pub lod_levels: Vec<u8>,
    pub generation: u64,
}

/// Response containing streamed tile data
#[derive(Debug, Clone, Serialize)]
pub struct TileStreamingResponse {
    pub tiles: Vec<GameTile>,
    pub instance_data: Vec<TileInstanceData>,
    pub generation: u64,
    pub has_more: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<usize>,
}

/// Frontend tile representation
#[derive(Debug, Clone, Serialize)]
pub struct GameTile {
    pub id: u32,
    pub hex: HexCoord,
    pub terrain: String, // TerrainType as string to match frontend enum
    pub elevation: f32,
    #[serde(rename = "worldX")]
    pub world_x: f32,
    #[serde(rename = "worldZ")]
    pub world_z: f32,
    pub biome: Option<u8>,
    #[serde(rename = "resourceMask")]
    pub resource_mask: Option<u32>,
    // Remove chunk_id as frontend doesn't expect it
}

/// Hex coordinate for frontend
// HexCoordinate removed - using HexCoord from core::zig_ffi instead

/// Per-instance data for GPU rendering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileInstanceData {
    pub tile_id: u32,
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub height: f32,
    pub biome: f32,
    pub resource_mask: f32,
    pub lod_level: f32,
    pub flags: f32,
    pub last_updated: u64,
}

/// Batch of tile updates
#[derive(Debug, Clone, Serialize)]
pub struct TileUpdateBatch {
    pub updated_tiles: Vec<u32>,
    pub removed_tiles: Vec<u32>,
    pub timestamp: u64,
}

/// Stream tiles based on camera position and view requirements
#[tauri::command]
#[instrument(skip(game_state))]
pub async fn stream_tiles(
    request: TileStreamingRequest,
    game_state: State<'_, AppState>,
) -> Result<TileStreamingResponse, String> {
    debug!(
        "Streaming tiles for camera position: {:?}, radius: {}, max: {}",
        request.camera_position, request.view_radius, request.max_tiles
    );

    let state = &*game_state;
    
    // Get tiles within view radius
    let camera_hex = pixel_to_hex(request.camera_position[0], request.camera_position[2]);
    let hex_radius = (request.view_radius / 2.0) as i32; // Convert to hex radius

    // DEBUG: Log coordinate conversion and radius info
    eprintln!("🔍 TILE STREAMING DEBUG:");
    eprintln!("  Camera position: [{}, {}, {}]", 
              request.camera_position[0], request.camera_position[1], request.camera_position[2]);
    eprintln!("  Converted to hex: ({}, {})", camera_hex.q, camera_hex.r);
    eprintln!("  View radius: {} -> hex_radius: {}", request.view_radius, hex_radius);

    let mut streamed_tiles = Vec::new();
    let mut instance_data = Vec::new();

    // OPTIMIZED: Batch query tiles in radius instead of individual lookups
    let tiles_in_full_radius = state.tile_manager.get_tiles_in_radius(camera_hex, hex_radius as u32);
    let tiles_requested = tiles_in_full_radius.len();
    let mut tiles_found = 0;
    
    eprintln!("  Tiles found in radius {}: {}", hex_radius, tiles_requested);
    
    // Process batched results with LOD filtering
    for tile_id in tiles_in_full_radius.into_iter() {
        if let Ok(tile) = state.tile_manager.get_component::<Tile>(tile_id) {
            tiles_found += 1;
            let hex = tile.hex;
            
            // Calculate LOD level for this tile
            let lod_level = calculate_lod_level(camera_hex, hex);
            
            // Skip culled tiles completely
            if lod_level == LODLevel::Culled {
                continue;
            }
            
            // Check if this LOD level is requested
            if !should_render_at_lod(camera_hex, hex, &request.lod_levels) {
                continue;
            }
            
            // Stop if we've reached the maximum tile limit
            if streamed_tiles.len() >= request.max_tiles {
                break;
            }
            
            let game_tile = convert_to_game_tile(tile, hex);
            let mut instance = create_instance_data(&game_tile, request.generation);
            
            // Set the calculated LOD level in instance data
            instance.lod_level = lod_level.to_f32();
            
            streamed_tiles.push(game_tile);
            instance_data.push(instance);
        }
    }
    
    // Calculate LOD distribution for debugging
    let mut lod_counts = [0u32; 4]; // [High, Medium, Low, Culled]
    for instance in &instance_data {
        let lod = instance.lod_level as u8;
        if lod < 4 {
            lod_counts[lod as usize] += 1;
        }
    }
    
    debug!("🔍 TILE DEBUG: Requested {} tiles, found {} tiles, streaming {} tiles", 
           tiles_requested, tiles_found, streamed_tiles.len());
    debug!("📊 LOD DISTRIBUTION: High: {}, Medium: {}, Low: {}, Culled: 0", 
           lod_counts[0], lod_counts[1], lod_counts[2]);

    // DEBUG: Final streaming results
    eprintln!("  ✅ FINAL RESULTS:");
    eprintln!("    Tiles processed: {}/{}", tiles_found, tiles_requested);
    eprintln!("    Tiles streamed to frontend: {}", streamed_tiles.len());
    eprintln!("    Instance data entries: {}", instance_data.len());

    let response = TileStreamingResponse {
        tiles: streamed_tiles,
        instance_data,
        generation: state.world.lock().unwrap().world_generation() as u64,
        has_more: false, // TODO: Implement pagination for large datasets
        next_offset: None,
    };

    debug!("Streamed {} tiles", response.tiles.len());
    Ok(response)
}

/// Get specific tile by ID
#[tauri::command]
#[instrument(skip(game_state))]
pub async fn get_tile(
    tile_id: u32,
    game_state: State<'_, AppState>,
) -> Result<Option<GameTile>, String> {
    let state = &*game_state;
    
    if let Some(tile_data) = get_tile_by_id(&state, TileId(tile_id)).await {
        let hex = tile_data.hex;
        let game_tile = convert_to_game_tile(tile_data, hex);
        Ok(Some(game_tile))
    } else {
        Ok(None)
    }
}

/// Get updates for specific tiles
#[tauri::command]
#[instrument(skip(game_state))]
pub async fn get_tile_updates(
    tile_ids: Vec<u32>,
    last_update_time: u64,
    game_state: State<'_, AppState>,
) -> Result<TileUpdateBatch, String> {
    let state = &*game_state;
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    // TODO: Implement actual change detection
    // For now, return empty updates
    let batch = TileUpdateBatch {
        updated_tiles: Vec::new(),
        removed_tiles: Vec::new(),
        timestamp: current_time,
    };

    Ok(batch)
}

// Helper functions

/// Convert pixel coordinates to hex coordinates
/// ALIGNED with frontend HexUtils.pixelToHex() for consistency
fn pixel_to_hex(x: f32, z: f32) -> HexCoord {
    let hex_size = 1.0;
    let sqrt3 = (3.0_f32).sqrt();
    
    // EXACT MATCH to frontend HexUtils.pixelToHex()
    let q = ((sqrt3 / 3.0) * x - (1.0 / 3.0) * z) / hex_size;
    let r = ((2.0 / 3.0) * z) / hex_size;
    
    // Round to nearest hex
    let q_round = q.round() as i32;
    let r_round = r.round() as i32;
    
    HexCoord {
        q: q_round,
        r: r_round,
    }
}

/// Get tile by ID from game state
async fn get_tile_by_id(state: &AppState, tile_id: TileId) -> Option<Tile> {
    state.tile_manager.get_component::<Tile>(tile_id).ok()
}

/// Convert internal tile to frontend game tile
fn convert_to_game_tile(tile: Tile, hex: HexCoord) -> GameTile {
    let world_pos = hex_to_pixel(hex);
    
    // Convert terrain type to frontend string enum
    let terrain_string = match tile.terrain_type {
        TerrainType::Ocean => "ocean",
        TerrainType::Grassland => "grassland",
        TerrainType::Plains => "plains",
        TerrainType::Desert => "desert",
        TerrainType::Tundra => "tundra",
        TerrainType::Snow => "snow",
        TerrainType::Forest => "forest",
        TerrainType::Jungle => "jungle",
        TerrainType::Hills => "hills",
        TerrainType::Mountain => "mountain",
        TerrainType::Mountains => "mountain", // Alias for Mountain
        TerrainType::River => "ocean", // Rivers treated as water
        TerrainType::Coast => "ocean", // Coast treated as water
    }.to_string();
    
    // Map terrain type to biome index (0-9 as expected by shader)
    let biome = match tile.terrain_type {
        TerrainType::Ocean => 0,
        TerrainType::Grassland => 1,
        TerrainType::Plains => 2,
        TerrainType::Desert => 3,
        TerrainType::Tundra => 4,
        TerrainType::Snow => 5,
        TerrainType::Forest => 6,
        TerrainType::Jungle => 7,
        TerrainType::Hills => 8,
        TerrainType::Mountain => 9,
        TerrainType::Mountains => 9, // Same as Mountain for biome purposes
        TerrainType::River => 0,     // Rivers use ocean-like biome
        TerrainType::Coast => 0,     // Coastal areas use ocean-like biome
    };
    
    // Calculate elevation based on terrain type if not set
    let elevation = if tile.elevation != 0.0 {
        tile.elevation
    } else {
        match tile.terrain_type {
            TerrainType::Ocean => -0.2 + (hex.q % 10) as f32 * 0.01,
            TerrainType::Plains | TerrainType::Grassland => 0.1 + (hex.r % 10) as f32 * 0.03,
            TerrainType::Forest => 0.2 + ((hex.q + hex.r) % 10) as f32 * 0.04,
            TerrainType::Hills => 0.4 + (hex.q % 10) as f32 * 0.06,
            TerrainType::Mountain | TerrainType::Mountains => 0.8 + (hex.r % 10) as f32 * 0.04,
            TerrainType::Desert => 0.3 + ((hex.q * hex.r) % 10) as f32 * 0.02,
            TerrainType::Snow => 0.9 + (hex.q % 5) as f32 * 0.02,
            _ => 0.0,
        }
    };
    
    GameTile {
        id: tile.id.0,
        hex: hex,
        terrain: terrain_string,
        elevation,
        world_x: world_pos.0,
        world_z: world_pos.1,
        biome: Some(biome),
        resource_mask: Some(0), // TODO: Get actual resource data
    }
}

/// Convert hex coordinates to pixel coordinates
/// ALIGNED with frontend HexUtils.hexToPixel() for consistency
fn hex_to_pixel(hex: HexCoord) -> (f32, f32) {
    let hex_size = 1.0 * 1.1; // Base hex size with spacing factor
    let sqrt3 = (3.0_f32).sqrt();
    
    // EXACT MATCH to frontend HexUtils.hexToPixel()
    let x = hex_size * (sqrt3 * hex.q as f32 + (sqrt3 / 2.0) * hex.r as f32);
    let z = hex_size * (1.5 * hex.r as f32);
    
    (x, z)
}

/// Create instance data for rendering
fn create_instance_data(tile: &GameTile, generation: u64) -> TileInstanceData {
    // Determine color based on terrain type string
    let color = match tile.terrain.as_str() {
        "ocean" => [0.12, 0.25, 0.69], // Ocean - blue
        "grassland" => [0.13, 0.77, 0.37], // Grassland - green
        "plains" => [0.52, 0.8, 0.09],  // Plains - light green
        "desert" => [0.92, 0.70, 0.03], // Desert - yellow
        "tundra" => [0.39, 0.45, 0.55], // Tundra - gray
        "snow" => [0.95, 0.96, 0.97], // Snow - white
        "forest" => [0.09, 0.40, 0.20], // Forest - dark green
        "jungle" => [0.08, 0.33, 0.18], // Jungle - very dark green
        "hills" => [0.64, 0.64, 0.64], // Hills - gray
        "mountain" => [0.32, 0.32, 0.32], // Mountain - dark gray
        _ => [0.5, 0.5, 0.5],    // Default - medium gray
    };

    TileInstanceData {
        tile_id: tile.id,
        position: [tile.world_x, tile.elevation * 0.5, tile.world_z],
        color,
        height: tile.elevation,
        biome: tile.biome.unwrap_or(0) as f32,
        resource_mask: tile.resource_mask.unwrap_or(0) as f32,
        lod_level: 0.0, // TODO: Calculate based on distance
        flags: 0.0,
        last_updated: generation,
    }
}
