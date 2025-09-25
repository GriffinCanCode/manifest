//! Bitfield structures for efficient modifier storage
//!
//! Contains compact bit-packed structures for storing tile modifiers efficiently.

use modular_bitfield::prelude::*;
use serde::{Deserialize, Serialize};

/// Maximum number of stacked modifiers per type
pub const MAX_MODIFIER_STACKS: u8 = 15;

/// Core tile modifiers packed into efficient bitfield
#[bitfield(bits = 64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreModifiers {
    /// Food production modifier (0-15, maps to -50% to +200%)
    #[bits = 4]
    pub food_modifier: B4,
    
    /// Production modifier (0-15, maps to -50% to +200%) 
    #[bits = 4]
    pub production_modifier: B4,
    
    /// Gold modifier (0-15, maps to -50% to +200%)
    #[bits = 4]
    pub gold_modifier: B4,
    
    /// Science modifier (0-15, maps to -50% to +200%)
    #[bits = 4]
    pub science_modifier: B4,
    
    /// Culture modifier (0-15, maps to -50% to +200%)
    #[bits = 4]
    pub culture_modifier: B4,
    
    /// Faith modifier (0-15, maps to -50% to +200%)
    #[bits = 4]
    pub faith_modifier: B4,
    
    /// Movement cost modifier (0-15, maps to 0.1x to 3.0x)
    #[bits = 4]
    pub movement_modifier: B4,
    
    /// Defense bonus (0-15, maps to 0% to +150%)
    #[bits = 4]
    pub defense_modifier: B4,
    
    /// Appeal/amenity modifier (0-15, maps to -7 to +7)
    #[bits = 4]
    pub appeal_modifier: B4,
    
    /// Health modifier (0-15, maps to -50% to +100%)
    #[bits = 4]
    pub health_modifier: B4,
    
    /// Special flags for various boolean modifiers
    #[bits = 4]
    pub special_flags: B4,
    
    /// Visibility modifier (0-15, maps to range and strength)
    #[bits = 4]
    pub visibility_modifier: B4,
    
    /// Strategic resource access flags
    #[bits = 4]
    pub strategic_access: B4,
    
    /// Luxury resource access flags  
    #[bits = 4]
    pub luxury_access: B4,
    
    /// Environmental status flags
    #[bits = 4]
    pub environmental_flags: B4,
    
    /// Reserved for future expansion
    #[bits = 4]
    pub reserved: B4,
}

impl Default for CoreModifiers {
    fn default() -> Self {
        Self::new()
            .with_food_modifier(8)      // 8 = neutral (0% modifier)
            .with_production_modifier(8)
            .with_gold_modifier(8)
            .with_science_modifier(8)
            .with_culture_modifier(8)
            .with_faith_modifier(8)
            .with_movement_modifier(8)
            .with_defense_modifier(8)
            .with_appeal_modifier(8)
            .with_health_modifier(8)
            .with_special_flags(0)
            .with_visibility_modifier(8)
            .with_strategic_access(0)
            .with_luxury_access(0)
            .with_environmental_flags(0)
            .with_reserved(0)
    }
}

impl CoreModifiers {
    /// Convert 4-bit modifier value to actual multiplier
    pub fn modifier_to_multiplier(modifier: u8) -> f32 {
        match modifier {
            0..=7 => 0.5 + (modifier as f32 * 0.0625), // -50% to 0%
            8 => 1.0,                                   // Neutral (0%)
            9..=15 => 1.0 + ((modifier - 8) as f32 * 0.25), // 0% to +175%
            _ => 1.0, // Fallback
        }
    }

    /// Convert multiplier back to 4-bit value
    pub fn multiplier_to_modifier(multiplier: f32) -> u8 {
        if multiplier < 1.0 {
            ((multiplier - 0.5) / 0.0625) as u8
        } else if multiplier > 1.0 {
            (8.0 + ((multiplier - 1.0) / 0.25)) as u8
        } else {
            8 // Neutral
        }
    }

    /// Get food production multiplier
    pub fn food_multiplier(self) -> f32 {
        Self::modifier_to_multiplier(self.food_modifier())
    }

