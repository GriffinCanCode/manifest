//! Core cache implementation using moka for high-performance concurrent caching
//!
//! Provides the main cache infrastructure with support for:
//! - Concurrent access with minimal lock contention
//! - TTL-based expiration and custom eviction policies
//! - Memory-bounded caching with automatic cleanup
//! - Integration with game's hashing strategies

use std::sync::Arc;
use std::time::{Duration, Instant};
use dashmap::DashMap;
use moka::future::Cache as MokaCache;
use tokio::sync::{RwLock, OnceCell};
use serde::{Serialize, Deserialize};
use tracing::{debug, instrument};

use crate::core::logging::{LoggingSystem, game_logging};
use super::{
    CacheConfig, CacheKey, CachePriority, CacheResult, CacheError, 
    CacheInvalidationEvent, CacheStats, WriteStrategy
};

/// High-performance multi-layered cache system
#[derive(Clone, Debug)]
pub struct GameCache {
    /// Hot cache layer using moka for frequently accessed data
    hot_cache: MokaCache<u64, Arc<CachedValue>>,
    /// Warm cache layer for less frequent but still important data  
    warm_cache: Arc<DashMap<u64, Arc<CachedValue>>>,
    /// Cache metadata and statistics
    metadata: Arc<RwLock<CacheMetadata>>,
    /// Configuration
    config: CacheConfig,
}

/// Cached value wrapper with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedValue {
    pub data: Vec<u8>, // Serialized data
    pub key: CacheKey,
    pub created_at: u64, // Unix timestamp
    pub last_accessed: u64,
    pub access_count: u32,
    pub priority: CachePriority,
    pub size_bytes: usize,
}

impl CachedValue {
    pub fn new<T: Serialize>(value: T, key: CacheKey, priority: CachePriority) -> CacheResult<Self> {
        let data = bincode::serialize(&value)?;
        let size_bytes = data.len() + std::mem::size_of::<Self>();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("System time should be after Unix epoch for cache entry creation")
            .as_secs();

        Ok(Self {
            data,
            key,
            created_at: now,
            last_accessed: now,
            access_count: 0,
            priority,
            size_bytes,
        })
    }

    pub fn deserialize<T: for<'de> Deserialize<'de>>(&self) -> CacheResult<T> {
        bincode::deserialize(&self.data).map_err(CacheError::SerializationError)
    }

    pub fn access(&mut self) {
        self.last_accessed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("System time should be after Unix epoch for cache access tracking")
            .as_secs();
        self.access_count = self.access_count.saturating_add(1);
    }

    /// Get age of cached value
    pub fn age(&self) -> std::time::Duration {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        std::time::Duration::from_secs(now.saturating_sub(self.created_at))
    }

    pub fn age_seconds(&self) -> u64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("System time should be after Unix epoch for cache age calculation")
            .as_secs();
        now.saturating_sub(self.created_at)
    }

    pub fn should_evict(&self, max_age: Duration) -> bool {
        self.age_seconds() > max_age.as_secs()
    }
}

/// Cache metadata and statistics
#[derive(Debug)]
pub struct CacheMetadata {
    pub stats: CacheStats,
    pub last_cleanup: Instant,
    pub total_capacity_bytes: u64,
    pub eviction_policy: EvictionPolicy,
}

impl Default for CacheMetadata {
    fn default() -> Self {
        Self {
            stats: CacheStats::default(),
            last_cleanup: Instant::now(),
            total_capacity_bytes: 0,
            eviction_policy: EvictionPolicy::default(),
        }
    }
}

/// Cache eviction policies
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvictionPolicy {
    /// Least Recently Used
    LRU,
    /// Least Frequently Used  
    LFU,
    /// Time-To-Live based
    TTL,
    /// Priority-based (game-specific)
    Priority,
    /// Adaptive based on game state
    Adaptive,
}

impl Default for EvictionPolicy {
    fn default() -> Self {
        Self::Adaptive
    }
}

