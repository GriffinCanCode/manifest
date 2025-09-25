//! Defense bonus system with ordered float precision
//!
//! Provides defense bonus calculations with precise floating-point
//! arithmetic and bonus stacking from multiple sources.

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use ordered_float::OrderedFloat;

/// Defense bonuses with ordered float precision
#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub struct DefenseBonuses {
    /// Base terrain defense
    pub terrain_bonus: OrderedFloat<f32>,
    /// Improvement defense bonus
    pub improvement_bonus: OrderedFloat<f32>,
    /// Elevation advantage
    pub elevation_bonus: OrderedFloat<f32>,
    /// Final combined bonus
    pub total_bonus: OrderedFloat<f32>,
}

impl Default for DefenseBonuses {
    fn default() -> Self {
        Self {
            terrain_bonus: OrderedFloat(0.0),
            improvement_bonus: OrderedFloat(0.0),
            elevation_bonus: OrderedFloat(0.0),
            total_bonus: OrderedFloat(0.0),
        }
    }
}

impl DefenseBonuses {
    /// Create new defense bonuses with terrain bonus
    pub fn new(terrain_bonus: f32) -> Self {
        let mut bonuses = Self {
            terrain_bonus: OrderedFloat(terrain_bonus),
            ..Default::default()
        };
        bonuses.calculate_total();
        bonuses
    }

    /// Set terrain defense bonus
    pub fn set_terrain_bonus(&mut self, bonus: f32) {
        self.terrain_bonus = OrderedFloat(bonus);
        self.calculate_total();
    }

    /// Set improvement defense bonus
    pub fn set_improvement_bonus(&mut self, bonus: f32) {
        self.improvement_bonus = OrderedFloat(bonus);
        self.calculate_total();
    }

    /// Set elevation defense bonus
    pub fn set_elevation_bonus(&mut self, bonus: f32) {
        self.elevation_bonus = OrderedFloat(bonus);
        self.calculate_total();
    }

    /// Add to terrain bonus
    pub fn add_terrain_bonus(&mut self, bonus: f32) {
        self.terrain_bonus = OrderedFloat(self.terrain_bonus.into_inner() + bonus);
        self.calculate_total();
    }

    /// Add to improvement bonus
    pub fn add_improvement_bonus(&mut self, bonus: f32) {
        self.improvement_bonus = OrderedFloat(self.improvement_bonus.into_inner() + bonus);
        self.calculate_total();
    }

    /// Add to elevation bonus
    pub fn add_elevation_bonus(&mut self, bonus: f32) {
        self.elevation_bonus = OrderedFloat(self.elevation_bonus.into_inner() + bonus);
        self.calculate_total();
    }

    /// Calculate total defense bonus
    pub fn calculate_total(&mut self) {
        let raw_total = self.terrain_bonus.into_inner() + 
                       self.improvement_bonus.into_inner() + 
                       self.elevation_bonus.into_inner();
        
        // Cap at 90% bonus to prevent overpowered defensive positions
        self.total_bonus = OrderedFloat(raw_total.min(0.9));
    }

    /// Get effective total defense bonus
    pub fn get_total(&self) -> f32 {
        self.total_bonus.into_inner()
    }

    /// Get defense category
    pub fn defense_category(&self) -> DefenseCategory {
        let total = self.total_bonus.into_inner();
        match total {
            t if t <= 0.0 => DefenseCategory::None,
            t if t <= 0.1 => DefenseCategory::Weak,
            t if t <= 0.25 => DefenseCategory::Moderate,
            t if t <= 0.5 => DefenseCategory::Strong,
            _ => DefenseCategory::Fortress,
        }
    }

    /// Check if position provides significant defensive advantage
    pub fn is_defensible(&self) -> bool {
        self.total_bonus.into_inner() > 0.1
    }

    /// Get damage reduction percentage
    pub fn damage_reduction_percent(&self) -> f32 {
        self.total_bonus.into_inner() * 100.0
    }

    /// Apply weather modifier to defense
    pub fn apply_weather_modifier(&mut self, weather_modifier: f32) {
        let current_total = self.terrain_bonus.into_inner() + 
                           self.improvement_bonus.into_inner() + 
                           self.elevation_bonus.into_inner();
        let modified_total = current_total * weather_modifier;
        self.total_bonus = OrderedFloat(modified_total.min(0.9));
    }

