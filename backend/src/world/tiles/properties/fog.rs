//! Fog of war system with bitvec visibility tracking
//!
//! Provides efficient fog of war implementation using bitvec
//! for player visibility and discovery tracking.

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use bitvec::prelude::*;

use crate::world::tiles::ownership::PlayerId;

/// Fog of war with bitvec visibility tracking
#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub struct FogOfWar {
    /// Players who have discovered this tile (bitfield)
    discovered: BitArray<[u64; 1]>,  // Support up to 64 players
    /// Players who currently have vision (bitfield)
    visible: BitArray<[u64; 1]>,
    /// Last turn seen by each player
    last_seen: [u16; 8], // Support up to 8 active players for last_seen tracking
    /// Vision level (0=unexplored, 1=discovered, 2=visible, 3=always_visible)
    vision_levels: [u8; 8],
}

impl Default for FogOfWar {
    fn default() -> Self {
        Self {
            discovered: BitArray::ZERO,
            visible: BitArray::ZERO,
            last_seen: [0; 8],
            vision_levels: [0; 8],
        }
    }
}

impl FogOfWar {
    /// Create new fog of war
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if tile is discovered by player
    pub fn is_discovered_by(&self, player_id: PlayerId) -> bool {
        if (player_id as usize) < 64 {
            self.discovered[player_id as usize]
        } else {
            false
        }
    }

    /// Check if tile is visible to player
    pub fn is_visible_to(&self, player_id: PlayerId) -> bool {
        if (player_id as usize) < 64 {
            self.visible[player_id as usize]
        } else {
            false
        }
    }

    /// Mark tile as discovered by player
    pub fn discover_for_player(&mut self, player_id: PlayerId, turn: u16) {
        if (player_id as usize) < 64 {
            self.discovered.set(player_id as usize, true);
            
            if (player_id as usize) < self.last_seen.len() {
                self.last_seen[player_id as usize] = turn;
            }
            
            // Set minimum vision level to discovered
            if (player_id as usize) < self.vision_levels.len() {
                self.vision_levels[player_id as usize] = self.vision_levels[player_id as usize].max(1);
            }
        }
    }

    /// Set visibility for player
    pub fn set_visible_to_player(&mut self, player_id: PlayerId, visible: bool) {
        if (player_id as usize) < 64 {
            self.visible.set(player_id as usize, visible);
            
            if visible {
                // If setting visible, also mark as discovered
                self.discovered.set(player_id as usize, true);
                
                if (player_id as usize) < self.vision_levels.len() {
                    self.vision_levels[player_id as usize] = self.vision_levels[player_id as usize].max(2);
                }
            }
        }
    }

    /// Get vision level for player
    pub fn get_vision_level(&self, player_id: PlayerId) -> VisionLevel {
        if (player_id as usize) >= self.vision_levels.len() {
            return VisionLevel::Unexplored;
        }

        match self.vision_levels[player_id as usize] {
            0 => VisionLevel::Unexplored,
            1 => VisionLevel::Discovered,
            2 => VisionLevel::Visible,
            3 => VisionLevel::AlwaysVisible,
            _ => VisionLevel::Unexplored,
        }
    }

    /// Set vision level for player
    pub fn set_vision_level(&mut self, player_id: PlayerId, level: VisionLevel) {
        if (player_id as usize) < self.vision_levels.len() {
            self.vision_levels[player_id as usize] = level as u8;
            
            // Update bitfields based on vision level
            match level {
                VisionLevel::Unexplored => {
                    if (player_id as usize) < 64 {
                        self.discovered.set(player_id as usize, false);
                        self.visible.set(player_id as usize, false);
                    }
                },
                VisionLevel::Discovered => {
                    if (player_id as usize) < 64 {
                        self.discovered.set(player_id as usize, true);
                        self.visible.set(player_id as usize, false);
                    }
                },
                VisionLevel::Visible | VisionLevel::AlwaysVisible => {
                    if (player_id as usize) < 64 {
                        self.discovered.set(player_id as usize, true);
                        self.visible.set(player_id as usize, true);
                    }
                },
            }
        }
    }

