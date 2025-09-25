//! Container for managing improvements on a single tile
//!
//! Contains the TileImprovements struct and related functionality.

use arrayvec::ArrayVec;
use slotmap::SlotMap;
use serde::{Deserialize, Serialize};

use crate::world::tiles::chunks::TileId;
use super::{
    types::{ImprovementKey, ImprovementType, MAX_IMPROVEMENTS_PER_TILE},
    core::Improvement,
    stats::ImprovementError,
};

/// Collection of improvements for a single tile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileImprovements {
    /// Improvements on this tile (limited capacity)
    improvements: ArrayVec<ImprovementKey, MAX_IMPROVEMENTS_PER_TILE>,
    /// Tile this collection belongs to
    tile_id: TileId,
}

impl TileImprovements {
    /// Create new tile improvements collection
    pub fn new(tile_id: TileId) -> Self {
        Self {
            improvements: ArrayVec::new(),
            tile_id,
        }
    }

    /// Add improvement to tile
    pub fn add_improvement(&mut self, key: ImprovementKey) -> Result<(), ImprovementError> {
        if self.improvements.is_full() {
            return Err(ImprovementError::TileCapacityExceeded);
        }
        
        self.improvements.push(key);
        Ok(())
    }

    /// Remove improvement from tile
    pub fn remove_improvement(&mut self, key: ImprovementKey) -> bool {
        if let Some(pos) = self.improvements.iter().position(|&k| k == key) {
            self.improvements.swap_remove(pos);
            true
        } else {
            false
        }
    }

    /// Get all improvement keys
    pub fn improvements(&self) -> &[ImprovementKey] {
        &self.improvements
    }

    /// Get number of improvements
    pub fn count(&self) -> usize {
        self.improvements.len()
    }

    /// Check if tile has specific improvement type
    pub fn has_improvement_type(&self, improvement_type: ImprovementType, improvements_map: &SlotMap<ImprovementKey, Improvement>) -> bool {
        self.improvements.iter()
            .any(|&key| {
                improvements_map.get(key)
                    .map(|imp| imp.improvement_type == improvement_type)
                    .unwrap_or(false)
            })
    }

    /// Check if tile can accept another improvement
    pub fn can_add_improvement(&self, improvement_type: ImprovementType, improvements_map: &SlotMap<ImprovementKey, Improvement>) -> bool {
        if self.improvements.is_full() {
            return false;
        }

        // Check compatibility rules
        self.is_compatible_with_existing(improvement_type, improvements_map)
    }

    /// Check if new improvement is compatible with existing ones
    fn is_compatible_with_existing(&self, new_improvement: ImprovementType, improvements_map: &SlotMap<ImprovementKey, Improvement>) -> bool {
        use ImprovementType::*;

        for &key in &self.improvements {
            if let Some(existing) = improvements_map.get(key) {
                let existing_type = existing.improvement_type;
                
                // Check incompatibility rules
                match (existing_type, new_improvement) {
                    // Only one of each type allowed
                    (Farm, Farm) | (Mine, Mine) | (Lumbermill, Lumbermill) => return false,
                    
                    // Infrastructure conflicts
                    (Road, Railroad) | (Railroad, Road) => return false, // Can't have both
                    
                    // Military conflicts
                    (Fort, Fortress) | (Fortress, Fort) => return false, // Only one fortification
                    (Barracks, Arsenal) | (Arsenal, Barracks) => return false, // Only one military building
                    
                    // Economic conflicts
                    (Market, TradingPost) | (TradingPost, Market) => return false, // Redundant commerce buildings
                    
                    // Cultural conflicts - can have multiple but check space
                    (Temple, Temple) | (Library, Library) | (Theater, Theater) => {
                        // Allow only one of each cultural building
                        return false;
                    }
                    
                    _ => {} // Compatible
                }
            }
        }

        true
    }

    /// Get improvements by category
    pub fn get_by_category(&self, category: super::types::ImprovementCategory, improvements_map: &SlotMap<ImprovementKey, Improvement>) -> Vec<ImprovementKey> {
        self.improvements.iter()
            .copied()
            .filter(|&key| {
                improvements_map.get(key)
                    .map(|imp| imp.improvement_type.category() == category)
                    .unwrap_or(false)
            })
            .collect()
    }

