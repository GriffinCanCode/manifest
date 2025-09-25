//! Modular tile improvements system
//!
//! This module has been refactored from a large monolithic file into focused submodules:
//! - `types`: Core types, enums, and keys (ImprovementType, ImprovementKey, etc.)
//! - `core`: Basic Improvement struct and methods
//! - `container`: TileImprovements container for managing improvements on a tile
//! - `stats`: Statistics, results, and error types

pub mod types;
pub mod core;
pub mod container;
pub mod stats;

// Re-export commonly used types and functions
pub use types::{
    ImprovementKey, ImprovementType, ImprovementCategory, ImprovementState,
    MAX_IMPROVEMENTS_PER_TILE
};

pub use core::{
    Improvement, ImprovementProperties, ResourceYields
};

pub use container::TileImprovements;

pub use stats::{
    ImprovementError, ImprovementResult, ImprovementTurnResults,
    ImprovementStats, ImprovementCompletionInfo, ImprovementUpgradeInfo,
    ImprovementDestructionInfo, DestructionCause, StateBreakdown
};

// Convenient type aliases
pub type ImprovementManagerResult<T> = Result<T, ImprovementError>;

/// Trait for objects that can be improved (tiles, cities, etc.)
pub trait Improvable {
    /// Get current improvements
    fn get_improvements(&self) -> &TileImprovements;
    
    /// Get mutable improvements
    fn get_improvements_mut(&mut self) -> &mut TileImprovements;
    
    /// Check if can accept new improvement
    fn can_add_improvement(&self, improvement_type: ImprovementType) -> bool;
    
    /// Get terrain compatibility
    fn is_terrain_compatible(&self, improvement_type: ImprovementType) -> bool;
}

/// Trait for improvement yield calculation
pub trait YieldProvider {
    /// Calculate base yields
    fn base_yields(&self) -> ResourceYields;
    
    /// Calculate modified yields (including bonuses, penalties, etc.)
    fn modified_yields(&self) -> ResourceYields {
        self.base_yields()
    }
    
    /// Check if currently providing yields
    fn is_providing_yields(&self) -> bool;
}

impl YieldProvider for Improvement {
    fn base_yields(&self) -> ResourceYields {
        self.improvement_type.base_yields()
    }
    
    fn modified_yields(&self) -> ResourceYields {
        self.get_yields()
    }
    
    fn is_providing_yields(&self) -> bool {
        self.state.is_functional()
    }
}

/// Utility functions for improvement management
pub mod utils {
    use super::*;
    use super::types::ImprovementType;

    /// Get all resource-producing improvement types
    pub fn get_resource_improvements() -> Vec<ImprovementType> {
        vec![
            ImprovementType::Farm,
            ImprovementType::Mine,
            ImprovementType::Lumbermill,
            ImprovementType::Quarry,
            ImprovementType::Pasture,
        ]
    }

    /// Get all infrastructure improvement types
    pub fn get_infrastructure_improvements() -> Vec<ImprovementType> {
        vec![
            ImprovementType::Road,
            ImprovementType::Railroad,
            ImprovementType::Bridge,
            ImprovementType::Tunnel,
            ImprovementType::Fort,
        ]
    }

    /// Get all economic improvement types
    pub fn get_economic_improvements() -> Vec<ImprovementType> {
        vec![
            ImprovementType::TradingPost,
            ImprovementType::Market,
            ImprovementType::Bank,
            ImprovementType::Factory,
            ImprovementType::Port,
        ]
    }

    /// Get all cultural improvement types
    pub fn get_cultural_improvements() -> Vec<ImprovementType> {
        vec![
            ImprovementType::Temple,
            ImprovementType::University,
            ImprovementType::Library,
            ImprovementType::Monument,
            ImprovementType::Theater,
        ]
    }

    /// Get all military improvement types
    pub fn get_military_improvements() -> Vec<ImprovementType> {
        vec![
            ImprovementType::Barracks,
            ImprovementType::Arsenal,
            ImprovementType::Fortress,
            ImprovementType::Watchtower,
            ImprovementType::Bunker,
        ]
    }

    /// Get all specialized improvement types
    pub fn get_specialized_improvements() -> Vec<ImprovementType> {
        vec![
            ImprovementType::Observatory,
            ImprovementType::Lighthouse,
            ImprovementType::Aqueduct,
            ImprovementType::Windmill,
            ImprovementType::Irrigation,
        ]
    }

