//! Visibility and fog of war components
//!
//! Contains components for managing player visibility, fog of war, and discovery
//! using efficient bitfield storage.

use bevy_ecs::prelude::Component;
use serde::{Deserialize, Serialize};
use modular_bitfield::prelude::*;

/// Player visibility bitfield for efficient storage
#[bitfield(bits = 64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerVisibilityFlags {
    /// Players who have discovered this tile (32 players max)
    #[bits = 32]
    pub discovered_by: u32,
    /// Players who currently have vision (32 players max)
    #[bits = 32] 
    pub visible_to: u32,
}

impl Default for PlayerVisibilityFlags {
    fn default() -> Self {
        Self::new()
    }
}

impl PlayerVisibilityFlags {
    /// Check if player has discovered this tile
    pub fn is_discovered_by_player(&self, player_id: u8) -> bool {
        if player_id >= 32 { return false; }
        (self.discovered_by() & (1 << player_id)) != 0
    }
    
    /// Set discovery status for player
    pub fn set_discovered_by_player(&mut self, player_id: u8, discovered: bool) {
        if player_id >= 32 { return; }
        let mask = 1 << player_id;
        if discovered {
            self.set_discovered_by(self.discovered_by() | mask);
        } else {
            self.set_discovered_by(self.discovered_by() & !mask);
        }
    }
    
    /// Check if player has vision of this tile
    pub fn is_visible_to_player(&self, player_id: u8) -> bool {
        if player_id >= 32 { return false; }
        (self.visible_to() & (1 << player_id)) != 0
    }
    
    /// Set visibility for player
    pub fn set_visible_to_player(&mut self, player_id: u8, visible: bool) {
        if player_id >= 32 { return; }
        let mask = 1 << player_id;
        if visible {
            self.set_visible_to(self.visible_to() | mask);
        } else {
            self.set_visible_to(self.visible_to() & !mask);
        }
    }
}

/// Visibility component for fog of war with improved bitfield storage
#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub struct Visibility {
    /// Player visibility flags (discovered and visible)
    pub player_flags: PlayerVisibilityFlags,
    /// Last turn this tile was seen by each player
    pub last_seen: [u16; 8], // Support up to 8 players for tracking
}

impl Default for Visibility {
    fn default() -> Self {
        Self {
            player_flags: PlayerVisibilityFlags::default(),
            last_seen: [0; 8],
        }
    }
}

impl Visibility {
    /// Check if player has discovered this tile
    pub fn is_discovered_by(&self, player_id: u8) -> bool {
        self.player_flags.is_discovered_by_player(player_id)
    }
    
    /// Set discovery status for player
    pub fn set_discovered_by(&mut self, player_id: u8, discovered: bool) {
        self.player_flags.set_discovered_by_player(player_id, discovered);
        if discovered && player_id < 8 {
            // Update last_seen when discovered
            self.last_seen[player_id as usize] = 1; // Would use current turn in real implementation
        }
    }
    
    /// Check if player has vision of this tile
    pub fn is_visible_to(&self, player_id: u8) -> bool {
        self.player_flags.is_visible_to_player(player_id)
    }
    
    /// Set visibility for player
    pub fn set_visible_to(&mut self, player_id: u8, visible: bool) {
        self.player_flags.set_visible_to_player(player_id, visible);
        if visible && player_id < 8 {
            // Update last_seen when visible
            self.last_seen[player_id as usize] = 1; // Would use current turn in real implementation
        }
    }
    
    /// Get last turn this tile was seen by player
    pub fn last_seen_by(&self, player_id: u8) -> u16 {
        if player_id < 8 {
            self.last_seen[player_id as usize]
        } else {
            0
        }
    }
    
    /// Set last seen turn for player
    pub fn set_last_seen(&mut self, player_id: u8, turn: u16) {
        if player_id < 8 {
            self.last_seen[player_id as usize] = turn;
        }
    }
}
