//! Tile components with hecs sparse storage integration
//!
//! Provides efficient sparse component storage for tiles using hecs ECS,
//! integrated with the main bevy_ecs world for optimal performance.

use hecs::{World as HecsWorld, Entity as HecsEntity, Bundle, Component, Query, With, Without};
use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use glam::{Vec2, Vec3, IVec2};
use parking_lot::RwLock;
use std::sync::Arc;
use crate::core::zig_ffi::HexCoord;
use crate::world::tiles::chunks::{TileId, ChunkCoord};
use crate::ecs::components::{Position, Name};
use tracing::{debug, instrument};

/// Core tile component representing a single hex tile
#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub struct Tile {
    /// Unique tile identifier
    pub id: TileId,
    /// Hex coordinate in world space
    pub hex: HexCoord,
    /// Chunk this tile belongs to
    pub chunk: ChunkCoord,
    /// Local coordinates within chunk
    pub local_x: u8,
    pub local_y: u8,
    /// Tile type/terrain identifier
    pub terrain_type: TerrainType,
    /// Base elevation in game units
    pub elevation: f32,
}

impl Tile {
    /// Create new tile with specified parameters
    pub fn new(id: TileId, hex: HexCoord, chunk: ChunkCoord, local_x: u8, local_y: u8, terrain_type: TerrainType) -> Self {
        Self {
            id,
            hex,
            chunk,
            local_x,
            local_y,
            terrain_type,
            elevation: 0.0,
        }
    }

    /// Get world position as Vec2 for rendering
    pub fn world_position(&self) -> Vec2 {
        // Convert hex to pixel coordinates
        let hex_size = 1.0; // Base hex size
        let x = hex_size * (3.0 / 2.0 * self.hex.q as f32);
        let y = hex_size * ((3.0_f32).sqrt() / 2.0 * self.hex.q as f32 + (3.0_f32).sqrt() * self.hex.r as f32);
        Vec2::new(x, y)
    }

    /// Get 3D world position including elevation
    pub fn world_position_3d(&self) -> Vec3 {
        let pos_2d = self.world_position();
        Vec3::new(pos_2d.x, self.elevation, pos_2d.y)
    }
}

/// Terrain type enumeration for tiles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum TerrainType {
    Ocean = 0,
    Grassland = 1,
    Plains = 2,
    Desert = 3,
    Tundra = 4,
    Snow = 5,
    Forest = 6,
    Jungle = 7,
    Hills = 8,
    Mountain = 9,
    // Add more terrain types as needed
}

impl Default for TerrainType {
    fn default() -> Self {
        Self::Ocean
    }
}

/// Resource component for tiles (sparse)
#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub struct TileResource {
    /// Resource type identifier  
    pub resource_type: ResourceType,
    /// Quantity available (0-255 for memory efficiency)
    pub quantity: u8,
    /// Whether resource is visible to players
    pub discovered: bool,
    /// Depletion rate over time
    pub depletion_rate: f32,
}

/// Resource type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum ResourceType {
    None = 0,
    Iron = 1,
    Coal = 2,
    Oil = 3,
    Gold = 4,
    Silver = 5,
    Copper = 6,
    Stone = 7,
    Wheat = 8,
    Fish = 9,
    Cattle = 10,
    // Add more resource types as needed
}

impl Default for ResourceType {
    fn default() -> Self {
        Self::None
    }
}

/// Climate data component for environmental simulation
#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub struct Climate {
    /// Average temperature (-50 to 50 Celsius)
    pub temperature: i8,
    /// Rainfall amount (0-255mm annually)
    pub rainfall: u8,
    /// Humidity percentage (0-100)
    pub humidity: u8,
    /// Wind strength (0-255 arbitrary units)
    pub wind_strength: u8,
}

impl Default for Climate {
    fn default() -> Self {
        Self {
            temperature: 20, // 20°C
            rainfall: 100,   // 100mm
            humidity: 50,    // 50%
            wind_strength: 10, // Light wind
        }
    }
}

/// Fertility component for agricultural potential
#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub struct Fertility {
    /// Base fertility value (0.0 to 1.0)
    pub base_fertility: f32,
    /// Current fertility (affected by usage/improvements)
    pub current_fertility: f32,
    /// Fertility regeneration rate
    pub regen_rate: f32,
}

impl Default for Fertility {
    fn default() -> Self {
        Self {
            base_fertility: 0.5,
            current_fertility: 0.5,
            regen_rate: 0.01,
        }
    }
}

/// River component for tiles with water flow
#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub struct River {
    /// River strength/flow rate (0-255)
    pub flow_rate: u8,
    /// Directions where river flows (bitfield)
    pub flow_directions: u8,
    /// Whether this is a river source
    pub is_source: bool,
    /// River system ID for connected waterways
    pub river_system_id: u32,
}

