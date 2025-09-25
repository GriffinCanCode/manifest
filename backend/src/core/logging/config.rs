//! Logging system configuration with hot-reload support
//!
//! Provides comprehensive configuration for all aspects of the logging system,
//! including performance tuning, output destinations, and runtime adjustment.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use crate::core::hashing::{collections, FastHashMap};
use super::LoggingError;

/// Main logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Default log level for all modules
    pub default_level: String,
    
    /// Per-module log level overrides (optimized for fast lookups)
    #[serde(
        serialize_with = "serialize_module_levels",
        deserialize_with = "deserialize_module_levels"
    )]
    pub module_levels: FastHashMap<String, String>,
    
    /// Console output configuration
    pub console: ConsoleConfig,
    
    /// File output configurations
    pub files: Vec<FileConfig>,
    
    /// Performance and optimization settings
    pub performance: PerformanceConfig,
    
    /// Game-specific logging settings
    pub game: GameLoggingConfig,
    
    /// Development/debug settings
    pub debug: DebugConfig,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        let mut module_levels = collections::fast_hash_map();
        
        // Set sensible defaults for game modules
        module_levels.insert("game::entities".to_string(), "info".to_string());
        module_levels.insert("game::spatial".to_string(), "debug".to_string());
        module_levels.insert("game::performance".to_string(), "trace".to_string());
        module_levels.insert("game::ecs".to_string(), "info".to_string());
        module_levels.insert("manifest::core".to_string(), "info".to_string());
        module_levels.insert("manifest::ecs".to_string(), "info".to_string());
        
        Self {
            default_level: "info".to_string(),
            module_levels,
            console: ConsoleConfig::default(),
            files: vec![
                FileConfig::new_game_log(),
                FileConfig::new_error_log(),
                FileConfig::new_performance_log(),
            ],
            performance: PerformanceConfig::default(),
            game: GameLoggingConfig::default(),
            debug: DebugConfig::default(),
        }
    }
}

/// Console output configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleConfig {
    /// Enable console output
    pub enabled: bool,
    
    /// Use colored output
    pub colored: bool,
    
    /// Show timestamps
    pub timestamps: bool,
    
    /// Show module paths
    pub show_module: bool,
    
    /// Show thread names
    pub show_thread: bool,
    
    /// Compact format for production
    pub compact: bool,
    
    /// Filter sensitive information in production
    pub filter_sensitive: bool,
}

impl Default for ConsoleConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            colored: cfg!(debug_assertions),
            timestamps: true,
            show_module: cfg!(debug_assertions),
            show_thread: cfg!(debug_assertions),
            compact: !cfg!(debug_assertions),
            filter_sensitive: !cfg!(debug_assertions),
        }
    }
}

/// File output configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileConfig {
    /// Enable this file appender
    pub enabled: bool,
    
    /// File path (supports templates like {date}, {pid})
    pub path: PathBuf,
    
    /// Log level filter for this file
    pub level: String,
    
    /// Target filters (module names)
    pub targets: Vec<String>,
    
    /// File rotation settings
    pub rotation: RotationConfig,
    
    /// File format settings
    pub format: FileFormatConfig,
    
    /// Buffer size for async writes
    pub buffer_size: usize,
    
    /// Flush interval in milliseconds
    pub flush_interval_ms: u64,
}

impl FileConfig {
    /// Create configuration for main game log
    pub fn new_game_log() -> Self {
        Self {
            enabled: true,
            path: PathBuf::from("logs/manifest-{date}.log"),
            level: "info".to_string(),
            targets: vec![
                "game".to_string(),
                "manifest".to_string(),
            ],
            rotation: RotationConfig::daily(),
            format: FileFormatConfig::structured(),
            buffer_size: 64 * 1024, // 64KB buffer
            flush_interval_ms: 1000, // 1 second
        }
    }
    
