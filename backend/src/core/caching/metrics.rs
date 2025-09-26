//! Cache metrics and performance monitoring
//!
//! Comprehensive metrics collection for cache performance analysis:
//! - Hit/miss ratios and access patterns
//! - Memory usage and capacity planning
//! - Cache efficiency and optimization recommendations
//! - Performance impact measurement

use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};

use crate::core::hashing::FastHashMap;
use super::{CacheKey, CachePriority};

/// Comprehensive cache metrics collector
#[derive(Debug, Clone)]
pub struct CacheMetrics {
    /// Overall cache statistics
    pub global: GlobalMetrics,
    /// Per-cache-type breakdown
    pub by_type: FastHashMap<String, TypeMetrics>,
    /// Per-priority-level breakdown  
    pub by_priority: FastHashMap<CachePriority, PriorityMetrics>,
    /// Time-series data for trend analysis
    pub time_series: TimeSeries,
    /// Performance impact measurements
    pub performance: PerformanceMetrics,
    /// Memory usage tracking
    pub memory: MemoryMetrics,
}

impl Default for CacheMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl CacheMetrics {
    /// Create new metrics collector
    pub fn new() -> Self {
        Self {
            global: GlobalMetrics::new(),
            by_type: FastHashMap::default(),
            by_priority: FastHashMap::default(),
            time_series: TimeSeries::new(),
            performance: PerformanceMetrics::new(),
            memory: MemoryMetrics::new(),
        }
    }

    /// Record a cache hit
    pub fn record_hit(&mut self, key: &CacheKey, access_time: Duration) {
        let key_type = self.key_type_name(key);
        
        // Global metrics
        self.global.record_hit(access_time);
        
        // Per-type metrics
        self.by_type.entry(key_type)
            .or_insert_with(TypeMetrics::new)
            .record_hit(access_time);
        
        // Per-priority metrics
        self.by_priority.entry(key.priority())
            .or_insert_with(PriorityMetrics::new)
            .record_hit(access_time);
        
        // Time series
        self.time_series.record_event(CacheEvent::Hit, key);
        
        // Performance tracking
        self.performance.record_access(access_time, true);
    }

    /// Record a cache miss
    pub fn record_miss(&mut self, key: &CacheKey, computation_time: Duration) {
        let key_type = self.key_type_name(key);
        
        // Global metrics
        self.global.record_miss(computation_time);
        
        // Per-type metrics
        self.by_type.entry(key_type)
            .or_insert_with(TypeMetrics::new)
            .record_miss(computation_time);
        
        // Per-priority metrics
        self.by_priority.entry(key.priority())
            .or_insert_with(PriorityMetrics::new)
            .record_miss(computation_time);
        
        // Time series
        self.time_series.record_event(CacheEvent::Miss, key);
        
        // Performance tracking
        self.performance.record_access(computation_time, false);
    }

    /// Record a cache write (set operation)
    pub fn record_write(&mut self, key: &CacheKey, size_bytes: usize) {
        let key_type = self.key_type_name(key);
        
        // Global metrics
        self.global.record_write(size_bytes);
        
        // Per-type metrics
        self.by_type.entry(key_type)
            .or_insert_with(TypeMetrics::new)
            .record_write(size_bytes);
        
        // Memory tracking
        self.memory.record_allocation(size_bytes);
        
        // Time series
        self.time_series.record_event(CacheEvent::Write, key);
    }

    /// Record a cache eviction
    pub fn record_eviction(&mut self, key: &CacheKey, size_bytes: usize, reason: EvictionReason) {
        let key_type = self.key_type_name(key);
        
        // Global metrics
        self.global.record_eviction(size_bytes);
        
        // Per-type metrics
        self.by_type.entry(key_type)
            .or_insert_with(TypeMetrics::new)
            .record_eviction(size_bytes, reason);
        
        // Memory tracking
        self.memory.record_deallocation(size_bytes);
        
        // Time series
        self.time_series.record_event(CacheEvent::Eviction, key);
    }

