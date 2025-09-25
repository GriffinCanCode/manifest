//! Game-specific caching strategies for grand strategy games
//!
//! Specialized cache implementations for:
//! - Pathfinding results with movement type awareness
//! - AI decision trees and evaluation caching
//! - Rendering asset and UI state caching
//! - Player data and technology trees

use std::collections::HashMap;
use glam::IVec2;
use serde::{Serialize, Deserialize};
use bevy_ecs::prelude::Entity;

use crate::core::hashing::{FastHashMap, FastHasher};
use super::{CacheKey, CachePriority};

// =================================================================================================
// PATHFINDING CACHE
// =================================================================================================

/// Pathfinding cache key
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathfindingCacheKey {
    pub start: IVec2,
    pub end: IVec2,
    pub movement_type: MovementType,
    pub player_id: u32, // For visibility/territory restrictions
    pub world_generation: u32,
    pub path_constraints: PathConstraints,
}

/// Movement types that affect pathfinding
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum MovementType {
    /// Land-based movement
    Land { movement_points: u32 },
    /// Naval movement
    Naval { movement_points: u32 },
    /// Air movement (ignores most terrain)
    Air { range: u32 },
    /// Special units (can cross certain terrain)
    Special { movement_points: u32, abilities: Vec<MovementAbility> },
}

/// Special movement abilities
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum MovementAbility {
    IgnoreTerrain,
    SwampMovement,
    MountainClimbing,
    RiverCrossing,
    EnemyTerritoryAccess,
}

/// Pathfinding constraints
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathConstraints {
    /// Avoid enemy territory
    pub avoid_enemies: bool,
    /// Maximum path length
    pub max_distance: Option<u32>,
    /// Required terrain types
    pub required_terrain: Option<Vec<String>>,
    /// Forbidden terrain types  
    pub forbidden_terrain: Option<Vec<String>>,
    /// Minimum visibility required
    pub min_visibility: u8,
}

impl Default for PathConstraints {
    fn default() -> Self {
        Self {
            avoid_enemies: false,
            max_distance: None,
            required_terrain: None,
            forbidden_terrain: None,
            min_visibility: 0,
        }
    }
}

impl PathfindingCacheKey {
    pub fn new(start: IVec2, end: IVec2, movement_type: MovementType, player_id: u32, world_generation: u32) -> Self {
        Self {
            start,
            end,
            movement_type,
            player_id,
            world_generation,
            path_constraints: PathConstraints::default(),
        }
    }

    pub fn with_constraints(mut self, constraints: PathConstraints) -> Self {
        self.path_constraints = constraints;
        self
    }

    /// Calculate cache priority based on path complexity
    pub fn cache_priority(&self) -> CachePriority {
        let distance = self.start.distance_squared(self.end) as u32;
        
        if distance > 100 {
            CachePriority::High // Long paths are expensive to compute
        } else if self.path_constraints.avoid_enemies || self.path_constraints.required_terrain.is_some() {
            CachePriority::Normal // Complex constraints
        } else {
            CachePriority::Low // Simple short paths
        }
    }
}

/// Pathfinding result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathfindingResult {
    /// The calculated path
    pub path: Vec<IVec2>,
    /// Total movement cost
    pub total_cost: u32,
    /// Whether path was found
    pub found: bool,
    /// Reason for failure if path not found
    pub failure_reason: Option<PathfindingFailure>,
    /// Computation time in microseconds
    pub computation_time_us: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PathfindingFailure {
    NoPathExists,
    ExceedsMaxDistance,
    BlockedByEnemies,
    InsufficientMovement,
    TerrainImpassable,
}

impl PathfindingResult {
    pub fn size_bytes(&self) -> usize {
        self.path.len() * 8 + 32 // Path + metadata
    }

    pub fn is_valid(&self) -> bool {
        self.found && !self.path.is_empty()
    }
}

// =================================================================================================
// AI CACHE
// =================================================================================================

/// AI cache key for decision making results
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct AICacheKey {
    /// Entity being evaluated
    pub entity: Entity,
    /// AI context/situation
    pub context: AIContext,
    /// Decision tree depth
    pub depth: u8,
    /// Player ID for AI state
    pub player_id: u32,
    /// Turn number for invalidation
    pub turn: u32,
}

