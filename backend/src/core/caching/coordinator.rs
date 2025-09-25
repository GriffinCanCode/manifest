//! Cache coordination system for unified cache management
//!
//! The CacheCoordinator provides:
//! - Unified invalidation events across all cache layers
//! - Integration between spatial, query, and archetype caches
//! - Hierarchy traversal caching
//! - Save metadata caching
//! - Consolidated performance metrics

use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use tokio::sync::{RwLock, broadcast};
use bevy_ecs::prelude::Entity;
use glam::IVec2;
use serde::{Serialize, Deserialize};
use tracing::{info, debug, warn, error, instrument};

use crate::core::hashing::{FastHashMap, FastHashSet};
use super::{
    GameCache, SpatialCache, QueryCache, 
    CacheKey, CacheInvalidationEvent, CachePriority,
    SpatialCacheKey, QueryCacheKey, AICacheKey, PathfindingCacheKey,
    CacheMetrics, CacheStats
};

/// Central cache coordination system
pub struct CacheCoordinator {
    /// Core game cache (hot/warm layers)
    core_cache: Arc<GameCache>,
    /// Spatial query cache
    spatial_cache: Arc<RwLock<SpatialCache>>,
    /// ECS query result cache
    query_cache: Arc<RwLock<QueryCache>>,
    /// Archetype-specific cache integration
    archetype_cache: Arc<RwLock<ArchetypeCache>>,
    /// Hierarchy traversal cache
    hierarchy_cache: Arc<RwLock<HierarchyCache>>,
    /// Save metadata cache
    save_cache: Arc<RwLock<SaveMetadataCache>>,
    /// Invalidation event broadcaster
    invalidation_sender: broadcast::Sender<CoordinatedInvalidationEvent>,
    /// Unified metrics collector
    metrics: Arc<RwLock<UnifiedCacheMetrics>>,
    /// Current world state
    world_state: Arc<RwLock<WorldState>>,
}

/// World state tracking for cache invalidation
#[derive(Debug, Clone)]
struct WorldState {
    pub generation: u32,
    pub turn: u32,
    pub player_turn: u32,
    pub modified_entities: FastHashSet<Entity>,
    pub modified_positions: FastHashSet<IVec2>,
    pub modified_archetypes: FastHashSet<u64>,
}

impl Default for WorldState {
    fn default() -> Self {
        Self {
            generation: 1,
            turn: 1,
            player_turn: 1,
            modified_entities: FastHashSet::default(),
            modified_positions: FastHashSet::default(),
            modified_archetypes: FastHashSet::default(),
        }
    }
}

/// Coordinated invalidation events that affect multiple cache layers
#[derive(Debug, Clone)]
pub enum CoordinatedInvalidationEvent {
    /// World generation advanced - invalidate everything
    WorldGeneration(u32),
    /// Turn advanced - invalidate turn-dependent caches
    TurnAdvanced { old_turn: u32, new_turn: u32 },
    /// Player turn changed
    PlayerTurnChanged { player_id: u32 },
    /// Entity modified - cascade through all relevant caches
    EntityModified { 
        entity: Entity, 
        archetype_changed: bool,
        position_changed: Option<IVec2>,
        components_changed: Vec<std::any::TypeId>,
    },
    /// Archetype structure changed
    ArchetypeChanged { 
        archetype_id: u64,
        entities_affected: Vec<Entity>,
    },
    /// Spatial region modified
    SpatialRegion { 
        center: IVec2, 
        radius: u32,
        modification_type: SpatialModification,
    },
    /// Player state changed
    PlayerStateChanged { 
        player_id: u32, 
        state_type: PlayerStateType,
    },
    /// Save file operations
    SaveOperation { 
        operation_type: SaveOperationType,
        save_id: Option<String>,
    },
    /// Manual invalidation with filter
    Manual { 
        description: String,
        cache_types: Vec<CacheType>,
    },
}

#[derive(Debug, Clone)]
pub enum SpatialModification {
    TerrainChanged,
    EntityAdded,
    EntityRemoved,
    EntityMoved,
    VisibilityChanged,
}

