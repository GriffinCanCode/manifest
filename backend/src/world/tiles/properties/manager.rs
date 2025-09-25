//! Tile Properties System Manager
//!
//! Provides the main TilePropertiesSystem manager and ECS systems
//! for managing tile properties with Lua scripting integration.

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, collections::HashMap, path::PathBuf};
use tracing::{debug, info, error};

use crate::{
    core::caching::{GameCache, GameCacheBuilder},
    world::tiles::{
        chunks::TileId,
        components::Tile,
    },
    scripting::{ScriptManager, ScriptResult, ScriptError},
};

use super::{
    terrain::EnhancedTerrainType,
    climate::EnhancedClimate,
    biome::{BiomeDefinition, Biome, BiomeSuitabilityCalculator},
    resources::{ResourceConfig, ResourceSpawner},
    movement::MovementCosts,
    defense::DefenseBonuses,
    culture::CulturalInfluence,
};

/// Main tile properties system manager
#[derive(Resource)]
pub struct TilePropertiesSystem {
    /// Lua script manager for scripted effects
    script_manager: Arc<ScriptManager>,
    /// Biome definitions loaded from RON files
    biome_definitions: HashMap<String, BiomeDefinition>,
    /// Resource configurations loaded from TOML files
    resource_config: ResourceConfig,
    /// Properties cache for performance
    properties_cache: GameCache,
    /// Cultural influence tracker
    cultural_influence: Arc<CulturalInfluence>,
}

// Manual Debug implementation since GameCache may not be Debug
impl std::fmt::Debug for TilePropertiesSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TilePropertiesSystem")
            .field("script_manager", &"Arc<ScriptManager>")
            .field("biome_definitions", &self.biome_definitions)
            .field("resource_config", &self.resource_config)
            .field("properties_cache", &"GameCache")
            .field("cultural_influence", &"Arc<CulturalInfluence>")
            .finish()
    }
}

impl Default for TilePropertiesSystem {
    fn default() -> Self {
        Self::new().expect("Failed to create TilePropertiesSystem")
    }
}

impl TilePropertiesSystem {
    /// Create new tile properties system
    pub fn new() -> ScriptResult<Self> {
        let script_manager = Arc::new(ScriptManager::new()?);
        let biome_definitions = Self::load_biome_definitions()?;
        let resource_config = Self::load_resource_config()?;
        let cultural_influence = Arc::new(CulturalInfluence::default());
        
        let properties_cache = GameCacheBuilder::new()
            .max_memory_mb(64) // 64MB cache for tile properties
            .default_ttl(std::time::Duration::from_secs(300)) // 5 minute TTL
            .build();

        let system = Self {
            script_manager,
            biome_definitions,
            resource_config,
            properties_cache,
            cultural_influence,
        };

        system.initialize_lua_scripts()?;
        
        info!("🌍 Tile Properties System initialized with {} biomes and {} resources", 
              system.biome_definitions.len(),
              system.resource_config.resources.len());
        
        Ok(system)
    }

    /// Create with custom parameters
    pub fn with_custom_config(
        biome_definitions: HashMap<String, BiomeDefinition>,
        resource_config: ResourceConfig,
    ) -> ScriptResult<Self> {
        let script_manager = Arc::new(ScriptManager::new()?);
        let cultural_influence = Arc::new(CulturalInfluence::default());
        
        let properties_cache = GameCacheBuilder::new()
            .max_memory_mb(64)
            .default_ttl(std::time::Duration::from_secs(300))
            .build();

        let system = Self {
            script_manager,
            biome_definitions,
            resource_config,
            properties_cache,
            cultural_influence,
        };

        system.initialize_lua_scripts()?;
        
        Ok(system)
    }

    /// Load biome definitions from RON files
    fn load_biome_definitions() -> ScriptResult<HashMap<String, BiomeDefinition>> {
        let config_path = PathBuf::from("backend/configs/biomes.ron");
        
        if !config_path.exists() {
            error!("Biome configuration file not found: {}", config_path.display());
            return Ok(Self::create_default_biomes());
        }
        
        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| ScriptError::FileNotFound { 
                path: config_path.clone() 
            })?;
        
        let definitions: HashMap<String, BiomeDefinition> = ron::from_str(&content)
            .map_err(|e| ScriptError::CompilationFailed {
                reason: format!("Failed to parse biome definitions from {}: {}", config_path.display(), e)
            })?;
        