/// AI context types
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum AIContext {
    /// Unit combat evaluation
    Combat {
        enemy_entities: Vec<Entity>,
        terrain_bonus: i16,
    },
    /// City production decision
    Production {
        available_options: Vec<u32>, // Production option IDs
        current_needs: CityNeeds,
    },
    /// Research/technology decisions
    Research {
        available_techs: Vec<u32>,
        research_focus: ResearchFocus,
    },
    /// Diplomatic evaluation
    Diplomacy {
        other_players: Vec<u32>,
        relationship_context: DiplomaticContext,
    },
    /// Strategic planning
    Strategy {
        objectives: Vec<StrategicObjective>,
        time_horizon: u32,
    },
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct CityNeeds {
    pub military_pressure: u8,  // 0-100
    pub growth_priority: u8,    // 0-100
    pub economic_need: u8,      // 0-100
    pub infrastructure_need: u8, // 0-100
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResearchFocus {
    Military,
    Economic,
    Infrastructure,
    Cultural,
    Balanced,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiplomaticContext {
    Trade,
    Alliance,
    Conflict,
    Neutral,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum StrategicObjective {
    Expansion,
    Defense,
    Economic,
    Victory,
}

impl AICacheKey {
    pub fn combat_evaluation(entity: Entity, enemies: Vec<Entity>, player_id: u32, turn: u32) -> Self {
        Self {
            entity,
            context: AIContext::Combat {
                enemy_entities: enemies,
                terrain_bonus: 0,
            },
            depth: 3, // Standard combat evaluation depth
            player_id,
            turn,
        }
    }

    pub fn production_decision(entity: Entity, options: Vec<u32>, needs: CityNeeds, player_id: u32, turn: u32) -> Self {
        Self {
            entity,
            context: AIContext::Production {
                available_options: options,
                current_needs: needs,
            },
            depth: 2, // Production decisions are simpler
            player_id,
            turn,
        }
    }

    pub fn cache_priority(&self) -> CachePriority {
        match &self.context {
            AIContext::Combat { .. } => CachePriority::High, // Combat is time-sensitive
            AIContext::Production { .. } => CachePriority::Normal,
            AIContext::Research { .. } => CachePriority::Normal,
            AIContext::Diplomacy { .. } => CachePriority::Low, // Less frequent
            AIContext::Strategy { .. } => CachePriority::Low, // Long-term planning
        }
    }
}

/// AI evaluation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIResult {
    /// Recommended action
    pub action: AIAction,
    /// Confidence score (0.0-1.0)
    pub confidence: f32,
    /// Alternative actions considered
    pub alternatives: Vec<(AIAction, f32)>,
    /// Reasoning (for debugging/explanation)
    pub reasoning: String,
    /// Computation complexity score
    pub complexity_score: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AIAction {
    /// Move unit to position
    Move { target: IVec2, path: Vec<IVec2> },
    /// Attack target entity
    Attack { target: Entity },
    /// Build/produce something
    Build { production_id: u32, location: Option<IVec2> },
    /// Research technology
    Research { tech_id: u32 },
    /// Diplomatic action
    Diplomacy { target_player: u32, action_type: String },
    /// Do nothing this turn
    Wait,
    /// Custom action
    Custom { action_type: String, parameters: HashMap<String, String> },
}

impl AIResult {
    pub fn size_bytes(&self) -> usize {
        let base_size = 32;
        let alternatives_size = self.alternatives.len() * 24;
        let reasoning_size = self.reasoning.len();
        
        base_size + alternatives_size + reasoning_size
    }
}

// =================================================================================================
// RENDERING CACHE
// =================================================================================================

/// Rendering cache key
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderingCacheKey {
    /// Asset type
    pub asset_type: RenderingAssetType,
    /// Asset identifier
    pub asset_id: String,
    /// Rendering parameters
    pub parameters: RenderingParameters,
    /// LOD level
    pub lod_level: u8,
    /// Theme/style variant
    pub theme: Option<String>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum RenderingAssetType {
    /// Unit sprites/models
    Unit,
    /// Building/city graphics
    Building,
    /// Terrain tiles
    Terrain,
    /// UI elements
    UI,
    /// Effect animations
    Effect,
    /// Technology icons
    Technology,
    /// Resource icons
    Resource,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderingParameters {
    /// Size/scale multiplier
    pub scale: u16, // Fixed-point scale (100 = 1.0x)
    /// Rotation in degrees
    pub rotation: u16,
    /// Color tint (RGBA)
    pub tint: Option<[u8; 4]>,
    /// Animation frame
    pub frame: Option<u16>,
    /// Player color scheme
    pub player_colors: Option<u32>,
}

impl Default for RenderingParameters {
    fn default() -> Self {
        Self {
            scale: 100, // 1.0x scale
            rotation: 0,
            tint: None,
            frame: None,
            player_colors: None,
        }
    }
}

impl RenderingCacheKey {
    pub fn unit_sprite(unit_type: &str, player_colors: u32) -> Self {
        Self {
            asset_type: RenderingAssetType::Unit,
            asset_id: unit_type.to_string(),
            parameters: RenderingParameters {
                player_colors: Some(player_colors),
                ..Default::default()
            },
            lod_level: 0,
            theme: None,
        }
    }

    pub fn terrain_tile(terrain_type: &str, theme: Option<String>) -> Self {
        Self {
            asset_type: RenderingAssetType::Terrain,
            asset_id: terrain_type.to_string(),
            parameters: RenderingParameters::default(),
            lod_level: 0,
            theme,
        }
    }

    pub fn cache_priority(&self) -> CachePriority {
        match self.asset_type {
            RenderingAssetType::Unit | RenderingAssetType::Building => CachePriority::Normal,
            RenderingAssetType::Terrain => CachePriority::High, // Terrain is accessed very frequently
            RenderingAssetType::UI => CachePriority::High, // UI needs to be responsive
            RenderingAssetType::Effect => CachePriority::Low, // Effects are temporary
            RenderingAssetType::Technology | RenderingAssetType::Resource => CachePriority::Low,
        }
    }
}

/// Rendering data result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RenderingResult {
    /// Sprite/texture data
    Sprite {
        data: Vec<u8>, // Image data
        width: u32,
        height: u32,
        format: String,
    },
    /// 3D model data  
    Model {
        vertices: Vec<f32>,
        indices: Vec<u32>,
        materials: Vec<String>,
    },
    /// UI layout data
    Layout {
        elements: Vec<UIElement>,
        total_size: (u32, u32),
    },
    /// Animation frame sequence
    Animation {
        frames: Vec<AnimationFrame>,
        duration_ms: u32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIElement {
    pub position: (f32, f32),
    pub size: (f32, f32),
    pub element_type: String,
    pub properties: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationFrame {
    pub data: Vec<u8>,
    pub duration_ms: u32,
}

impl RenderingResult {
    pub fn size_bytes(&self) -> usize {
        match self {
            RenderingResult::Sprite { data, .. } => data.len() + 24,
            RenderingResult::Model { vertices, indices, .. } => {
                vertices.len() * 4 + indices.len() * 4 + 64
            }
            RenderingResult::Layout { elements, .. } => {
                elements.len() * 64 + 16 // Approximate
            }
            RenderingResult::Animation { frames, .. } => {
                frames.iter().map(|f| f.data.len() + 8).sum::<usize>() + 8
            }
        }
    }
}

// =================================================================================================
// PLAYER DATA CACHE  
// =================================================================================================

/// Player data cache key
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerCacheKey {
    pub player_id: u32,
    pub data_type: PlayerDataType,
    pub turn: u32, // For turn-based data
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayerDataType {
    /// Technology tree state
    Technologies,
    /// Resource stockpiles
    Resources,
    /// Diplomatic relationships
    Diplomacy,
    /// Territory/border information
    Territory,
    /// Military unit counts and composition
    Military,
    /// City and population information  
    Cities,
    /// Trade routes and economic data
    Economy,
    /// Victory progress tracking
    Victory,
}

impl PlayerCacheKey {
    pub fn technologies(player_id: u32, turn: u32) -> Self {
        Self {
            player_id,
            data_type: PlayerDataType::Technologies,
            turn,
        }
    }

    pub fn resources(player_id: u32, turn: u32) -> Self {
        Self {
            player_id,
            data_type: PlayerDataType::Resources,
            turn,
        }
    }

    pub fn cache_priority(&self) -> CachePriority {
        match self.data_type {
            PlayerDataType::Technologies => CachePriority::Critical,
            PlayerDataType::Resources => CachePriority::Critical,
            PlayerDataType::Territory => CachePriority::High,
            PlayerDataType::Military => CachePriority::High,
            PlayerDataType::Cities => CachePriority::Normal,
            PlayerDataType::Diplomacy => CachePriority::Normal,
            PlayerDataType::Economy => CachePriority::Normal,
            PlayerDataType::Victory => CachePriority::Low,
        }
    }
}

/// Player data result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlayerDataResult {
    /// Technology tree with research progress
    Technologies {
        completed: FastHashSet<u32>,
        in_progress: Option<(u32, u16)>, // Tech ID, progress
        available: FastHashSet<u32>,
    },
    /// Resource stockpiles
    Resources {
        stockpiles: FastHashMap<String, u32>,
        income: FastHashMap<String, i32>,
        capacity: FastHashMap<String, u32>,
    },
    /// Diplomatic state
    Diplomacy {
        relationships: FastHashMap<u32, DiplomaticRelationship>,
        active_deals: Vec<DiplomaticDeal>,
    },
    /// Territory control
    Territory {
        controlled_tiles: FastHashSet<IVec2>,
        borders: Vec<IVec2>,
        total_area: u32,
    },
    /// Military summary
    Military {
        unit_counts: FastHashMap<String, u32>,
        total_strength: u32,
        maintenance_cost: u32,
    },
}

use crate::core::hashing::FastHashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiplomaticRelationship {
    pub attitude: i16, // -100 to +100
    pub relationship_type: String,
    pub trade_deals: u16,
    pub military_agreements: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiplomaticDeal {
    pub deal_id: u32,
    pub other_player: u32,
    pub deal_type: String,
    pub terms: HashMap<String, i32>,
    pub expires_turn: u32,
}

impl PlayerDataResult {
    pub fn size_bytes(&self) -> usize {
        match self {
            PlayerDataResult::Technologies { completed, available, .. } => {
                completed.len() * 4 + available.len() * 4 + 16
            }
            PlayerDataResult::Resources { stockpiles, income, capacity } => {
                stockpiles.len() * 32 + income.len() * 32 + capacity.len() * 32
            }
            PlayerDataResult::Diplomacy { relationships, active_deals } => {
                relationships.len() * 64 + active_deals.len() * 128
            }
            PlayerDataResult::Territory { controlled_tiles, borders, .. } => {
                controlled_tiles.len() * 8 + borders.len() * 8 + 8
            }
            PlayerDataResult::Military { unit_counts, .. } => {
                unit_counts.len() * 32 + 16
            }
        }
    }
}
