//! Core types and enums for tile improvements
//!
//! Contains type definitions, keys, enums, and basic type implementations.

use slotmap::Key;
use serde::{Deserialize, Serialize};

/// Maximum number of improvements per tile
pub const MAX_IMPROVEMENTS_PER_TILE: usize = 8;

/// Unique identifier for improvements using slotmap  
slotmap::new_key_type! {
    /// Stable handle to an improvement that remains valid across saves/loads
    pub struct ImprovementKey;
}

// Add Serialize/Deserialize implementations for slotmap key
impl serde::Serialize for ImprovementKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.data().as_ffi().serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for ImprovementKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let data = u64::deserialize(deserializer)?;
        Ok(Self::from(slotmap::KeyData::from_ffi(data)))
    }
}

/// Types of improvements that can be built on tiles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u16)]
pub enum ImprovementType {
    // Basic improvements
    Farm = 0,
    Mine = 1,
    Lumbermill = 2,
    Quarry = 3,
    Pasture = 4,
    
    // Infrastructure
    Road = 10,
    Railroad = 11,
    Bridge = 12,
    Tunnel = 13,
    Fort = 14,
    
    // Economic
    TradingPost = 20,
    Market = 21,
    Bank = 22,
    Factory = 23,
    Port = 24,
    
    // Cultural/Religious
    Temple = 30,
    University = 31,
    Library = 32,
    Monument = 33,
    Theater = 34,
    
    // Military
    Barracks = 40,
    Arsenal = 41,
    Fortress = 42,
    Watchtower = 43,
    Bunker = 44,
    
    // Specialized
    Observatory = 50,
    Lighthouse = 51,
    Aqueduct = 52,
    Windmill = 53,
    Irrigation = 54,
}

