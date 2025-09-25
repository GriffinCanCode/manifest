//! Tile improvement system with Lua-scripted effects
//!
//! Provides tile improvements that are different from the main improvements module.
//! These are property-based improvements with Lua callback support.

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Improvement with Lua-scripted effects
#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub struct TileImprovement {
    pub improvement_type: String,
    pub level: u8,
    pub construction_progress: f32,
    pub effects: ImprovementEffects,
    pub lua_callback: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovementEffects {
    pub movement_cost_modifier: f32,
    pub defense_bonus: f32,
    pub resource_yield_modifiers: HashMap<String, f32>,
    pub population_capacity: i32,
}

impl Default for TileImprovement {
    fn default() -> Self {
        Self {
            improvement_type: "none".to_string(),
            level: 0,
            construction_progress: 0.0,
            effects: ImprovementEffects {
                movement_cost_modifier: 1.0,
                defense_bonus: 0.0,
                resource_yield_modifiers: HashMap::new(),
                population_capacity: 0,
            },
            lua_callback: None,
        }
    }
}

impl TileImprovement {
    /// Create new tile improvement
    pub fn new(improvement_type: String) -> Self {
        Self {
            improvement_type,
            ..Default::default()
        }
    }

    /// Create improvement with level
    pub fn with_level(improvement_type: String, level: u8) -> Self {
        Self {
            improvement_type,
            level,
            ..Default::default()
        }
    }

    /// Create improvement with effects
    pub fn with_effects(improvement_type: String, effects: ImprovementEffects) -> Self {
        Self {
            improvement_type,
            effects,
            ..Default::default()
        }
    }

    /// Add construction progress
    pub fn add_progress(&mut self, progress: f32) -> bool {
        self.construction_progress += progress;
        if self.construction_progress >= 100.0 {
            self.construction_progress = 100.0;
            true // Construction completed
        } else {
            false
        }
    }

    /// Check if improvement is completed
    pub fn is_completed(&self) -> bool {
        self.construction_progress >= 100.0
    }

    /// Check if improvement is under construction
    pub fn is_under_construction(&self) -> bool {
        self.construction_progress > 0.0 && self.construction_progress < 100.0
    }

    /// Upgrade improvement level
    pub fn upgrade(&mut self) -> bool {
        if self.is_completed() && self.level < 10 {
            self.level += 1;
            self.construction_progress = 0.0;
            self.update_effects_for_level();
            true
        } else {
            false
        }
    }

    /// Update effects based on improvement level
    fn update_effects_for_level(&mut self) {
        let level_multiplier = 1.0 + (self.level as f32 * 0.2);
        
        // Scale effects based on level
        self.effects.defense_bonus *= level_multiplier;
        self.effects.population_capacity = (self.effects.population_capacity as f32 * level_multiplier) as i32;
        
        // Movement cost gets better with higher levels
        if self.effects.movement_cost_modifier > 1.0 {
            self.effects.movement_cost_modifier /= level_multiplier;
        }
        
        // Scale resource yield modifiers
        for modifier in self.effects.resource_yield_modifiers.values_mut() {
            *modifier *= level_multiplier;
        }
    }

    /// Get effective movement cost modifier
    pub fn effective_movement_cost_modifier(&self) -> f32 {
        if self.is_completed() {
            self.effects.movement_cost_modifier
        } else {
            1.0 // No effect while under construction
        }
    }

    /// Get effective defense bonus
    pub fn effective_defense_bonus(&self) -> f32 {
        if self.is_completed() {
            self.effects.defense_bonus
        } else {
            0.0 // No defense while under construction
        }
    }

    /// Get effective population capacity
    pub fn effective_population_capacity(&self) -> i32 {
        if self.is_completed() {
            self.effects.population_capacity
        } else {
            0 // No capacity while under construction
        }
    }

    /// Get resource yield modifier for specific resource
    pub fn get_resource_yield_modifier(&self, resource_type: &str) -> f32 {
        if self.is_completed() {
            self.effects.resource_yield_modifiers.get(resource_type).copied().unwrap_or(1.0)
        } else {
            1.0 // No modifier while under construction
        }
    }

    /// Set Lua callback for custom effects
    pub fn set_lua_callback(&mut self, callback: String) {
        self.lua_callback = Some(callback);
    }

    /// Check if improvement has Lua callback
    pub fn has_lua_callback(&self) -> bool {
        self.lua_callback.is_some()
    }

    /// Get improvement category
    pub fn category(&self) -> ImprovementCategory {
        match self.improvement_type.as_str() {
            "road" | "railroad" | "bridge" => ImprovementCategory::Infrastructure,
            "farm" | "mine" | "quarry" | "lumbermill" => ImprovementCategory::Resource,
            "fort" | "fortress" | "watchtower" => ImprovementCategory::Military,
            "market" | "trading_post" | "bank" => ImprovementCategory::Economic,
            "temple" | "university" | "monument" => ImprovementCategory::Cultural,
            _ => ImprovementCategory::Misc,
        }
    }

    /// Get construction time based on type and level
    pub fn construction_time(&self) -> u32 {
        let base_time = match self.category() {
            ImprovementCategory::Infrastructure => 5,
            ImprovementCategory::Resource => 8,
            ImprovementCategory::Military => 10,
            ImprovementCategory::Economic => 12,
            ImprovementCategory::Cultural => 15,
            ImprovementCategory::Misc => 6,
        };

        base_time + (self.level as u32 * 2)
    }

