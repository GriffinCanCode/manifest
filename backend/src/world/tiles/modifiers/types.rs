//! Core types and enums for tile modifiers
//!
//! Contains type definitions, enums, and basic type implementations.

use serde::{Deserialize, Serialize};

/// Maximum number of different modifier types per tile
pub const MAX_MODIFIER_TYPES: usize = 16;

/// Special boolean flags for tiles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum SpecialFlag {
    Impassable = 0,
    Fortified = 1,
    Pillaged = 2,
    NaturalWonder = 3,
}

/// Environmental status flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum EnvironmentalStatus {
    Polluted = 0,
    Irradiated = 1,
    Flooded = 2,
    Diseased = 3,
}

/// Source of modifier (for stacking and removal)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, thiserror::Error)]
pub enum ModifierSource {
    /// Base terrain modifier
    #[error("Terrain modifier")]
    Terrain,
    /// From tile improvement
    #[error("Improvement modifier")]
    Improvement,
    /// From building in city
    #[error("Building modifier")]
    Building,
    /// From government policy
    #[error("Policy modifier")]
    Policy,
    /// From religious belief
    #[error("Religion modifier")]
    Religion,
    /// From natural wonder
    #[error("Natural Wonder modifier")]
    NaturalWonder,
    /// From temporary event
    #[error("Event modifier")]
    Event,
    /// From leader ability
    #[error("Leader modifier")]
    Leader,
    /// From technology
    #[error("Technology modifier")]
    Technology,
    /// From trade route
    #[error("Trade Route modifier")]
    TradeRoute,
    /// From military unit stationed
    #[error("Unit modifier")]
    Unit,
    /// From environmental effect
    #[error("Environmental modifier")]
    Environmental,
}

impl ModifierSource {
    /// Get display name for the source
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Terrain => "Terrain",
            Self::Improvement => "Improvement",
            Self::Building => "Building",
            Self::Policy => "Policy",
            Self::Religion => "Religion",
            Self::NaturalWonder => "Natural Wonder",
            Self::Event => "Event",
            Self::Leader => "Leader",
            Self::Technology => "Technology",
            Self::TradeRoute => "Trade Route",
            Self::Unit => "Unit",
            Self::Environmental => "Environmental",
        }
    }

    /// Check if this source is temporary (can expire)
    pub fn is_temporary(self) -> bool {
        matches!(self, Self::Event | Self::Unit | Self::Environmental)
    }

    /// Check if this source is permanent (doesn't expire)
    pub fn is_permanent(self) -> bool {
        !self.is_temporary()
    }

    /// Get priority for conflict resolution (higher = takes precedence)
    pub fn priority(self) -> u8 {
        match self {
            Self::Terrain => 1,
            Self::Improvement => 2,
            Self::Building => 3,
            Self::Technology => 4,
            Self::Religion => 5,
            Self::Policy => 6,
            Self::Leader => 7,
            Self::NaturalWonder => 8,
            Self::TradeRoute => 9,
            Self::Unit => 10,
            Self::Environmental => 11,
            Self::Event => 12, // Events have highest priority
        }
    }

    /// Check if this source can stack with others
    pub fn can_stack_with(self, other: ModifierSource) -> bool {
        match (self, other) {
            // Same source types don't stack (except events)
            (a, b) if a == b => matches!(a, Self::Event),
            
            // Terrain conflicts with improvements for some modifier types
            (Self::Terrain, Self::Improvement) | (Self::Improvement, Self::Terrain) => false,
            
            // Most other combinations can stack
            _ => true,
        }
    }

    /// Get all source types
    pub fn all() -> &'static [ModifierSource] {
        &[
            Self::Terrain, Self::Improvement, Self::Building, Self::Policy,
            Self::Religion, Self::NaturalWonder, Self::Event, Self::Leader,
            Self::Technology, Self::TradeRoute, Self::Unit, Self::Environmental,
        ]
    }
}

/// Types of modifiers that can be applied
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModifierType {
    Food,
    Production,
    Gold,
    Science,
    Culture,
    Faith,
    Movement,
    Defense,
    Appeal,
    Health,
    Tourism,
    Happiness,
    DisasterResistance,
    TradeCapacity,
    ReligiousPressure,
    SpyNetwork,
    BorderGrowth,
}

