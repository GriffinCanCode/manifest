//! Type definitions for tile hierarchy system
//!
//! Contains all data structures, enums, and basic implementations
//! for the hierarchical tile organization system.

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::core::{
    zig_ffi::HexCoord,
    hashing::FastHashMap,
};
use crate::ecs::hierarchy::{HierarchyValidation, RelationshipType};
use crate::world::tiles::chunks::TileId;

/// Tile-specific relationship types extending the base relationship system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TileRelationshipType {
    /// Parent region contains child tiles (multi-resolution)
    RegionParent,
    /// Child tile belongs to parent region
    RegionChild,
    /// Tile influences adjacent tiles (cultural, economic spread)
    Influence,
    /// Tile borders another tile (spatial adjacency)
    Adjacent,
    /// Tile is part of a river system
    RiverFlow,
    /// Tile shares resources with another
    ResourceShare,
    /// Tile is part of a trade route
    TradeRoute,
}

impl TileRelationshipType {
    /// Convert to base relationship type for hierarchy system integration
    pub fn to_base_relationship(self) -> RelationshipType {
        match self {
            Self::RegionParent => RelationshipType::Parent,
            Self::RegionChild => RelationshipType::Child,
            Self::Influence => RelationshipType::Attachment,
            Self::Adjacent => RelationshipType::Attachment,
            Self::RiverFlow => RelationshipType::Dependency,
            Self::ResourceShare => RelationshipType::Dependency,
            Self::TradeRoute => RelationshipType::Dependency,
        }
    }

    /// Check if this relationship type is spatial (affects neighbor calculations)
    pub fn is_spatial(self) -> bool {
        matches!(self, Self::Adjacent | Self::Influence)
    }

    /// Check if this relationship type is hierarchical (affects resolution)
    pub fn is_hierarchical(self) -> bool {
        matches!(self, Self::RegionParent | Self::RegionChild)
    }
}

/// Multi-resolution tile entity for hierarchical organization
#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub struct HierarchicalTile {
    /// Base tile ID at highest resolution
    pub base_tile_id: TileId,
    /// Hex coordinate in world space
    pub hex: HexCoord,
    /// Resolution level (0 = highest detail, higher = more aggregated)
    pub resolution: u8,
    /// Aggregated area covered by this tile (in base tiles)
    pub coverage_area: u16,
    /// Spatial bounds of this hierarchical tile
    pub bounds: HexBounds,
}

impl HierarchicalTile {
    /// Create new hierarchical tile
    pub fn new(base_tile_id: TileId, hex: HexCoord, resolution: u8) -> Self {
        let coverage_area = Self::calculate_coverage_area(resolution);
        let bounds = HexBounds::from_center_and_radius(hex, coverage_area as u32 / 2);
        
        Self {
            base_tile_id,
            hex,
            resolution,
            coverage_area,
            bounds,
        }
    }

    /// Calculate how many base tiles this hierarchical tile covers
    fn calculate_coverage_area(resolution: u8) -> u16 {
        // Each resolution level increases coverage by factor of 4 (2x2 in hex space)
        4_u16.pow(resolution as u32).min(u16::MAX)
    }

    /// Check if this hierarchical tile contains the given hex coordinate
    pub fn contains_hex(&self, hex: HexCoord) -> bool {
        self.bounds.contains(hex)
    }

    /// Get all child tile coordinates at next resolution level
    pub fn get_child_hexes(&self) -> Vec<HexCoord> {
        if self.resolution == 0 {
            return vec![self.hex]; // Base resolution
        }

        let radius = (self.coverage_area as i32) / 4; // Next level down
        let mut child_hexes = Vec::new();
        
        // Generate hex ring pattern for child tiles
        for q_offset in -radius..=radius {
            for r_offset in -radius..=radius {
                let s_offset = -q_offset - r_offset;
                if s_offset.abs() <= radius {
                    child_hexes.push(HexCoord {
                        q: self.hex.q + q_offset,
                        r: self.hex.r + r_offset,
                    });
                }
            }
        }

        child_hexes
    }

    /// Get memory usage in bytes
    pub fn memory_size(&self) -> usize {
        std::mem::size_of::<Self>()
    }
}

/// Spatial bounds in hex coordinate space
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct HexBounds {
    pub min_q: i32,
    pub max_q: i32,
    pub min_r: i32,
    pub max_r: i32,
}

impl HexBounds {
    /// Create bounds from center and radius
    pub fn from_center_and_radius(center: HexCoord, radius: u32) -> Self {
        let r = radius as i32;
        Self {
            min_q: center.q - r,
            max_q: center.q + r,
            min_r: center.r - r,
            max_r: center.r + r,
        }
    }

    /// Check if bounds contain the given hex coordinate
    pub fn contains(&self, hex: HexCoord) -> bool {
        hex.q >= self.min_q && hex.q <= self.max_q &&
        hex.r >= self.min_r && hex.r <= self.max_r
    }

    /// Calculate area covered by bounds
    pub fn area(&self) -> u32 {
        let width = (self.max_q - self.min_q + 1) as u32;
        let height = (self.max_r - self.min_r + 1) as u32;
        width * height
    }
}

/// Statistics for tile hierarchy performance monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileHierarchyStats {
    pub resolution_counts: FastHashMap<u8, usize>,
    pub total_hierarchical_tiles: usize,
    pub max_resolution: u8,
    pub cache_hit_rate: f32,
}

/// Validation results for tile hierarchy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileHierarchyValidation {
    pub base_validation: HierarchyValidation,
    pub resolution_levels: u8,
    pub has_resolution_gaps: bool,
    pub total_hierarchical_entities: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hierarchical_tile_creation() {
        let hex = HexCoord { q: 10, r: 20 };
        let tile = HierarchicalTile::new(TileId(123), hex, 2);
        
        assert_eq!(tile.base_tile_id, TileId(123));
        assert_eq!(tile.hex, hex);
        assert_eq!(tile.resolution, 2);
        assert_eq!(tile.coverage_area, 16); // 4^2
        assert!(tile.contains_hex(hex));
    }

    #[test]
    fn test_hex_bounds() {
        let center = HexCoord { q: 0, r: 0 };
        let bounds = HexBounds::from_center_and_radius(center, 2);
        
        assert!(bounds.contains(HexCoord { q: 1, r: 1 }));
        assert!(bounds.contains(HexCoord { q: -2, r: -2 }));
        assert!(!bounds.contains(HexCoord { q: 3, r: 3 }));
        assert_eq!(bounds.area(), 25); // 5x5
    }

    #[test]
    fn test_tile_relationship_type_conversion() {
        assert_eq!(TileRelationshipType::RegionParent.to_base_relationship(), RelationshipType::Parent);
        assert_eq!(TileRelationshipType::Adjacent.to_base_relationship(), RelationshipType::Attachment);
        assert!(TileRelationshipType::Adjacent.is_spatial());
        assert!(TileRelationshipType::RegionParent.is_hierarchical());
    }

    #[test]
    fn test_child_hex_generation() {
        let parent = HierarchicalTile::new(TileId(1), HexCoord { q: 0, r: 0 }, 1);
        let child_hexes = parent.get_child_hexes();
        
        assert!(!child_hexes.is_empty());
        assert!(child_hexes.contains(&HexCoord { q: 0, r: 0 })); // Should include center
    }
}