    /// Check if two improvement types are compatible on the same tile
    pub fn are_compatible(type1: ImprovementType, type2: ImprovementType) -> bool {
        use ImprovementType::*;
        
        // Same type is not compatible
        if type1 == type2 {
            return false;
        }

        match (type1, type2) {
            // Resource improvements are mutually exclusive
            (Farm | Mine | Lumbermill | Quarry | Pasture, 
             Farm | Mine | Lumbermill | Quarry | Pasture) => false,
            
            // Road/Railroad conflict
            (Road, Railroad) | (Railroad, Road) => false,
            
            // Military conflicts
            (Fort, Fortress) | (Fortress, Fort) => false,
            (Barracks, Arsenal) | (Arsenal, Barracks) => false,
            
            // Economic conflicts
            (Market, TradingPost) | (TradingPost, Market) => false,
            
            // Otherwise compatible
            _ => true,
        }
    }

    /// Get recommended improvements for a terrain type
    pub fn get_recommended_for_terrain(terrain: crate::world::tiles::components::TerrainType) -> Vec<ImprovementType> {
        use crate::world::tiles::components::TerrainType;
        use ImprovementType::*;

        match terrain {
            TerrainType::Grassland => vec![Farm, Pasture, Road],
            TerrainType::Plains => vec![Farm, Pasture, Road, Windmill],
            TerrainType::Hills => vec![Mine, Quarry, Watchtower, Observatory],
            TerrainType::Mountains => vec![Mine, Quarry, Tunnel, Observatory],
            TerrainType::Forest => vec![Lumbermill, Road],
            TerrainType::Jungle => vec![Lumbermill],
            TerrainType::Desert => vec![Irrigation, Road],
            TerrainType::Tundra => vec![Road],
            TerrainType::Coast => vec![Port, Lighthouse, Windmill],
            TerrainType::Ocean => vec![], // No improvements on ocean
            TerrainType::River => vec![Bridge, Aqueduct, Farm],
            TerrainType::Snow => vec![Observatory],
            TerrainType::Mountain => vec![Mine, Quarry, Tunnel, Observatory],
        }
    }

    /// Calculate total value of improvement (construction cost + maintenance over time)
    pub fn calculate_total_value(improvement_type: ImprovementType, turns_operational: u32) -> u32 {
        let construction_cost = improvement_type.construction_cost();
        let maintenance_cost = construction_cost / 10; // Rough estimate
        construction_cost + (maintenance_cost * turns_operational)
    }

    /// Estimate time to break even (when yields exceed costs)
    pub fn estimate_break_even_time(improvement_type: ImprovementType) -> Option<u32> {
        let construction_cost = improvement_type.construction_cost();
        let yields = improvement_type.base_yields();
        
        // Simple yield value calculation (each yield point = 1 value)
        let yield_per_turn = yields.food + yields.production + yields.commerce + yields.culture + yields.science;
        
        if yield_per_turn > 0 {
            Some(construction_cost / yield_per_turn)
        } else {
            None // Non-productive improvement
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_improvement_type_properties() {
        let farm = ImprovementType::Farm;
        assert_eq!(farm.name(), "Farm");
        assert_eq!(farm.category(), ImprovementCategory::Resource);
        assert!(farm.construction_cost() > 0);
        assert!(farm.construction_time() > 0);
    }

    #[test]
    fn test_improvement_state_transitions() {
        let mut state = ImprovementState::Planned;
        assert!(!state.is_functional());
        
        state = ImprovementState::UnderConstruction { turns_remaining: 3 };
        assert!(state.is_in_progress());
        assert!(!state.is_functional());
        
        state = ImprovementState::Completed;
        assert!(state.is_functional());
        assert!(state.can_be_worked());
        assert_eq!(state.effectiveness_factor(), 1.0);
    }

    #[test]
    fn test_resource_yields() {
        let yields1 = ResourceYields { food: 2, production: 1, commerce: 0, culture: 0, science: 0 };
        let yields2 = ResourceYields { food: 1, production: 0, commerce: 2, culture: 1, science: 0 };
        let total = yields1.add(yields2);
        
        assert_eq!(total.food, 3);
        assert_eq!(total.production, 1);
        assert_eq!(total.commerce, 2);
        assert_eq!(total.culture, 1);
        assert_eq!(total.science, 0);
    }

    #[test]
    fn test_improvement_compatibility() {
        assert!(!utils::are_compatible(ImprovementType::Farm, ImprovementType::Mine));
        assert!(!utils::are_compatible(ImprovementType::Road, ImprovementType::Railroad));
        assert!(utils::are_compatible(ImprovementType::Farm, ImprovementType::Road));
    }

    #[test]
    fn test_tile_improvements_capacity() {
        use crate::world::tiles::chunks::TileId;
        
        let mut tile_improvements = TileImprovements::new(TileId(1));
        assert!(tile_improvements.is_empty());
        assert!(!tile_improvements.is_full());
        assert_eq!(tile_improvements.available_slots(), MAX_IMPROVEMENTS_PER_TILE);
    }
}