    /// Get construction cost based on type and level
    pub fn construction_cost(&self) -> u32 {
        let base_cost = match self.category() {
            ImprovementCategory::Infrastructure => 100,
            ImprovementCategory::Resource => 150,
            ImprovementCategory::Military => 200,
            ImprovementCategory::Economic => 300,
            ImprovementCategory::Cultural => 250,
            ImprovementCategory::Misc => 80,
        };

        base_cost + (self.level as u32 * 50)
    }
}

/// Improvement categories for organization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ImprovementCategory {
    Infrastructure,
    Resource,
    Military,
    Economic,
    Cultural,
    Misc,
}

impl ImprovementCategory {
    /// Get category description
    pub fn description(&self) -> &'static str {
        match self {
            Self::Infrastructure => "Roads, bridges, and transportation improvements",
            Self::Resource => "Farms, mines, and resource extraction improvements",
            Self::Military => "Forts, watchtowers, and defensive improvements",
            Self::Economic => "Markets, banks, and trade improvements",
            Self::Cultural => "Temples, universities, and cultural improvements",
            Self::Misc => "Miscellaneous and special improvements",
        }
    }

    /// Get maintenance cost multiplier
    pub fn maintenance_multiplier(&self) -> f32 {
        match self {
            Self::Infrastructure => 0.5,
            Self::Resource => 1.0,
            Self::Military => 1.5,
            Self::Economic => 0.8,
            Self::Cultural => 1.2,
            Self::Misc => 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_improvement_creation() {
        let improvement = TileImprovement::new("farm".to_string());
        assert_eq!(improvement.improvement_type, "farm");
        assert_eq!(improvement.level, 0);
        assert_eq!(improvement.construction_progress, 0.0);
        assert!(!improvement.is_completed());
    }

    #[test]
    fn test_construction_progress() {
        let mut improvement = TileImprovement::new("road".to_string());
        
        assert!(!improvement.add_progress(50.0));
        assert!(improvement.is_under_construction());
        assert!(!improvement.is_completed());
        
        assert!(improvement.add_progress(60.0));
        assert!(improvement.is_completed());
        assert_eq!(improvement.construction_progress, 100.0);
    }

    #[test]
    fn test_upgrade_system() {
        let mut improvement = TileImprovement::new("fort".to_string());
        improvement.construction_progress = 100.0; // Complete it
        
        assert!(improvement.upgrade());
        assert_eq!(improvement.level, 1);
        assert_eq!(improvement.construction_progress, 0.0);
        
        // Can't upgrade while under construction
        assert!(!improvement.upgrade());
    }

    #[test]
    fn test_effective_values() {
        let mut improvement = TileImprovement::with_effects(
            "fort".to_string(),
            ImprovementEffects {
                movement_cost_modifier: 1.0,
                defense_bonus: 0.5,
                resource_yield_modifiers: HashMap::new(),
                population_capacity: 100,
            },
        );

        // Under construction - no effects
        assert_eq!(improvement.effective_defense_bonus(), 0.0);
        assert_eq!(improvement.effective_population_capacity(), 0);
        
        // Complete construction
        improvement.construction_progress = 100.0;
        assert_eq!(improvement.effective_defense_bonus(), 0.5);
        assert_eq!(improvement.effective_population_capacity(), 100);
    }

    #[test]
    fn test_improvement_categories() {
        assert_eq!(TileImprovement::new("road".to_string()).category(), ImprovementCategory::Infrastructure);
        assert_eq!(TileImprovement::new("farm".to_string()).category(), ImprovementCategory::Resource);
        assert_eq!(TileImprovement::new("fort".to_string()).category(), ImprovementCategory::Military);
        assert_eq!(TileImprovement::new("market".to_string()).category(), ImprovementCategory::Economic);
    }

    #[test]
    fn test_resource_yield_modifiers() {
        let mut effects = ImprovementEffects {
            movement_cost_modifier: 1.0,
            defense_bonus: 0.0,
            resource_yield_modifiers: HashMap::new(),
            population_capacity: 0,
        };
        effects.resource_yield_modifiers.insert("food".to_string(), 1.5);
        
        let mut improvement = TileImprovement::with_effects("farm".to_string(), effects);
        improvement.construction_progress = 100.0;
        
        assert_eq!(improvement.get_resource_yield_modifier("food"), 1.5);
        assert_eq!(improvement.get_resource_yield_modifier("production"), 1.0);
    }

    #[test]
    fn test_lua_callback() {
        let mut improvement = TileImprovement::new("custom".to_string());
        assert!(!improvement.has_lua_callback());
        
        improvement.set_lua_callback("custom_effect".to_string());
        assert!(improvement.has_lua_callback());
        assert_eq!(improvement.lua_callback, Some("custom_effect".to_string()));
    }

    #[test]
    fn test_construction_costs() {
        let infrastructure = TileImprovement::new("road".to_string());
        let resource = TileImprovement::new("farm".to_string());
        let military = TileImprovement::new("fort".to_string());
        
        assert!(resource.construction_cost() > infrastructure.construction_cost());
        assert!(military.construction_cost() > resource.construction_cost());
        assert!(military.construction_time() > infrastructure.construction_time());
    }
}