impl ImprovementType {
    /// Get the name of the improvement
    pub fn name(self) -> &'static str {
        match self {
            Self::Farm => "Farm",
            Self::Mine => "Mine", 
            Self::Lumbermill => "Lumbermill",
            Self::Quarry => "Quarry",
            Self::Pasture => "Pasture",
            Self::Road => "Road",
            Self::Railroad => "Railroad",
            Self::Bridge => "Bridge",
            Self::Tunnel => "Tunnel",
            Self::Fort => "Fort",
            Self::TradingPost => "Trading Post",
            Self::Market => "Market",
            Self::Bank => "Bank",
            Self::Factory => "Factory",
            Self::Port => "Port",
            Self::Temple => "Temple",
            Self::University => "University",
            Self::Library => "Library",
            Self::Monument => "Monument",
            Self::Theater => "Theater",
            Self::Barracks => "Barracks",
            Self::Arsenal => "Arsenal",
            Self::Fortress => "Fortress",
            Self::Watchtower => "Watchtower",
            Self::Bunker => "Bunker",
            Self::Observatory => "Observatory",
            Self::Lighthouse => "Lighthouse",
            Self::Aqueduct => "Aqueduct",
            Self::Windmill => "Windmill",
            Self::Irrigation => "Irrigation",
        }
    }

    /// Get the description of the improvement
    pub fn description(self) -> &'static str {
        match self {
            Self::Farm => "Increases food production from fertile land",
            Self::Mine => "Extracts valuable minerals and metals",
            Self::Lumbermill => "Processes wood from forest resources",
            Self::Quarry => "Extracts stone and building materials",
            Self::Pasture => "Provides livestock and animal products",
            Self::Road => "Improves movement speed and trade",
            Self::Railroad => "Enables fast transportation of goods and people",
            Self::Bridge => "Allows crossing rivers and water bodies",
            Self::Tunnel => "Enables passage through mountains",
            Self::Fort => "Provides defensive bonuses to military units",
            Self::TradingPost => "Facilitates commerce and trade",
            Self::Market => "Increases economic activity and wealth",
            Self::Bank => "Generates additional gold from commerce",
            Self::Factory => "Mass produces goods and materials",
            Self::Port => "Enables maritime trade and naval operations",
            Self::Temple => "Provides spiritual guidance and cultural influence",
            Self::University => "Advances research and education",
            Self::Library => "Preserves knowledge and cultural heritage",
            Self::Monument => "Inspires citizens and showcases civilization",
            Self::Theater => "Entertains population and spreads culture",
            Self::Barracks => "Trains and houses military units",
            Self::Arsenal => "Produces and stores military equipment",
            Self::Fortress => "Massive defensive structure",
            Self::Watchtower => "Provides early warning of approaching enemies",
            Self::Bunker => "Heavily fortified defensive position",
            Self::Observatory => "Studies celestial bodies and navigation",
            Self::Lighthouse => "Guides ships safely to port",
            Self::Aqueduct => "Provides fresh water to settlements",
            Self::Windmill => "Harnesses wind power for various purposes",
            Self::Irrigation => "Channels water to improve agriculture",
        }
    }

    /// Get the category this improvement belongs to
    pub fn category(self) -> ImprovementCategory {
        match self {
            Self::Farm | Self::Mine | Self::Lumbermill | Self::Quarry | Self::Pasture => {
                ImprovementCategory::Resource
            }
            Self::Road | Self::Railroad | Self::Bridge | Self::Tunnel | Self::Fort => {
                ImprovementCategory::Infrastructure
            }
            Self::TradingPost | Self::Market | Self::Bank | Self::Factory | Self::Port => {
                ImprovementCategory::Economic
            }
            Self::Temple | Self::University | Self::Library | Self::Monument | Self::Theater => {
                ImprovementCategory::Cultural
            }
            Self::Barracks | Self::Arsenal | Self::Fortress | Self::Watchtower | Self::Bunker => {
                ImprovementCategory::Military
            }
            Self::Observatory | Self::Lighthouse | Self::Aqueduct | Self::Windmill | Self::Irrigation => {
                ImprovementCategory::Specialized
            }
        }
    }

    /// Get the base construction cost
    pub fn construction_cost(self) -> u32 {
        match self {
            Self::Farm => 60,
            Self::Mine => 80,
            Self::Lumbermill => 70,
            Self::Quarry => 80,
            Self::Pasture => 50,
            Self::Road => 30,
            Self::Railroad => 120,
            Self::Bridge => 100,
            Self::Tunnel => 150,
            Self::Fort => 200,
            Self::TradingPost => 90,
            Self::Market => 120,
            Self::Bank => 180,
            Self::Factory => 200,
            Self::Port => 150,
            Self::Temple => 100,
            Self::University => 250,
            Self::Library => 150,
            Self::Monument => 300,
            Self::Theater => 180,
            Self::Barracks => 120,
            Self::Arsenal => 200,
            Self::Fortress => 400,
            Self::Watchtower => 80,
            Self::Bunker => 250,
            Self::Observatory => 300,
            Self::Lighthouse => 120,
            Self::Aqueduct => 200,
            Self::Windmill => 90,
            Self::Irrigation => 80,
        }
    }

    /// Get the base construction time in turns
    pub fn construction_time(self) -> u32 {
        match self {
            Self::Farm => 3,
            Self::Mine => 4,
            Self::Lumbermill => 3,
            Self::Quarry => 4,
            Self::Pasture => 2,
            Self::Road => 2,
            Self::Railroad => 6,
            Self::Bridge => 5,
            Self::Tunnel => 8,
            Self::Fort => 8,
            Self::TradingPost => 4,
            Self::Market => 5,
            Self::Bank => 7,
            Self::Factory => 8,
            Self::Port => 6,
            Self::Temple => 5,
            Self::University => 10,
            Self::Library => 6,
            Self::Monument => 12,
            Self::Theater => 7,
            Self::Barracks => 5,
            Self::Arsenal => 8,
            Self::Fortress => 15,
            Self::Watchtower => 3,
            Self::Bunker => 10,
            Self::Observatory => 12,
            Self::Lighthouse => 5,
            Self::Aqueduct => 8,
            Self::Windmill => 4,
            Self::Irrigation => 3,
        }
    }

    /// Check if this improvement can be built on the given terrain type
    pub fn can_build_on_terrain(self, terrain: crate::world::tiles::components::TerrainType) -> bool {
        use crate::world::tiles::components::TerrainType;
        
        match self {
            Self::Farm => matches!(terrain, TerrainType::Grassland | TerrainType::Plains | TerrainType::River),
            Self::Mine => matches!(terrain, TerrainType::Hills | TerrainType::Mountains),
            Self::Lumbermill => matches!(terrain, TerrainType::Forest | TerrainType::Jungle),
            Self::Quarry => matches!(terrain, TerrainType::Hills | TerrainType::Mountains),
            Self::Pasture => matches!(terrain, TerrainType::Grassland | TerrainType::Plains),
            Self::Road => true, // Roads can be built anywhere
            Self::Railroad => true, // Railroads can be built anywhere
            Self::Bridge => matches!(terrain, TerrainType::River),
            Self::Tunnel => matches!(terrain, TerrainType::Mountains),
            Self::Fort => !matches!(terrain, TerrainType::Ocean | TerrainType::River),
            Self::TradingPost => !matches!(terrain, TerrainType::Ocean),
            Self::Market => !matches!(terrain, TerrainType::Ocean | TerrainType::Mountains),
            Self::Bank => !matches!(terrain, TerrainType::Ocean | TerrainType::Mountains | TerrainType::Desert),
            Self::Factory => !matches!(terrain, TerrainType::Ocean | TerrainType::Mountains),
            Self::Port => matches!(terrain, TerrainType::Coast),
            Self::Temple => !matches!(terrain, TerrainType::Ocean),
            Self::University => !matches!(terrain, TerrainType::Ocean | TerrainType::Desert | TerrainType::Tundra),
            Self::Library => !matches!(terrain, TerrainType::Ocean | TerrainType::Desert),
            Self::Monument => !matches!(terrain, TerrainType::Ocean),
            Self::Theater => !matches!(terrain, TerrainType::Ocean | TerrainType::Desert | TerrainType::Tundra),
            Self::Barracks => !matches!(terrain, TerrainType::Ocean),
            Self::Arsenal => !matches!(terrain, TerrainType::Ocean | TerrainType::Desert),
            Self::Fortress => !matches!(terrain, TerrainType::Ocean | TerrainType::River),
            Self::Watchtower => matches!(terrain, TerrainType::Hills | TerrainType::Mountains),
            Self::Bunker => !matches!(terrain, TerrainType::Ocean | TerrainType::River),
            Self::Observatory => matches!(terrain, TerrainType::Hills | TerrainType::Mountains),
            Self::Lighthouse => matches!(terrain, TerrainType::Coast),
            Self::Aqueduct => !matches!(terrain, TerrainType::Ocean | TerrainType::Desert),
            Self::Windmill => matches!(terrain, TerrainType::Hills | TerrainType::Plains | TerrainType::Coast),
            Self::Irrigation => matches!(terrain, TerrainType::Desert | TerrainType::Plains),
        }
    }

    /// Get all improvement types as an iterator
    pub fn all() -> impl Iterator<Item = Self> {
        [
            Self::Farm, Self::Mine, Self::Lumbermill, Self::Quarry, Self::Pasture,
            Self::Road, Self::Railroad, Self::Bridge, Self::Tunnel, Self::Fort,
            Self::TradingPost, Self::Market, Self::Bank, Self::Factory, Self::Port,
            Self::Temple, Self::University, Self::Library, Self::Monument, Self::Theater,
            Self::Barracks, Self::Arsenal, Self::Fortress, Self::Watchtower, Self::Bunker,
            Self::Observatory, Self::Lighthouse, Self::Aqueduct, Self::Windmill, Self::Irrigation,
        ].into_iter()
    }
}

