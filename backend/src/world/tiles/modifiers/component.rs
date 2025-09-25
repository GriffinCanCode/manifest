//! TileModifiers ECS component
//!
//! Contains the main TileModifiers component for efficient modifier storage and computation.

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use arrayvec::ArrayVec;

use super::{
    bitfields::{CoreModifiers, ExtendedModifiers},
    instance::ModifierInstance,
    types::{ModifierType, ModifierSource, StackingMethod, MAX_MODIFIER_TYPES},
    stats::{ModifierError, ComputedModifiers},
};

/// Complete modifier set for a tile with efficient storage
#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub struct TileModifiers {
    /// Core packed modifiers (64 bits)
    pub core: CoreModifiers,
    /// Extended packed modifiers (32 bits)
    pub extended: ExtendedModifiers,
    /// Individual modifier instances for tracking
    pub instances: ArrayVec<ModifierInstance, MAX_MODIFIER_TYPES>,
    /// Cache of computed final values (not serialized)
    #[serde(skip)]
    computed_cache: Option<ComputedModifiers>,
    /// Generation counter for cache invalidation
    generation: u64,
}

impl TileModifiers {
    /// Create new tile modifiers with defaults
    pub fn new() -> Self {
        Self {
            core: CoreModifiers::default(),
            extended: ExtendedModifiers::default(),
            instances: ArrayVec::new(),
            computed_cache: None,
            generation: 0,
        }
    }

    /// Add modifier instance
    pub fn add_modifier(&mut self, modifier: ModifierInstance) -> Result<(), ModifierError> {
        // Check if we can stack with existing modifier
        if let Some(existing_idx) = self.instances.iter().position(|m| 
            m.modifier_type == modifier.modifier_type && 
            m.source == modifier.source &&
            m.source_id == modifier.source_id
        ) {
            // Update existing modifier
            self.instances[existing_idx] = modifier;
        } else {
            // Add new modifier
            if self.instances.is_full() {
                return Err(ModifierError::TooManyModifiers);
            }
            self.instances.push(modifier);
        }

        self.recompute_modifiers();
        Ok(())
    }

    /// Remove modifier by source and type
    pub fn remove_modifier(&mut self, modifier_type: ModifierType, source: ModifierSource, source_id: Option<u32>) -> bool {
        let initial_len = self.instances.len();
        
        self.instances.retain(|m| !(
            m.modifier_type == modifier_type &&
            m.source == source &&
            m.source_id == source_id
        ));

        let removed = self.instances.len() != initial_len;
        if removed {
            self.recompute_modifiers();
        }
        
        removed
    }

    /// Remove all modifiers from a source
    pub fn remove_modifiers_from_source(&mut self, source: ModifierSource, source_id: Option<u32>) -> usize {
        let initial_len = self.instances.len();
        
        self.instances.retain(|m| !(
            m.source == source &&
            (source_id.is_none() || m.source_id == source_id)
        ));

        let removed = initial_len - self.instances.len();
        if removed > 0 {
            self.recompute_modifiers();
        }
        
        removed
    }

    /// Process turn for temporary modifiers
    pub fn process_turn(&mut self, current_turn: u32) -> usize {
        let initial_len = self.instances.len();
        
        // Remove expired modifiers
        self.instances.retain(|m| !m.is_expired(current_turn));
        
        let expired = initial_len - self.instances.len();
        if expired > 0 {
            self.recompute_modifiers();
        }
        
        expired
    }

    /// Get computed modifier values (cached)
    pub fn computed(&mut self) -> &ComputedModifiers {
        if self.computed_cache.is_none() {
            self.recompute_modifiers();
        }
        
        self.computed_cache.as_ref().expect("Computed cache should be populated after calling computed() method")
    }

    /// Get computed modifier values without mutating (may return outdated cache)
    pub fn computed_readonly(&self) -> Option<&ComputedModifiers> {
        self.computed_cache.as_ref()
    }

    /// Force recomputation of modifiers
    fn recompute_modifiers(&mut self) {
        let mut computed = ComputedModifiers::default();
        
        // Start with base core modifiers
        computed.food_multiplier = self.core.food_multiplier();
        computed.production_multiplier = self.core.production_multiplier();
        computed.gold_multiplier = self.core.gold_multiplier();
        computed.science_multiplier = self.core.science_multiplier();
        computed.culture_multiplier = self.core.culture_multiplier();
        computed.faith_multiplier = self.core.faith_multiplier();
        computed.movement_cost_multiplier = self.core.movement_multiplier();
        computed.defense_bonus = self.core.defense_bonus_percent();
        computed.appeal_modifier = self.core.appeal_value();
        computed.health_multiplier = self.core.health_multiplier();
        
        // Add extended modifiers
        computed.tourism_multiplier = self.extended.tourism_multiplier();
        computed.happiness_modifier = self.extended.happiness_value();
        computed.disaster_resistance = self.extended.disaster_resistance_percent();
        computed.trade_capacity_multiplier = self.extended.trade_capacity_multiplier();
        computed.religious_pressure_multiplier = self.extended.religious_pressure_multiplier();
        computed.spy_effectiveness_multiplier = self.extended.spy_effectiveness_multiplier();
        computed.border_growth_multiplier = self.extended.border_growth_multiplier();
        
        // Apply modifier instances with proper stacking
        self.apply_instances_to_computed(&mut computed);

        self.computed_cache = Some(computed);
        self.generation += 1;
    }