impl GameCache {
    /// Create a new game cache with the given configuration
    pub fn new(config: CacheConfig) -> Self {
        let _hot_cache_capacity = (config.max_memory_mb as usize * 1024 * 1024) / 2; // 50% for hot cache
        
        let hot_cache = MokaCache::builder()
            .max_capacity(1000) // Number of items, not bytes - moka handles this
            .time_to_live(config.default_ttl)
            .time_to_idle(Duration::from_secs(60))
            .build();

        let warm_cache = Arc::new(DashMap::new());

        let metadata = Arc::new(RwLock::new(CacheMetadata {
            total_capacity_bytes: config.max_memory_mb * 1024 * 1024,
            eviction_policy: EvictionPolicy::Adaptive,
            ..Default::default()
        }));

        Self {
            hot_cache,
            warm_cache,
            metadata,
            config,
        }
    }

    /// Get a cached value by key
    #[instrument(name = "cache_get", skip(self), fields(key_type = ?key))]
    pub async fn get<T: for<'de> Deserialize<'de>>(&self, key: &CacheKey) -> CacheResult<Option<T>> {
        let hash = key.fast_hash();
        let start_time = Instant::now();
        let correlation_id = LoggingSystem::generate_correlation_id();

        debug!(
            target: "game::cache",
            correlation_id = correlation_id,
            key_type = ?key,
            key_hash = hash,
            "Cache lookup initiated"
        );

        // Try hot cache first
        if let Some(cached_value) = self.hot_cache.get(&hash).await {
            self.update_hit_stats(start_time).await;
            let mut value = Arc::try_unwrap(cached_value).unwrap_or_else(|arc| (*arc).clone());
            let access_count = value.access_count;
            let age_ms = value.age().as_millis();
            
            value.access();
            
            debug!(
                target: "game::cache",
                correlation_id = correlation_id,
                key_type = ?key,
                key_hash = hash,
                cache_tier = "hot",
                access_count = access_count,
                age_ms = age_ms,
                size_bytes = value.size_bytes,
                "Cache hit in hot tier"
            );
            
            let lookup_duration = start_time.elapsed().as_secs_f64() * 1000.0;
            game_logging::log_performance_event("cache_hit_hot", lookup_duration, 1);
            
            return Ok(Some(value.deserialize::<T>()?));
        }

        // Try warm cache
        if let Some(cached_value) = self.warm_cache.get(&hash) {
            let mut value = Arc::try_unwrap(Arc::clone(&cached_value)).unwrap_or_else(|arc| (*arc).clone());
            let access_count = value.access_count;
            let promoted = value.access_count >= 3;
            value.access();
            
            // Promote to hot cache if frequently accessed
            if promoted {
                debug!(
                    target: "game::cache",
                    correlation_id = correlation_id,
                    key_type = ?key,
                    key_hash = hash,
                    access_count = access_count,
                    "Promoting cache entry from warm to hot tier"
                );
                let _ = self.hot_cache.insert(hash, Arc::new(value.clone())).await;
            }
            
            self.update_hit_stats(start_time).await;
            
            debug!(
                target: "game::cache",
                correlation_id = correlation_id,
                key_type = ?key,
                key_hash = hash,
                cache_tier = "warm",
                access_count = access_count,
                promoted = promoted,
                size_bytes = value.size_bytes,
                "Cache hit in warm tier"
            );
            
            let lookup_duration = start_time.elapsed().as_secs_f64() * 1000.0;
            game_logging::log_performance_event("cache_hit_warm", lookup_duration, 1);
            
            return Ok(Some(value.deserialize::<T>()?));
        }

        self.update_miss_stats().await;
        let lookup_duration = start_time.elapsed().as_secs_f64() * 1000.0;
        
        debug!(
            target: "game::cache",
            correlation_id = correlation_id,
            key_type = ?key,
            key_hash = hash,
            lookup_duration_ms = lookup_duration,
            "Cache miss - key not found"
        );
        
        game_logging::log_performance_event("cache_miss", lookup_duration, 0);
        
        Ok(None)
    }

