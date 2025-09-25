//! Improvement slots using slotmap for efficient tile improvement management
//!
//! Provides high-performance slot-based storage for tile improvements using
//! slotmap for stable handles and efficient memory management.

use slotmap::{SlotMap, HopSlotMap, SecondaryMap, Key};
use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use arrayvec::ArrayVec;

use crate::core::{
    zig_ffi::HexCoord,
    hashing::{FastHashMap, FastHashSet},
    caching::{GameCache, GameCacheBuilder, CacheKey, CachePriority}
};
use crate::world::tiles::{
    chunks::{TileId, ChunkCoord},
    components::{Tile, TerrainType, TileComponentManager},
    ownership::{PlayerId, OwnershipStrength}
};
use tracing::{debug, instrument, warn};

/// Maximum number of improvements per tile
pub const MAX_IMPROVEMENTS_PER_TILE: usize = 8;

/// Unique identifier for improvements using slotmap
slotmap::new_key_type! {
    /// Stable handle to an improvement that remains valid across saves/loads
    pub struct ImprovementKey;
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
    
    // Late Game
    PowerPlant = 60,
    Airport = 61,
    SpaceCenter = 62,
    ResearchLab = 63,
    NuclearReactor = 64,
}

impl ImprovementType {
    /// Get all improvement types
    pub const fn all() -> &'static [ImprovementType] {
        &[
            ImprovementType::Farm, ImprovementType::Mine, ImprovementType::Lumbermill,
            ImprovementType::Quarry, ImprovementType::Pasture, ImprovementType::Road,
            ImprovementType::Railroad, ImprovementType::Bridge, ImprovementType::Tunnel,
            ImprovementType::Fort, ImprovementType::TradingPost, ImprovementType::Market,
            ImprovementType::Bank, ImprovementType::Factory, ImprovementType::Port,
            ImprovementType::Temple, ImprovementType::University, ImprovementType::Library,
            ImprovementType::Monument, ImprovementType::Theater, ImprovementType::Barracks,
            ImprovementType::Arsenal, ImprovementType::Fortress, ImprovementType::Watchtower,
            ImprovementType::Bunker, ImprovementType::Observatory, ImprovementType::Lighthouse,
            ImprovementType::Aqueduct, ImprovementType::Windmill, ImprovementType::Irrigation,
            ImprovementType::PowerPlant, ImprovementType::Airport, ImprovementType::SpaceCenter,
            ImprovementType::ResearchLab, ImprovementType::NuclearReactor,
        ]
    }

    /// Get category of improvement
    pub fn category(self) -> ImprovementCategory {
        match self {
            ImprovementType::Farm | ImprovementType::Mine | ImprovementType::Lumbermill |
            ImprovementType::Quarry | ImprovementType::Pasture => ImprovementCategory::Resource,
            
            ImprovementType::Road | ImprovementType::Railroad | ImprovementType::Bridge |
            ImprovementType::Tunnel | ImprovementType::Aqueduct | ImprovementType::Irrigation => ImprovementCategory::Infrastructure,
            
            ImprovementType::TradingPost | ImprovementType::Market | ImprovementType::Bank |
            ImprovementType::Factory | ImprovementType::Port => ImprovementCategory::Economic,
            
            ImprovementType::Temple | ImprovementType::University | ImprovementType::Library |
            ImprovementType::Monument | ImprovementType::Theater => ImprovementCategory::Cultural,
            
            ImprovementType::Barracks | ImprovementType::Arsenal | ImprovementType::Fortress |
            ImprovementType::Watchtower | ImprovementType::Bunker | ImprovementType::Fort => ImprovementCategory::Military,
            
            ImprovementType::Observatory | ImprovementType::Lighthouse | ImprovementType::Windmill |
            ImprovementType::PowerPlant | ImprovementType::Airport | ImprovementType::SpaceCenter |
            ImprovementType::ResearchLab | ImprovementType::NuclearReactor => ImprovementCategory::Specialized,
        }
    }

    /// Get base construction cost
    pub fn base_cost(self) -> u32 {
        match self {
            ImprovementType::Farm | ImprovementType::Pasture => 50,
            ImprovementType::Mine | ImprovementType::Quarry => 75,
            ImprovementType::Lumbermill => 60,
            ImprovementType::Road => 25,
            ImprovementType::Railroad => 100,
            ImprovementType::Bridge => 150,
            ImprovementType::Tunnel => 200,
            ImprovementType::Fort => 120,
            ImprovementType::TradingPost => 80,
            ImprovementType::Market => 150,
            ImprovementType::Bank => 300,
            ImprovementType::Factory => 400,
            ImprovementType::Port => 200,
            ImprovementType::Temple => 100,
            ImprovementType::University => 500,
            ImprovementType::Library => 200,
            ImprovementType::Monument => 250,
            ImprovementType::Theater => 180,
            ImprovementType::Barracks => 120,
            ImprovementType::Arsenal => 300,
            ImprovementType::Fortress => 500,
            ImprovementType::Watchtower => 80,
            ImprovementType::Bunker => 250,
            ImprovementType::Observatory => 400,
            ImprovementType::Lighthouse => 150,
            ImprovementType::Aqueduct => 180,
            ImprovementType::Windmill => 120,
            ImprovementType::Irrigation => 100,
            ImprovementType::PowerPlant => 800,
            ImprovementType::Airport => 1000,
            ImprovementType::SpaceCenter => 2000,
            ImprovementType::ResearchLab => 600,
            ImprovementType::NuclearReactor => 1500,
        }
    }

    /// Get construction time in turns
    pub fn construction_time(self) -> u16 {
        (self.base_cost() / 50).max(1) as u16
    }

    /// Check if improvement can be built on terrain type
    pub fn can_build_on_terrain(self, terrain: TerrainType) -> bool {
        use TerrainType::*;
        use ImprovementType::*;
        
        match (self, terrain) {
            // Farms work on most fertile land
            (Farm, Grassland | Plains) => true,
            
            // Mines work on hills/mountains
            (Mine, Hills | Mountain) => true,
            
            // Lumbermill requires forest
            (Lumbermill, Forest | Jungle) => true,
            
            // Quarry works on hills/mountains
            (Quarry, Hills | Mountain | Desert) => true,
            
            // Pasture works on grasslands
            (Pasture, Grassland | Plains) => true,
            
            // Infrastructure can be built almost anywhere on land
            (Road | Railroad | Bridge | Fort | Watchtower, terrain) if terrain != Ocean => true,
            
            // Ports require coastal access
            (Port, _) => true, // Would check for adjacent ocean in real implementation
            
            // Most improvements can be built on suitable terrain
            (_, terrain) if terrain != Ocean => true,
            
            _ => false,
        }
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

/// State of improvement construction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImprovementState {
    /// Improvement is under construction
    UnderConstruction { turns_remaining: u16 },
    /// Improvement is completed and functional
    Completed,
    /// Improvement is damaged and not fully functional
    Damaged { repair_cost: u32 },
    /// Improvement is being upgraded
    Upgrading { turns_remaining: u16, target_level: u8 },
    /// Improvement is being demolished
    Demolishing { turns_remaining: u16 },
}

