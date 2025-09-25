//! Statistics and error types for tile modifiers
//!
//! Contains computed modifier results, statistics tracking, and error definitions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::types::{ModifierSource, ModifierType};
use crate::world::tiles::chunks::TileId;
use super::bitfields::MAX_MODIFIER_STACKS;
use super::types::MAX_MODIFIER_TYPES;

/// Computed final modifier values (cached for performance)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ComputedModifiers {
    // Yield modifiers
    pub food_multiplier: f32,
    pub production_multiplier: f32,
    pub gold_multiplier: f32,
    pub science_multiplier: f32,
    pub culture_multiplier: f32,
    pub faith_multiplier: f32,
    
    // Combat and movement modifiers
    pub movement_cost_multiplier: f32,
    pub defense_bonus: f32,
    
    // City and environmental modifiers
    pub appeal_modifier: i8,
    pub health_multiplier: f32,
    pub happiness_modifier: i8,
    
    // Extended modifiers
    pub tourism_multiplier: f32,
    pub disaster_resistance: f32,
    pub trade_capacity_multiplier: f32,
    pub religious_pressure_multiplier: f32,
    pub spy_effectiveness_multiplier: f32,
    pub border_growth_multiplier: f32,
}

impl ComputedModifiers {
    /// Create computed modifiers with neutral values
    pub fn neutral() -> Self {
        Self {
            food_multiplier: 1.0,
            production_multiplier: 1.0,
            gold_multiplier: 1.0,
            science_multiplier: 1.0,
            culture_multiplier: 1.0,
            faith_multiplier: 1.0,
            movement_cost_multiplier: 1.0,
            defense_bonus: 0.0,
            appeal_modifier: 0,
            health_multiplier: 1.0,
            happiness_modifier: 0,
            tourism_multiplier: 1.0,
            disaster_resistance: 0.0,
            trade_capacity_multiplier: 1.0,
            religious_pressure_multiplier: 1.0,
            spy_effectiveness_multiplier: 1.0,
            border_growth_multiplier: 1.0,
        }
    }

    /// Check if modifiers are effectively neutral (no significant impact)
    pub fn is_neutral(&self, tolerance: f32) -> bool {
        (self.food_multiplier - 1.0).abs() < tolerance &&
        (self.production_multiplier - 1.0).abs() < tolerance &&
        (self.gold_multiplier - 1.0).abs() < tolerance &&
        (self.science_multiplier - 1.0).abs() < tolerance &&
        (self.culture_multiplier - 1.0).abs() < tolerance &&
        (self.faith_multiplier - 1.0).abs() < tolerance &&
        (self.movement_cost_multiplier - 1.0).abs() < tolerance &&
        self.defense_bonus.abs() < tolerance &&
        self.appeal_modifier.abs() <= 1 &&
        (self.health_multiplier - 1.0).abs() < tolerance &&
        self.happiness_modifier.abs() <= 1 &&
        (self.tourism_multiplier - 1.0).abs() < tolerance &&
        self.disaster_resistance < tolerance &&
        (self.trade_capacity_multiplier - 1.0).abs() < tolerance &&
        (self.religious_pressure_multiplier - 1.0).abs() < tolerance &&
        (self.spy_effectiveness_multiplier - 1.0).abs() < tolerance &&
        (self.border_growth_multiplier - 1.0).abs() < tolerance
    }