    /// Update memory usage
    pub fn update_memory_usage(&mut self, current_bytes: u64, capacity_bytes: u64) {
        self.memory.update_usage(current_bytes, capacity_bytes);
    }

    /// Get cache efficiency score (0.0-1.0, higher is better)
    pub fn efficiency_score(&self) -> f64 {
        let hit_ratio = self.global.hit_ratio();
        let memory_efficiency = self.memory.efficiency_ratio();
        let performance_score = self.performance.efficiency_score();
        
        // Weighted average of different efficiency measures
        (hit_ratio * 0.4) + (memory_efficiency * 0.3) + (performance_score * 0.3)
    }

    /// Generate optimization recommendations
    pub fn optimization_recommendations(&self) -> Vec<OptimizationRecommendation> {
        let mut recommendations = Vec::new();
        
        // Check hit ratios by type
        for (cache_type, metrics) in &self.by_type {
            if metrics.hit_ratio() < 0.6 {
                recommendations.push(OptimizationRecommendation {
                    category: OptimizationCategory::HitRatio,
                    severity: if metrics.hit_ratio() < 0.3 { Severity::High } else { Severity::Medium },
                    description: format!("Low hit ratio ({:.1}%) for {} cache", 
                                       metrics.hit_ratio() * 100.0, cache_type),
                    suggestion: "Consider increasing TTL or cache size for this type".to_string(),
                });
            }
        }
        
        // Check memory usage
        if self.memory.usage_ratio() > 0.9 {
            recommendations.push(OptimizationRecommendation {
                category: OptimizationCategory::Memory,
                severity: Severity::High,
                description: format!("High memory usage ({:.1}%)", self.memory.usage_ratio() * 100.0),
                suggestion: "Increase cache capacity or implement more aggressive eviction".to_string(),
            });
        }
        
        // Check performance impact
        if self.performance.avg_miss_penalty() > Duration::from_millis(100) {
            recommendations.push(OptimizationRecommendation {
                category: OptimizationCategory::Performance,
                severity: Severity::Medium,
                description: format!("High miss penalty ({:.1}ms average)", 
                                   self.performance.avg_miss_penalty().as_secs_f64() * 1000.0),
                suggestion: "Focus on caching expensive computations".to_string(),
            });
        }
        
        recommendations
    }

    /// Generate performance report
    pub fn performance_report(&self) -> PerformanceReport {
        PerformanceReport {
            hit_ratio: self.global.hit_ratio(),
            miss_ratio: 1.0 - self.global.hit_ratio(),
            avg_hit_time: self.global.avg_hit_time(),
            avg_miss_time: self.global.avg_miss_time(),
            total_hits: self.global.total_hits,
            total_misses: self.global.total_misses,
            memory_usage: self.memory.current_usage_bytes,
            memory_capacity: self.memory.capacity_bytes,
            efficiency_score: self.efficiency_score(),
            top_cache_types: self.top_performing_cache_types(5),
            recommendations: self.optimization_recommendations(),
        }
    }

