//! Climate Effects
//!
//! Orographic and continental effects using Zig for SIMD calculations.

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use super::zig_ffi::{self, ClimateParams};

/// Orographic effects processor - uses Zig for SIMD calculations
#[derive(Debug, Component, Serialize, Deserialize)]
pub struct OrographicEffects {
    /// Mountain ranges affecting precipitation
    pub mountain_ranges: Vec<MountainRange>,
    /// Wind direction for orographic calculations
    pub prevailing_wind_direction: f32,
    /// Maximum orographic precipitation bonus (mm)
    pub max_orographic_bonus: f32,
    /// Rain shadow reduction factor
    pub rain_shadow_factor: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountainRange {
    pub center_x: f32,
    pub center_y: f32,
    pub width: f32,
    pub height: f32,
    pub orientation: f32, // Radians
}

impl Default for OrographicEffects {
    fn default() -> Self {
        Self {
            mountain_ranges: Vec::new(),
            prevailing_wind_direction: std::f32::consts::PI * 1.5, // Westerly
            max_orographic_bonus: 200.0,
            rain_shadow_factor: 0.6,
        }
    }
}

impl OrographicEffects {
    /// Calculate orographic precipitation effect using Zig SIMD
    pub fn calculate_precipitation_effect(
        &self,
        positions: &[(f32, f32)],
        elevations: &[f32],
        base_rainfall: &[f32],
        wind_direction: f32,
    ) -> Result<Vec<f32>, String> {
        // Create wind directions array for each position
        let wind_directions: Vec<f32> = vec![wind_direction; positions.len()];
        
        // Create climate parameters
        let params = ClimateParams {
            max_orographic_bonus: self.max_orographic_bonus,
            rain_shadow_factor: self.rain_shadow_factor,
            temperature_amplification: 1.5, // Not used for orographic
            humidity_reduction: 0.8,        // Not used for orographic
            world_width: 256.0,            // Not used for orographic
            world_height: 256.0,           // Not used for orographic
        };
        
        // Get orographic multipliers using Zig SIMD
        let orographic_multipliers = zig_ffi::climate_orographic_effects(
            positions,
            elevations,
            &wind_directions,
            &params,
        )?;
        
        // Apply multipliers to base rainfall
        let result: Vec<f32> = base_rainfall.iter().zip(orographic_multipliers.iter())
            .map(|(&base, &multiplier)| base * multiplier)
            .collect();
            
        Ok(result)
    }

    /// Calculate comprehensive orographic effects including rain shadows using Zig SIMD
    pub fn calculate_comprehensive_orographic_effects(
        &self,
        positions: &[(f32, f32)],
        elevations: &[f32],
        base_rainfall: &[f32],
        wind_direction: f32,
    ) -> Result<Vec<f32>, String> {
        // First get the basic orographic enhancement
        let enhanced_rainfall = self.calculate_precipitation_effect(positions, elevations, base_rainfall, wind_direction)?;

        // Apply rain shadow effects if we have mountain ranges
        if !self.mountain_ranges.is_empty() {
            let zig_mountain_ranges: Vec<super::zig_ffi::MountainRange> = self.mountain_ranges.iter()
                .map(|range| super::zig_ffi::MountainRange {
                    center: (range.center_x, range.center_y),
                    width: range.width,
                    height: range.height,
                    orientation: range.orientation,
                })
                .collect();

            let shadow_effects = super::zig_ffi::climate_rain_shadow_effects(
                positions,
                elevations,
                &zig_mountain_ranges,
                wind_direction,
                self.rain_shadow_factor,
            )?;

            // Apply both orographic enhancement and rain shadow reduction
            let final_rainfall: Vec<f32> = enhanced_rainfall.iter().zip(shadow_effects.iter())
                .map(|(&enhanced, &shadow_effect)| enhanced * shadow_effect)
                .collect();

            Ok(final_rainfall)
        } else {
            Ok(enhanced_rainfall)
        }
    }
    
    /// Add mountain range
    pub fn add_mountain_range(&mut self, range: MountainRange) {
        self.mountain_ranges.push(range);
    }
    
