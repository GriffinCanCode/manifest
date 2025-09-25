//! Comprehensive Tile Properties System with Lua-scripted effects
//!
//! Implements terrain types, elevation, climate, biomes, resources, improvements,
//! movement costs, defense bonuses, fog of war, and cultural influence using
//! a combination of Rust performance and Lua flexibility.

use strum::{EnumIter, EnumString, Display, IntoStaticStr};
use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use ordered_float::OrderedFloat;
use fixedbitset::FixedBitSet;
use bitvec::prelude::*;
use dashmap::DashMap;
use std::{sync::Arc, collections::HashMap};
use tracing::{debug, info};

use crate::core::caching::{GameCache, GameCacheBuilder};
use crate::world::tiles::{
    chunks::TileId,
    components::{Tile, TerrainType, Climate},
    ownership::PlayerId
};
use crate::scripting::{ScriptManager, ScriptResult};

/// Enhanced terrain type enumeration with Lua integration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

/// Elevation data with noise generation support
#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub struct Elevation {
    /// Base elevation in meters
    pub base: f32,
    /// Noise-generated variation
    pub variation: f32,
    /// Final computed elevation
    pub final_elevation: f32,
    /// Slope gradient (for movement calculations)
    pub slope: f32,
}

impl Default for Elevation {
    fn default() -> Self {
        Self {
            base: 0.0,
            variation: 0.0,
            final_elevation: 0.0,
            slope: 0.0,
        }
    }
}

impl Elevation {
    /// Create elevation with noise variation
    pub fn with_noise(base: f32, noise_value: f32, amplitude: f32) -> Self {
        let variation = noise_value * amplitude;
        let final_elevation = base + variation;
        
        Self {
            base,
            variation,
            final_elevation,
            slope: 0.0, // Will be calculated separately
        }
    }

    /// Update slope based on neighboring elevations
    pub fn update_slope(&mut self, neighbor_elevations: &[f32]) {
        if neighbor_elevations.is_empty() {
            return;
        }

        let max_diff = neighbor_elevations.iter()
            .map(|&elev| (self.final_elevation - elev).abs())
            .fold(0.0f32, f32::max);
            
        self.slope = max_diff / 100.0; // Normalize slope
    }
}

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
        Self {
            temperature: climate.temperature,
            rainfall: climate.rainfall as u16,
            humidity: climate.humidity,
            wind_strength: climate.wind_strength,
            temperature_variation: 10, // Default variation
            interpolated: ClimateInterpolation {
                smooth_temperature: climate.temperature as f32,
                smooth_rainfall: climate.rainfall as f32,
                climate_zone: "temperate".to_string(),
            },
        }
    }
}

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
            modifiers: BiomeModifiers {
                movement_cost_multiplier: 1.0,
                defense_bonus: 0.0,
                agriculture_yield: 1.0,
                mining_yield: 1.0,
                population_capacity: 1.0,
            },
        }
    }
}

/// Resource configuration loaded from TOML files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConfig {
    pub resources: HashMap<String, ResourceDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceDefinition {
    pub name: String,
    pub rarity: f32,
    pub base_yield: u8,
    pub required_tech: Option<String>,
    pub terrain_preferences: Vec<String>,
    pub biome_modifiers: HashMap<String, f32>,
}

/// Improvement with Lua-scripted effects
#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub struct TileImprovement {
    pub improvement_type: String,
    pub level: u8,
    pub construction_progress: f32,
    pub effects: ImprovementEffects,
    pub lua_callback: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovementEffects {
    pub movement_cost_modifier: f32,
    pub defense_bonus: f32,
    pub resource_yield_modifiers: HashMap<String, f32>,
    pub population_capacity: i32,
}

impl Default for TileImprovement {
    fn default() -> Self {
        Self {
            improvement_type: "none".to_string(),
            level: 0,
            construction_progress: 0.0,
            effects: ImprovementEffects {
                movement_cost_modifier: 1.0,
                defense_bonus: 0.0,
                resource_yield_modifiers: HashMap::new(),
                population_capacity: 0,
            },
            lua_callback: None,
        }
    }
}