    /// Get summary of significant modifiers for display
    pub fn significant_modifiers(&self, threshold: f32) -> Vec<String> {
        let mut modifiers = Vec::new();

        if (self.food_multiplier - 1.0).abs() >= threshold {
            let pct = ((self.food_multiplier - 1.0) * 100.0) as i32;
            modifiers.push(format!("Food: {:+}%", pct));
        }
        
        if (self.production_multiplier - 1.0).abs() >= threshold {
            let pct = ((self.production_multiplier - 1.0) * 100.0) as i32;
            modifiers.push(format!("Production: {:+}%", pct));
        }
        
        if (self.gold_multiplier - 1.0).abs() >= threshold {
            let pct = ((self.gold_multiplier - 1.0) * 100.0) as i32;
            modifiers.push(format!("Gold: {:+}%", pct));
        }
        
        if (self.science_multiplier - 1.0).abs() >= threshold {
            let pct = ((self.science_multiplier - 1.0) * 100.0) as i32;
            modifiers.push(format!("Science: {:+}%", pct));
        }
        
        if (self.culture_multiplier - 1.0).abs() >= threshold {
            let pct = ((self.culture_multiplier - 1.0) * 100.0) as i32;
            modifiers.push(format!("Culture: {:+}%", pct));
        }
        
        if (self.faith_multiplier - 1.0).abs() >= threshold {
            let pct = ((self.faith_multiplier - 1.0) * 100.0) as i32;
            modifiers.push(format!("Faith: {:+}%", pct));
        }
        
        if (self.movement_cost_multiplier - 1.0).abs() >= threshold {
            let pct = ((self.movement_cost_multiplier - 1.0) * 100.0) as i32;
            modifiers.push(format!("Movement Cost: {:+}%", pct));
        }
        
        if self.defense_bonus.abs() >= threshold {
            modifiers.push(format!("Defense: {:+.0}%", self.defense_bonus));
        }
        
        if self.appeal_modifier != 0 {
            modifiers.push(format!("Appeal: {:+}", self.appeal_modifier));
        }
        
        if (self.health_multiplier - 1.0).abs() >= threshold {
            let pct = ((self.health_multiplier - 1.0) * 100.0) as i32;
            modifiers.push(format!("Health: {:+}%", pct));
        }
        
        if self.happiness_modifier != 0 {
            modifiers.push(format!("Happiness: {:+}", self.happiness_modifier));
        }
        
        if (self.tourism_multiplier - 1.0).abs() >= threshold {
            let pct = ((self.tourism_multiplier - 1.0) * 100.0) as i32;
            modifiers.push(format!("Tourism: {:+}%", pct));
        }
        
        if self.disaster_resistance >= threshold {
            modifiers.push(format!("Disaster Resistance: {:.0}%", self.disaster_resistance));
        }
        
        modifiers
    }

    /// Get combined effectiveness score (for AI evaluation)
    pub fn effectiveness_score(&self) -> f32 {
        let yield_score = (self.food_multiplier + self.production_multiplier + self.gold_multiplier +
                          self.science_multiplier + self.culture_multiplier + self.faith_multiplier - 6.0) * 10.0;
                          
        let combat_score = (self.defense_bonus * 0.1) + ((1.0 - self.movement_cost_multiplier) * 5.0);
        
        let utility_score = (self.appeal_modifier as f32 * 0.5) + 
                           ((self.health_multiplier - 1.0) * 5.0) +
                           (self.happiness_modifier as f32 * 0.5) +
                           ((self.tourism_multiplier - 1.0) * 3.0) +
                           (self.disaster_resistance * 0.05);
        
        yield_score + combat_score + utility_score
    }

    /// Apply minimum and maximum caps to all modifiers
    pub fn clamp_to_limits(&mut self) {
        const MIN_MULTIPLIER: f32 = 0.1;
        const MAX_MULTIPLIER: f32 = 5.0;
        const MAX_DEFENSE: f32 = 200.0;
        const MIN_APPEAL: i8 = -10;
        const MAX_APPEAL: i8 = 10;
        const MIN_HAPPINESS: i8 = -10;
        const MAX_HAPPINESS: i8 = 10;
        
        self.food_multiplier = self.food_multiplier.clamp(MIN_MULTIPLIER, MAX_MULTIPLIER);
        self.production_multiplier = self.production_multiplier.clamp(MIN_MULTIPLIER, MAX_MULTIPLIER);
        self.gold_multiplier = self.gold_multiplier.clamp(MIN_MULTIPLIER, MAX_MULTIPLIER);
        self.science_multiplier = self.science_multiplier.clamp(MIN_MULTIPLIER, MAX_MULTIPLIER);
        self.culture_multiplier = self.culture_multiplier.clamp(MIN_MULTIPLIER, MAX_MULTIPLIER);
        self.faith_multiplier = self.faith_multiplier.clamp(MIN_MULTIPLIER, MAX_MULTIPLIER);
        self.movement_cost_multiplier = self.movement_cost_multiplier.clamp(MIN_MULTIPLIER, MAX_MULTIPLIER);
        self.defense_bonus = self.defense_bonus.clamp(0.0, MAX_DEFENSE);
        self.appeal_modifier = self.appeal_modifier.clamp(MIN_APPEAL, MAX_APPEAL);
        self.health_multiplier = self.health_multiplier.clamp(MIN_MULTIPLIER, MAX_MULTIPLIER);
        self.happiness_modifier = self.happiness_modifier.clamp(MIN_HAPPINESS, MAX_HAPPINESS);
        self.tourism_multiplier = self.tourism_multiplier.clamp(MIN_MULTIPLIER, MAX_MULTIPLIER);
        self.disaster_resistance = self.disaster_resistance.clamp(0.0, 100.0);
        self.trade_capacity_multiplier = self.trade_capacity_multiplier.clamp(MIN_MULTIPLIER, MAX_MULTIPLIER);
        self.religious_pressure_multiplier = self.religious_pressure_multiplier.clamp(MIN_MULTIPLIER, MAX_MULTIPLIER);
        self.spy_effectiveness_multiplier = self.spy_effectiveness_multiplier.clamp(MIN_MULTIPLIER, MAX_MULTIPLIER);
        self.border_growth_multiplier = self.border_growth_multiplier.clamp(MIN_MULTIPLIER, MAX_MULTIPLIER);
    }