impl Default for ImprovementState {
    fn default() -> Self {
        Self::Completed
    }
}

impl ImprovementState {
    /// Check if improvement is functional
    pub fn is_functional(self) -> bool {
        matches!(self, ImprovementState::Completed | ImprovementState::Upgrading { .. })
    }

    /// Get efficiency factor (0.0 to 1.0)
    pub fn efficiency_factor(self) -> f32 {
        match self {
            ImprovementState::Completed => 1.0,
            ImprovementState::Upgrading { .. } => 0.8,
            ImprovementState::Damaged { .. } => 0.5,
            ImprovementState::UnderConstruction { .. } => 0.0,
            ImprovementState::Demolishing { .. } => 0.0,
        }
    }
}

/// Individual improvement instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Improvement {
    /// Type of improvement
    pub improvement_type: ImprovementType,
    /// Current state of the improvement
    pub state: ImprovementState,
    /// Level of the improvement (for upgrades)
    pub level: u8,
    /// Owner of the improvement
    pub owner: Option<PlayerId>,
    /// Tile this improvement is on
    pub tile_id: TileId,
    /// Hex coordinate for quick lookup
    pub hex: HexCoord,
    /// Turn when improvement was built
    pub built_turn: u32,
    /// Last maintenance turn
    pub last_maintenance: u32,
    /// Custom properties for scripting
    pub properties: FastHashMap<String, f32>,
}

