//! Biome system with definition loading and modifiers
//!
//! Provides biome classification, definitions loaded from RON files,
//! and gameplay modifiers for different biome types.

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::core::hashing::FastHashMap;

/// Biome definition loaded from RON files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeDefinition {
    pub name: String,
    pub description: String,
    pub climate_requirements: ClimateRequirements,
    pub terrain_preferences: Vec<String>,
    pub modifiers: BiomeModifiers,
    pub special_resources: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClimateRequirements {
    pub temperature_range: (i8, i8),
    pub rainfall_range: (u16, u16),
    pub elevation_range: Option<(f32, f32)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeModifiers {
    pub movement_cost_multiplier: f32,
    pub defense_bonus: f32,
    pub agriculture_yield: f32,
    pub mining_yield: f32,
    pub population_capacity: f32,
}

impl Default for BiomeModifiers {
    fn default() -> Self {
        Self {
            movement_cost_multiplier: 1.0,
            defense_bonus: 0.0,
            agriculture_yield: 1.0,
            mining_yield: 1.0,
            population_capacity: 1.0,
        }
    }
}

/// Biome component for tiles
#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub struct Biome {
    pub biome_type: String,
    pub suitability_score: f32,
    pub modifiers: BiomeModifiers,
}

impl Default for Biome {
    fn default() -> Self {
        Self {
            biome_type: "temperate_grassland".to_string(),
            suitability_score: 1.0,
            modifiers: BiomeModifiers::default(),
        }
    }
}

impl Biome {
    /// Create new biome with type and suitability
    pub fn new(biome_type: String, suitability_score: f32) -> Self {
        Self {
            biome_type,
            suitability_score,
            modifiers: BiomeModifiers::default(),
        }
    }

    /// Create biome with custom modifiers
    pub fn with_modifiers(biome_type: String, suitability_score: f32, modifiers: BiomeModifiers) -> Self {
        Self {
            biome_type,
            suitability_score,
            modifiers,
        }
    }

    /// Get effective movement cost multiplier
    pub fn effective_movement_cost(&self) -> f32 {
        self.modifiers.movement_cost_multiplier * self.suitability_score
    }

    /// Get effective defense bonus
    pub fn effective_defense_bonus(&self) -> f32 {
        self.modifiers.defense_bonus * self.suitability_score
    }

    /// Get effective agriculture yield
    pub fn effective_agriculture_yield(&self) -> f32 {
        self.modifiers.agriculture_yield * self.suitability_score
    }

    /// Get effective mining yield
    pub fn effective_mining_yield(&self) -> f32 {
        self.modifiers.mining_yield * self.suitability_score
    }

    /// Get effective population capacity
    pub fn effective_population_capacity(&self) -> f32 {
        self.modifiers.population_capacity * self.suitability_score
    }

    /// Check if biome is habitable
    pub fn is_habitable(&self) -> bool {
        self.effective_population_capacity() > 0.1
    }

    /// Get biome category
    pub fn category(&self) -> BiomeCategory {
        match self.biome_type.as_str() {
            "ocean" | "deep_ocean" | "coastal" => BiomeCategory::Aquatic,
            "polar" | "arctic_tundra" | "glacial" => BiomeCategory::Polar,
            "arid_desert" | "desert_oasis" => BiomeCategory::Desert,
            "tropical_rainforest" | "tropical_forest" | "tropical_grassland" => BiomeCategory::Tropical,
            "temperate_grassland" | "temperate_forest" | "temperate_plains" | "temperate_hills" => BiomeCategory::Temperate,
            "alpine" | "highland" | "volcanic" => BiomeCategory::Mountain,
            "wetland" | "swamp" => BiomeCategory::Wetland,
            "savanna" | "steppe" => BiomeCategory::Grassland,
            _ => BiomeCategory::Temperate,
        }
    }
}

/// Biome categories for grouping similar biomes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BiomeCategory {
    Aquatic,
    Polar,
    Desert,
    Tropical,
    Temperate,
    Mountain,
    Wetland,
    Grassland,
}

