//! High-performance caching system optimized for grand strategy games
//!
//! This module provides a comprehensive caching infrastructure designed for
//! turn-based strategy games with complex entity relationships, spatial queries,
//! and AI calculations. Built on 2025's best Rust caching libraries.
//!
//! # Architecture
//!
//! The caching system is designed with multiple layers:
//! - **Core Cache**: High-performance concurrent caching using moka
//! - **Spatial Cache**: Hex-grid optimized spatial query results
//! - **Query Cache**: ECS component query result caching
//! - **Strategy Cache**: Game-specific caches (pathfinding, AI, rendering)
//! - **Metrics**: Comprehensive performance monitoring
//!
//! # Integration
//!
//! Leverages existing infrastructure:
//! - XXHash3/Blake3 hashing strategies for optimal performance
//! - Bevy ECS integration for component queries
//! - Turn-based invalidation for deterministic gameplay
//! - Memory-efficient serialization for save/load compatibility

pub mod cache;
pub mod policies;
pub mod spatial;
pub mod query;
pub mod strategies;
pub mod metrics;
pub mod events;

pub use cache::*;
pub use policies::*;
pub use spatial::*;
pub use query::*;
pub use strategies::*;
pub use metrics::*;
pub use events::*;

use std::time::{Duration, Instant};
use tracing::{info, debug, warn, error, instrument, Span};
use crate::core::{hashing::{FastHasher, HashStrategies}, logging::{LoggingSystem, game_logging}};

/// Global cache configuration for the game
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum memory usage in MB for all caches combined
    pub max_memory_mb: u64,
    /// Default TTL for cached items
    pub default_ttl: Duration,
    /// Turn-based cache invalidation enabled
    pub turn_based_invalidation: bool,
    /// Enable cache metrics collection
    pub enable_metrics: bool,
    /// Cache write-through vs write-back strategy
    pub write_strategy: WriteStrategy,
    /// Number of cache shards for concurrent access
    pub num_shards: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_memory_mb: 512, // 512MB default for strategy games
            default_ttl: Duration::from_secs(300), // 5 minutes
            turn_based_invalidation: true,
            enable_metrics: true,
            write_strategy: WriteStrategy::WriteBack,
            num_shards: num_cpus::get(),
        }
    }
}

/// Cache write strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteStrategy {
    /// Write to cache only, update backing store separately
    WriteBack,
    /// Write to cache and backing store simultaneously  
    WriteThrough,
    /// Write to backing store only, bypass cache
    WriteAround,
}

/// Cache priority levels for different data types
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub enum CachePriority {
    /// Critical game state that must remain cached
    Critical = 100,
    /// Important data accessed frequently (pathfinding, AI)
    High = 75,
    /// Normal game data (entity queries, UI state)
    Normal = 50,
    /// Nice-to-have data that can be evicted easily
    Low = 25,
}

/// Cache key types for type-safe cache access
#[derive(Debug, Clone, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CacheKey {
    /// Spatial query key (position, radius, component filter)
    Spatial(SpatialCacheKey),
    /// ECS query key (component signature, world generation)
    Query(QueryCacheKey),
    /// Pathfinding result key (start, end, movement type)
    Pathfinding(PathfindingCacheKey),
    /// AI evaluation key (entity, context, depth)
    AI(AICacheKey),
    /// Rendering data key (sprite name, LOD level, theme)
    Rendering(RenderingCacheKey),
    /// Player data key (player ID, data type)
    Player(PlayerCacheKey),
    /// Custom cache key for game-specific data
    Custom(String),
}

impl CacheKey {
    /// Get cache priority for this key type
    pub fn priority(&self) -> CachePriority {
        match self {
            CacheKey::Spatial(_) => CachePriority::High,
            CacheKey::Query(_) => CachePriority::Normal,
            CacheKey::Pathfinding(_) => CachePriority::High,
            CacheKey::AI(_) => CachePriority::Normal,
            CacheKey::Rendering(_) => CachePriority::Low,
            CacheKey::Player(_) => CachePriority::Critical,
            CacheKey::Custom(_) => CachePriority::Normal,
        }
    }

    /// Get estimated size in bytes for capacity planning
    pub fn estimated_size(&self) -> usize {
        match self {
            CacheKey::Spatial(_) => 64, // Position + metadata
            CacheKey::Query(_) => 128, // Component signature + results
            CacheKey::Pathfinding(_) => 256, // Path vectors
            CacheKey::AI(_) => 512, // Complex decision trees
            CacheKey::Rendering(_) => 1024, // Sprite/texture data
            CacheKey::Player(_) => 64, // Player stats
            CacheKey::Custom(_) => 128, // Conservative estimate
        }
    }