    /// Get all functional improvements (completed and not damaged beyond repair)
    pub fn get_functional_improvements(&self, improvements_map: &SlotMap<ImprovementKey, Improvement>) -> Vec<ImprovementKey> {
        self.improvements.iter()
            .copied()
            .filter(|&key| {
                improvements_map.get(key)
                    .map(|imp| imp.state.is_functional())
                    .unwrap_or(false)
            })
            .collect()
    }

    /// Get improvements under construction or repair
    pub fn get_in_progress_improvements(&self, improvements_map: &SlotMap<ImprovementKey, Improvement>) -> Vec<ImprovementKey> {
        self.improvements.iter()
            .copied()
            .filter(|&key| {
                improvements_map.get(key)
                    .map(|imp| imp.state.is_in_progress())
                    .unwrap_or(false)
            })
            .collect()
    }

    /// Calculate total yields from all functional improvements on this tile
    pub fn total_yields(&self, improvements_map: &SlotMap<ImprovementKey, Improvement>) -> super::core::ResourceYields {
        let mut total = super::core::ResourceYields::zero();
        
        for &key in &self.improvements {
            if let Some(improvement) = improvements_map.get(key) {
                if improvement.state.is_functional() {
                    total = total.add(improvement.get_yields());
                }
            }
        }
        
        total
    }

    /// Calculate total maintenance cost for all improvements on this tile
    pub fn total_maintenance_cost(&self, improvements_map: &SlotMap<ImprovementKey, Improvement>) -> u32 {
        self.improvements.iter()
            .filter_map(|&key| improvements_map.get(key))
            .map(|imp| imp.properties.maintenance_cost)
            .sum()
    }

    /// Get tile ID
    pub fn tile_id(&self) -> TileId {
        self.tile_id
    }

    /// Check if tile is empty (no improvements)
    pub fn is_empty(&self) -> bool {
        self.improvements.is_empty()
    }

    /// Check if tile is at capacity
    pub fn is_full(&self) -> bool {
        self.improvements.is_full()
    }

    /// Get available slots
    pub fn available_slots(&self) -> usize {
        MAX_IMPROVEMENTS_PER_TILE - self.improvements.len()
    }

    /// Clear all improvements (for tile clearing/destruction)
    pub fn clear(&mut self) {
        self.improvements.clear();
    }

    /// Get memory usage in bytes
    pub fn memory_size(&self) -> usize {
        std::mem::size_of::<Self>() + 
        self.improvements.len() * std::mem::size_of::<ImprovementKey>()
    }

    /// Validate that all improvement keys exist in the provided map
    pub fn validate_keys(&self, improvements_map: &SlotMap<ImprovementKey, Improvement>) -> Result<(), ImprovementError> {
        for &key in &self.improvements {
            if improvements_map.get(key).is_none() {
                return Err(ImprovementError::ImprovementNotFound(key));
            }
        }
        Ok(())
    }

    /// Get improvements sorted by construction date (oldest first)
    pub fn get_improvements_by_age(&self, improvements_map: &SlotMap<ImprovementKey, Improvement>) -> Vec<ImprovementKey> {
        let mut improvements = self.improvements.to_vec();
        improvements.sort_by_key(|&key| {
            improvements_map.get(key)
                .map(|imp| imp.construction_started)
                .unwrap_or(u32::MAX)
        });
        improvements
    }

    /// Find improvement by type (returns first match)
    pub fn find_by_type(&self, improvement_type: ImprovementType, improvements_map: &SlotMap<ImprovementKey, Improvement>) -> Option<ImprovementKey> {
        self.improvements.iter()
            .copied()
            .find(|&key| {
                improvements_map.get(key)
                    .map(|imp| imp.improvement_type == improvement_type)
                    .unwrap_or(false)
            })
    }

    /// Count improvements of specific type
    pub fn count_type(&self, improvement_type: ImprovementType, improvements_map: &SlotMap<ImprovementKey, Improvement>) -> usize {
        self.improvements.iter()
            .filter(|&&key| {
                improvements_map.get(key)
                    .map(|imp| imp.improvement_type == improvement_type)
                    .unwrap_or(false)
            })
            .count()
    }
}