/// Movement cost component for pathfinding
#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub struct MovementCost {
    /// Base movement cost for this tile
    pub base_cost: f32,
    /// Current modified cost (affected by improvements, weather, etc.)
    pub current_cost: f32,
    /// Whether tile blocks movement entirely
    pub impassable: bool,
}

impl Default for MovementCost {
    fn default() -> Self {
        Self {
            base_cost: 1.0,
            current_cost: 1.0,
            impassable: false,
        }
    }
}

/// Visibility component for fog of war
#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub struct Visibility {
    /// Players who have discovered this tile (bitfield)
    pub discovered_by: u64,
    /// Players who currently have vision (bitfield) 
    pub visible_to: u64,
    /// Last turn this tile was seen by each player
    pub last_seen: [u16; 8], // Support up to 8 players
}

impl Default for Visibility {
    fn default() -> Self {
        Self {
            discovered_by: 0,
            visible_to: 0,
            last_seen: [0; 8],
        }
    }
}

/// High-performance tile component manager using hecs
#[derive(Debug)]
pub struct TileComponentManager {
    /// Hecs world for sparse tile components
    hecs_world: Arc<RwLock<HecsWorld>>,
    /// Mapping from tile ID to hecs entity
    tile_entity_map: Arc<RwLock<std::collections::HashMap<TileId, HecsEntity>>>,
    /// Next available tile ID
    next_tile_id: Arc<RwLock<TileId>>,
}

impl TileComponentManager {
    /// Create new tile component manager
    pub fn new() -> Self {
        Self {
            hecs_world: Arc::new(RwLock::new(HecsWorld::new())),
            tile_entity_map: Arc::new(RwLock::new(std::collections::HashMap::new())),
            next_tile_id: Arc::new(RwLock::new(1)), // Start at 1, 0 is reserved for INVALID_TILE
        }
    }

    /// Create new tile with components
    #[instrument(skip(self))]
    pub fn create_tile(&self, hex: HexCoord, chunk: ChunkCoord, local_x: u8, local_y: u8, terrain_type: TerrainType) -> TileId {
        let tile_id = {
            let mut next_id = self.next_tile_id.write();
            let id = *next_id;
            *next_id += 1;
            id
        };

        let tile = Tile::new(tile_id, hex, chunk, local_x, local_y, terrain_type);
        let movement_cost = MovementCost::default();
        let visibility = Visibility::default();

        let entity = {
            let mut world = self.hecs_world.write();
            world.spawn((tile, movement_cost, visibility))
        };

        self.tile_entity_map.write().insert(tile_id, entity);
        debug!("Created tile {} at hex {:?}", tile_id, hex);

        tile_id
    }

    /// Add component to existing tile
    pub fn add_component<T: Component>(&self, tile_id: TileId, component: T) -> Result<(), TileError> {
        let entity = self.get_entity(tile_id)?;
        
        let mut world = self.hecs_world.write();
        world.insert_one(entity, component).map_err(|_| TileError::ComponentAddFailed)?;
        
        Ok(())
    }

    /// Remove component from tile
    pub fn remove_component<T: Component>(&self, tile_id: TileId) -> Result<T, TileError> {
        let entity = self.get_entity(tile_id)?;
        
        let mut world = self.hecs_world.write();
        world.remove_one::<T>(entity).map_err(|_| TileError::ComponentNotFound)
    }

    /// Get component from tile
    pub fn get_component<T: Component>(&self, tile_id: TileId) -> Result<T, TileError> 
    where
        T: Clone
    {
        let entity = self.get_entity(tile_id)?;
        
        let world = self.hecs_world.read();
        world.get::<&T>(entity)
            .map(|comp| comp.clone())
            .map_err(|_| TileError::ComponentNotFound)
    }

    /// Query tiles with specific components
    pub fn query_tiles<Q: Query>(&self) -> Vec<(TileId, Q::Item<'_>)> {
        let world = self.hecs_world.read();
        let entity_map = self.tile_entity_map.read();
        
        let mut results = Vec::new();
        
        for (entity, item) in world.query::<Q>().iter() {
            // Find tile ID for this entity
            if let Some((tile_id, _)) = entity_map.iter().find(|(_, &e)| e == entity) {
                results.push((*tile_id, item));
            }
        }
        
        results
    }

    /// Get all tiles in a specific chunk
    pub fn get_tiles_in_chunk(&self, chunk: ChunkCoord) -> Vec<TileId> {
        let world = self.hecs_world.read();
        let entity_map = self.tile_entity_map.read();
        let mut results = Vec::new();

        for (entity, tile) in world.query::<&Tile>().iter() {
            if tile.chunk == chunk {
                if let Some((tile_id, _)) = entity_map.iter().find(|(_, &e)| e == entity) {
                    results.push(*tile_id);
                }
            }
        }

        results
    }

