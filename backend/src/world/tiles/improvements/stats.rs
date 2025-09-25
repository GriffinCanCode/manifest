//! Statistics, results, and error types for improvements
//!
//! Contains error handling, turn results, and statistics for the improvement system.

use serde::{Deserialize, Serialize};
use crate::world::tiles::{
    chunks::TileId,
    components::TerrainType,
    ownership::PlayerId
};
use super::types::{ImprovementKey, ImprovementType};

/// Errors that can occur during improvement operations
#[derive(Debug, Clone, thiserror::Error, Serialize, Deserialize)]
pub enum ImprovementError {
    #[error("Improvement not found: {0:?}")]
    ImprovementNotFound(ImprovementKey),
    
    #[error("Tile not found: {tile_id:?}")]
    TileNotFound { tile_id: TileId },
    
    #[error("Tile capacity exceeded - maximum improvements per tile")]
    TileCapacityExceeded,
    
    #[error("Invalid terrain: cannot build {improvement_type:?} on {terrain_type:?}")]
    InvalidTerrain { 
        improvement_type: ImprovementType,
        terrain_type: TerrainType 
    },
    
    #[error("Incompatible improvement: {improvement_type:?} cannot coexist with existing improvements")]
    IncompatibleImprovement { 
        improvement_type: ImprovementType 
    },
    
    #[error("Insufficient resources: need {required}, have {available}")]
    InsufficientResources { 
        required: u32,
        available: u32 
    },
    
    #[error("Improvement not completed: cannot {action} improvement in state {state}")]
    ImprovementNotCompleted { 
        action: String,
        state: String 
    },
    
    #[error("Permission denied: player {player_id} cannot modify improvement owned by {owner_id}")]
    PermissionDenied { 
        player_id: PlayerId,
        owner_id: PlayerId 
    },
    
    #[error("Invalid upgrade: {improvement_type:?} already at maximum level {level}")]
    InvalidUpgrade { 
        improvement_type: ImprovementType,
        level: u8 
    },
    
    #[error("Construction blocked: {reason}")]
    ConstructionBlocked { reason: String },
    
    #[error("Serialization error: {message}")]
    SerializationError { message: String },
    
    #[error("Cache error: {message}")]
    CacheError { message: String },
    
    #[error("System error: {message}")]
    SystemError { message: String },
}

/// Result type for improvement operations
pub type ImprovementResult<T> = Result<T, ImprovementError>;

/// Results from processing improvements for a turn
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovementTurnResults {
    /// Improvements completed this turn
    pub completed_constructions: Vec<ImprovementCompletionInfo>,
    /// Improvements that completed upgrades
    pub completed_upgrades: Vec<ImprovementUpgradeInfo>,
    /// Improvements that were repaired
    pub completed_repairs: Vec<ImprovementKey>,
    /// Improvements that were destroyed
    pub destroyed_improvements: Vec<ImprovementDestructionInfo>,
    /// Total maintenance costs incurred
    pub total_maintenance_cost: u32,
    /// Total yields generated
    pub total_yields: super::core::ResourceYields,
    /// Errors that occurred during processing
    pub errors: Vec<ImprovementError>,
}

/// Information about a completed improvement construction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovementCompletionInfo {
    pub key: ImprovementKey,
    pub improvement_type: ImprovementType,
    pub tile_id: TileId,
    pub owner: Option<PlayerId>,
    pub completion_turn: u32,
    pub construction_time: u32,
}

/// Information about a completed improvement upgrade
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovementUpgradeInfo {
    pub key: ImprovementKey,
    pub improvement_type: ImprovementType,
    pub tile_id: TileId,
    pub owner: Option<PlayerId>,
    pub old_level: u8,
    pub new_level: u8,
    pub completion_turn: u32,
}

/// Information about a destroyed improvement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovementDestructionInfo {
    pub key: ImprovementKey,
    pub improvement_type: ImprovementType,
    pub tile_id: TileId,
    pub owner: Option<PlayerId>,
    pub destruction_cause: DestructionCause,
}

/// Causes of improvement destruction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DestructionCause {
    /// Destroyed by military action
    Combat,
    /// Natural disaster
    NaturalDisaster { disaster_type: String },
    /// Lack of maintenance
    Neglect,
    /// Deliberately demolished by owner
    Demolition,
    /// Infrastructure failure
    SystemFailure,
    /// Environmental damage
    Environmental,
}

impl Default for ImprovementTurnResults {
    fn default() -> Self {
        Self {
            completed_constructions: Vec::new(),
            completed_upgrades: Vec::new(),
            completed_repairs: Vec::new(),
            destroyed_improvements: Vec::new(),
            total_maintenance_cost: 0,
            total_yields: super::core::ResourceYields::zero(),
            errors: Vec::new(),
        }
    }
}