impl Improvement {
    /// Create new improvement
    pub fn new(improvement_type: ImprovementType, tile_id: TileId, hex: HexCoord, owner: Option<PlayerId>) -> Self {
        let construction_time = improvement_type.construction_time();
        
        Self {
            improvement_type,
            state: ImprovementState::UnderConstruction { turns_remaining: construction_time },
            level: 1,
            owner,
            tile_id,
            hex,
            built_turn: 0, // Would be set by game logic
            last_maintenance: 0,
            properties: FastHashMap::default(),
        }
    }

    /// Get current efficiency of improvement
    pub fn efficiency(&self) -> f32 {
        let base_efficiency = self.state.efficiency_factor();
        let level_modifier = 1.0 + (self.level as f32 - 1.0) * 0.2; // +20% per level
        
        base_efficiency * level_modifier
    }

    /// Get maintenance cost per turn
    pub fn maintenance_cost(&self) -> u32 {
        let base_cost = self.improvement_type.base_cost() / 10; // 10% of construction cost
        let level_cost = base_cost * (self.level as u32);
        
        match self.state {
            ImprovementState::Damaged { .. } => level_cost * 2, // Damaged improvements cost more
            _ => level_cost,
        }
    }

    /// Check if improvement can be upgraded
    pub fn can_upgrade(&self) -> bool {
        matches!(self.state, ImprovementState::Completed) && self.level < 5
    }

    /// Get upgrade cost
    pub fn upgrade_cost(&self) -> u32 {
        if self.can_upgrade() {
            self.improvement_type.base_cost() * (self.level as u32 + 1) / 2
        } else {
            0
        }
    }

    /// Start upgrade process
    pub fn start_upgrade(&mut self) {
        if self.can_upgrade() {
            let upgrade_time = self.improvement_type.construction_time() / 2;
            self.state = ImprovementState::Upgrading {
                turns_remaining: upgrade_time,
                target_level: self.level + 1,
            };
        }
    }

    /// Apply damage to improvement
    pub fn apply_damage(&mut self, damage_amount: u32) {
        if matches!(self.state, ImprovementState::Completed | ImprovementState::Upgrading { .. }) {
            let repair_cost = damage_amount * self.improvement_type.base_cost() / 100;
            self.state = ImprovementState::Damaged { repair_cost };
        }
    }

    /// Repair improvement
    pub fn repair(&mut self) {
        if let ImprovementState::Damaged { .. } = self.state {
            self.state = ImprovementState::Completed;
        }
    }

    /// Process turn for this improvement
    pub fn process_turn(&mut self) -> bool {
        match &mut self.state {
            ImprovementState::UnderConstruction { turns_remaining } => {
                *turns_remaining = turns_remaining.saturating_sub(1);
                if *turns_remaining == 0 {
                    self.state = ImprovementState::Completed;
                    return true; // Construction completed
                }
            }
            ImprovementState::Upgrading { turns_remaining, target_level } => {
                *turns_remaining = turns_remaining.saturating_sub(1);
                if *turns_remaining == 0 {
                    self.level = *target_level;
                    self.state = ImprovementState::Completed;
                    return true; // Upgrade completed
                }
            }
            ImprovementState::Demolishing { turns_remaining } => {
                *turns_remaining = turns_remaining.saturating_sub(1);
                return *turns_remaining == 0; // Ready for removal
            }
            _ => {}
        }
        
        false
    }