    /// Set a cached value with automatic tier selection
    #[instrument(name = "cache_set", skip(self, value), fields(key_type = ?key, priority = ?priority))]
    pub async fn set<T: Serialize>(&self, key: CacheKey, value: T, priority: CachePriority) -> CacheResult<()> {
        let set_start = Instant::now();
        let correlation_id = LoggingSystem::generate_correlation_id();
        
        let cached_value = Arc::new(CachedValue::new(value, key.clone(), priority)?);
        let hash = key.fast_hash();
        let size_bytes = cached_value.size_bytes;
        let selected_tier;

        match priority {
            CachePriority::Critical | CachePriority::High => {
                // Store in hot cache for quick access
                self.hot_cache.insert(hash, Arc::clone(&cached_value)).await;
                selected_tier = "hot";
            }
            CachePriority::Normal | CachePriority::Medium | CachePriority::Low => {
                // Store in warm cache  
                self.warm_cache.insert(hash, Arc::clone(&cached_value));
                selected_tier = "warm";
            }
        }

        let set_duration = set_start.elapsed().as_secs_f64() * 1000.0;

        debug!(
            target: "game::cache",
            correlation_id = correlation_id,
            key_type = ?key,
            key_hash = hash,
            priority = ?priority,
            cache_tier = selected_tier,
            size_bytes = size_bytes,
            set_duration_ms = set_duration,
            "Cache entry stored successfully"
        );

        self.update_write_stats(&cached_value).await;
        game_logging::log_performance_event("cache_set", set_duration, 1);
        
        Ok(())
    }

    /// Remove a cached value
    pub async fn remove(&self, key: &CacheKey) -> bool {
        let hash = key.fast_hash();
        
        let hot_removed = self.hot_cache.remove(&hash).await.is_some();
        let warm_removed = self.warm_cache.remove(&hash).is_some();

        hot_removed || warm_removed
    }

    /// Clear all cached values
    pub async fn clear(&self) {
        self.hot_cache.invalidate_all();
        self.warm_cache.clear();
        
        let mut metadata = self.metadata.write().await;
        metadata.stats = CacheStats::default();
    }

    /// Handle cache invalidation events
    pub async fn handle_invalidation(&self, event: &CacheInvalidationEvent) {
        match event {
            CacheInvalidationEvent::TurnAdvanced(turn) => {
                if self.config.turn_based_invalidation {
                    self.invalidate_turn_dependent_caches(*turn).await;
                }
            }
            CacheInvalidationEvent::EntityChanged(entity) => {
                self.invalidate_entity_caches(*entity).await;
            }
            CacheInvalidationEvent::PlayerChanged(player_id) => {
                self.invalidate_player_caches(*player_id).await;
            }
            CacheInvalidationEvent::WorldChanged => {
                self.invalidate_spatial_caches().await;
            }
            CacheInvalidationEvent::Manual(filter) => {
                self.invalidate_with_filter(filter).await;
            }
            CacheInvalidationEvent::WorldGeneration(_generation) => {
                // World generation changed - clear all caches
                self.clear().await;
            }
            CacheInvalidationEvent::EntityModified { entity, .. } => {
                // Entity was modified - invalidate related caches
                self.invalidate_entity_caches(*entity).await;
            }
        }
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        let metadata = self.metadata.read().await;
        let mut stats = metadata.stats.clone();
        
        // Add memory usage from both cache layers
        stats.memory_usage_bytes = self.calculate_memory_usage().await;
        stats.cache_count = self.hot_cache.entry_count() as usize + self.warm_cache.len();
        stats.update_hit_ratio();
        
        stats
    }

    /// Perform cache maintenance (cleanup, optimization)
    pub async fn maintain(&self) {
        let now = Instant::now();
        let mut metadata = self.metadata.write().await;
        
        // Only run maintenance periodically
        if now.duration_since(metadata.last_cleanup) < Duration::from_secs(30) {
            return;
        }

        metadata.last_cleanup = now;
        drop(metadata); // Release lock for async operations

        // Clean expired entries from warm cache
        self.cleanup_expired_entries().await;
        
        // Optimize cache distribution
        self.optimize_cache_distribution().await;
        
        // Update eviction policy based on access patterns
        self.update_eviction_policy().await;
    }