impl ImprovementTurnResults {
    /// Create empty turn results
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a completed construction
    pub fn add_completion(&mut self, info: ImprovementCompletionInfo) {
        self.completed_constructions.push(info);
    }

    /// Add a completed upgrade
    pub fn add_upgrade(&mut self, info: ImprovementUpgradeInfo) {
        self.completed_upgrades.push(info);
    }

    /// Add a completed repair
    pub fn add_repair(&mut self, key: ImprovementKey) {
        self.completed_repairs.push(key);
    }

    /// Add a destroyed improvement
    pub fn add_destruction(&mut self, info: ImprovementDestructionInfo) {
        self.destroyed_improvements.push(info);
    }

    /// Add maintenance cost
    pub fn add_maintenance_cost(&mut self, cost: u32) {
        self.total_maintenance_cost += cost;
    }

    /// Add yields
    pub fn add_yields(&mut self, yields: super::core::ResourceYields) {
        self.total_yields = self.total_yields.add(yields);
    }

    /// Add an error
    pub fn add_error(&mut self, error: ImprovementError) {
        self.errors.push(error);
    }

    /// Check if there were any errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Get count of completed activities
    pub fn total_completions(&self) -> usize {
        self.completed_constructions.len() + 
        self.completed_upgrades.len() + 
        self.completed_repairs.len()
    }

    /// Check if any improvements were completed this turn
    pub fn has_completions(&self) -> bool {
        self.total_completions() > 0
    }
}

/// Statistics about improvements across the game
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovementStats {
    /// Total number of improvements
    pub total_improvements: usize,
    /// Improvements by type
    pub by_type: std::collections::HashMap<ImprovementType, usize>,
    /// Improvements by player
    pub by_player: std::collections::HashMap<PlayerId, usize>,
    /// Improvements by state
    pub by_state: StateBreakdown,
    /// Total construction cost invested
    pub total_investment: u32,
    /// Total maintenance cost per turn
    pub total_maintenance: u32,
    /// Total yields generated per turn
    pub total_yields: super::core::ResourceYields,
    /// Memory usage in bytes
    pub memory_usage: usize,
}

/// Breakdown of improvements by state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateBreakdown {
    pub planned: usize,
    pub under_construction: usize,
    pub completed: usize,
    pub damaged: usize,
    pub destroyed: usize,
    pub under_repair: usize,
}

impl Default for ImprovementStats {
    fn default() -> Self {
        Self {
            total_improvements: 0,
            by_type: std::collections::HashMap::new(),
            by_player: std::collections::HashMap::new(),
            by_state: StateBreakdown::default(),
            total_investment: 0,
            total_maintenance: 0,
            total_yields: super::core::ResourceYields::zero(),
            memory_usage: 0,
        }
    }
}

impl Default for StateBreakdown {
    fn default() -> Self {
        Self {
            planned: 0,
            under_construction: 0,
            completed: 0,
            damaged: 0,
            destroyed: 0,
            under_repair: 0,
        }
    }
}

impl ImprovementStats {
    /// Create empty stats
    pub fn new() -> Self {
        Self::default()
    }

    /// Calculate efficiency ratio (completed / total)
    pub fn efficiency_ratio(&self) -> f32 {
        if self.total_improvements == 0 {
            0.0
        } else {
            self.by_state.completed as f32 / self.total_improvements as f32
        }
    }

    /// Calculate construction progress ratio
    pub fn construction_progress(&self) -> f32 {
        let total_non_planned = self.total_improvements - self.by_state.planned;
        if total_non_planned == 0 {
            0.0
        } else {
            self.by_state.completed as f32 / total_non_planned as f32
        }
    }

    /// Get most common improvement type
    pub fn most_common_type(&self) -> Option<ImprovementType> {
        self.by_type.iter()
            .max_by_key(|(_, count)| *count)
            .map(|(imp_type, _)| *imp_type)
    }

    /// Get player with most improvements
    pub fn top_builder(&self) -> Option<PlayerId> {
        self.by_player.iter()
            .max_by_key(|(_, count)| *count)
            .map(|(player_id, _)| *player_id)
    }

    /// Calculate return on investment (yields per investment)
    pub fn return_on_investment(&self) -> f32 {
        if self.total_investment == 0 {
            0.0
        } else {
            let total_yield_value = self.total_yields.food + 
                                   self.total_yields.production + 
                                   self.total_yields.commerce + 
                                   self.total_yields.culture + 
                                   self.total_yields.science;
            total_yield_value as f32 / self.total_investment as f32
        }
    }

    /// Calculate maintenance burden (maintenance per yield)
    pub fn maintenance_burden(&self) -> f32 {
        let total_yield_value = self.total_yields.food + 
                               self.total_yields.production + 
                               self.total_yields.commerce + 
                               self.total_yields.culture + 
                               self.total_yields.science;
        if total_yield_value == 0 {
            0.0
        } else {
            self.total_maintenance as f32 / total_yield_value as f32
        }
    }
}
