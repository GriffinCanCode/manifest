//! Modular Biome Generation System
//!
//! Smart biome determination using climate data and Lua rule systems.
//! Integrates heavily with existing ECS and climate systems.

pub mod core;
pub mod rules; 
pub mod systems;
pub mod transitions;

// Re-export public API
pub use core::{BiomeGenerator, BiomeGenConfig};
pub use rules::{LuaBiomeRules, BiomeDecisionTree};
pub use systems::{biome_generation_system, biome_transition_system, biome_validation_system};
pub use transitions::{BiomeTransitionManager, TransitionType};

use bevy_ecs::prelude::*;

/// Biome generation resource bundle for ECS
#[derive(Bundle)]
pub struct BiomeBundle {
    pub generator: BiomeGenerator,
    pub lua_rules: LuaBiomeRules,
    pub transition_manager: BiomeTransitionManager,
}

impl BiomeBundle {
    /// Create new biome bundle with default configuration
    pub fn new() -> crate::scripting::ScriptResult<Self> {
        Ok(Self {
            generator: BiomeGenerator::new(BiomeGenConfig::default())?,
            lua_rules: LuaBiomeRules::new()?,
            transition_manager: BiomeTransitionManager::default(),
        })
    }
}

impl Default for BiomeBundle {
    fn default() -> Self {
        Self::new().expect("Failed to create default biome bundle")
    }
}

/// Biome generation stages for ECS scheduling
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum BiomeStage {
    /// Generate base biomes from climate data
    Generation,
    /// Apply Lua rules and decision trees
    RuleProcessing,
    /// Handle biome transitions and borders
    Transitions,
    /// Validate biome assignments
    Validation,
}
