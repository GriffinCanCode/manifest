//! Modular Tile Properties System
//!
//! This module has been refactored from a large monolithic properties.rs file into focused submodules:
//! - `terrain`: Enhanced terrain types and conversions
//! - `elevation`: Elevation data structures with noise generation support
//! - `climate`: Climate data structures with interpolation support
//! - `biome`: Biome definitions, modifiers, and related structures  
//! - `resources`: Resource configurations and definitions
//! - `improvement`: Tile improvement structures with Lua scripting
//! - `movement`: Movement cost calculations with bitset optimizations
//! - `defense`: Defense bonus system with ordered float precision
//! - `fog`: Fog of war system with bitvec visibility tracking
//! - `culture`: Cultural influence system with concurrent access
//! - `manager`: Main TilePropertiesSystem and ECS systems

pub mod terrain;
pub mod elevation;
pub mod climate;
pub mod biome;
pub mod resources;
pub mod improvement;
pub mod movement;
pub mod defense;
pub mod fog;
pub mod culture;
pub mod manager;

// Re-export commonly used types from terrain
pub use terrain::EnhancedTerrainType;

// Re-export commonly used types from elevation
pub use elevation::{Elevation, ElevationTier};

// Re-export commonly used types from climate
pub use climate::{EnhancedClimate, ClimateInterpolation, VegetationType};

// Re-export commonly used types from biome
pub use biome::{
    Biome, BiomeDefinition, BiomeModifiers, ClimateRequirements,
    BiomeCategory, BiomeSuitabilityCalculator
};

// Re-export commonly used types from resources
pub use resources::{
    ResourceConfig, ResourceDefinition, ResourceCategory, ResourceYields,
    ResourceSpawner
};

// Re-export commonly used types from improvement
pub use improvement::{TileImprovement, ImprovementEffects, ImprovementCategory};

// Re-export commonly used types from movement
pub use movement::{
    MovementCosts, UnitType, HexDirection, WeatherEffect, MovementCategory
};

// Re-export commonly used types from defense
pub use defense::{
    DefenseBonuses, DefenseCategory, DefenseBreakdown, DefenseComparison, DefenseSource
};

// Re-export commonly used types from fog
pub use fog::{FogOfWar, VisionLevel, FogStatus};

// Re-export commonly used types from culture
pub use culture::{CulturalInfluence, PlayerCulture};

// Re-export the main system and ECS systems
pub use manager::{
    TilePropertiesSystem,
    update_tile_properties,
    update_cultural_influence,
    process_tile_property_changes,
};

// Convenient type aliases
pub type PropertiesResult<T> = Result<T, crate::scripting::ScriptError>;

/// Trait for objects that have comprehensive tile properties
pub trait TileProperties {
    /// Get terrain type
    fn get_terrain(&self) -> EnhancedTerrainType;
    
    /// Get elevation data
    fn get_elevation(&self) -> &Elevation;
    
    /// Get climate data
    fn get_climate(&self) -> &EnhancedClimate;
    
    /// Get biome information
    fn get_biome(&self) -> &Biome;
    
    /// Get movement costs
    fn get_movement_costs(&self) -> &MovementCosts;
    
    /// Get defense bonuses
    fn get_defense_bonuses(&self) -> &DefenseBonuses;
    
    /// Check if tile has specific property
    fn has_property(&self, property: TilePropertyType) -> bool;
}

/// Types of tile properties for querying
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TilePropertyType {
    Terrain,
    Elevation,
    Climate,
    Biome,
    Resources,
    Improvements,
    Movement,
    Defense,
    Visibility,
    Culture,
}

impl TilePropertyType {
    /// Get property type name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Terrain => "terrain",
            Self::Elevation => "elevation",
            Self::Climate => "climate", 
            Self::Biome => "biome",
            Self::Resources => "resources",
            Self::Improvements => "improvements",
            Self::Movement => "movement",
            Self::Defense => "defense",
            Self::Visibility => "visibility",
            Self::Culture => "culture",
        }
    }

    /// Get all property types
    pub fn all_types() -> Vec<TilePropertyType> {
        vec![
            TilePropertyType::Terrain,
            TilePropertyType::Elevation,
            TilePropertyType::Climate,
            TilePropertyType::Biome,
            TilePropertyType::Resources,
            TilePropertyType::Improvements,
            TilePropertyType::Movement,
            TilePropertyType::Defense,
            TilePropertyType::Visibility,
            TilePropertyType::Culture,
        ]
    }
}

/// Utility functions for tile properties management
pub mod utils {
    use super::*;
    