    /// Get mountain count
    pub fn mountain_count(&self) -> usize {
        self.mountain_ranges.len()
    }
}

/// Continental effects processor
#[derive(Debug, Component, Serialize, Deserialize)]
pub struct ContinentalEffects {
    /// Ocean proximity map cache
    ocean_proximity_cache: std::collections::HashMap<(u32, u32), f32>,
    /// Continental temperature amplification
    pub temperature_amplification: f32,
    /// Continental humidity reduction
    pub humidity_reduction: f32,
    /// World size for calculations
    pub world_size: (u32, u32),
}

impl Default for ContinentalEffects {
    fn default() -> Self {
        Self {
            ocean_proximity_cache: std::collections::HashMap::new(),
            temperature_amplification: 1.5,
            humidity_reduction: 0.8,
            world_size: (256, 256),
        }
    }
}

impl ContinentalEffects {
    /// Calculate continental effect on temperature
    pub fn apply_temperature_effect(&self, base_temp: i8, continentality: f32) -> i8 {
        let continental_effect = continentality * self.temperature_amplification;
        
        // Continental areas have more extreme temperatures
        let temp_modifier = if base_temp > 10 {
            continental_effect * 5.0 // Hotter summers
        } else {
            -continental_effect * 8.0 // Colder winters
        };
        
        (base_temp as f32 + temp_modifier).clamp(-50.0, 50.0) as i8
    }
    
    /// Calculate continental effect on humidity
    pub fn apply_humidity_effect(&self, base_humidity: u8, continentality: f32) -> u8 {
        let humidity_reduction = continentality * self.humidity_reduction * 20.0;
        (base_humidity as f32 - humidity_reduction).clamp(0.0, 100.0) as u8
    }
    
    /// Calculate ocean proximity (0.0 = continental, 1.0 = oceanic)
    pub fn calculate_ocean_proximity(&mut self, x: f32, y: f32) -> f32 {
        let grid_x = (x / 4.0) as u32; // Grid for caching
        let grid_y = (y / 4.0) as u32;
        let key = (grid_x, grid_y);
        
        if let Some(&cached) = self.ocean_proximity_cache.get(&key) {
            return cached;
        }
        
        // Calculate distance to nearest edge (simplified ocean proximity)
        let world_width = self.world_size.0 as f32;
        let world_height = self.world_size.1 as f32;
        
        let edge_dist_x = (x / world_width).min((world_width - x) / world_width);
        let edge_dist_y = (y / world_height).min((world_height - y) / world_height);
        let edge_distance = edge_dist_x.min(edge_dist_y);
        
        // Convert to ocean proximity (inverse of distance from edge)
        let proximity = 1.0 - edge_distance * 2.0; // Scale to 0-1 range
        let proximity = proximity.clamp(0.0, 1.0);
        
        // Cache result
        self.ocean_proximity_cache.insert(key, proximity);
        proximity
    }
    
    /// Calculate continentality (inverse of ocean proximity)
    pub fn calculate_continentality(&mut self, x: f32, y: f32) -> f32 {
        1.0 - self.calculate_ocean_proximity(x, y)
    }

    /// Calculate ocean proximity using Zig SIMD for batch operations
    pub fn calculate_ocean_proximity_batch(&self, positions: &[(f32, f32)]) -> Result<Vec<f32>, String> {
        super::zig_ffi::climate_ocean_proximity(positions, self.world_size.0 as f32, self.world_size.1 as f32)
    }

    /// Calculate maritime influence using Zig SIMD
    pub fn calculate_maritime_influence_batch(&self, positions: &[(f32, f32)]) -> Result<Vec<f32>, String> {
        super::zig_ffi::climate_maritime_influence(positions, self.world_size.0 as f32, self.world_size.1 as f32)
    }
    
    /// Batch calculate continental effects using Zig SIMD
    pub fn calculate_continental_effects_batch(
        &mut self,
        positions: &[(f32, f32)],
        base_temperatures: &[i8],
        base_humidity: &[u8],
    ) -> Result<(Vec<i8>, Vec<u8>), String> {
        // Create climate parameters
        let params = ClimateParams {
            max_orographic_bonus: 200.0,   // Not used for continental
            rain_shadow_factor: 0.6,       // Not used for continental
            temperature_amplification: self.temperature_amplification,
            humidity_reduction: self.humidity_reduction,
            world_width: self.world_size.0 as f32,
            world_height: self.world_size.1 as f32,
        };
        
        // Use Zig SIMD for continental effects calculation
        let (modified_temperatures, modified_humidity) = zig_ffi::climate_continental_effects(
            positions,
            base_temperatures,
            base_humidity,
            &params,
        )?;
        
        // Clear cache after batch processing to prevent stale data
        self.clear_cache();
        
        Ok((modified_temperatures, modified_humidity))
    }