    /// Get top performing cache types
    fn top_performing_cache_types(&self, limit: usize) -> Vec<CacheTypeReport> {
        let mut types: Vec<CacheTypeReport> = self.by_type.iter()
            .map(|(name, metrics)| CacheTypeReport {
                cache_type: name.clone(),
                hit_ratio: metrics.hit_ratio(),
                total_accesses: metrics.total_hits + metrics.total_misses,
                avg_hit_time: metrics.avg_hit_time(),
                total_size_bytes: metrics.total_size_bytes,
            })
            .collect();
        
        // Sort by hit ratio * access count (weighted performance)
        types.sort_by(|a, b| {
            let score_a = a.hit_ratio * (a.total_accesses as f64).log2();
            let score_b = b.hit_ratio * (b.total_accesses as f64).log2();
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        types.into_iter().take(limit).collect()
    }

    /// Get cache key type name for metrics categorization
    fn key_type_name(&self, key: &CacheKey) -> String {
        match key {
            CacheKey::Spatial(_) => "Spatial".to_string(),
            CacheKey::Query(_) => "Query".to_string(),
            CacheKey::Pathfinding(_) => "Pathfinding".to_string(),
            CacheKey::AI(_) => "AI".to_string(),
            CacheKey::Rendering(_) => "Rendering".to_string(),
            CacheKey::Player(_) => "Player".to_string(),
            CacheKey::Custom(name) => format!("Custom-{}", name),
            CacheKey::Tectonic(_) => "Tectonic".to_string(),
        }
    }

    /// Reset all metrics (for testing or periodic cleanup)
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Get metrics summary as JSON-serializable structure
    pub fn summary(&self) -> MetricsSummary {
        MetricsSummary {
            global_hit_ratio: self.global.hit_ratio(),
            total_accesses: self.global.total_hits + self.global.total_misses,
            memory_usage_mb: self.memory.current_usage_bytes as f64 / (1024.0 * 1024.0),
            memory_capacity_mb: self.memory.capacity_bytes as f64 / (1024.0 * 1024.0),
            efficiency_score: self.efficiency_score(),
            cache_types: self.by_type.len(),
            recommendations_count: self.optimization_recommendations().len(),
        }
    }
}

/// Global cache metrics
#[derive(Debug, Clone)]
pub struct GlobalMetrics {
    pub total_hits: u64,
    pub total_misses: u64,
    pub total_writes: u64,
    pub total_evictions: u64,
    pub total_hit_time: Duration,
    pub total_miss_time: Duration,
    pub total_bytes_written: u64,
    pub total_bytes_evicted: u64,
}

impl GlobalMetrics {
    pub fn new() -> Self {
        Self {
            total_hits: 0,
            total_misses: 0,
            total_writes: 0,
            total_evictions: 0,
            total_hit_time: Duration::ZERO,
            total_miss_time: Duration::ZERO,
            total_bytes_written: 0,
            total_bytes_evicted: 0,
        }
    }

    pub fn record_hit(&mut self, access_time: Duration) {
        self.total_hits += 1;
        self.total_hit_time += access_time;
    }

    pub fn record_miss(&mut self, computation_time: Duration) {
        self.total_misses += 1;
        self.total_miss_time += computation_time;
    }

    pub fn record_write(&mut self, size_bytes: usize) {
        self.total_writes += 1;
        self.total_bytes_written += size_bytes as u64;
    }

    pub fn record_eviction(&mut self, size_bytes: usize) {
        self.total_evictions += 1;
        self.total_bytes_evicted += size_bytes as u64;
    }

    pub fn hit_ratio(&self) -> f64 {
        let total = self.total_hits + self.total_misses;
        if total > 0 {
            self.total_hits as f64 / total as f64
        } else {
            0.0
        }
    }

    pub fn avg_hit_time(&self) -> Duration {
        if self.total_hits > 0 {
            self.total_hit_time / self.total_hits as u32
        } else {
            Duration::ZERO
        }
    }

    pub fn avg_miss_time(&self) -> Duration {
        if self.total_misses > 0 {
            self.total_miss_time / self.total_misses as u32
        } else {
            Duration::ZERO
        }
    }
}

/// Per-cache-type metrics
#[derive(Debug, Clone)]
pub struct TypeMetrics {
    pub total_hits: u64,
    pub total_misses: u64,
    pub total_writes: u64,
    pub total_evictions: u64,
    pub total_hit_time: Duration,
    pub total_miss_time: Duration,
    pub total_size_bytes: u64,
    pub eviction_reasons: FastHashMap<EvictionReason, u32>,
}

impl TypeMetrics {
    pub fn new() -> Self {
        Self {
            total_hits: 0,
            total_misses: 0,
            total_writes: 0,
            total_evictions: 0,
            total_hit_time: Duration::ZERO,
            total_miss_time: Duration::ZERO,
            total_size_bytes: 0,
            eviction_reasons: FastHashMap::default(),
        }
    }

    pub fn record_hit(&mut self, access_time: Duration) {
        self.total_hits += 1;
        self.total_hit_time += access_time;
    }

    pub fn record_miss(&mut self, computation_time: Duration) {
        self.total_misses += 1;
        self.total_miss_time += computation_time;
    }

    pub fn record_write(&mut self, size_bytes: usize) {
        self.total_writes += 1;
        self.total_size_bytes += size_bytes as u64;
    }

    pub fn record_eviction(&mut self, size_bytes: usize, reason: EvictionReason) {
        self.total_evictions += 1;
        self.total_size_bytes = self.total_size_bytes.saturating_sub(size_bytes as u64);
        *self.eviction_reasons.entry(reason).or_insert(0) += 1;
    }

    pub fn hit_ratio(&self) -> f64 {
        let total = self.total_hits + self.total_misses;
        if total > 0 {
            self.total_hits as f64 / total as f64
        } else {
            0.0
        }
    }

    pub fn avg_hit_time(&self) -> Duration {
        if self.total_hits > 0 {
            self.total_hit_time / self.total_hits as u32
        } else {
            Duration::ZERO
        }
    }
}

/// Per-priority metrics
#[derive(Debug, Clone)]
pub struct PriorityMetrics {
    pub total_hits: u64,
    pub total_misses: u64,
    pub total_evictions: u64,
    pub avg_access_time: Duration,
}

impl PriorityMetrics {
    pub fn new() -> Self {
        Self {
            total_hits: 0,
            total_misses: 0,
            total_evictions: 0,
            avg_access_time: Duration::ZERO,
        }
    }

    pub fn record_hit(&mut self, access_time: Duration) {
        self.total_hits += 1;
        self.update_avg_access_time(access_time);
    }

    pub fn record_miss(&mut self, _computation_time: Duration) {
        self.total_misses += 1;
    }

    pub fn hit_ratio(&self) -> f64 {
        let total = self.total_hits + self.total_misses;
        if total > 0 {
            self.total_hits as f64 / total as f64
        } else {
            0.0
        }
    }

    fn update_avg_access_time(&mut self, new_time: Duration) {
        let total_accesses = self.total_hits + self.total_misses;
        if total_accesses > 0 {
            let current_total = self.avg_access_time * (total_accesses - 1) as u32;
            self.avg_access_time = (current_total + new_time) / total_accesses as u32;
        } else {
            self.avg_access_time = new_time;
        }
    }
}

/// Time-series data for trend analysis
#[derive(Debug, Clone)]
pub struct TimeSeries {
    events: Vec<TimeSeriesPoint>,
    max_points: usize,
}

impl TimeSeries {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            max_points: 1000, // Keep last 1000 events
        }
    }

