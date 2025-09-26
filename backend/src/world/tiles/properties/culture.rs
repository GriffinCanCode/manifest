//! Cultural influence system with concurrent access
//!
//! Provides cultural influence tracking with dashmap for efficient
//! concurrent access and cultural spread mechanics.

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use dashmap::DashMap;
use std::collections::HashMap;

use crate::world::tiles::{
    chunks::TileId,
    ownership::PlayerId
};

/// Cultural influence with dashmap for concurrent access
#[derive(Debug, Resource)]
pub struct CulturalInfluence {
    /// Cultural influence values by tile and player
    influence_map: DashMap<TileId, PlayerCulture>,
    /// Culture spread rate
    spread_rate: f32,
    /// Maximum influence distance
    max_distance: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerCulture {
    /// Influence strength by player (0.0 to 1.0)
    pub influences: HashMap<PlayerId, f32>,
    /// Dominant culture
    pub dominant_player: Option<PlayerId>,
    /// Cultural conversion pressure
    pub conversion_pressure: f32,
}

impl Default for CulturalInfluence {
    fn default() -> Self {
        Self {
            influence_map: DashMap::new(),
            spread_rate: 0.01,
            max_distance: 5,
        }
    }
}

impl CulturalInfluence {
    /// Create new cultural influence system
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with custom parameters
    pub fn with_params(spread_rate: f32, max_distance: u32) -> Self {
        Self {
            influence_map: DashMap::new(),
            spread_rate,
            max_distance,
        }
    }

    /// Get cultural influence for a tile
    pub fn get_influence(&self, tile_id: TileId) -> Option<PlayerCulture> {
        self.influence_map.get(&tile_id).map(|entry| entry.clone())
    }

    /// Set cultural influence for a tile
    pub fn set_influence(&self, tile_id: TileId, culture: PlayerCulture) {
        self.influence_map.insert(tile_id, culture);
    }

    /// Add influence for a specific player
    pub fn add_player_influence(&self, tile_id: TileId, player_id: PlayerId, amount: f32) {
        let mut entry = self.influence_map.entry(tile_id).or_insert_with(|| PlayerCulture {
            influences: HashMap::new(),
            dominant_player: None,
            conversion_pressure: 0.0,
        });
        
        let current = entry.influences.get(&player_id).unwrap_or(&0.0);
        let new_influence = (current + amount).min(1.0).max(0.0);
        entry.influences.insert(player_id, new_influence);
        
        // Update dominant player
        entry.dominant_player = entry.influences.iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(player, _)| *player);
    }