/// Movement costs with fixedbitset for efficient calculations
#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub struct MovementCosts {
    /// Base movement cost
    pub base_cost: OrderedFloat<f32>,
    /// Current modified cost
    pub current_cost: OrderedFloat<f32>,
    /// Bitset for movement restrictions by unit type
    pub restrictions: FixedBitSet,
    /// Road network connectivity
    pub road_connections: FixedBitSet,
    /// Weather-affected cost
    pub weather_modified_cost: OrderedFloat<f32>,
}

impl Default for MovementCosts {
    fn default() -> Self {
        Self {
            base_cost: OrderedFloat(1.0),
            current_cost: OrderedFloat(1.0),
            restrictions: FixedBitSet::with_capacity(32), // Up to 32 unit types
            road_connections: FixedBitSet::with_capacity(6), // 6 hex directions
            weather_modified_cost: OrderedFloat(1.0),
        }
    }
}

/// Defense bonuses with ordered float precision
#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub struct DefenseBonuses {
    /// Base terrain defense
    pub terrain_bonus: OrderedFloat<f32>,
    /// Improvement defense bonus
    pub improvement_bonus: OrderedFloat<f32>,
    /// Elevation advantage
    pub elevation_bonus: OrderedFloat<f32>,
    /// Final combined bonus
    pub total_bonus: OrderedFloat<f32>,
}

impl Default for DefenseBonuses {
    fn default() -> Self {
        Self {
            terrain_bonus: OrderedFloat(0.0),
            improvement_bonus: OrderedFloat(0.0),
            elevation_bonus: OrderedFloat(0.0),
            total_bonus: OrderedFloat(0.0),
        }
    }
}

impl DefenseBonuses {
    /// Calculate total defense bonus
    pub fn calculate_total(&mut self) {
        self.total_bonus = OrderedFloat(
            (self.terrain_bonus.into_inner() + 
             self.improvement_bonus.into_inner() + 
             self.elevation_bonus.into_inner()).min(0.9) // Cap at 90% bonus
        );
    }
}

/// Fog of war with bitvec visibility tracking
#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub struct FogOfWar {
    /// Players who have discovered this tile (bitfield)
    discovered: BitArray<[u64; 1]>,  // Support up to 64 players
    /// Players who currently have vision (bitfield)
    visible: BitArray<[u64; 1]>,
    /// Last turn seen by each player
    last_seen: [u16; 8], // Support up to 8 active players for last_seen tracking
    /// Vision level (0=unexplored, 1=discovered, 2=visible, 3=always_visible)
    vision_levels: [u8; 8],
}

impl Default for FogOfWar {
    fn default() -> Self {
        Self {
            discovered: BitArray::ZERO,
            visible: BitArray::ZERO,
            last_seen: [0; 8],
            vision_levels: [0; 8],
        }
    }
}

impl FogOfWar {
    /// Check if tile is discovered by player
    pub fn is_discovered_by(&self, player_id: PlayerId) -> bool {
        self.discovered[player_id as usize]
    }

    /// Check if tile is visible to player
    pub fn is_visible_to(&self, player_id: PlayerId) -> bool {
        self.visible[player_id as usize]
    }

    /// Mark tile as discovered by player
    pub fn discover_for_player(&mut self, player_id: PlayerId, turn: u16) {
        self.discovered.set(player_id as usize, true);
        if (player_id as usize) < self.last_seen.len() {
            self.last_seen[player_id as usize] = turn;
        }
    }

    /// Set visibility for player
    pub fn set_visible_to_player(&mut self, player_id: PlayerId, visible: bool) {
        self.visible.set(player_id as usize, visible);
    }
}