#[derive(Debug, Clone)]
pub enum PlayerStateType {
    Resources,
    Technology,
    Diplomacy,
    Territory,
    Military,
}

#[derive(Debug, Clone)]
pub enum SaveOperationType {
    Load,
    Save,
    Delete,
    Metadata,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum CacheType {
    Core,
    Spatial,
    Query,
    Archetype,
    Hierarchy,
    Save,
    All,
}

impl CacheCoordinator {
    /// Create a new cache coordinator with all subsystems
    pub async fn new() -> Self {
        let core_cache = Arc::new(GameCache::new(super::CacheConfig::default()));
        let spatial_cache = Arc::new(RwLock::new(SpatialCache::new()));
        let query_cache = Arc::new(RwLock::new(QueryCache::new()));
        let archetype_cache = Arc::new(RwLock::new(ArchetypeCache::new()));
        let hierarchy_cache = Arc::new(RwLock::new(HierarchyCache::new()));
        let save_cache = Arc::new(RwLock::new(SaveMetadataCache::new()));
        let metrics = Arc::new(RwLock::new(UnifiedCacheMetrics::new()));
        let world_state = Arc::new(RwLock::new(WorldState::default()));
        
        let (invalidation_sender, _) = broadcast::channel(1000);

        let coordinator = Self {
            core_cache,
            spatial_cache,
            query_cache,
            archetype_cache,
            hierarchy_cache,
            save_cache,
            invalidation_sender,
            metrics,
            world_state,
        };

        coordinator.start_background_tasks().await;
        coordinator
    }

    /// Get the core cache for direct access
    pub fn core_cache(&self) -> &Arc<GameCache> {
        &self.core_cache
    }

    /// Get the spatial cache
    pub fn spatial_cache(&self) -> &Arc<RwLock<SpatialCache>> {
        &self.spatial_cache
    }

    /// Get the query cache
    pub fn query_cache(&self) -> &Arc<RwLock<QueryCache>> {
        &self.query_cache
    }

    /// Subscribe to invalidation events
    pub fn subscribe_invalidation(&self) -> broadcast::Receiver<CoordinatedInvalidationEvent> {
        self.invalidation_sender.subscribe()
    }

    /// Advance world generation - invalidates all caches
    #[instrument(name = "advance_world_generation", skip(self))]
    pub async fn advance_world_generation(&self) -> u32 {
        let new_generation = {
            let mut world_state = self.world_state.write().await;
            world_state.generation += 1;
            world_state.modified_entities.clear();
            world_state.modified_positions.clear();
            world_state.modified_archetypes.clear();
            world_state.generation
        };

        // Clear all caches
        self.core_cache.clear().await;
        self.spatial_cache.write().await.advance_world_generation();
        self.query_cache.write().await.advance_world_generation();
        self.archetype_cache.write().await.clear();
        self.hierarchy_cache.write().await.clear();
        
        // Don't clear save cache as it persists across generations

        // Broadcast invalidation event
        let _ = self.invalidation_sender.send(
            CoordinatedInvalidationEvent::WorldGeneration(new_generation)
        );

        info!(
            target: "cache_coordinator",
            generation = new_generation,
            "Advanced world generation, all caches invalidated"
        );

        new_generation
    }

    /// Advance game turn
    #[instrument(name = "advance_turn", skip(self))]
    pub async fn advance_turn(&self, new_turn: u32) {
        let old_turn = {
            let mut world_state = self.world_state.write().await;
            let old_turn = world_state.turn;
            world_state.turn = new_turn;
            old_turn
        };

        // Invalidate turn-dependent caches
        self.invalidate_turn_caches(old_turn, new_turn).await;

        // Broadcast event
        let _ = self.invalidation_sender.send(
            CoordinatedInvalidationEvent::TurnAdvanced { old_turn, new_turn }
        );

        debug!(
            target: "cache_coordinator",
            old_turn = old_turn,
            new_turn = new_turn,
            "Advanced game turn"
        );
    }