    /// Enhanced continental effects calculation with maritime influence
    pub fn calculate_enhanced_continental_effects(
        &mut self,
        positions: &[(f32, f32)],
        base_temperatures: &[i8],
        base_humidity: &[u8],
    ) -> Result<(Vec<i8>, Vec<u8>), String> {
        // Get maritime influence for more accurate continental modeling
        let maritime_influences = self.calculate_maritime_influence_batch(positions)?;

        // Use the standard continental effects calculation
        let (mut continental_temps, mut continental_humidity) = 
            self.calculate_continental_effects_batch(positions, base_temperatures, base_humidity)?;

        // Apply maritime moderation (reduces continental extremes near coasts)
        for (i, &maritime_influence) in maritime_influences.iter().enumerate() {
            if i < continental_temps.len() {
                let base_temp = base_temperatures[i] as f32;
                let continental_temp = continental_temps[i] as f32;
                
                // Maritime influence moderates temperature extremes
                let moderated_temp = base_temp + (continental_temp - base_temp) * (1.0 - maritime_influence * 0.5);
                continental_temps[i] = moderated_temp.clamp(-50.0, 50.0) as i8;

                // Maritime influence increases humidity
                let base_hum = base_humidity[i] as f32;
                let continental_hum = continental_humidity[i] as f32;
                
                let moderated_humidity = continental_hum + maritime_influence * 20.0; // Up to +20% humidity near coasts
                continental_humidity[i] = moderated_humidity.clamp(0.0, 100.0) as u8;
            }
        }

        Ok((continental_temps, continental_humidity))
    }
    
    /// Clear proximity cache (call when world changes)
    pub fn clear_cache(&mut self) {
        self.ocean_proximity_cache.clear();
    }
}

/// Climate modifier that combines multiple effects
#[derive(Debug, Component)]
pub struct ClimateModifier {
    pub orographic: OrographicEffects,
    pub continental: ContinentalEffects,
}

impl Default for ClimateModifier {
    fn default() -> Self {
        Self {
            orographic: OrographicEffects::default(),
            continental: ContinentalEffects::default(),
        }
    }
}

impl ClimateModifier {
    /// Apply all climate effects to a batch of data using enhanced functions
    pub fn apply_effects_batch(
        &mut self,
        positions: &[(f32, f32)],
        elevations: &[f32],
        base_temperatures: &[i8],
        base_rainfall: &[f32],
        base_humidity: &[u8],
        wind_direction: f32,
    ) -> Result<(Vec<i8>, Vec<f32>, Vec<u8>), String> {
        // Apply comprehensive orographic effects (including rain shadows)
        let modified_rainfall = self.orographic.calculate_comprehensive_orographic_effects(
            positions,
            elevations, 
            base_rainfall,
            wind_direction,
        )?;
        
        // Apply enhanced continental effects (with maritime moderation)
        let (modified_temperatures, modified_humidity) = self.continental
            .calculate_enhanced_continental_effects(
                positions,
                base_temperatures,
                base_humidity,
            )?;
        
        Ok((modified_temperatures, modified_rainfall, modified_humidity))
    }
    
