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

        // Update game time through world manager
        self.world_manager.update_game_time(delta);
        
        // Validate hierarchical data periodically through subsystem registry
        self.subsystem_registry.validate_hierarchy(self.world(), delta);

        // Process hot reload events
        #[cfg(debug_assertions)]
        self.subsystem_registry.process_reload_events();

        // Run systems through system coordinator
        self.system_coordinator.run_system_stages(self.world_mut());
    }

    /// Update the world with a fixed time step (useful for deterministic simulation)
    pub fn update_fixed(&mut self, fixed_delta: f32) {
        // Game time is now updated by systems, but we can set a target speed
        if let Some(game_time) = self.world().get_resource::<GameTime>() {
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

        // Run systems through system coordinator
        self.system_coordinator.run_system_stages(self.world_mut());
    }

    // System stage execution now handled by SystemCoordinator
    // Hot reload event processing now handled by SubsystemRegistry
}
