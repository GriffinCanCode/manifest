//! Logging system performance metrics and monitoring
//!
//! Provides comprehensive monitoring of the logging system's performance:
//! - Event throughput and latency tracking
//! - Memory usage monitoring
//! - Filter effectiveness metrics
//! - Appender health monitoring
//! - System-wide logging statistics

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use tracing::{Metadata, Event, Subscriber};
use tracing_subscriber::{Layer, registry::LookupSpan};
use serde::{Serialize, Deserialize};
use crate::core::hashing::{collections, FastHashMap, HashStrategies};
use super::{RateLimitStats, SamplingStats, AppenderStats};

/// Main logging metrics collector
pub struct LoggingMetrics {
    /// Overall system metrics
    system_metrics: Arc<RwLock<SystemMetrics>>,
    /// Per-target metrics (optimized for target string keys)
    target_metrics: Arc<RwLock<FastHashMap<String, TargetMetrics>>>,
    /// Per-level metrics
    level_metrics: Arc<RwLock<[LevelMetrics; 5]>>, // TRACE, DEBUG, INFO, WARN, ERROR
    /// Performance tracking
    performance_tracker: Arc<PerformanceTracker>,
    /// Start time for rate calculations
    start_time: Instant,
}

impl LoggingMetrics {
    pub fn new() -> Self {
        Self {
            system_metrics: Arc::new(RwLock::new(SystemMetrics::default())),
            target_metrics: Arc::new(RwLock::new(collections::fast_hash_map())),
            level_metrics: Arc::new(RwLock::new([
                LevelMetrics::new("TRACE"),
                LevelMetrics::new("DEBUG"), 
                LevelMetrics::new("INFO"),
                LevelMetrics::new("WARN"),
                LevelMetrics::new("ERROR"),
            ])),
            performance_tracker: Arc::new(PerformanceTracker::new()),
            start_time: Instant::now(),
        }
    }
    
    /// Record a log event
    pub fn record_event(&self, metadata: &Metadata, event_size: usize) {
        // Update system metrics
        {
            let mut system = self.system_metrics.write();
            system.total_events.fetch_add(1, Ordering::Relaxed);
            system.total_bytes.fetch_add(event_size.try_into().unwrap_or(0), Ordering::Relaxed);
            system.last_event_time = Some(Instant::now());
        }
        
        // Update per-target metrics
        {
            let mut targets = self.target_metrics.write();
            let target_metric = targets
                .entry(metadata.target().to_string())
                .or_insert_with(|| TargetMetrics::new(metadata.target()));
            target_metric.record_event(event_size);
        }
        
        // Update per-level metrics
        {
            let mut levels = self.level_metrics.write();
            let level_index = match *metadata.level() {
                tracing::Level::TRACE => 0,
                tracing::Level::DEBUG => 1,
                tracing::Level::INFO => 2,
                tracing::Level::WARN => 3,
                tracing::Level::ERROR => 4,
            };
            levels[level_index].record_event(event_size);
        }
        
        // Track performance
        self.performance_tracker.record_event(metadata, event_size);
    }
    
    /// Record a filtered event (event that was not logged due to filtering)
    pub fn record_filtered(&self, metadata: &Metadata, filter_type: FilterType) {
        let mut system = self.system_metrics.write();
        system.filtered_events.fetch_add(1, Ordering::Relaxed);
        
        match filter_type {
            FilterType::Level => system.level_filtered.fetch_add(1, Ordering::Relaxed),
            FilterType::RateLimit => system.rate_limited.fetch_add(1, Ordering::Relaxed),
            FilterType::Sampling => system.sampled_out.fetch_add(1, Ordering::Relaxed),
            FilterType::Sensitive => system.sensitive_filtered.fetch_add(1, Ordering::Relaxed),
        };
    }
    