    /// Get production multiplier
    pub fn production_multiplier(self) -> f32 {
        Self::modifier_to_multiplier(self.production_modifier())
    }

    /// Get gold multiplier
    pub fn gold_multiplier(self) -> f32 {
        Self::modifier_to_multiplier(self.gold_modifier())
    }

    /// Get science multiplier
    pub fn science_multiplier(self) -> f32 {
        Self::modifier_to_multiplier(self.science_modifier())
    }

    /// Get culture multiplier
    pub fn culture_multiplier(self) -> f32 {
        Self::modifier_to_multiplier(self.culture_modifier())
    }

    /// Get faith multiplier
    pub fn faith_multiplier(self) -> f32 {
        Self::modifier_to_multiplier(self.faith_modifier())
    }

    /// Get movement cost multiplier
    pub fn movement_multiplier(self) -> f32 {
        // Movement uses different scale: 0.1x to 3.0x
        0.1 + (self.movement_modifier() as f32 * 0.1933)
    }

    /// Get defense bonus percentage
    pub fn defense_bonus_percent(self) -> f32 {
        self.defense_modifier() as f32 * 10.0 // 0% to 150%
    }

    /// Get appeal modifier (-7 to +7)
    pub fn appeal_value(self) -> i8 {
        self.appeal_modifier() as i8 - 7
    }

    /// Get health multiplier
    pub fn health_multiplier(self) -> f32 {
        // Health uses range -50% to +100%
        if self.health_modifier() < 8 {
            0.5 + (self.health_modifier() as f32 * 0.0625)
        } else {
            1.0 + ((self.health_modifier() - 8) as f32 * 0.125)
        }
    }

    /// Check if special flag is set
    pub fn has_special_flag(self, flag: super::types::SpecialFlag) -> bool {
        let mask = 1 << (flag as u8);
        (self.special_flags() & mask) != 0
    }

    /// Set special flag
    pub fn with_special_flag(self, flag: super::types::SpecialFlag, enabled: bool) -> Self {
        let mask = 1 << (flag as u8);
        let flags = if enabled {
            self.special_flags() | mask
        } else {
            self.special_flags() & !mask
        };
        self.with_special_flags(flags)
    }

    /// Check if environmental status is active
    pub fn has_environmental_status(self, status: super::types::EnvironmentalStatus) -> bool {
        let mask = 1 << (status as u8);
        (self.environmental_flags() & mask) != 0
    }

    /// Set environmental status
    pub fn with_environmental_status(self, status: super::types::EnvironmentalStatus, enabled: bool) -> Self {
        let mask = 1 << (status as u8);
        let flags = if enabled {
            self.environmental_flags() | mask
        } else {
            self.environmental_flags() & !mask
        };
        self.with_environmental_flags(flags)
    }

    /// Check if tile is impassable
    pub fn is_impassable(self) -> bool {
        self.has_special_flag(super::types::SpecialFlag::Impassable)
    }

    /// Check if tile is fortified
    pub fn is_fortified(self) -> bool {
        self.has_special_flag(super::types::SpecialFlag::Fortified)
    }

    /// Check if tile is pillaged
    pub fn is_pillaged(self) -> bool {
        self.has_special_flag(super::types::SpecialFlag::Pillaged)
    }

    /// Check if tile has natural wonder
    pub fn has_natural_wonder(self) -> bool {
        self.has_special_flag(super::types::SpecialFlag::NaturalWonder)
    }

    /// Get all active environmental statuses
    pub fn active_environmental_statuses(self) -> Vec<super::types::EnvironmentalStatus> {
        let mut statuses = Vec::new();
        for status in [
            super::types::EnvironmentalStatus::Polluted,
            super::types::EnvironmentalStatus::Irradiated,
            super::types::EnvironmentalStatus::Flooded,
            super::types::EnvironmentalStatus::Diseased,
        ] {
            if self.has_environmental_status(status) {
                statuses.push(status);
            }
        }
        statuses
    }
}