    /// Get last turn seen by player
    pub fn last_seen_by(&self, player_id: PlayerId) -> Option<u16> {
        if (player_id as usize) < self.last_seen.len() {
            let turn = self.last_seen[player_id as usize];
            if turn > 0 {
                Some(turn)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Update last seen turn for player
    pub fn update_last_seen(&mut self, player_id: PlayerId, turn: u16) {
        if (player_id as usize) < self.last_seen.len() {
            self.last_seen[player_id as usize] = turn;
        }
    }

    /// Get all players who have discovered this tile
    pub fn discovered_by_players(&self) -> Vec<PlayerId> {
        let mut players = Vec::new();
        for (index, bit) in self.discovered.iter().enumerate() {
            if *bit && index < 64 {
                players.push(index as PlayerId);
            }
        }
        players
    }

    /// Get all players who can currently see this tile
    pub fn visible_to_players(&self) -> Vec<PlayerId> {
        let mut players = Vec::new();
        for (index, bit) in self.visible.iter().enumerate() {
            if *bit && index < 64 {
                players.push(index as PlayerId);
            }
        }
        players
    }

    /// Check if tile is unexplored by all players
    pub fn is_unexplored(&self) -> bool {
        self.discovered.not_any()
    }

    /// Check if tile is visible to any player
    pub fn is_visible_to_any(&self) -> bool {
        self.visible.any()
    }

    /// Clear all visibility (for testing or reset)
    pub fn clear_all_visibility(&mut self) {
        self.discovered = BitArray::ZERO;
        self.visible = BitArray::ZERO;
        self.last_seen = [0; 8];
        self.vision_levels = [0; 8];
    }

    /// Share vision between players (ally mechanic)
    pub fn share_vision(&mut self, from_player: PlayerId, to_player: PlayerId) {
        if self.is_visible_to(from_player) {
            self.set_visible_to_player(to_player, true);
        } else if self.is_discovered_by(from_player) {
            self.discover_for_player(to_player, 0); // Use 0 as placeholder turn
        }
    }

    /// Get fog of war status for player
    pub fn get_status(&self, player_id: PlayerId) -> FogStatus {
        let vision_level = self.get_vision_level(player_id);
        let last_seen = self.last_seen_by(player_id);
        
        FogStatus {
            vision_level,
            last_seen_turn: last_seen,
            is_stale: last_seen.map(|turn| turn < 100).unwrap_or(false), // Example staleness
        }
    }
}

/// Vision levels for fog of war
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum VisionLevel {
    Unexplored = 0,
    Discovered = 1,
    Visible = 2,
    AlwaysVisible = 3,
}

impl VisionLevel {
    /// Check if level allows seeing current state
    pub fn can_see_current(&self) -> bool {
        matches!(self, VisionLevel::Visible | VisionLevel::AlwaysVisible)
    }

    /// Check if level allows seeing terrain
    pub fn can_see_terrain(&self) -> bool {
        !matches!(self, VisionLevel::Unexplored)
    }

    /// Check if level allows seeing units
    pub fn can_see_units(&self) -> bool {
        matches!(self, VisionLevel::Visible | VisionLevel::AlwaysVisible)
    }

    /// Get description of vision level
    pub fn description(&self) -> &'static str {
        match self {
            VisionLevel::Unexplored => "Unexplored - no information available",
            VisionLevel::Discovered => "Discovered - terrain visible, units unknown",
            VisionLevel::Visible => "Visible - current state visible",
            VisionLevel::AlwaysVisible => "Always visible - permanent vision",
        }
    }

    /// Get opacity for rendering (0.0 = transparent, 1.0 = opaque)
    pub fn fog_opacity(&self) -> f32 {
        match self {
            VisionLevel::Unexplored => 1.0,
            VisionLevel::Discovered => 0.6,
            VisionLevel::Visible => 0.0,
            VisionLevel::AlwaysVisible => 0.0,
        }
    }
}

/// Fog of war status for UI and gameplay
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FogStatus {
    pub vision_level: VisionLevel,
    pub last_seen_turn: Option<u16>,
    pub is_stale: bool,
}

impl FogStatus {
    /// Check if information is reliable
    pub fn is_reliable(&self) -> bool {
        self.vision_level.can_see_current() && !self.is_stale
    }

    /// Get staleness description
    pub fn staleness_description(&self) -> String {
        match (self.last_seen_turn, self.is_stale) {
            (Some(turn), true) => format!("Last seen {} turns ago", turn),
            (Some(_), false) => "Recently seen".to_string(),
            (None, _) => "Never seen".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fog_of_war_creation() {
        let fog = FogOfWar::new();
        assert!(fog.is_unexplored());
        assert!(!fog.is_visible_to_any());
    }

    #[test]
    fn test_player_discovery() {
        let mut fog = FogOfWar::new();
        
        fog.discover_for_player(1, 10);
        assert!(fog.is_discovered_by(1));
        assert!(!fog.is_visible_to(1));
        assert_eq!(fog.last_seen_by(1), Some(10));
        assert_eq!(fog.get_vision_level(1), VisionLevel::Discovered);
    }

    #[test]
    fn test_player_visibility() {
        let mut fog = FogOfWar::new();
        
        fog.set_visible_to_player(1, true);
        assert!(fog.is_visible_to(1));
        assert!(fog.is_discovered_by(1)); // Should auto-discover
        assert_eq!(fog.get_vision_level(1), VisionLevel::Visible);
        
        fog.set_visible_to_player(1, false);
        assert!(!fog.is_visible_to(1));
        assert!(fog.is_discovered_by(1)); // Should remain discovered
    }

    #[test]
    fn test_vision_levels() {
        let mut fog = FogOfWar::new();
        
        fog.set_vision_level(1, VisionLevel::Discovered);
        assert!(fog.is_discovered_by(1));
        assert!(!fog.is_visible_to(1));
        
        fog.set_vision_level(1, VisionLevel::Visible);
        assert!(fog.is_discovered_by(1));
        assert!(fog.is_visible_to(1));
        
        fog.set_vision_level(1, VisionLevel::AlwaysVisible);
        assert!(fog.is_visible_to(1));
        
        fog.set_vision_level(1, VisionLevel::Unexplored);
        assert!(!fog.is_discovered_by(1));
        assert!(!fog.is_visible_to(1));
    }

    #[test]
    fn test_multiple_players() {
        let mut fog = FogOfWar::new();
        
        fog.discover_for_player(1, 5);
        fog.set_visible_to_player(2, true);
        fog.set_vision_level(3, VisionLevel::AlwaysVisible);
        
        assert_eq!(fog.discovered_by_players().len(), 3);
        assert_eq!(fog.visible_to_players().len(), 2); // Players 2 and 3
    }

    #[test]
    fn test_vision_sharing() {
        let mut fog = FogOfWar::new();
        
        fog.set_visible_to_player(1, true);
        fog.share_vision(1, 2);
        
        assert!(fog.is_visible_to(2));
        assert!(fog.is_discovered_by(2));
    }

    #[test]
    fn test_fog_status() {
        let mut fog = FogOfWar::new();
        fog.discover_for_player(1, 50);
        
        let status = fog.get_status(1);
        assert_eq!(status.vision_level, VisionLevel::Discovered);
        assert_eq!(status.last_seen_turn, Some(50));
        
        fog.set_visible_to_player(1, true);
        let status2 = fog.get_status(1);
        assert_eq!(status2.vision_level, VisionLevel::Visible);
        assert!(status2.is_reliable());
    }

    #[test]
    fn test_vision_level_properties() {
        assert!(!VisionLevel::Unexplored.can_see_terrain());
        assert!(VisionLevel::Discovered.can_see_terrain());
        assert!(!VisionLevel::Discovered.can_see_units());
        assert!(VisionLevel::Visible.can_see_units());
        assert!(VisionLevel::AlwaysVisible.can_see_current());
        
        assert_eq!(VisionLevel::Unexplored.fog_opacity(), 1.0);
        assert_eq!(VisionLevel::Visible.fog_opacity(), 0.0);
    }

    #[test]
    fn test_clear_visibility() {
        let mut fog = FogOfWar::new();
        
        fog.set_visible_to_player(1, true);
        fog.discover_for_player(2, 10);
        
        assert!(fog.is_visible_to_any());
        assert!(!fog.is_unexplored());
        
        fog.clear_all_visibility();
        
        assert!(!fog.is_visible_to_any());
        assert!(fog.is_unexplored());
    }

    #[test]
    fn test_player_id_bounds() {
        let mut fog = FogOfWar::new();
        
        // Valid player ID
        fog.discover_for_player(5, 10);
        assert!(fog.is_discovered_by(5));
        
        // Invalid player ID (too high for bitfield)
        fog.discover_for_player(100, 10);
        assert!(!fog.is_discovered_by(100));
        
        // Edge case - player 63 (max for 64-bit bitfield)
        fog.discover_for_player(63, 10);
        assert!(fog.is_discovered_by(63));
    }
}