    /// Get memory usage in bytes
    pub fn memory_size(&self) -> usize {
        std::mem::size_of::<Self>() +
        self.properties.len() * (std::mem::size_of::<String>() + std::mem::size_of::<f32>())
    }
}

/// Collection of improvements for a single tile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileImprovements {
    /// Improvements on this tile (limited capacity)
    improvements: ArrayVec<ImprovementKey, MAX_IMPROVEMENTS_PER_TILE>,
    /// Tile this collection belongs to
    tile_id: TileId,
}

impl TileImprovements {
    /// Create new tile improvements collection
    pub fn new(tile_id: TileId) -> Self {
        Self {
            improvements: ArrayVec::new(),
            tile_id,
        }
    }

    /// Add improvement to tile
    pub fn add_improvement(&mut self, key: ImprovementKey) -> Result<(), ImprovementError> {
        if self.improvements.is_full() {
            return Err(ImprovementError::TileCapacityExceeded);
        }
        
        self.improvements.push(key);
        Ok(())
    }

    /// Remove improvement from tile
    pub fn remove_improvement(&mut self, key: ImprovementKey) -> bool {
        if let Some(pos) = self.improvements.iter().position(|&k| k == key) {
            self.improvements.swap_remove(pos);
            true
        } else {
            false
        }
    }

    /// Get all improvement keys
    pub fn improvements(&self) -> &[ImprovementKey] {
        &self.improvements
    }

    /// Get number of improvements
    pub fn count(&self) -> usize {
        self.improvements.len()
    }

    /// Check if tile has specific improvement type
    pub fn has_improvement_type(&self, improvement_type: ImprovementType, improvements_map: &SlotMap<ImprovementKey, Improvement>) -> bool {
        self.improvements.iter()
            .any(|&key| {
                improvements_map.get(key)
                    .map(|imp| imp.improvement_type == improvement_type)
                    .unwrap_or(false)
            })
    }

    /// Check if tile can accept another improvement
    pub fn can_add_improvement(&self, improvement_type: ImprovementType, improvements_map: &SlotMap<ImprovementKey, Improvement>) -> bool {
        if self.improvements.is_full() {
            return false;
        }

        // Check for conflicting improvements
        match improvement_type.category() {
            ImprovementCategory::Resource => {
                // Only one resource improvement per tile
                !self.improvements.iter().any(|&key| {
                    improvements_map.get(key)
                        .map(|imp| imp.improvement_type.category() == ImprovementCategory::Resource)
                        .unwrap_or(false)
                })
            }
            _ => true, // Other categories can coexist
        }
    }
}

/// High-performance improvement management system using slotmap
#[derive(Debug)]
pub struct TileImprovementManager {
    /// Master storage for all improvements using slotmap
    improvements: Arc<RwLock<SlotMap<ImprovementKey, Improvement>>>,
    /// Secondary map for tile -> improvements lookup
    tile_improvements: Arc<RwLock<HopSlotMap<TileId, TileImprovements>>>,
    /// Spatial index for location-based queries
    spatial_index: Arc<RwLock<FastHashMap<HexCoord, ImprovementKey>>>,
    /// Owner index for player-based queries  
    owner_index: Arc<RwLock<FastHashMap<PlayerId, Vec<ImprovementKey>>>>,
    /// Type index for improvement type queries
    type_index: Arc<RwLock<FastHashMap<ImprovementType, Vec<ImprovementKey>>>>,
    /// Cache for improvement queries
    cache: GameCache,
    /// Tile component manager for validation
    tile_manager: Arc<TileComponentManager>,
}

impl TileImprovementManager {
    /// Create new improvement manager
    pub fn new(tile_manager: Arc<TileComponentManager>) -> Self {
        let cache = GameCacheBuilder::new()
            .max_memory_mb(64) // 64MB for improvement cache
            .default_ttl(std::time::Duration::from_secs(180)) // 3 minute TTL
            .turn_based_invalidation(false)
            .build();

        Self {
            improvements: Arc::new(RwLock::new(SlotMap::with_key())),
            tile_improvements: Arc::new(RwLock::new(HopSlotMap::with_key())),
            spatial_index: Arc::new(RwLock::new(FastHashMap::default())),
            owner_index: Arc::new(RwLock::new(FastHashMap::default())),
            type_index: Arc::new(RwLock::new(FastHashMap::default())),
            cache,
            tile_manager,
        }
    }