    pub fn record_event(&mut self, event: CacheEvent, key: &CacheKey) {
        let point = TimeSeriesPoint {
            timestamp: Instant::now(),
            event,
            key_type: self.key_type_from_cache_key(key),
            priority: key.priority(),
        };

        self.events.push(point);
        
        // Keep only recent events
        if self.events.len() > self.max_points {
            self.events.drain(0..self.events.len() - self.max_points);
        }
    }

    pub fn hit_ratio_trend(&self, window: Duration) -> Vec<f64> {
        // Calculate hit ratio over sliding time windows
        let cutoff = Instant::now() - window;
        let recent_events: Vec<_> = self.events.iter()
            .filter(|event| event.timestamp > cutoff)
            .collect();
        
        // Simple implementation - could be more sophisticated
        let window_size = recent_events.len() / 10.max(1); // 10 data points
        let mut ratios = Vec::new();
        
        for chunk in recent_events.chunks(window_size) {
            let hits = chunk.iter().filter(|e| e.event == CacheEvent::Hit).count();
            let total = chunk.iter().filter(|e| matches!(e.event, CacheEvent::Hit | CacheEvent::Miss)).count();
            
            if total > 0 {
                ratios.push(hits as f64 / total as f64);
            }
        }
        
        ratios
    }

    fn key_type_from_cache_key(&self, key: &CacheKey) -> String {
        match key {
            CacheKey::Spatial(_) => "Spatial".to_string(),
            CacheKey::Query(_) => "Query".to_string(),
            CacheKey::Pathfinding(_) => "Pathfinding".to_string(),
            CacheKey::AI(_) => "AI".to_string(),
            CacheKey::Rendering(_) => "Rendering".to_string(),
            CacheKey::Player(_) => "Player".to_string(),
            CacheKey::Custom(name) => format!("Custom-{}", name),
            CacheKey::Tectonic(_) => "Tectonic".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TimeSeriesPoint {
    pub timestamp: Instant,
    pub event: CacheEvent,
    pub key_type: String,
    pub priority: CachePriority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheEvent {
    Hit,
    Miss,
    Write,
    Eviction,
}

/// Performance impact metrics
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    total_cache_time: Duration,
    total_computation_time: Duration,
    cache_accesses: u64,
    computations: u64,
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            total_cache_time: Duration::ZERO,
            total_computation_time: Duration::ZERO,
            cache_accesses: 0,
            computations: 0,
        }
    }

    pub fn record_access(&mut self, time: Duration, was_hit: bool) {
        if was_hit {
            self.total_cache_time += time;
            self.cache_accesses += 1;
        } else {
            self.total_computation_time += time;
            self.computations += 1;
        }
    }

    pub fn avg_cache_time(&self) -> Duration {
        if self.cache_accesses > 0 {
            self.total_cache_time / self.cache_accesses as u32
        } else {
            Duration::ZERO
        }
    }

    pub fn avg_computation_time(&self) -> Duration {
        if self.computations > 0 {
            self.total_computation_time / self.computations as u32
        } else {
            Duration::ZERO
        }
    }

    pub fn avg_miss_penalty(&self) -> Duration {
        let cache_time = self.avg_cache_time();
        let comp_time = self.avg_computation_time();
        
        comp_time.saturating_sub(cache_time)
    }

    pub fn efficiency_score(&self) -> f64 {
        let cache_time = self.avg_cache_time().as_nanos() as f64;
        let comp_time = self.avg_computation_time().as_nanos() as f64;
        
        if comp_time > 0.0 {
            1.0 - (cache_time / comp_time).min(1.0)
        } else {
            1.0
        }
    }
}

/// Memory usage tracking
#[derive(Debug, Clone)]
pub struct MemoryMetrics {
    pub current_usage_bytes: u64,
    pub capacity_bytes: u64,
    pub peak_usage_bytes: u64,
    pub allocations: u64,
    pub deallocations: u64,
}

impl MemoryMetrics {
    pub fn new() -> Self {
        Self {
            current_usage_bytes: 0,
            capacity_bytes: 512 * 1024 * 1024, // 512MB default
            peak_usage_bytes: 0,
            allocations: 0,
            deallocations: 0,
        }
    }