    /// Handle entity modification with cascading invalidation
    #[instrument(name = "entity_modified", skip(self))]
    pub async fn entity_modified(
        &self, 
        entity: Entity, 
        archetype_changed: bool,
        position_changed: Option<IVec2>,
        components_changed: Vec<std::any::TypeId>
    ) {
        // Track in world state
        {
            let mut world_state = self.world_state.write().await;
            world_state.modified_entities.insert(entity);
            if let Some(pos) = position_changed {
                world_state.modified_positions.insert(pos);
            }
        }

        // Invalidate relevant caches
        if archetype_changed {
            self.query_cache.write().await.invalidate_component_type(std::any::TypeId::of::<()>());
            self.archetype_cache.write().await.invalidate_entity(entity);
        }

        if let Some(position) = position_changed {
            self.spatial_cache.write().await.invalidate_position(position);
        }

        // Invalidate hierarchy if entity relationships might have changed
        if archetype_changed {
            self.hierarchy_cache.write().await.invalidate_entity(entity);
        }

        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.record_invalidation("entity_modified", 1);
        }

        // Broadcast event
        let _ = self.invalidation_sender.send(
            CoordinatedInvalidationEvent::EntityModified {
                entity,
                archetype_changed,
                position_changed,
                components_changed,
            }
        );
    }

    /// Handle archetype changes
    #[instrument(name = "archetype_changed", skip(self))]
    pub async fn archetype_changed(&self, archetype_id: u64, entities_affected: Vec<Entity>) {
        // Track in world state
        {
            let mut world_state = self.world_state.write().await;
            world_state.modified_archetypes.insert(archetype_id);
            for entity in &entities_affected {
                world_state.modified_entities.insert(*entity);
            }
        }

        // Invalidate archetype cache
        self.archetype_cache.write().await.invalidate_archetype(archetype_id);
        
        // Invalidate query cache
        self.query_cache.write().await.invalidate_archetype(archetype_id);

        // Invalidate hierarchy if relationships changed
        for entity in &entities_affected {
            self.hierarchy_cache.write().await.invalidate_entity(*entity);
        }

        // Broadcast event
        let _ = self.invalidation_sender.send(
            CoordinatedInvalidationEvent::ArchetypeChanged { archetype_id, entities_affected }
        );
    }

    /// Handle spatial region modifications
    #[instrument(name = "spatial_region_modified", skip(self))]
    pub async fn spatial_region_modified(
        &self, 
        center: IVec2, 
        radius: u32,
        modification_type: SpatialModification
    ) {
        // Invalidate spatial cache for the region
        self.spatial_cache.write().await.invalidate_position(center);
        
        // For larger radius, invalidate broader area
        if radius > 1 {
            for dx in -(radius as i32)..=(radius as i32) {
                for dy in -(radius as i32)..=(radius as i32) {
                    let pos = IVec2::new(center.x + dx, center.y + dy);
                    if pos.distance_squared(center) <= (radius * radius) as i32 {
                        self.spatial_cache.write().await.invalidate_position(pos);
                    }
                }
            }
        }

        // Update world state
        {
            let mut world_state = self.world_state.write().await;
            world_state.modified_positions.insert(center);
        }

        // Broadcast event
        let _ = self.invalidation_sender.send(
            CoordinatedInvalidationEvent::SpatialRegion { 
                center, 
                radius, 
                modification_type,
            }
        );
    }

    /// Handle player state changes
    #[instrument(name = "player_state_changed", skip(self))]
    pub async fn player_state_changed(&self, player_id: u32, state_type: PlayerStateType) {
        // Invalidate player-specific caches
        self.query_cache.write().await.invalidate_player(player_id);
        self.spatial_cache.write().await.invalidate_player(player_id);

        // Invalidate core cache entries for this player
        let invalidation_event = CacheInvalidationEvent::PlayerChanged(player_id);
        self.core_cache.handle_invalidation(&invalidation_event).await;

        // Broadcast event
        let _ = self.invalidation_sender.send(
            CoordinatedInvalidationEvent::PlayerStateChanged { player_id, state_type }
        );
    }