    /// Estimate memory usage across all cache layers
    async fn calculate_memory_usage(&self) -> u64 {
        let hot_size = self.hot_cache.entry_count() * 200; // Rough estimate
        let warm_size: usize = self.warm_cache.iter()
            .map(|entry| entry.size_bytes)
            .sum();
        
        (hot_size as u64) + (warm_size as u64)
    }

    /// Clean up expired entries from warm cache
    async fn cleanup_expired_entries(&self) {
        let max_age = self.config.default_ttl;
        let expired_keys: Vec<u64> = self.warm_cache.iter()
            .filter_map(|entry| {
                if entry.should_evict(max_age) {
                    Some(*entry.key())
                } else {
                    None
                }
            })
            .collect();

        let expired_count = expired_keys.len();
        for key in expired_keys {
            self.warm_cache.remove(&key);
        }

        // Update eviction count
        let mut metadata = self.metadata.write().await;
        metadata.stats.total_evictions += expired_count as u64;
    }

    /// Optimize distribution between hot and warm caches
    async fn optimize_cache_distribution(&self) {
        // Promote frequently accessed warm cache entries to hot cache
        let candidates: Vec<(u64, Arc<CachedValue>)> = self.warm_cache.iter()
            .filter(|entry| entry.access_count >= 5 && entry.priority >= CachePriority::Normal)
            .map(|entry| (*entry.key(), Arc::clone(entry.value())))
            .collect();

        for (hash, value) in candidates {
            if self.hot_cache.entry_count() < 800 { // Leave some room
                self.hot_cache.insert(hash, value).await;
                self.warm_cache.remove(&hash);
            }
        }
    }

    /// Update eviction policy based on access patterns
    async fn update_eviction_policy(&self) {
        let stats = self.stats().await;
        let mut metadata = self.metadata.write().await;
        
        // Switch to more aggressive eviction if memory usage is high
        if stats.memory_usage_bytes > (metadata.total_capacity_bytes * 9 / 10) {
            metadata.eviction_policy = EvictionPolicy::LRU;
        } else if stats.hit_ratio > 0.8 {
            // High hit ratio - can use more relaxed policy
            metadata.eviction_policy = EvictionPolicy::TTL;
        } else {
            // Adaptive policy for normal operation
            metadata.eviction_policy = EvictionPolicy::Adaptive;
        }
    }

    /// Invalidate turn-dependent caches
    async fn invalidate_turn_dependent_caches(&self, _turn: u32) {
        // Remove entries that should be invalidated on turn change
        let keys_to_remove: Vec<u64> = self.warm_cache.iter()
            .filter_map(|entry| {
                match &entry.key {
                    CacheKey::AI(_) | CacheKey::Pathfinding(_) => Some(*entry.key()),
                    _ => None,
                }
            })
            .collect();

        for key in keys_to_remove {
            self.hot_cache.remove(&key).await;
            self.warm_cache.remove(&key);
        }
    }

    /// Invalidate entity-related caches with sophisticated key tracking
    async fn invalidate_entity_caches(&self, entity: bevy_ecs::entity::Entity) {
        let entity_index = entity.index();
        let entity_string = format!("{}:{}", entity_index, entity.generation());
        
        let keys_to_remove: Vec<u64> = self.warm_cache.iter()
            .filter_map(|entry| {
                if self.is_entity_related_cache_key(&entry.key, entity_index, &entity_string) {
                    Some(*entry.key())
                } else {
                    None
                }
            })
            .collect();

        let keys_count = keys_to_remove.len();
        for key in keys_to_remove {
            self.hot_cache.remove(&key).await;
            self.warm_cache.remove(&key);
        }
        
        debug!("Invalidated {} cache entries related to entity {:?}", keys_count, entity);
    }

