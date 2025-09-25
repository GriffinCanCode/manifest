//! Spatial queries and indexing
//!
//! Contains methods for finding entities by position, range, and other spatial queries.

use bevy_ecs::prelude::*;
use bevy_ecs::query::With;
use glam::IVec2;
use slotmap::Key;

use super::core::GameWorld;

impl GameWorld {
    /// Find all entities at a specific hex position using R-tree
    pub fn entities_at_position(&self, pos: IVec2) -> Vec<Entity> {
        self.spatial_index().entities_at_position(pos)
    }

    /// Find all entities within a hex range using optimized R-tree spatial queries
    pub fn entities_in_range(&self, center: IVec2, radius: u32) -> Vec<Entity> {
        self.spatial_index().entities_in_range(center, radius)
    }

    /// Find all entities owned by a player using high-performance spatial queries
    pub fn entities_owned_by_player(&self, player_id: u32) -> Vec<Entity> {
        self.spatial_index().entities_owned_by_player(player_id)
    }

    /// Find entities within a rectangular area
    pub fn entities_in_rectangle(&self, min: IVec2, max: IVec2) -> Vec<Entity> {
        self.spatial_index().entities_in_rectangle(min, max)
    }

    /// Find the nearest entity to a given position
    pub fn nearest_entity(&self, pos: IVec2) -> Option<Entity> {
        self.spatial_index().nearest_entity(pos)
    }

    /// Find all entities of a specific type at a position
    pub fn entities_at_position_with_component<T: Component>(&mut self, pos: IVec2) -> Vec<Entity> {
        let spatial_entities = self.entities_at_position(pos);
        let mut query = self.world.query_filtered::<Entity, With<T>>();
        let entities_with_component: std::collections::HashSet<Entity> = 
            query.iter(&self.world).collect();
        
        spatial_entities.into_iter()
            .filter(|e| entities_with_component.contains(e))
            .collect()
    }

    /// Find all entities of a specific type within range
    pub fn entities_in_range_with_component<T: Component>(&mut self, center: IVec2, radius: u32) -> Vec<Entity> {
        let spatial_entities = self.entities_in_range(center, radius);
        let mut query = self.world.query_filtered::<Entity, With<T>>();
        let entities_with_component: std::collections::HashSet<Entity> = 
            query.iter(&self.world).collect();
        
        spatial_entities.into_iter()
            .filter(|e| entities_with_component.contains(e))
            .collect()
    }

    /// Count entities at a specific position
    pub fn count_entities_at_position(&self, pos: IVec2) -> usize {
        self.spatial_index().count_entities_at_position(pos)
    }

    /// Count entities within a range
    pub fn count_entities_in_range(&self, center: IVec2, radius: u32) -> usize {
        self.spatial_index().count_entities_in_range(center, radius)
    }

    /// Check if a position is occupied by any entity
    pub fn is_position_occupied(&self, pos: IVec2) -> bool {
        !self.spatial_index().entities_at_position(pos).is_empty()
    }

    /// Find all empty positions within a range
    pub fn find_empty_positions_in_range(&self, center: IVec2, radius: u32) -> Vec<IVec2> {
        let mut empty_positions = Vec::new();
        
        // Generate all positions in hex range
        for q in (center.x - radius as i32)..=(center.x + radius as i32) {
            for r in (center.y - radius as i32)..=(center.y + radius as i32) {
                let pos = IVec2::new(q, r);
                
                // Check if position is within hex distance
                let hex_distance = ((q - center.x).abs() + 
                                   ((q + r) - (center.x + center.y)).abs() + 
                                   (r - center.y).abs()) / 2;
                
                if hex_distance <= radius as i32 && !self.is_position_occupied(pos) {
                    empty_positions.push(pos);
                }
            }
        }
        
        empty_positions
    }
}
