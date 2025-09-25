//! Resource type definitions and properties
//!
//! Enhanced resource system with comprehensive properties for
//! geological simulation, market dynamics, and gameplay balance.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use bevy_ecs::prelude::Component;

/// Enhanced resource type with comprehensive properties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceType {
    pub id: String,
    pub name: String,
    pub category: ResourceCategory,
    pub properties: ResourceProperties,
    pub distribution: DistributionRules,
    pub economics: EconomicProperties,
}

/// Resource categories for organization and behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceCategory {
    /// Strategic resources (oil, uranium, rare earths)
    Strategic,
    /// Industrial resources (iron, coal, copper)
    Industrial, 
    /// Precious resources (gold, silver, diamonds)
    Precious,
    /// Agricultural resources (wheat, cattle, fish)
    Agricultural,
    /// Luxury resources (spices, silk, furs)
    Luxury,
    /// Energy resources (coal, oil, uranium, geothermal)
    Energy,
    /// Building materials (stone, lumber, sand)
    Construction,
}

/// Comprehensive resource properties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceProperties {
    /// Base rarity (0.0 = very common, 1.0 = extremely rare)
    pub rarity: f32,
    /// Base value per unit
    pub base_value: f32,
    /// Resource quality range (min, max)
    pub quality_range: (f32, f32),
    /// Whether resource is renewable
    pub renewable: bool,
    /// Regeneration rate if renewable (units per turn)
    pub regen_rate: f32,
    /// Base depletion rate for non-renewable resources
    pub depletion_rate: f32,
    /// Discovery difficulty (higher = harder to find)
    pub discovery_difficulty: f32,
    /// Technology required to extract (empty = stone age)
    pub required_tech: Vec<String>,
}

/// Distribution rules for resource placement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionRules {
    /// Preferred terrain types for this resource
    pub terrain_affinity: HashMap<String, f32>,
    /// Geological requirements (elevation, plate age, etc.)
    pub geological_rules: GeologicalRules,
    /// Climate preferences
    pub climate_rules: ClimateRules,
    /// Clustering behavior
    pub clustering: ClusteringRules,
}

/// Geological requirements for resource formation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeologicalRules {
    /// Preferred elevation range (meters)
    pub elevation_range: Option<(f32, f32)>,
    /// Preferred plate age (millions of years)
    pub plate_age_range: Option<(f32, f32)>,
    /// Required tectonic features (mountains, rifts, etc.)
    pub tectonic_features: Vec<String>,
    /// Preferred distance from tectonic boundaries
    pub boundary_distance: Option<(f32, f32)>,
    /// Volcanic activity requirements
    pub volcanic_requirements: Option<VolcanicRequirements>,
}

/// Volcanic activity requirements for certain resources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolcanicRequirements {
    /// Requires active volcanism
    pub active_volcanism: bool,
    /// Requires ancient volcanic activity
    pub ancient_volcanism: bool,
    /// Preferred distance from volcanoes
    pub distance_range: (f32, f32),
}

/// Climate requirements for resource formation/growth
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClimateRules {
    /// Temperature range preferences (Celsius)
    pub temperature_range: Option<(f32, f32)>,
    /// Rainfall range preferences (mm/year)
    pub rainfall_range: Option<(f32, f32)>,
    /// Humidity preferences (0-100%)
    pub humidity_range: Option<(f32, f32)>,
    /// Seasonal variation tolerance
    pub seasonal_tolerance: f32,
}

/// Resource clustering behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusteringRules {
    /// Tendency to form clusters (0.0 = scattered, 1.0 = highly clustered)
    pub cluster_tendency: f32,
    /// Average cluster size
    pub cluster_size: u32,
    /// Maximum distance between cluster members (hex tiles)
    pub cluster_radius: u32,
    /// Probability of secondary clusters forming nearby
    pub secondary_cluster_chance: f32,
}

/// Economic properties for market simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicProperties {
    /// Base market demand
    pub base_demand: f32,
    /// Price volatility factor
    pub volatility: f32,
    /// Strategic importance (affects AI behavior)
    pub strategic_value: f32,
    /// Trade route value multiplier
    pub trade_value: f32,
    /// Stockpiling preference by AI
    pub stockpile_priority: f32,
}

