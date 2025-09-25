//! Query methods for tile hierarchy navigation
//!
//! Contains methods for finding, searching, and navigating through
//! the hierarchical tile structure using spatial and hierarchical queries.

use bevy_ecs::prelude::*;
use petgraph::Direction;

use crate::core::{
    zig_ffi::HexCoord,
    caching::CacheKey
};
use crate::ecs::hierarchy::RelationshipType;

use super::{
    manager::TileHierarchy,
    types::HierarchicalTile
};

// Access to the hierarchical_tiles field in TileHierarchy
impl TileHierarchy {
    pub fn hierarchical_tiles(&self) -> &std::sync::Arc<parking_lot::RwLock<crate::core::hashing::FastHashMap<u8, Vec<bevy_ecs::entity::Entity>>>> {
        &self.hierarchical_tiles
    }
}

impl TileHierarchy {
    /// Find hierarchical tile containing the given hex coordinate
    pub async fn find_containing_tile(&self, hex: HexCoord, resolution: u8) -> Option<Entity> {
        let cache_key = CacheKey::Custom(format!("containing_tile:{}:{}:{}", hex.q, hex.r, resolution));
        
        // Check cache first
        if let Ok(Some(entity)) = self.cache().get::<Entity>(&cache_key).await {
            return Some(entity);
        }

        // Search tiles at specified resolution
        let tiles = self.get_tiles_at_resolution(resolution);
        
        // Note: This method needs world access, should be called with world parameter
        // For now, return None to indicate world access is needed
        // TODO: Refactor to accept World parameter
        
        None
        /*
        for tile_entity in tiles {
            // TODO: This needs world access - should be refactored to accept &World parameter
            if false { // Placeholder condition
                    // Check if this tile contains the hex coordinate
                    if let Ok(hierarchical_tile) = world.get::<HierarchicalTile>(tile_entity) {
                        if hierarchical_tile.bounds.contains_hex(hex) {
                            // Cache the result for future queries
                            let _ = self.cache().set(cache_key, tile_entity, crate::core::caching::CachePriority::Normal).await;
                            return Some(tile_entity);
                        }
                    } else if let Some(position) = world.get::<crate::ecs::components::Position>(tile_entity) {
                        // Fallback: check if hex coordinate matches position
                        if position.hex() == hex {
                            let _ = self.cache().set(cache_key, tile_entity, crate::core::caching::CachePriority::Normal).await;
                            return Some(tile_entity);
                        }
                    }
                }
            }
        }
        */
    }

    /// Get all ancestor tiles up the hierarchy
    pub async fn get_ancestor_tiles(&self, tile_entity: Entity) -> Vec<Entity> {
        self.hierarchy_queries().ancestors(tile_entity).await
    }

    /// Get all descendant tiles down the hierarchy
    pub async fn get_descendant_tiles(&self, tile_entity: Entity) -> Vec<Entity> {
        self.hierarchy_queries().descendants(tile_entity).await
    }

    /// Find tiles influenced by the given tile (using influence relationships)
    pub fn get_influenced_tiles(&self, world: &mut World, tile_entity: Entity) -> Vec<Entity> {
        self.hierarchy_queries().find_by_relationship(world, RelationshipType::Attachment, Direction::Outgoing)
            .get(&tile_entity)
            .cloned()
            .unwrap_or_default()
    }

    /// Find adjacent tiles (direct spatial neighbors)
    pub fn get_adjacent_tiles(&self, world: &mut World, tile_entity: Entity) -> Vec<Entity> {
        // This would use the spatial adjacency relationships
        // Implementation depends on how adjacency is tracked
        self.hierarchy_queries().find_by_relationship(world, RelationshipType::Attachment, Direction::Outgoing)
            .get(&tile_entity)
            .cloned()
            .unwrap_or_default()
    }

    /// Find tiles at a specific resolution level within a given area
    pub fn find_tiles_in_area(&self, world: &World, center: HexCoord, radius: u32, resolution: u8) -> Vec<Entity> {
        let mut result = Vec::new();
        let tiles = self.get_tiles_at_resolution(resolution);
        
        for tile_entity in tiles {
            if let Some(hierarchical_tile) = world.get::<HierarchicalTile>(tile_entity) {
                if self.hex_distance(center, hierarchical_tile.hex) <= radius {
                    result.push(tile_entity);
                }
            }
        }
        
        result
    }

    /// Find the nearest hierarchical tile to a given coordinate at a specific resolution
    pub fn find_nearest_tile(&self, world: &World, target: HexCoord, resolution: u8) -> Option<Entity> {
        let tiles = self.get_tiles_at_resolution(resolution);
        let mut nearest = None;
        let mut min_distance = u32::MAX;
        
        for tile_entity in tiles {
            if let Some(hierarchical_tile) = world.get::<HierarchicalTile>(tile_entity) {
                let distance = self.hex_distance(target, hierarchical_tile.hex);
                if distance < min_distance {
                    min_distance = distance;
                    nearest = Some(tile_entity);
                }
            }
        }
        
        nearest
    }

    /// Get all tiles that contain a specific base tile ID at any resolution
    pub fn find_containing_hierarchical_tiles(&self, world: &World, base_tile_id: crate::world::tiles::chunks::TileId) -> Vec<(Entity, u8)> {
        let mut result = Vec::new();
        let hierarchical_tiles = self.hierarchical_tiles().read();
        
        for (&resolution, entities) in hierarchical_tiles.iter() {
            for &entity in entities {
                if let Some(hierarchical_tile) = world.get::<HierarchicalTile>(entity) {
                    if hierarchical_tile.base_tile_id == base_tile_id {
                        result.push((entity, resolution));
                    }
                }
            }
        }
        
        result
    }

    /// Get tiles that overlap with a given bounding area
    pub fn find_overlapping_tiles(&self, world: &World, min_q: i32, max_q: i32, min_r: i32, max_r: i32, resolution: u8) -> Vec<Entity> {
        let mut result = Vec::new();
        let tiles = self.get_tiles_at_resolution(resolution);
        
        for tile_entity in tiles {
            if let Some(hierarchical_tile) = world.get::<HierarchicalTile>(tile_entity) {
                // Check if tile bounds overlap with query area
                let bounds = &hierarchical_tile.bounds;
                if bounds.max_q >= min_q && bounds.min_q <= max_q &&
                   bounds.max_r >= min_r && bounds.min_r <= max_r {
                    result.push(tile_entity);
                }
            }
        }
        
        result
    }

    /// Get all hierarchical tiles at or above a minimum resolution level
    pub fn get_tiles_at_or_above_resolution(&self, min_resolution: u8) -> Vec<(Entity, u8)> {
        let mut result = Vec::new();
        let hierarchical_tiles = self.hierarchical_tiles().read();
        
        for (&resolution, entities) in hierarchical_tiles.iter() {
            if resolution >= min_resolution {
                for &entity in entities {
                    result.push((entity, resolution));
                }
            }
        }
        
        result
    }

    /// Get all hierarchical tiles at or below a maximum resolution level
    pub fn get_tiles_at_or_below_resolution(&self, max_resolution: u8) -> Vec<(Entity, u8)> {
        let mut result = Vec::new();
        let hierarchical_tiles = self.hierarchical_tiles().read();
        
        for (&resolution, entities) in hierarchical_tiles.iter() {
            if resolution <= max_resolution {
                for &entity in entities {
                    result.push((entity, resolution));
                }
            }
        }
        
        result
    }
}