    pub fn record_allocation(&mut self, bytes: usize) {
        self.current_usage_bytes += bytes as u64;
        self.allocations += 1;
        self.peak_usage_bytes = self.peak_usage_bytes.max(self.current_usage_bytes);
    }

    pub fn record_deallocation(&mut self, bytes: usize) {
        self.current_usage_bytes = self.current_usage_bytes.saturating_sub(bytes as u64);
        self.deallocations += 1;
    }

    pub fn update_usage(&mut self, current: u64, capacity: u64) {
        self.current_usage_bytes = current;
        self.capacity_bytes = capacity;
        self.peak_usage_bytes = self.peak_usage_bytes.max(current);
    }

    pub fn usage_ratio(&self) -> f64 {
        if self.capacity_bytes > 0 {
            self.current_usage_bytes as f64 / self.capacity_bytes as f64
        } else {
            0.0
        }
    }

    pub fn efficiency_ratio(&self) -> f64 {
        let usage = self.usage_ratio();
        // Efficiency decreases as we approach capacity
        if usage < 0.8 {
            1.0
        } else if usage < 0.9 {
            0.8
        } else {
            0.5
        }
    }
}

/// Eviction reason tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EvictionReason {
    TTLExpired,
    CapacityExceeded,
    ManualInvalidation,
    TurnAdvanced,
    MemoryPressure,
    PolicyEviction,
}