    /// Check if a cache key is related to a specific entity
    fn is_entity_related_cache_key(&self, key: &CacheKey, entity_index: u32, entity_string: &str) -> bool {
        match key {
            // AI cache keys contain entity references
            CacheKey::AI(ai_key) => {
                // AI keys contain entity information in their context
                ai_key.entity.index() == entity_index
            },
            
            // Query cache keys may contain results with this entity
            CacheKey::Query(query_key) => {
                // For query caches, we need to be conservative and invalidate if:
                // 1. It's a broad query that might include this entity
                // 2. The entity might have components that match the query signature
                use super::query::QueryType;
                match query_key.query_type {
                    QueryType::EntitiesByComponents => true, // Conservative - entity might match
                    QueryType::EntitiesWithData => true,     // Conservative - entity might match
                    QueryType::ArchetypeQuery => true,       // Conservative - entity might be in archetype
                    _ => false,
                }
            },
            
            // Spatial cache keys don't directly reference entities, but might contain results with this entity
            CacheKey::Spatial(_) => {
                // Spatial queries return entity lists, so if an entity moves or is deleted,
                // we should invalidate spatial caches in the area
                // This is conservative but safe
                true
            },
            
            // Custom cache keys might contain entity IDs in the string
            CacheKey::Custom(custom_string) => {
                // Check if the entity ID appears in the custom key string
                custom_string.contains(&entity_index.to_string()) || 
                custom_string.contains(entity_string)
            },
            
            // Pathfinding caches don't directly reference specific entities
            // but might need invalidation if the entity affects terrain/movement
            CacheKey::Pathfinding(_) => {
                // Conservative approach - pathfinding might be affected by entity movement
                // In a more sophisticated implementation, we could check if the entity
                // is at the start/end positions or affects the path
                false // For now, only invalidate pathfinding on broader changes
            },
            
            // Player and Rendering caches are not typically entity-specific
            CacheKey::Player(_) | CacheKey::Rendering(_) => false,
        }
    }

    /// Invalidate player-related caches
    async fn invalidate_player_caches(&self, _player_id: u32) {
        let keys_to_remove: Vec<u64> = self.warm_cache.iter()
            .filter_map(|entry| {
                match &entry.key {
                    CacheKey::Player(_) | CacheKey::AI(_) => Some(*entry.key()),
                    _ => None,
                }
            })
            .collect();

        for key in keys_to_remove {
            self.hot_cache.remove(&key).await;
            self.warm_cache.remove(&key);
        }
    }

    /// Invalidate spatial caches
    async fn invalidate_spatial_caches(&self) {
        let keys_to_remove: Vec<u64> = self.warm_cache.iter()
            .filter_map(|entry| {
                match &entry.key {
                    CacheKey::Spatial(_) => Some(*entry.key()),
                    _ => None,
                }
            })
            .collect();

        for key in keys_to_remove {
            self.hot_cache.remove(&key).await;
            self.warm_cache.remove(&key);
        }
    }

    /// Invalidate caches using custom filter
    async fn invalidate_with_filter(&self, filter: &(dyn Fn(&CacheKey) -> bool + Send + Sync)) {
        let keys_to_remove: Vec<u64> = self.warm_cache.iter()
            .filter_map(|entry| {
                if filter(&entry.key) {
                    Some(*entry.key())
                } else {
                    None
                }
            })
            .collect();

        for key in keys_to_remove {
            self.hot_cache.remove(&key).await;
            self.warm_cache.remove(&key);
        }
    }

    /// Update hit statistics
    async fn update_hit_stats(&self, start_time: Instant) {
        let access_time = start_time.elapsed().as_micros() as f64;
        let mut metadata = self.metadata.write().await;
        metadata.stats.total_hits += 1;
        
        // Update rolling average of access time
        let total_accesses = metadata.stats.total_hits + metadata.stats.total_misses;
        metadata.stats.avg_access_time_micros = 
            (metadata.stats.avg_access_time_micros * (total_accesses - 1) as f64 + access_time) / total_accesses as f64;
    }

    /// Update miss statistics
    async fn update_miss_stats(&self) {
        let mut metadata = self.metadata.write().await;
        metadata.stats.total_misses += 1;
    }

    /// Update write statistics
    async fn update_write_stats(&self, cached_value: &CachedValue) {
        let mut metadata = self.metadata.write().await;
        metadata.stats.memory_usage_bytes += cached_value.size_bytes as u64;
    }
}

/// Cache builder for easy configuration
pub struct GameCacheBuilder {
    config: CacheConfig,
}

impl Default for GameCacheBuilder {
    fn default() -> Self {
        Self {
            config: CacheConfig::default(),
        }
    }
}

