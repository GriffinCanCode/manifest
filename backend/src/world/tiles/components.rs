//! Tile components with hecs sparse storage integration
//!
//! Provides efficient sparse component storage for tiles using hecs ECS,
//! integrated with the main bevy_ecs world for optimal performance.

use hecs::{World as HecsWorld, Entity as HecsEntity, Component, Query};
use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use glam::{Vec2, Vec3};
use parking_lot::RwLock;
use std::sync::Arc;
use modular_bitfield::prelude::*;
use crate::core::zig_ffi::HexCoord;
use crate::world::tiles::chunks::{TileId, ChunkCoord};
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
    Mountains = 10, // Alias for Mountain for backward compatibility
    River = 11,
    Coast = 12,
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

/// River flow directions bitfield for compact storage
#[bitfield(bits = 8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RiverFlowDirections {
    /// East direction flow
    east: bool,
    /// Northeast direction flow
    northeast: bool,
    /// Northwest direction flow
    northwest: bool,
    /// West direction flow
    west: bool,
    /// Southwest direction flow
    southwest: bool,
    /// Southeast direction flow
    southeast: bool,
    /// Reserved for future use
    #[bits = 2]
    reserved: B2,
}

impl Default for RiverFlowDirections {
    fn default() -> Self {
        Self::new()
    }
}

impl RiverFlowDirections {
    /// Set flow direction using HexDirection enum
    pub fn set_direction(&mut self, direction: crate::world::tiles::adjacency::HexDirection, flowing: bool) {
        use crate::world::tiles::adjacency::HexDirection;
        match direction {
            HexDirection::East => self.set_east(flowing),
            HexDirection::Northeast => self.set_northeast(flowing),
            HexDirection::Northwest => self.set_northwest(flowing),
            HexDirection::West => self.set_west(flowing),
            HexDirection::Southwest => self.set_southwest(flowing),
            HexDirection::Southeast => self.set_southeast(flowing),
        }
    }
    
    /// Check if flowing in specific direction
    pub fn is_flowing(&self, direction: crate::world::tiles::adjacency::HexDirection) -> bool {
        use crate::world::tiles::adjacency::HexDirection;
        match direction {
            HexDirection::East => self.east(),
            HexDirection::Northeast => self.northeast(),
            HexDirection::Northwest => self.northwest(),
            HexDirection::West => self.west(),
            HexDirection::Southwest => self.southwest(),
            HexDirection::Southeast => self.southeast(),
        }
    }
    
    /// Get all flowing directions
    pub fn get_flowing_directions(&self) -> Vec<crate::world::tiles::adjacency::HexDirection> {
        use crate::world::tiles::adjacency::HexDirection;
        let mut directions = Vec::new();
        if self.east() { directions.push(HexDirection::East); }
        if self.northeast() { directions.push(HexDirection::Northeast); }
        if self.northwest() { directions.push(HexDirection::Northwest); }
        if self.west() { directions.push(HexDirection::West); }
        if self.southwest() { directions.push(HexDirection::Southwest); }
        if self.southeast() { directions.push(HexDirection::Southeast); }
        directions
    }
}

/// River component for tiles with water flow
#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub struct River {
    /// River strength/flow rate (0-255)
    pub flow_rate: u8,
    /// Directions where river flows (using bitfield)
    pub flow_directions: RiverFlowDirections,
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

/// Player visibility bitfield for efficient storage
#[bitfield(bits = 64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerVisibilityFlags {
    /// Players who have discovered this tile (32 players max)
    #[bits = 32]
    pub discovered_by: u32,
    /// Players who currently have vision (32 players max)
    #[bits = 32] 
    pub visible_to: u32,
}

impl Default for PlayerVisibilityFlags {
    fn default() -> Self {
        Self::new()
    }
}

impl PlayerVisibilityFlags {
    /// Check if player has discovered this tile
    pub fn is_discovered_by_player(&self, player_id: u8) -> bool {
        if player_id >= 32 { return false; }
        (self.discovered_by() & (1 << player_id)) != 0
    }
    
