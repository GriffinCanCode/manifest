//! World state serialization and persistence
//!
//! Contains methods for saving and loading world state to/from files.

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn};

use crate::ecs::{
    saves::*,
    entity_serialization::{serialize_entity, deserialize_entity},
    hierarchy::{HierarchyQueries, StableEntityId, Relationships, Hierarchical},
    WorldState
};

use super::core::GameWorld;

impl GameWorld {
    /// Export current world state for saving
    pub fn export_world_state(&mut self) -> WorldState {
        let mut state = WorldState::default();
        
        // Serialize all entities with their components
        let mut entity_query = self.world.query::<Entity>();
        let entities: Vec<Entity> = entity_query.iter(&self.world).collect();
        
        for entity in entities {
            if let Some(serialized_entity) = serialize_entity(&self.world, entity) {
                state.entities.push(serialized_entity);
                state.entity_count += 1;
            }
        }
        
        // Export hierarchical relationships (legacy format for compatibility)
        let hierarchical_entities: Vec<StableEntityId> = {
            let mut hierarchical_query = self.world.query_filtered::<Entity, With<Hierarchical>>();
            hierarchical_query.iter(&self.world)
                .filter_map(|entity| StableEntityId::from_entity(entity))
                .collect()
        };
        
        let entity_relationships: HashMap<StableEntityId, Relationships> = {
            let mut relationships_query = self.world.query::<(Entity, &Relationships)>();
            relationships_query.iter(&self.world)
                .filter_map(|(entity, relationships)| {
                    StableEntityId::from_entity(entity)
                        .map(|stable_id| (stable_id, relationships.clone()))
                })
                .collect()
        };
        
        state.hierarchical_entities = hierarchical_entities;
        state.entity_relationships = entity_relationships;
        
        info!("Exported world state with {} entities", state.entity_count);
        state
    }

    /// Import world state from saved data
    pub fn import_world_state(&mut self, state: WorldState) {
        // Clear existing entities but keep resources
        self.world.clear_entities();
        
        // Create entity mapping for relationship reconstruction
        let mut entity_mapping = HashMap::new();
        let mut restored_count = 0;
        
        // Restore entities
        for serialized_entity in &state.entities {
            let stable_id = serialized_entity.stable_id;
            let entity = deserialize_entity(&mut self.world, serialized_entity);
            entity_mapping.insert(stable_id, entity);
            
            // Register with archetype manager based on components
            // This is a bit tricky since we need to determine the bundle type dynamically
            if let Some(mut archetype_manager) = self.world.get_resource_mut::<crate::ecs::archetypes::ArchetypeManager>() {
                if serialized_entity.position.is_some() && 
                   serialized_entity.movement.is_some() && 
                   serialized_entity.health.is_some() {
                    // This is a UnitBundle entity
                    let _ = archetype_manager.register_entity::<crate::ecs::entities::UnitBundle>(entity);
                } else if serialized_entity.position.is_some() && 
                         serialized_entity.health.is_some() {
                    // This is a LivingEntityBundle entity
                    let _ = archetype_manager.register_entity::<crate::ecs::entities::LivingEntityBundle>(entity);
                } else if serialized_entity.position.is_some() && 
                         serialized_entity.movement.is_some() {
                    // This is a MovableEntityBundle entity
                    let _ = archetype_manager.register_entity::<crate::ecs::entities::MovableEntityBundle>(entity);
                }
            }

            restored_count += 1;
        }

        // Legacy: Restore hierarchical entities (for backwards compatibility)
        for stable_id in &state.hierarchical_entities {
            if let Some(&entity) = entity_mapping.get(stable_id) {
                if self.world.get_entity(entity).is_some() {
                    self.world.entity_mut(entity).insert(Hierarchical);
                }
            }
        }

        // Legacy: Restore entity relationships (for backwards compatibility)
        for (stable_id, relationships) in &state.entity_relationships {
            if let Some(&entity) = entity_mapping.get(stable_id) {
                if self.world.get_entity(entity).is_some() {
                    let mut updated_relationships = relationships.clone();
                    updated_relationships.remap_entities(&entity_mapping);
                    self.world.entity_mut(entity).insert(updated_relationships);
                }
            }
        }

        // Sync hierarchy system after import
        if let Some(hierarchy_queries) = self.world.remove_resource::<HierarchyQueries>() {
            // Manually sync the hierarchy using our direct approach
            let mut relationships_query = self.world.query::<(Entity, &Relationships)>();
            let updates: Vec<_> = relationships_query.iter(&self.world)
                .map(|(entity, relationships)| (entity, relationships.clone()))
                .collect();
            
            // Apply updates to hierarchy graph using public interface
            if let Err(e) = hierarchy_queries.update_relationships_sync(updates) {
                warn!("Failed to sync hierarchy after import: {}", e);
            }
            
            // Put the resource back
            self.world.insert_resource(hierarchy_queries);
        }

        info!("Imported world state with {} entities restored ({} entities in save file)", 
                      restored_count, state.entity_count);
    }

    /// Save world to a file
    pub fn save_world_to_file(&mut self, filename: &str) -> Result<(), SaveError> {
        let world_state = self.export_world_state();
        save_world_state_to_file(&world_state, filename)
    }

    /// Load world from a file
    pub fn load_world_from_file(&mut self, filename: &str) -> Result<(), SaveError> {
        let world_state = load_world_state_from_file(filename)?;
        self.import_world_state(world_state);
        Ok(())
    }

    /// Create a quick save
    pub fn quick_save(&mut self) -> Result<(), SaveError> {
        self.save_world_to_file("quicksave.json")
    }

    /// Load from quick save
    pub fn quick_load(&mut self) -> Result<(), SaveError> {
        self.load_world_from_file("quicksave.json")
    }

    /// Create an autosave with timestamp
    pub fn auto_save(&mut self) -> Result<String, SaveError> {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let filename = format!("autosave_{}.json", timestamp);
        self.save_world_to_file(&filename)?;
        Ok(filename)
    }

    /// Get available save files
    pub fn list_save_files() -> Result<Vec<SaveInfo>, SaveError> {
        list_available_saves()
    }

    /// Delete a save file
    pub fn delete_save_file(filename: &str) -> Result<(), SaveError> {
        delete_save_file(filename)
    }
}