    /// Get a snapshot of current metrics
    pub fn snapshot(&self) -> LoggingMetricsSnapshot {
        let system = self.system_metrics.read().snapshot();
        let targets = {
            let targets_guard = self.target_metrics.read();
            targets_guard
                .iter()
                .map(|(name, metrics)| (name.clone(), metrics.snapshot()))
                .collect()
        };
        let levels = {
            let levels_guard = self.level_metrics.read();
            levels_guard.iter().map(|l| l.snapshot()).collect()
        };
        let performance = self.performance_tracker.snapshot();
        let uptime = self.start_time.elapsed();
        
        LoggingMetricsSnapshot {
            system,
            targets,
            levels,
            performance,
            uptime,
        }
    }
    
    /// Get current events per second
    pub fn events_per_second(&self) -> f64 {
        let system = self.system_metrics.read();
        let total_events = system.total_events.load(Ordering::Relaxed) as f64;
        let uptime_secs = self.start_time.elapsed().as_secs_f64();
        
        if uptime_secs > 0.0 {
            total_events / uptime_secs
        } else {
            0.0
        }
    }
    
    /// Get memory usage estimate for logging system
    pub fn memory_usage_bytes(&self) -> usize {
        let targets_mem = {
            let targets = self.target_metrics.read();
            targets.len() * 200 // Rough estimate per target
        };
        
        let system_mem = std::mem::size_of::<SystemMetrics>();
        let levels_mem = std::mem::size_of::<[LevelMetrics; 5]>();
        let performance_mem = self.performance_tracker.memory_usage();
        
        targets_mem + system_mem + levels_mem + performance_mem
    }
    
    /// Reset all metrics (useful for testing or periodic resets)
    pub fn reset(&self) {
        {
            let mut system = self.system_metrics.write();
            *system = SystemMetrics::default();
        }
        
        {
            let mut targets = self.target_metrics.write();
            targets.clear();
        }
        
        {
            let mut levels = self.level_metrics.write();
            for (i, level_name) in ["TRACE", "DEBUG", "INFO", "WARN", "ERROR"].iter().enumerate() {
                levels[i] = LevelMetrics::new(level_name);
            }
        }
        
        self.performance_tracker.reset();
    }
}

/// System-wide logging metrics
#[derive(Debug)]
struct SystemMetrics {
    /// Total events processed
    total_events: AtomicU64,
    /// Total bytes processed
    total_bytes: AtomicU64,
    /// Events filtered out by level
    level_filtered: AtomicU64,
    /// Events filtered out by rate limiting
    rate_limited: AtomicU64,
    /// Events filtered out by sampling
    sampled_out: AtomicU64,
    /// Events filtered out for sensitive data
    sensitive_filtered: AtomicU64,
    /// Total filtered events
    filtered_events: AtomicU64,
    /// Last event timestamp
    last_event_time: Option<Instant>,
}

impl Default for SystemMetrics {
    fn default() -> Self {
        Self {
            total_events: AtomicU64::new(0),
            total_bytes: AtomicU64::new(0),
            level_filtered: AtomicU64::new(0),
            rate_limited: AtomicU64::new(0),
            sampled_out: AtomicU64::new(0),
            sensitive_filtered: AtomicU64::new(0),
            filtered_events: AtomicU64::new(0),
            last_event_time: None,
        }
    }
}

impl SystemMetrics {
    fn snapshot(&self) -> SystemMetricsSnapshot {
        SystemMetricsSnapshot {
            total_events: self.total_events.load(Ordering::Relaxed),
            total_bytes: self.total_bytes.load(Ordering::Relaxed),
            level_filtered: self.level_filtered.load(Ordering::Relaxed),
            rate_limited: self.rate_limited.load(Ordering::Relaxed),
            sampled_out: self.sampled_out.load(Ordering::Relaxed),
            sensitive_filtered: self.sensitive_filtered.load(Ordering::Relaxed),
            filtered_events: self.filtered_events.load(Ordering::Relaxed),
            last_event_time: self.last_event_time,
        }
    }
}