    /// Create configuration for error log
    pub fn new_error_log() -> Self {
        Self {
            enabled: true,
            path: PathBuf::from("logs/errors-{date}.log"),
            level: "error".to_string(),
            targets: vec![], // All modules
            rotation: RotationConfig::daily(),
            format: FileFormatConfig::detailed(),
            buffer_size: 32 * 1024, // 32KB buffer
            flush_interval_ms: 500, // 0.5 seconds for errors
        }
    }
    
    /// Create configuration for performance log
    pub fn new_performance_log() -> Self {
        Self {
            enabled: cfg!(feature = "performance-logging"),
            path: PathBuf::from("logs/performance-{date}.log"),
            level: "trace".to_string(),
            targets: vec![
                "game::performance".to_string(),
                "manifest::core::scheduler".to_string(),
            ],
            rotation: RotationConfig::size_based(100 * 1024 * 1024), // 100MB
            format: FileFormatConfig::json(),
            buffer_size: 128 * 1024, // 128KB buffer for high throughput
            flush_interval_ms: 5000, // 5 seconds
        }
    }
}

/// File rotation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationConfig {
    /// Rotation strategy
    pub strategy: RotationStrategy,
    
    /// Maximum number of archived files to keep
    pub max_archives: usize,
    
    /// Compress archived files
    pub compress: bool,
    
    /// Cleanup old files after this many days
    pub cleanup_days: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RotationStrategy {
    /// Rotate daily at midnight
    Daily,
    /// Rotate hourly
    Hourly,
    /// Rotate when file exceeds size (bytes)
    Size(u64),
    /// Rotate after time period (seconds)
    Time(u64),
    /// No rotation
    Never,
}

impl RotationConfig {
    pub fn daily() -> Self {
        Self {
            strategy: RotationStrategy::Daily,
            max_archives: 30, // Keep 30 days
            compress: true,
            cleanup_days: Some(90), // Clean up after 90 days
        }
    }
    
    pub fn size_based(max_size: u64) -> Self {
        Self {
            strategy: RotationStrategy::Size(max_size),
            max_archives: 10,
            compress: true,
            cleanup_days: Some(30),
        }
    }
    
    pub fn hourly() -> Self {
        Self {
            strategy: RotationStrategy::Hourly,
            max_archives: 24 * 7, // Keep 7 days
            compress: true,
            cleanup_days: Some(14),
        }
    }
}

/// File format configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileFormatConfig {
    /// Output format type
    pub format_type: FileFormatType,
    
    /// Include timestamps
    pub timestamps: bool,
    
    /// Include correlation IDs
    pub correlation_ids: bool,
    
    /// Include thread information
    pub thread_info: bool,
    
    /// Include source location (file, line)
    pub source_location: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileFormatType {
    /// Human-readable plain text
    Plain,
    /// Structured JSON output
    Json,
    /// Logfmt format (key=value pairs)
    Logfmt,
    /// Custom game format optimized for analysis
    GameFormat,
}

impl FileFormatConfig {
    pub fn structured() -> Self {
        Self {
            format_type: FileFormatType::Logfmt,
            timestamps: true,
            correlation_ids: true,
            thread_info: false,
            source_location: cfg!(debug_assertions),
        }
    }
    
    pub fn json() -> Self {
        Self {
            format_type: FileFormatType::Json,
            timestamps: true,
            correlation_ids: true,
            thread_info: true,
            source_location: true,
        }
    }
    
    pub fn detailed() -> Self {
        Self {
            format_type: FileFormatType::Plain,
            timestamps: true,
            correlation_ids: true,
            thread_info: true,
            source_location: true,
        }
    }
}

