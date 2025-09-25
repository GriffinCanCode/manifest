//! Cache policies for eviction, expiration, and memory management
//!
//! Provides sophisticated policies optimized for grand strategy games:
//! - Turn-based TTL that aligns with game mechanics
//! - Priority-based eviction for critical game data
//! - Memory pressure handling with graceful degradation
//! - Adaptive policies that learn from access patterns

use std::time::{Duration, Instant};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

use crate::core::hashing::FastHashMap;
use super::{CacheKey, CachePriority, CachedValue};

/// Cache policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachePolicy {
    /// Time-to-live settings
    pub ttl: TTLPolicy,
    /// Eviction strategy
    pub eviction: EvictionPolicy,
    /// Memory management
    pub memory: MemoryPolicy,
    /// Turn-based game mechanics
    pub turn_based: TurnBasedPolicy,
}

impl Default for CachePolicy {
    fn default() -> Self {
        Self {
            ttl: TTLPolicy::default(),
            eviction: EvictionPolicy::Adaptive,
            memory: MemoryPolicy::default(),
            turn_based: TurnBasedPolicy::default(),
        }
    }
}

/// Time-to-live policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TTLPolicy {
    /// Default TTL for all cache entries
    pub default_duration: Duration,
    /// TTL multipliers for different priority levels
    pub priority_multipliers: HashMap<CachePriority, f64>,
    /// Dynamic TTL based on access frequency
    pub dynamic_ttl: bool,
    /// Maximum TTL regardless of other factors
    pub max_ttl: Duration,
}

impl Default for TTLPolicy {
    fn default() -> Self {
        let mut priority_multipliers = HashMap::new();
        priority_multipliers.insert(CachePriority::Critical, 10.0); // 10x longer TTL
        priority_multipliers.insert(CachePriority::High, 3.0);
        priority_multipliers.insert(CachePriority::Normal, 1.0);
        priority_multipliers.insert(CachePriority::Low, 0.5); // 50% shorter TTL

        Self {
            default_duration: Duration::from_secs(300), // 5 minutes
            priority_multipliers,
            dynamic_ttl: true,
            max_ttl: Duration::from_secs(3600), // 1 hour max
        }
    }
}

impl TTLPolicy {
    /// Calculate TTL for a specific cache entry
    pub fn calculate_ttl(&self, priority: CachePriority, access_count: u32) -> Duration {
        let base_duration = self.default_duration;
        let priority_multiplier = self.priority_multipliers.get(&priority).copied().unwrap_or(1.0);
        
        let mut ttl = Duration::from_secs_f64(base_duration.as_secs_f64() * priority_multiplier);

        // Dynamic TTL based on access frequency
        if self.dynamic_ttl && access_count > 0 {
            // More frequently accessed items get longer TTL
            let frequency_multiplier = 1.0 + (access_count as f64).log2() / 10.0;
            ttl = Duration::from_secs_f64(ttl.as_secs_f64() * frequency_multiplier);
        }

        // Cap at maximum TTL
        if ttl > self.max_ttl {
            ttl = self.max_ttl;
        }

        ttl
    }

    /// Check if an entry has expired
    pub fn is_expired(&self, cached_value: &CachedValue) -> bool {
        let ttl = self.calculate_ttl(cached_value.priority, cached_value.access_count);
        cached_value.age_seconds() > ttl.as_secs()
    }
}

/// Eviction policies for when cache reaches capacity
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvictionPolicy {
    /// Least Recently Used
    LRU,
    /// Least Frequently Used
    LFU,
    /// First In, First Out
    FIFO,
    /// Priority-based eviction (never evict critical data)
    Priority,
    /// Random eviction
    Random,
    /// Adaptive policy that switches based on patterns
    Adaptive,
    /// Custom game-specific eviction logic
    GameSpecific(GameSpecificEviction),
}

/// Game-specific eviction strategies
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameSpecificEviction {
    /// Evict based on turn distance (older turns first)
    TurnDistance,
    /// Evict based on spatial distance from player focus
    SpatialDistance,
    /// Evict AI calculations before rendering data
    TypePriority,
    /// Hybrid approach combining multiple factors
    Hybrid,
}

/// Memory management policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPolicy {
    /// Maximum memory usage in bytes
    pub max_bytes: u64,
    /// Memory usage threshold for starting eviction (0.0-1.0)
    pub eviction_threshold: f64,
    /// Memory usage threshold for aggressive cleanup (0.0-1.0)
    pub aggressive_threshold: f64,
    /// Target memory usage after cleanup (0.0-1.0)
    pub target_usage: f64,
    /// Enable memory pressure monitoring
    pub pressure_monitoring: bool,
}