/// Per-target logging metrics
#[derive(Debug)]
struct TargetMetrics {
    name: String,
    events: AtomicU64,
    bytes: AtomicU64,
    last_event: Option<Instant>,
    first_event: Option<Instant>,
}

impl TargetMetrics {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            events: AtomicU64::new(0),
            bytes: AtomicU64::new(0),
            last_event: None,
            first_event: None,
        }
    }
    
    fn record_event(&mut self, event_size: usize) {
        self.events.fetch_add(1, Ordering::Relaxed);
        self.bytes.fetch_add(event_size.try_into().unwrap_or(0), Ordering::Relaxed);
        let now = Instant::now();
        self.last_event = Some(now);
        if self.first_event.is_none() {
            self.first_event = Some(now);
        }
    }
    
    fn snapshot(&self) -> TargetMetricsSnapshot {
        TargetMetricsSnapshot {
            name: self.name.clone(),
            events: self.events.load(Ordering::Relaxed),
            bytes: self.bytes.load(Ordering::Relaxed),
            last_event: self.last_event,
            first_event: self.first_event,
        }
    }
}

/// Per-level logging metrics
#[derive(Debug)]
struct LevelMetrics {
    name: String,
    events: AtomicU64,
    bytes: AtomicU64,
}

impl LevelMetrics {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            events: AtomicU64::new(0),
            bytes: AtomicU64::new(0),
        }
    }
    
    fn record_event(&self, event_size: usize) {
        self.events.fetch_add(1, Ordering::Relaxed);
        self.bytes.fetch_add(event_size.try_into().unwrap_or(0), Ordering::Relaxed);
    }
    
    fn snapshot(&self) -> LevelMetricsSnapshot {
        LevelMetricsSnapshot {
            name: self.name.clone(),
            events: self.events.load(Ordering::Relaxed),
            bytes: self.bytes.load(Ordering::Relaxed),
        }
    }
}

/// Performance tracking for logging operations
struct PerformanceTracker {
    /// Histogram of event processing times (in microseconds)
    processing_times: RwLock<Vec<u64>>,
    /// Peak memory usage
    peak_memory: AtomicUsize,
    /// Current queue depth estimate
    queue_depth: AtomicUsize,
    /// Hot targets (frequently logging targets)
    hot_targets: RwLock<FastHashMap<String, HotTargetStats>>,
}

impl PerformanceTracker {
    fn new() -> Self {
        Self {
            processing_times: RwLock::new(Vec::with_capacity(10000)), // Ring buffer style
            peak_memory: AtomicUsize::new(0),
            queue_depth: AtomicUsize::new(0),
            hot_targets: RwLock::new(collections::fast_hash_map()),
        }
    }
    
    fn record_event(&self, metadata: &Metadata, event_size: usize) {
        let start = Instant::now();
        
        // Simulate processing time
        let processing_time = start.elapsed().as_micros() as u64;
        
        // Record processing time
        {
            let mut times = self.processing_times.write();
            if times.len() >= 10000 {
                times.clear(); // Simple ring buffer
            }
            times.push(processing_time);
        }
        
        // Track hot targets
        {
            let mut hot_targets = self.hot_targets.write();
            let stats = hot_targets
                .entry(metadata.target().to_string())
                .or_insert_with(|| HotTargetStats::new(metadata.target()));
            stats.record_event(event_size, processing_time);
        }
    }
    