/// Cultural influence with dashmap for concurrent access
#[derive(Debug, Resource)]
pub struct CulturalInfluence {
    /// Cultural influence values by tile and player
    influence_map: DashMap<TileId, PlayerCulture>,
    /// Culture spread rate
    spread_rate: f32,
    /// Maximum influence distance
    max_distance: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerCulture {
    /// Influence strength by player (0.0 to 1.0)
    pub influences: HashMap<PlayerId, f32>,
    /// Dominant culture
    pub dominant_player: Option<PlayerId>,
    /// Cultural conversion pressure
    pub conversion_pressure: f32,
}

impl Default for CulturalInfluence {
    fn default() -> Self {
        Self {
            influence_map: DashMap::new(),
            spread_rate: 0.01,
            max_distance: 5,
        }
    }
}

impl CulturalInfluence {
    /// Get cultural influence for a tile
    pub fn get_influence(&self, tile_id: TileId) -> Option<PlayerCulture> {
        self.influence_map.get(&tile_id).map(|entry| entry.clone())
    }

    /// Set cultural influence for a tile
    pub fn set_influence(&self, tile_id: TileId, culture: PlayerCulture) {
        self.influence_map.insert(tile_id, culture);
    }

    /// Add influence for a specific player
    pub fn add_player_influence(&self, tile_id: TileId, player_id: PlayerId, amount: f32) {
        let mut entry = self.influence_map.entry(tile_id).or_insert_with(|| PlayerCulture {
            influences: HashMap::new(),
            dominant_player: None,
            conversion_pressure: 0.0,
        });
        
        let current = entry.influences.get(&player_id).unwrap_or(&0.0);
        let new_influence = (current + amount).min(1.0).max(0.0);
        entry.influences.insert(player_id, new_influence);
        
        // Update dominant player
        entry.dominant_player = entry.influences.iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(player, _)| *player);
    }
}

/// Main tile properties system manager
#[derive(Resource)]
pub struct TilePropertiesSystem {
    /// Lua script manager for scripted effects
    script_manager: Arc<ScriptManager>,
    /// Biome definitions loaded from RON files
    biome_definitions: HashMap<String, BiomeDefinition>,
    /// Resource configurations loaded from TOML files
    resource_config: ResourceConfig,
    /// Properties cache for performance
    properties_cache: GameCache,
    /// Cultural influence tracker
    cultural_influence: Arc<CulturalInfluence>,
}

// Manual Debug implementation since GameCache may not be Debug
impl std::fmt::Debug for TilePropertiesSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TilePropertiesSystem")
            .field("script_manager", &"Arc<ScriptManager>")
            .field("biome_definitions", &self.biome_definitions)
            .field("resource_config", &self.resource_config)
            .field("properties_cache", &"GameCache")
            .field("cultural_influence", &"Arc<CulturalInfluence>")
            .finish()
    }
}

impl Default for TilePropertiesSystem {
    fn default() -> Self {
        Self::new().expect("Failed to create TilePropertiesSystem")
    }
}

impl TilePropertiesSystem {
    /// Create new tile properties system
    pub fn new() -> ScriptResult<Self> {
        let script_manager = Arc::new(ScriptManager::new()?);
        let biome_definitions = Self::load_biome_definitions()?;
        let resource_config = Self::load_resource_config()?;
        let cultural_influence = Arc::new(CulturalInfluence::default());
        
        let properties_cache = GameCacheBuilder::new()
            .max_memory_mb(64) // 64MB cache for tile properties
            .default_ttl(std::time::Duration::from_secs(300)) // 5 minute TTL
            .build();

        let system = Self {
            script_manager,
            biome_definitions,
            resource_config,
            properties_cache,
            cultural_influence,
        };

        system.initialize_lua_scripts()?;
        
        info!("🌍 Tile Properties System initialized with {} biomes and {} resources", 
              system.biome_definitions.len(),
              system.resource_config.resources.len());
        
        Ok(system)
    }

    /// Load biome definitions from RON files
    fn load_biome_definitions() -> ScriptResult<HashMap<String, BiomeDefinition>> {
        use std::path::PathBuf;
        
        let config_path = PathBuf::from("backend/configs/biomes.ron");
        
        if !config_path.exists() {
            return Err(crate::scripting::ScriptError::FileNotFound {
                path: config_path.clone()
            });
        }
        
        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| crate::scripting::ScriptError::FileNotFound { 
                path: config_path.clone() 
            })?;
        
