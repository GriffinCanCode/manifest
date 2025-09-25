//! Individual modifier instances with tracking
//!
//! Contains the ModifierInstance struct for tracking individual modifier effects.

use serde::{Deserialize, Serialize};
use super::{
    types::{ModifierType, ModifierSource},
    bitfields::MAX_MODIFIER_STACKS,
};

/// Individual modifier instance with source tracking
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModifierInstance {
    /// Type of modifier
    pub modifier_type: ModifierType,
    /// Source of this modifier
    pub source: ModifierSource,
    /// Strength of the modifier (0-15)
    pub strength: u8,
    /// Duration in turns (None = permanent)
    pub duration: Option<u16>,
    /// Turn when modifier was applied
    pub applied_turn: u32,
    /// Source-specific identifier for removal
    pub source_id: Option<u32>,
}

impl ModifierInstance {
    /// Create new modifier instance
    pub fn new(modifier_type: ModifierType, source: ModifierSource, strength: u8) -> Self {
        Self {
            modifier_type,
            source,
            strength: strength.min(MAX_MODIFIER_STACKS),
            duration: None,
            applied_turn: 0, // Would be set by game logic
            source_id: None,
        }
    }

    /// Create temporary modifier with duration
    pub fn temporary(modifier_type: ModifierType, source: ModifierSource, strength: u8, duration: u16) -> Self {
        Self {
            modifier_type,
            source,
            strength: strength.min(MAX_MODIFIER_STACKS),
            duration: Some(duration),
            applied_turn: 0,
            source_id: None,
        }
    }

    /// Create modifier with source ID for tracking
    pub fn with_source_id(modifier_type: ModifierType, source: ModifierSource, strength: u8, source_id: u32) -> Self {
        Self {
            modifier_type,
            source,
            strength: strength.min(MAX_MODIFIER_STACKS),
            duration: None,
            applied_turn: 0,
            source_id: Some(source_id),
        }
    }

    /// Create temporary modifier with source ID
    pub fn temporary_with_source_id(
        modifier_type: ModifierType, 
        source: ModifierSource, 
        strength: u8, 
        duration: u16, 
        source_id: u32
    ) -> Self {
        Self {
            modifier_type,
            source,
            strength: strength.min(MAX_MODIFIER_STACKS),
            duration: Some(duration),
            applied_turn: 0,
            source_id: Some(source_id),
        }
    }

    /// Set the turn when this modifier was applied
    pub fn set_applied_turn(&mut self, turn: u32) {
        self.applied_turn = turn;
    }

    /// Check if modifier has expired
    pub fn is_expired(&self, current_turn: u32) -> bool {
        self.duration.map_or(false, |dur| current_turn >= self.applied_turn + dur as u32)
    }

    /// Get effective strength (considering duration for fading effects)
    pub fn effective_strength(&self, current_turn: u32) -> u8 {
        if let Some(duration) = self.duration {
            let elapsed = current_turn.saturating_sub(self.applied_turn);
            if elapsed >= duration as u32 {
                0
            } else {
                // Optionally implement fading effects here
                self.strength
            }
        } else {
            self.strength
        }
    }

    /// Get effective strength with fading for certain modifier types
    pub fn effective_strength_with_fading(&self, current_turn: u32) -> u8 {
        if let Some(duration) = self.duration {
            let elapsed = current_turn.saturating_sub(self.applied_turn);
            if elapsed >= duration as u32 {
                return 0;
            }

            // Apply fading for certain temporary effects
            match self.source {
                ModifierSource::Event | ModifierSource::Environmental => {
                    let progress = elapsed as f32 / duration as f32;
                    let fade_factor = match progress {
                        p if p < 0.5 => 1.0, // Full strength for first half
                        p => 1.0 - (p - 0.5) * 0.5, // Linear fade in second half
                    };
                    (self.strength as f32 * fade_factor) as u8
                }
                _ => self.strength, // No fading for other sources
            }
        } else {
            self.strength
        }
    }

    /// Check if modifier is permanent
    pub fn is_permanent(&self) -> bool {
        self.duration.is_none()
    }

    /// Check if modifier is temporary
    pub fn is_temporary(&self) -> bool {
        self.duration.is_some()
    }

    /// Get remaining duration in turns (None if permanent)
    pub fn remaining_duration(&self, current_turn: u32) -> Option<u16> {
        self.duration.map(|dur| {
            let elapsed = current_turn.saturating_sub(self.applied_turn) as u16;
            dur.saturating_sub(elapsed)
        })
    }

    /// Get age in turns since application
    pub fn age(&self, current_turn: u32) -> u32 {
        current_turn.saturating_sub(self.applied_turn)
    }

    /// Check if this modifier conflicts with another (same type, same source ID)
    pub fn conflicts_with(&self, other: &ModifierInstance) -> bool {
        self.modifier_type == other.modifier_type &&
        self.source == other.source &&
        self.source_id == other.source_id
    }

    /// Check if this modifier can stack with another
    pub fn can_stack_with(&self, other: &ModifierInstance) -> bool {
        // Same modifier type must be stackable
        if !self.modifier_type.stacks() {
            return self.modifier_type != other.modifier_type;
        }

        // Check if sources can stack
        self.source.can_stack_with(other.source) && !self.conflicts_with(other)
    }

    /// Get priority for conflict resolution (higher wins)
    pub fn priority(&self) -> u16 {
        let source_priority = self.source.priority() as u16;
        let strength_priority = self.strength as u16;
        
        // Combine source priority (high weight) with strength (low weight)
        source_priority * 16 + strength_priority
    }

