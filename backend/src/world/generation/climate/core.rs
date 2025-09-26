//! Core Climate Generation
//!
//! Integrates noise generation, ECS, and Lua scripting for sophisticated climate modeling.

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tracing::{debug, info, instrument};

use crate::{
    core::{
        scheduler::{Scheduler, SchedulerError}, 
        caching::{GameCache, GameCacheBuilder, CacheKey, CachePriority, CacheConfig},
        hashing::HashStrategies,
    },
    scripting::{ScriptManager, ScriptResult, LuaEventData, LuaEventValue},
    world::{
        generation::noise::NoiseGenerator,
        tiles::{
            chunks::TileId,
            properties::EnhancedClimate,
        },
    },
};

/// Climate generation configuration - minimal but powerful
#[derive(Debug, Clone, Serialize, Deserialize, Resource)]
pub struct ClimateGenConfig {
    pub world_size: (u32, u32),
    pub temperature_range: (i8, i8),
    pub rainfall_range: (u16, u16),
    pub use_lua_rules: bool,
    pub latitude_effect: f32,
    pub elevation_lapse_rate: f32,
}

impl Default for ClimateGenConfig {
    fn default() -> Self {
        Self {
            world_size: (256, 256),
            temperature_range: (-30, 45),
            rainfall_range: (0, 500),
            use_lua_rules: true,
            latitude_effect: 0.8,
            elevation_lapse_rate: 6.5, // °C per 1000m
        }
    }
}

/// Core climate generator - leverages existing noise and scripting systems
#[derive(Debug, Resource)]
pub struct ClimateGenerator {
    config: ClimateGenConfig,
    script_manager: Arc<ScriptManager>,
    cache: GameCache,
}

impl ClimateGenerator {
    /// Create new generator using existing systems
    pub fn new(config: ClimateGenConfig) -> ScriptResult<Self> {
        let script_manager = Arc::new(ScriptManager::new()?);
        let cache = GameCacheBuilder::new()
            .max_memory_mb(64)
            .default_ttl(std::time::Duration::from_secs(180))
            .build();
        
        let mut generator = Self { config, script_manager, cache };
        
        if generator.config.use_lua_rules {
            generator.load_scripts()?;
        }
        
        info!("🌡️ Climate Generator initialized (Lua: {})", generator.config.use_lua_rules);
        Ok(generator)
    }
    
    /// Load Lua climate scripts - smart loading only what's needed
    #[instrument(skip(self))]
    fn load_scripts(&self) -> ScriptResult<()> {
        let scripts = [
            "climate/temperature_zones.lua",
            "climate/ocean_currents.lua",
            "climate/wind_patterns.lua", 
            "climate/rainfall_shadows.lua",
        ];
        
        for script in &scripts {
            if let Err(e) = self.script_manager.load_script(script) {
                debug!("Optional climate script not found: {} ({})", script, e);
            }
        }
        
        Ok(())
    }
    
    /// Generate climate using Zig SIMD + Lua enhancement with deterministic hashing
    #[instrument(skip(self))]
    pub async fn generate_climate(
        &self,
        tile_id: TileId,
        x: f64,
        y: f64,
        elevation: f32,
        noise_gen: &NoiseGenerator,
    ) -> ScriptResult<EnhancedClimate> {
        // Create deterministic cache key using HashStrategies
        let position_hash = HashStrategies::hash_string(&format!("{}:{}", x, y));
        let cache_key = HashStrategies::combine_hashes(&[
            position_hash,
            HashStrategies::hash_bytes(&elevation.to_ne_bytes()),
            HashStrategies::hash_string(&format!("{:?}", tile_id)),
        ]);
        
        // Check cache first for deterministic results
        let cache_key_obj = CacheKey::Custom(format!("climate_{}", cache_key));
        if let Ok(Some(cached_climate)) = self.cache.get::<EnhancedClimate>(&cache_key_obj).await {
            return Ok(cached_climate);
        }
        
        // Use Zig-optimized base climate generation
        let base_climate = self.generate_base_climate(x, y, elevation, noise_gen);
        
        let final_climate = if self.config.use_lua_rules {
            self.apply_lua_rules(tile_id, base_climate, x, y, elevation)?
        } else {
            base_climate
        };
        
        // Cache result for performance and consistency
        let _ = self.cache.set(cache_key_obj, final_climate.clone(), CachePriority::Medium).await;
        
        Ok(final_climate)
    }
    
    /// Generate climate synchronously (without caching for systems)
    pub fn generate_climate_sync(
        &self,
        tile_id: TileId,
        x: f64,
        y: f64,
        elevation: f32,
        noise_gen: &NoiseGenerator,
    ) -> ScriptResult<EnhancedClimate> {
        // Use Zig-optimized base climate generation
        let base_climate = self.generate_base_climate(x, y, elevation, noise_gen);
        
        let final_climate = if self.config.use_lua_rules {
            self.apply_lua_rules(tile_id, base_climate, x, y, elevation)?
        } else {
            base_climate
        };
        
        Ok(final_climate)
    }
    