impl BiomeCategory {
    /// Get base movement cost for this biome category
    pub fn base_movement_cost(&self) -> f32 {
        match self {
            Self::Aquatic => 3.0,
            Self::Polar => 2.5,
            Self::Desert => 2.0,
            Self::Tropical => 2.0,
            Self::Temperate => 1.0,
            Self::Mountain => 3.0,
            Self::Wetland => 2.5,
            Self::Grassland => 1.0,
        }
    }

    /// Get base defense bonus for this biome category
    pub fn base_defense_bonus(&self) -> f32 {
        match self {
            Self::Aquatic => 0.0,
            Self::Polar => 0.1,
            Self::Desert => 0.05,
            Self::Tropical => 0.15,
            Self::Temperate => 0.0,
            Self::Mountain => 0.25,
            Self::Wetland => 0.1,
            Self::Grassland => 0.0,
        }
    }

    /// Get habitability rating for this biome category
    pub fn habitability_rating(&self) -> f32 {
        match self {
            Self::Aquatic => 0.0,
            Self::Polar => 0.2,
            Self::Desert => 0.3,
            Self::Tropical => 0.9,
            Self::Temperate => 1.0,
            Self::Mountain => 0.4,
            Self::Wetland => 0.6,
            Self::Grassland => 0.9,
        }
    }
}

/// Biome suitability calculator
pub struct BiomeSuitabilityCalculator;

impl BiomeSuitabilityCalculator {
    /// Calculate biome suitability based on climate and terrain
    pub fn calculate_suitability(
        climate_temp: i8,
        climate_rainfall: u16,
        climate_humidity: u8,
        elevation: f32,
        terrain_type: &str,
        biome_def: &BiomeDefinition,
    ) -> f32 {
        let mut score = 1.0;

        // Check temperature requirements
        let temp_range = &biome_def.climate_requirements.temperature_range;
        if climate_temp < temp_range.0 || climate_temp > temp_range.1 {
            let temp_diff = if climate_temp < temp_range.0 {
                temp_range.0 - climate_temp
            } else {
                climate_temp - temp_range.1
            } as f32;
            score *= (1.0 - (temp_diff * 0.1)).max(0.0);
        }

        // Check rainfall requirements
        let rainfall_range = &biome_def.climate_requirements.rainfall_range;
        if climate_rainfall < rainfall_range.0 || climate_rainfall > rainfall_range.1 {
            let rainfall_diff = if climate_rainfall < rainfall_range.0 {
                rainfall_range.0 - climate_rainfall
            } else {
                climate_rainfall - rainfall_range.1
            } as f32;
            score *= (1.0 - (rainfall_diff * 0.001)).max(0.0);
        }

        // Check elevation requirements if specified
        if let Some(elev_range) = &biome_def.climate_requirements.elevation_range {
            if elevation < elev_range.0 || elevation > elev_range.1 {
                let elev_diff = if elevation < elev_range.0 {
                    elev_range.0 - elevation
                } else {
                    elevation - elev_range.1
                };
                score *= (1.0 - (elev_diff * 0.0001)).max(0.0);
            }
        }

        // Check terrain preferences
        if !biome_def.terrain_preferences.contains(&terrain_type.to_string()) {
            score *= 0.5; // Penalty for non-preferred terrain
        }

        score.max(0.0).min(1.0)
    }