    fn snapshot(&self) -> PerformanceSnapshot {
        let times = self.processing_times.read();
        let (avg_processing_time, p95_processing_time, p99_processing_time) = if times.is_empty() {
            (0.0, 0, 0)
        } else {
            let mut sorted_times = times.clone();
            sorted_times.sort_unstable();
            
            let avg = sorted_times.iter().sum::<u64>() as f64 / sorted_times.len() as f64;
            let p95_idx = (sorted_times.len() as f64 * 0.95) as usize;
            let p99_idx = (sorted_times.len() as f64 * 0.99) as usize;
            
            let p95 = sorted_times.get(p95_idx).copied().unwrap_or(0);
            let p99 = sorted_times.get(p99_idx).copied().unwrap_or(0);
            
            (avg, p95, p99)
        };
        
        let hot_targets = {
            let hot_targets_guard = self.hot_targets.read();
            hot_targets_guard
                .iter()
                .map(|(name, stats)| (name.clone(), stats.snapshot()))
                .collect()
        };
        
        PerformanceSnapshot {
            avg_processing_time_us: avg_processing_time,
            p95_processing_time_us: p95_processing_time,
            p99_processing_time_us: p99_processing_time,
            peak_memory_bytes: self.peak_memory.load(Ordering::Relaxed),
            current_queue_depth: self.queue_depth.load(Ordering::Relaxed),
            hot_targets,
        }
    }
    
    fn memory_usage(&self) -> usize {
        let times_mem = {
            let times = self.processing_times.read();
            times.len() * std::mem::size_of::<u64>()
        };
        
        let hot_targets_mem = {
            let hot_targets = self.hot_targets.read();
            hot_targets.len() * 300 // Rough estimate per hot target
        };
        
        times_mem + hot_targets_mem + std::mem::size_of::<Self>()
    }
    
    fn reset(&self) {
        self.processing_times.write().clear();
        self.peak_memory.store(0, Ordering::Relaxed);
        self.queue_depth.store(0, Ordering::Relaxed);
        self.hot_targets.write().clear();
    }
}

/// Statistics for frequently logging targets
#[derive(Debug)]
struct HotTargetStats {
    name: String,
    events_count: u64,
    total_bytes: u64,
    total_processing_time_us: u64,
    first_seen: Instant,
    last_seen: Instant,
}

impl HotTargetStats {
    fn new(name: &str) -> Self {
        let now = Instant::now();
        Self {
            name: name.to_string(),
            events_count: 0,
            total_bytes: 0,
            total_processing_time_us: 0,
            first_seen: now,
            last_seen: now,
        }
    }
    
    fn record_event(&mut self, event_size: usize, processing_time_us: u64) {
        self.events_count += 1;
        self.total_bytes += event_size as u64;
        self.total_processing_time_us += processing_time_us;
        self.last_seen = Instant::now();
    }
    
    fn snapshot(&self) -> HotTargetSnapshot {
        HotTargetSnapshot {
            name: self.name.clone(),
            events_count: self.events_count,
            total_bytes: self.total_bytes,
            avg_processing_time_us: if self.events_count > 0 {
                self.total_processing_time_us as f64 / self.events_count as f64
            } else {
                0.0
            },
            events_per_second: {
                let duration = self.last_seen.duration_since(self.first_seen).as_secs_f64();
                if duration > 0.0 {
                    self.events_count as f64 / duration
                } else {
                    0.0
                }
            },
            first_seen: self.first_seen,
            last_seen: self.last_seen,
        }
    }
}

/// Filter type enumeration for metrics
#[derive(Debug, Clone, Copy)]
pub enum FilterType {
    Level,
    RateLimit,
    Sampling,
    Sensitive,
}

/// Metrics layer that integrates with tracing
pub struct MetricsLayer {
    metrics: Arc<LoggingMetrics>,
}

impl MetricsLayer {
    pub fn new(metrics: Arc<LoggingMetrics>) -> Self {
        Self { metrics }
    }
}

impl<S> Layer<S> for MetricsLayer
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    fn on_event(&self, event: &Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        let metadata = event.metadata();
        // Estimate event size (rough approximation)
        let event_size = metadata.name().len() + metadata.target().len() + 50; // Base overhead
        
        self.metrics.record_event(metadata, event_size);
    }
}