    /// Add new improvement to a tile
    #[instrument(skip(self))]
    pub fn add_improvement(&self, tile_id: TileId, improvement_type: ImprovementType, owner: Option<PlayerId>) -> Result<ImprovementKey, ImprovementError> {
        // Get tile information for validation
        let tile = self.tile_manager.get_component::<Tile>(tile_id)
            .map_err(|_| ImprovementError::TileNotFound { tile_id })?;

        // Check if improvement can be built on this terrain
        if !improvement_type.can_build_on_terrain(tile.terrain_type) {
            return Err(ImprovementError::InvalidTerrain { 
                improvement_type, 
                terrain_type: tile.terrain_type 
            });
        }

        // Check tile capacity
        {
            let tile_improvements = self.tile_improvements.read();
            if let Some(existing) = tile_improvements.get(tile_id) {
                let improvements = self.improvements.read();
                if !existing.can_add_improvement(improvement_type, &improvements) {
                    return Err(ImprovementError::TileCapacityExceeded);
                }
            }
        }

        // Create improvement
        let improvement = Improvement::new(improvement_type, tile_id, tile.hex, owner);
        
        // Add to main storage
        let key = {
            let mut improvements = self.improvements.write();
            improvements.insert(improvement)
        };

        // Update indices
        self.update_indices_for_add(key, &tile, improvement_type, owner);

        // Add to tile improvements
        {
            let mut tile_improvements = self.tile_improvements.write();
            let tile_imps = tile_improvements.entry(tile_id).or_insert_with(|| TileImprovements::new(tile_id));
            tile_imps.add_improvement(key)?;
        }

        debug!("Added improvement {:?} to tile {} at {:?}", improvement_type, tile_id, tile.hex);
        Ok(key)
    }

    /// Remove improvement
    pub fn remove_improvement(&self, key: ImprovementKey) -> Result<Improvement, ImprovementError> {
        // Remove from main storage
        let improvement = {
            let mut improvements = self.improvements.write();
            improvements.remove(key).ok_or(ImprovementError::ImprovementNotFound { key })?
        };

        // Update indices
        self.update_indices_for_remove(key, &improvement);

        // Remove from tile improvements
        {
            let mut tile_improvements = self.tile_improvements.write();
            if let Some(tile_imps) = tile_improvements.get_mut(improvement.tile_id) {
                tile_imps.remove_improvement(key);
                
                // Clean up empty tile improvement collections
                if tile_imps.count() == 0 {
                    tile_improvements.remove(improvement.tile_id);
                }
            }
        }

        debug!("Removed improvement {:?} from tile {}", improvement.improvement_type, improvement.tile_id);
        Ok(improvement)
    }

    /// Get improvement by key
    pub fn get_improvement(&self, key: ImprovementKey) -> Option<Improvement> {
        self.improvements.read().get(key).cloned()
    }

