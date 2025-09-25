//! Core Biome Generation
//!
//! Leverages existing climate data and tile properties for sophisticated biome assignment.

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, instrument};

use crate::{
    core::{
        scheduler::{Scheduler, SchedulerError}, 
        caching::{GameCache, CacheConfig, CacheKey, CachePriority},
        hashing::{HashStrategies, FastHashMap},
    },
    scripting::{ScriptManager, ScriptResult, LuaEventData, LuaEventValue},
    world::tiles::{
        chunks::TileId,
        properties::{
            Biome, BiomeDefinition, BiomeSuitabilityCalculator,
            EnhancedClimate, EnhancedTerrainType, Elevation,
        },
    },
};

/// Biome generation configuration
#[derive(Debug, Clone, Serialize, Deserialize, Resource)]
pub struct BiomeGenConfig {
    /// Enable Lua-based biome rules
    pub use_lua_rules: bool,
    /// Minimum suitability threshold for biome assignment
    pub min_suitability_threshold: f32,
    /// Use climate-based biome transitions
    pub enable_transitions: bool,
    /// Cache size for biome calculations
    pub cache_size: usize,
}

impl Default for BiomeGenConfig {
    fn default() -> Self {
        Self {
            use_lua_rules: true,
            min_suitability_threshold: 0.3,
            enable_transitions: true,
            cache_size: 1000,
        }
    }
}

/// Core biome generator - integrates with existing systems
#[derive(Debug, Resource)]
pub struct BiomeGenerator {
    config: BiomeGenConfig,
    biome_definitions: FastHashMap<String, BiomeDefinition>,
    script_manager: Arc<ScriptManager>,
    cache: GameCache,
}

impl BiomeGenerator {
    /// Create new biome generator using existing systems
    pub fn new(config: BiomeGenConfig) -> ScriptResult<Self> {
        let biome_definitions = Self::load_biome_definitions()?;
        let script_manager = Arc::new(ScriptManager::new()?);
        let cache = GameCache::new(CacheConfig::default());
        
        let mut generator = Self {
            config,
            biome_definitions,
            script_manager,
            cache,
        };
        
        if generator.config.use_lua_rules {
            generator.load_biome_scripts()?;
        }
        
        info!("🌿 Biome Generator initialized with {} biomes (Lua: {})", 
              generator.biome_definitions.len(), generator.config.use_lua_rules);
        
        Ok(generator)
    }
    
    /// Load biome definitions from RON files
    fn load_biome_definitions() -> ScriptResult<FastHashMap<String, BiomeDefinition>> {
        let mut definitions = FastHashMap::default();
        
        // Load built-in biome definitions
        let builtin_biomes = [
            ("tropical_rainforest", Self::create_tropical_rainforest()),
            ("tropical_grassland", Self::create_tropical_grassland()),
            ("temperate_forest", Self::create_temperate_forest()),
            ("temperate_grassland", Self::create_temperate_grassland()),
            ("desert", Self::create_desert()),
            ("tundra", Self::create_tundra()),
            ("taiga", Self::create_taiga()),
            ("mediterranean", Self::create_mediterranean()),
            ("savanna", Self::create_savanna()),
            ("alpine", Self::create_alpine()),
        ];
        
        for (name, biome) in builtin_biomes {
            definitions.insert(name.to_string(), biome);
        }
        
        // TODO: Load custom biomes from RON files in configs/biomes/
        // This would integrate with the existing RON configuration system
        
        Ok(definitions)
    }
    
    /// Load Lua biome scripts
    #[instrument(skip(self))]
    fn load_biome_scripts(&self) -> ScriptResult<()> {
        let scripts = [
            "biomes/biome_determination.lua",
            "biomes/transitions.lua", 
            "biomes/special_biomes.lua",
            "biomes/validation.lua",
        ];
        
        for script in &scripts {
            if let Err(e) = self.script_manager.load_script(script) {
                debug!("Optional biome script not found: {} ({})", script, e);
            }
        }
        
        Ok(())
    }
    
