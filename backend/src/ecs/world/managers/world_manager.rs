//! Core world operations manager
//!
//! Handles ECS world management, resource management, and basic world state operations.
//! Delegates specialized operations to focused subsystems.

use bevy_ecs::prelude::*;
use std::time::Instant;
use tracing::{info, warn};

use crate::ecs::{
    resources::*,
    hierarchy::HierarchyQueries,
};

/// Manages core ECS world operations and resource initialization
#[derive(Debug)]
pub struct WorldManager {
    /// The ECS world containing all entities, components, and resources
    world: World,
    /// World generation for cache invalidation
    world_generation: u32,
    /// Last update time for delta time calculation
    last_update: Instant,
}

impl WorldManager {
    /// Create a new world manager with initialized resources
    pub fn new() -> Self {
        let mut world = World::new();
        
        // Initialize core game resources
        Self::initialize_core_resources(&mut world);
        Self::initialize_world_generation_resources(&mut world);
        
        Self {
            world,
            world_generation: 0,
            last_update: Instant::now(),
        }
    }

    /// Initialize core game resources
    fn initialize_core_resources(world: &mut World) {
        world.insert_resource(GameTime::default());
        world.insert_resource(Players::default());
        world.insert_resource(Camera::default());
        world.insert_resource(Selection::default());
        world.insert_resource(HierarchyQueries::new());
        
        info!("🎮 Core game resources initialized");
    }

    /// Initialize world generation resources
    fn initialize_world_generation_resources(world: &mut World) {
        // Initialize tile properties system
        match crate::world::tiles::TilePropertiesSystem::new() {
            Ok(properties_system) => {
                world.insert_resource(properties_system);
                info!("🌍 Tile Properties System initialized successfully");
            }
            Err(e) => {
                warn!("Failed to initialize Tile Properties System: {}", e);
            }
        }
        
        // Initialize world generation system
        let cache = crate::core::caching::GameCacheBuilder::new()
            .max_memory_mb(128)
            .default_ttl(std::time::Duration::from_secs(300))
            .build();
        let world_gen_config = crate::world::generation::WorldGenConfig::default();
        match crate::world::generation::WorldGenerator::new(world_gen_config, cache) {
            Ok(world_generator) => {
                world.insert_resource(world_generator);
                info!("🌍 World Generator initialized successfully");
            }
            Err(e) => {
                warn!("Failed to initialize World Generator: {}", e);
            }
        }
        
        // Initialize climate and biome resources
        world.insert_resource(crate::world::generation::climate::WindPatterns::default());
        world.insert_resource(crate::world::generation::climate::OceanCurrents::default());  
        world.insert_resource(crate::world::generation::climate::SeasonalVariation::default());
        world.insert_resource(crate::world::generation::biomes::BiomeTransitionManager::default());
        
        // Initialize climate bundle
        match crate::world::generation::climate::ClimateBundle::new() {
            Ok(climate_bundle) => {
                world.insert_resource(climate_bundle.generator);
                world.insert_resource(climate_bundle.wind_patterns);
                world.insert_resource(climate_bundle.ocean_currents);
                world.insert_resource(climate_bundle.seasonal_variation);
                info!("🌡️ Climate systems initialized successfully");
            }
            Err(e) => {
                warn!("Failed to initialize Climate Bundle: {}", e);
            }
        }
        
        // Initialize biome bundle
        match crate::world::generation::biomes::BiomeBundle::new() {
            Ok(biome_bundle) => {
                world.insert_resource(biome_bundle.generator);
                world.insert_resource(biome_bundle.lua_rules);
                world.insert_resource(biome_bundle.transition_manager);
                info!("🌿 Biome systems initialized successfully");
            }
            Err(e) => {
                warn!("Failed to initialize Biome Bundle: {}", e);
            }
        }
    }

    /// Get a reference to the ECS world for external access
    pub fn world(&self) -> &World {
        &self.world
    }

    /// Get a mutable reference to the ECS world for external modifications
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    /// Get current world generation
    pub fn world_generation(&self) -> u32 {
        self.world_generation
    }

    /// Increment world generation and invalidate caches
    pub fn increment_world_generation(&mut self) {
        self.world_generation += 1;
    }

    /// Get the last update time
    pub fn last_update(&self) -> Instant {
        self.last_update
    }

    /// Update the last update time
    pub fn set_last_update(&mut self, time: Instant) {
        self.last_update = time;
    }

    /// Update game time with delta time
    pub fn update_game_time(&mut self, delta: f32) {
        // Update game time through simulation state
        let simulation_state = self.world.get_resource::<crate::core::time::SimulationState>().cloned();
        if let (Some(mut game_time), Some(simulation_state)) = (
            self.world.get_resource_mut::<GameTime>(),
            simulation_state
        ) {
            game_time.update(delta, &simulation_state);
        }
    }

    /// Get current turn number
    pub fn get_turn(&self) -> u32 {
        self.world.get_resource::<GameTime>()
            .map(|game_time| game_time.turn)
            .unwrap_or(1)
    }

    /// Check if the game is paused
    pub fn is_paused(&self) -> bool {
        self.world.get_resource::<GameTime>()
            .map(|game_time| game_time.paused)
            .unwrap_or(false)
    }

    /// Set paused state
    pub fn set_paused(&mut self, paused: bool) {
        if let Some(mut game_time) = self.world.get_resource_mut::<GameTime>() {
            game_time.paused = paused;
        }
    }

    /// Clear all entities while preserving resources
    pub fn clear_entities(&mut self) {
        self.world.clear_entities();
        self.increment_world_generation();
    }

    /// Get hierarchy queries resource
    pub fn hierarchy_queries(&self) -> Option<&HierarchyQueries> {
        self.world.get_resource::<HierarchyQueries>()
    }
}

impl Default for WorldManager {
    fn default() -> Self {
        Self::new()
    }
}
