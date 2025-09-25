//! Climate Patterns
//!
//! Wind patterns, ocean currents, and seasonal variations using ECS resources.

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::world::tiles::chunks::TileId;

/// Global wind patterns resource
#[derive(Debug, Resource, Serialize, Deserialize)]
pub struct WindPatterns {
    /// Wind data by latitude band (0-179)
    pub latitude_winds: HashMap<u32, WindBelt>,
    /// Terrain-based wind modifications  
    pub terrain_effects: HashMap<String, TerrainWindEffect>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindBelt {
    pub direction: f32,    // Radians
    pub base_speed: f32,   // km/h
    pub consistency: f32,  // 0-1, how consistent the wind is
    pub belt_type: WindBeltType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WindBeltType {
    TradeWinds,
    Westerlies, 
    PolarEasterlies,
    Doldrums,
    Variable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainWindEffect {
    pub speed_multiplier: f32,
    pub direction_change: f32, // Radians
    pub turbulence: f32,       // 0-1
}

impl Default for WindPatterns {
    fn default() -> Self {
        let mut latitude_winds = HashMap::new();
        
        // Generate realistic wind belts
        for lat in 0..180 {
            let actual_lat = (lat as f32 / 180.0 - 0.5) * 180.0;
            let abs_lat = actual_lat.abs();
            
            let belt = match abs_lat as u32 {
                0..=30 => WindBelt {
                    direction: if actual_lat >= 0.0 { 1.2 } else { -1.2 },
                    base_speed: 45.0,
                    consistency: 0.8,
                    belt_type: WindBeltType::TradeWinds,
                },
                31..=60 => WindBelt {
                    direction: if actual_lat >= 0.0 { -0.8 } else { 0.8 },
                    base_speed: 65.0,
                    consistency: 0.6,
                    belt_type: WindBeltType::Westerlies,
                },
                61..=90 => WindBelt {
                    direction: if actual_lat >= 0.0 { 0.4 } else { -0.4 },
                    base_speed: 35.0,
                    consistency: 0.5,
                    belt_type: WindBeltType::PolarEasterlies,
                },
                _ => WindBelt {
                    direction: 0.0,
                    base_speed: 25.0,
                    consistency: 0.3,
                    belt_type: WindBeltType::Variable,
                },
            };
            
            latitude_winds.insert(lat, belt);
        }
        
        let mut terrain_effects = HashMap::new();
        terrain_effects.insert("mountain".to_string(), TerrainWindEffect {
            speed_multiplier: 1.8,
            direction_change: 0.3,
            turbulence: 0.4,
        });
        terrain_effects.insert("valley".to_string(), TerrainWindEffect {
            speed_multiplier: 0.6,
            direction_change: -0.2,
            turbulence: 0.1,
        });
        terrain_effects.insert("plains".to_string(), TerrainWindEffect {
            speed_multiplier: 1.0,
            direction_change: 0.0,
            turbulence: 0.0,
        });
        terrain_effects.insert("coast".to_string(), TerrainWindEffect {
            speed_multiplier: 1.3,
            direction_change: 0.1,
            turbulence: 0.1,
        });
        
        Self { latitude_winds, terrain_effects }
    }
}

impl WindPatterns {
    /// Get wind for a latitude band
    pub fn get_wind(&self, latitude_index: u32) -> Option<&WindBelt> {
        self.latitude_winds.get(&latitude_index.clamp(0, 179))
    }
    
    /// Apply terrain effects to wind
    pub fn apply_terrain_effects(&self, base_wind: &WindBelt, terrain_type: &str) -> WindBelt {
        let effects = self.terrain_effects.get(terrain_type)
            .unwrap_or(&TerrainWindEffect {
                speed_multiplier: 1.0,
                direction_change: 0.0,
                turbulence: 0.0,
            });
            
        WindBelt {
            direction: base_wind.direction + effects.direction_change,
            base_speed: base_wind.base_speed * effects.speed_multiplier,
            consistency: base_wind.consistency * (1.0 - effects.turbulence),
            belt_type: base_wind.belt_type.clone(),
        }
    }
}

/// Ocean currents resource
#[derive(Debug, Resource, Serialize, Deserialize)]
pub struct OceanCurrents {
    /// Current strength by tile
    pub current_map: HashMap<TileId, f32>,
    /// Current direction by tile (radians)
    pub direction_map: HashMap<TileId, f32>,
    /// Current type by tile
    pub current_types: HashMap<TileId, CurrentType>,
    /// Temperature effects
    pub temperature_effects: CurrentEffects,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CurrentType {
    WarmWesternBoundary,
    ColdEasternBoundary, 
    EquatorialWestward,
    SubtropicalGyre,
    Circumpolar,
    Calm,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentEffects {
    pub warm_current_temp: f32,    // °C
    pub cold_current_temp: f32,    // °C
    pub warm_current_humidity: i8,  // %
    pub cold_current_humidity: i8,  // %
}

impl Default for OceanCurrents {
    fn default() -> Self {
        Self {
            current_map: HashMap::new(),
            direction_map: HashMap::new(), 
            current_types: HashMap::new(),
            temperature_effects: CurrentEffects {
                warm_current_temp: 3.0,
                cold_current_temp: -4.0,
                warm_current_humidity: 15,
                cold_current_humidity: -8,
            },
        }
    }
}

impl OceanCurrents {
    /// Get current effect for a tile
    pub fn get_current_effect(&self, tile_id: TileId) -> Option<(f32, f32, &CurrentType)> {
        let strength = self.current_map.get(&tile_id)?;
        let direction = self.direction_map.get(&tile_id)?;
        let current_type = self.current_types.get(&tile_id)?;
        Some((*strength, *direction, current_type))
    }
    
    /// Calculate temperature effect from current
    pub fn temperature_effect(&self, current_type: &CurrentType, strength: f32) -> f32 {
        match current_type {
            CurrentType::WarmWesternBoundary => self.temperature_effects.warm_current_temp * strength,
            CurrentType::ColdEasternBoundary => self.temperature_effects.cold_current_temp * strength,
            _ => 0.0,
        }
    }
    
    /// Calculate humidity effect from current  
    pub fn humidity_effect(&self, current_type: &CurrentType, strength: f32) -> i8 {
        match current_type {
            CurrentType::WarmWesternBoundary => (self.temperature_effects.warm_current_humidity as f32 * strength) as i8,
            CurrentType::ColdEasternBoundary => (self.temperature_effects.cold_current_humidity as f32 * strength) as i8,
            _ => 0,
        }
    }
    
    /// Update current for a tile
    pub fn update_current(&mut self, tile_id: TileId, strength: f32, direction: f32, current_type: CurrentType) {
        self.current_map.insert(tile_id, strength);
        self.direction_map.insert(tile_id, direction);
        self.current_types.insert(tile_id, current_type);
    }

    /// Calculate ocean proximity for positions using Zig SIMD
    pub fn calculate_ocean_proximity_batch(
        &self,
        positions: &[(f32, f32)],
        world_width: f32,
        world_height: f32,
    ) -> Result<Vec<f32>, String> {
        super::zig_ffi::climate_ocean_proximity(positions, world_width, world_height)
    }

    /// Calculate maritime influence for positions using Zig SIMD
    pub fn calculate_maritime_influence_batch(
        &self,
        positions: &[(f32, f32)],
        world_width: f32,
        world_height: f32,
    ) -> Result<Vec<f32>, String> {
        super::zig_ffi::climate_maritime_influence(positions, world_width, world_height)
    }

    /// Generate ocean currents based on climate data and position using Zig optimization
    pub fn generate_currents_for_positions(
        &mut self,
        tile_positions: &[(TileId, f32, f32)],
        climate_temps: &[i8],
        world_width: f32,
        world_height: f32,
    ) -> Result<(), String> {
        if tile_positions.len() != climate_temps.len() {
            return Err("Position and temperature arrays must have same length".to_string());
        }

        // Extract just positions for batch processing
        let positions: Vec<(f32, f32)> = tile_positions.iter()
            .map(|(_, x, y)| (*x, *y))
            .collect();

        // Get ocean proximity using Zig SIMD
        let proximities = self.calculate_ocean_proximity_batch(&positions, world_width, world_height)?;

        // Generate currents based on temperature and ocean proximity
        for ((tile_id, x, y), (&temperature, &proximity)) in tile_positions.iter().zip(climate_temps.iter().zip(proximities.iter())) {
            // Only generate currents for positions near water (high ocean proximity)
            if proximity > 0.3 {
                let latitude = ((*y / world_height) - 0.5) * 180.0;
                
                // Determine current type based on temperature and location
                let current_type = if temperature > 20 {
                    CurrentType::WarmWesternBoundary
                } else if temperature < 10 {
                    CurrentType::ColdEasternBoundary
                } else if latitude.abs() < 30.0 {
                    CurrentType::EquatorialWestward
                } else if latitude.abs() > 60.0 {
                    CurrentType::Circumpolar
                } else {
                    CurrentType::SubtropicalGyre
                };

                // Calculate current strength and direction
                let base_strength = proximity * 0.8; // Stronger currents near coasts
                let current_strength = match current_type {
                    CurrentType::WarmWesternBoundary => base_strength * 1.5,
                    CurrentType::ColdEasternBoundary => base_strength * 1.2,
                    CurrentType::EquatorialWestward => base_strength * 1.3,
                    CurrentType::Circumpolar => base_strength * 2.0,
                    CurrentType::SubtropicalGyre => base_strength,
                    CurrentType::Calm => base_strength * 0.3,
                };

                // Calculate direction based on current type and latitude
                let current_direction = match current_type {
                    CurrentType::WarmWesternBoundary => if latitude > 0.0 { 1.5 } else { -1.5 },
                    CurrentType::ColdEasternBoundary => if latitude > 0.0 { -0.8 } else { 0.8 },
                    CurrentType::EquatorialWestward => std::f32::consts::PI,
                    CurrentType::Circumpolar => if latitude > 0.0 { 0.2 } else { -0.2 },
                    CurrentType::SubtropicalGyre => latitude.signum() * 0.5,
                    CurrentType::Calm => 0.0,
                };

                self.update_current(*tile_id, current_strength, current_direction, current_type);
            }
        }

        Ok(())
    }

    /// Apply current effects to temperature and humidity using batch processing
    pub fn apply_current_effects_batch(
        &self,
        tile_ids: &[TileId],
        base_temperatures: &[i8],
        base_humidity: &[u8],
    ) -> Result<(Vec<i8>, Vec<u8>), String> {
        if tile_ids.len() != base_temperatures.len() || tile_ids.len() != base_humidity.len() {
            return Err("All input arrays must have same length".to_string());
        }

        let mut modified_temps = Vec::with_capacity(tile_ids.len());
        let mut modified_humidity = Vec::with_capacity(tile_ids.len());

        for (i, &tile_id) in tile_ids.iter().enumerate() {
            let base_temp = base_temperatures[i];
            let base_hum = base_humidity[i];

            if let Some((strength, _, current_type)) = self.get_current_effect(tile_id) {
                let temp_effect = self.temperature_effect(current_type, strength);
                let humidity_effect = self.humidity_effect(current_type, strength);

                let new_temp = ((base_temp as f32) + temp_effect).clamp(-50.0, 50.0) as i8;
                let new_hum = ((base_hum as i8) + humidity_effect).clamp(0, 100) as u8;

                modified_temps.push(new_temp);
                modified_humidity.push(new_hum);
            } else {
                modified_temps.push(base_temp);
                modified_humidity.push(base_hum);
            }
        }

        Ok((modified_temps, modified_humidity))
    }
}

/// Seasonal variation resource
#[derive(Debug, Resource, Serialize, Deserialize)]
pub struct SeasonalVariation {
    /// Current season (0.0-1.0 where 0.0 = spring)
    pub current_season: f32,
    /// Temperature variation by climate zone
    pub temperature_variation: HashMap<String, f32>,
    /// Rainfall variation by climate zone
    pub rainfall_variation: HashMap<String, f32>,
}

impl Default for SeasonalVariation {
    fn default() -> Self {
        let mut temp_var = HashMap::new();
        temp_var.insert("equatorial".to_string(), 2.0);
        temp_var.insert("tropical".to_string(), 5.0);
        temp_var.insert("temperate".to_string(), 15.0);
        temp_var.insert("polar".to_string(), 25.0);
        
        let mut rain_var = HashMap::new();
        rain_var.insert("equatorial".to_string(), 50.0);
        rain_var.insert("tropical".to_string(), 100.0);
        rain_var.insert("temperate".to_string(), 75.0);
        rain_var.insert("polar".to_string(), 25.0);
        
        Self {
            current_season: 0.0,
            temperature_variation: temp_var,
            rainfall_variation: rain_var,
        }
    }
}

impl SeasonalVariation {
    /// Apply seasonal variation to temperature using Zig SIMD when possible
    pub fn apply_temperature_variation(&self, base_temp: i8, climate_zone: &str, latitude: f32) -> i8 {
        // Use Zig SIMD for batch processing of single item for consistency
        match self.apply_temperature_variation_batch(&[base_temp], &[climate_zone], &[latitude]) {
            Ok(results) if !results.is_empty() => results[0],
            _ => {
                // Fallback to Rust implementation
                self.apply_temperature_variation_rust_fallback(base_temp, climate_zone, latitude)
            }
        }
    }

    /// Rust fallback for temperature variation
    fn apply_temperature_variation_rust_fallback(&self, base_temp: i8, climate_zone: &str, latitude: f32) -> i8 {
        let variation = self.temperature_variation.get(climate_zone).unwrap_or(&10.0);
        
        // Seasonal cycle with latitude adjustment (southern hemisphere inverted)
        let season_cycle = if latitude >= 0.0 {
            (self.current_season * 2.0 * std::f32::consts::PI).sin()
        } else {
            ((self.current_season + 0.5) * 2.0 * std::f32::consts::PI).sin()
        };
        
        let temp_change = season_cycle * variation;
        (base_temp as f32 + temp_change).clamp(-50.0, 50.0) as i8
    }
    
    /// Batch temperature variation using Zig SIMD
    pub fn apply_temperature_variation_batch(
        &self,
        base_temps: &[i8],
        climate_zones: &[&str],
        latitudes: &[f32],
    ) -> Result<Vec<i8>, String> {
        if base_temps.len() != climate_zones.len() || base_temps.len() != latitudes.len() {
            return Err("All input arrays must have same length".to_string());
        }

        // Convert climate zones to Zig enum values
        let zig_zones: Vec<super::zig_ffi::ClimateZone> = climate_zones.iter()
            .map(|zone| match zone.to_lowercase().as_str() {
                "equatorial" => super::zig_ffi::ClimateZone::Equatorial,
                "tropical" => super::zig_ffi::ClimateZone::Tropical,
                "temperate" => super::zig_ffi::ClimateZone::Temperate,
                "polar" => super::zig_ffi::ClimateZone::Polar,
                "desert" => super::zig_ffi::ClimateZone::Desert,
                "mediterranean" => super::zig_ffi::ClimateZone::Mediterranean,
                _ => super::zig_ffi::ClimateZone::Temperate, // Default
            })
            .collect();

        // Prepare temperature variation array for each zone
        let temp_variations = [
            self.temperature_variation.get("equatorial").unwrap_or(&2.0),
            self.temperature_variation.get("tropical").unwrap_or(&5.0),
            self.temperature_variation.get("temperate").unwrap_or(&15.0),
            self.temperature_variation.get("polar").unwrap_or(&25.0),
            self.temperature_variation.get("desert").unwrap_or(&12.0),
            self.temperature_variation.get("mediterranean").unwrap_or(&8.0),
        ];

        // Use Zig SIMD for seasonal temperature calculation
        super::zig_ffi::climate_seasonal_temperature(
            base_temps,
            &zig_zones,
            latitudes,
            self.current_season,
            &temp_variations.map(|v| *v),
        )
    }
    
    /// Apply seasonal variation to rainfall using Zig SIMD when possible
    pub fn apply_rainfall_variation(&self, base_rainfall: u16, climate_zone: &str) -> u16 {
        // Use Zig SIMD for batch processing of single item for consistency
        match self.apply_rainfall_variation_batch(&[base_rainfall], &[climate_zone], &[0.0]) { // latitude not used for rainfall in current impl
            Ok(results) if !results.is_empty() => results[0],
            _ => {
                // Fallback to Rust implementation
                self.apply_rainfall_variation_rust_fallback(base_rainfall, climate_zone)
            }
        }
    }

    /// Rust fallback for rainfall variation
    fn apply_rainfall_variation_rust_fallback(&self, base_rainfall: u16, climate_zone: &str) -> u16 {
        let variation = self.rainfall_variation.get(climate_zone).unwrap_or(&50.0);
        
        // Seasonal rainfall cycle (different from temperature)
        let rain_cycle = ((self.current_season + 0.25) * 2.0 * std::f32::consts::PI).sin();
        let rain_change = rain_cycle * variation;
        
        (base_rainfall as f32 + rain_change).clamp(0.0, 500.0) as u16
    }
    
    /// Batch rainfall variation using Zig SIMD
    pub fn apply_rainfall_variation_batch(
        &self,
        base_rainfall: &[u16],
        climate_zones: &[&str],
        latitudes: &[f32], // Needed for Zig function signature but not used much
    ) -> Result<Vec<u16>, String> {
        if base_rainfall.len() != climate_zones.len() || base_rainfall.len() != latitudes.len() {
            return Err("All input arrays must have same length".to_string());
        }

        // Convert climate zones to Zig enum values
        let zig_zones: Vec<super::zig_ffi::ClimateZone> = climate_zones.iter()
            .map(|zone| match zone.to_lowercase().as_str() {
                "equatorial" => super::zig_ffi::ClimateZone::Equatorial,
                "tropical" => super::zig_ffi::ClimateZone::Tropical,
                "temperate" => super::zig_ffi::ClimateZone::Temperate,
                "polar" => super::zig_ffi::ClimateZone::Polar,
                "desert" => super::zig_ffi::ClimateZone::Desert,
                "mediterranean" => super::zig_ffi::ClimateZone::Mediterranean,
                _ => super::zig_ffi::ClimateZone::Temperate, // Default
            })
            .collect();

        // Prepare rainfall variation array for each zone
        let rain_variations = [
            self.rainfall_variation.get("equatorial").unwrap_or(&50.0),
            self.rainfall_variation.get("tropical").unwrap_or(&100.0),
            self.rainfall_variation.get("temperate").unwrap_or(&75.0),
            self.rainfall_variation.get("polar").unwrap_or(&25.0),
            self.rainfall_variation.get("desert").unwrap_or(&20.0),
            self.rainfall_variation.get("mediterranean").unwrap_or(&120.0),
        ];

        // Use Zig SIMD for seasonal rainfall calculation
        super::zig_ffi::climate_seasonal_rainfall(
            base_rainfall,
            &zig_zones,
            latitudes,
            self.current_season,
            &rain_variations,
        )
    }

    /// Apply monsoon effects to positions using Zig SIMD
    pub fn apply_monsoon_effects(
        &self,
        positions: &[(f32, f32)],
        monsoon_strength: f32,
    ) -> Result<Vec<f32>, String> {
        let seasonal_state = super::zig_ffi::SeasonalState {
            current_season: self.current_season,
            year_progress: self.current_season,
            hemisphere_modifier: 1.0,
        };

        super::zig_ffi::climate_monsoon_effects(positions, seasonal_state, monsoon_strength)
    }
    
    /// Update current season
    pub fn update_season(&mut self, season: f32) {
        self.current_season = season.clamp(0.0, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_wind_patterns_creation() {
        let patterns = WindPatterns::default();
        assert_eq!(patterns.latitude_winds.len(), 180);
        assert!(patterns.terrain_effects.contains_key("mountain"));
    }
    
    #[test]
    fn test_seasonal_variation() {
        let seasonal = SeasonalVariation::default();
        
        // Summer in northern hemisphere
        let summer_temp = seasonal.apply_temperature_variation(20, "temperate", 45.0);
        assert!(summer_temp > 20);
        
        // Winter in northern hemisphere 
        let mut winter_seasonal = seasonal.clone();
        winter_seasonal.update_season(0.75);
        let winter_temp = winter_seasonal.apply_temperature_variation(20, "temperate", 45.0);
        assert!(winter_temp < 20);
    }

    #[test]
    fn test_seasonal_batch_processing() {
        let seasonal = SeasonalVariation::default();
        
        let base_temps = [20, 15, 25];
        let climate_zones = ["temperate", "polar", "tropical"];
        let latitudes = [45.0, 70.0, 10.0];
        
        let results = seasonal.apply_temperature_variation_batch(&base_temps, &climate_zones, &latitudes);
        assert!(results.is_ok());
        
        let temp_results = results.unwrap();
        assert_eq!(temp_results.len(), 3);
        
        // Results should be different from base temperatures due to seasonal effects
        assert!(temp_results[0] != base_temps[0] || temp_results[1] != base_temps[1] || temp_results[2] != base_temps[2]);
    }

    #[test]
    fn test_monsoon_effects() {
        let seasonal = SeasonalVariation::default();
        let positions = vec![(100.0, 100.0), (150.0, 50.0)];
        
        let results = seasonal.apply_monsoon_effects(&positions, 100.0);
        assert!(results.is_ok());
        
        let effects = results.unwrap();
        assert_eq!(effects.len(), 2);
    }

    #[test]
    fn test_ocean_currents_zig_integration() {
        let ocean_currents = OceanCurrents::default();
        let positions = vec![(10.0, 10.0), (128.0, 128.0)];
        
        // Test ocean proximity calculation
        let proximity_result = ocean_currents.calculate_ocean_proximity_batch(&positions, 256.0, 256.0);
        assert!(proximity_result.is_ok());
        
        let proximities = proximity_result.unwrap();
        assert_eq!(proximities.len(), 2);
        assert!(proximities[0] > proximities[1]); // Coastal should be more oceanic than inland
        
        // Test maritime influence calculation
        let maritime_result = ocean_currents.calculate_maritime_influence_batch(&positions, 256.0, 256.0);
        assert!(maritime_result.is_ok());
        
        let influences = maritime_result.unwrap();
        assert_eq!(influences.len(), 2);
        assert!(influences[0] > influences[1]); // Coastal should have more maritime influence
    }
}
