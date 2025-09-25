//! Climate data structures with interpolation support
//!
//! Provides climate components for tiles with smooth transitions
//! and climate zone classification for gameplay mechanics.

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::world::tiles::components::Climate;

/// Enhanced climate data with interpolation support
#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub struct EnhancedClimate {
    /// Temperature in Celsius (-50 to 50)
    pub temperature: i8,
    /// Annual rainfall in mm (0-500)
    pub rainfall: u16,
    /// Humidity percentage (0-100)
    pub humidity: u8,
    /// Wind strength (0-255)
    pub wind_strength: u8,
    /// Seasonal temperature variation
    pub temperature_variation: u8,
    /// Interpolated values for smooth transitions
    pub interpolated: ClimateInterpolation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClimateInterpolation {
    /// Interpolated temperature (for smooth climate transitions)
    pub smooth_temperature: f32,
    /// Interpolated rainfall
    pub smooth_rainfall: f32,
    /// Climate zone identifier
    pub climate_zone: String,
}

impl Default for EnhancedClimate {
    fn default() -> Self {
        Self {
            temperature: 15,
            rainfall: 100,
            humidity: 50,
            wind_strength: 50,
            temperature_variation: 10,
            interpolated: ClimateInterpolation {
                smooth_temperature: 15.0,
                smooth_rainfall: 100.0,
                climate_zone: "temperate".to_string(),
            },
        }
    }
}

impl From<Climate> for EnhancedClimate {
    fn from(climate: Climate) -> Self {
        let climate_zone = Self::determine_climate_zone(
            climate.temperature,
            climate.rainfall as u16,
            climate.humidity
        );
        
        Self {
            temperature: climate.temperature,
            rainfall: climate.rainfall as u16,
            humidity: climate.humidity,
            wind_strength: climate.wind_strength,
            temperature_variation: 10, // Default variation
            interpolated: ClimateInterpolation {
                smooth_temperature: climate.temperature as f32,
                smooth_rainfall: climate.rainfall as f32,
                climate_zone,
            },
        }
    }
}

impl EnhancedClimate {
    /// Create new enhanced climate
    pub fn new(temperature: i8, rainfall: u16, humidity: u8) -> Self {
        let climate_zone = Self::determine_climate_zone(temperature, rainfall, humidity);
        
        Self {
            temperature,
            rainfall,
            humidity,
            wind_strength: 50,
            temperature_variation: 10,
            interpolated: ClimateInterpolation {
                smooth_temperature: temperature as f32,
                smooth_rainfall: rainfall as f32,
                climate_zone,
            },
        }
    }

    /// Update interpolated values for smooth transitions
    pub fn update_interpolation(&mut self, neighboring_climates: &[EnhancedClimate]) {
        if neighboring_climates.is_empty() {
            return;
        }

        let total_count = neighboring_climates.len() as f32;
        let neighbor_temp: f32 = neighboring_climates.iter()
            .map(|c| c.temperature as f32)
            .sum::<f32>() / total_count;
        let neighbor_rainfall: f32 = neighboring_climates.iter()
            .map(|c| c.rainfall as f32)
            .sum::<f32>() / total_count;

        // Blend with neighbors for smooth transitions
        self.interpolated.smooth_temperature = (self.temperature as f32 * 0.7) + (neighbor_temp * 0.3);
        self.interpolated.smooth_rainfall = (self.rainfall as f32 * 0.7) + (neighbor_rainfall * 0.3);

        // Update climate zone based on interpolated values
        self.interpolated.climate_zone = Self::determine_climate_zone_f32(
            self.interpolated.smooth_temperature,
            self.interpolated.smooth_rainfall,
            self.humidity
        );
    }

    /// Determine climate zone based on temperature and rainfall
    fn determine_climate_zone(temperature: i8, rainfall: u16, humidity: u8) -> String {
        Self::determine_climate_zone_f32(temperature as f32, rainfall as f32, humidity)
    }

    fn determine_climate_zone_f32(temperature: f32, rainfall: f32, humidity: u8) -> String {
        match (temperature, rainfall, humidity) {
            // Polar/Arctic
            (t, _, _) if t < -10.0 => "polar".to_string(),
            
            // Cold climates
            (t, r, _) if t < 5.0 && r < 200.0 => "arctic_tundra".to_string(),
            (t, r, _) if t < 5.0 && r >= 200.0 => "subarctic".to_string(),
            
            // Temperate climates
            (t, r, h) if t >= 5.0 && t < 20.0 && r < 100.0 => "temperate_dry".to_string(),
            (t, r, h) if t >= 5.0 && t < 20.0 && r >= 100.0 && r < 300.0 => "temperate".to_string(),
            (t, r, h) if t >= 5.0 && t < 20.0 && r >= 300.0 => "temperate_wet".to_string(),
            
            // Warm climates
            (t, r, h) if t >= 20.0 && t < 30.0 && r < 50.0 => "arid".to_string(),
            (t, r, h) if t >= 20.0 && t < 30.0 && r >= 50.0 && r < 200.0 => "mediterranean".to_string(),
            (t, r, h) if t >= 20.0 && t < 30.0 && r >= 200.0 && h < 60 => "subtropical".to_string(),
            (t, r, h) if t >= 20.0 && t < 30.0 && r >= 200.0 && h >= 60 => "humid_subtropical".to_string(),
            
            // Hot climates
            (t, r, _) if t >= 30.0 && r < 100.0 => "desert".to_string(),
            (t, r, h) if t >= 30.0 && r >= 100.0 && r < 300.0 && h < 70 => "tropical_dry".to_string(),
            (t, r, h) if t >= 30.0 && r >= 300.0 || (r >= 200.0 && h >= 70) => "tropical_rainforest".to_string(),
            
            // Fallback
            _ => "temperate".to_string(),
        }
    }

