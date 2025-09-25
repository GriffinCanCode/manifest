//! Enhanced terrain type system with conversion support
//!
//! Provides enhanced terrain types that extend the base TerrainType enum
//! with additional terrain varieties for richer world generation.

use strum::{EnumIter, EnumString, Display, IntoStaticStr};
use serde::{Deserialize, Serialize};
use bevy_ecs::prelude::*;

use crate::world::tiles::components::TerrainType;

/// Enhanced terrain type enumeration with Lua integration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Component)]
#[derive(EnumIter, EnumString, Display, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum EnhancedTerrainType {
    Ocean,
    Grassland,
    Plains,
    Desert,
    Tundra,
    Snow,
    Forest,
    Jungle,
    Hills,
    Mountain,
    Swamp,
    Oasis,
    Volcano,
    Glacier,
    Beach,
}

impl Default for EnhancedTerrainType {
    fn default() -> Self {
        Self::Ocean
    }
}

impl From<TerrainType> for EnhancedTerrainType {
    fn from(terrain: TerrainType) -> Self {
        match terrain {
            TerrainType::Ocean => Self::Ocean,
            TerrainType::Grassland => Self::Grassland,
            TerrainType::Plains => Self::Plains,
            TerrainType::Desert => Self::Desert,
            TerrainType::Tundra => Self::Tundra,
            TerrainType::Snow => Self::Snow,
            TerrainType::Forest => Self::Forest,
            TerrainType::Jungle => Self::Jungle,
            TerrainType::Hills => Self::Hills,
            TerrainType::Mountain => Self::Mountain,
            TerrainType::Mountains => Self::Mountain, // Alias for Mountain
            TerrainType::River => Self::Ocean, // Rivers behave like water
            TerrainType::Coast => Self::Ocean, // Coast behaves like water
        }
    }
}

impl Into<TerrainType> for EnhancedTerrainType {
    fn into(self) -> TerrainType {
        match self {
            Self::Ocean => TerrainType::Ocean,
            Self::Grassland => TerrainType::Grassland,
            Self::Plains => TerrainType::Plains,
            Self::Desert => TerrainType::Desert,
            Self::Tundra => TerrainType::Tundra,
            Self::Snow => TerrainType::Snow,
            Self::Forest => TerrainType::Forest,
            Self::Jungle => TerrainType::Jungle,
            Self::Hills => TerrainType::Hills,
            Self::Mountain => TerrainType::Mountain,
            // Map additional types to closest existing ones
            Self::Swamp => TerrainType::Forest,
            Self::Oasis => TerrainType::Desert,
            Self::Volcano => TerrainType::Mountain,
            Self::Glacier => TerrainType::Snow,
            Self::Beach => TerrainType::Plains,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terrain_conversion() {
        // Test basic conversions
        let basic_terrain = TerrainType::Forest;
        let enhanced: EnhancedTerrainType = basic_terrain.into();
        assert_eq!(enhanced, EnhancedTerrainType::Forest);
        
        let back: TerrainType = enhanced.into();
        assert_eq!(back, TerrainType::Forest);
    }

    #[test]
    fn test_enhanced_terrain_mapping() {
        // Test that enhanced types map to appropriate base types
        let volcano: TerrainType = EnhancedTerrainType::Volcano.into();
        assert_eq!(volcano, TerrainType::Mountain);
        
        let oasis: TerrainType = EnhancedTerrainType::Oasis.into();
        assert_eq!(oasis, TerrainType::Desert);
        
        let swamp: TerrainType = EnhancedTerrainType::Swamp.into();
        assert_eq!(swamp, TerrainType::Forest);
    }

    #[test]
    fn test_terrain_string_conversion() {
        let terrain = EnhancedTerrainType::Volcano;
        let terrain_str: &'static str = terrain.into();
        assert_eq!(terrain_str, "volcano");
    }
}
