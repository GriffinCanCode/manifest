//! Core GameWorld struct - Refactored for Focused Responsibilities
//!
//! GameWorld now delegates specialized operations to focused manager structs,
//! reducing responsibilities and improving maintainability.

use bevy_ecs::prelude::*;
use std::time::Instant;
use tracing::info;

use super::managers::{WorldManager, SystemCoordinator, SubsystemRegistry};
use crate::ecs::spatial::OptimalSpatialIndex;
use crate::core::caching::GameCache;

/// Main game world wrapper - Now delegates to specialized managers
#[derive(Debug)]
pub struct GameWorld {
    /// Core world operations manager
    world_manager: WorldManager,
    /// System coordination and execution
    system_coordinator: SystemCoordinator,
    /// Specialized subsystem coordination
    subsystem_registry: SubsystemRegistry,
}

impl GameWorld {
    /// Create a new game world with specialized manager delegation
    pub fn new() -> Self {
        // Create world manager with initialized resources
        let mut world_manager = WorldManager::new();
        
        // Create system coordinator and configure all systems
        let mut system_coordinator = SystemCoordinator::new()
            .expect("Failed to create SystemCoordinator");
        system_coordinator.configure_systems(world_manager.world_mut());
        
        // Create subsystem registry with all specialized subsystems
        let subsystem_registry = SubsystemRegistry::new(world_manager.world_mut());
        
        info!("🎮 GameWorld initialized with specialized manager delegation");
        
        Self {
            world_manager,
            system_coordinator,
            subsystem_registry,
        }
    }

    // System configuration now handled by SystemCoordinator

    /// Get a reference to the ECS world for external access
    pub fn world(&self) -> &World {
        self.world_manager.world()
    }

    /// Get a mutable reference to the ECS world for external modifications
    pub fn world_mut(&mut self) -> &mut World {
        self.world_manager.world_mut()
    }

    /// Get scheduler performance metrics
    pub fn scheduler_metrics(&self) -> crate::core::SchedulerMetrics {
        self.system_coordinator.scheduler_metrics()
    }

    /// Check if the scheduler is currently busy executing systems
    pub fn is_updating(&self) -> bool {
        self.system_coordinator.is_updating()
    }

    /// Access the high-performance spatial index
    pub fn spatial_index(&self) -> &OptimalSpatialIndex {
        self.subsystem_registry.spatial_index()
    }

    /// Get current world generation
    pub fn world_generation(&self) -> u32 {
        self.world_manager.world_generation()
    }

    /// Get reference to the query cache
    pub fn query_cache(&self) -> &GameCache {
        self.subsystem_registry.query_cache()
    }

    /// Get reference to the reload manager (debug builds only)
    #[cfg(debug_assertions)]
    pub fn reload_manager(&self) -> &Option<crate::core::reloader::ReloadManager> {
        self.subsystem_registry.reload_manager()
    }

    /// Get mutable reference to the reload manager (debug builds only)
    #[cfg(debug_assertions)]
    pub fn reload_manager_mut(&mut self) -> &mut Option<crate::core::reloader::ReloadManager> {
        self.subsystem_registry.reload_manager_mut()
    }

    /// Increment world generation and invalidate caches
    pub(super) fn increment_world_generation(&mut self) {
        self.world_manager.increment_world_generation();
    }

    /// Get the last update time
    pub fn last_update(&self) -> Instant {
        self.world_manager.last_update()
    }

    /// Update the last update time
    pub fn set_last_update(&mut self, time: Instant) {
        self.world_manager.set_last_update(time);
    }

    /// Get mutable reference to the scheduler
    pub fn scheduler_mut(&mut self) -> &mut crate::ecs::EcsScheduler {
        self.system_coordinator.scheduler_mut()
    }

    /// Get reference to the scheduler
    pub fn scheduler(&self) -> &crate::ecs::EcsScheduler {
        self.system_coordinator.scheduler()
    }

    /// Get current turn number
    pub fn get_turn(&self) -> u32 {
        self.world_manager.get_turn()
    }

    /// Check if the game is paused
    pub fn is_paused(&self) -> bool {
        self.world_manager.is_paused()
    }

    /// Set paused state
    pub fn set_paused(&mut self, paused: bool) {
        self.world_manager.set_paused(paused);
    }

    /// Update game time with delta time
    pub fn update_game_time(&mut self, delta: f32) {
        self.world_manager.update_game_time(delta);
    }

    /// Run all system stages in sequence
    pub fn run_system_stages(&mut self) {
        // Split the borrow to avoid simultaneous mutable borrows
        let GameWorld { world_manager, system_coordinator, .. } = self;
        system_coordinator.run_system_stages(world_manager.world_mut());
    }

    /// Validate hierarchical data periodically
    pub fn validate_hierarchy(&mut self, delta: f32) {
        // Split the borrow to avoid simultaneous borrows
        let GameWorld { world_manager, subsystem_registry, .. } = self;
        subsystem_registry.validate_hierarchy(world_manager.world_mut(), delta);
    }

    /// Process hot reload events (debug builds only)
    #[cfg(debug_assertions)]
    pub fn process_reload_events(&mut self) {
        self.subsystem_registry.process_reload_events();
    }

    // Export/import state now handled by serialization module
}

impl Default for GameWorld {
    fn default() -> Self {
        Self::new()
    }
}