    /// Get climate comfort rating (0.0 to 1.0)
    pub fn comfort_rating(&self) -> f32 {
        let temp_comfort = match self.temperature {
            t if t < -20 => 0.1,
            t if t < 0 => 0.3,
            t if t >= 0 && t < 30 => 1.0,
            t if t < 40 => 0.7,
            _ => 0.2,
        };

        let rainfall_comfort = match self.rainfall {
            r if r < 50 => 0.3,
            r if r < 400 => 1.0,
            _ => 0.6,
        };

        let humidity_comfort = match self.humidity {
            h if h < 30 => 0.6,
            h if h < 70 => 1.0,
            _ => 0.4,
        };

        (temp_comfort + rainfall_comfort + humidity_comfort) / 3.0
    }

    /// Get agricultural suitability (0.0 to 1.0)
    pub fn agricultural_suitability(&self) -> f32 {
        let temp_suit = match self.temperature {
            t if t < -10 => 0.0,
            t if t < 0 => 0.2,
            t if t >= 0 && t < 35 => 1.0,
            _ => 0.3,
        };

        let rain_suit = match self.rainfall {
            r if r < 100 => (r as f32 / 100.0).min(1.0),
            r if r < 500 => 1.0,
            _ => 0.7,
        };

        (temp_suit + rain_suit) / 2.0
    }

    /// Check if climate supports specific vegetation type
    pub fn supports_vegetation(&self, vegetation_type: VegetationType) -> bool {
        match vegetation_type {
            VegetationType::Desert => self.rainfall < 100 && self.temperature > 10,
            VegetationType::Grassland => self.rainfall >= 100 && self.rainfall < 300 && self.temperature > 0,
            VegetationType::Forest => self.rainfall >= 200 && self.temperature > 5 && self.temperature < 35,
            VegetationType::Jungle => self.rainfall >= 300 && self.temperature > 20 && self.humidity > 60,
            VegetationType::Tundra => self.temperature < 5 && self.rainfall > 50,
            VegetationType::Alpine => self.temperature < 10, // Elevation would also factor in
        }
    }
}

/// Vegetation types supported by different climates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VegetationType {
    Desert,
    Grassland,
    Forest,
    Jungle,
    Tundra,
    Alpine,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_climate_zone_determination() {
        let tropical = EnhancedClimate::new(28, 400, 75);
        assert_eq!(tropical.interpolated.climate_zone, "tropical_rainforest");

        let desert = EnhancedClimate::new(35, 30, 25);
        assert_eq!(desert.interpolated.climate_zone, "desert");

        let temperate = EnhancedClimate::new(15, 200, 55);
        assert_eq!(temperate.interpolated.climate_zone, "temperate");
    }

    #[test]
    fn test_comfort_rating() {
        let comfortable = EnhancedClimate::new(20, 150, 50);
        assert!(comfortable.comfort_rating() > 0.8);

        let harsh = EnhancedClimate::new(-25, 20, 90);
        assert!(harsh.comfort_rating() < 0.5);
    }

    #[test]
    fn test_agricultural_suitability() {
        let good_farming = EnhancedClimate::new(22, 250, 60);
        assert!(good_farming.agricultural_suitability() > 0.8);

        let poor_farming = EnhancedClimate::new(-5, 30, 20);
        assert!(poor_farming.agricultural_suitability() < 0.3);
    }

    #[test]
    fn test_vegetation_support() {
        let jungle_climate = EnhancedClimate::new(26, 350, 80);
        assert!(jungle_climate.supports_vegetation(VegetationType::Jungle));
        assert!(!jungle_climate.supports_vegetation(VegetationType::Desert));

        let desert_climate = EnhancedClimate::new(32, 40, 20);
        assert!(desert_climate.supports_vegetation(VegetationType::Desert));
        assert!(!desert_climate.supports_vegetation(VegetationType::Forest));
    }

    #[test]
    fn test_interpolation_update() {
        let mut climate = EnhancedClimate::new(20, 200, 50);
        let neighbors = vec![
            EnhancedClimate::new(25, 250, 55),
            EnhancedClimate::new(18, 180, 45),
        ];

        climate.update_interpolation(&neighbors);

        // Should be blended with neighbors
        assert!(climate.interpolated.smooth_temperature > 20.0);
        assert!(climate.interpolated.smooth_temperature < 25.0);
        assert!(climate.interpolated.smooth_rainfall > 200.0);
    }

    #[test]
    fn test_climate_conversion() {
        let basic_climate = Climate {
            temperature: 18,
            rainfall: 180,
            humidity: 60,
            wind_strength: 40,
        };

        let enhanced: EnhancedClimate = basic_climate.into();
        assert_eq!(enhanced.temperature, 18);
        assert_eq!(enhanced.rainfall, 180);
        assert_eq!(enhanced.humidity, 60);
        assert_eq!(enhanced.wind_strength, 40);
    }
}