    /// Set discovery status for player
    pub fn set_discovered_by_player(&mut self, player_id: u8, discovered: bool) {
        if player_id >= 32 { return; }
        let mask = 1 << player_id;
        if discovered {
            self.set_discovered_by(self.discovered_by() | mask);
        } else {
            self.set_discovered_by(self.discovered_by() & !mask);
        }
    }
    
    /// Check if player has vision of this tile
    pub fn is_visible_to_player(&self, player_id: u8) -> bool {
        if player_id >= 32 { return false; }
        (self.visible_to() & (1 << player_id)) != 0
    }
    
    /// Set visibility for player
    pub fn set_visible_to_player(&mut self, player_id: u8, visible: bool) {
        if player_id >= 32 { return; }
        let mask = 1 << player_id;
        if visible {
            self.set_visible_to(self.visible_to() | mask);
        } else {
            self.set_visible_to(self.visible_to() & !mask);
        }
    }
}

/// Visibility component for fog of war with improved bitfield storage
#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub struct Visibility {
    /// Player visibility flags (discovered and visible)
    pub player_flags: PlayerVisibilityFlags,
    /// Last turn this tile was seen by each player
    pub last_seen: [u16; 8], // Support up to 8 players for tracking
}

impl Default for Visibility {
    fn default() -> Self {
        Self {
            player_flags: PlayerVisibilityFlags::default(),
            last_seen: [0; 8],
        }
    }
}

impl Visibility {
    /// Check if player has discovered this tile
    pub fn is_discovered_by(&self, player_id: u8) -> bool {
        self.player_flags.is_discovered_by_player(player_id)
    }
    
    /// Set discovery status for player
    pub fn set_discovered_by(&mut self, player_id: u8, discovered: bool) {
        self.player_flags.set_discovered_by_player(player_id, discovered);
        if discovered && player_id < 8 {
            // Update last_seen when discovered
            self.last_seen[player_id as usize] = 1; // Would use current turn in real implementation
        }
    }
    
    /// Check if player has vision of this tile
    pub fn is_visible_to(&self, player_id: u8) -> bool {
        self.player_flags.is_visible_to_player(player_id)
    }
    
    /// Set visibility for player
    pub fn set_visible_to(&mut self, player_id: u8, visible: bool) {
        self.player_flags.set_visible_to_player(player_id, visible);
        if visible && player_id < 8 {
            // Update last_seen when visible
            self.last_seen[player_id as usize] = 1; // Would use current turn in real implementation
        }
    }
    
    /// Get last turn this tile was seen by player
    pub fn last_seen_by(&self, player_id: u8) -> u16 {
        if player_id < 8 {
            self.last_seen[player_id as usize]
        } else {
            0
        }
    }
    
    /// Set last seen turn for player
    pub fn set_last_seen(&mut self, player_id: u8, turn: u16) {
        if player_id < 8 {
            self.last_seen[player_id as usize] = turn;
        }
    }
}

/// High-performance tile component manager using hecs
pub struct TileComponentManager {
    /// Hecs world for sparse tile components
    hecs_world: Arc<RwLock<HecsWorld>>,
    /// Mapping from tile ID to hecs entity
    tile_entity_map: Arc<RwLock<std::collections::HashMap<TileId, HecsEntity>>>,
    /// Mapping from tile ID to bevy entity (for ECS system integration)
    bevy_entity_map: Arc<RwLock<std::collections::HashMap<TileId, bevy_ecs::entity::Entity>>>,
    /// Reverse mapping from bevy entity to tile ID
    bevy_reverse_map: Arc<RwLock<std::collections::HashMap<bevy_ecs::entity::Entity, TileId>>>,
    /// Next available tile ID
    next_tile_id: Arc<RwLock<TileId>>,
}

impl std::fmt::Debug for TileComponentManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TileComponentManager")
            .field("tile_entity_count", &self.tile_entity_map.read().len())
            .field("next_tile_id", &self.next_tile_id)
            .finish()
    }
}

