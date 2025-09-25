//! Core ownership types and constants for tile ownership system
//!
//! Defines basic types, enums, and constants used throughout the ownership system.

use serde::{Deserialize, Serialize};

/// Maximum number of players supported (for bitvec sizing)
pub const MAX_PLAYERS: usize = 64;

/// Player identifier type
pub type PlayerId = u8;

/// Ownership status for a single tile
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OwnershipStatus {
    /// Tile is unowned (neutral)
    Unowned,
    /// Tile is owned by a specific player
    Owned(PlayerId),
    /// Tile is contested by multiple players
    Contested,
    /// Tile ownership is disputed (recent conflict)
    Disputed,
}

impl Default for OwnershipStatus {
    fn default() -> Self {
        Self::Unowned
    }
}

/// Ownership strength levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum OwnershipStrength {
    None = 0,
    Weak = 1,      // Recently claimed, easily lost
    Moderate = 2,  // Established presence
    Strong = 3,    // Well-defended territory
    Absolute = 4,  // Core territory, very hard to take
}

impl Default for OwnershipStrength {
    fn default() -> Self {
        Self::None
    }
}

impl OwnershipStrength {
    /// Convert strength to multiplier for various game mechanics
    pub fn as_multiplier(self) -> f32 {
        match self {
            Self::None => 0.0,
            Self::Weak => 0.25,
            Self::Moderate => 0.5,
            Self::Strong => 0.75,
            Self::Absolute => 1.0,
        }
    }

    /// Check if ownership strength allows certain actions
    pub fn allows_action(self, required_strength: OwnershipStrength) -> bool {
        (self as u8) >= (required_strength as u8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ownership_status() {
        assert_eq!(OwnershipStatus::default(), OwnershipStatus::Unowned);
    }

    #[test]
    fn test_ownership_strength() {
        assert!(OwnershipStrength::Strong.allows_action(OwnershipStrength::Moderate));
        assert!(!OwnershipStrength::Weak.allows_action(OwnershipStrength::Strong));
        
        assert_eq!(OwnershipStrength::Absolute.as_multiplier(), 1.0);
        assert_eq!(OwnershipStrength::None.as_multiplier(), 0.0);
    }
}