    /// Generate biome from climate and terrain data with deterministic hashing
    #[instrument(skip(self))]
    pub async fn generate_biome(
        &self,
        tile_id: TileId,
        climate: &EnhancedClimate,
        terrain: &EnhancedTerrainType,
        elevation: &Elevation,
    ) -> ScriptResult<Biome> {
        // Create deterministic cache key using HashStrategies
        let cache_key = HashStrategies::combine_hashes(&[
            HashStrategies::hash_string(&format!("{:?}", tile_id)),
            HashStrategies::hash_bytes(&climate.temperature.to_ne_bytes()),
            HashStrategies::hash_bytes(&climate.rainfall.to_ne_bytes()),
            HashStrategies::hash_bytes(&climate.humidity.to_ne_bytes()),
            HashStrategies::hash_bytes(&elevation.final_elevation.to_ne_bytes()),
            HashStrategies::hash_string(&terrain.to_string()),
        ]);
        
        // Check cache first for deterministic results
        let cache_key_obj = CacheKey::Custom(cache_key.to_string());
        if let Ok(Some(cached_biome)) = self.cache.get::<Biome>(&cache_key_obj).await {
            return Ok(cached_biome);
        }
        
        // Find best matching biome using existing suitability calculator
        let best_match = BiomeSuitabilityCalculator::find_best_biome(
            climate.temperature,
            climate.rainfall,
            climate.humidity,
            elevation.final_elevation,
            &terrain.to_string(),
            &self.biome_definitions,
        );
        
        let (biome_type, suitability) = best_match
            .unwrap_or_else(|| ("temperate_grassland".to_string(), 0.5));
        
        // Apply minimum threshold
        if suitability < self.config.min_suitability_threshold {
            return Ok(Biome::new("barren".to_string(), suitability));
        }
        
        // Get biome definition for modifiers
        let biome_def = self.biome_definitions.get(&biome_type)
            .ok_or_else(|| crate::scripting::ScriptError::ExecutionFailed { 
                reason: format!("Biome definition not found: {}", biome_type)
            })?;
        
        let mut biome = Biome::with_modifiers(biome_type.clone(), suitability, biome_def.modifiers.clone());
        
        // Apply Lua rules if enabled
        let final_biome = if self.config.use_lua_rules {
            self.apply_lua_biome_rules(tile_id, biome, climate, terrain, elevation)?
        } else {
            biome
        };
        
        // Cache result for performance and consistency
        let _ = self.cache.set(cache_key_obj, final_biome.clone(), CachePriority::Normal).await;
        
        Ok(final_biome)
    }
    