impl Default for MemoryPolicy {
    fn default() -> Self {
        Self {
            max_bytes: 512 * 1024 * 1024, // 512MB
            eviction_threshold: 0.8, // Start evicting at 80%
            aggressive_threshold: 0.95, // Aggressive cleanup at 95%
            target_usage: 0.6, // Clean down to 60%
            pressure_monitoring: true,
        }
    }
}

impl MemoryPolicy {
    /// Check if memory usage requires eviction
    pub fn needs_eviction(&self, current_usage: u64) -> EvictionLevel {
        let usage_ratio = current_usage as f64 / self.max_bytes as f64;
        
        if usage_ratio >= self.aggressive_threshold {
            EvictionLevel::Aggressive
        } else if usage_ratio >= self.eviction_threshold {
            EvictionLevel::Normal
        } else {
            EvictionLevel::None
        }
    }

    /// Calculate target number of bytes to free
    pub fn bytes_to_free(&self, current_usage: u64) -> u64 {
        let target_bytes = (self.max_bytes as f64 * self.target_usage) as u64;
        if current_usage > target_bytes {
            current_usage - target_bytes
        } else {
            0
        }
    }
}

/// Memory eviction urgency levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvictionLevel {
    None,
    Normal,
    Aggressive,
}

/// Turn-based game policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnBasedPolicy {
    /// Invalidate caches when turn advances
    pub invalidate_on_turn: bool,
    /// Cache entries that persist across turns
    pub persistent_types: Vec<CacheKeyType>,
    /// Maximum number of turns to keep cache entries
    pub max_turn_age: u32,
    /// Turn-based memory scaling
    pub turn_memory_scaling: bool,
}

impl Default for TurnBasedPolicy {
    fn default() -> Self {
        Self {
            invalidate_on_turn: true,
            persistent_types: vec![
                CacheKeyType::Player,
                CacheKeyType::Rendering,
            ],
            max_turn_age: 10,
            turn_memory_scaling: true,
        }
    }
}

/// Cache key types for policy configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CacheKeyType {
    Spatial,
    Query,
    Pathfinding,
    AI,
    Rendering,
    Player,
    Custom,
}

impl From<&CacheKey> for CacheKeyType {
    fn from(key: &CacheKey) -> Self {
        match key {
            CacheKey::Spatial(_) => CacheKeyType::Spatial,
            CacheKey::Query(_) => CacheKeyType::Query,
            CacheKey::Pathfinding(_) => CacheKeyType::Pathfinding,
            CacheKey::AI(_) => CacheKeyType::AI,
            CacheKey::Rendering(_) => CacheKeyType::Rendering,
            CacheKey::Player(_) => CacheKeyType::Player,
            CacheKey::Custom(_) => CacheKeyType::Custom,
        }
    }
}

/// Cache policy engine that applies policies to cache operations
pub struct PolicyEngine {
    policy: CachePolicy,
    current_turn: u32,
    access_patterns: FastHashMap<u64, AccessPattern>,
    last_cleanup: Instant,
}

/// Access pattern tracking for adaptive policies
#[derive(Debug, Clone)]
pub struct AccessPattern {
    pub total_accesses: u32,
    pub recent_accesses: u32,
    pub last_access: Instant,
    pub access_frequency: f64, // Accesses per second
    pub trend: AccessTrend,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessTrend {
    Increasing,
    Stable,
    Decreasing,
}

impl PolicyEngine {
    /// Create a new policy engine with default policies
    pub fn new() -> Self {
        Self {
            policy: CachePolicy::default(),
            current_turn: 1,
            access_patterns: FastHashMap::default(),
            last_cleanup: Instant::now(),
        }
    }

    /// Create a policy engine with custom policies
    pub fn with_policy(policy: CachePolicy) -> Self {
        Self {
            policy,
            current_turn: 1,
            access_patterns: FastHashMap::default(),
            last_cleanup: Instant::now(),
        }
    }

    /// Update current game turn
    pub fn advance_turn(&mut self, turn: u32) {
        self.current_turn = turn;
    }

    /// Check if a cache entry should be evicted
    pub fn should_evict(&self, cached_value: &CachedValue, current_memory: u64) -> bool {
        // Check TTL expiration
        if self.policy.ttl.is_expired(cached_value) {
            return true;
        }

        // Check memory pressure
        let eviction_level = self.policy.memory.needs_eviction(current_memory);
        match eviction_level {
            EvictionLevel::None => false,
            EvictionLevel::Normal => {
                self.should_evict_normal(cached_value)
            }
            EvictionLevel::Aggressive => {
                self.should_evict_aggressive(cached_value)
            }
        }
    }

