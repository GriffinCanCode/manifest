//! Cache coordination events and lightweight management
//!
//! Simple event-driven cache coordination without replacing existing systems.
//! Provides invalidation coordination and unified metrics collection.

use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
use std::time::{Instant, Duration};
use tokio::sync::{broadcast, RwLock};
use bevy_ecs::prelude::Entity;
use glam::IVec2;
use tracing::{debug, warn, instrument};

use crate::core::hashing::FastHashMap;
use super::{CacheStats, CachePriority};

/// Lightweight cache coordination events
#[derive(Debug, Clone)]
pub enum CacheInvalidationEvent {
    /// World generation advanced - invalidate all caches
    WorldGeneration(u32),
    /// Turn advanced - invalidate turn-dependent caches
    TurnAdvanced { old_turn: u32, new_turn: u32 },
    /// Entity modified - cascade through relevant caches
    EntityModified { 
        entity: Entity, 
        archetype_changed: bool,
        position_changed: Option<IVec2>,
    },
    /// Archetype changed - invalidate query caches
    ArchetypeChanged { archetype_id: u64 },
    /// Spatial region modified - invalidate spatial caches
    SpatialRegion { center: IVec2, radius: u32 },
    /// Player state changed - invalidate player caches
    PlayerStateChanged { player_id: u32 },
    /// Save operation - invalidate save metadata cache
    SaveOperation { save_name: String },
}

/// Lightweight cache coordination service
pub struct CacheEventBus {
    /// Event broadcast channel
    sender: broadcast::Sender<CacheInvalidationEvent>,
    /// Metrics collector
    metrics: Arc<RwLock<UnifiedMetrics>>,
}

impl CacheEventBus {
    /// Create new cache event bus
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1000);
        let metrics = Arc::new(RwLock::new(UnifiedMetrics::new()));
        
        Self {
            sender,
            metrics,
        }
    }
    
    /// Subscribe to cache invalidation events
    pub fn subscribe(&self) -> broadcast::Receiver<CacheInvalidationEvent> {
        self.sender.subscribe()
    }
    
    /// Broadcast an invalidation event
    #[instrument(name = "cache_event", skip(self), fields(event_type = std::any::type_name::<CacheInvalidationEvent>()))]
    pub async fn broadcast(&self, event: CacheInvalidationEvent) {
        // Record metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.record_invalidation_event(&event);
        }
        
        // Broadcast event
        if let Err(e) = self.sender.send(event) {
            warn!("Failed to broadcast cache invalidation event: {}", e);
        }
    }
    
    /// Get unified metrics across all cache subsystems
    pub async fn metrics(&self) -> UnifiedMetrics {
        self.metrics.read().await.clone()
    }
    
    /// Register cache subsystem metrics
    pub async fn register_subsystem_metrics(&self, subsystem: &str, stats: SubsystemStats) {
        let mut metrics = self.metrics.write().await;
        metrics.subsystems.insert(subsystem.to_string(), stats);
    }
}

impl Default for CacheEventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Unified metrics across all cache subsystems
#[derive(Debug, Clone)]
pub struct UnifiedMetrics {
    /// Per-subsystem metrics
    pub subsystems: FastHashMap<String, SubsystemStats>,
    /// Global counters
    pub global_stats: GlobalStats,
    /// Last update time
    pub last_updated: Instant,
}

impl UnifiedMetrics {
    pub fn new() -> Self {
        Self {
            subsystems: FastHashMap::default(),
            global_stats: GlobalStats::new(),
            last_updated: Instant::now(),
        }
    }
    
    /// Record an invalidation event
    pub fn record_invalidation_event(&mut self, event: &CacheInvalidationEvent) {
        self.global_stats.total_invalidations.fetch_add(1, Ordering::Relaxed);
        
        match event {
            CacheInvalidationEvent::WorldGeneration(_) => {
                self.global_stats.world_generation_invalidations.fetch_add(1, Ordering::Relaxed);
            }
            CacheInvalidationEvent::TurnAdvanced { .. } => {
                self.global_stats.turn_invalidations.fetch_add(1, Ordering::Relaxed);
            }
            CacheInvalidationEvent::EntityModified { .. } => {
                self.global_stats.entity_invalidations.fetch_add(1, Ordering::Relaxed);
            }
            CacheInvalidationEvent::ArchetypeChanged { .. } => {
                self.global_stats.archetype_invalidations.fetch_add(1, Ordering::Relaxed);
            }
            CacheInvalidationEvent::SpatialRegion { .. } => {
                self.global_stats.spatial_invalidations.fetch_add(1, Ordering::Relaxed);
            }
            CacheInvalidationEvent::PlayerStateChanged { .. } => {
                self.global_stats.player_invalidations.fetch_add(1, Ordering::Relaxed);
            }
            CacheInvalidationEvent::SaveOperation { .. } => {
                self.global_stats.save_invalidations.fetch_add(1, Ordering::Relaxed);
            }
        }
        
        self.last_updated = Instant::now();
    }
    
    /// Calculate overall hit ratio across all subsystems
    pub fn overall_hit_ratio(&self) -> f64 {
        let mut total_hits = 0.0;
        let mut total_accesses = 0.0;
        
        for stats in self.subsystems.values() {
            total_hits += stats.hits as f64;
            total_accesses += (stats.hits + stats.misses) as f64;
        }
        
        if total_accesses > 0.0 {
            total_hits / total_accesses
        } else {
            0.0
        }
    }
    