    /// Get unified performance metrics
    pub async fn get_metrics(&self) -> UnifiedCacheMetrics {
        let mut unified_metrics = self.metrics.write().await;
        
        // Collect metrics from all subsystems
        let core_stats = self.core_cache.stats().await;
        let spatial_stats = self.spatial_cache.read().await.stats();
        let query_stats = self.query_cache.read().await.stats();
        let archetype_stats = self.archetype_cache.read().await.stats();
        let hierarchy_stats = self.hierarchy_cache.read().await.stats();
        let save_stats = self.save_cache.read().await.stats();

        // Update unified metrics
        unified_metrics.update_subsystem_metrics(CacheType::Core, core_stats.into());
        unified_metrics.update_subsystem_metrics(CacheType::Spatial, spatial_stats.into());
        unified_metrics.update_subsystem_metrics(CacheType::Query, query_stats.into());
        unified_metrics.update_subsystem_metrics(CacheType::Archetype, archetype_stats.into());
        unified_metrics.update_subsystem_metrics(CacheType::Hierarchy, hierarchy_stats.into());
        unified_metrics.update_subsystem_metrics(CacheType::Save, save_stats.into());

        unified_metrics.clone()
    }

    /// Perform coordinated maintenance across all caches
    #[instrument(name = "maintain_caches", skip(self))]
    pub async fn maintain_caches(&self) {
        let start_time = Instant::now();

        // Maintain core cache
        self.core_cache.maintain().await;

        // Maintain spatial cache
        self.spatial_cache.write().await.cleanup(Duration::from_secs(300));

        // Maintain query cache
        self.query_cache.write().await.cleanup_by_value(1000);

        // Maintain archetype cache
        self.archetype_cache.write().await.cleanup();

        // Maintain hierarchy cache
        self.hierarchy_cache.write().await.cleanup();

        // Maintain save cache
        self.save_cache.write().await.cleanup();

        let maintenance_time = start_time.elapsed();
        
        debug!(
            target: "cache_coordinator",
            maintenance_time_ms = maintenance_time.as_millis(),
            "Completed coordinated cache maintenance"
        );

        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.record_maintenance(maintenance_time);
        }
    }

    /// Invalidate turn-dependent caches
    async fn invalidate_turn_caches(&self, _old_turn: u32, _new_turn: u32) {
        // Invalidate AI and pathfinding caches (turn-dependent)
        let invalidation_event = CacheInvalidationEvent::TurnAdvanced(_new_turn);
        self.core_cache.handle_invalidation(&invalidation_event).await;
        
        // Clear archetype cache entries that are turn-dependent
        self.archetype_cache.write().await.invalidate_turn_dependent();
    }

    /// Start background maintenance tasks
    async fn start_background_tasks(&self) {
        let coordinator_weak = Arc::downgrade(&Arc::new(self.clone()));
        
        // Spawn maintenance task
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                if let Some(coordinator) = coordinator_weak.upgrade() {
                    coordinator.maintain_caches().await;
                } else {
                    break; // Coordinator was dropped
                }
            }
        });
    }
}

// Implement Clone for background tasks (using Arc clones)
impl Clone for CacheCoordinator {
    fn clone(&self) -> Self {
        Self {
            core_cache: Arc::clone(&self.core_cache),
            spatial_cache: Arc::clone(&self.spatial_cache),
            query_cache: Arc::clone(&self.query_cache),
            archetype_cache: Arc::clone(&self.archetype_cache),
            hierarchy_cache: Arc::clone(&self.hierarchy_cache),
            save_cache: Arc::clone(&self.save_cache),
            invalidation_sender: self.invalidation_sender.clone(),
            metrics: Arc::clone(&self.metrics),
            world_state: Arc::clone(&self.world_state),
        }
    }
}

/// Archetype-specific cache for component queries
pub struct ArchetypeCache {
    cache: FastHashMap<u64, ArchetypeCacheEntry>,
    entity_to_archetype: FastHashMap<Entity, u64>,
}

#[derive(Debug, Clone)]
pub struct ArchetypeCacheEntry {
    pub archetype_id: u64,
    pub entities: Vec<Entity>,
    pub component_signature: u64,
    pub created_at: Instant,
    pub access_count: u32,
}

