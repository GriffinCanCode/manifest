//! High-performance logging filters for game-specific needs
//!
//! Provides sophisticated filtering capabilities with minimal runtime overhead,
//! including sampling, rate limiting, and context-aware filtering.

use std::sync::Arc;
use parking_lot::RwLock;
use tracing::{Level, Metadata, subscriber::Interest};
use tracing_subscriber::{filter::FilterFn, layer::{Context, Filter}, Layer};
use std::time::{Duration, Instant};
use crate::core::hashing::{collections, FastHashMap, FastHashSet, HashStrategies};
use super::{LoggingConfig, LoggingError};

/// Advanced filter layer that combines multiple filtering strategies
pub struct CustomFilterLayer {
    config: Arc<RwLock<LoggingConfig>>,
    rate_limiter: Arc<RateLimiter>,
    sampler: Arc<Sampler>,
    sensitive_filter: Arc<SensitiveDataFilter>,
}

impl CustomFilterLayer {
    pub fn new(config: Arc<RwLock<LoggingConfig>>) -> Self {
        let rate_limiter = Arc::new(RateLimiter::new());
        let sampler = Arc::new(Sampler::new());
        let sensitive_filter = Arc::new(SensitiveDataFilter::new());
        
        Self {
            config,
            rate_limiter,
            sampler,
            sensitive_filter,
        }
    }
    
    /// Check if a log event should be allowed through all filters
    fn should_allow(&self, metadata: &Metadata<'_>) -> bool {
        let config = self.config.read();
        
        // Check level filtering
        let target_level = config.level_for_module(metadata.target());
        let required_level: Level = target_level.parse().unwrap_or(Level::INFO);
        
        if *metadata.level() > required_level {
            return false;
        }
        
        // Apply rate limiting for high-frequency events
        if self.is_high_frequency_target(metadata.target()) {
            if !self.rate_limiter.allow(metadata.target()) {
                return false;
            }
        }
        
        // Apply sampling for performance events
        if self.is_performance_target(metadata.target()) {
            if !self.sampler.should_sample(&config, metadata.target()) {
                return false;
            }
        }
        
        true
    }
    
    fn is_high_frequency_target(&self, target: &str) -> bool {
        target.starts_with("game::performance") ||
        target.starts_with("game::spatial") ||
        target.contains("hot_path")
    }
    
    fn is_performance_target(&self, target: &str) -> bool {
        target.starts_with("game::performance") ||
        target.contains("::performance")
    }
}

impl<S> Filter<S> for CustomFilterLayer {
    fn enabled(&self, meta: &Metadata<'_>, _ctx: &Context<'_, S>) -> bool {
        self.should_allow(meta)
    }
    
    fn callsite_enabled(&self, meta: &Metadata<'_>) -> Interest {
        if self.should_allow(meta) {
            Interest::always()
        } else {
            Interest::never()
        }
    }
    
    fn max_level_hint(&self) -> Option<tracing::level_filters::LevelFilter> {
        let config = self.config.read();
        match config.default_level.as_str() {
            "trace" => Some(tracing::level_filters::LevelFilter::TRACE),
            "debug" => Some(tracing::level_filters::LevelFilter::DEBUG),
            "info" => Some(tracing::level_filters::LevelFilter::INFO),
            "warn" => Some(tracing::level_filters::LevelFilter::WARN),
            "error" => Some(tracing::level_filters::LevelFilter::ERROR),
            _ => Some(tracing::level_filters::LevelFilter::INFO),
        }
    }
}

/// Rate limiter to prevent log flooding from high-frequency events
pub struct RateLimiter {
    /// Rate limits per target (optimized for string keys)
    limits: RwLock<FastHashMap<String, RateLimitState>>,
}