    /// High-performance batch processing using complete Zig pipeline
    pub fn apply_effects_batch_optimized(
        &mut self,
        positions: &[(f32, f32)],
        elevations: &[f32],
        base_temperatures: &[i8],
        base_rainfall: &[f32],
        base_humidity: &[u8],
        wind_direction: f32,
    ) -> Result<(Vec<i8>, Vec<f32>, Vec<u8>), String> {
        // Create wind directions array for all positions
        let wind_directions: Vec<f32> = vec![wind_direction; positions.len()];
        
        // Use complete Zig climate processing pipeline for maximum performance
        let (mut modified_temperatures, mut modified_rainfall, mut modified_humidity) = zig_ffi::climate_process_all(
            positions,
            elevations,
            base_temperatures,
            base_rainfall,
            base_humidity,
            &wind_directions,
        )?;

        // Apply additional rain shadow effects if we have mountain ranges
        if !self.orographic.mountain_ranges.is_empty() {
            let zig_mountain_ranges: Vec<super::zig_ffi::MountainRange> = self.orographic.mountain_ranges.iter()
                .map(|range| super::zig_ffi::MountainRange {
                    center: (range.center_x, range.center_y),
                    width: range.width,
                    height: range.height,
                    orientation: range.orientation,
                })
                .collect();

            let shadow_effects = super::zig_ffi::climate_rain_shadow_effects(
                positions,
                elevations,
                &zig_mountain_ranges,
                wind_direction,
                self.orographic.rain_shadow_factor,
            )?;

            // Apply rain shadow to the already processed rainfall
            for (i, &shadow_effect) in shadow_effects.iter().enumerate() {
                if i < modified_rainfall.len() {
                    modified_rainfall[i] *= shadow_effect;
                }
            }
        }

        // Apply maritime influence moderation for coastal areas
        if let Ok(maritime_influences) = self.continental.calculate_maritime_influence_batch(positions) {
            for (i, &maritime_influence) in maritime_influences.iter().enumerate() {
                if i < modified_temperatures.len() {
                    let base_temp = base_temperatures[i] as f32;
                    let processed_temp = modified_temperatures[i] as f32;
                    
                    // Maritime influence moderates temperature extremes
                    let moderated_temp = base_temp + (processed_temp - base_temp) * (1.0 - maritime_influence * 0.3);
                    modified_temperatures[i] = moderated_temp.clamp(-50.0, 50.0) as i8;

                    // Maritime influence increases humidity
                    if i < modified_humidity.len() {
                        let moderated_humidity = (modified_humidity[i] as f32) + maritime_influence * 15.0;
                        modified_humidity[i] = moderated_humidity.clamp(0.0, 100.0) as u8;
                    }
                }
            }
        }
        
        // Clear continental cache after processing
        self.continental.clear_cache();
        
        Ok((modified_temperatures, modified_rainfall, modified_humidity))
    }

    /// Ultra-high performance batch processing using pure Zig pipeline with minimal Rust overhead
    pub fn apply_effects_batch_ultra_optimized(
        &mut self,
        positions: &[(f32, f32)],
        elevations: &[f32],
        base_temperatures: &[i8],
        base_rainfall: &[f32],
        base_humidity: &[u8],
        wind_direction: f32,
    ) -> Result<(Vec<i8>, Vec<f32>, Vec<u8>), String> {
        // For maximum performance, use pure Zig pipeline with minimal post-processing
        let wind_directions: Vec<f32> = vec![wind_direction; positions.len()];
        
        let (modified_temperatures, modified_rainfall, modified_humidity) = zig_ffi::climate_process_all(
            positions,
            elevations,
            base_temperatures,
            base_rainfall,
            base_humidity,
            &wind_directions,
        )?;
        
        // Clear cache and return results with minimal overhead
        self.continental.clear_cache();
        
        Ok((modified_temperatures, modified_rainfall, modified_humidity))
    }
    
    /// Add mountain range for orographic effects
    pub fn add_mountain_range(&mut self, range: MountainRange) {
        self.orographic.add_mountain_range(range);
    }
    