    /// Get memory usage in bytes
    pub fn memory_size(&self) -> usize {
        std::mem::size_of::<Self>()
    }
}

/// Results from processing modifiers for a turn
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModifierTurnResults {
    pub total_tiles_processed: usize,
    pub expired_modifiers: usize,
    pub modifiers_updated: usize,
    pub cache_invalidations: usize,
    pub processing_time_ms: u64,
    pub errors: Vec<String>,
}

impl ModifierTurnResults {
    /// Create new empty results
    pub fn new() -> Self {
        Self::default()
    }

    /// Add processing time
    pub fn add_processing_time(&mut self, ms: u64) {
        self.processing_time_ms += ms;
    }

    /// Add error message
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }

    /// Check if processing completed successfully
    pub fn is_successful(&self) -> bool {
        self.errors.is_empty()
    }

    /// Get processing rate (tiles per second)
    pub fn processing_rate(&self) -> f32 {
        if self.processing_time_ms == 0 {
            0.0
        } else {
            (self.total_tiles_processed as f32) / (self.processing_time_ms as f32 / 1000.0)
        }
    }
}

/// Statistics for modifier monitoring
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModifierStats {
    pub total_modified_tiles: usize,
    pub total_modifier_instances: usize,
    pub permanent_modifiers: usize,
    pub temporary_modifiers: usize,
    pub by_source: HashMap<ModifierSource, usize>,
    pub by_type: HashMap<ModifierType, usize>,
    pub average_modifiers_per_tile: f32,
    pub max_modifiers_on_tile: usize,
    pub memory_usage_bytes: usize,
    pub cache_hit_rate: f32,
}

impl ModifierStats {
    /// Create new empty stats
    pub fn new() -> Self {
        Self::default()
    }

    /// Compute derived statistics
    pub fn compute_derived(&mut self) {
        if self.total_modified_tiles > 0 {
            self.average_modifiers_per_tile = self.total_modifier_instances as f32 / self.total_modified_tiles as f32;
        }
    }

    /// Get most common modifier source
    pub fn most_common_source(&self) -> Option<(ModifierSource, usize)> {
        self.by_source.iter()
            .max_by_key(|(_, count)| *count)
            .map(|(source, count)| (*source, *count))
    }

    /// Get most common modifier type
    pub fn most_common_type(&self) -> Option<(ModifierType, usize)> {
        self.by_type.iter()
            .max_by_key(|(_, count)| *count)
            .map(|(modifier_type, count)| (*modifier_type, *count))
    }

    /// Get percentage breakdown by source
    pub fn source_percentages(&self) -> HashMap<ModifierSource, f32> {
        let total = self.total_modifier_instances as f32;
        if total == 0.0 {
            return HashMap::new();
        }

        self.by_source.iter()
            .map(|(source, count)| (*source, (*count as f32 / total) * 100.0))
            .collect()
    }

    /// Get percentage breakdown by type
    pub fn type_percentages(&self) -> HashMap<ModifierType, f32> {
        let total = self.total_modifier_instances as f32;
        if total == 0.0 {
            return HashMap::new();
        }

        self.by_type.iter()
            .map(|(modifier_type, count)| (*modifier_type, (*count as f32 / total) * 100.0))
            .collect()
    }

    /// Export to summary string
    pub fn summary_string(&self) -> String {
        format!(
            "Modifiers: {} tiles, {} instances ({} permanent, {} temporary), avg {:.1}/tile, max {}/tile, {:.1}KB memory, {:.1}% cache hit rate",
            self.total_modified_tiles,
            self.total_modifier_instances,
            self.permanent_modifiers,
            self.temporary_modifiers,
            self.average_modifiers_per_tile,
            self.max_modifiers_on_tile,
            self.memory_usage_bytes as f32 / 1024.0,
            self.cache_hit_rate * 100.0
        )
    }
}

/// Modifier system errors
#[derive(Debug, thiserror::Error, Clone, Serialize, Deserialize)]
pub enum ModifierError {
    #[error("Tile not found: {tile_id}")]
    TileNotFound { tile_id: TileId },
    
    #[error("Too many modifiers on tile (max {MAX_MODIFIER_TYPES})")]
    TooManyModifiers,
    
    #[error("Invalid modifier strength: {strength} (max {MAX_MODIFIER_STACKS})")]
    InvalidStrength { strength: u8 },
    