    /// Generate base climate using Zig SIMD-optimized calculations
    fn generate_base_climate(
        &self,
        x: f64, 
        y: f64,
        elevation: f32,
        noise_gen: &NoiseGenerator,
    ) -> EnhancedClimate {
        // Use Zig for batch processing of single item for consistency
        match self.generate_base_climate_batch(&[(x as f32, y as f32)], &[elevation], noise_gen) {
            Ok(results) if !results.is_empty() => results[0].clone(),
            _ => {
                // Fallback to Rust implementation
                self.generate_base_climate_rust_fallback(x, y, elevation, noise_gen)
            }
        }
    }

    /// Rust fallback for base climate generation
    fn generate_base_climate_rust_fallback(
        &self,
        x: f64, 
        y: f64,
        elevation: f32,
        noise_gen: &NoiseGenerator,
    ) -> EnhancedClimate {
        // Leverage existing noise functions
        let temp_noise = noise_gen.sample_temperature(x, y);
        let rainfall_noise = noise_gen.sample_moisture(x, y);
        
        // Calculate latitude effect
        let latitude = ((y / self.config.world_size.1 as f64) - 0.5) * 180.0;
        let latitude_cooling = (latitude.abs() / 90.0) * self.config.latitude_effect as f64 * 30.0;
        
        // Apply elevation cooling
        let elevation_cooling = (elevation / 1000.0) * self.config.elevation_lapse_rate;
        
        // Calculate temperature
        let temp_range = self.config.temperature_range;
        let base_temp = temp_range.0 as f32 + (temp_range.1 - temp_range.0) as f32 * (temp_noise + 1.0) * 0.5;
        let temperature = (base_temp as f64 - latitude_cooling - elevation_cooling as f64).clamp(-50.0, 50.0) as i8;
        
        // Calculate rainfall
        let rain_range = self.config.rainfall_range;
        let rainfall = (rain_range.0 as f32 + (rain_range.1 - rain_range.0) as f32 * (rainfall_noise + 1.0) * 0.5)
            .clamp(0.0, 500.0) as u16;
        
        // Derive humidity and wind from primary values
        let humidity = ((rainfall as f32 / 300.0) * 50.0 + ((temperature as f32 + 20.0) / 70.0) * 30.0)
            .clamp(10.0, 90.0) as u8;
            
        let wind_strength = ((noise_gen.sample_height(x * 0.5, y * 0.5) + 1.0) * 127.5)
            .clamp(0.0, 255.0) as u8;
        
        EnhancedClimate::new(temperature, rainfall, humidity).with_wind(wind_strength)
    }

    /// Generate base climate data using Zig SIMD optimization
    fn generate_base_climate_batch(
        &self,
        positions: &[(f32, f32)],
        elevations: &[f32],
        noise_gen: &NoiseGenerator,
    ) -> Result<Vec<EnhancedClimate>, String> {
        if positions.len() != elevations.len() {
            return Err("Position and elevation arrays must have same length".to_string());
        }

        // Generate noise data for all positions
        let mut base_temperatures = Vec::with_capacity(positions.len());
        let mut base_rainfall = Vec::with_capacity(positions.len());
        let wind_directions = vec![0.0f32; positions.len()]; // Default wind direction

        // Sample noise for all positions (this could be optimized further)
        for &(x, y) in positions {
            let temp_noise = noise_gen.sample_temperature(x as f64, y as f64);
            let rainfall_noise = noise_gen.sample_moisture(x as f64, y as f64);
            
            // Calculate base values using config ranges
            let temp_range = self.config.temperature_range;
            let base_temp = temp_range.0 as f32 + (temp_range.1 - temp_range.0) as f32 * (temp_noise + 1.0) * 0.5;
            
            let rain_range = self.config.rainfall_range;
            let base_rain = rain_range.0 as f32 + (rain_range.1 - rain_range.0) as f32 * (rainfall_noise + 1.0) * 0.5;
            
            base_temperatures.push(base_temp as i8);
            base_rainfall.push(base_rain);
        }

        // Use Zig SIMD for comprehensive climate processing
        let (final_temperatures, final_rainfall, final_humidity) = super::zig_ffi::climate_process_all(
            positions,
            elevations,
            &base_temperatures,
            &base_rainfall,
            &vec![50u8; positions.len()], // Base humidity
            &wind_directions,
        )?;

        // Convert results to EnhancedClimate objects
        let mut results = Vec::with_capacity(positions.len());
        for i in 0..positions.len() {
            if i < final_temperatures.len() {
                let climate = EnhancedClimate::new(
                    final_temperatures[i],
                    final_rainfall[i] as u16,
                    final_humidity[i],
                );
                results.push(climate);
            }
        }

        Ok(results)
    }
    