impl ArchetypeCache {
    pub fn new() -> Self {
        Self {
            cache: FastHashMap::default(),
            entity_to_archetype: FastHashMap::default(),
        }
    }

    pub fn get_entities(&mut self, archetype_id: u64) -> Option<Vec<Entity>> {
        if let Some(entry) = self.cache.get_mut(&archetype_id) {
            entry.access_count += 1;
            Some(entry.entities.clone())
        } else {
            None
        }
    }

    pub fn set_entities(&mut self, archetype_id: u64, entities: Vec<Entity>, component_signature: u64) {
        // Update entity mappings
        for entity in &entities {
            self.entity_to_archetype.insert(*entity, archetype_id);
        }

        let entry = ArchetypeCacheEntry {
            archetype_id,
            entities,
            component_signature,
            created_at: Instant::now(),
            access_count: 0,
        };
        
        self.cache.insert(archetype_id, entry);
    }

    pub fn invalidate_archetype(&mut self, archetype_id: u64) {
        if let Some(entry) = self.cache.remove(&archetype_id) {
            // Remove entity mappings
            for entity in &entry.entities {
                self.entity_to_archetype.remove(entity);
            }
        }
    }

    pub fn invalidate_entity(&mut self, entity: Entity) {
        if let Some(&archetype_id) = self.entity_to_archetype.get(&entity) {
            self.invalidate_archetype(archetype_id);
        }
    }

    pub fn invalidate_turn_dependent(&mut self) {
        // Clear all entries for turn-based invalidation
        self.cache.clear();
        self.entity_to_archetype.clear();
    }

    pub fn clear(&mut self) {
        self.cache.clear();
        self.entity_to_archetype.clear();
    }

    pub fn cleanup(&mut self) {
        let cutoff = Instant::now() - Duration::from_secs(300);
        self.cache.retain(|_, entry| entry.created_at > cutoff);
    }