/// Performance-related configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Use async logging for better performance
    pub async_logging: bool,
    
    /// Use structured logging (more overhead but better analysis)
    pub structured_logs: bool,
    
    /// Maximum queue size for async logging
    pub async_queue_size: usize,
    
    /// Enable logging metrics collection
    pub enable_metrics: bool,
    
    /// Sampling rate for high-frequency events (0.0-1.0)
    pub sampling_rate: f64,
    
    /// Drop logs instead of blocking when queue is full
    pub drop_on_full_queue: bool,
    
    /// Minimum duration (ms) to log performance events
    pub min_performance_duration_ms: f64,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            async_logging: true,
            structured_logs: true,
            async_queue_size: 10000,
            enable_metrics: true,
            sampling_rate: if cfg!(debug_assertions) { 1.0 } else { 0.1 },
            drop_on_full_queue: !cfg!(debug_assertions),
            min_performance_duration_ms: 0.1,
        }
    }
}

/// Game-specific logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameLoggingConfig {
    /// Log entity creation/destruction
    pub log_entity_lifecycle: bool,
    
    /// Log spatial operations (movement, queries)
    pub log_spatial_operations: bool,
    
    /// Log ECS system execution times
    pub log_system_performance: bool,
    
    /// Log archetype operations
    pub log_archetype_operations: bool,
    
    /// Log player actions
    pub log_player_actions: bool,
    
    /// Log AI decision making
    pub log_ai_decisions: bool,
    
    /// Minimum entity count to log batch operations
    pub min_batch_size: usize,
    
    /// Log save/load operations
    pub log_save_load: bool,
    
    /// Log hot reload events
    pub log_hot_reload: bool,
}

impl Default for GameLoggingConfig {
    fn default() -> Self {
        Self {
            log_entity_lifecycle: cfg!(debug_assertions),
            log_spatial_operations: cfg!(debug_assertions),
            log_system_performance: true,
            log_archetype_operations: cfg!(debug_assertions),
            log_player_actions: true,
            log_ai_decisions: cfg!(debug_assertions),
            min_batch_size: 10,
            log_save_load: true,
            log_hot_reload: cfg!(debug_assertions),
        }
    }
}

/// Debug and development configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugConfig {
    /// Enable trace-level logging in debug builds
    pub verbose_debug: bool,
    
    /// Log memory allocations (very verbose)
    pub log_allocations: bool,
    
    /// Enable console subscriber for tokio debugging
    pub tokio_console: bool,
    
    /// Pretty-print structured logs in console
    pub pretty_console: bool,
    
    /// Include full backtraces in error logs
    pub full_backtraces: bool,
    
    /// Maximum depth for nested span contexts
    pub max_span_depth: usize,
}

impl Default for DebugConfig {
    fn default() -> Self {
        Self {
            verbose_debug: cfg!(debug_assertions),
            log_allocations: false,
            tokio_console: cfg!(debug_assertions),
            pretty_console: cfg!(debug_assertions),
            full_backtraces: cfg!(debug_assertions),
            max_span_depth: 32,
        }
    }
}

impl LoggingConfig {
    /// Load configuration from file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, LoggingError> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = serde_json::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }
    