/// Resource deposit component for world tiles
#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub struct ResourceDeposit {
    /// Resource type identifier
    pub resource_type: String,
    /// Current quantity (0-255 for memory efficiency)
    pub quantity: u8,
    /// Resource quality (0.0-1.0)
    pub quality: f32,
    /// Whether discovered by any player
    pub discovered: bool,
    /// Discovery difficulty for this specific deposit
    pub discovery_difficulty: f32,
    /// Current depletion state
    pub depletion_state: DepletionState,
    /// Extraction modifiers
    pub extraction_modifiers: ExtractionModifiers,
}

/// Resource depletion tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepletionState {
    /// Original quantity when first discovered
    pub original_quantity: u8,
    /// Current extraction rate per turn
    pub current_extraction: f32,
    /// Efficiency loss due to depletion
    pub efficiency_penalty: f32,
    /// Estimated turns until exhaustion
    pub turns_remaining: u32,
}

/// Extraction cost and efficiency modifiers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionModifiers {
    /// Technology level bonus/penalty
    pub tech_modifier: f32,
    /// Infrastructure bonus (roads, ports, etc.)
    pub infrastructure_modifier: f32,
    /// Environmental penalties (difficult terrain, etc.)
    pub environmental_penalty: f32,
    /// Labor availability modifier
    pub labor_modifier: f32,
}

impl Default for ResourceDeposit {
    fn default() -> Self {
        Self {
            resource_type: String::new(),
            quantity: 0,
            quality: 0.5,
            discovered: false,
            discovery_difficulty: 0.5,
            depletion_state: DepletionState::default(),
            extraction_modifiers: ExtractionModifiers::default(),
        }
    }
}

impl Default for DepletionState {
    fn default() -> Self {
        Self {
            original_quantity: 0,
            current_extraction: 0.0,
            efficiency_penalty: 0.0,
            turns_remaining: 0,
        }
    }
}

impl Default for ExtractionModifiers {
    fn default() -> Self {
        Self {
            tech_modifier: 1.0,
            infrastructure_modifier: 1.0,
            environmental_penalty: 1.0,
            labor_modifier: 1.0,
        }
    }
}

/// Resource vein component for connected deposits
#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub struct ResourceVein {
    /// Unique vein identifier
    pub vein_id: u64,
    /// Vein type (determines shape and extent)
    pub vein_type: VeinType,
    /// Total estimated reserves in this vein
    pub total_reserves: u32,
    /// Connected tile positions
    pub connected_tiles: Vec<(i32, i32)>,
}

impl<'lua> mlua::FromLua<'lua> for ResourceVein {
    fn from_lua(lua_value: mlua::Value<'lua>, _lua: &'lua mlua::Lua) -> mlua::Result<Self> {
        match lua_value {
            mlua::Value::Table(table) => {
                let vein_id: u64 = table.get("vein_id").unwrap_or(0);
                let vein_type_str: String = table.get("vein_type").unwrap_or_else(|_| "Linear".to_string());
                let vein_type = match vein_type_str.as_str() {
                    "Linear" => VeinType::Linear,
                    "Circular" => VeinType::Circular,
                    "Branching" => VeinType::Branching,
                    "Scattered" => VeinType::Scattered,
                    "Massive" => VeinType::Massive,
                    _ => VeinType::Linear,
                };
                let total_reserves: u32 = table.get("total_reserves").unwrap_or(0);
                // Manually parse connected_tiles from Lua array
                let connected_tiles = if let Ok(mlua::Value::Table(tiles_table)) = table.get::<_, mlua::Value>("connected_tiles") {
                    let mut tiles = Vec::new();
                    for i in 1..=(tiles_table.len().unwrap_or(0)) {
                        if let Ok(mlua::Value::Table(tile)) = tiles_table.get::<_, mlua::Value>(i) {
                            let x: i32 = tile.get(1).unwrap_or(0);
                            let y: i32 = tile.get(2).unwrap_or(0);
                            tiles.push((x, y));
                        }
                    }
                    tiles
                } else {
                    Vec::new()
                };
                
                Ok(ResourceVein {
                    vein_id,
                    vein_type,
                    total_reserves,
                    connected_tiles,
                })
            }
            _ => Err(mlua::Error::FromLuaConversionError {
                from: lua_value.type_name(),
                to: "ResourceVein",
                message: Some("expected table with vein fields".to_string()),
            }),
        }
    }
}

/// Types of resource veins with different characteristics
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum VeinType {
    /// Linear vein following geological structures
    Linear,
    /// Circular/oval deposit
    Circular,
    /// Branching vein network
    Branching,
    /// Scattered pockets
    Scattered,
    /// Massive continuous deposit
    Massive,
}
