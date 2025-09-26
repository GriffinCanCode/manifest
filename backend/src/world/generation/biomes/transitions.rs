//! Biome Transitions
//!
//! Handles smooth transitions between different biomes for realistic boundaries.

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::world::tiles::{chunks::TileId, properties::Biome};

/// Biome transition manager
#[derive(Component, Debug, Resource, Default)]
pub struct BiomeTransitionManager {
    /// Transition rules between biome types
    transition_rules: HashMap<(String, String), TransitionType>,
    /// Transition zones by tile
    transition_zones: HashMap<TileId, BiomeTransition>,
}

/// Types of transitions between biomes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransitionType {
    /// Gradual transition over multiple tiles
    Gradual { distance: u32 },
    /// Sharp boundary between biomes
    Sharp,
    /// Ecotone - mixed biome characteristics
    Ecotone { blend_ratio: f32 },
    /// Impossible transition (enforced separation)
    Blocked,
}

/// Biome transition data for a tile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeTransition {
    /// Primary biome for this tile
    pub primary_biome: String,
    /// Secondary biome influence (for transitions)
    pub secondary_biome: Option<String>,
    /// Transition strength (0.0 = pure primary, 1.0 = pure secondary)
    pub transition_strength: f32,
    /// Transition type
    pub transition_type: TransitionType,
}

impl BiomeTransitionManager {
    /// Create new transition manager with default rules
    pub fn new() -> Self {
        let mut manager = Self::default();
        manager.setup_default_transition_rules();
        manager
    }
    
    /// Set up realistic biome transition rules
    fn setup_default_transition_rules(&mut self) {
        // Tropical transitions
        self.add_transition_rule("tropical_rainforest", "tropical_grassland", 
                                TransitionType::Gradual { distance: 3 });
        self.add_transition_rule("tropical_grassland", "savanna",
                                TransitionType::Gradual { distance: 2 });
        
        // Temperate transitions
        self.add_transition_rule("temperate_forest", "temperate_grassland",
                                TransitionType::Ecotone { blend_ratio: 0.3 });
        self.add_transition_rule("temperate_grassland", "steppe",
                                TransitionType::Gradual { distance: 4 });
        
        // Desert transitions
        self.add_transition_rule("desert", "steppe", 
                                TransitionType::Gradual { distance: 5 });
        self.add_transition_rule("steppe", "temperate_grassland",
                                TransitionType::Gradual { distance: 3 });
        
        // Mountain transitions
        self.add_transition_rule("alpine", "temperate_forest",
                                TransitionType::Sharp);
        self.add_transition_rule("alpine", "tundra",
                                TransitionType::Gradual { distance: 2 });
        
        // Arctic transitions
        self.add_transition_rule("tundra", "taiga",
                                TransitionType::Gradual { distance: 4 });
        self.add_transition_rule("taiga", "temperate_forest",
                                TransitionType::Gradual { distance: 3 });
        
        // Impossible transitions (blocks)
        self.add_transition_rule("tropical_rainforest", "tundra", TransitionType::Blocked);
        self.add_transition_rule("desert", "tropical_rainforest", TransitionType::Blocked);
        self.add_transition_rule("polar_desert", "tropical_rainforest", TransitionType::Blocked);
    }
    
    /// Add transition rule between two biome types
    pub fn add_transition_rule(&mut self, from: &str, to: &str, transition_type: TransitionType) {
        let key1 = (from.to_string(), to.to_string());
        let key2 = (to.to_string(), from.to_string()); // Bidirectional
        
        self.transition_rules.insert(key1, transition_type.clone());
        self.transition_rules.insert(key2, transition_type);
    }
    
    /// Get transition type between two biomes
    pub fn get_transition_type(&self, biome1: &str, biome2: &str) -> TransitionType {
        self.transition_rules
            .get(&(biome1.to_string(), biome2.to_string()))
            .cloned()
            .unwrap_or(TransitionType::Sharp) // Default to sharp transition
    }
    