    /// Remove player influence
    pub fn remove_player_influence(&self, tile_id: TileId, player_id: PlayerId) {
        if let Some(mut entry) = self.influence_map.get_mut(&tile_id) {
            entry.influences.remove(&player_id);
            
            // Update dominant player
            entry.dominant_player = entry.influences.iter()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(player, _)| *player);
        }
    }

    /// Get dominant culture for tile
    pub fn get_dominant_culture(&self, tile_id: TileId) -> Option<PlayerId> {
        self.influence_map.get(&tile_id)?.dominant_player
    }

    /// Get culture strength for player on tile
    pub fn get_culture_strength(&self, tile_id: TileId, player_id: PlayerId) -> f32 {
        self.influence_map.get(&tile_id)
            .and_then(|entry| entry.influences.get(&player_id).copied())
            .unwrap_or(0.0)
    }

    /// Apply cultural conversion pressure
    pub fn apply_conversion_pressure(&self, tile_id: TileId, pressure: f32) {
        if let Some(mut entry) = self.influence_map.get_mut(&tile_id) {
            entry.conversion_pressure += pressure;
            
            // If pressure is high enough, start converting weaker influences
            if entry.conversion_pressure > 1.0 {
                self.process_cultural_conversion(&mut entry);
            }
        }
    }

    /// Process cultural conversion based on pressure
    fn process_cultural_conversion(&self, culture: &mut PlayerCulture) {
        if let Some(dominant) = culture.dominant_player {
            let dominant_strength = culture.influences.get(&dominant).copied().unwrap_or(0.0);
            
            // Reduce weaker influences
            for (player_id, influence) in culture.influences.iter_mut() {
                if *player_id != dominant {
                    let conversion_amount = culture.conversion_pressure * 0.1 * dominant_strength;
                    *influence = (*influence - conversion_amount).max(0.0);
                }
            }
            
            // Remove influences that became too weak
            culture.influences.retain(|_, influence| *influence > 0.01);
            
            // Reset pressure
            culture.conversion_pressure = 0.0;
        }
    }

    /// Spread culture to neighboring tiles
    pub fn spread_culture(&self, source_tile: TileId, target_tiles: &[TileId], distance: u32) {
        if distance > self.max_distance {
            return;
        }

        if let Some(source_culture) = self.get_influence(source_tile) {
            if let Some(dominant) = source_culture.dominant_player {
                let source_strength = source_culture.influences.get(&dominant).copied().unwrap_or(0.0);
                let spread_amount = self.spread_rate * source_strength * (1.0 / (distance as f32 + 1.0));
                
                for &target_tile in target_tiles {
                    self.add_player_influence(target_tile, dominant, spread_amount);
                }
            }
        }
    }

    /// Get cultural diversity for a tile (0.0 = mono-culture, 1.0 = very diverse)
    pub fn get_cultural_diversity(&self, tile_id: TileId) -> f32 {
        if let Some(culture) = self.get_influence(tile_id) {
            if culture.influences.is_empty() {
                return 0.0;
            }

            // Calculate Shannon diversity index
            let total: f32 = culture.influences.values().sum();
            if total <= 0.0 {
                return 0.0;
            }

            let mut diversity = 0.0;
            for influence in culture.influences.values() {
                if *influence > 0.0 {
                    let proportion = influence / total;
                    diversity -= proportion * proportion.ln();
                }
            }

            diversity / (culture.influences.len() as f32).ln()
        } else {
            0.0
        }
    }

    /// Check if tile has cultural unrest
    pub fn has_cultural_unrest(&self, tile_id: TileId) -> bool {
        if let Some(culture) = self.get_influence(tile_id) {
            // Unrest occurs when there are competing strong influences
            let strong_influences = culture.influences.values()
                .filter(|&&influence| influence > 0.3)
                .count();
            
            strong_influences > 1 || culture.conversion_pressure > 0.5
        } else {
            false
        }
    }

    /// Get cultural border strength between two tiles
    pub fn get_border_strength(&self, tile1: TileId, tile2: TileId) -> f32 {
        let culture1 = self.get_influence(tile1);
        let culture2 = self.get_influence(tile2);

        match (culture1, culture2) {
            (Some(c1), Some(c2)) => {
                if c1.dominant_player == c2.dominant_player {
                    0.0 // Same culture, no border
                } else {
                    // Calculate border strength based on influence differences
                    let dominant1 = c1.dominant_player
                        .and_then(|p| c1.influences.get(&p))
                        .copied()
                        .unwrap_or(0.0);
                    let dominant2 = c2.dominant_player
                        .and_then(|p| c2.influences.get(&p))
                        .copied()
                        .unwrap_or(0.0);
                    
                    (dominant1 + dominant2) / 2.0
                }
            },
            (Some(_), None) | (None, Some(_)) => 0.5, // One cultural, one neutral
            (None, None) => 0.0, // No cultures
        }
    }

    /// Get total number of influenced tiles
    pub fn total_influenced_tiles(&self) -> usize {
        self.influence_map.len()
    }

    /// Get all tiles influenced by a player
    pub fn get_player_tiles(&self, player_id: PlayerId) -> Vec<TileId> {
        self.influence_map.iter()
            .filter_map(|entry| {
                let (tile_id, culture) = entry.pair();
                if culture.influences.contains_key(&player_id) {
                    Some(*tile_id)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Clear all cultural influence
    pub fn clear_all(&self) {
        self.influence_map.clear();
    }

    /// Get spread rate
    pub fn spread_rate(&self) -> f32 {
        self.spread_rate
    }

    /// Set spread rate
    pub fn set_spread_rate(&mut self, rate: f32) {
        self.spread_rate = rate.max(0.0).min(1.0);
    }

    /// Get max distance
    pub fn max_distance(&self) -> u32 {
        self.max_distance
    }

    /// Set max distance
    pub fn set_max_distance(&mut self, distance: u32) {
        self.max_distance = distance;
    }
}

impl PlayerCulture {
    /// Create new player culture
    pub fn new() -> Self {
        Self {
            influences: HashMap::new(),
            dominant_player: None,
            conversion_pressure: 0.0,
        }
    }

    /// Create with dominant player
    pub fn with_dominant(player_id: PlayerId, strength: f32) -> Self {
        let mut influences = HashMap::new();
        influences.insert(player_id, strength);
        
        Self {
            influences,
            dominant_player: Some(player_id),
            conversion_pressure: 0.0,
        }
    }

    /// Check if culture is stable (low conversion pressure)
    pub fn is_stable(&self) -> bool {
        self.conversion_pressure < 0.3
    }

    /// Get total cultural strength
    pub fn total_strength(&self) -> f32 {
        self.influences.values().sum()
    }

    /// Get number of cultural influences
    pub fn influence_count(&self) -> usize {
        self.influences.len()
    }
}

impl Default for PlayerCulture {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cultural_influence_creation() {
        let influence = CulturalInfluence::new();
        assert_eq!(influence.total_influenced_tiles(), 0);
        assert_eq!(influence.spread_rate(), 0.01);
        assert_eq!(influence.max_distance(), 5);
    }

    #[test]
    fn test_player_influence_operations() {
        let influence = CulturalInfluence::new();
        let tile_id = TileId(1);
        let player_id = 1;

        // Add influence
        influence.add_player_influence(tile_id, player_id, 0.5);
        assert_eq!(influence.get_culture_strength(tile_id, player_id), 0.5);
        assert_eq!(influence.get_dominant_culture(tile_id), Some(player_id));

        // Add more influence
        influence.add_player_influence(tile_id, player_id, 0.3);
        assert_eq!(influence.get_culture_strength(tile_id, player_id), 0.8);

        // Remove influence
        influence.remove_player_influence(tile_id, player_id);
        assert_eq!(influence.get_culture_strength(tile_id, player_id), 0.0);
    }

    #[test]
    fn test_dominant_culture() {
        let influence = CulturalInfluence::new();
        let tile_id = TileId(1);

        influence.add_player_influence(tile_id, 1, 0.3);
        influence.add_player_influence(tile_id, 2, 0.7);
        influence.add_player_influence(tile_id, 3, 0.2);

        assert_eq!(influence.get_dominant_culture(tile_id), Some(2));
    }

    #[test]
    fn test_cultural_diversity() {
        let influence = CulturalInfluence::new();
        let tile_id = TileId(1);

        // Mono-culture
        influence.add_player_influence(tile_id, 1, 1.0);
        assert!(influence.get_cultural_diversity(tile_id) < 0.1);

        // Add diversity
        let tile_id2 = TileId(2);
        influence.add_player_influence(tile_id2, 1, 0.5);
        influence.add_player_influence(tile_id2, 2, 0.3);
        influence.add_player_influence(tile_id2, 3, 0.2);
        assert!(influence.get_cultural_diversity(tile_id2) > 0.5);
    }

    #[test]
    fn test_cultural_unrest() {
        let influence = CulturalInfluence::new();
        let tile_id = TileId(1);

        // No unrest with single strong culture
        influence.add_player_influence(tile_id, 1, 0.8);
        assert!(!influence.has_cultural_unrest(tile_id));

        // Unrest with competing strong cultures
        influence.add_player_influence(tile_id, 2, 0.6);
        assert!(influence.has_cultural_unrest(tile_id));
    }

    #[test]
    fn test_conversion_pressure() {
        let influence = CulturalInfluence::new();
        let tile_id = TileId(1);

        influence.add_player_influence(tile_id, 1, 0.8);
        influence.add_player_influence(tile_id, 2, 0.3);

        // Apply conversion pressure
        influence.apply_conversion_pressure(tile_id, 1.5);

        // Weaker influence should be reduced
        assert!(influence.get_culture_strength(tile_id, 2) < 0.3);
    }

    #[test]
    fn test_culture_spread() {
        let influence = CulturalInfluence::new();
        let source_tile = TileId(1);
        let target_tile = TileId(2);

        influence.add_player_influence(source_tile, 1, 0.8);
        
        influence.spread_culture(source_tile, &[target_tile], 1);
        
        // Target should have some influence now
        assert!(influence.get_culture_strength(target_tile, 1) > 0.0);
    }

    #[test]
    fn test_border_strength() {
        let influence = CulturalInfluence::new();
        let tile1 = TileId(1);
        let tile2 = TileId(2);

        // Same culture - no border
        influence.add_player_influence(tile1, 1, 0.8);
        influence.add_player_influence(tile2, 1, 0.7);
        assert_eq!(influence.get_border_strength(tile1, tile2), 0.0);

        // Different cultures - strong border
        influence.add_player_influence(tile2, 2, 0.9);
        assert!(influence.get_border_strength(tile1, tile2) > 0.0);
    }

    #[test]
    fn test_player_culture() {
        let culture = PlayerCulture::with_dominant(1, 0.8);
        assert_eq!(culture.dominant_player, Some(1));
        assert_eq!(culture.total_strength(), 0.8);
        assert_eq!(culture.influence_count(), 1);
        assert!(culture.is_stable());
    }

    #[test]
    fn test_player_tiles() {
        let influence = CulturalInfluence::new();
        
        influence.add_player_influence(TileId(1), 1, 0.5);
        influence.add_player_influence(TileId(2), 1, 0.3);
        influence.add_player_influence(TileId(3), 2, 0.8);
        
        let player1_tiles = influence.get_player_tiles(1);
        assert_eq!(player1_tiles.len(), 2);
        assert!(player1_tiles.contains(&TileId(1)));
        assert!(player1_tiles.contains(&TileId(2)));
    }
}