    /// Calculate total memory usage across all subsystems
    pub fn total_memory_usage(&self) -> u64 {
        self.subsystems.values()
            .map(|stats| stats.memory_usage_bytes)
            .sum()
    }
    
    /// Get invalidation rate (invalidations per second over last minute)
    pub fn invalidation_rate(&self) -> f64 {
        let total_invalidations = self.global_stats.total_invalidations.load(Ordering::Relaxed);
        let elapsed_secs = self.last_updated.elapsed().as_secs_f64().max(1.0);
        total_invalidations as f64 / elapsed_secs
    }
}

/// Global cache statistics
#[derive(Debug, Default)]
pub struct GlobalStats {
    pub total_invalidations: AtomicU64,
    pub world_generation_invalidations: AtomicU64,
    pub turn_invalidations: AtomicU64,
    pub entity_invalidations: AtomicU64,
    pub archetype_invalidations: AtomicU64,
    pub spatial_invalidations: AtomicU64,
    pub player_invalidations: AtomicU64,
    pub save_invalidations: AtomicU64,
}

impl GlobalStats {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Clone for GlobalStats {
    fn clone(&self) -> Self {
        Self {
            total_invalidations: AtomicU64::new(self.total_invalidations.load(Ordering::Relaxed)),
            world_generation_invalidations: AtomicU64::new(self.world_generation_invalidations.load(Ordering::Relaxed)),
            turn_invalidations: AtomicU64::new(self.turn_invalidations.load(Ordering::Relaxed)),
            entity_invalidations: AtomicU64::new(self.entity_invalidations.load(Ordering::Relaxed)),
            archetype_invalidations: AtomicU64::new(self.archetype_invalidations.load(Ordering::Relaxed)),
            spatial_invalidations: AtomicU64::new(self.spatial_invalidations.load(Ordering::Relaxed)),
            player_invalidations: AtomicU64::new(self.player_invalidations.load(Ordering::Relaxed)),
            save_invalidations: AtomicU64::new(self.save_invalidations.load(Ordering::Relaxed)),
        }
    }
}

/// Per-subsystem cache statistics
#[derive(Debug, Clone)]
pub struct SubsystemStats {
    pub hits: u64,
    pub misses: u64,
    pub entries: usize,
    pub memory_usage_bytes: u64,
    pub avg_access_time_micros: f64,
    pub last_updated: Instant,
}

impl SubsystemStats {
    pub fn new() -> Self {
        Self {
            hits: 0,
            misses: 0,
            entries: 0,
            memory_usage_bytes: 0,
            avg_access_time_micros: 0.0,
            last_updated: Instant::now(),
        }
    }
    
    pub fn hit_ratio(&self) -> f64 {
        let total = self.hits + self.misses;
        if total > 0 {
            self.hits as f64 / total as f64
        } else {
            0.0
        }
    }
}

impl Default for SubsystemStats {
    fn default() -> Self {
        Self::new()
    }
}

// Conversion implementations
impl From<CacheStats> for SubsystemStats {
    fn from(stats: CacheStats) -> Self {
        Self {
            hits: stats.total_hits,
            misses: stats.total_misses,
            entries: stats.cache_count,
            memory_usage_bytes: stats.memory_usage_bytes,
            avg_access_time_micros: stats.avg_access_time_micros,
            last_updated: Instant::now(),
        }
    }
}

/// Global cache event bus instance
static CACHE_EVENT_BUS: once_cell::sync::Lazy<CacheEventBus> = 
    once_cell::sync::Lazy::new(CacheEventBus::new);

/// Get the global cache event bus
pub fn global_cache_events() -> &'static CacheEventBus {
    &CACHE_EVENT_BUS
}

/// Convenience function to broadcast cache invalidation
pub async fn broadcast_cache_invalidation(event: CacheInvalidationEvent) {
    global_cache_events().broadcast(event).await;
}

/// Macro for easy cache invalidation
#[macro_export]
macro_rules! invalidate_cache {
    (world_generation: $gen:expr) => {
        $crate::core::caching::events::broadcast_cache_invalidation(
            $crate::core::caching::events::CacheInvalidationEvent::WorldGeneration($gen)
        )
    };
    (turn_advanced: $old:expr => $new:expr) => {
        $crate::core::caching::events::broadcast_cache_invalidation(
            $crate::core::caching::events::CacheInvalidationEvent::TurnAdvanced { 
                old_turn: $old, 
                new_turn: $new 
            }
        )
    };
    (entity_modified: $entity:expr) => {
        $crate::core::caching::events::broadcast_cache_invalidation(
            $crate::core::caching::events::CacheInvalidationEvent::EntityModified { 
                entity: $entity, 
                archetype_changed: false,
                position_changed: None,
            }
        )
    };
    (entity_modified: $entity:expr, archetype: $arch:expr, position: $pos:expr) => {
        $crate::core::caching::events::broadcast_cache_invalidation(
            $crate::core::caching::events::CacheInvalidationEvent::EntityModified { 
                entity: $entity, 
                archetype_changed: $arch,
                position_changed: $pos,
            }
        )
    };
}

/// Trait for subsystems to implement cache event handling
#[async_trait::async_trait]
pub trait CacheEventHandler {
    /// Handle a cache invalidation event
    async fn handle_invalidation(&mut self, event: &CacheInvalidationEvent);
    
    /// Get current cache statistics
    fn cache_stats(&self) -> SubsystemStats;
    
    /// Get subsystem name for metrics
    fn subsystem_name(&self) -> &'static str;
}