    /// Calculate biome transition for a tile based on neighbors
    pub fn calculate_transition(
        &self,
        tile_id: TileId,
        primary_biome: &str,
        neighbor_biomes: &[(TileId, String, f32)], // (tile_id, biome, distance)
    ) -> Option<BiomeTransition> {
        if neighbor_biomes.is_empty() {
            return Some(BiomeTransition {
                primary_biome: primary_biome.to_string(),
                secondary_biome: None,
                transition_strength: 0.0,
                transition_type: TransitionType::Sharp,
            });
        }
        
        // Find the most common neighboring biome different from primary
        let mut biome_weights: HashMap<String, f32> = HashMap::new();
        
        for (_, neighbor_biome, distance) in neighbor_biomes {
            if neighbor_biome != primary_biome {
                let weight = 1.0 / (distance + 1.0); // Closer neighbors have more influence
                *biome_weights.entry(neighbor_biome.clone()).or_insert(0.0) += weight;
            }
        }
        
        if biome_weights.is_empty() {
            // No different neighbors, pure biome
            return Some(BiomeTransition {
                primary_biome: primary_biome.to_string(),
                secondary_biome: None,
                transition_strength: 0.0,
                transition_type: TransitionType::Sharp,
            });
        }
        
        // Get the most influential secondary biome
        let (secondary_biome, influence) = biome_weights
            .into_iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap();
        
        // Get transition type
        let transition_type = self.get_transition_type(primary_biome, &secondary_biome);
        
        // Calculate transition strength based on type and influence
        let transition_strength = match &transition_type {
            TransitionType::Gradual { distance } => {
                (influence * 2.0).min(1.0) / (*distance as f32)
            }
            TransitionType::Sharp => 0.0, // No transition
            TransitionType::Ecotone { blend_ratio } => influence * blend_ratio,
            TransitionType::Blocked => 0.0, // No transition allowed
        };
        
        Some(BiomeTransition {
            primary_biome: primary_biome.to_string(),
            secondary_biome: Some(secondary_biome),
            transition_strength,
            transition_type,
        })
    }
    
    /// Apply transition to modify biome properties
    pub fn apply_transition_to_biome(&self, biome: &mut Biome, transition: &BiomeTransition) {
        if let Some(secondary_biome) = &transition.secondary_biome {
            if transition.transition_strength > 0.1 { // Only apply significant transitions
                // Modify biome type to indicate transition
                match transition.transition_type {
                    TransitionType::Ecotone { .. } => {
                        biome.biome_type = format!("{}-{} ecotone", 
                                                 transition.primary_biome, 
                                                 secondary_biome);
                    }
                    TransitionType::Gradual { .. } => {
                        if transition.transition_strength > 0.5 {
                            biome.biome_type = format!("{} transitioning to {}", 
                                                     transition.primary_biome,
                                                     secondary_biome);
                        }
                    }
                    _ => {} // No biome type change for sharp or blocked transitions
                }
                
                // Interpolate modifiers based on transition strength
                // This would require access to secondary biome modifiers
                let blend = transition.transition_strength;
                
                // Simplified modifier blending (would be more sophisticated with actual data)
                biome.modifiers.movement_cost_multiplier = 
                    biome.modifiers.movement_cost_multiplier * (1.0 - blend) + 
                    1.2 * blend; // Assume average modifier for secondary biome
                
                biome.modifiers.defense_bonus = 
                    biome.modifiers.defense_bonus * (1.0 - blend) + 
                    0.1 * blend;
                
                // Reduce suitability slightly for transition zones
                biome.suitability_score *= 1.0 - (blend * 0.2);
            }
        }
    }
    
    /// Update transition for a tile
    pub fn update_transition(&mut self, tile_id: TileId, transition: BiomeTransition) {
        self.transition_zones.insert(tile_id, transition);
    }
    
    /// Get transition for a tile
    pub fn get_transition(&self, tile_id: TileId) -> Option<&BiomeTransition> {
        self.transition_zones.get(&tile_id)
    }
    
    /// Remove transition data for a tile
    pub fn remove_transition(&mut self, tile_id: TileId) -> Option<BiomeTransition> {
        self.transition_zones.remove(&tile_id)
    }
    