    /// Get tiles within radius of hex coordinate
    pub fn get_tiles_in_radius(&self, center: HexCoord, radius: u32) -> Vec<TileId> {
        let world = self.hecs_world.read();
        let entity_map = self.tile_entity_map.read();
        let mut results = Vec::new();

        for (entity, tile) in world.query::<&Tile>().iter() {
            let distance = self.hex_distance(center, tile.hex);
            if distance <= radius {
                if let Some((tile_id, _)) = entity_map.iter().find(|(_, &e)| e == entity) {
                    results.push(*tile_id);
                }
            }
        }

        results
    }

    /// Delete tile and all its components
    pub fn delete_tile(&self, tile_id: TileId) -> Result<(), TileError> {
        let entity = self.get_entity(tile_id)?;
        
        {
            let mut world = self.hecs_world.write();
            world.despawn(entity).map_err(|_| TileError::TileNotFound)?;
        }
        
        self.tile_entity_map.write().remove(&tile_id);
        debug!("Deleted tile {}", tile_id);
        
        Ok(())
    }

    /// Get memory usage statistics
    pub fn memory_stats(&self) -> TileComponentStats {
        let world = self.hecs_world.read();
        let entity_count = world.len() as usize;
        let archetype_count = world.archetypes().len();
        
        TileComponentStats {
            tile_count: entity_count,
            archetype_count,
            estimated_memory_bytes: entity_count * 256, // Rough estimate
        }
    }

    /// Internal helper to get hecs entity for tile ID
    fn get_entity(&self, tile_id: TileId) -> Result<HecsEntity, TileError> {
        self.tile_entity_map.read()
            .get(&tile_id)
            .copied()
            .ok_or(TileError::TileNotFound)
    }

    /// Calculate hex distance (using Manhattan distance in cube coordinates)
    fn hex_distance(&self, hex1: HexCoord, hex2: HexCoord) -> u32 {
        let dx = (hex1.q - hex2.q).abs();
        let dy = (hex1.q + hex1.r - hex2.q - hex2.r).abs();
        let dz = (hex1.r - hex2.r).abs();
        ((dx + dy + dz) / 2) as u32
    }
}

impl Default for TileComponentManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileComponentStats {
    pub tile_count: usize,
    pub archetype_count: usize,
    pub estimated_memory_bytes: usize,
}

/// Tile system errors
#[derive(Debug, thiserror::Error)]
pub enum TileError {
    #[error("Tile not found")]
    TileNotFound,
    
    #[error("Component not found")]
    ComponentNotFound,
    
    #[error("Failed to add component")]
    ComponentAddFailed,
    
    #[error("Invalid tile coordinate")]
    InvalidCoordinate,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_creation() {
        let manager = TileComponentManager::new();
        let hex = HexCoord { q: 10, r: 20 };
        let chunk = ChunkCoord::new(1, 2);
        
        let tile_id = manager.create_tile(hex, chunk, 10, 20, TerrainType::Grassland);
        assert_ne!(tile_id, 0);
        
        let tile = manager.get_component::<Tile>(tile_id).unwrap();
        assert_eq!(tile.hex, hex);
        assert_eq!(tile.terrain_type, TerrainType::Grassland);
    }

    #[test]
    fn test_component_operations() {
        let manager = TileComponentManager::new();
        let hex = HexCoord { q: 0, r: 0 };
        let chunk = ChunkCoord::new(0, 0);
        
        let tile_id = manager.create_tile(hex, chunk, 0, 0, TerrainType::Forest);
        
        // Add resource component
        let resource = TileResource {
            resource_type: ResourceType::Iron,
            quantity: 100,
            discovered: false,
            depletion_rate: 0.1,
        };
        
        assert!(manager.add_component(tile_id, resource.clone()).is_ok());
        
        // Get resource component
        let retrieved = manager.get_component::<TileResource>(tile_id).unwrap();
        assert_eq!(retrieved.resource_type, ResourceType::Iron);
        assert_eq!(retrieved.quantity, 100);
    }

    #[test]
    fn test_tile_queries() {
        let manager = TileComponentManager::new();
        
        // Create tiles with different terrain types
        let hex1 = HexCoord { q: 0, r: 0 };
        let hex2 = HexCoord { q: 1, r: 0 };
        let chunk = ChunkCoord::new(0, 0);
        
        let tile1 = manager.create_tile(hex1, chunk, 0, 0, TerrainType::Forest);
        let tile2 = manager.create_tile(hex2, chunk, 1, 0, TerrainType::Mountain);
        
        // Query all tiles
        let all_tiles = manager.query_tiles::<&Tile>();
        assert_eq!(all_tiles.len(), 2);
    }
}