/// Serializable snapshot of all logging metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingMetricsSnapshot {
    pub system: SystemMetricsSnapshot,
    pub targets: FastHashMap<String, TargetMetricsSnapshot>,
    pub levels: Vec<LevelMetricsSnapshot>,
    pub performance: PerformanceSnapshot,
    pub uptime: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetricsSnapshot {
    pub total_events: u64,
    pub total_bytes: u64,
    pub level_filtered: u64,
    pub rate_limited: u64,
    pub sampled_out: u64,
    pub sensitive_filtered: u64,
    pub filtered_events: u64,
    #[serde(skip)]
    pub last_event_time: Option<Instant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetMetricsSnapshot {
    pub name: String,
    pub events: u64,
    pub bytes: u64,
    #[serde(skip)]
    pub last_event: Option<Instant>,
    #[serde(skip)]
    pub first_event: Option<Instant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelMetricsSnapshot {
    pub name: String,
    pub events: u64,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSnapshot {
    pub avg_processing_time_us: f64,
    pub p95_processing_time_us: u64,
    pub p99_processing_time_us: u64,
    pub peak_memory_bytes: usize,
    pub current_queue_depth: usize,
    pub hot_targets: FastHashMap<String, HotTargetSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HotTargetSnapshot {
    pub name: String,
    pub events_count: u64,
    pub total_bytes: u64,
    pub avg_processing_time_us: f64,
    pub events_per_second: f64,
    #[serde(skip)]
    pub first_seen: Instant,
    #[serde(skip)]
    pub last_seen: Instant,
}

impl Default for HotTargetSnapshot {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            name: String::new(),
            events_count: 0,
            total_bytes: 0,
            avg_processing_time_us: 0.0,
            events_per_second: 0.0,
            first_seen: now,
            last_seen: now,
        }
    }
}

impl LoggingMetricsSnapshot {
    /// Get overall logging health score (0.0 = poor, 1.0 = excellent)
    pub fn health_score(&self) -> f64 {
        let mut score = 1.0;
        
        // Penalize high filter rates
        let total_attempted = self.system.total_events + self.system.filtered_events;
        if total_attempted > 0 {
            let filter_rate = self.system.filtered_events as f64 / total_attempted as f64;
            if filter_rate > 0.5 {
                score -= (filter_rate - 0.5) * 0.5; // Max penalty 0.25 for 100% filter rate
            }
        }
        
        // Penalize very high processing times
        if self.performance.p99_processing_time_us > 10_000 {  // > 10ms
            let penalty = (self.performance.p99_processing_time_us as f64 - 10_000.0) / 100_000.0;
            score -= penalty.min(0.25);
        }
        
        // Penalize high memory usage
        if self.performance.peak_memory_bytes > 100 * 1024 * 1024 { // > 100MB
            score -= 0.1;
        }
        
        score.max(0.0)
    }
    
    /// Get the most active targets
    pub fn top_targets(&self, limit: usize) -> Vec<(String, u64)> {
        let mut targets: Vec<_> = self.targets
            .iter()
            .map(|(name, metrics)| (name.clone(), metrics.events))
            .collect();
        
        targets.sort_by(|a, b| b.1.cmp(&a.1));
        targets.into_iter().take(limit).collect()
    }
    
    /// Generate a summary report
    pub fn summary(&self) -> String {
        format!(
            "Logging System Summary:\n\
             - Total Events: {} ({:.2} MB)\n\
             - Events/sec: {:.2}\n\
             - Filtered: {} ({:.1}%)\n\
             - Avg Processing: {:.2}μs\n\
             - Health Score: {:.2}\n\
             - Active Targets: {}",
            self.system.total_events,
            self.system.total_bytes as f64 / (1024.0 * 1024.0),
            self.system.total_events as f64 / self.uptime.as_secs_f64().max(1.0),
            self.system.filtered_events,
            if self.system.total_events > 0 {
                (self.system.filtered_events as f64 / self.system.total_events as f64) * 100.0
            } else { 0.0 },
            self.performance.avg_processing_time_us,
            self.health_score(),
            self.targets.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing::Level;
    
    #[test]
    fn test_metrics_creation() {
        let metrics = LoggingMetrics::new();
        let snapshot = metrics.snapshot();
        
        assert_eq!(snapshot.system.total_events, 0);
        assert_eq!(snapshot.targets.len(), 0);
        assert_eq!(snapshot.levels.len(), 5);
    }
    
    #[test]
    fn test_event_recording() {
        let metrics = LoggingMetrics::new();
        
        let metadata = tracing::metadata! {
            name: "test",
            target: "test::module",
            level: Level::INFO,
            fields: &[],
            callsite: tracing::callsite! {
                name: "test",
                kind: tracing::metadata::Kind::SPAN,
                target: "test::module", 
                level: Level::INFO,
                fields: &[],
                location: &tracing::Location::caller(),
            },
            kind: tracing::metadata::Kind::SPAN,
        };
        
        metrics.record_event(&metadata, 100);
        
        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.system.total_events, 1);
        assert_eq!(snapshot.system.total_bytes, 100);
        assert_eq!(snapshot.targets.len(), 1);
        assert!(snapshot.targets.contains_key("test::module"));
    }
    
    #[test]
    fn test_filter_recording() {
        let metrics = LoggingMetrics::new();
        
        let metadata = tracing::metadata! {
            name: "test",
            target: "test::module",
            level: Level::DEBUG,
            fields: &[],
            callsite: tracing::callsite! {
                name: "test",
                kind: tracing::metadata::Kind::SPAN,
                target: "test::module",
                level: Level::DEBUG,
                fields: &[],
                location: &tracing::Location::caller(),
            },
            kind: tracing::metadata::Kind::SPAN,
        };
        
        metrics.record_filtered(&metadata, FilterType::Level);
        metrics.record_filtered(&metadata, FilterType::RateLimit);
        
        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.system.filtered_events, 2);
        assert_eq!(snapshot.system.level_filtered, 1);
        assert_eq!(snapshot.system.rate_limited, 1);
    }
    
    #[test]
    fn test_health_score() {
        let mut snapshot = LoggingMetricsSnapshot {
            system: SystemMetricsSnapshot {
                total_events: 100,
                total_bytes: 10000,
                level_filtered: 0,
                rate_limited: 0,
                sampled_out: 0,
                sensitive_filtered: 0,
                filtered_events: 0,
                last_event_time: None,
            },
            targets: collections::fast_hash_map(),
            levels: vec![],
            performance: PerformanceSnapshot {
                avg_processing_time_us: 100.0,
                p95_processing_time_us: 500,
                p99_processing_time_us: 1000,
                peak_memory_bytes: 1024 * 1024, // 1MB
                current_queue_depth: 0,
                hot_targets: collections::fast_hash_map(),
            },
            uptime: Duration::from_secs(60),
        };
        
        // Perfect health
        assert_eq!(snapshot.health_score(), 1.0);
        
        // High filter rate should reduce score
        snapshot.system.filtered_events = 150; // 60% filter rate
        let score_with_filters = snapshot.health_score();
        assert!(score_with_filters < 1.0);
        assert!(score_with_filters > 0.8);
    }
    
    #[test]
    fn test_metrics_reset() {
        let metrics = LoggingMetrics::new();
        
        let metadata = tracing::metadata! {
            name: "test",
            target: "test::module",
            level: Level::INFO,
            fields: &[],
            callsite: tracing::callsite! {
                name: "test",
                kind: tracing::metadata::Kind::SPAN,
                target: "test::module",
                level: Level::INFO,
                fields: &[],
                location: &tracing::Location::caller(),
            },
            kind: tracing::metadata::Kind::SPAN,
        };
        
        metrics.record_event(&metadata, 100);
        
        let snapshot_before = metrics.snapshot();
        assert_eq!(snapshot_before.system.total_events, 1);
        
        metrics.reset();
        
        let snapshot_after = metrics.snapshot();
        assert_eq!(snapshot_after.system.total_events, 0);
        assert_eq!(snapshot_after.targets.len(), 0);
    }
}