    /// Get all transitions
    pub fn transitions(&self) -> &HashMap<TileId, BiomeTransition> {
        &self.transition_zones
    }
    
    /// Clear all transitions
    pub fn clear_transitions(&mut self) {
        self.transition_zones.clear();
    }
    
    /// Check if transition is allowed between two biomes
    pub fn is_transition_allowed(&self, from: &str, to: &str) -> bool {
        !matches!(self.get_transition_type(from, to), TransitionType::Blocked)
    }
    
    /// Get transition distance for gradual transitions
    pub fn get_transition_distance(&self, from: &str, to: &str) -> Option<u32> {
        match self.get_transition_type(from, to) {
            TransitionType::Gradual { distance } => Some(distance),
            _ => None,
        }
    }
    
    /// Get statistics about transitions
    pub fn get_transition_stats(&self) -> TransitionStats {
        let total_transitions = self.transition_zones.len();
        let mut by_type = HashMap::new();
        let mut avg_strength = 0.0;
        
        for transition in self.transition_zones.values() {
            let type_name = match &transition.transition_type {
                TransitionType::Gradual { .. } => "Gradual",
                TransitionType::Sharp => "Sharp",
                TransitionType::Ecotone { .. } => "Ecotone", 
                TransitionType::Blocked => "Blocked",
            };
            
            *by_type.entry(type_name).or_insert(0u32) += 1;
            avg_strength += transition.transition_strength;
        }
        
        if total_transitions > 0 {
            avg_strength /= total_transitions as f32;
        }
        
        TransitionStats {
            total_transitions,
            by_type,
            average_strength: avg_strength,
        }
    }
}

/// Transition statistics
#[derive(Debug)]
pub struct TransitionStats {
    pub total_transitions: usize,
    pub by_type: HashMap<&'static str, u32>,
    pub average_strength: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_transition_manager_creation() {
        let manager = BiomeTransitionManager::new();
        assert!(!manager.transition_rules.is_empty());
    }
    
    #[test]
    fn test_transition_rules() {
        let manager = BiomeTransitionManager::new();
        
        // Test allowed transition
        let transition = manager.get_transition_type("temperate_forest", "temperate_grassland");
        assert!(matches!(transition, TransitionType::Ecotone { .. }));
        
        // Test blocked transition
        let blocked = manager.get_transition_type("tropical_rainforest", "tundra");
        assert!(matches!(blocked, TransitionType::Blocked));
        
        // Test bidirectional
        let reverse = manager.get_transition_type("temperate_grassland", "temperate_forest");
        assert!(matches!(reverse, TransitionType::Ecotone { .. }));
    }
    
    #[test]
    fn test_transition_calculation() {
        let manager = BiomeTransitionManager::new();
        
        let neighbors = vec![
            (TileId::new(1), "temperate_grassland".to_string(), 1.0),
            (TileId::new(2), "temperate_grassland".to_string(), 1.0),
            (TileId::new(3), "steppe".to_string(), 1.5),
        ];
        
        let transition = manager.calculate_transition(
            TileId::new(0),
            "temperate_forest",
            &neighbors,
        );
        
        assert!(transition.is_some());
        let transition = transition.unwrap();
        assert_eq!(transition.primary_biome, "temperate_forest");
        assert!(transition.secondary_biome.is_some());
    }
    
    #[test]
    fn test_transition_stats() {
        let mut manager = BiomeTransitionManager::new();
        
        // Add some test transitions
        manager.update_transition(TileId::new(0), BiomeTransition {
            primary_biome: "forest".to_string(),
            secondary_biome: Some("grassland".to_string()),
            transition_strength: 0.3,
            transition_type: TransitionType::Gradual { distance: 2 },
        });
        
        manager.update_transition(TileId::new(1), BiomeTransition {
            primary_biome: "grassland".to_string(),
            secondary_biome: None,
            transition_strength: 0.0,
            transition_type: TransitionType::Sharp,
        });
        
        let stats = manager.get_transition_stats();
        assert_eq!(stats.total_transitions, 2);
        assert!(stats.by_type.contains_key("Gradual"));
        assert!(stats.by_type.contains_key("Sharp"));
    }
}