    /// Get all improvements on a tile
    pub fn get_tile_improvements(&self, tile_id: TileId) -> Vec<Improvement> {
        let tile_improvements = self.tile_improvements.read();
        let improvements = self.improvements.read();
        
        if let Some(tile_imps) = tile_improvements.get(tile_id) {
            tile_imps.improvements()
                .iter()
                .filter_map(|&key| improvements.get(key).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get improvements by owner
    pub async fn get_player_improvements(&self, player_id: PlayerId) -> Vec<Improvement> {
        let cache_key = CacheKey::Custom(format!("player_improvements:{}", player_id));
        
        // Check cache first
        if let Ok(Some(improvements)) = self.cache.get::<Vec<Improvement>>(&cache_key).await {
            return improvements;
        }

        // Cache miss - compute improvements
        let improvements = {
            let owner_index = self.owner_index.read();
            let improvements_map = self.improvements.read();
            
            if let Some(keys) = owner_index.get(&player_id) {
                keys.iter()
                    .filter_map(|&key| improvements_map.get(key).cloned())
                    .collect()
            } else {
                Vec::new()
            }
        };

        // Cache the result
        let _ = self.cache.set(cache_key, improvements.clone(), CachePriority::Medium).await;
        improvements
    }

    /// Get improvements by type
    pub fn get_improvements_by_type(&self, improvement_type: ImprovementType) -> Vec<Improvement> {
        let type_index = self.type_index.read();
        let improvements = self.improvements.read();
        
        if let Some(keys) = type_index.get(&improvement_type) {
            keys.iter()
                .filter_map(|&key| improvements.get(key).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get improvement at specific hex coordinate
    pub fn get_improvement_at_hex(&self, hex: HexCoord, improvement_type: Option<ImprovementType>) -> Option<Improvement> {
        let spatial_index = self.spatial_index.read();
        let improvements = self.improvements.read();
        
        // This is simplified - in reality, multiple improvements could be at same hex
        if let Some(&key) = spatial_index.get(&hex) {
            if let Some(improvement) = improvements.get(key) {
                if improvement_type.map_or(true, |t| improvement.improvement_type == t) {
                    return Some(improvement.clone());
                }
            }
        }
        
        None
    }

    /// Process turn for all improvements
    #[instrument(skip(self))]
    pub fn process_turn(&self) -> ImprovementTurnResults {
        let mut results = ImprovementTurnResults::default();
        let mut to_remove = Vec::new();
        
        {
            let mut improvements = self.improvements.write();
            
            for (key, improvement) in improvements.iter_mut() {
                let completed = improvement.process_turn();
                
                match improvement.state {
                    ImprovementState::Completed if completed => {
                        results.completed_constructions.push(key);
                    }
                    ImprovementState::Demolishing { turns_remaining: 0 } => {
                        to_remove.push(key);
                    }
                    _ => {}
                }
            }
        }

        // Remove demolished improvements
        for key in to_remove {
            if let Ok(_) = self.remove_improvement(key) {
                results.demolished.push(key);
            }
        }

        results.total_processed = {
            let improvements = self.improvements.read();
            improvements.len()
        };

        debug!("Processed turn for {} improvements", results.total_processed);
        results
    }

    /// Start demolition of improvement
    pub fn demolish_improvement(&self, key: ImprovementKey) -> Result<(), ImprovementError> {
        let mut improvements = self.improvements.write();
        if let Some(improvement) = improvements.get_mut(key) {
            let demolition_time = improvement.improvement_type.construction_time() / 4; // Demolition is faster
            improvement.state = ImprovementState::Demolishing { turns_remaining: demolition_time.max(1) };
            Ok(())
        } else {
            Err(ImprovementError::ImprovementNotFound { key })
        }
    }

    /// Upgrade improvement
    pub fn upgrade_improvement(&self, key: ImprovementKey) -> Result<(), ImprovementError> {
        let mut improvements = self.improvements.write();
        if let Some(improvement) = improvements.get_mut(key) {
            if improvement.can_upgrade() {
                improvement.start_upgrade();
                Ok(())
            } else {
                Err(ImprovementError::CannotUpgrade { key })
            }
        } else {
            Err(ImprovementError::ImprovementNotFound { key })
        }
    }

    /// Get improvement statistics
    pub fn improvement_stats(&self) -> ImprovementStats {
        let improvements = self.improvements.read();
        let mut stats = ImprovementStats::default();
        
        stats.total_improvements = improvements.len();
        
        let mut type_counts = HashMap::new();
        let mut category_counts = HashMap::new();
        let mut owner_counts = HashMap::new();
        
        for improvement in improvements.values() {
            *type_counts.entry(improvement.improvement_type).or_insert(0) += 1;
            *category_counts.entry(improvement.improvement_type.category()).or_insert(0) += 1;
            
            if let Some(owner) = improvement.owner {
                *owner_counts.entry(owner).or_insert(0) += 1;
            }
            
            match improvement.state {
                ImprovementState::Completed => stats.completed += 1,
                ImprovementState::UnderConstruction { .. } => stats.under_construction += 1,
                ImprovementState::Damaged { .. } => stats.damaged += 1,
                ImprovementState::Upgrading { .. } => stats.upgrading += 1,
                ImprovementState::Demolishing { .. } => stats.demolishing += 1,
            }
        }
        
        stats.by_type = type_counts;
        stats.by_category = category_counts;
        stats.by_owner = owner_counts;
        
        stats
    }

    /// Get memory usage statistics
    pub fn memory_usage(&self) -> usize {
        let improvements = self.improvements.read();
        let base_size = std::mem::size_of::<Self>();
        let improvements_size = improvements.values().map(|imp| imp.memory_size()).sum::<usize>();
        let indices_size = self.spatial_index.read().len() * std::mem::size_of::<(HexCoord, ImprovementKey)>();
        
        base_size + improvements_size + indices_size
    }

    /// Update indices when adding improvement
    fn update_indices_for_add(&self, key: ImprovementKey, tile: &Tile, improvement_type: ImprovementType, owner: Option<PlayerId>) {
        // Update spatial index
        {
            let mut spatial = self.spatial_index.write();
            spatial.insert(tile.hex, key);
        }

        // Update owner index
        if let Some(owner) = owner {
            let mut owner_index = self.owner_index.write();
            owner_index.entry(owner).or_insert_with(Vec::new).push(key);
        }

        // Update type index
        {
            let mut type_index = self.type_index.write();
            type_index.entry(improvement_type).or_insert_with(Vec::new).push(key);
        }
    }

    /// Update indices when removing improvement
    fn update_indices_for_remove(&self, key: ImprovementKey, improvement: &Improvement) {
        // Remove from spatial index
        {
            let mut spatial = self.spatial_index.write();
            spatial.remove(&improvement.hex);
        }

        // Remove from owner index
        if let Some(owner) = improvement.owner {
            let mut owner_index = self.owner_index.write();
            if let Some(keys) = owner_index.get_mut(&owner) {
                keys.retain(|&k| k != key);
                if keys.is_empty() {
                    owner_index.remove(&owner);
                }
            }
        }

        // Remove from type index
        {
            let mut type_index = self.type_index.write();
            if let Some(keys) = type_index.get_mut(&improvement.improvement_type) {
                keys.retain(|&k| k != key);
                if keys.is_empty() {
                    type_index.remove(&improvement.improvement_type);
                }
            }
        }
    }
}

impl Default for TileImprovementManager {
    fn default() -> Self {
        let tile_manager = Arc::new(TileComponentManager::new());
        Self::new(tile_manager)
    }
}

/// Results from processing a turn for improvements
#[derive(Debug, Clone, Default)]
pub struct ImprovementTurnResults {
    pub total_processed: usize,
    pub completed_constructions: Vec<ImprovementKey>,
    pub demolished: Vec<ImprovementKey>,
}

/// Statistics for improvement monitoring
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImprovementStats {
    pub total_improvements: usize,
    pub completed: usize,
    pub under_construction: usize,
    pub damaged: usize,
    pub upgrading: usize,
    pub demolishing: usize,
    pub by_type: HashMap<ImprovementType, usize>,
    pub by_category: HashMap<ImprovementCategory, usize>,
    pub by_owner: HashMap<PlayerId, usize>,
}

/// Improvement system errors
#[derive(Debug, thiserror::Error)]
pub enum ImprovementError {
    #[error("Tile not found: {tile_id}")]
    TileNotFound { tile_id: TileId },
    
    #[error("Improvement not found: {key:?}")]
    ImprovementNotFound { key: ImprovementKey },
    
    #[error("Tile capacity exceeded - maximum {MAX_IMPROVEMENTS_PER_TILE} improvements per tile")]
    TileCapacityExceeded,
    
    #[error("Cannot build {improvement_type:?} on {terrain_type:?} terrain")]
    InvalidTerrain { improvement_type: ImprovementType, terrain_type: TerrainType },
    
    #[error("Cannot upgrade improvement {key:?}")]
    CannotUpgrade { key: ImprovementKey },
    
    #[error("Insufficient resources for improvement")]
    InsufficientResources,
}

/// System for processing improvement turns
pub fn process_improvements_system(
    mut improvement_manager: ResMut<TileImprovementManager>,
    // Would include turn/time resources
) {
    let results = improvement_manager.process_turn();
    
    if !results.completed_constructions.is_empty() {
        debug!("Completed construction of {} improvements", results.completed_constructions.len());
    }
    
    if !results.demolished.is_empty() {
        debug!("Demolished {} improvements", results.demolished.len());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_improvement_type_properties() {
        assert_eq!(ImprovementType::Farm.category(), ImprovementCategory::Resource);
        assert!(ImprovementType::Farm.base_cost() > 0);
        assert!(ImprovementType::Farm.construction_time() > 0);
        assert!(ImprovementType::Farm.can_build_on_terrain(TerrainType::Grassland));
        assert!(!ImprovementType::Mine.can_build_on_terrain(TerrainType::Ocean));
    }

    #[test]
    fn test_improvement_state() {
        let under_construction = ImprovementState::UnderConstruction { turns_remaining: 5 };
        let completed = ImprovementState::Completed;
        let damaged = ImprovementState::Damaged { repair_cost: 100 };
        
        assert!(!under_construction.is_functional());
        assert!(completed.is_functional());
        assert!(!damaged.is_functional());
        
        assert_eq!(completed.efficiency_factor(), 1.0);
        assert_eq!(under_construction.efficiency_factor(), 0.0);
        assert_eq!(damaged.efficiency_factor(), 0.5);
    }

    #[test]
    fn test_improvement_creation() {
        let hex = HexCoord { q: 10, r: 20 };
        let improvement = Improvement::new(ImprovementType::Farm, 123, hex, Some(1));
        
        assert_eq!(improvement.improvement_type, ImprovementType::Farm);
        assert_eq!(improvement.tile_id, 123);
        assert_eq!(improvement.hex, hex);
        assert_eq!(improvement.owner, Some(1));
        assert_eq!(improvement.level, 1);
        
        matches!(improvement.state, ImprovementState::UnderConstruction { .. });
    }

    #[test]
    fn test_tile_improvements_capacity() {
        let mut tile_improvements = TileImprovements::new(123);
        let improvements_map = SlotMap::with_key();
        
        // Should be able to add up to MAX_IMPROVEMENTS_PER_TILE
        for i in 0..MAX_IMPROVEMENTS_PER_TILE {
            let key = ImprovementKey::from(slotmap::KeyData::from_ffi(i as u64));
            assert!(tile_improvements.add_improvement(key).is_ok());
        }
        
        // Adding one more should fail
        let extra_key = ImprovementKey::from(slotmap::KeyData::from_ffi(MAX_IMPROVEMENTS_PER_TILE as u64));
        assert!(tile_improvements.add_improvement(extra_key).is_err());
    }

    #[test]
    fn test_improvement_upgrade() {
        let hex = HexCoord { q: 0, r: 0 };
        let mut improvement = Improvement::new(ImprovementType::Farm, 1, hex, None);
        
        // Complete construction first
        improvement.state = ImprovementState::Completed;
        
        assert!(improvement.can_upgrade());
        
        let initial_level = improvement.level;
        improvement.start_upgrade();
        
        assert_eq!(improvement.level, initial_level); // Level not changed until upgrade completes
        matches!(improvement.state, ImprovementState::Upgrading { .. });
    }

    #[test]
    fn test_improvement_manager() {
        let tile_manager = Arc::new(TileComponentManager::new());
        let manager = TileImprovementManager::new(tile_manager);
        
        // Create a test tile first (simplified)
        // In real usage, this would come from the tile component manager
        
        let stats = manager.improvement_stats();
        assert_eq!(stats.total_improvements, 0);
        
        let memory = manager.memory_usage();
        assert!(memory > 0);
    }
}