impl TileComponentManager {
    /// Create new tile component manager
    pub fn new() -> Self {
        Self {
            hecs_world: Arc::new(RwLock::new(HecsWorld::new())),
            tile_entity_map: Arc::new(RwLock::new(std::collections::HashMap::new())),
            bevy_entity_map: Arc::new(RwLock::new(std::collections::HashMap::new())),
            bevy_reverse_map: Arc::new(RwLock::new(std::collections::HashMap::new())),
            next_tile_id: Arc::new(RwLock::new(TileId(1))), // Start at 1, 0 is reserved for INVALID_TILE
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
    pub fn get_component<T: Component + Clone>(&self, tile_id: TileId) -> Result<T, TileError> {
        let entity = self.get_entity(tile_id)?;
        
        let world = self.hecs_world.read();
        let result = if let Ok(component_ref) = world.get::<&T>(entity) {
            Ok((*component_ref).clone())
        } else {
            Err(TileError::ComponentNotFound)
        };
        result
    }

    /// Query tiles with specific components - returns tile IDs only to avoid lifetime issues
    pub fn query_tiles<Q: Query>(&self) -> Vec<TileId> {
        let world = self.hecs_world.read();
        let entity_map = self.tile_entity_map.read();
        
        let mut results = Vec::new();
        
        for (entity, _item) in world.query::<Q>().iter() {
            // Find tile ID for this entity
            if let Some((tile_id, _)) = entity_map.iter().find(|(_, &e)| e == entity) {
                results.push(*tile_id);
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
        
        // Clean up mappings
        self.tile_entity_map.write().remove(&tile_id);
        
        // Clean up Bevy entity mappings if they exist
        if let Some(bevy_entity) = self.bevy_entity_map.write().remove(&tile_id) {
            self.bevy_reverse_map.write().remove(&bevy_entity);
        }
        
        debug!("Deleted tile {}", tile_id);
        
        Ok(())
    }

    /// Get tile entity for a tile ID (hecs version)
    pub fn get_tile_entity(&self, tile_id: TileId) -> Option<HecsEntity> {
        self.tile_entity_map.read().get(&tile_id).copied()
    }

    /// Get Bevy ECS entity for a tile ID
    pub fn get_bevy_entity(&self, tile_id: TileId) -> Option<bevy_ecs::entity::Entity> {
        self.bevy_entity_map.read().get(&tile_id).copied()
    }

    /// Get tile ID from Bevy entity
    pub fn get_tile_id_from_bevy_entity(&self, entity: bevy_ecs::entity::Entity) -> Option<TileId> {
        self.bevy_reverse_map.read().get(&entity).copied()
    }

    /// Create tile entity in Bevy ECS world (for ECS system integration)
    pub fn create_bevy_tile_entity(&self, tile_id: TileId, world: &mut bevy_ecs::world::World) -> Result<bevy_ecs::entity::Entity, TileError> {
        // Get tile data from hecs world
        let hecs_entity = self.get_entity(tile_id)?;
        let tile_data = {
            let hecs_world_guard = self.hecs_world.read();
            let tile_ref = hecs_world_guard.get::<&Tile>(hecs_entity)
                .map_err(|_| TileError::ComponentNotFound)?;
            tile_ref.clone()
        };

        // Create corresponding entity in Bevy world
        let bevy_entity = world.spawn((
            tile_data,
            // Add Hierarchical marker for hierarchy system integration
            crate::ecs::hierarchy::components::Hierarchical,
        )).id();

        // Store mappings
        self.bevy_entity_map.write().insert(tile_id, bevy_entity);
        self.bevy_reverse_map.write().insert(bevy_entity, tile_id);

        debug!("Created Bevy entity {:?} for tile {}", bevy_entity, tile_id);
        Ok(bevy_entity)
    }

    /// Remove Bevy entity mapping for a tile
    pub fn remove_bevy_entity(&self, tile_id: TileId, world: &mut bevy_ecs::world::World) -> Result<(), TileError> {
        if let Some(bevy_entity) = self.bevy_entity_map.write().remove(&tile_id) {
            self.bevy_reverse_map.write().remove(&bevy_entity);
            
            // Despawn from Bevy world
            if let Some(entity_mut) = world.get_entity_mut(bevy_entity) {
                entity_mut.despawn();
            }
            
            debug!("Removed Bevy entity {:?} for tile {}", bevy_entity, tile_id);
        }
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
        
        let tile = manager.get_component::<Tile>(tile_id).expect("Tile component should exist after creation");
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
        let retrieved = manager.get_component::<TileResource>(tile_id).expect("TileResource component should exist after being added");
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