    /// Reset bonuses
    pub fn reset(&mut self) {
        self.terrain_bonus = OrderedFloat(0.0);
        self.improvement_bonus = OrderedFloat(0.0);
        self.elevation_bonus = OrderedFloat(0.0);
        self.total_bonus = OrderedFloat(0.0);
    }

    /// Get breakdown of defense sources
    pub fn get_breakdown(&self) -> DefenseBreakdown {
        DefenseBreakdown {
            terrain: self.terrain_bonus.into_inner(),
            improvement: self.improvement_bonus.into_inner(),
            elevation: self.elevation_bonus.into_inner(),
            total: self.total_bonus.into_inner(),
        }
    }

    /// Compare with another defense bonus set
    pub fn compare_with(&self, other: &DefenseBonuses) -> DefenseComparison {
        let self_total = self.total_bonus.into_inner();
        let other_total = other.total_bonus.into_inner();
        let difference = self_total - other_total;

        match difference {
            d if d.abs() < 0.01 => DefenseComparison::Equal,
            d if d > 0.0 => DefenseComparison::Superior(d),
            d => DefenseComparison::Inferior(d.abs()),
        }
    }
}

/// Defense categories for UI and gameplay
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DefenseCategory {
    None,
    Weak,
    Moderate,
    Strong,
    Fortress,
}

impl DefenseCategory {
    /// Get category description
    pub fn description(&self) -> &'static str {
        match self {
            Self::None => "No defensive advantage",
            Self::Weak => "Weak defensive position",
            Self::Moderate => "Moderate defensive advantage",
            Self::Strong => "Strong defensive position",
            Self::Fortress => "Fortress-like defenses",
        }
    }

    /// Get category color for UI
    pub fn color(&self) -> (u8, u8, u8) {
        match self {
            Self::None => (128, 128, 128),    // Gray
            Self::Weak => (255, 255, 0),      // Yellow
            Self::Moderate => (255, 128, 0),  // Orange
            Self::Strong => (255, 0, 0),      // Red
            Self::Fortress => (128, 0, 128),  // Purple
        }
    }

    /// Get minimum bonus for this category
    pub fn min_bonus(&self) -> f32 {
        match self {
            Self::None => 0.0,
            Self::Weak => 0.01,
            Self::Moderate => 0.11,
            Self::Strong => 0.26,
            Self::Fortress => 0.51,
        }
    }
}

/// Defense bonus breakdown for analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefenseBreakdown {
    pub terrain: f32,
    pub improvement: f32,
    pub elevation: f32,
    pub total: f32,
}

impl DefenseBreakdown {
    /// Get primary source of defense bonus
    pub fn primary_source(&self) -> DefenseSource {
        let terrain = self.terrain;
        let improvement = self.improvement;
        let elevation = self.elevation;

        if terrain >= improvement && terrain >= elevation {
            DefenseSource::Terrain
        } else if improvement >= elevation {
            DefenseSource::Improvement
        } else {
            DefenseSource::Elevation
        }
    }

    /// Check if defenses are well-rounded
    pub fn is_balanced(&self) -> bool {
        let values = [self.terrain, self.improvement, self.elevation];
        let max = values.iter().cloned().fold(0.0f32, f32::max);
        let min = values.iter().cloned().fold(f32::INFINITY, f32::min);
        
        max - min < 0.2 // Difference less than 20%
    }
}

/// Defense comparison result
#[derive(Debug, Clone, PartialEq)]
pub enum DefenseComparison {
    Superior(f32),
    Equal,
    Inferior(f32),
}

impl DefenseComparison {
    /// Get comparison description
    pub fn description(&self) -> String {
        match self {
            Self::Superior(diff) => format!("Superior defense (+{:.1}%)", diff * 100.0),
            Self::Equal => "Equal defense".to_string(),
            Self::Inferior(diff) => format!("Inferior defense (-{:.1}%)", diff * 100.0),
        }
    }
}

/// Defense source for breakdown analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DefenseSource {
    Terrain,
    Improvement,
    Elevation,
}