    /// Find best matching biome from definitions
    pub fn find_best_biome(
        climate_temp: i8,
        climate_rainfall: u16,
        climate_humidity: u8,
        elevation: f32,
        terrain_type: &str,
        biome_definitions: &FastHashMap<String, BiomeDefinition>,
    ) -> Option<(String, f32)> {
        let mut best_biome = None;
        let mut best_score = 0.0;

        for (biome_name, biome_def) in biome_definitions.iter() {
            let score = Self::calculate_suitability(
                climate_temp,
                climate_rainfall,
                climate_humidity,
                elevation,
                terrain_type,
                biome_def,
            );

            if score > best_score {
                best_score = score;
                best_biome = Some((biome_name.clone(), score));
            }
        }

        best_biome
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_biome_creation() {
        let biome = Biome::new("tropical_rainforest".to_string(), 0.9);
        assert_eq!(biome.biome_type, "tropical_rainforest");
        assert_eq!(biome.suitability_score, 0.9);
        assert_eq!(biome.category(), BiomeCategory::Tropical);
    }

    #[test]
    fn test_effective_values() {
        let modifiers = BiomeModifiers {
            movement_cost_multiplier: 2.0,
            defense_bonus: 0.5,
            agriculture_yield: 1.5,
            mining_yield: 0.5,
            population_capacity: 0.8,
        };
        
        let biome = Biome::with_modifiers("alpine".to_string(), 0.6, modifiers);
        
        assert_eq!(biome.effective_movement_cost(), 1.2); // 2.0 * 0.6
        assert_eq!(biome.effective_defense_bonus(), 0.3); // 0.5 * 0.6
        assert_eq!(biome.effective_agriculture_yield(), 0.9); // 1.5 * 0.6
    }

    #[test]
    fn test_biome_categories() {
        assert_eq!(BiomeCategory::Mountain.base_movement_cost(), 3.0);
        assert_eq!(BiomeCategory::Mountain.base_defense_bonus(), 0.25);
        assert_eq!(BiomeCategory::Temperate.habitability_rating(), 1.0);
        assert_eq!(BiomeCategory::Polar.habitability_rating(), 0.2);
    }

    #[test]
    fn test_habitability() {
        let habitable_biome = Biome::with_modifiers(
            "temperate_grassland".to_string(),
            0.8,
            BiomeModifiers {
                population_capacity: 1.0,
                ..Default::default()
            },
        );
        assert!(habitable_biome.is_habitable());

        let uninhabitable_biome = Biome::with_modifiers(
            "polar".to_string(),
            0.5,
            BiomeModifiers {
                population_capacity: 0.05,
                ..Default::default()
            },
        );
        assert!(!uninhabitable_biome.is_habitable());
    }

    #[test]
    fn test_suitability_calculation() {
        let biome_def = BiomeDefinition {
            name: "Test Biome".to_string(),
            description: "Test".to_string(),
            climate_requirements: ClimateRequirements {
                temperature_range: (15, 25),
                rainfall_range: (100, 300),
                elevation_range: Some((0.0, 1000.0)),
            },
            terrain_preferences: vec!["grassland".to_string()],
            modifiers: BiomeModifiers::default(),
            special_resources: vec![],
        };

        // Perfect match
        let perfect_score = BiomeSuitabilityCalculator::calculate_suitability(
            20, 200, 50, 500.0, "grassland", &biome_def
        );
        assert!(perfect_score > 0.9);

        // Poor match
        let poor_score = BiomeSuitabilityCalculator::calculate_suitability(
            0, 50, 20, 2000.0, "ocean", &biome_def
        );
        assert!(poor_score < 0.5);
    }

    #[test]
    fn test_best_biome_selection() {
        let mut biome_definitions = crate::core::hashing::collections::fast_hash_map();
        
        biome_definitions.insert("temperate".to_string(), BiomeDefinition {
            name: "Temperate".to_string(),
            description: "Temperate biome".to_string(),
            climate_requirements: ClimateRequirements {
                temperature_range: (10, 25),
                rainfall_range: (100, 300),
                elevation_range: None,
            },
            terrain_preferences: vec!["grassland".to_string()],
            modifiers: BiomeModifiers::default(),
            special_resources: vec![],
        });

        biome_definitions.insert("tropical".to_string(), BiomeDefinition {
            name: "Tropical".to_string(),
            description: "Tropical biome".to_string(),
            climate_requirements: ClimateRequirements {
                temperature_range: (25, 35),
                rainfall_range: (200, 500),
                elevation_range: None,
            },
            terrain_preferences: vec!["jungle".to_string()],
            modifiers: BiomeModifiers::default(),
            special_resources: vec![],
        });

        let result = BiomeSuitabilityCalculator::find_best_biome(
            15, 150, 60, 200.0, "grassland", &biome_definitions
        );

        assert!(result.is_some());
        let (biome_name, score) = result.unwrap();
        assert_eq!(biome_name, "temperate");
        assert!(score > 0.5);
    }
}