#[derive(Debug, Clone)]
struct RateLimitState {
    last_allowed: Instant,
    count: u64,
    interval: Duration,
    max_per_interval: u64,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            limits: RwLock::new(collections::fast_hash_map()),
        }
    }
    
    /// Check if a log from the given target should be allowed
    pub fn allow(&self, target: &str) -> bool {
        let now = Instant::now();
        let mut limits = self.limits.write();
        
        // Get or create rate limit state for this target
        let state = limits
            .entry(target.to_string())
            .or_insert_with(|| self.default_rate_limit_for_target(target));
        
        // Reset counter if interval has passed
        if now.duration_since(state.last_allowed) >= state.interval {
            state.count = 0;
            state.last_allowed = now;
        }
        
        // Check if we're under the limit
        if state.count < state.max_per_interval {
            state.count += 1;
            true
        } else {
            false
        }
    }
    
    fn default_rate_limit_for_target(&self, target: &str) -> RateLimitState {
        let (interval, max_per_interval) = match target {
            t if t.starts_with("game::performance") => (Duration::from_secs(1), 100),
            t if t.starts_with("game::spatial") => (Duration::from_secs(1), 500),
            t if t.contains("hot_path") => (Duration::from_secs(1), 10),
            t if t.contains("error") => (Duration::from_secs(1), 1000), // Allow more errors
            _ => (Duration::from_secs(1), 1000), // Default: 1000 logs per second
        };
        
        RateLimitState {
            last_allowed: Instant::now(),
            count: 0,
            interval,
            max_per_interval,
        }
    }
    
    /// Get current rate limit statistics
    pub fn stats(&self) -> RateLimitStats {
        let limits = self.limits.read();
        
        RateLimitStats {
            total_targets: limits.len(),
            active_limits: limits
                .values()
                .filter(|state| state.count > 0)
                .count(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RateLimitStats {
    pub total_targets: usize,
    pub active_limits: usize,
}

/// Intelligent sampling for high-volume logging
pub struct Sampler {
    /// Per-target sampling state (optimized for string keys)
    states: RwLock<FastHashMap<String, SamplingState>>,
}

#[derive(Debug, Clone)]
struct SamplingState {
    counter: u64,
    last_sampled: Instant,
    sample_rate: f64,
    hash_seed: u64,
}

impl Sampler {
    pub fn new() -> Self {
        Self {
            states: RwLock::new(collections::fast_hash_map()),
        }
    }
    
    /// Determine if this log should be sampled based on configuration
    pub fn should_sample(&self, config: &LoggingConfig, target: &str) -> bool {
        // Always sample errors and warnings
        if target.contains("error") || target.contains("warn") {
            return true;
        }
        
        let mut states = self.states.write();
        let state = states
            .entry(target.to_string())
            .or_insert_with(|| self.initial_sampling_state(target, config));
        
        state.counter += 1;
        
        // Use deterministic sampling based on hash
        let hash = HashStrategies::combine_hashes(&[
            state.hash_seed,
            HashStrategies::hash_string(target),
            state.counter,
        ]);
        
        let normalized_hash = (hash as f64) / (u64::MAX as f64);
        normalized_hash < state.sample_rate
    }
    
    fn initial_sampling_state(&self, target: &str, config: &LoggingConfig) -> SamplingState {
        let base_rate = config.performance.sampling_rate;
        
        // Adjust sampling rate based on target
        let sample_rate = match target {
            t if t.starts_with("game::performance") => base_rate,
            t if t.starts_with("game::entities") => base_rate * 2.0, // Sample more entity events
            t if t.starts_with("game::spatial") => base_rate * 0.5, // Sample fewer spatial events
            t if t.contains("hot_path") => base_rate * 0.1, // Very aggressive sampling
            _ => base_rate,
        }.min(1.0);
        
        SamplingState {
            counter: 0,
            last_sampled: Instant::now(),
            sample_rate,
            hash_seed: HashStrategies::hash_string(target),
        }
    }
    
    /// Get current sampling statistics
    pub fn stats(&self) -> SamplingStats {
        let states = self.states.read();
        
        let total_events: u64 = states.values().map(|s| s.counter).sum();
        let avg_sample_rate = states
            .values()
            .map(|s| s.sample_rate)
            .sum::<f64>() / states.len().max(1) as f64;
        
        SamplingStats {
            total_targets: states.len(),
            total_events,
            avg_sample_rate,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SamplingStats {
    pub total_targets: usize,
    pub total_events: u64,
    pub avg_sample_rate: f64,
}

/// Filter for sensitive data in production environments
pub struct SensitiveDataFilter {
    /// Patterns that should be redacted (optimized for fast lookups)
    sensitive_patterns: FastHashSet<String>,
    /// Fields that should be filtered out entirely
    sensitive_fields: FastHashSet<String>,
}

impl SensitiveDataFilter {
    pub fn new() -> Self {
        let mut sensitive_patterns = collections::fast_hash_set();
        sensitive_patterns.insert("password".to_string());
        sensitive_patterns.insert("token".to_string());
        sensitive_patterns.insert("secret".to_string());
        sensitive_patterns.insert("key".to_string());
        sensitive_patterns.insert("auth".to_string());
        sensitive_patterns.insert("credential".to_string());
        sensitive_patterns.insert("api_key".to_string());
        
        let mut sensitive_fields = collections::fast_hash_set();
        sensitive_fields.insert("password".to_string());
        sensitive_fields.insert("password_hash".to_string());
        sensitive_fields.insert("access_token".to_string());
        sensitive_fields.insert("refresh_token".to_string());
        sensitive_fields.insert("api_key".to_string());
        sensitive_fields.insert("secret_key".to_string());
        
        Self {
            sensitive_patterns,
            sensitive_fields,
        }
    }
    
    /// Check if a field name should be filtered
    pub fn should_filter_field(&self, field_name: &str) -> bool {
        let lower_field = field_name.to_lowercase();
        
        // Check exact matches
        if self.sensitive_fields.contains(&lower_field) {
            return true;
        }
        
        // Check patterns
        for pattern in &self.sensitive_patterns {
            if lower_field.contains(pattern) {
                return true;
            }
        }
        
        false
    }
    
    /// Redact sensitive data from a string
    pub fn redact_sensitive_data(&self, input: &str) -> String {
        let mut result = input.to_string();
        
        for pattern in &self.sensitive_patterns {
            // Simple pattern matching - in production you might want regex
            if let Some(pos) = result.to_lowercase().find(pattern) {
                let end = pos + pattern.len();
                if let Some(value_start) = result[end..].find('=').or_else(|| result[end..].find(':')) {
                    let actual_start = end + value_start + 1;
                    if let Some(value_end) = result[actual_start..].find(' ').or_else(|| result[actual_start..].find(',')) {
                        let actual_end = actual_start + value_end;
                        result.replace_range(actual_start..actual_end, "[REDACTED]");
                    }
                }
            }
        }
        
        result
    }
}

/// Context-aware filter that considers system state
pub struct ContextualFilter {
    /// Current game state information
    game_context: RwLock<GameContext>,
}

#[derive(Debug, Clone, Default)]
struct GameContext {
    is_paused: bool,
    current_turn: u32,
    active_players: usize,
    performance_mode: bool,
}

impl ContextualFilter {
    pub fn new() -> Self {
        Self {
            game_context: RwLock::new(GameContext::default()),
        }
    }
    
    /// Update game context for filtering decisions
    pub fn update_context(&self, is_paused: bool, turn: u32, players: usize, performance_mode: bool) {
        let mut context = self.game_context.write();
        context.is_paused = is_paused;
        context.current_turn = turn;
        context.active_players = players;
        context.performance_mode = performance_mode;
    }
    
    /// Check if a log should be allowed based on current context
    pub fn should_allow_contextual(&self, metadata: &Metadata<'_>) -> bool {
        let context = self.game_context.read();
        
        // In performance mode, be more aggressive about filtering
        if context.performance_mode {
            match *metadata.level() {
                Level::TRACE | Level::DEBUG => {
                    // Only allow debug/trace from critical systems
                    metadata.target().starts_with("game::performance") ||
                    metadata.target().contains("error")
                },
                _ => true,
            }
        }
        // When paused, we can afford more verbose logging
        else if context.is_paused {
            true
        }
        // During active gameplay, filter more aggressively
        else {
            match *metadata.level() {
                Level::TRACE => {
                    // Only allow traces from performance monitoring
                    metadata.target().starts_with("game::performance")
                },
                Level::DEBUG => {
                    // Allow debug from important systems
                    metadata.target().starts_with("game::entities") ||
                    metadata.target().starts_with("game::ecs") ||
                    metadata.target().contains("error")
                },
                _ => true,
            }
        }
    }
}

/// Performance-optimized filter functions
pub mod filter_functions {
    use super::*;
    
    /// Create a filter function for entity operations
    pub fn entity_operation_filter() -> FilterFn<impl Fn(&Metadata<'_>) -> bool> {
        FilterFn::new(|meta| {
            // Only log entity operations if they affect multiple entities or are errors
            if meta.target() == "game::entities" {
                // This would need to inspect the actual log fields in a real implementation
                // For now, we'll use level as a proxy
                *meta.level() <= Level::INFO
            } else {
                true
            }
        })
    }
    
    /// Create a filter function for spatial operations
    pub fn spatial_operation_filter(min_radius: u32) -> FilterFn<impl Fn(&Metadata<'_>) -> bool> {
        FilterFn::new(move |meta| {
            if meta.target() == "game::spatial" {
                // In a real implementation, we'd check the radius field
                // For now, only allow INFO and above for spatial operations
                *meta.level() <= Level::INFO
            } else {
                true
            }
        })
    }
    
    /// Create a filter function for performance events
    pub fn performance_filter(min_duration_ms: f64) -> FilterFn<impl Fn(&Metadata<'_>) -> bool> {
        FilterFn::new(move |meta| {
            if meta.target().starts_with("game::performance") {
                // In a real implementation, we'd check the duration field
                // For now, allow all performance logs through
                true
            } else {
                true
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_rate_limiter() {
        let limiter = RateLimiter::new();
        
        // Should allow initial requests
        assert!(limiter.allow("test::target"));
        
        // Should eventually rate limit
        let mut allowed_count = 0;
        for _ in 0..2000 {
            if limiter.allow("game::performance::test") {
                allowed_count += 1;
            }
        }
        
        // Should have rate limited (less than 2000 allowed)
        assert!(allowed_count < 2000);
        assert!(allowed_count > 0);
    }
    
    #[test]
    fn test_sampler() {
        let sampler = Sampler::new();
        let config = LoggingConfig::default();
        
        let mut sampled_count = 0;
        for _ in 0..1000 {
            if sampler.should_sample(&config, "game::performance::test") {
                sampled_count += 1;
            }
        }
        
        // Should have sampled some but not all (based on sampling rate)
        assert!(sampled_count > 0);
        assert!(sampled_count < 1000);
    }
    
    #[test]
    fn test_sensitive_data_filter() {
        let filter = SensitiveDataFilter::new();
        
        assert!(filter.should_filter_field("password"));
        assert!(filter.should_filter_field("api_key"));
        assert!(filter.should_filter_field("user_password"));
        assert!(!filter.should_filter_field("username"));
        
        let input = "login successful: username=john password=secret123 token=abc";
        let redacted = filter.redact_sensitive_data(input);
        assert!(redacted.contains("[REDACTED]"));
        assert!(!redacted.contains("secret123"));
    }
    
    #[test]
    fn test_contextual_filter() {
        let filter = ContextualFilter::new();
        
        // Update to performance mode
        filter.update_context(false, 100, 4, true);
        
        let debug_meta = tracing::metadata! {
            name: "test",
            target: "game::entities",
            level: Level::DEBUG,
            fields: &[],
            callsite: tracing::callsite! {
                name: "test",
                kind: tracing::metadata::Kind::SPAN,
                target: "game::entities",
                level: Level::DEBUG,
                fields: &[],
                location: &tracing::Location::caller(),
            },
            kind: tracing::metadata::Kind::SPAN,
        };
        
        // Should filter debug logs in performance mode unless from critical systems
        assert!(!filter.should_allow_contextual(&debug_meta));
        
        // Update to paused state
        filter.update_context(true, 100, 4, false);
        
        // Should allow debug logs when paused
        assert!(filter.should_allow_contextual(&debug_meta));
    }
}
