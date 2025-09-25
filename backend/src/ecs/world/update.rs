//! World update systems and time management
//!
//! Contains the main update loops for running systems and managing time.

use std::time::Instant;
use tracing::{warn, error, debug};

use crate::core::{
    Stage,
    logging::{LoggingSystem, game_logging}
};
use crate::ecs::{
    resources::GameTime,
    hierarchy::HierarchyQueries
};

use super::core::GameWorld;

impl GameWorld {
    /// Update the world with automatic delta time calculation
    pub fn update(&mut self) {
        let now = Instant::now();
        let delta = now.duration_since(self.last_update()).as_secs_f32();
        self.set_last_update(now);

        // Update game time through systems  
        let simulation_state = self.world.get_resource::<crate::core::time::SimulationState>().cloned();
        if let (Some(mut game_time), Some(simulation_state)) = (
            self.world.get_resource_mut::<GameTime>(),
            simulation_state
        ) {
            game_time.update(delta, &simulation_state);
        }
        
        // Validate hierarchical data periodically (every 5 seconds in debug, 30 seconds in release)
        let validation_interval = if cfg!(debug_assertions) { 5.0 } else { 30.0 };
        
        if let Some(game_time) = self.world.get_resource::<GameTime>() {
            // Use tick count as a rough approximation for total elapsed time
            let elapsed_time = game_time.tick as f32 * game_time.delta_time;
            if elapsed_time % validation_interval < delta {
                let correlation_id = LoggingSystem::generate_correlation_id();
                let validation_start = Instant::now();
                
                if let Some(hierarchy_queries) = self.world.get_resource::<HierarchyQueries>() {
                    match hierarchy_queries.validate_hierarchy(&self.world) {
                        Ok(validation) => {
                            let validation_duration = validation_start.elapsed().as_secs_f64() * 1000.0;
                            
                            if validation.has_cycles {
                                error!(
                                    target: "game::world::hierarchy",
                                    correlation_id = correlation_id,
                                    entity_count = validation.entity_count,
                                    relationship_count = validation.relationship_count,
                                    "Hierarchy cycles detected!"
                                );
                            }
                            
                            if validation.orphaned_entities > 0 {
                                warn!(
                                    target: "game::world::hierarchy",
                                    correlation_id = correlation_id,
                                    orphaned_entities = validation.orphaned_entities,
                                    "Found orphaned entities in hierarchy"
                                );
                            }
                            
                            debug!(
                                target: "game::world::hierarchy",
                                correlation_id = correlation_id,
                                entity_count = validation.entity_count,
                                relationship_count = validation.relationship_count,
                                orphaned_entities = validation.orphaned_entities,
                                has_cycles = validation.has_cycles,
                                validation_duration_ms = validation_duration,
                                "Hierarchy validation completed"
                            );
                            
                            game_logging::log_performance_event("hierarchy_validation", validation_duration, validation.entity_count);
                        }
                        Err(e) => {
                            error!(
                                target: "game::world::hierarchy",
                                correlation_id = correlation_id,
                                error = %e,
                                "Hierarchy validation failed"
                            );
                        }
                    }
                }
            }
        }

        // Process hot reload events
        #[cfg(debug_assertions)]
        self.process_reload_events();

        // Run systems in parallel stages
        self.run_system_stages();
    }

    /// Update the world with a fixed time step (useful for deterministic simulation)
    pub fn update_fixed(&mut self, fixed_delta: f32) {
        // Game time is now updated by systems, but we can set a target speed
        if let Some(game_time) = self.world.get_resource::<GameTime>() {
            let target_speed = fixed_delta / (1.0 / 60.0); // Convert to speed multiplier
            let _ = game_time.set_speed(target_speed); // Ignore errors for now
            
            debug!(
                target: "game::world::fixed",
                fixed_delta = fixed_delta,
                target_speed = target_speed,
                "Fixed timestep update"
            );
        }

        // Spatial indexing now handled automatically by incremental_spatial_sync system
        // No expensive full rebuilds needed in fixed timestep mode!

        // Run systems in parallel stages
        self.run_system_stages();
    }

    /// Run all system stages in sequence
    fn run_system_stages(&mut self) {
        let stages = [Stage::PreUpdate, Stage::Update, Stage::PostUpdate, Stage::Cleanup];
        for stage in stages {
            if let Err(errors) = self.scheduler.run_stage(stage, &mut self.world) {
                error!("System execution errors: {:?}", errors);
            }
        }
    }

    /// Process hot reload events
    #[cfg(debug_assertions)]
    fn process_reload_events(&mut self) {
        use crate::core::reloader::{ReloadEvent};
        
        if let Some(ref manager) = self.reload_manager() {
            for event in manager.poll_events() {
                match event {
                    ReloadEvent::Reloaded { path, handler } => {
                        debug!("🔄 Reloaded {} with {}", path.display(), handler);
                    }
                    ReloadEvent::Failed { path, error } => {
                        warn!("❌ Reload failed for {}: {}", path.display(), error);
                    }
                    ReloadEvent::FileChanged { path } => {
                        debug!("📝 File changed: {}", path.display());
                    }
                }
            }
        }
    }
}