        debug!("📚 Loaded {} biome definitions from {}", definitions.len(), config_path.display());
        Ok(definitions)
    }

    /// Create default biomes if file loading fails
    fn create_default_biomes() -> HashMap<String, BiomeDefinition> {
        let mut biomes = HashMap::new();
        
        biomes.insert("temperate_grassland".to_string(), BiomeDefinition {
            name: "Temperate Grassland".to_string(),
            description: "Moderate climate grassland suitable for agriculture".to_string(),
            climate_requirements: super::biome::ClimateRequirements {
                temperature_range: (10, 25),
                rainfall_range: (100, 300),
                elevation_range: Some((0.0, 1000.0)),
            },
            terrain_preferences: vec!["grassland".to_string(), "plains".to_string()],
            modifiers: super::biome::BiomeModifiers {
                movement_cost_multiplier: 1.0,
                defense_bonus: 0.0,
                agriculture_yield: 1.2,
                mining_yield: 1.0,
                population_capacity: 1.1,
            },
            special_resources: vec!["wheat".to_string(), "cattle".to_string()],
        });

        biomes.insert("desert".to_string(), BiomeDefinition {
            name: "Desert".to_string(),
            description: "Hot, arid region with limited resources".to_string(),
            climate_requirements: super::biome::ClimateRequirements {
                temperature_range: (25, 45),
                rainfall_range: (0, 100),
                elevation_range: None,
            },
            terrain_preferences: vec!["desert".to_string()],
            modifiers: super::biome::BiomeModifiers {
                movement_cost_multiplier: 1.5,
                defense_bonus: 0.1,
                agriculture_yield: 0.3,
                mining_yield: 1.2,
                population_capacity: 0.4,
            },
            special_resources: vec!["oil".to_string(), "gems".to_string()],
        });

        debug!("🏜️ Created {} default biome definitions", biomes.len());
        biomes
    }

    /// Load resource configuration from TOML files
    fn load_resource_config() -> ScriptResult<ResourceConfig> {
        let config_path = PathBuf::from("backend/configs/resources.toml");
        
        if !config_path.exists() {
            error!("Resource configuration file not found: {}", config_path.display());
            return Ok(ResourceConfig::default());
        }
        
        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| ScriptError::FileNotFound { 
                path: config_path.clone() 
            })?;
        
        let config: ResourceConfig = toml::from_str(&content)
            .map_err(|e| ScriptError::CompilationFailed {
                reason: format!("Failed to parse resource config from {}: {}", config_path.display(), e)
            })?;
        
        debug!("📦 Loaded {} resource definitions from {}", config.resources.len(), config_path.display());
        Ok(config)
    }

    /// Initialize Lua scripts for tile properties
    fn initialize_lua_scripts(&self) -> ScriptResult<()> {
        // Load tile properties script
        if let Err(e) = self.script_manager.load_script("tiles/properties.lua") {
            debug!("Could not load tiles/properties.lua: {}, using fallback", e);
        }
        
        // Load biome system script
        if let Err(e) = self.script_manager.load_script("tiles/biomes.lua") {
            debug!("Could not load tiles/biomes.lua: {}, using fallback", e);
        }
        
        info!("📜 Tile properties Lua scripts initialized (with fallbacks)");
        Ok(())
    }

    /// Calculate movement cost for a tile using Lua scripts
    pub fn calculate_movement_cost(&self, tile_id: TileId, terrain: EnhancedTerrainType, 
                                   modifiers: HashMap<String, f32>) -> ScriptResult<f32> {
        // Convert terrain enum to string for Lua
        let terrain_str: &'static str = terrain.into();
        
        // Create argument tuple for Lua function: (tile_id, terrain_str, modifiers_map)
        let modifiers_vec: Vec<(String, f32)> = modifiers.into_iter().collect();
        let args = (tile_id, terrain_str, modifiers_vec);
        
        // Try to call Lua function first
        match self.script_manager.call_function::<(u64, &str, Vec<(String, f32)>), f32>("calculate_movement_cost", args) {
            Ok(cost) => {
                debug!("Lua movement cost calculation succeeded for tile {}: {}", tile_id, cost);
                Ok(cost)
            },
            Err(lua_error) => {
                // Fallback to hardcoded values if Lua script fails
                debug!("Lua movement cost calculation failed for tile {} ({}), using fallback", tile_id, lua_error);
                Ok(Self::fallback_movement_cost(terrain, &modifiers_vec))
            }
        }
    }

    /// Fallback movement cost calculation
    fn fallback_movement_cost(terrain: EnhancedTerrainType, modifiers: &[(String, f32)]) -> f32 {
        let base_cost = match terrain {
            EnhancedTerrainType::Ocean => 3.0,
            EnhancedTerrainType::Grassland => 1.0,
            EnhancedTerrainType::Plains => 1.0,
            EnhancedTerrainType::Desert => 2.0,
            EnhancedTerrainType::Tundra => 2.0,
            EnhancedTerrainType::Snow => 2.5,
            EnhancedTerrainType::Forest => 2.0,
            EnhancedTerrainType::Jungle => 3.0,
            EnhancedTerrainType::Hills => 2.0,
            EnhancedTerrainType::Mountain => 4.0,
            EnhancedTerrainType::Swamp => 3.0,
            EnhancedTerrainType::Oasis => 1.0,
            EnhancedTerrainType::Volcano => 4.0,
            EnhancedTerrainType::Glacier => 3.0,
            EnhancedTerrainType::Beach => 1.5,
        };
        
        // Apply modifier effects
        let modifier_multiplier = modifiers.iter().fold(1.0, |acc, (_, &modifier)| {
            acc * (1.0 + modifier)
        });
        
        base_cost * modifier_multiplier
    }

    /// Determine biome for a tile using Lua scripts
    pub fn determine_biome(&self, climate: &EnhancedClimate, terrain: EnhancedTerrainType, 
                          elevation: f32) -> ScriptResult<Option<String>> {
        // Convert terrain enum to string for Lua
        let terrain_str: &'static str = terrain.into();
        
        // Create argument tuple for Lua function: (climate_data, terrain_str, elevation)
        let climate_data = (
            climate.temperature,
            climate.rainfall,
            climate.humidity,
            climate.wind_strength,
        );
        let args = (climate_data, terrain_str, elevation);
        
        // Try to call Lua function first
        match self.script_manager.call_function::<((i8, u16, u8, u8), &str, f32), String>("determine_biome", args) {
            Ok(biome) => {
                debug!("Lua biome determination succeeded: {} for terrain {} at elevation {}", biome, terrain_str, elevation);
                Ok(Some(biome))
            },
            Err(lua_error) => {
                // Fallback to rust-based biome determination
                debug!("Lua biome determination failed ({}), using fallback", lua_error);
                Ok(self.fallback_determine_biome(climate, terrain, elevation))
            }
        }
    }

    /// Fallback biome determination
    fn fallback_determine_biome(&self, climate: &EnhancedClimate, terrain: EnhancedTerrainType, elevation: f32) -> Option<String> {
        // Use the biome suitability calculator if we have definitions
        if !self.biome_definitions.is_empty() {
            let terrain_str: &'static str = terrain.into();
            BiomeSuitabilityCalculator::find_best_biome(
                climate.temperature,
                climate.rainfall,
                climate.humidity,
                elevation,
                terrain_str,
                &self.biome_definitions,
            ).map(|(biome_name, _)| biome_name)
        } else {
            // Simple hardcoded fallback
            let biome = match (terrain, climate.temperature, elevation) {
                (EnhancedTerrainType::Ocean, _, _) => "ocean",
                (EnhancedTerrainType::Grassland, temp, _) if temp > 20 => "tropical_grassland",
                (EnhancedTerrainType::Grassland, _, _) => "temperate_grassland",
                (EnhancedTerrainType::Plains, temp, _) if temp > 20 => "savanna", 
                (EnhancedTerrainType::Plains, _, _) => "temperate_plains",
                (EnhancedTerrainType::Desert, _, _) => "arid_desert",
                (EnhancedTerrainType::Tundra, _, _) => "arctic_tundra",
                (EnhancedTerrainType::Snow, _, _) => "polar",
                (EnhancedTerrainType::Forest, temp, _) if temp > 20 => "tropical_forest",
                (EnhancedTerrainType::Forest, _, _) => "temperate_forest",
                (EnhancedTerrainType::Jungle, _, _) => "tropical_rainforest",
                (EnhancedTerrainType::Hills, _, elev) if elev > 1000.0 => "highland",
                (EnhancedTerrainType::Hills, _, _) => "temperate_hills",
                (EnhancedTerrainType::Mountain, _, _) => "alpine",
                (EnhancedTerrainType::Swamp, _, _) => "wetland",
                (EnhancedTerrainType::Oasis, _, _) => "desert_oasis",
                (EnhancedTerrainType::Volcano, _, _) => "volcanic",
                (EnhancedTerrainType::Glacier, _, _) => "glacial",
                (EnhancedTerrainType::Beach, _, _) => "coastal",
            };
            
            Some(biome.to_string())
        }
    }

    /// Generate resources for a tile
    pub fn generate_tile_resources(&self, terrain: EnhancedTerrainType, biome_type: &str, seed: u64) -> Vec<String> {
        let terrain_str: &'static str = terrain.into();
        ResourceSpawner::generate_tile_resources(terrain_str, biome_type, &self.resource_config, seed)
    }

    /// Get cultural influence system
    pub fn cultural_influence(&self) -> Arc<CulturalInfluence> {
        self.cultural_influence.clone()
    }

    /// Access to script manager
    pub fn script_manager(&self) -> Arc<ScriptManager> {
        self.script_manager.clone()
    }

    /// Get biome definitions
    pub fn biome_definitions(&self) -> &HashMap<String, BiomeDefinition> {
        &self.biome_definitions
    }

    /// Get resource config
    pub fn resource_config(&self) -> &ResourceConfig {
        &self.resource_config
    }

    /// Reload configuration files
    pub fn reload_configs(&mut self) -> ScriptResult<()> {
        self.biome_definitions = Self::load_biome_definitions()?;
        self.resource_config = Self::load_resource_config()?;
        self.properties_cache.clear(); // Clear cache after reload
        
        info!("🔄 Reloaded tile properties configurations");
        Ok(())
    }
}