    /// Set world size for continental calculations
    pub fn set_world_size(&mut self, width: u32, height: u32) {
        self.continental.world_size = (width, height);
        self.continental.clear_cache();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_orographic_effects_creation() {
        let effects = OrographicEffects::default();
        assert_eq!(effects.mountain_count(), 0);
        assert_eq!(effects.max_orographic_bonus, 200.0);
    }
    
    #[test]
    fn test_continental_effects() {
        let mut effects = ContinentalEffects::default();
        
        // Test ocean proximity calculation
        let coastal = effects.calculate_ocean_proximity(10.0, 10.0);
        let inland = effects.calculate_ocean_proximity(128.0, 128.0);
        
        assert!(coastal > inland);
        
        // Test temperature effects
        let continental_temp = effects.apply_temperature_effect(20, 0.8);
        let oceanic_temp = effects.apply_temperature_effect(20, 0.2);
        
        assert!(continental_temp != oceanic_temp);
    }
    
    #[test]
    fn test_mountain_range_addition() {
        let mut modifier = ClimateModifier::default();
        
        let range = MountainRange {
            center_x: 100.0,
            center_y: 100.0,
            width: 50.0,
            height: 2000.0,
            orientation: 0.0,
        };
        
        modifier.add_mountain_range(range);
        assert_eq!(modifier.orographic.mountain_count(), 1);
    }

    #[test]
    fn test_comprehensive_orographic_effects() {
        let mut orographic = OrographicEffects::default();
        orographic.add_mountain_range(MountainRange {
            center_x: 150.0,
            center_y: 150.0,
            width: 100.0,
            height: 2000.0,
            orientation: 0.0,
        });

        let positions = vec![(100.0, 100.0), (200.0, 200.0)];
        let elevations = vec![500.0, 1500.0];
        let base_rainfall = vec![100.0, 150.0];

        let results = orographic.calculate_comprehensive_orographic_effects(
            &positions, &elevations, &base_rainfall, 0.0
        );

        assert!(results.is_ok());
        let enhanced_rainfall = results.unwrap();
        assert_eq!(enhanced_rainfall.len(), 2);

        // High elevation should generally get more rainfall after orographic enhancement
        // but may be reduced by rain shadow effects
        assert!(enhanced_rainfall.iter().all(|&r| r >= 0.0));
    }

    #[test]
    fn test_enhanced_continental_effects() {
        let mut continental = ContinentalEffects::default();
        let positions = vec![(10.0, 10.0), (128.0, 128.0)]; // Coast vs inland
        let base_temps = vec![20i8, 20i8];
        let base_humidity = vec![60u8, 60u8];

        let results = continental.calculate_enhanced_continental_effects(
            &positions, &base_temps, &base_humidity
        );

        assert!(results.is_ok());
        let (enhanced_temps, enhanced_humidity) = results.unwrap();
        assert_eq!(enhanced_temps.len(), 2);
        assert_eq!(enhanced_humidity.len(), 2);

        // Maritime influence should moderate continental effects
        // Coastal areas should have less extreme temperatures and higher humidity
        assert!(enhanced_humidity[0] >= enhanced_humidity[1]); // Coast should have higher humidity
    }

    #[test]
    fn test_ultra_optimized_batch_processing() {
        let mut modifier = ClimateModifier::default();
        
        let positions = vec![(100.0, 100.0), (150.0, 150.0)];
        let elevations = vec![500.0, 1000.0];
        let base_temps = vec![20i8, 15i8];
        let base_rainfall = vec![100.0, 150.0];
        let base_humidity = vec![60u8, 70u8];

        let results = modifier.apply_effects_batch_ultra_optimized(
            &positions, &elevations, &base_temps, &base_rainfall, &base_humidity, 0.0
        );

        assert!(results.is_ok());
        let (temps, rainfall, humidity) = results.unwrap();
        
        assert_eq!(temps.len(), 2);
        assert_eq!(rainfall.len(), 2);
        assert_eq!(humidity.len(), 2);

        // Results should be processed but we can't predict exact values
        assert!(temps.iter().all(|&t| t >= -50 && t <= 50));
        assert!(rainfall.iter().all(|&r| r >= 0.0));
        assert!(humidity.iter().all(|&h| h <= 100));
    }

    #[test]
    fn test_maritime_influence_batch() {
        let continental = ContinentalEffects::default();
        let positions = vec![(10.0, 10.0), (128.0, 128.0), (200.0, 200.0)];

        let maritime_result = continental.calculate_maritime_influence_batch(&positions);
        assert!(maritime_result.is_ok());

        let influences = maritime_result.unwrap();
        assert_eq!(influences.len(), 3);

        // Maritime influence should decrease with distance from coast
        assert!(influences[0] >= influences[1]); // Coast > inland
        assert!(influences[1] >= influences[2]); // Mid > far inland (generally)
        
        // All values should be valid
        assert!(influences.iter().all(|&i| i >= 0.0 && i <= 3.0)); // Maritime influence can exceed 1.0
    }
}
