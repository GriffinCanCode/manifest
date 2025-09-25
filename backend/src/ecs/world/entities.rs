//! Entity management and lifecycle
//!
//! Contains entity spawning, despawning, and archetype management functionality.

use bevy_ecs::prelude::*;
use std::time::Instant;
use tracing::{info, warn, error};
use slotmap::Key;

use crate::core::logging::{LoggingSystem, game_logging};
use crate::ecs::{
    components::{Name, Position},
    archetypes::{ArchetypeManager, BundleComponentExtractor}
};

use super::core::GameWorld;

impl GameWorld {
    /// Spawn an entity with comprehensive tracking
    /// Maintains separation of concerns between spatial, archetype, and hierarchy systems
    pub fn spawn_entity_registered<T: Bundle + BundleComponentExtractor>(&mut self, bundle: T) -> Entity {
        let spawn_start = Instant::now();
        let correlation_id = LoggingSystem::generate_correlation_id();
        
        // Step 1: Spawn in ECS world (primary source of truth)
        let entity = self.world.spawn(bundle).id();
        
        // Step 2: Register with archetype manager (component organization)
        let archetype_id = match self.world.get_resource_mut::<ArchetypeManager>() {
            Some(mut archetype_manager) => archetype_manager.register_entity::<T>(entity),
            None => {
                error!(
                    target: "game::world::entities",
                    correlation_id = correlation_id,
                    entity = ?entity,
                    "ArchetypeManager resource not found - creating fallback archetype"
                );
                // Create proper fallback archetype ID based on entity index
                // This ensures deterministic archetype assignment even without manager
                let raw_entity_id = entity.index();
                slotmap::Key::from(slotmap::KeyData::from_ffi(raw_entity_id as u64))
            }
        };
        
        let archetype_id = match archetype_id {
            Ok(id) => id,
            Err(e) => {
                error!(
                    target: "game::world::entities",
                    correlation_id = correlation_id,
                    entity = ?entity,
                    error = %e,
                    "Failed to register entity with archetype system"
                );
                // Return entity but continue without archetype tracking
                return entity;
            }
        };
        let spawn_duration = spawn_start.elapsed().as_secs_f64() * 1000.0;
        
        // Step 3: SpatialIndex will pick it up automatically via incremental sync system
        // No need to manually sync here - happens automatically via Added<Position> queries
        
        // Log the entity creation using game-specific logging
        if let Some(name_component) = self.world.get::<Name>(entity) {
            info!(
                target: "game::world::entities",
                correlation_id = correlation_id,
                entity = ?entity,
                archetype_id = ?archetype_id,
                name = %name_component.value(),
                spawn_duration_ms = spawn_duration,
                "Entity spawned with name"
            );
            
            game_logging::log_entity_operation(entity, "spawn", Some(name_component.value()));
        } else {
            info!(
                target: "game::world::entities",
                correlation_id = correlation_id,
                entity = ?entity,
                archetype_id = ?archetype_id,
                spawn_duration_ms = spawn_duration,
                "Entity spawned"
            );
            
            game_logging::log_entity_operation(entity, "spawn", None);
        }
        
        // Log position if available
        if let Some(position) = self.world.get::<Position>(entity) {
            game_logging::log_spatial_operation(position.hex(), "entity_spawn", None);
        }
        
        game_logging::log_archetype_operation(archetype_id.data().as_ffi(), "entity_added", 1);
        game_logging::log_performance_event("entity_spawn", spawn_duration, 1);
        
        // Increment world generation to invalidate caches
        self.increment_world_generation();
        
        entity
    }

    /// Remove entity from all tracking systems
    /// Maintains clean separation during entity destruction
    pub fn despawn_entity_registered(&mut self, entity: Entity) -> bool {
        let despawn_start = Instant::now();
        let correlation_id = LoggingSystem::generate_correlation_id();
        
        // Get entity info before despawning for logging
        let name = self.world.get::<Name>(entity).map(|n| n.value().to_string());
        let position = self.world.get::<Position>(entity).map(|p| p.hex());
        
        // Step 1: Remove from archetype tracking (component organization)
        let archetype_result = if let Some(mut archetype_manager) = self.world.get_resource_mut::<ArchetypeManager>() {
            archetype_manager.unregister_entity(entity)
        } else {
            Err(crate::ecs::archetypes::ArchetypeError::EntityNotFound(entity))
        };
        
        // Step 2: Remove from ECS world (spatial index automatically updated via RemovedComponents<Position>)
        let success = if let Some(entity_mut) = self.world.get_entity_mut(entity) {
            entity_mut.despawn();
            true
        } else {
            false
        };
        
        let despawn_duration = despawn_start.elapsed().as_secs_f64() * 1000.0;
        
        if success {
            info!(
                target: "game::world::entities",
                correlation_id = correlation_id,
                entity = ?entity,
                name = ?name,
                position = ?position,
                despawn_duration_ms = despawn_duration,
                "Entity despawned successfully"
            );
            
            game_logging::log_entity_operation(entity, "despawn", name.as_deref());
            
            if let Some(pos) = position {
                game_logging::log_spatial_operation(pos, "entity_despawn", None);
            }
            
            if archetype_result.is_ok() {
                game_logging::log_archetype_operation(0, "entity_removed", 1);
            }
        } else {
            warn!(
                target: "game::world::entities",
                correlation_id = correlation_id,
                entity = ?entity,
                "Failed to despawn entity - entity not found"
            );
        }
        
        game_logging::log_performance_event("entity_despawn", despawn_duration, if success { 1 } else { 0 });
        
        if success {
            // Increment world generation to invalidate caches
            self.increment_world_generation();
        }
        
        success
    }

    /// Update entity archetype when components change
    /// Maintains archetype organization without interfering with spatial indexing
    pub fn update_entity_archetype<T: BundleComponentExtractor>(&mut self, entity: Entity) -> Result<(), String> {
        if let Some(mut archetype_manager) = self.world.get_resource_mut::<ArchetypeManager>() {
            archetype_manager.update_entity_archetype::<T>(entity)
                .map_err(|e| format!("Failed to update entity archetype: {}", e))
        } else {
            Err("ArchetypeManager resource not found".to_string())
        }
    }

    /// Cleanup empty archetypes (maintenance operation)
    /// Pure archetype manager responsibility
    pub fn cleanup_archetypes(&mut self) -> usize {
        if let Some(mut archetype_manager) = self.world.get_resource_mut::<ArchetypeManager>() {
            archetype_manager.cleanup()
        } else {
            0
        }
    }
}