    #[error("Cache error: {message}")]
    CacheError { message: String },
    
    #[error("Modifier not found: {modifier_type:?} from {source:?}")]
    ModifierNotFound { modifier_type: ModifierType, source: ModifierSource },
    
    #[error("Modifier conflict: cannot apply {modifier_type:?} from {source:?} (conflicts with existing)")]
    ModifierConflict { modifier_type: ModifierType, source: ModifierSource },
    
    #[error("Invalid duration: {duration} (max 65535 turns)")]
    InvalidDuration { duration: u32 },
    
    #[error("Modifier expired: {modifier_type:?} expired {turns_ago} turns ago")]
    ModifierExpired { modifier_type: ModifierType, turns_ago: u32 },
    
    #[error("System error: {message}")]
    SystemError { message: String },
}

impl ModifierError {
    /// Create cache error
    pub fn cache(message: impl Into<String>) -> Self {
        Self::CacheError { message: message.into() }
    }

    /// Create system error
    pub fn system(message: impl Into<String>) -> Self {
        Self::SystemError { message: message.into() }
    }

    /// Check if error is recoverable
    pub fn is_recoverable(&self) -> bool {
        matches!(self, 
            Self::CacheError { .. } | 
            Self::SystemError { .. }
        )
    }

    /// Get error severity level
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            Self::TileNotFound { .. } | Self::ModifierNotFound { .. } => ErrorSeverity::Warning,
            Self::TooManyModifiers | Self::InvalidStrength { .. } | 
            Self::InvalidDuration { .. } | Self::ModifierConflict { .. } => ErrorSeverity::Error,
            Self::ModifierExpired { .. } => ErrorSeverity::Info,
            Self::CacheError { .. } | Self::SystemError { .. } => ErrorSeverity::Critical,
        }
    }
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    Info,
    Warning, 
    Error,
    Critical,
}

impl ErrorSeverity {
    /// Get display name for severity
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Info => "Info",
            Self::Warning => "Warning",
            Self::Error => "Error", 
            Self::Critical => "Critical",
        }
    }

    /// Get color for UI display
    pub fn color_rgb(self) -> (u8, u8, u8) {
        match self {
            Self::Info => (100, 150, 255),    // Light blue
            Self::Warning => (255, 200, 0),   // Orange
            Self::Error => (255, 100, 100),   // Light red
            Self::Critical => (200, 0, 0),    // Dark red
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_computed_modifiers_neutral() {
        let computed = ComputedModifiers::neutral();
        assert!(computed.is_neutral(0.01));
        assert_eq!(computed.effectiveness_score(), 0.0);
    }

    #[test]
    fn test_computed_modifiers_significant() {
        let mut computed = ComputedModifiers::neutral();
        computed.food_multiplier = 1.5;
        computed.production_multiplier = 0.8;
        
        let significant = computed.significant_modifiers(0.1);
        assert!(significant.len() >= 2);
        assert!(significant.iter().any(|s| s.contains("Food")));
        assert!(significant.iter().any(|s| s.contains("Production")));
    }

    #[test]
    fn test_modifier_stats() {
        let mut stats = ModifierStats::new();
        stats.total_modified_tiles = 100;
        stats.total_modifier_instances = 350;
        stats.compute_derived();
        
        assert_eq!(stats.average_modifiers_per_tile, 3.5);
    }

    #[test]
    fn test_turn_results() {
        let mut results = ModifierTurnResults::new();
        results.total_tiles_processed = 1000;
        results.add_processing_time(500);
        
        assert_eq!(results.processing_rate(), 2.0); // 2 tiles per second
        assert!(results.is_successful());
        
        results.add_error("Test error".to_string());
        assert!(!results.is_successful());
    }

    #[test]
    fn test_modifier_error_severity() {
        let error = ModifierError::TooManyModifiers;
        assert_eq!(error.severity(), ErrorSeverity::Error);
        assert!(!error.is_recoverable());
        
        let cache_error = ModifierError::cache("Test error");
        assert_eq!(cache_error.severity(), ErrorSeverity::Critical);
        assert!(cache_error.is_recoverable());
    }

    #[test]
    fn test_computed_modifiers_clamping() {
        let mut computed = ComputedModifiers::default();
        computed.food_multiplier = 10.0; // Above max
        computed.defense_bonus = -50.0; // Below min
        computed.appeal_modifier = 20; // Above max
        
        computed.clamp_to_limits();
        
        assert_eq!(computed.food_multiplier, 5.0); // Clamped to max
        assert_eq!(computed.defense_bonus, 0.0); // Clamped to min
        assert_eq!(computed.appeal_modifier, 10); // Clamped to max
    }
}
