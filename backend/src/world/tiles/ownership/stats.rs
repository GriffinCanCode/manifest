//! Ownership statistics and monitoring utilities
//!
//! Provides OwnershipStats struct for monitoring ownership metrics
//! and performance tracking across the ownership system.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use super::types::PlayerId;

/// Ownership statistics for monitoring
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OwnershipStats {
    pub total_chunks: usize,
    pub total_claimed_tiles: usize,
    pub owned_tiles: usize,
    pub contested_tiles: usize,
    pub disputed_tiles: usize,
    pub active_players: u8,
    pub player_territories: HashMap<PlayerId, usize>,
}

impl OwnershipStats {
    /// Create new empty stats
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the player with the most territories
    pub fn dominant_player(&self) -> Option<(PlayerId, usize)> {
        self.player_territories
            .iter()
            .max_by_key(|(_, &count)| count)
            .map(|(&player, &count)| (player, count))
    }

    /// Get percentage of tiles that are owned vs contested/disputed
    pub fn ownership_distribution(&self) -> (f32, f32, f32) {
        if self.total_claimed_tiles == 0 {
            return (0.0, 0.0, 0.0);
        }

        let owned_pct = self.owned_tiles as f32 / self.total_claimed_tiles as f32;
        let contested_pct = self.contested_tiles as f32 / self.total_claimed_tiles as f32;
        let disputed_pct = self.disputed_tiles as f32 / self.total_claimed_tiles as f32;

        (owned_pct, contested_pct, disputed_pct)
    }

    /// Get the average territories per active player
    pub fn avg_territories_per_player(&self) -> f32 {
        if self.active_players == 0 {
            0.0
        } else {
            self.owned_tiles as f32 / self.active_players as f32
        }
    }

    /// Get players ranked by territory count (descending)
    pub fn player_ranking(&self) -> Vec<(PlayerId, usize)> {
        let mut ranking: Vec<_> = self.player_territories.iter()
            .map(|(&player, &count)| (player, count))
            .collect();
        ranking.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
        ranking
    }

    /// Check if ownership distribution is balanced
    pub fn is_balanced(&self, max_dominance_threshold: f32) -> bool {
        if let Some((_, max_territories)) = self.dominant_player() {
            let dominance = max_territories as f32 / self.owned_tiles.max(1) as f32;
            dominance <= max_dominance_threshold
        } else {
            true
        }
    }

    /// Get conflict level (percentage of contested + disputed tiles)
    pub fn conflict_level(&self) -> f32 {
        if self.total_claimed_tiles == 0 {
            0.0
        } else {
            (self.contested_tiles + self.disputed_tiles) as f32 / self.total_claimed_tiles as f32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ownership_stats() {
        let mut stats = OwnershipStats::new();
        
        // Set up test data
        stats.total_chunks = 10;
        stats.total_claimed_tiles = 100;
        stats.owned_tiles = 70;
        stats.contested_tiles = 20;
        stats.disputed_tiles = 10;
        stats.active_players = 3;
        
        stats.player_territories.insert(1, 40);
        stats.player_territories.insert(2, 20);
        stats.player_territories.insert(3, 10);
        
        // Test dominant player
        let (dominant_player, territories) = stats.dominant_player().unwrap();
        assert_eq!(dominant_player, 1);
        assert_eq!(territories, 40);
        
        // Test ownership distribution
        let (owned_pct, contested_pct, disputed_pct) = stats.ownership_distribution();
        assert_eq!(owned_pct, 0.7);
        assert_eq!(contested_pct, 0.2);
        assert_eq!(disputed_pct, 0.1);
        
        // Test average territories per player
        let avg = stats.avg_territories_per_player();
        assert!((avg - 23.33).abs() < 0.01);
        
        // Test player ranking
        let ranking = stats.player_ranking();
        assert_eq!(ranking.len(), 3);
        assert_eq!(ranking[0], (1, 40));
        assert_eq!(ranking[1], (2, 20));
        assert_eq!(ranking[2], (3, 10));
        
        // Test balance check
        assert!(!stats.is_balanced(0.5)); // Player 1 has 40/70 = 57% dominance
        assert!(stats.is_balanced(0.6));  // Above 60% threshold
        
        // Test conflict level
        let conflict = stats.conflict_level();
        assert_eq!(conflict, 0.3); // (20 + 10) / 100 = 0.3
    }
}