    /// Calculate comprehensive tile suitability for various purposes
    pub fn calculate_tile_suitability(
        terrain: EnhancedTerrainType,
        elevation: &Elevation,
        climate: &EnhancedClimate,
        biome: &Biome,
    ) -> TileSuitability {
        TileSuitability {
            agriculture: calculate_agriculture_suitability(terrain, climate, biome),
            settlement: calculate_settlement_suitability(elevation, climate, biome),
            military: calculate_military_suitability(elevation, terrain, biome),
            trade: calculate_trade_suitability(terrain, elevation),
            resource_extraction: calculate_resource_suitability(terrain, biome),
        }
    }

    fn calculate_agriculture_suitability(
        terrain: EnhancedTerrainType,
        climate: &EnhancedClimate,
        biome: &Biome,
    ) -> f32 {
        let terrain_factor = match terrain {
            EnhancedTerrainType::Grassland | EnhancedTerrainType::Plains => 1.0,
            EnhancedTerrainType::Forest => 0.7,
            EnhancedTerrainType::Hills => 0.6,
            EnhancedTerrainType::Desert | EnhancedTerrainType::Mountain => 0.2,
            EnhancedTerrainType::Ocean => 0.0,
            _ => 0.5,
        };

        let climate_factor = climate.agricultural_suitability();
        let biome_factor = biome.effective_agriculture_yield();

        (terrain_factor * climate_factor * biome_factor).min(1.0)
    }

    fn calculate_settlement_suitability(
        elevation: &Elevation,
        climate: &EnhancedClimate,
        biome: &Biome,
    ) -> f32 {
        let elevation_factor = if elevation.final_elevation < 0.0 {
            0.0 // Can't settle underwater
        } else if elevation.final_elevation > 3000.0 {
            0.3 // High altitude is difficult
        } else {
            1.0
        };

        let climate_factor = climate.comfort_rating();
        let biome_factor = biome.effective_population_capacity();

        (elevation_factor * climate_factor * biome_factor).min(1.0)
    }

    fn calculate_military_suitability(
        elevation: &Elevation,
        terrain: EnhancedTerrainType,
        biome: &Biome,
    ) -> f32 {
        let elevation_factor = if elevation.has_defensive_advantage() { 1.0 } else { 0.5 };
        
        let terrain_factor = match terrain {
            EnhancedTerrainType::Mountain | EnhancedTerrainType::Hills => 1.0,
            EnhancedTerrainType::Forest | EnhancedTerrainType::Swamp => 0.8,
            EnhancedTerrainType::Plains | EnhancedTerrainType::Grassland => 0.6,
            EnhancedTerrainType::Ocean => 0.0,
            _ => 0.4,
        };

        let biome_factor = 1.0 + biome.effective_defense_bonus();

        (elevation_factor * terrain_factor * biome_factor).min(1.0)
    }

    fn calculate_trade_suitability(terrain: EnhancedTerrainType, elevation: &Elevation) -> f32 {
        let terrain_factor = match terrain {
            EnhancedTerrainType::Plains | EnhancedTerrainType::Grassland => 1.0,
            EnhancedTerrainType::Hills => 0.7,
            EnhancedTerrainType::Ocean => 0.9, // For ports
            EnhancedTerrainType::Desert => 0.6,
            EnhancedTerrainType::Mountain | EnhancedTerrainType::Swamp => 0.3,
            _ => 0.5,
        };

        let elevation_factor = if elevation.final_elevation < 0.0 {
            0.9 // Water trade
        } else if elevation.final_elevation > 2000.0 {
            0.4 // High altitude is harder for trade
        } else {
            1.0
        };

        terrain_factor * elevation_factor
    }

    fn calculate_resource_suitability(terrain: EnhancedTerrainType, biome: &Biome) -> f32 {
        let terrain_factor = match terrain {
            EnhancedTerrainType::Mountain | EnhancedTerrainType::Hills => 1.0, // Mining
            EnhancedTerrainType::Forest | EnhancedTerrainType::Jungle => 0.8, // Lumber
            EnhancedTerrainType::Desert => 0.7, // Oil, gems
            EnhancedTerrainType::Ocean => 0.6, // Fish, oil
            _ => 0.5,
        };

        let biome_factor = biome.effective_mining_yield();

        terrain_factor * biome_factor
    }

    /// Get recommended improvements for a tile
    pub fn get_recommended_improvements(
        terrain: EnhancedTerrainType,
        biome: &Biome,
        suitability: &TileSuitability,
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        // Agriculture improvements
        if suitability.agriculture > 0.6 {
            match terrain {
                EnhancedTerrainType::Grassland | EnhancedTerrainType::Plains => {
                    recommendations.push("farm".to_string());
                    recommendations.push("irrigation".to_string());
                },
                _ => {},
            }
        }

        // Resource extraction
        if suitability.resource_extraction > 0.7 {
            match terrain {
                EnhancedTerrainType::Mountain | EnhancedTerrainType::Hills => {
                    recommendations.push("mine".to_string());
                    recommendations.push("quarry".to_string());
                },
                EnhancedTerrainType::Forest | EnhancedTerrainType::Jungle => {
                    recommendations.push("lumbermill".to_string());
                },
                _ => {},
            }
        }

        // Military improvements
        if suitability.military > 0.7 {
            recommendations.push("fort".to_string());
            if terrain == EnhancedTerrainType::Hills || terrain == EnhancedTerrainType::Mountain {
                recommendations.push("watchtower".to_string());
            }
        }

        // Trade improvements
        if suitability.trade > 0.7 {
            recommendations.push("road".to_string());
            if terrain == EnhancedTerrainType::Ocean {
                recommendations.push("port".to_string());
            } else {
                recommendations.push("trading_post".to_string());
            }
        }

        recommendations
    }
}