/// Categories for organizing improvements
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ImprovementCategory {
    Resource,
    Infrastructure,
    Economic,
    Cultural,
    Military,
    Specialized,
}

/// Current state of an improvement
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImprovementState {
    /// Improvement is planned but construction hasn't started
    Planned,
    /// Currently under construction (turns_remaining)
    UnderConstruction { turns_remaining: u32 },
    /// Fully constructed and operational
    Completed,
    /// Temporarily damaged but still functional
    Damaged { severity: u8 }, // 0-100
    /// Completely destroyed and non-functional
    Destroyed,
    /// Under repair (turns_remaining)
    UnderRepair { turns_remaining: u32 },
}

impl Default for ImprovementState {
    fn default() -> Self {
        Self::Planned
    }
}

impl ImprovementState {
    /// Check if the improvement is functional (can provide benefits)
    pub fn is_functional(self) -> bool {
        matches!(self, Self::Completed | Self::Damaged { .. })
    }

    /// Check if the improvement is under construction or repair
    pub fn is_in_progress(self) -> bool {
        matches!(self, Self::UnderConstruction { .. } | Self::UnderRepair { .. })
    }

    /// Check if the improvement can be worked by citizens
    pub fn can_be_worked(self) -> bool {
        matches!(self, Self::Completed)
    }

    /// Get effectiveness factor (0.0 to 1.0) based on state
    pub fn effectiveness_factor(self) -> f32 {
        match self {
            Self::Completed => 1.0,
            Self::Damaged { severity } => 1.0 - (severity as f32 / 100.0) * 0.5,
            _ => 0.0,
        }
    }
}