    /// Apply Lua climate rules - only if enabled
    #[instrument(skip(self))]
    fn apply_lua_rules(
        &self,
        tile_id: TileId,
        mut climate: EnhancedClimate,
        x: f64,
        y: f64,
        elevation: f32,
    ) -> ScriptResult<EnhancedClimate> {
        let mut event_data = LuaEventData {
            event_type: "climate_generation".to_string(),
            data: HashMap::new(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            source: Some("climate_generator".to_string()),
        };
        event_data.data.insert("tile_id".to_string(), LuaEventValue::String(format!("{:?}", tile_id)));
        event_data.data.insert("x".to_string(), LuaEventValue::Number(x));
        event_data.data.insert("y".to_string(), LuaEventValue::Number(y));
        event_data.data.insert("elevation".to_string(), LuaEventValue::Number(elevation as f64));
        event_data.data.insert("base_temperature".to_string(), LuaEventValue::Integer(climate.temperature as i64));
        event_data.data.insert("base_rainfall".to_string(), LuaEventValue::Integer(climate.rainfall as i64));
        event_data.data.insert("base_humidity".to_string(), LuaEventValue::Integer(climate.humidity as i64));
        event_data.data.insert("wind_strength".to_string(), LuaEventValue::Number(climate.wind_strength as f64));
        
        // Apply each Lua system if available
        self.apply_lua_system(&mut climate, "climate_temperature_zones", &event_data)?;
        self.apply_lua_system(&mut climate, "climate_rainfall_shadows", &event_data)?;
        self.apply_lua_system(&mut climate, "climate_ocean_currents", &event_data)?;
        self.apply_lua_system(&mut climate, "climate_wind_patterns", &event_data)?;
        
        Ok(climate)
    }
    
    /// Apply individual Lua system - helper to reduce code duplication
    fn apply_lua_system(
        &self,
        climate: &mut EnhancedClimate,
        event_name: &str,
        event_data: &LuaEventData,
    ) -> ScriptResult<()> {
        if let Ok(results) = self.script_manager.trigger_event(event_name, event_data) {
            for result in results {
                self.parse_lua_result(climate, &result);
            }
        }
        Ok(())
    }
    
    /// Parse Lua results and apply to climate - centralized parsing
    fn parse_lua_result(&self, climate: &mut EnhancedClimate, result: &str) {
        if let Some((key, value)) = result.split_once(':') {
            match key {
                "temperature_mod" => {
                    if let Ok(mod_val) = value.parse::<i8>() {
                        climate.temperature = (climate.temperature + mod_val).clamp(-50, 50);
                    }
                }
                "rainfall" => {
                    if let Ok(rainfall) = value.parse::<u16>() {
                        climate.rainfall = rainfall.min(500);
                    }
                }
                "humidity_mod" => {
                    if let Ok(mod_val) = value.parse::<i8>() {
                        climate.humidity = ((climate.humidity as i8) + mod_val).clamp(0, 100) as u8;
                    }
                }
                "wind_strength" => {
                    if let Ok(wind) = value.parse::<u8>() {
                        climate.wind_strength = wind;
                    }
                }
                _ => {} // Ignore unknown keys
            }
        } else if let Ok(temp) = result.parse::<i8>() {
            // Direct temperature result
            climate.temperature = temp.clamp(-50, 50);
        }
    }
    
    /// Batch generation using existing scheduler with deterministic processing
    pub fn generate_batch(
        &self,
        mut tiles: Vec<(TileId, f64, f64, f32)>,
        noise_gen: NoiseGenerator,
        scheduler: &Scheduler,
    ) -> Result<HashMap<TileId, EnhancedClimate>, String> {
        use crate::core::scheduler::{TaskBatch, Stage, Resource as SchedulerResource};
        use std::sync::{Arc, Mutex};
        
        // Use the owned noise generator directly
        
        // Sort tiles by TileId for deterministic processing order
        tiles.sort_by_key(|(tile_id, _, _, _)| format!("{:?}", tile_id));
        
        let results = Arc::new(Mutex::new(HashMap::new()));
        let chunk_size = 64.max(tiles.len() / scheduler.active_count().max(1));
        let mut batch = TaskBatch::new(Stage::Update);
        
        // Create deterministic batch hash for reproducible results
        let batch_hash = HashStrategies::hash_string(&format!("climate_batch_{}", 
            HashStrategies::combine_hashes(&tiles.iter()
                .map(|(tile_id, x, y, elev)| HashStrategies::combine_hashes(&[
                    HashStrategies::hash_string(&format!("{:?}", tile_id)),
                    HashStrategies::hash_bytes(&x.to_ne_bytes()),
                    HashStrategies::hash_bytes(&y.to_ne_bytes()),
                    HashStrategies::hash_bytes(&elev.to_ne_bytes()),
                ]))
                .collect::<Vec<_>>())
        ));
        
        for (chunk_idx, chunk) in tiles.chunks(chunk_size).enumerate() {
            let results_clone = Arc::clone(&results);
            let chunk_data = chunk.to_vec();
            let config_clone = self.config.clone();
            let script_manager = Arc::clone(&self.script_manager);
            // Create a new noise generator with the same config for this task
            let noise_gen_for_task = NoiseGenerator::new(noise_gen.config());
            
            batch.add_task_with_resources(
                format!("climate_batch_{}_{}", batch_hash, chunk_idx),
                vec![SchedulerResource::write::<HashMap<TileId, EnhancedClimate>>()],
                move || -> Result<(), crate::core::scheduler::SchedulerError> {
                    // Create a temporary generator with cloned data
                    let temp_generator = ClimateGenerator {
                        config: config_clone,
                        script_manager,
                        cache: GameCache::new(CacheConfig::default()),
                    };
                    let mut local_results = HashMap::new();
                    
                    // Process chunk in deterministic order
                    for (tile_id, x, y, elevation) in chunk_data {
                        match temp_generator.generate_climate_sync(tile_id, x, y, elevation, &noise_gen_for_task) {
                            Ok(climate) => {
                                local_results.insert(tile_id, climate);
                            }
                            Err(e) => {
                                tracing::warn!("Failed to generate climate for {:?}: {}", tile_id, e);
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
            format!("Scheduler error: {:?}", errors.into_iter().next().unwrap_or_else(|| SchedulerError::TaskFailed("Unknown scheduler error".to_string())))
        })?;
        
        let final_results = results.lock().unwrap().clone();
        Ok(final_results)
    }
    
    /// High-performance batch generation using Zig SIMD optimizations
    #[instrument(skip(self))]
    pub fn generate_batch_optimized(
        &self,
        mut tiles: Vec<(TileId, f64, f64, f32)>,
        noise_gen: &NoiseGenerator,
    ) -> Result<HashMap<TileId, EnhancedClimate>, String> {
        if tiles.is_empty() {
            return Ok(HashMap::new());
        }
        
        // Sort tiles for deterministic processing
        tiles.sort_by_key(|(tile_id, _, _, _)| format!("{:?}", tile_id));
        
        // Extract data for batch processing
        let positions: Vec<(f32, f32)> = tiles.iter()
            .map(|(_, x, y, _)| (*x as f32, *y as f32))
            .collect();
        let elevations: Vec<f32> = tiles.iter()
            .map(|(_, _, _, elevation)| *elevation)
            .collect();
        
        // Use new Zig-optimized base climate batch generation
        let base_climates = self.generate_base_climate_batch(&positions, &elevations, noise_gen)
            .map_err(|e| format!("Base climate batch generation failed: {}", e))?;
        
        // Combine results into final map
        let mut results = HashMap::new();
        for (i, (tile_id, _, _, _)) in tiles.iter().enumerate() {
            if i < base_climates.len() {
                let mut climate = base_climates[i].clone();
                
                if self.config.use_lua_rules {
                    // Apply Lua rules for enhanced climate modeling
                    match self.apply_lua_rules(*tile_id, climate.clone(), positions[i].0 as f64, positions[i].1 as f64, elevations[i]) {
                        Ok(enhanced) => climate = enhanced,
                        Err(e) => {
                            tracing::warn!("Lua rules failed for {:?}: {}", tile_id, e);
                            // Continue with base climate if Lua fails
                        }
                    }
                }
                
                results.insert(*tile_id, climate);
            }
        }
        
        info!("🚀 Generated {} climates using optimized Zig batch processing", results.len());
        Ok(results)
    }
    
    /// Get configuration
    pub fn config(&self) -> &ClimateGenConfig {
        &self.config
    }
}

// Extension trait for enhanced climate to reduce code duplication
trait ClimateExt {
    fn with_wind(self, wind_strength: u8) -> Self;
}

impl ClimateExt for EnhancedClimate {
    fn with_wind(mut self, wind_strength: u8) -> Self {
        self.wind_strength = wind_strength;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_climate_generator_creation() {
        let generator = ClimateGenerator::new(ClimateGenConfig::default());
        assert!(generator.is_ok());
    }
    
    #[test]
    fn test_lua_result_parsing() {
        let generator = ClimateGenerator::new(ClimateGenConfig::default()).unwrap();
        let mut climate = EnhancedClimate::new(20, 200, 50);
        
        generator.parse_lua_result(&mut climate, "temperature_mod:5");
        assert_eq!(climate.temperature, 25);
        
        generator.parse_lua_result(&mut climate, "rainfall:300");
        assert_eq!(climate.rainfall, 300);
    }
}