        let definitions: HashMap<String, BiomeDefinition> = ron::from_str(&content)
            .map_err(|e| crate::scripting::ScriptError::CompilationFailed {
                reason: format!("Failed to parse biome definitions from {}: {}", config_path.display(), e)
            })?;
        
        debug!("📚 Loaded {} biome definitions from {}", definitions.len(), config_path.display());
        Ok(definitions)
    }

    /// Load resource configuration from TOML files
    fn load_resource_config() -> ScriptResult<ResourceConfig> {
        use std::path::PathBuf;
        
        let config_path = PathBuf::from("backend/configs/resources.toml");
        
        if !config_path.exists() {
            return Err(crate::scripting::ScriptError::FileNotFound {
                path: config_path.clone()
            });
        }
        
        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| crate::scripting::ScriptError::FileNotFound { 
                path: config_path.clone() 
            })?;
        
        let config: ResourceConfig = toml::from_str(&content)
            .map_err(|e| crate::scripting::ScriptError::CompilationFailed {
                reason: format!("Failed to parse resource config from {}: {}", config_path.display(), e)
            })?;
        
        debug!("📦 Loaded {} resource definitions from {}", config.resources.len(), config_path.display());
        Ok(config)
    }

    /// Initialize Lua scripts for tile properties
    fn initialize_lua_scripts(&self) -> ScriptResult<()> {
        // Load tile properties script
        self.script_manager.load_script("tiles/properties.lua")?;
        
        // Load biome system script
        self.script_manager.load_script("tiles/biomes.lua")?;
        
        info!("📜 Tile properties Lua scripts initialized");
        Ok(())
    }

    /// Calculate movement cost for a tile using Lua scripts
    pub fn calculate_movement_cost(&self, tile_id: TileId, terrain: EnhancedTerrainType, 
                                   modifiers: HashMap<String, f32>) -> ScriptResult<f32> {
        // Convert terrain enum to string for Lua
        let terrain_str: &'static str = terrain.into();
        
        // Create argument tuple for Lua function: (tile_id, terrain_str, modifiers_map)
        let modifiers_vec: Vec<(String, f32)> = modifiers.into_iter().collect();
        let args = (tile_id, terrain_str, modifiers_vec);
        
        // Try to call Lua function first
        match self.script_manager.call_function::<(u64, &str, Vec<(String, f32)>), f32>("calculate_movement_cost", args) {
            Ok(cost) => {
                debug!("Lua movement cost calculation succeeded for tile {}: {}", tile_id, cost);
                Ok(cost)
            },
            Err(lua_error) => {
                // Fallback to hardcoded values if Lua script fails
                debug!("Lua movement cost calculation failed for tile {} ({}), using fallback", tile_id, lua_error);
                
                let base_cost = match terrain {
                    EnhancedTerrainType::Ocean => 3.0,
                    EnhancedTerrainType::Grassland => 1.0,
                    EnhancedTerrainType::Plains => 1.0,
                    EnhancedTerrainType::Desert => 2.0,
                    EnhancedTerrainType::Tundra => 2.0,
                    EnhancedTerrainType::Snow => 2.5,
                    EnhancedTerrainType::Forest => 2.0,
                    EnhancedTerrainType::Jungle => 3.0,
                    EnhancedTerrainType::Hills => 2.0,
                    EnhancedTerrainType::Mountain => 4.0,
                    EnhancedTerrainType::Swamp => 3.0,
                    EnhancedTerrainType::Oasis => 1.0,
                    EnhancedTerrainType::Volcano => 4.0,
                    EnhancedTerrainType::Glacier => 3.0,
                    EnhancedTerrainType::Beach => 1.5,
                };
                
                // Apply modifier effects (reconstruct modifiers from Vec)
                let modifier_multiplier = modifiers_vec.iter().fold(1.0, |acc, (_, &modifier)| {
                    acc * (1.0 + modifier)
                });
                
                Ok(base_cost * modifier_multiplier)
            }
        }
    }

    /// Determine biome for a tile using Lua scripts
    pub fn determine_biome(&self, climate: &EnhancedClimate, terrain: EnhancedTerrainType, 
                          elevation: f32) -> ScriptResult<Option<String>> {
        // Convert terrain enum to string for Lua
        let terrain_str: &'static str = terrain.into();
        
        // Create argument tuple for Lua function: (climate_data, terrain_str, elevation)
        let climate_data = (
            climate.temperature,
            climate.rainfall,
            climate.humidity,
            climate.wind_strength,
        );
        let args = (climate_data, terrain_str, elevation);
        
        // Try to call Lua function first
        match self.script_manager.call_function::<((i8, u16, u8, u8), &str, f32), String>("determine_biome", args) {
            Ok(biome) => {
                debug!("Lua biome determination succeeded: {} for terrain {} at elevation {}", biome, terrain_str, elevation);
                Ok(Some(biome))
            },
            Err(lua_error) => {
                // Fallback to hardcoded biome determination if Lua script fails
                debug!("Lua biome determination failed ({}), using fallback", lua_error);
                
                let biome = match (terrain, climate.temperature, elevation) {
                    (EnhancedTerrainType::Ocean, _, _) => "ocean",
                    (EnhancedTerrainType::Grassland, temp, _) if temp > 20 => "tropical_grassland",
                    (EnhancedTerrainType::Grassland, _, _) => "temperate_grassland",
                    (EnhancedTerrainType::Plains, temp, _) if temp > 20 => "savanna", 
                    (EnhancedTerrainType::Plains, _, _) => "temperate_plains",
                    (EnhancedTerrainType::Desert, _, _) => "arid_desert",
                    (EnhancedTerrainType::Tundra, _, _) => "arctic_tundra",
                    (EnhancedTerrainType::Snow, _, _) => "polar",
                    (EnhancedTerrainType::Forest, temp, _) if temp > 20 => "tropical_forest",
                    (EnhancedTerrainType::Forest, _, _) => "temperate_forest",
                    (EnhancedTerrainType::Jungle, _, _) => "tropical_rainforest",
                    (EnhancedTerrainType::Hills, _, elev) if elev > 1000.0 => "highland",
                    (EnhancedTerrainType::Hills, _, _) => "temperate_hills",
                    (EnhancedTerrainType::Mountain, _, _) => "alpine",
                    (EnhancedTerrainType::Swamp, _, _) => "wetland",
                    (EnhancedTerrainType::Oasis, _, _) => "desert_oasis",
                    (EnhancedTerrainType::Volcano, _, _) => "volcanic",
                    (EnhancedTerrainType::Glacier, _, _) => "glacial",
                    (EnhancedTerrainType::Beach, _, _) => "coastal",
                };
                
                Ok(Some(biome.to_string()))
            }
        }
    }

    /// Get cultural influence system
    pub fn cultural_influence(&self) -> Arc<CulturalInfluence> {
        self.cultural_influence.clone()
    }

    /// Access to script manager
    pub fn script_manager(&self) -> Arc<ScriptManager> {
        self.script_manager.clone()
    }
}

/// System for updating tile properties  
pub fn update_tile_properties(
    mut tiles_query: Query<(Entity, &TileId, &mut MovementCosts, &mut DefenseBonuses), Changed<Tile>>,
    properties_system: Res<TilePropertiesSystem>,
) {
    for (entity, tile_id, mut movement_costs, mut defense_bonuses) in tiles_query.iter_mut() {
        // Update calculated properties when tiles change
        movement_costs.current_cost = movement_costs.base_cost;
        defense_bonuses.calculate_total();
        
        // Could trigger Lua callbacks here for advanced effects
        debug!("Updated properties for tile {:?}", tile_id);
    }
}

/// System for cultural influence spread
pub fn update_cultural_influence(
    tiles_query: Query<&TileId>,
    properties_system: Res<TilePropertiesSystem>,
) {
    let influence = properties_system.cultural_influence();
    
    // Implement cultural influence spread logic here
    // This would calculate influence propagation between adjacent tiles
    
    debug!("Updated cultural influence for {} tiles", tiles_query.iter().len());
}