    /// Apply modifier instances to computed values with proper stacking
    fn apply_instances_to_computed(&self, computed: &mut ComputedModifiers) {
        // Group modifiers by type for stacking
        let mut modifier_groups: std::collections::HashMap<ModifierType, Vec<u8>> = 
            std::collections::HashMap::new();

        for modifier in &self.instances {
            let strength = modifier.effective_strength(0); // Would use current turn in real implementation
            if strength > 0 {
                modifier_groups.entry(modifier.modifier_type)
                    .or_default()
                    .push(strength);
            }
        }

        // Apply stacked modifiers
        for (modifier_type, strengths) in modifier_groups {
            let stacking_method = modifier_type.stacking_method();
            let final_strength = stacking_method.apply(&strengths);
            let multiplier = CoreModifiers::modifier_to_multiplier(final_strength);
            
            // Apply to appropriate computed field
            match modifier_type {
                ModifierType::Food => computed.food_multiplier *= multiplier,
                ModifierType::Production => computed.production_multiplier *= multiplier,
                ModifierType::Gold => computed.gold_multiplier *= multiplier,
                ModifierType::Science => computed.science_multiplier *= multiplier,
                ModifierType::Culture => computed.culture_multiplier *= multiplier,
                ModifierType::Faith => computed.faith_multiplier *= multiplier,
                ModifierType::Movement => computed.movement_cost_multiplier *= multiplier,
                ModifierType::Defense => {
                    match stacking_method {
                        StackingMethod::Maximum => {
                            computed.defense_bonus = computed.defense_bonus.max(
                                CoreModifiers::modifier_to_multiplier(final_strength) * 100.0 - 100.0
                            );
                        }
                        _ => computed.defense_bonus += (multiplier - 1.0) * 100.0,
                    }
                }
                ModifierType::Appeal => {
                    computed.appeal_modifier += final_strength as i8 - 8;
                }
                ModifierType::Health => computed.health_multiplier *= multiplier,
                ModifierType::Tourism => computed.tourism_multiplier *= multiplier,
                ModifierType::Happiness => {
                    computed.happiness_modifier += final_strength as i8 - 8;
                }
                ModifierType::DisasterResistance => {
                    computed.disaster_resistance = computed.disaster_resistance.max(
                        (final_strength as f32 / 15.0) * 100.0
                    );
                }
                ModifierType::TradeCapacity => computed.trade_capacity_multiplier *= multiplier,
                ModifierType::ReligiousPressure => computed.religious_pressure_multiplier *= multiplier,
                ModifierType::SpyNetwork => computed.spy_effectiveness_multiplier *= multiplier,
                ModifierType::BorderGrowth => computed.border_growth_multiplier *= multiplier,
            }
        }
    }

    /// Get all active modifiers (non-expired, non-zero strength)
    pub fn active_modifiers(&self) -> Vec<&ModifierInstance> {
        self.instances.iter()
            .filter(|m| m.effective_strength(0) > 0) // Would use current turn
            .collect()
    }

    /// Get modifiers by type
    pub fn modifiers_by_type(&self, modifier_type: ModifierType) -> Vec<&ModifierInstance> {
        self.instances.iter()
            .filter(|m| m.modifier_type == modifier_type)
            .collect()
    }

    /// Get modifiers by source
    pub fn modifiers_by_source(&self, source: ModifierSource) -> Vec<&ModifierInstance> {
        self.instances.iter()
            .filter(|m| m.source == source)
            .collect()
    }

    /// Check if has modifier of specific type and source
    pub fn has_modifier(&self, modifier_type: ModifierType, source: ModifierSource) -> bool {
        self.instances.iter().any(|m| 
            m.modifier_type == modifier_type && m.source == source
        )
    }

    /// Get total modifier count
    pub fn modifier_count(&self) -> usize {
        self.instances.len()
    }

    /// Get active modifier count (non-expired)
    pub fn active_modifier_count(&self) -> usize {
        self.active_modifiers().len()
    }

    /// Check if at modifier capacity
    pub fn is_at_capacity(&self) -> bool {
        self.instances.is_full()
    }