/// Comprehensive tile suitability ratings
#[derive(Debug, Clone)]
pub struct TileSuitability {
    pub agriculture: f32,
    pub settlement: f32,
    pub military: f32,
    pub trade: f32,
    pub resource_extraction: f32,
}

impl TileSuitability {
    /// Get overall suitability score
    pub fn overall_score(&self) -> f32 {
        (self.agriculture + self.settlement + self.military + self.trade + self.resource_extraction) / 5.0
    }

    /// Get best use case for this tile
    pub fn best_use(&self) -> TileUseCase {
        let scores = [
            (TileUseCase::Agriculture, self.agriculture),
            (TileUseCase::Settlement, self.settlement),
            (TileUseCase::Military, self.military),
            (TileUseCase::Trade, self.trade),
            (TileUseCase::ResourceExtraction, self.resource_extraction),
        ];

        scores.iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(use_case, _)| *use_case)
            .unwrap_or(TileUseCase::Settlement)
    }
}

/// Primary use cases for tiles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TileUseCase {
    Agriculture,
    Settlement,
    Military,
    Trade,
    ResourceExtraction,
}

impl TileUseCase {
    /// Get use case description
    pub fn description(&self) -> &'static str {
        match self {
            Self::Agriculture => "Best suited for farming and food production",
            Self::Settlement => "Ideal for cities and population centers",
            Self::Military => "Strategic location for military installations",
            Self::Trade => "Excellent for trade routes and commerce",
            Self::ResourceExtraction => "Rich in natural resources for extraction",
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_comprehensive_tile_analysis() {
        let terrain = EnhancedTerrainType::Grassland;
        let elevation = Elevation::new(100.0);
        let climate = EnhancedClimate::new(20, 200, 60);
        let biome = Biome::new("temperate_grassland".to_string(), 0.9);

        let suitability = utils::calculate_tile_suitability(terrain, &elevation, &climate, &biome);
        
        // Grassland should be good for agriculture and settlement
        assert!(suitability.agriculture > 0.7);
        assert!(suitability.settlement > 0.5);
        assert_eq!(suitability.best_use(), TileUseCase::Agriculture);
    }

    #[test] 
    fn test_mountain_tile_analysis() {
        let terrain = EnhancedTerrainType::Mountain;
        let elevation = Elevation::new(2500.0);
        let climate = EnhancedClimate::new(5, 150, 50);
        let biome = Biome::new("alpine".to_string(), 0.6);

        let suitability = utils::calculate_tile_suitability(terrain, &elevation, &climate, &biome);
        
        // Mountain should be good for military and resource extraction
        assert!(suitability.military > 0.6);
        assert!(suitability.resource_extraction > 0.5);
        assert!(suitability.agriculture < 0.5); // Not good for farming
    }

    #[test]
    fn test_recommended_improvements() {
        let terrain = EnhancedTerrainType::Hills;
        let biome = Biome::new("temperate_hills".to_string(), 0.8);
        let elevation = Elevation::new(600.0);
        let climate = EnhancedClimate::new(15, 180, 55);
        
        let suitability = utils::calculate_tile_suitability(terrain, &elevation, &climate, &biome);
        let recommendations = utils::get_recommended_improvements(terrain, &biome, &suitability);
        
        // Hills should recommend military and resource improvements
        assert!(!recommendations.is_empty());
        // Exact recommendations depend on suitability scores
    }

    #[test]
    fn test_property_type_enumeration() {
        let all_types = TilePropertyType::all_types();
        assert_eq!(all_types.len(), 10);
        assert!(all_types.contains(&TilePropertyType::Terrain));
        assert!(all_types.contains(&TilePropertyType::Culture));
        
        assert_eq!(TilePropertyType::Biome.name(), "biome");
    }

    #[test]
    fn test_tile_suitability_overall_score() {
        let suitability = TileSuitability {
            agriculture: 0.8,
            settlement: 0.6,
            military: 0.4,
            trade: 0.7,
            resource_extraction: 0.5,
        };
        
        let overall = suitability.overall_score();
        assert!((overall - 0.6).abs() < 0.01); // (0.8+0.6+0.4+0.7+0.5)/5 = 0.6
        assert_eq!(suitability.best_use(), TileUseCase::Agriculture);
    }
}
