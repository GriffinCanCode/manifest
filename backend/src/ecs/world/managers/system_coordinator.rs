//! System coordination and execution manager
//!
//! Handles ECS system configuration, scheduling, and execution coordination.
//! Manages the parallel scheduler and system stage execution.

use bevy_ecs::prelude::*;
use tracing::{info, error};

use crate::core::{Stage, SchedulerMetrics};
use crate::ecs::{
    systems::*,
    EcsScheduler,
    spatial::*,
    ResourceAccess,
};

/// Coordinates system configuration and execution
#[derive(Debug)]
pub struct SystemCoordinator {
    /// Parallel scheduler for running systems efficiently
    scheduler: EcsScheduler,
}

impl SystemCoordinator {
    /// Create a new system coordinator with configured systems
    pub fn new() -> Result<Self, String> {
        // Create parallel scheduler with optimal thread count
        let scheduler = EcsScheduler::new(None)
            .map_err(|e| format!("Failed to create ECS scheduler: {}", e))?;

        Ok(Self { scheduler })
    }

    /// Configure all game systems
    pub fn configure_systems(&mut self, world: &mut World) {
        self.configure_core_systems(world);
        self.configure_spatial_systems(world);
        self.configure_world_generation_systems(world);
        self.configure_change_detection_systems(world);
    }

    /// Configure core ECS systems
    fn configure_core_systems(&mut self, world: &mut World) {
        configure_parallel_systems(&mut self.scheduler, world);
        info!("⚙️ Core parallel systems configured");
    }

    /// Configure spatial indexing systems
    fn configure_spatial_systems(&mut self, world: &mut World) {
        self.scheduler.add_system_with_accesses(
            Stage::PreUpdate,
            "incremental_spatial_sync".to_string(),
            incremental_spatial_sync,
            vec![
                ResourceAccess::write::<OptimalSpatialIndex>(),
                // Component queries (Position, Owner, Movement) handled by Bevy's system
                // Commands handled by Bevy's system
            ],
            world,
        );
        
        self.scheduler.add_system_with_accesses(
            Stage::PostUpdate,
            "spatial_cache_maintenance".to_string(),
            spatial_cache_maintenance,
            vec![
                ResourceAccess::write::<OptimalSpatialIndex>(),
                // Commands handled by Bevy's system  
                // SpatialSyncNeeded resource access handled by Bevy's system
            ],
            world,
        );
        
        // Spatial rebuild check system for performance optimization
        self.scheduler.add_system_with_accesses(
            Stage::Cleanup,
            "full_spatial_rebuild_check".to_string(),
            full_spatial_rebuild_check,
            vec![
                ResourceAccess::write::<OptimalSpatialIndex>(),
                ResourceAccess::read::<crate::ecs::resources::GameTime>(),
            ],
            world,
        );
        
        info!("🗺️ Spatial indexing systems configured");
    }
    
    /// Configure world generation systems (climate and biome)
    fn configure_world_generation_systems(&mut self, world: &mut World) {
        // Add climate generation systems
        self.scheduler.add_system_with_accesses(
            Stage::Update,
            "climate_generation".to_string(),
            crate::world::generation::climate::systems::climate_generation_system,
            vec![
                ResourceAccess::read::<crate::world::generation::climate::ClimateGenerator>(),
                ResourceAccess::read::<crate::world::generation::noise::NoiseGenerator>(),
                ResourceAccess::read::<crate::core::scheduler::Scheduler>(),
                // Entity queries handled by Bevy's system
            ],
            world,
        );
        
        self.scheduler.add_system_with_accesses(
            Stage::Update,
            "climate_interpolation".to_string(),
            crate::world::generation::climate::systems::climate_interpolation_system,
            vec![], // Bevy handles component queries
            world,
        );
        
        self.scheduler.add_system_with_accesses(
            Stage::Update,
            "seasonal_climate".to_string(),
            crate::world::generation::climate::systems::seasonal_climate_system,
            vec![
                ResourceAccess::write::<crate::world::generation::climate::SeasonalVariation>(),
                ResourceAccess::read::<crate::ecs::resources::GameTime>(),
            ],
            world,
        );
        
        // Add biome generation systems
        self.scheduler.add_system_with_accesses(
            Stage::Update,
            "biome_generation".to_string(),
            crate::world::generation::biomes::systems::biome_generation_system,
            vec![
                ResourceAccess::read::<crate::world::generation::biomes::BiomeGenerator>(),
                ResourceAccess::read::<crate::core::scheduler::Scheduler>(),
            ],
            world,
        );
        
        self.scheduler.add_system_with_accesses(
            Stage::Update,
            "biome_transitions".to_string(),
            crate::world::generation::biomes::systems::biome_transition_system,
            vec![
                ResourceAccess::write::<crate::world::generation::biomes::BiomeTransitionManager>(),
            ],
            world,
        );
        
        self.scheduler.add_system_with_accesses(
            Stage::PostUpdate,
            "biome_validation".to_string(),
            crate::world::generation::biomes::systems::biome_validation_system,
            vec![
                ResourceAccess::read::<crate::world::generation::biomes::BiomeGenerator>(),
                ResourceAccess::read::<crate::world::generation::biomes::LuaBiomeRules>(),
            ],
            world,
        );
        
        info!("🌡️🌿 Climate and biome generation systems configured");
    }

    /// Configure change detection systems
    fn configure_change_detection_systems(&mut self, world: &mut World) {
        configure_change_detection(&mut self.scheduler, world);
        info!("📊 Change detection systems configured");
    }

    /// Run all system stages in sequence
    pub fn run_system_stages(&mut self, world: &mut World) {
        let stages = [Stage::PreUpdate, Stage::Update, Stage::PostUpdate, Stage::Cleanup];
        for stage in stages {
            let stage_name = format!("{:?}", stage); // Store stage name before moving
            if let Err(errors) = self.scheduler.run_stage(stage, world) {
                error!("System execution errors in stage {}: {:?}", stage_name, errors);
            }
        }
    }

    /// Get scheduler performance metrics
    pub fn scheduler_metrics(&self) -> SchedulerMetrics {
        self.scheduler.metrics()
    }

    /// Check if the scheduler is currently busy executing systems
    pub fn is_updating(&self) -> bool {
        self.scheduler.is_busy()
    }

    /// Get reference to the scheduler
    pub fn scheduler(&self) -> &EcsScheduler {
        &self.scheduler
    }

    /// Get mutable reference to the scheduler
    pub fn scheduler_mut(&mut self) -> &mut EcsScheduler {
        &mut self.scheduler
    }
}

impl Default for SystemCoordinator {
    fn default() -> Self {
        Self::new().expect("Failed to create default SystemCoordinator")
    }
}