    /// Save configuration to file
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), LoggingError> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
    
    /// Validate configuration
    pub fn validate(&self) -> Result<(), LoggingError> {
        // Validate log levels
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        
        if !valid_levels.contains(&self.default_level.as_str()) {
            return Err(LoggingError::ConfigError(
                format!("Invalid default level: {}", self.default_level)
            ));
        }
        
        for (module, level) in &self.module_levels {
            if !valid_levels.contains(&level.as_str()) {
                return Err(LoggingError::ConfigError(
                    format!("Invalid level '{}' for module '{}'", level, module)
                ));
            }
        }
        
        // Validate file paths
        for file_config in &self.files {
            if file_config.enabled {
                if let Some(parent) = file_config.path.parent() {
                    if !parent.exists() {
                        std::fs::create_dir_all(parent)?;
                    }
                }
            }
        }
        
        // Validate performance settings
        if self.performance.sampling_rate < 0.0 || self.performance.sampling_rate > 1.0 {
            return Err(LoggingError::ConfigError(
                "Sampling rate must be between 0.0 and 1.0".to_string()
            ));
        }
        
        Ok(())
    }
    
    /// Create a development-optimized configuration
    pub fn development() -> Self {
        let mut config = Self::default();
        config.default_level = "debug".to_string();
        config.console.colored = true;
        config.console.show_module = true;
        config.console.show_thread = true;
        config.console.compact = false;
        config.performance.sampling_rate = 1.0;
        config.debug.verbose_debug = true;
        config.debug.pretty_console = true;
        config
    }
    
    /// Create a production-optimized configuration
    pub fn production() -> Self {
        let mut config = Self::default();
        config.default_level = "info".to_string();
        config.console.colored = false;
        config.console.show_module = false;
        config.console.show_thread = false;
        config.console.compact = true;
        config.console.filter_sensitive = true;
        config.performance.sampling_rate = 0.01; // 1% sampling
        config.performance.drop_on_full_queue = true;
        config.debug.verbose_debug = false;
        config.debug.pretty_console = false;
        config
    }
    
    /// Get log level for specific module
    pub fn level_for_module(&self, module: &str) -> &str {
        // Try exact match first
        if let Some(level) = self.module_levels.get(module) {
            return level;
        }
        
        // Try parent modules (e.g., "game::entities" -> "game")
        let mut parts: Vec<&str> = module.split("::").collect();
        while parts.len() > 1 {
            parts.pop();
            let parent_module = parts.join("::");
            if let Some(level) = self.module_levels.get(&parent_module) {
                return level;
            }
        }
        
        &self.default_level
    }
}

// Custom serialization for FastHashMap to ensure deterministic output
fn serialize_module_levels<S>(
    map: &FastHashMap<String, String>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let mut sorted: Vec<_> = map.iter().collect();
    sorted.sort_by_key(|(k, _)| *k);
    sorted.serialize(serializer)
}

fn deserialize_module_levels<'de, D>(
    deserializer: D,
) -> Result<FastHashMap<String, String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let vec: Vec<(String, String)> = Vec::deserialize(deserializer)?;
    let mut map = collections::fast_hash_map();
    for (k, v) in vec {
        map.insert(k, v);
    }
    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_default_config_validation() {
        let config = LoggingConfig::default();
        assert!(config.validate().is_ok());
    }
    
    #[test]
    fn test_config_serialization() {
        let config = LoggingConfig::default();
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: LoggingConfig = serde_json::from_str(&serialized).unwrap();
        
        // Verify key fields are preserved
        assert_eq!(config.default_level, deserialized.default_level);
        assert_eq!(config.console.enabled, deserialized.console.enabled);
        assert_eq!(config.files.len(), deserialized.files.len());
    }
    
    #[test]
    fn test_file_operations() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("logging.json");
        
        let config = LoggingConfig::development();
        assert!(config.to_file(&config_path).is_ok());
        
        let loaded = LoggingConfig::from_file(&config_path).unwrap();
        assert_eq!(config.default_level, loaded.default_level);
    }
    
    #[test]
    fn test_module_level_resolution() {
        let mut config = LoggingConfig::default();
        config.module_levels.insert("game".to_string(), "debug".to_string());
        config.module_levels.insert("game::entities".to_string(), "trace".to_string());
        
        assert_eq!(config.level_for_module("game::entities"), "trace");
        assert_eq!(config.level_for_module("game::spatial"), "debug");
        assert_eq!(config.level_for_module("other"), "info"); // default
    }
    
    #[test]
    fn test_rotation_strategies() {
        let daily = RotationConfig::daily();
        assert!(matches!(daily.strategy, RotationStrategy::Daily));
        assert_eq!(daily.max_archives, 30);
        
        let size_based = RotationConfig::size_based(1024);
        assert!(matches!(size_based.strategy, RotationStrategy::Size(1024)));
    }
}