    /// Get available modifier slots
    pub fn available_slots(&self) -> usize {
        MAX_MODIFIER_TYPES - self.instances.len()
    }

    /// Clear all modifiers
    pub fn clear(&mut self) {
        self.instances.clear();
        self.recompute_modifiers();
    }

    /// Clear modifiers from specific source
    pub fn clear_source(&mut self, source: ModifierSource) {
        let removed = self.remove_modifiers_from_source(source, None);
        if removed > 0 {
            self.recompute_modifiers();
        }
    }

    /// Get generation counter (for change detection)
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Get memory usage in bytes
    pub fn memory_size(&self) -> usize {
        std::mem::size_of::<Self>() + 
        self.instances.iter().map(|m| m.memory_size()).sum::<usize>() +
        self.computed_cache.as_ref().map_or(0, |c| std::mem::size_of::<ComputedModifiers>())
    }

    /// Export modifier summary for UI display
    pub fn export_summary(&self) -> ModifierSummary {
        let active_mods = self.active_modifiers();
        let mut by_category = std::collections::HashMap::new();
        
        for modifier in &active_mods {
            let category = modifier.modifier_type.category();
            by_category.entry(category)
                .or_insert_with(Vec::new)
                .push(modifier.description());
        }

        ModifierSummary {
            total_modifiers: self.instances.len(),
            active_modifiers: active_mods.len(),
            temporary_modifiers: active_mods.iter().filter(|m| m.is_temporary()).count(),
            by_category,
            generation: self.generation,
        }
    }
}

impl Default for TileModifiers {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary of modifiers for UI display
#[derive(Debug, Clone)]
pub struct ModifierSummary {
    pub total_modifiers: usize,
    pub active_modifiers: usize,
    pub temporary_modifiers: usize,
    pub by_category: std::collections::HashMap<super::types::ModifierCategory, Vec<String>>,
    pub generation: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_modifiers_creation() {
        let modifiers = TileModifiers::new();
        assert_eq!(modifiers.modifier_count(), 0);
        assert!(!modifiers.is_at_capacity());
        assert_eq!(modifiers.available_slots(), MAX_MODIFIER_TYPES);
    }

    #[test]
    fn test_add_remove_modifiers() {
        let mut modifiers = TileModifiers::new();
        
        let modifier = ModifierInstance::new(
            ModifierType::Food,
            ModifierSource::Improvement,
            10
        );
        
        assert!(modifiers.add_modifier(modifier).is_ok());
        assert_eq!(modifiers.modifier_count(), 1);
        
        assert!(modifiers.remove_modifier(
            ModifierType::Food, 
            ModifierSource::Improvement, 
            None
        ));
        assert_eq!(modifiers.modifier_count(), 0);
    }

    #[test]
    fn test_modifier_stacking() {
        let mut modifiers = TileModifiers::new();
        
        // Add multiple food modifiers from different sources
        let mod1 = ModifierInstance::new(ModifierType::Food, ModifierSource::Terrain, 10);
        let mod2 = ModifierInstance::new(ModifierType::Food, ModifierSource::Building, 12);
        
        assert!(modifiers.add_modifier(mod1).is_ok());
        assert!(modifiers.add_modifier(mod2).is_ok());
        assert_eq!(modifiers.modifier_count(), 2);
        
        let computed = modifiers.computed();
        assert!(computed.food_multiplier > 1.0); // Should be combined effect
    }

    #[test]
    fn test_process_turn() {
        let mut modifiers = TileModifiers::new();
        
        let temporary_mod = ModifierInstance::temporary(
            ModifierType::Defense,
            ModifierSource::Event,
            12,
            5
        );
        
        assert!(modifiers.add_modifier(temporary_mod).is_ok());
        assert_eq!(modifiers.modifier_count(), 1);
        
        // Process 6 turns (should expire the 5-turn modifier)
        let expired = modifiers.process_turn(6);
        assert_eq!(expired, 1);
        assert_eq!(modifiers.modifier_count(), 0);
    }

    #[test]
    fn test_modifier_summary() {
        let mut modifiers = TileModifiers::new();
        
        let food_mod = ModifierInstance::new(ModifierType::Food, ModifierSource::Improvement, 10);
        let defense_mod = ModifierInstance::temporary(ModifierType::Defense, ModifierSource::Event, 12, 3);
        
        modifiers.add_modifier(food_mod).expect("Should be able to add food modifier to empty TileModifiers");
        modifiers.add_modifier(defense_mod).expect("Should be able to add defense modifier to TileModifiers");
        
        let summary = modifiers.export_summary();
        assert_eq!(summary.total_modifiers, 2);
        assert_eq!(summary.active_modifiers, 2);
        assert_eq!(summary.temporary_modifiers, 1);
        assert!(!summary.by_category.is_empty());
    }
}
