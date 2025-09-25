//! Entity management and lifecycle
//!
//! Contains entity spawning, despawning, and archetype management functionality.

use bevy_ecs::prelude::*;
use std::time::Instant;
use tracing::{info, warn};

use crate::core::logging::{LoggingSystem, game_logging};
use crate::ecs::components::{Name, Position};

use super::core::GameWorld;

impl GameWorld {
    /// Spawn an entity with automatic spatial and change detection
    /// Uses Bevy ECS built-in systems for archetype management and change detection
    pub fn spawn_entity<T: Bundle>(&mut self, bundle: T) -> Entity {
        let spawn_start = Instant::now();
        let correlation_id = LoggingSystem::generate_correlation_id();
        
        // Spawn in ECS world - Bevy handles archetype management automatically
        let entity = self.world.spawn(bundle).id();
        
        let spawn_duration = spawn_start.elapsed().as_secs_f64() * 1000.0;
        
        // Spatial index updates automatically via Added<Position> change detection
        // Archetype organization handled automatically by Bevy ECS
        
        // Log the entity creation
        if let Some(name_component) = self.world.get::<Name>(entity) {
            info!(
                target: "game::world::entities",
                correlation_id = correlation_id,
                entity = ?entity,
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
                spawn_duration_ms = spawn_duration,
                "Entity spawned"
            );
            
            game_logging::log_entity_operation(entity, "spawn", None);
        }
        
        // Log position if available
        if let Some(position) = self.world.get::<Position>(entity) {
            game_logging::log_spatial_operation(position.hex(), "entity_spawn", None);
        }
        
        game_logging::log_performance_event("entity_spawn", spawn_duration, 1);
        
        // Increment world generation to invalidate caches
        self.increment_world_generation();
        
        entity
    }

    /// Remove entity with automatic cleanup via change detection
    /// Uses Bevy ECS built-in systems for archetype and spatial cleanup
    pub fn despawn_entity(&mut self, entity: Entity) -> bool {
        let despawn_start = Instant::now();
        let correlation_id = LoggingSystem::generate_correlation_id();
        
        // Get entity info before despawning for logging
        let name = self.world.get::<Name>(entity).map(|n| n.value().to_string());
        let position = self.world.get::<Position>(entity).map(|p| p.hex());
        
        // Despawn from ECS world - Bevy handles archetype cleanup automatically
        // Spatial index updated automatically via RemovedComponents<Position>
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

    // Archetype management is now handled automatically by Bevy ECS
    // No need for manual archetype tracking or cleanup
}