/// Optimization recommendations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationRecommendation {
    pub category: OptimizationCategory,
    pub severity: Severity,
    pub description: String,
    pub suggestion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationCategory {
    HitRatio,
    Memory,
    Performance,
    Configuration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

/// Performance report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceReport {
    pub hit_ratio: f64,
    pub miss_ratio: f64,
    pub avg_hit_time: Duration,
    pub avg_miss_time: Duration,
    pub total_hits: u64,
    pub total_misses: u64,
    pub memory_usage: u64,
    pub memory_capacity: u64,
    pub efficiency_score: f64,
    pub top_cache_types: Vec<CacheTypeReport>,
    pub recommendations: Vec<OptimizationRecommendation>,
}

/// Cache type performance report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheTypeReport {
    pub cache_type: String,
    pub hit_ratio: f64,
    pub total_accesses: u64,
    pub avg_hit_time: Duration,
    pub total_size_bytes: u64,
}

/// Metrics summary for dashboards
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSummary {
    pub global_hit_ratio: f64,
    pub total_accesses: u64,
    pub memory_usage_mb: f64,
    pub memory_capacity_mb: f64,
    pub efficiency_score: f64,
    pub cache_types: usize,
    pub recommendations_count: usize,
}

// ============================================================================
// CACHE PERFORMANCE TRACKING UTILITIES
// ============================================================================

/// Cache performance statistics helper for fine-grained tracking
#[derive(Debug, Clone)]
pub struct CachePerformanceTracker {
    cache_hits: u64,
    cache_misses: u64,
    total_cache_time: Duration,
    total_compute_time: Duration,
}

impl CachePerformanceTracker {
    pub fn new() -> Self {
        Self {
            cache_hits: 0,
            cache_misses: 0,
            total_cache_time: Duration::ZERO,
            total_compute_time: Duration::ZERO,
        }
    }

    pub fn record_hit(&mut self, access_time: Duration) {
        self.cache_hits += 1;
        self.total_cache_time += access_time;
    }

    pub fn record_miss(&mut self, compute_time: Duration) {
        self.cache_misses += 1;
        self.total_compute_time += compute_time;
    }

    pub fn hit_ratio(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total > 0 {
            self.cache_hits as f64 / total as f64
        } else {
            0.0
        }
    }

    pub fn avg_cache_time(&self) -> Duration {
        if self.cache_hits > 0 {
            self.total_cache_time / self.cache_hits as u32
        } else {
            Duration::ZERO
        }
    }

    pub fn avg_compute_time(&self) -> Duration {
        if self.cache_misses > 0 {
            self.total_compute_time / self.cache_misses as u32
        } else {
            Duration::ZERO
        }
    }

    pub fn performance_gain(&self) -> f64 {
        let cache_time = self.avg_cache_time().as_nanos() as f64;
        let compute_time = self.avg_compute_time().as_nanos() as f64;
        
        if compute_time > 0.0 {
            (compute_time - cache_time) / compute_time
        } else {
            0.0
        }
    }
    
    pub fn total_hits(&self) -> u64 {
        self.cache_hits
    }
    
    pub fn total_misses(&self) -> u64 {
        self.cache_misses
    }
}

impl Default for CachePerformanceTracker {
    fn default() -> Self {
        Self::new()
    }
}