/// System for updating tile properties  
pub fn update_tile_properties(
    mut tiles_query: Query<(Entity, &TileId, &mut MovementCosts, &mut DefenseBonuses), Changed<Tile>>,
    properties_system: Res<TilePropertiesSystem>,
) {
    for (entity, tile_id, mut movement_costs, mut defense_bonuses) in tiles_query.iter_mut() {
        // Update calculated properties when tiles change
        movement_costs.current_cost = movement_costs.base_cost;
        defense_bonuses.calculate_total();
        
        // Could trigger Lua callbacks here for advanced effects
        debug!("Updated properties for tile {:?}", tile_id);
    }
}

/// System for cultural influence spread
pub fn update_cultural_influence(
    tiles_query: Query<&TileId>,
    properties_system: Res<TilePropertiesSystem>,
) {
    let influence = properties_system.cultural_influence();
    
    // Implement cultural influence spread logic here
    // This would calculate influence propagation between adjacent tiles
    
    debug!("Updated cultural influence for {} tiles", tiles_query.iter().len());
}

/// System for processing tile property changes
pub fn process_tile_property_changes(
    mut changed_tiles: Query<(Entity, &TileId, &mut Biome), Changed<EnhancedClimate>>,
    properties_system: Res<TilePropertiesSystem>,
) {
    for (entity, tile_id, mut biome) in changed_tiles.iter_mut() {
        // Recalculate biome when climate changes
        // This would need access to climate and terrain components
        debug!("Processing property changes for tile {:?}", tile_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_movement_cost() {
        let cost = TilePropertiesSystem::fallback_movement_cost(
            EnhancedTerrainType::Mountain,
            &[("road".to_string(), -0.5)]
        );
        assert_eq!(cost, 2.0); // 4.0 * (1 + (-0.5)) = 4.0 * 0.5 = 2.0
    }

    #[test]
    fn test_default_biomes_creation() {
        let biomes = TilePropertiesSystem::create_default_biomes();
        assert!(!biomes.is_empty());
        assert!(biomes.contains_key("temperate_grassland"));
        assert!(biomes.contains_key("desert"));
    }

    #[test]
    fn test_biome_determination_fallback() {
        let system = TilePropertiesSystem::new().expect("Failed to create system");
        let climate = EnhancedClimate::new(25, 50, 20);
        
        let biome = system.fallback_determine_biome(
            &climate,
            EnhancedTerrainType::Desert,
            200.0
        );
        
        assert!(biome.is_some());
        // Should be either from definitions or fallback
    }

    #[test]
    fn test_resource_generation() {
        let system = TilePropertiesSystem::new().expect("Failed to create system");
        
        let resources = system.generate_tile_resources(
            EnhancedTerrainType::Grassland,
            "temperate_grassland",
            12345
        );
        
        // Should return a list (may be empty if no config loaded)
        assert!(resources.len() <= 3); // Respects max resources
    }
}