    /// Apply Lua biome rules
    #[instrument(skip(self, biome, climate, terrain, elevation))]
    fn apply_lua_biome_rules(
        &self,
        tile_id: TileId,
        mut biome: Biome,
        climate: &EnhancedClimate,
        terrain: &EnhancedTerrainType,
        elevation: &Elevation,
    ) -> ScriptResult<Biome> {
        use crate::scripting::LuaEventData;
        
        let mut event_data = LuaEventData {
            event_type: "biome_generation".to_string(),
            data: std::collections::HashMap::default(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            source: Some("biome_generator".to_string()),
        };
        event_data.data.insert("tile_id".to_string(), LuaEventValue::String(format!("{:?}", tile_id)));
        event_data.data.insert("biome_type".to_string(), LuaEventValue::String(biome.biome_type.clone()));
        event_data.data.insert("suitability".to_string(), LuaEventValue::Number(biome.suitability_score as f64));
        event_data.data.insert("temperature".to_string(), LuaEventValue::Integer(climate.temperature as i64));
        event_data.data.insert("rainfall".to_string(), LuaEventValue::Integer(climate.rainfall as i64));
        event_data.data.insert("humidity".to_string(), LuaEventValue::Integer(climate.humidity as i64));
        event_data.data.insert("elevation".to_string(), LuaEventValue::String(elevation.final_elevation.to_string()));
        event_data.data.insert("terrain_type".to_string(), LuaEventValue::String(terrain.to_string()));
        event_data.data.insert("climate_zone".to_string(), LuaEventValue::String(climate.interpolated.climate_zone.clone()));
        
        // Apply biome determination rules
        if let Ok(results) = self.script_manager.trigger_event("biome_determination", &event_data) {
            for result in results {
                if let Some((key, value)) = result.split_once(':') {
                    match key {
                        "biome_type" => biome.biome_type = value.to_string(),
                        "suitability" => {
                            if let Ok(suit) = value.parse::<f32>() {
                                biome.suitability_score = suit.clamp(0.0, 1.0);
                            }
                        }
                        "modifier" => {
                            // Parse modifier changes from Lua
                            if let Some((mod_type, mod_value)) = value.split_once('=') {
                                if let Ok(val) = mod_value.parse::<f32>() {
                                    match mod_type {
                                        "movement_cost" => biome.modifiers.movement_cost_multiplier = val,
                                        "defense_bonus" => biome.modifiers.defense_bonus = val,
                                        "agriculture" => biome.modifiers.agriculture_yield = val,
                                        "mining" => biome.modifiers.mining_yield = val,
                                        "population" => biome.modifiers.population_capacity = val,
                                        _ => {}
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        
        // Apply special biome rules
        if let Ok(results) = self.script_manager.trigger_event("special_biomes", &event_data) {
            for result in results {
                if result.starts_with("special:") {
                    biome.biome_type = result[8..].to_string(); // Remove "special:" prefix
                }
            }
        }
        
        Ok(biome)
    }
    
    /// Batch generate biomes using scheduler with deterministic processing
    pub async fn generate_biomes_batch(
        &self,
        mut tiles: Vec<(TileId, EnhancedClimate, EnhancedTerrainType, Elevation)>,
        scheduler: &Scheduler,
    ) -> Result<FastHashMap<TileId, Biome>, String> {
        use crate::core::scheduler::{TaskBatch, Stage, Resource as SchedulerResource};
        use std::sync::{Arc, Mutex};
        
        // Sort tiles by TileId for deterministic processing order
        tiles.sort_by_key(|(tile_id, _, _, _)| format!("{:?}", tile_id));
        
        let results = Arc::new(Mutex::new(FastHashMap::default()));
        let chunk_size = 64.max(tiles.len() / scheduler.active_count().max(1));
        let mut batch = TaskBatch::new(Stage::Update);
        
        // Create deterministic batch hash for reproducible results
        let batch_hash = HashStrategies::hash_string(&format!("biome_batch_{}", 
            HashStrategies::combine_hashes(&tiles.iter()
                .map(|(tile_id, _, _, _)| HashStrategies::hash_string(&format!("{:?}", tile_id)))
                .collect::<Vec<_>>())
        ));
        
        for (chunk_idx, chunk) in tiles.chunks(chunk_size).enumerate() {
            let results_clone = Arc::clone(&results);
            let chunk_data = chunk.to_vec();
            let generator_ptr = self as *const Self;
            
            batch.add_task_with_resources(
                format!("biome_batch_{}_{}", batch_hash, chunk_idx),
                vec![SchedulerResource::write::<FastHashMap<TileId, Biome>>()],
                move || -> Result<(), crate::core::scheduler::SchedulerError> {
                    let generator = unsafe { &*generator_ptr };
                    let mut local_results = FastHashMap::default();
                    
                    // Process chunk in deterministic order using tokio runtime
                    let rt = tokio::runtime::Handle::try_current()
                        .unwrap_or_else(|_| tokio::runtime::Runtime::new().unwrap().handle().clone());
                        
                    for (tile_id, climate, terrain, elevation) in chunk_data {
                        match rt.block_on(generator.generate_biome(tile_id, &climate, &terrain, &elevation)) {
                            Ok(biome) => {
                                local_results.insert(tile_id, biome);
                            }
                            Err(e) => {
                                tracing::warn!("Failed to generate biome for {:?}: {}", tile_id, e);
                            }
                        }
                    }
                    
                    results_clone.lock().unwrap().extend(local_results);
                    Ok(())
                }
            );
        }
        
        scheduler.add_batch(batch);
        scheduler.run_stage(Stage::Update).map_err(|errors| {
            errors.into_iter().next().unwrap_or_else(|| SchedulerError::TaskFailed("Unknown scheduler error".to_string()))
        })?;
        
        Ok(results.lock().unwrap().clone())
    }
    
    /// Get biome definitions
    pub fn biome_definitions(&self) -> &FastHashMap<String, BiomeDefinition> {
        &self.biome_definitions
    }
    
    /// Get configuration
    pub fn config(&self) -> &BiomeGenConfig {
        &self.config
    }
    
    // Built-in biome definitions - compact but comprehensive
    fn create_tropical_rainforest() -> BiomeDefinition {
        BiomeDefinition {
            name: "Tropical Rainforest".to_string(),
            description: "Dense, humid forest with high biodiversity".to_string(),
            climate_requirements: crate::world::tiles::properties::ClimateRequirements {
                temperature_range: (20, 35),
                rainfall_range: (200, 500),
                elevation_range: Some((0.0, 1000.0)),
            },
            terrain_preferences: vec!["jungle".to_string(), "forest".to_string()],
            modifiers: crate::world::tiles::properties::BiomeModifiers {
                movement_cost_multiplier: 2.5,
                defense_bonus: 0.2,
                agriculture_yield: 0.6,
                mining_yield: 0.3,
                population_capacity: 0.8,
            },
            special_resources: vec!["exotic_wood".to_string(), "medicinal_plants".to_string()],
        }
    }
    
    fn create_tropical_grassland() -> BiomeDefinition {
        BiomeDefinition {
            name: "Tropical Grassland".to_string(),
            description: "Hot, seasonal grasslands with scattered trees".to_string(),
            climate_requirements: crate::world::tiles::properties::ClimateRequirements {
                temperature_range: (18, 30),
                rainfall_range: (50, 200),
                elevation_range: None,
            },
            terrain_preferences: vec!["grassland".to_string(), "plains".to_string()],
            modifiers: crate::world::tiles::properties::BiomeModifiers {
                movement_cost_multiplier: 1.0,
                defense_bonus: 0.0,
                agriculture_yield: 1.2,
                mining_yield: 0.8,
                population_capacity: 1.0,
            },
            special_resources: vec!["wildlife".to_string()],
        }
    }
    
    fn create_temperate_forest() -> BiomeDefinition {
        BiomeDefinition {
            name: "Temperate Forest".to_string(),
            description: "Deciduous and mixed forests with moderate climate".to_string(),
            climate_requirements: crate::world::tiles::properties::ClimateRequirements {
                temperature_range: (5, 25),
                rainfall_range: (100, 300),
                elevation_range: Some((0.0, 1500.0)),
            },
            terrain_preferences: vec!["forest".to_string(), "hills".to_string()],
            modifiers: crate::world::tiles::properties::BiomeModifiers {
                movement_cost_multiplier: 1.5,
                defense_bonus: 0.15,
                agriculture_yield: 0.9,
                mining_yield: 1.0,
                population_capacity: 1.1,
            },
            special_resources: vec!["timber".to_string(), "game".to_string()],
        }
    }
    
    fn create_temperate_grassland() -> BiomeDefinition {
        BiomeDefinition {
            name: "Temperate Grassland".to_string(),
            description: "Fertile grasslands ideal for agriculture".to_string(),
            climate_requirements: crate::world::tiles::properties::ClimateRequirements {
                temperature_range: (0, 25),
                rainfall_range: (50, 250),
                elevation_range: None,
            },
            terrain_preferences: vec!["grassland".to_string(), "plains".to_string()],
            modifiers: crate::world::tiles::properties::BiomeModifiers {
                movement_cost_multiplier: 1.0,
                defense_bonus: 0.0,
                agriculture_yield: 1.5,
                mining_yield: 0.7,
                population_capacity: 1.3,
            },
            special_resources: vec!["grain".to_string(), "livestock".to_string()],
        }
    }
    
    fn create_desert() -> BiomeDefinition {
        BiomeDefinition {
            name: "Desert".to_string(),
            description: "Arid landscape with extreme temperatures".to_string(),
            climate_requirements: crate::world::tiles::properties::ClimateRequirements {
                temperature_range: (15, 45),
                rainfall_range: (0, 50),
                elevation_range: None,
            },
            terrain_preferences: vec!["desert".to_string(), "plains".to_string()],
            modifiers: crate::world::tiles::properties::BiomeModifiers {
                movement_cost_multiplier: 1.8,
                defense_bonus: 0.1,
                agriculture_yield: 0.1,
                mining_yield: 1.5,
                population_capacity: 0.2,
            },
            special_resources: vec!["oil".to_string(), "minerals".to_string(), "salt".to_string()],
        }
    }
    
    fn create_tundra() -> BiomeDefinition {
        BiomeDefinition {
            name: "Tundra".to_string(),
            description: "Cold, treeless plains with permafrost".to_string(),
            climate_requirements: crate::world::tiles::properties::ClimateRequirements {
                temperature_range: (-20, 5),
                rainfall_range: (20, 150),
                elevation_range: None,
            },
            terrain_preferences: vec!["tundra".to_string(), "plains".to_string()],
            modifiers: crate::world::tiles::properties::BiomeModifiers {
                movement_cost_multiplier: 2.0,
                defense_bonus: 0.05,
                agriculture_yield: 0.2,
                mining_yield: 1.2,
                population_capacity: 0.3,
            },
            special_resources: vec!["furs".to_string(), "metals".to_string()],
        }
    }
    
    fn create_taiga() -> BiomeDefinition {
        BiomeDefinition {
            name: "Taiga".to_string(),
            description: "Coniferous forest of the subarctic".to_string(),
            climate_requirements: crate::world::tiles::properties::ClimateRequirements {
                temperature_range: (-10, 15),
                rainfall_range: (100, 250),
                elevation_range: Some((0.0, 2000.0)),
            },
            terrain_preferences: vec!["forest".to_string(), "hills".to_string()],
            modifiers: crate::world::tiles::properties::BiomeModifiers {
                movement_cost_multiplier: 1.8,
                defense_bonus: 0.1,
                agriculture_yield: 0.4,
                mining_yield: 1.1,
                population_capacity: 0.6,
            },
            special_resources: vec!["softwood".to_string(), "furs".to_string()],
        }
    }
    
    fn create_mediterranean() -> BiomeDefinition {
        BiomeDefinition {
            name: "Mediterranean".to_string(),
            description: "Warm, dry summers and mild, wet winters".to_string(),
            climate_requirements: crate::world::tiles::properties::ClimateRequirements {
                temperature_range: (10, 28),
                rainfall_range: (80, 200),
                elevation_range: Some((0.0, 800.0)),
            },
            terrain_preferences: vec!["hills".to_string(), "coast".to_string()],
            modifiers: crate::world::tiles::properties::BiomeModifiers {
                movement_cost_multiplier: 1.2,
                defense_bonus: 0.08,
                agriculture_yield: 1.3,
                mining_yield: 0.9,
                population_capacity: 1.2,
            },
            special_resources: vec!["olives".to_string(), "wine".to_string()],
        }
    }
    
    fn create_savanna() -> BiomeDefinition {
        BiomeDefinition {
            name: "Savanna".to_string(),
            description: "Grassland with scattered trees and seasonal rainfall".to_string(),
            climate_requirements: crate::world::tiles::properties::ClimateRequirements {
                temperature_range: (20, 32),
                rainfall_range: (60, 180),
                elevation_range: None,
            },
            terrain_preferences: vec!["grassland".to_string(), "plains".to_string()],
            modifiers: crate::world::tiles::properties::BiomeModifiers {
                movement_cost_multiplier: 1.1,
                defense_bonus: 0.0,
                agriculture_yield: 1.0,
                mining_yield: 0.8,
                population_capacity: 0.9,
            },
            special_resources: vec!["wildlife".to_string(), "ivory".to_string()],
        }
    }
    
    fn create_alpine() -> BiomeDefinition {
        BiomeDefinition {
            name: "Alpine".to_string(),
            description: "High altitude mountain environment".to_string(),
            climate_requirements: crate::world::tiles::properties::ClimateRequirements {
                temperature_range: (-15, 10),
                rainfall_range: (100, 300),
                elevation_range: Some((1500.0, 5000.0)),
            },
            terrain_preferences: vec!["mountain".to_string(), "hills".to_string()],
            modifiers: crate::world::tiles::properties::BiomeModifiers {
                movement_cost_multiplier: 3.0,
                defense_bonus: 0.3,
                agriculture_yield: 0.1,
                mining_yield: 2.0,
                population_capacity: 0.2,
            },
            special_resources: vec!["rare_metals".to_string(), "gems".to_string()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_biome_generator_creation() {
        let generator = BiomeGenerator::new(BiomeGenConfig::default());
        assert!(generator.is_ok());
        
        let gen = generator.unwrap();
        assert!(gen.biome_definitions().len() > 5);
    }
    
    #[test]
    fn test_biome_definitions_loading() {
        let definitions = BiomeGenerator::load_biome_definitions().unwrap();
        assert!(definitions.contains_key("tropical_rainforest"));
        assert!(definitions.contains_key("temperate_grassland"));
        assert!(definitions.contains_key("desert"));
    }
}