    /// Convert modifier strength to percentage change
    pub fn to_percentage(&self) -> f32 {
        super::bitfields::CoreModifiers::modifier_to_multiplier(self.strength) - 1.0
    }

    /// Get human-readable description
    pub fn description(&self) -> String {
        let percentage = (self.to_percentage() * 100.0) as i32;
        let sign = if percentage >= 0 { "+" } else { "" };
        
        format!(
            "{}{:}% {} from {}",
            sign,
            percentage,
            self.modifier_type.display_name(),
            self.source.display_name()
        )
    }

    /// Get detailed description including duration
    pub fn detailed_description(&self, current_turn: u32) -> String {
        let mut desc = self.description();
        
        if let Some(remaining) = self.remaining_duration(current_turn) {
            desc.push_str(&format!(" ({} turns remaining)", remaining));
        }
        
        if self.source_id.is_some() {
            desc.push_str(&format!(" [ID: {}]", self.source_id.unwrap()));
        }
        
        desc
    }

    /// Check if modifier should be displayed to player
    pub fn should_display(&self) -> bool {
        // Don't display very weak modifiers
        if self.strength <= 1 || self.strength >= 15 {
            return false;
        }

        // Don't display expired temporary modifiers
        if self.is_temporary() && self.effective_strength(0) == 0 {
            return false;
        }

        true
    }

    /// Get color for UI display based on strength and type
    pub fn display_color(&self) -> ModifierDisplayColor {
        let is_positive = self.strength > 8;
        let magnitude = if is_positive { 
            self.strength - 8 
        } else { 
            8 - self.strength 
        };

        match (is_positive, magnitude) {
            (true, 1..=2) => ModifierDisplayColor::LightGreen,
            (true, 3..=5) => ModifierDisplayColor::Green,
            (true, _) => ModifierDisplayColor::DarkGreen,
            (false, 1..=2) => ModifierDisplayColor::LightRed,
            (false, 3..=5) => ModifierDisplayColor::Red,
            (false, _) => ModifierDisplayColor::DarkRed,
        }
    }

    /// Get memory usage in bytes
    pub fn memory_size(&self) -> usize {
        std::mem::size_of::<Self>()
    }
}

/// Colors for displaying modifiers in UI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModifierDisplayColor {
    DarkGreen,
    Green,
    LightGreen,
    LightRed,
    Red,
    DarkRed,
}

impl ModifierDisplayColor {
    /// Get RGB color values (0-255)
    pub fn rgb(self) -> (u8, u8, u8) {
        match self {
            Self::DarkGreen => (0, 100, 0),
            Self::Green => (0, 150, 0),
            Self::LightGreen => (100, 200, 100),
            Self::LightRed => (200, 100, 100),
            Self::Red => (200, 0, 0),
            Self::DarkRed => (150, 0, 0),
        }
    }

    /// Get hex color string
    pub fn hex(self) -> String {
        let (r, g, b) = self.rgb();
        format!("#{:02X}{:02X}{:02X}", r, g, b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modifier_instance_creation() {
        let modifier = ModifierInstance::new(
            ModifierType::Food,
            ModifierSource::Improvement,
            10
        );
        
        assert_eq!(modifier.modifier_type, ModifierType::Food);
        assert_eq!(modifier.source, ModifierSource::Improvement);
        assert_eq!(modifier.strength, 10);
        assert!(modifier.is_permanent());
    }

    #[test]
    fn test_temporary_modifier() {
        let modifier = ModifierInstance::temporary(
            ModifierType::Defense,
            ModifierSource::Event,
            12,
            5
        );
        
        assert!(modifier.is_temporary());
        assert_eq!(modifier.remaining_duration(0), Some(5));
        assert_eq!(modifier.remaining_duration(3), Some(2));
        assert!(modifier.is_expired(6));
    }

    #[test]
    fn test_modifier_stacking() {
        let mod1 = ModifierInstance::new(ModifierType::Food, ModifierSource::Terrain, 10);
        let mod2 = ModifierInstance::new(ModifierType::Food, ModifierSource::Building, 12);
        let mod3 = ModifierInstance::new(ModifierType::Food, ModifierSource::Terrain, 8);
        
        assert!(mod1.can_stack_with(&mod2));
        assert!(mod1.conflicts_with(&mod3));
    }

    #[test]
    fn test_modifier_description() {
        let modifier = ModifierInstance::new(
            ModifierType::Production,
            ModifierSource::Technology,
            10
        );
        
        let desc = modifier.description();
        assert!(desc.contains("Production"));
        assert!(desc.contains("Technology"));
        assert!(desc.contains("+"));
    }

    #[test]
    fn test_effective_strength_with_fading() {
        let mut modifier = ModifierInstance::temporary(
            ModifierType::Gold,
            ModifierSource::Event,
            12,
            10
        );
        modifier.set_applied_turn(0);
        
        assert_eq!(modifier.effective_strength_with_fading(3), 12); // First half
        assert!(modifier.effective_strength_with_fading(8) < 12); // Fading
        assert_eq!(modifier.effective_strength_with_fading(10), 0); // Expired
    }

    #[test]
    fn test_display_color() {
        let positive_mod = ModifierInstance::new(ModifierType::Food, ModifierSource::Terrain, 12);
        let negative_mod = ModifierInstance::new(ModifierType::Food, ModifierSource::Terrain, 4);
        
        matches!(positive_mod.display_color(), ModifierDisplayColor::Green);
        matches!(negative_mod.display_color(), ModifierDisplayColor::Red);
    }
}
