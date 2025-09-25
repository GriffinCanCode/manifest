//! Modular Climate Generation System
//!
//! Sophisticated climate generation using existing ECS, noise, and Lua systems.
//! Designed for extensibility and performance with minimal code duplication.

pub mod core;
pub mod patterns;
pub mod effects;
pub mod systems;
pub mod zig_ffi;

// Re-export public API
pub use core::{ClimateGenerator, ClimateGenConfig};
pub use patterns::{WindPatterns, OceanCurrents, SeasonalVariation};  
pub use effects::{OrographicEffects, ContinentalEffects};
pub use systems::{climate_generation_system, climate_interpolation_system};

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

/// Climate generation resource bundle for ECS
/// Note: Not a Bundle since these are Resources, not Components
pub struct ClimateBundle {
    pub generator: ClimateGenerator,
    pub wind_patterns: WindPatterns,
    pub ocean_currents: OceanCurrents,
    pub seasonal_variation: SeasonalVariation,
}

impl ClimateBundle {
    /// Create new climate bundle with default configuration
    pub fn new() -> crate::scripting::ScriptResult<Self> {
        Ok(Self {
            generator: ClimateGenerator::new(ClimateGenConfig::default())?,
            wind_patterns: WindPatterns::default(),
            ocean_currents: OceanCurrents::default(),
            seasonal_variation: SeasonalVariation::default(),
        })
    }
    
    /// Create with custom configuration
    pub fn with_config(config: ClimateGenConfig) -> crate::scripting::ScriptResult<Self> {
        Ok(Self {
            generator: ClimateGenerator::new(config)?,
            wind_patterns: WindPatterns::default(),
            ocean_currents: OceanCurrents::default(),
            seasonal_variation: SeasonalVariation::default(),
        })
    }
    
    /// Insert all climate resources into the world
    pub fn insert_into_world(self, world: &mut World) {
        world.insert_resource(self.generator);
        world.insert_resource(self.wind_patterns);
        world.insert_resource(self.ocean_currents);
        world.insert_resource(self.seasonal_variation);
    }
}

impl Default for ClimateBundle {
    fn default() -> Self {
        Self::new().expect("Failed to create default climate bundle")
    }
}

/// Climate generation stage for ECS scheduling
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum ClimateStage {
    /// Generate base climate data
    Generation,
    /// Apply climate patterns and effects
    Processing, 
    /// Interpolate climate between tiles
    Interpolation,
    /// Update seasonal variations
    Seasonal,
}
