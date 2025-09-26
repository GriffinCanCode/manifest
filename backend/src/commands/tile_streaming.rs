//! Tile streaming commands for frontend-backend communication
//! 
//! Handles efficient streaming of tile data from the backend ECS world
//! to the frontend rendering system via Tauri IPC.

use serde::{Deserialize, Serialize};
use tauri::State;
use tracing::{debug, instrument, warn};

use crate::{
    core::zig_ffi::HexCoord,
    world::tiles::{
        components::core::{Tile, TerrainType},
        chunks::{TileId, ChunkCoord},
        hierarchy::types::HierarchicalTile,
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
    pub next_offset: Option<usize>,
}

/// Frontend tile representation
#[derive(Debug, Clone, Serialize)]
pub struct GameTile {
    pub id: u32,
    pub hex: HexCoord,
    pub terrain: u8, // TerrainType as u8
    pub elevation: f32,
    pub world_x: f32,
    pub world_z: f32,
    pub biome: u8,
    pub resource_mask: u32,
    pub chunk_id: [i32; 2],
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

    let mut streamed_tiles = Vec::new();
    let mut instance_data = Vec::new();

    // Query tiles within radius
    for q_offset in -hex_radius..=hex_radius {
        for r_offset in -hex_radius..=hex_radius {
            let s_offset = -q_offset - r_offset;
            if s_offset.abs() <= hex_radius {
                let hex = HexCoord {
                    q: camera_hex.q + q_offset,
                    r: camera_hex.r + r_offset,
                };

                // Try to get tile from world
                if let Some(tile_data) = get_tile_at_hex(&state, hex).await {
                    if streamed_tiles.len() < request.max_tiles {
                        let game_tile = convert_to_game_tile(tile_data, hex);
                        let instance = create_instance_data(&game_tile, request.generation);
                        
                        streamed_tiles.push(game_tile);
                        instance_data.push(instance);
                    }
                }
            }
        }
    }

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
fn pixel_to_hex(x: f32, z: f32) -> HexCoord {
    let hex_size = 1.0;
    let q = (2.0 / 3.0 * x) / hex_size;
    let r = (-1.0 / 3.0 * x + (3.0_f32).sqrt() / 3.0 * z) / hex_size;
    
    // Round to nearest hex
    let q_round = q.round() as i32;
    let r_round = r.round() as i32;
    
    HexCoord {
        q: q_round,
        r: r_round,
    }
}

/// Get tile at specific hex coordinate from game state
async fn get_tile_at_hex(state: &AppState, hex: HexCoord) -> Option<Tile> {
    // TODO: Implement proper tile lookup from ECS world
    // This would query the tile component manager
    
    // For now, generate procedural data
    Some(Tile {
        id: TileId((hex.q * 1000 + hex.r) as u32),
        hex,
        chunk: ChunkCoord { x: hex.q / 32, y: hex.r / 32 },
        local_x: (hex.q % 32) as u8,
        local_y: (hex.r % 32) as u8,
        terrain_type: TerrainType::Grassland, // Default for now
        elevation: 0.0,
    })
}

/// Get tile by ID from game state
async fn get_tile_by_id(state: &AppState, tile_id: TileId) -> Option<Tile> {
    // TODO: Implement proper tile lookup by ID
    None
}

/// Convert internal tile to frontend game tile
fn convert_to_game_tile(tile: Tile, hex: HexCoord) -> GameTile {
    let world_pos = hex_to_pixel(hex);
    
    GameTile {
        id: tile.id.0,
        hex: hex,
        terrain: tile.terrain_type as u8,
        elevation: tile.elevation,
        world_x: world_pos.0,
        world_z: world_pos.1,
        biome: 1, // TODO: Get actual biome data
        resource_mask: 0, // TODO: Get actual resource data
        chunk_id: [tile.chunk.x, tile.chunk.y],
    }
}

/// Convert hex coordinates to pixel coordinates
fn hex_to_pixel(hex: HexCoord) -> (f32, f32) {
    let hex_size = 1.0;
    let x = hex_size * (3.0 / 2.0 * hex.q as f32);
    let z = hex_size * ((3.0_f32).sqrt() / 2.0 * hex.q as f32 + (3.0_f32).sqrt() * hex.r as f32);
    (x, z)
}

/// Create instance data for rendering
fn create_instance_data(tile: &GameTile, generation: u64) -> TileInstanceData {
    // Determine color based on terrain type
    let color = match tile.terrain {
        0 => [0.12, 0.25, 0.69], // Ocean - blue
        1 => [0.13, 0.77, 0.37], // Grassland - green
        2 => [0.52, 0.8, 0.09],  // Plains - light green
        3 => [0.92, 0.70, 0.03], // Desert - yellow
        4 => [0.39, 0.45, 0.55], // Tundra - gray
        5 => [0.95, 0.96, 0.97], // Snow - white
        6 => [0.09, 0.40, 0.20], // Forest - dark green
        7 => [0.08, 0.33, 0.18], // Jungle - very dark green
        8 => [0.64, 0.64, 0.64], // Hills - gray
        9 => [0.32, 0.32, 0.32], // Mountain - dark gray
        _ => [0.5, 0.5, 0.5],    // Default - medium gray
    };

    TileInstanceData {
        tile_id: tile.id,
        position: [tile.world_x, tile.elevation * 0.5, tile.world_z],
        color,
        height: tile.elevation,
        biome: tile.biome as f32,
        resource_mask: tile.resource_mask as f32,
        lod_level: 0.0, // TODO: Calculate based on distance
        flags: 0.0,
        last_updated: generation,
    }
}