    pub fn stats(&self) -> ArchetypeCacheStats {
        ArchetypeCacheStats {
            entry_count: self.cache.len(),
            total_entities: self.entity_to_archetype.len(),
            avg_access_count: if !self.cache.is_empty() {
                self.cache.values().map(|e| e.access_count as f64).sum::<f64>() / self.cache.len() as f64
            } else {
                0.0
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct ArchetypeCacheStats {
    pub entry_count: usize,
    pub total_entities: usize,
    pub avg_access_count: f64,
}

/// Hierarchy traversal cache for parent/child relationships
pub struct HierarchyCache {
    cache: FastHashMap<u64, HierarchyCacheEntry>,
}

#[derive(Debug, Clone)]
pub struct HierarchyCacheEntry {
    pub key: HierarchyKey,
    pub result: HierarchyResult,
    pub created_at: Instant,
    pub access_count: u32,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum HierarchyKey {
    Children(Entity),
    Parent(Entity),
    Descendants(Entity, u8), // Entity, max depth
    Ancestors(Entity, u8),
    Siblings(Entity),
}

#[derive(Debug, Clone)]
pub enum HierarchyResult {
    Entities(Vec<Entity>),
    Relationships(Vec<(Entity, Entity)>), // (parent, child) pairs
}

impl HierarchyCache {
    pub fn new() -> Self {
        Self {
            cache: FastHashMap::default(),
        }
    }

    pub fn get(&mut self, key: &HierarchyKey) -> Option<HierarchyResult> {
        use crate::core::hashing::FastHasher;
        let key_hash = FastHasher::hash_one(key);
        
        if let Some(entry) = self.cache.get_mut(&key_hash) {
            entry.access_count += 1;
            Some(entry.result.clone())
        } else {
            None
        }
    }

    pub fn set(&mut self, key: HierarchyKey, result: HierarchyResult) {
        use crate::core::hashing::FastHasher;
        let key_hash = FastHasher::hash_one(&key);
        
        let entry = HierarchyCacheEntry {
            key,
            result,
            created_at: Instant::now(),
            access_count: 0,
        };
        
        self.cache.insert(key_hash, entry);
    }

    pub fn invalidate_entity(&mut self, entity: Entity) {
        // Remove all entries involving this entity
        self.cache.retain(|_, entry| {
            !matches!(
                &entry.key,
                HierarchyKey::Children(e) | 
                HierarchyKey::Parent(e) | 
                HierarchyKey::Descendants(e, _) | 
                HierarchyKey::Ancestors(e, _) | 
                HierarchyKey::Siblings(e) 
                if *e == entity
            )
        });
    }

    pub fn clear(&mut self) {
        self.cache.clear();
    }

    pub fn cleanup(&mut self) {
        let cutoff = Instant::now() - Duration::from_secs(180);
        self.cache.retain(|_, entry| entry.created_at > cutoff);
    }

    pub fn stats(&self) -> HierarchyCacheStats {
        HierarchyCacheStats {
            entry_count: self.cache.len(),
            avg_access_count: if !self.cache.is_empty() {
                self.cache.values().map(|e| e.access_count as f64).sum::<f64>() / self.cache.len() as f64
            } else {
                0.0
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct HierarchyCacheStats {
    pub entry_count: usize,
    pub avg_access_count: f64,
}

/// Save file metadata cache
pub struct SaveMetadataCache {
    cache: FastHashMap<String, SaveMetadataEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveMetadataEntry {
    pub save_id: String,
    pub metadata: SaveMetadata,
    pub cached_at: u64, // Unix timestamp
    pub access_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveMetadata {
    pub turn: u32,
    pub player_count: u8,
    pub world_size: (u32, u32),
    pub difficulty: String,
    pub created_at: u64,
    pub last_played: u64,
    pub playtime_seconds: u64,
    pub version: String,
    pub checksum: u64,
}

impl SaveMetadataCache {
    pub fn new() -> Self {
        Self {
            cache: FastHashMap::default(),
        }
    }

    pub fn get(&mut self, save_id: &str) -> Option<SaveMetadata> {
        if let Some(entry) = self.cache.get_mut(save_id) {
            entry.access_count += 1;
            Some(entry.metadata.clone())
        } else {
            None
        }
    }

    pub fn set(&mut self, save_id: String, metadata: SaveMetadata) {
        let entry = SaveMetadataEntry {
            save_id: save_id.clone(),
            metadata,
            cached_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            access_count: 0,
        };
        
        self.cache.insert(save_id, entry);
    }

    pub fn remove(&mut self, save_id: &str) -> bool {
        self.cache.remove(save_id).is_some()
    }

    pub fn clear(&mut self) {
        self.cache.clear();
    }

    pub fn cleanup(&mut self) {
        let cutoff_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() - 3600; // 1 hour

        self.cache.retain(|_, entry| entry.cached_at > cutoff_time);
    }

    pub fn stats(&self) -> SaveCacheStats {
        SaveCacheStats {
            entry_count: self.cache.len(),
            total_size_bytes: self.cache.values()
                .map(|e| std::mem::size_of::<SaveMetadataEntry>())
                .sum(),
            avg_access_count: if !self.cache.is_empty() {
                self.cache.values().map(|e| e.access_count as f64).sum::<f64>() / self.cache.len() as f64
            } else {
                0.0
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct SaveCacheStats {
    pub entry_count: usize,
    pub total_size_bytes: usize,
    pub avg_access_count: f64,
}

/// Unified metrics across all cache subsystems
#[derive(Debug, Clone)]
pub struct UnifiedCacheMetrics {
    pub subsystem_metrics: FastHashMap<CacheType, SubsystemMetrics>,
    pub global_metrics: GlobalCacheMetrics,
    pub last_updated: Instant,
}

#[derive(Debug, Clone)]
pub struct SubsystemMetrics {
    pub hit_ratio: f64,
    pub entry_count: usize,
    pub memory_usage_bytes: u64,
    pub avg_access_time_micros: f64,
    pub invalidation_count: u64,
}

#[derive(Debug, Clone, Default)]
pub struct GlobalCacheMetrics {
    pub total_hits: u64,
    pub total_misses: u64,
    pub total_invalidations: u64,
    pub total_memory_bytes: u64,
    pub maintenance_count: u64,
    pub last_maintenance: Option<Instant>,
}

impl UnifiedCacheMetrics {
    pub fn new() -> Self {
        Self {
            subsystem_metrics: FastHashMap::default(),
            global_metrics: GlobalCacheMetrics::default(),
            last_updated: Instant::now(),
        }
    }

    pub fn update_subsystem_metrics(&mut self, cache_type: CacheType, metrics: SubsystemMetrics) {
        self.subsystem_metrics.insert(cache_type, metrics);
        self.last_updated = Instant::now();
    }

    pub fn record_invalidation(&mut self, _reason: &str, count: u64) {
        self.global_metrics.total_invalidations += count;
    }

    pub fn record_maintenance(&mut self, _duration: Duration) {
        self.global_metrics.maintenance_count += 1;
        self.global_metrics.last_maintenance = Some(Instant::now());
    }

    pub fn overall_hit_ratio(&self) -> f64 {
        let total_hits: f64 = self.subsystem_metrics.values()
            .map(|m| m.hit_ratio * m.entry_count as f64)
            .sum();
        let total_entries: usize = self.subsystem_metrics.values()
            .map(|m| m.entry_count)
            .sum();
        
        if total_entries > 0 {
            total_hits / total_entries as f64
        } else {
            0.0
        }
    }

    pub fn total_memory_usage(&self) -> u64 {
        self.subsystem_metrics.values()
            .map(|m| m.memory_usage_bytes)
            .sum()
    }
}

// Conversion implementations for subsystem stats
impl From<CacheStats> for SubsystemMetrics {
    fn from(stats: CacheStats) -> Self {
        Self {
            hit_ratio: stats.hit_ratio,
            entry_count: stats.cache_count,
            memory_usage_bytes: stats.memory_usage_bytes,
            avg_access_time_micros: stats.avg_access_time_micros,
            invalidation_count: stats.total_evictions,
        }
    }
}

impl From<super::SpatialCacheStats> for SubsystemMetrics {
    fn from(stats: super::SpatialCacheStats) -> Self {
        Self {
            hit_ratio: 0.0, // Would need to track this in spatial cache
            entry_count: stats.entry_count,
            memory_usage_bytes: stats.total_size_bytes as u64,
            avg_access_time_micros: 0.0, // Would need to track this
            invalidation_count: 0, // Would need to track this
        }
    }
}

impl From<super::QueryCacheStats> for SubsystemMetrics {
    fn from(stats: super::QueryCacheStats) -> Self {
        Self {
            hit_ratio: 0.0, // Would need to track this in query cache
            entry_count: stats.entry_count,
            memory_usage_bytes: stats.total_size_bytes as u64,
            avg_access_time_micros: 0.0, // Would need to track this
            invalidation_count: 0, // Would need to track this
        }
    }
}

impl From<ArchetypeCacheStats> for SubsystemMetrics {
    fn from(stats: ArchetypeCacheStats) -> Self {
        Self {
            hit_ratio: 0.0,
            entry_count: stats.entry_count,
            memory_usage_bytes: (stats.entry_count * 64) as u64, // Estimate
            avg_access_time_micros: 0.0,
            invalidation_count: 0,
        }
    }
}

impl From<HierarchyCacheStats> for SubsystemMetrics {
    fn from(stats: HierarchyCacheStats) -> Self {
        Self {
            hit_ratio: 0.0,
            entry_count: stats.entry_count,
            memory_usage_bytes: (stats.entry_count * 128) as u64, // Estimate
            avg_access_time_micros: 0.0,
            invalidation_count: 0,
        }
    }
}

impl From<SaveCacheStats> for SubsystemMetrics {
    fn from(stats: SaveCacheStats) -> Self {
        Self {
            hit_ratio: 0.0,
            entry_count: stats.entry_count,
            memory_usage_bytes: stats.total_size_bytes as u64,
            avg_access_time_micros: 0.0,
            invalidation_count: 0,
        }
    }
}