/// Extended modifiers for complex effects
#[bitfield(bits = 32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtendedModifiers {
    /// Tourism modifier (0-15)
    #[bits = 4]
    pub tourism_modifier: B4,
    
    /// Happiness modifier (0-15, maps to -7 to +7)
    #[bits = 4]
    pub happiness_modifier: B4,
    
    /// Disaster resistance (0-15)
    #[bits = 4]
    pub disaster_resistance: B4,
    
    /// Trade route capacity modifier (0-15)
    #[bits = 4]
    pub trade_capacity: B4,
    
    /// Religious pressure modifier (0-15)
    #[bits = 4]
    pub religious_pressure: B4,
    
    /// Spy network modifier (0-15)
    #[bits = 4]
    pub spy_modifier: B4,
    
    /// Border growth modifier (0-15)
    #[bits = 4]
    pub border_growth: B4,
    
    /// Reserved for future use
    #[bits = 4]
    pub reserved: B4,
}

impl Default for ExtendedModifiers {
    fn default() -> Self {
        Self::new()
            .with_tourism_modifier(8)
            .with_happiness_modifier(8)
            .with_disaster_resistance(8)
            .with_trade_capacity(8)
            .with_religious_pressure(8)
            .with_spy_modifier(8)
            .with_border_growth(8)
    }
}

impl ExtendedModifiers {
    /// Get tourism multiplier
    pub fn tourism_multiplier(self) -> f32 {
        CoreModifiers::modifier_to_multiplier(self.tourism_modifier())
    }

    /// Get happiness value (-7 to +7)
    pub fn happiness_value(self) -> i8 {
        self.happiness_modifier() as i8 - 7
    }

    /// Get disaster resistance percentage (0% to 150%)
    pub fn disaster_resistance_percent(self) -> f32 {
        self.disaster_resistance() as f32 * 10.0
    }

    /// Get trade route capacity modifier
    pub fn trade_capacity_multiplier(self) -> f32 {
        CoreModifiers::modifier_to_multiplier(self.trade_capacity())
    }

    /// Get religious pressure multiplier
    pub fn religious_pressure_multiplier(self) -> f32 {
        CoreModifiers::modifier_to_multiplier(self.religious_pressure())
    }

    /// Get spy effectiveness multiplier
    pub fn spy_effectiveness_multiplier(self) -> f32 {
        CoreModifiers::modifier_to_multiplier(self.spy_modifier())
    }

    /// Get border growth rate multiplier
    pub fn border_growth_multiplier(self) -> f32 {
        CoreModifiers::modifier_to_multiplier(self.border_growth())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_modifiers_size() {
        assert_eq!(std::mem::size_of::<CoreModifiers>(), 8); // 64 bits = 8 bytes
    }

    #[test]
    fn test_extended_modifiers_size() {
        assert_eq!(std::mem::size_of::<ExtendedModifiers>(), 4); // 32 bits = 4 bytes
    }

    #[test]
    fn test_modifier_conversions() {
        assert_eq!(CoreModifiers::modifier_to_multiplier(8), 1.0); // Neutral
        assert_eq!(CoreModifiers::modifier_to_multiplier(0), 0.5); // -50%
        assert_eq!(CoreModifiers::modifier_to_multiplier(15), 2.75); // +175%
        
        assert_eq!(CoreModifiers::multiplier_to_modifier(1.0), 8);
        assert_eq!(CoreModifiers::multiplier_to_modifier(0.5), 0);
    }

    #[test]
    fn test_special_flags() {
        let mut modifiers = CoreModifiers::default();
        assert!(!modifiers.is_impassable());
        
        modifiers = modifiers.with_special_flag(super::types::SpecialFlag::Impassable, true);
        assert!(modifiers.is_impassable());
    }

    #[test]
    fn test_default_values() {
        let core = CoreModifiers::default();
        assert_eq!(core.food_multiplier(), 1.0);
        assert_eq!(core.production_multiplier(), 1.0);
        assert_eq!(core.appeal_value(), 0);
        
        let extended = ExtendedModifiers::default();
        assert_eq!(extended.happiness_value(), 0);
        assert_eq!(extended.tourism_multiplier(), 1.0);
    }
}