impl GameCacheBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn max_memory_mb(mut self, mb: u64) -> Self {
        self.config.max_memory_mb = mb;
        self
    }

    pub fn default_ttl(mut self, ttl: Duration) -> Self {
        self.config.default_ttl = ttl;
        self
    }

    pub fn turn_based_invalidation(mut self, enabled: bool) -> Self {
        self.config.turn_based_invalidation = enabled;
        self
    }

    pub fn enable_metrics(mut self, enabled: bool) -> Self {
        self.config.enable_metrics = enabled;
        self
    }

    pub fn write_strategy(mut self, strategy: WriteStrategy) -> Self {
        self.config.write_strategy = strategy;
        self
    }

    pub fn num_shards(mut self, shards: usize) -> Self {
        self.config.num_shards = shards;
        self
    }

    pub fn build(self) -> GameCache {
        GameCache::new(self.config)
    }
}

// ============================================================================
// GLOBAL CACHE AND UTILITIES
// ============================================================================

/// Global cache instance for easy access across the application
static GLOBAL_CACHE: OnceCell<GameCache> = OnceCell::const_new();

/// Initialize the global cache with default configuration
pub async fn initialize_global_cache() -> &'static GameCache {
    GLOBAL_CACHE.get_or_init(|| async {
        GameCacheBuilder::new()
            .max_memory_mb(512)
            .default_ttl(Duration::from_secs(300))
            .turn_based_invalidation(true)
            .build()
    }).await
}

/// Get the global cache instance
pub async fn global_cache() -> &'static GameCache {
    GLOBAL_CACHE.get().unwrap_or_else(|| {
        panic!("Global cache not initialized. Call initialize_global_cache() first.");
    })
}

/// Macro for easily caching expensive function calls
#[macro_export]
macro_rules! cached_call {
    ($cache_key:expr, $priority:expr, $computation:block) => {{
        let cache = $crate::core::caching::cache::global_cache().await;
        
        // Try cache first
        if let Ok(Some(cached)) = cache.get(&$cache_key).await {
            cached
        } else {
            // Compute fresh result
            let start_time = std::time::Instant::now();
            let result = $computation;
            
            // Cache the result
            let computation_time = start_time.elapsed();
            let _ = cache.set($cache_key, result.clone(), $priority).await;
            
            tracing::debug!(
                target: "cache::helper",
                cache_key = ?$cache_key,
                computation_time_us = computation_time.as_micros(),
                "Cached function result"
            );
            
            result
        }
    }};
}

/// Helper trait for cacheable functions
#[async_trait::async_trait]
pub trait Cacheable<K, V> {
    /// Execute with caching
    async fn cached(&self, key: K, priority: CachePriority) -> CacheResult<V>;
}

/// Simple cache-aware wrapper for expensive computations
pub struct CachedComputation<T> {
    cache: GameCache,
    performance_tracker: crate::core::caching::metrics::CachePerformanceTracker,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> CachedComputation<T> 
where
    T: Clone + serde::Serialize + for<'de> serde::Deserialize<'de> + Send + Sync + 'static,
{
    pub fn new(cache: GameCache) -> Self {
        Self {
            cache,
            performance_tracker: crate::core::caching::metrics::CachePerformanceTracker::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    pub async fn compute<F, Fut>(&mut self, key: CacheKey, priority: CachePriority, computation: F) -> CacheResult<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        let cache_start = Instant::now();
        
        // Try cache first
        if let Ok(Some(cached)) = self.cache.get::<T>(&key).await {
            let cache_time = cache_start.elapsed();
            self.performance_tracker.record_hit(cache_time);
            return Ok(cached);
        }

        // Compute fresh result
        let compute_start = Instant::now();
        let result = computation().await;
        let compute_time = compute_start.elapsed();
        
        // Cache the result
        let _ = self.cache.set(key, result.clone(), priority).await;
        
        self.performance_tracker.record_miss(compute_time);
        
        Ok(result)
    }

    pub fn performance_stats(&self) -> &crate::core::caching::metrics::CachePerformanceTracker {
        &self.performance_tracker
    }
}
