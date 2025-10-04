//! Procedural World Generation Systems
//!
//! Comprehensive procedural generation using Rust + Zig SIMD optimization
//! for high-performance, deterministic world creation with extensible
//! noise generation, tectonic simulation, and biome generation.

pub mod noise;
pub mod tectonics;
pub mod hydrology;
pub mod resources;
pub mod climate;
pub mod biomes;

// Re-export commonly used types with explicit imports to avoid conflicts

// From noise module
pub use noise::{
    NoiseConfig, NoiseGenerator, NoiseType, NoiseCache,
    Interpolation, NoiseQuality, SimplexConfig,
    PerlinConfig, VoronoiConfig, WorleyConfig, 
    FbmConfig, DomainWarpConfig, RidgeConfig,
    ScheduledNoiseGenerator, NoiseResource,
};

// From noise module (with aliases for conflicting names)
pub use noise::{
    types as noise_types,
    core as noise_core,
};

// From tectonics module  
pub use tectonics::{
    TectonicSimulator, TectonicsConfig,
};

// From tectonics module (with alias for conflicting name)
pub use tectonics::{
    zig_ffi as tectonics_zig_ffi,
};

// From hydrology module
pub use hydrology::*;

// From resources module
pub use resources::{
    ResourceDistributionSystem, ResourceDiscoverySystem, ResourceDepletionSystem,
};

// From resources module (with aliases for conflicting names)  
pub use resources::{
    types as resource_types,
    core as resource_core,
};

// From climate module
pub use climate::{
    ClimateGenerator, ClimateGenConfig, WindPatterns, OceanCurrents, 
    SeasonalVariation, OrographicEffects, ContinentalEffects,
    ClimateBundle,
};

// From climate module (with alias for conflicting name)
pub use climate::{
    systems as climate_systems,
};

// From biomes module
pub use biomes::{
    BiomeGenerator, BiomeGenConfig, LuaBiomeRules, BiomeDecisionTree,
    BiomeTransitionManager, TransitionType, BiomeBundle, BiomeStage,
};

// From biomes module (with alias for conflicting name)
pub use biomes::{
    systems as biome_systems,
};


use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use crate::core::caching::GameCache;

/// World generation configuration
#[derive(Debug, Clone, Serialize, Deserialize, Resource)]
pub struct WorldGenConfig {
    /// Random seed for deterministic generation
    pub seed: u64,
    /// World size in chunks
    pub world_size: (u32, u32),
    /// Enable SIMD optimizations where available
    pub use_simd: bool,
    /// Noise generation settings
    pub noise_config: noise::NoiseConfig,
    /// Tectonic simulation settings
    pub tectonics_config: tectonics::TectonicsConfig,
    /// Climate generation settings
    pub climate_config: climate::ClimateGenConfig,
    /// Biome generation settings
    pub biome_config: biomes::BiomeGenConfig,
}

impl Default for WorldGenConfig {
    fn default() -> Self {
        Self {
            seed: 12345,
            world_size: (256, 256),
            use_simd: true,
            noise_config: noise::NoiseConfig::default(),
            tectonics_config: tectonics::TectonicsConfig::default(),
            climate_config: climate::ClimateGenConfig::default(),
            biome_config: biomes::BiomeGenConfig::default(),
        }
    }
}

/// World generation resource for ECS
#[derive(Debug, Resource)]
pub struct WorldGenerator {
    config: WorldGenConfig,
    cache: GameCache,
    noise_generator: noise::NoiseGenerator,
    tectonic_simulator: tectonics::TectonicSimulator,
    climate_generator: climate::ClimateGenerator,
    biome_generator: biomes::BiomeGenerator,
}

impl WorldGenerator {
    /// Create new world generator with configuration
    pub fn new(config: WorldGenConfig, cache: GameCache) -> Result<Self, crate::core::scheduler::SchedulerError> {
        let noise_generator = noise::NoiseGenerator::new(&config.noise_config);
        let tectonic_simulator = tectonics::TectonicSimulator::new(
            config.tectonics_config.clone(), 
            cache.clone()
        );
        let climate_generator = climate::ClimateGenerator::new(config.climate_config.clone())
            .map_err(|e| crate::core::scheduler::SchedulerError::TaskFailed(format!("Climate generator creation failed: {}", e)))?;
        let biome_generator = biomes::BiomeGenerator::new(config.biome_config.clone())
            .map_err(|e| crate::core::scheduler::SchedulerError::TaskFailed(format!("Biome generator creation failed: {}", e)))?;
        
        Ok(Self {
            config,
            cache,
            noise_generator,
            tectonic_simulator,
            climate_generator,
            biome_generator,
        })
    }

    /// Generate height value for tile coordinates
    pub fn generate_height(&self, x: f64, y: f64) -> f32 {
        self.noise_generator.sample_height(x, y)
    }

    /// Generate temperature value for tile coordinates
    pub fn generate_temperature(&self, x: f64, y: f64) -> f32 {
        self.noise_generator.sample_temperature(x, y)
    }

    /// Generate moisture value for tile coordinates  
    pub fn generate_moisture(&self, x: f64, y: f64) -> f32 {
        self.noise_generator.sample_moisture(x, y)
    }
    
    /// Generate complete tectonic simulation
    pub fn generate_tectonics(&mut self) -> Result<tectonics::TectonicResult, crate::core::scheduler::SchedulerError> {
        self.tectonic_simulator.generate_tectonics()
    }
    
    /// Sample tectonic influence at coordinates
    pub fn sample_tectonic_influence(&self, x: f64, y: f64, tectonic_result: &tectonics::TectonicResult) -> tectonics::TectonicInfluence {
        self.tectonic_simulator.sample_tectonic_influence(x, y, tectonic_result)
    }

    /// Generate climate data for coordinates
    pub fn generate_climate(&self, tile_id: crate::world::tiles::chunks::TileId, x: f64, y: f64, elevation: f32) -> crate::scripting::ScriptResult<crate::world::tiles::properties::EnhancedClimate> {
        self.climate_generator.generate_climate_sync(tile_id, x, y, elevation, &self.noise_generator)
    }
    
    /// Generate biome data for coordinates
    pub fn generate_biome(
        &self, 
        tile_id: crate::world::tiles::chunks::TileId,
        climate: &crate::world::tiles::properties::EnhancedClimate,
        terrain: &crate::world::tiles::properties::EnhancedTerrainType,
        elevation: &crate::world::tiles::properties::Elevation,
    ) -> crate::scripting::ScriptResult<crate::world::tiles::properties::Biome> {
        self.biome_generator.generate_biome_sync(tile_id, climate, terrain, elevation)
    }
}