    /// Check if entry should be evicted under normal memory pressure
    fn should_evict_normal(&self, cached_value: &CachedValue) -> bool {
        match &self.policy.eviction {
            EvictionPolicy::LRU => {
                // Evict if not accessed recently
                cached_value.age_seconds() > 300 // 5 minutes
            }
            EvictionPolicy::LFU => {
                // Evict if low access count
                cached_value.access_count < 3
            }
            EvictionPolicy::Priority => {
                // Never evict critical or high priority items
                cached_value.priority < CachePriority::Normal
            }
            EvictionPolicy::GameSpecific(strategy) => {
                self.should_evict_game_specific(cached_value, strategy)
            }
            EvictionPolicy::Adaptive => {
                self.should_evict_adaptive(cached_value)
            }
            _ => false,
        }
    }

    /// Check if entry should be evicted under aggressive memory pressure
    fn should_evict_aggressive(&self, cached_value: &CachedValue) -> bool {
        match cached_value.priority {
            CachePriority::Critical => false, // Never evict critical data
            CachePriority::High => {
                // Only evict high priority if very old or unused
                cached_value.age_seconds() > 1800 || cached_value.access_count == 0
            }
            _ => true, // Evict normal and low priority items
        }
    }

    /// Game-specific eviction logic
    fn should_evict_game_specific(&self, cached_value: &CachedValue, strategy: &GameSpecificEviction) -> bool {
        match strategy {
            GameSpecificEviction::TurnDistance => {
                // Evict cache entries from old turns
                let turn_age = self.current_turn.saturating_sub(1); // Rough approximation
                turn_age > self.policy.turn_based.max_turn_age
            }
            GameSpecificEviction::TypePriority => {
                // Evict AI calculations before rendering data
                match &cached_value.key {
                    CacheKey::AI(_) => cached_value.access_count < 2,
                    CacheKey::Pathfinding(_) => cached_value.age_seconds() > 60,
                    CacheKey::Rendering(_) => cached_value.age_seconds() > 600,
                    _ => cached_value.age_seconds() > 300,
                }
            }
            GameSpecificEviction::SpatialDistance => {
                // Would need player position context - simplified for now
                cached_value.access_count < 2
            }
            GameSpecificEviction::Hybrid => {
                // Combine multiple factors
                let age_factor = cached_value.age_seconds() > 300;
                let access_factor = cached_value.access_count < 3;
                let priority_factor = cached_value.priority < CachePriority::High;
                
                (age_factor && access_factor) || (priority_factor && age_factor)
            }
        }
    }

    /// Adaptive eviction based on learned patterns
    fn should_evict_adaptive(&self, cached_value: &CachedValue) -> bool {
        let key_hash = cached_value.key.fast_hash();
        
        if let Some(pattern) = self.access_patterns.get(&key_hash) {
            match pattern.trend {
                AccessTrend::Increasing => false, // Keep trending up data
                AccessTrend::Stable => cached_value.age_seconds() > 300,
                AccessTrend::Decreasing => true, // Evict trending down data
            }
        } else {
            // No pattern data - use conservative approach
            cached_value.age_seconds() > 600 && cached_value.access_count < 2
        }
    }

    /// Record cache access for pattern learning
    pub fn record_access(&mut self, key_hash: u64) {
        let now = Instant::now();
        
        self.access_patterns.entry(key_hash)
            .and_modify(|pattern| {
                pattern.total_accesses += 1;
                pattern.recent_accesses += 1;
                
                // Update frequency calculation
                let time_diff = now.duration_since(pattern.last_access).as_secs_f64();
                if time_diff > 0.0 {
                    pattern.access_frequency = pattern.recent_accesses as f64 / time_diff;
                }
                
                pattern.last_access = now;
                
                // Update trend (simplified)
                if pattern.access_frequency > 0.1 {
                    pattern.trend = AccessTrend::Increasing;
                } else if pattern.access_frequency < 0.01 {
                    pattern.trend = AccessTrend::Decreasing;
                } else {
                    pattern.trend = AccessTrend::Stable;
                }
            })
            .or_insert(AccessPattern {
                total_accesses: 1,
                recent_accesses: 1,
                last_access: now,
                access_frequency: 0.0,
                trend: AccessTrend::Stable,
            });
    }

    /// Periodic cleanup of access patterns
    pub fn cleanup_patterns(&mut self) {
        let now = Instant::now();
        
        // Only cleanup patterns periodically
        if now.duration_since(self.last_cleanup) < Duration::from_secs(300) {
            return;
        }

        self.last_cleanup = now;
        
        // Reset recent access counts
        for pattern in self.access_patterns.values_mut() {
            pattern.recent_accesses = 0;
        }

        // Remove very old patterns
        let cutoff = now - Duration::from_secs(3600);
        self.access_patterns.retain(|_, pattern| pattern.last_access > cutoff);
    }

    /// Get current cache policy
    pub fn policy(&self) -> &CachePolicy {
        &self.policy
    }

    /// Update cache policy
    pub fn update_policy(&mut self, policy: CachePolicy) {
        self.policy = policy;
    }
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}