    /// Fast hash using existing game hashing infrastructure
    pub fn fast_hash(&self) -> u64 {
        FastHasher::hash_one(self)
    }
}

/// Cache invalidation events
pub enum CacheInvalidationEvent {
    /// New turn started - invalidate turn-dependent caches
    TurnAdvanced(u32),
    /// Entity changed - invalidate entity-related caches  
    EntityChanged(bevy_ecs::entity::Entity),
    /// Player state changed - invalidate player caches
    PlayerChanged(u32),
    /// World state changed - invalidate spatial caches
    WorldChanged,
    /// World generation advanced - invalidate all caches
    WorldGeneration(u32),
    /// Entity modified - cascade through relevant caches
    EntityModified { 
        entity: bevy_ecs::entity::Entity, 
        archetype_changed: bool,
        position_changed: Option<glam::IVec2>,
    },
    /// Manual invalidation with custom filter
    Manual(Box<dyn Fn(&CacheKey) -> bool + Send + Sync>),
}

impl std::fmt::Debug for CacheInvalidationEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TurnAdvanced(turn) => f.debug_tuple("TurnAdvanced").field(turn).finish(),
            Self::EntityChanged(entity) => f.debug_tuple("EntityChanged").field(entity).finish(),
            Self::PlayerChanged(player) => f.debug_tuple("PlayerChanged").field(player).finish(),
            Self::WorldChanged => write!(f, "WorldChanged"),
            Self::WorldGeneration(gen) => f.debug_tuple("WorldGeneration").field(gen).finish(),
            Self::EntityModified { entity, archetype_changed, position_changed } => 
                f.debug_struct("EntityModified")
                    .field("entity", entity)
                    .field("archetype_changed", archetype_changed)
                    .field("position_changed", position_changed)
                    .finish(),
            Self::Manual(_) => write!(f, "Manual(<function>)"),
        }
    }
}

impl Clone for CacheInvalidationEvent {
    fn clone(&self) -> Self {
        match self {
            Self::TurnAdvanced(turn) => Self::TurnAdvanced(*turn),
            Self::EntityChanged(entity) => Self::EntityChanged(*entity),
            Self::PlayerChanged(player) => Self::PlayerChanged(*player),
            Self::WorldChanged => Self::WorldChanged,
            Self::WorldGeneration(gen) => Self::WorldGeneration(*gen),
            Self::EntityModified { entity, archetype_changed, position_changed } => 
                Self::EntityModified {
                    entity: *entity,
                    archetype_changed: *archetype_changed,
                    position_changed: *position_changed,
                },
            Self::Manual(_) => {
                // Functions can't be cloned, so we'll panic if someone tries to clone this variant
                panic!("Manual cache invalidation events cannot be cloned - they contain function pointers")
            }
        }
    }
}

/// Cache statistics aggregated across all cache types
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub total_hits: u64,
    pub total_misses: u64,
    pub total_evictions: u64,
    pub memory_usage_bytes: u64,
    pub cache_count: usize,
    pub avg_access_time_micros: f64,
    pub hit_ratio: f64,
}

impl CacheStats {
    /// Update hit ratio based on hits and misses
    pub fn update_hit_ratio(&mut self) {
        let total_accesses = self.total_hits + self.total_misses;
        if total_accesses > 0 {
            self.hit_ratio = self.total_hits as f64 / total_accesses as f64;
        }
    }
}

/// Error types for cache operations
#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    #[error("Cache capacity exceeded")]
    CapacityExceeded,
    #[error("Cache key not found: {key}")]
    KeyNotFound { key: String },
    #[error("Cache write failed: {reason}")]
    WriteFailed { reason: String },
    #[error("Cache serialization error: {0}")]
    SerializationError(#[from] bincode::Error),
    #[error("Cache configuration error: {0}")]
    ConfigError(String),
}

/// Result type for cache operations
pub type CacheResult<T> = Result<T, CacheError>;

/// Cache entry metadata
#[derive(Debug, Clone)]
pub struct CacheEntry<T> {
    pub value: T,
    pub created_at: Instant,
    pub last_accessed: Instant,
    pub access_count: u32,
    pub priority: CachePriority,
    pub size_bytes: usize,
}

impl<T> CacheEntry<T> {
    pub fn new(value: T, priority: CachePriority, size_bytes: usize) -> Self {
        let now = Instant::now();
        Self {
            value,
            created_at: now,
            last_accessed: now,
            access_count: 0,
            priority,
            size_bytes,
        }
    }

    pub fn access(&mut self) {
        self.last_accessed = Instant::now();
        self.access_count = self.access_count.saturating_add(1);
    }

    pub fn age(&self) -> Duration {
        Instant::now() - self.created_at
    }
}