impl ModifierType {
    /// Get all modifier types
    pub const fn all() -> &'static [ModifierType] {
        &[
            ModifierType::Food, ModifierType::Production, ModifierType::Gold,
            ModifierType::Science, ModifierType::Culture, ModifierType::Faith,
            ModifierType::Movement, ModifierType::Defense, ModifierType::Appeal,
            ModifierType::Health, ModifierType::Tourism, ModifierType::Happiness,
            ModifierType::DisasterResistance, ModifierType::TradeCapacity,
            ModifierType::ReligiousPressure, ModifierType::SpyNetwork,
            ModifierType::BorderGrowth,
        ]
    }

    /// Get display name for the modifier
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Food => "Food",
            Self::Production => "Production",
            Self::Gold => "Gold",
            Self::Science => "Science",
            Self::Culture => "Culture",
            Self::Faith => "Faith",
            Self::Movement => "Movement",
            Self::Defense => "Defense",
            Self::Appeal => "Appeal",
            Self::Health => "Health",
            Self::Tourism => "Tourism",
            Self::Happiness => "Happiness",
            Self::DisasterResistance => "Disaster Resistance",
            Self::TradeCapacity => "Trade Capacity",
            Self::ReligiousPressure => "Religious Pressure",
            Self::SpyNetwork => "Spy Network",
            Self::BorderGrowth => "Border Growth",
        }
    }

    /// Get description of what this modifier affects
    pub fn description(self) -> &'static str {
        match self {
            Self::Food => "Affects food production from this tile",
            Self::Production => "Affects production output from this tile",
            Self::Gold => "Affects gold generation from this tile",
            Self::Science => "Affects science generation from this tile",
            Self::Culture => "Affects culture generation from this tile",
            Self::Faith => "Affects faith generation from this tile",
            Self::Movement => "Affects movement cost to enter this tile",
            Self::Defense => "Affects defensive bonuses when fighting on this tile",
            Self::Appeal => "Affects the appeal/desirability of this tile",
            Self::Health => "Affects health and disease resistance",
            Self::Tourism => "Affects tourism generation from this tile",
            Self::Happiness => "Affects happiness contribution of this tile",
            Self::DisasterResistance => "Affects resistance to natural disasters",
            Self::TradeCapacity => "Affects number of trade routes that can pass through",
            Self::ReligiousPressure => "Affects religious influence spreading from this tile",
            Self::SpyNetwork => "Affects spy operations effectiveness",
            Self::BorderGrowth => "Affects rate of cultural border expansion",
        }
    }

    /// Check if modifier stacks with others of same type
    pub fn stacks(self) -> bool {
        match self {
            // These modifiers stack additively
            ModifierType::Food | ModifierType::Production | ModifierType::Gold |
            ModifierType::Science | ModifierType::Culture | ModifierType::Faith => true,
            
            // These modifiers use highest value
            ModifierType::Defense | ModifierType::DisasterResistance => false,
            
            // Most others stack with diminishing returns
            _ => true,
        }
    }

    /// Get stacking method for this modifier type
    pub fn stacking_method(self) -> StackingMethod {
        match self {
            ModifierType::Food | ModifierType::Production | ModifierType::Gold |
            ModifierType::Science | ModifierType::Culture | ModifierType::Faith => StackingMethod::Additive,
            
            ModifierType::Defense | ModifierType::DisasterResistance => StackingMethod::Maximum,
            
            ModifierType::Movement => StackingMethod::Multiplicative,
            
            _ => StackingMethod::DiminishingReturns,
        }
    }

    /// Check if this modifier type is yield-related (affects resource generation)
    pub fn is_yield_modifier(self) -> bool {
        matches!(self, 
            Self::Food | Self::Production | Self::Gold | 
            Self::Science | Self::Culture | Self::Faith
        )
    }

    /// Check if this modifier type affects combat
    pub fn is_combat_modifier(self) -> bool {
        matches!(self, Self::Defense | Self::Movement)
    }

    /// Check if this modifier type is economic
    pub fn is_economic_modifier(self) -> bool {
        matches!(self, Self::Gold | Self::TradeCapacity | Self::Tourism)
    }

    /// Check if this modifier type affects city management
    pub fn is_city_modifier(self) -> bool {
        matches!(self, Self::Happiness | Self::Health | Self::Appeal | Self::BorderGrowth)
    }

    /// Get category for UI grouping
    pub fn category(self) -> ModifierCategory {
        match self {
            Self::Food | Self::Production | Self::Gold | 
            Self::Science | Self::Culture | Self::Faith => ModifierCategory::Yields,
            
            Self::Movement | Self::Defense => ModifierCategory::Combat,
            
            Self::Tourism | Self::TradeCapacity => ModifierCategory::Economic,
            
            Self::Happiness | Self::Health | Self::Appeal | Self::BorderGrowth => ModifierCategory::City,
            
            Self::ReligiousPressure | Self::SpyNetwork => ModifierCategory::Influence,
            
            Self::DisasterResistance => ModifierCategory::Environmental,
        }
    }
}