impl DefenseSource {
    /// Get source description
    pub fn description(&self) -> &'static str {
        match self {
            Self::Terrain => "Natural terrain features",
            Self::Improvement => "Man-made fortifications",
            Self::Elevation => "Height advantage",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defense_bonus_creation() {
        let bonuses = DefenseBonuses::new(0.2);
        assert_eq!(bonuses.terrain_bonus.into_inner(), 0.2);
        assert_eq!(bonuses.get_total(), 0.2);
        assert_eq!(bonuses.defense_category(), DefenseCategory::Moderate);
    }

    #[test]
    fn test_bonus_calculation() {
        let mut bonuses = DefenseBonuses::default();
        
        bonuses.set_terrain_bonus(0.1);
        bonuses.set_improvement_bonus(0.15);
        bonuses.set_elevation_bonus(0.1);
        
        assert_eq!(bonuses.get_total(), 0.35);
        assert_eq!(bonuses.defense_category(), DefenseCategory::Strong);
    }

    #[test]
    fn test_bonus_capping() {
        let mut bonuses = DefenseBonuses::default();
        
        bonuses.set_terrain_bonus(0.5);
        bonuses.set_improvement_bonus(0.4);
        bonuses.set_elevation_bonus(0.3);
        
        // Should be capped at 0.9 (90%)
        assert_eq!(bonuses.get_total(), 0.9);
    }

    #[test]
    fn test_additive_bonuses() {
        let mut bonuses = DefenseBonuses::new(0.1);
        
        bonuses.add_terrain_bonus(0.05);
        bonuses.add_improvement_bonus(0.1);
        bonuses.add_elevation_bonus(0.05);
        
        assert_eq!(bonuses.get_total(), 0.3);
    }

    #[test]
    fn test_defense_categories() {
        assert_eq!(DefenseBonuses::new(0.0).defense_category(), DefenseCategory::None);
        assert_eq!(DefenseBonuses::new(0.05).defense_category(), DefenseCategory::Weak);
        assert_eq!(DefenseBonuses::new(0.2).defense_category(), DefenseCategory::Moderate);
        assert_eq!(DefenseBonuses::new(0.4).defense_category(), DefenseCategory::Strong);
        assert_eq!(DefenseBonuses::new(0.7).defense_category(), DefenseCategory::Fortress);
    }

    #[test]
    fn test_damage_reduction() {
        let bonuses = DefenseBonuses::new(0.25);
        assert_eq!(bonuses.damage_reduction_percent(), 25.0);
        assert!(bonuses.is_defensible());
    }

    #[test]
    fn test_weather_modifier() {
        let mut bonuses = DefenseBonuses::new(0.2);
        bonuses.apply_weather_modifier(1.5); // Storm increases defensive advantage
        
        assert_eq!(bonuses.get_total(), 0.3);
        
        bonuses.apply_weather_modifier(0.5); // Clear weather reduces bonus
        assert!(bonuses.get_total() < 0.2);
    }

    #[test]
    fn test_defense_breakdown() {
        let mut bonuses = DefenseBonuses::default();
        bonuses.set_terrain_bonus(0.1);
        bonuses.set_improvement_bonus(0.3);
        bonuses.set_elevation_bonus(0.05);
        
        let breakdown = bonuses.get_breakdown();
        assert_eq!(breakdown.terrain, 0.1);
        assert_eq!(breakdown.improvement, 0.3);
        assert_eq!(breakdown.elevation, 0.05);
        assert_eq!(breakdown.total, 0.45);
        assert_eq!(breakdown.primary_source(), DefenseSource::Improvement);
    }

    #[test]
    fn test_defense_comparison() {
        let bonuses1 = DefenseBonuses::new(0.3);
        let bonuses2 = DefenseBonuses::new(0.2);
        let bonuses3 = DefenseBonuses::new(0.3);
        
        match bonuses1.compare_with(&bonuses2) {
            DefenseComparison::Superior(diff) => assert!((diff - 0.1).abs() < 0.01),
            _ => panic!("Expected superior comparison"),
        }
        
        assert_eq!(bonuses1.compare_with(&bonuses3), DefenseComparison::Equal);
    }

    #[test]
    fn test_balanced_defenses() {
        let mut bonuses = DefenseBonuses::default();
        bonuses.set_terrain_bonus(0.1);
        bonuses.set_improvement_bonus(0.12);
        bonuses.set_elevation_bonus(0.08);
        
        let breakdown = bonuses.get_breakdown();
        assert!(breakdown.is_balanced());
        
        bonuses.set_improvement_bonus(0.5);
        let breakdown2 = bonuses.get_breakdown();
        assert!(!breakdown2.is_balanced());
    }

    #[test]
    fn test_reset_bonuses() {
        let mut bonuses = DefenseBonuses::new(0.2);
        bonuses.set_improvement_bonus(0.1);
        bonuses.set_elevation_bonus(0.05);
        
        bonuses.reset();
        assert_eq!(bonuses.get_total(), 0.0);
        assert_eq!(bonuses.defense_category(), DefenseCategory::None);
    }
}
