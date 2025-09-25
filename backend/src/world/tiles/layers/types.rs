//! Layer and feature type definitions for the multi-layer system
//!
//! Defines the core types used throughout the layer system including
//! layer categories and feature classifications.

use serde::{Deserialize, Serialize};

/// Layer types for organizing different aspects of tile data
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum LayerType {
    /// Base terrain and elevation
    Terrain = 0,
    /// Natural resources and deposits
    Resources = 1,
    /// Political boundaries and ownership
    Political = 2,
    /// Cultural influences and zones
    Cultural = 3,
    /// Religious presence and holy sites
    Religious = 4,
    /// Military presence and fortifications
    Military = 5,
    /// Economic networks and trade
    Economic = 6,
    /// Environmental effects and climate
    Environmental = 7,
}

impl LayerType {
    /// Get all layer types
    pub const fn all() -> &'static [LayerType] {
        &[
            LayerType::Terrain, LayerType::Resources, LayerType::Political,
            LayerType::Cultural, LayerType::Religious, LayerType::Military,
            LayerType::Economic, LayerType::Environmental,
        ]
    }

    /// Get layer priority (lower values render first)
    pub fn render_priority(self) -> u8 {
        self as u8
    }

    /// Check if layer affects gameplay mechanics
    pub fn is_gameplay_layer(self) -> bool {
        match self {
            LayerType::Terrain | LayerType::Resources | LayerType::Political | LayerType::Military => true,
            _ => false,
        }
    }

    /// Check if layer is purely visual/informational
    pub fn is_visual_layer(self) -> bool {
        !self.is_gameplay_layer()
    }
}

/// Types of features that can exist in layers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u16)]
pub enum FeatureType {
    // Terrain features
    River = 0,
    Forest = 1,
    Mountain = 2,
    Hill = 3,
    Desert = 4,
    Oasis = 5,
    Volcano = 6,
    Canyon = 7,
    
    // Resource features
    IronDeposit = 100,
    GoldVein = 101,
    OilField = 102,
    CoalMine = 103,
    Quarry = 104,
    FertileSoil = 105,
    FishingGrounds = 106,
    HuntingGrounds = 107,
    
    // Political features
    NationalBorder = 200,
    ProvinceBorder = 201,
    CityLimits = 202,
    MilitaryZone = 203,
    DemilitarizedZone = 204,
    TradeZone = 205,
    NaturalPark = 206,
    
    // Cultural features
    CulturalSite = 300,
    HistoricalSite = 301,
    ArtisticCenter = 302,
    EducationalHub = 303,
    CulturalBoundary = 304,
    LanguageZone = 305,
    TraditionArea = 306,
    
    // Religious features
    HolySite = 400,
    Temple = 401,
    Shrine = 402,
    Pilgrimage = 403,
    ReligiousBoundary = 404,
    Monastery = 405,
    Cemetery = 406,
    
    // Military features
    Fortress = 500,
    Barracks = 501,
    Watchtower = 502,
    Battlefield = 503,
    StrategicPoint = 504,
    SupplyDepot = 505,
    DefensiveLine = 506,
    
    // Economic features
    TradeRoute = 600,
    Market = 601,
    TradingPost = 602,
    Caravan = 603,
    Port = 604,
    Workshop = 605,
    Guild = 606,
    
    // Environmental features
    Pollution = 700,
    Radiation = 701,
    Disease = 702,
    ClimateZone = 703,
    Weather = 704,
    Disaster = 705,
    Restoration = 706,
}

impl FeatureType {
    /// Get the layer type this feature belongs to
    pub fn layer_type(self) -> LayerType {
        match (self as u16) / 100 {
            0 => LayerType::Terrain,
            1 => LayerType::Resources,
            2 => LayerType::Political,
            3 => LayerType::Cultural,
            4 => LayerType::Religious,
            5 => LayerType::Military,
            6 => LayerType::Economic,
            7 => LayerType::Environmental,
            _ => LayerType::Environmental, // Default fallback
        }
    }

    /// Check if feature affects tile properties
    pub fn affects_tile_properties(self) -> bool {
        match self.layer_type() {
            LayerType::Terrain | LayerType::Resources | LayerType::Environmental => true,
            _ => false,
        }
    }

    /// Get base influence radius for this feature type
    pub fn influence_radius(self) -> u8 {
        match self {
            // Local features
            FeatureType::River | FeatureType::Forest | FeatureType::IronDeposit => 1,
            
            // Regional features
            FeatureType::Mountain | FeatureType::Volcano | FeatureType::CityLimits => 2,
            
            // Large-scale features
            FeatureType::Desert | FeatureType::NationalBorder | FeatureType::TradeRoute => 3,
            
            // Minimal influence
            _ => 0,
        }
    }
}