/// Categories for organizing modifiers in UI
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModifierCategory {
    Yields,
    Combat,
    Economic,
    City,
    Influence,
    Environmental,
}

impl ModifierCategory {
    /// Get display name for the category
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Yields => "Resource Yields",
            Self::Combat => "Combat & Movement",
            Self::Economic => "Trade & Economy",
            Self::City => "City Management",
            Self::Influence => "Influence & Espionage",
            Self::Environmental => "Environmental",
        }
    }

    /// Get all categories
    pub fn all() -> &'static [ModifierCategory] {
        &[
            Self::Yields, Self::Combat, Self::Economic,
            Self::City, Self::Influence, Self::Environmental,
        ]
    }
}

/// Methods for stacking multiple modifiers of the same type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackingMethod {
    /// Add all modifier strengths together
    Additive,
    /// Use the highest modifier strength
    Maximum,
    /// Multiply modifiers together
    Multiplicative,
    /// Apply diminishing returns formula
    DiminishingReturns,
}

impl StackingMethod {
    /// Calculate combined modifier strength using this stacking method
    pub fn apply(self, strengths: &[u8]) -> u8 {
        if strengths.is_empty() {
            return 8; // Neutral value
        }

        match self {
            Self::Additive => {
                let total: i32 = strengths.iter()
                    .map(|&s| s as i32 - 8) // Convert to offset from neutral
                    .sum();
                (8 + total).clamp(0, 15) as u8
            }
            
            Self::Maximum => {
                *strengths.iter().max().unwrap_or(&8)
            }
            
            Self::Multiplicative => {
                let product: f32 = strengths.iter()
                    .map(|&s| super::bitfields::CoreModifiers::modifier_to_multiplier(s))
                    .product();
                super::bitfields::CoreModifiers::multiplier_to_modifier(product)
            }
            
            Self::DiminishingReturns => {
                // Apply diminishing returns: each additional modifier has 75% effectiveness
                let mut result = 8.0; // Start at neutral
                for &strength in strengths {
                    let modifier_value = strength as f32 - 8.0;
                    let effectiveness = 0.75_f32.powi(strengths.len() as i32 - 1);
                    result += modifier_value * effectiveness;
                }
                result.clamp(0.0, 15.0) as u8
            }
        }
    }

    /// Get description of this stacking method
    pub fn description(self) -> &'static str {
        match self {
            Self::Additive => "Multiple modifiers add together linearly",
            Self::Maximum => "Only the strongest modifier applies",
            Self::Multiplicative => "Multiple modifiers multiply together",
            Self::DiminishingReturns => "Additional modifiers have reduced effectiveness",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modifier_source_properties() {
        assert!(ModifierSource::Event.is_temporary());
        assert!(ModifierSource::Terrain.is_permanent());
        assert!(ModifierSource::Event.priority() > ModifierSource::Terrain.priority());
    }

    #[test]
    fn test_modifier_type_categories() {
        assert!(ModifierType::Food.is_yield_modifier());
        assert!(ModifierType::Defense.is_combat_modifier());
        assert!(!ModifierType::Tourism.is_yield_modifier());
        
        assert_eq!(ModifierType::Food.category(), ModifierCategory::Yields);
        assert_eq!(ModifierType::Defense.category(), ModifierCategory::Combat);
    }

    #[test]
    fn test_stacking_methods() {
        let strengths = vec![10, 12, 9]; // +25%, +50%, +12.5%
        
        assert_eq!(StackingMethod::Maximum.apply(&strengths), 12);
        assert!(StackingMethod::Additive.apply(&strengths) > 8);
        assert!(StackingMethod::DiminishingReturns.apply(&strengths) < StackingMethod::Additive.apply(&strengths));
    }

    #[test]
    fn test_modifier_source_stacking() {
        assert!(ModifierSource::Building.can_stack_with(ModifierSource::Technology));
        assert!(!ModifierSource::Terrain.can_stack_with(ModifierSource::Improvement));
        assert!(ModifierSource::Event.can_stack_with(ModifierSource::Event));
    }
}
