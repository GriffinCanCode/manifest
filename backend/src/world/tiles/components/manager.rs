//! High-performance tile component management
//!
//! Contains the TileComponentManager for efficient sparse component storage
//! using hecs ECS integrated with bevy_ecs for optimal performance.

use hecs::{World as HecsWorld, Entity as HecsEntity, Component, Query};
use bevy_ecs::prelude::*;
use parking_lot::RwLock;
use std::sync::Arc;
use tracing::{debug, instrument};

use crate::core::zig_ffi::HexCoord;
use crate::world::tiles::chunks::{TileId, ChunkCoord};
use super::{
    core::{Tile, TerrainType},
    movement::MovementCost,
    visibility::Visibility,
    errors::{TileError, TileComponentStats},
};

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
            (*tile_ref).clone()  // Dereference to get owned Tile
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
