//! Core types and enums for tile adjacency system
//!
//! Contains type definitions, directions, adjacency relationships, and basic implementations.

use serde::{Deserialize, Serialize};
use crate::core::zig_ffi::HexCoord;
use crate::world::tiles::chunks::TileId;

/// Direction of adjacency in hex grid (6 neighbors)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum HexDirection {
    East = 0,
    Northeast = 1,
    Northwest = 2,
    West = 3,
    Southwest = 4,
    Southeast = 5,
}

impl HexDirection {
    /// Get all six hex directions
    pub const ALL: [HexDirection; 6] = [
        HexDirection::East,
        HexDirection::Northeast,
        HexDirection::Northwest,
        HexDirection::West,
        HexDirection::Southwest,
        HexDirection::Southeast,
    ];

    /// Get opposite direction
    pub fn opposite(self) -> HexDirection {
        match self {
            HexDirection::East => HexDirection::West,
            HexDirection::Northeast => HexDirection::Southwest,
            HexDirection::Northwest => HexDirection::Southeast,
            HexDirection::West => HexDirection::East,
            HexDirection::Southwest => HexDirection::Northeast,
            HexDirection::Southeast => HexDirection::Northwest,
        }
    }

    /// Get hex offset for this direction
    pub fn offset(self) -> HexCoord {
        match self {
            HexDirection::East => HexCoord { q: 1, r: 0 },
            HexDirection::Northeast => HexCoord { q: 1, r: -1 },
            HexDirection::Northwest => HexCoord { q: 0, r: -1 },
            HexDirection::West => HexCoord { q: -1, r: 0 },
            HexDirection::Southwest => HexCoord { q: -1, r: 1 },
            HexDirection::Southeast => HexCoord { q: 0, r: 1 },
        }
    }

    /// Get direction from one hex to adjacent hex
    pub fn from_hex_to_hex(from: HexCoord, to: HexCoord) -> Option<HexDirection> {
        let diff = HexCoord { q: to.q - from.q, r: to.r - from.r };
        
        for direction in Self::ALL {
            if direction.offset().q == diff.q && direction.offset().r == diff.r {
                return Some(direction);
            }
        }
        
        None // Not adjacent
    }
}

/// Adjacency relationship between two tiles
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TileAdjacency {
    /// Source tile ID
    pub from_tile: TileId,
    /// Target tile ID  
    pub to_tile: TileId,
    /// Direction from source to target
    pub direction: HexDirection,
    /// Connection strength (0.0 = blocked, 1.0 = open)
    pub connection_strength: f32,
    /// Movement cost modifier for this connection
    pub movement_cost_modifier: f32,
    /// Whether this connection is bidirectional
    pub bidirectional: bool,
}

impl TileAdjacency {
    /// Create new adjacency relationship
    pub fn new(from_tile: TileId, to_tile: TileId, direction: HexDirection) -> Self {
        Self {
            from_tile,
            to_tile,
            direction,
            connection_strength: 1.0,
            movement_cost_modifier: 1.0,
            bidirectional: true,
        }
    }

    /// Create adjacency with custom properties
    pub fn with_properties(from_tile: TileId, to_tile: TileId, direction: HexDirection, 
                          strength: f32, cost_modifier: f32, bidirectional: bool) -> Self {
        Self {
            from_tile,
            to_tile,
            direction,
            connection_strength: strength.clamp(0.0, 1.0),
            movement_cost_modifier: cost_modifier.max(0.0),
            bidirectional,
        }
    }

    /// Check if connection is passable
    pub fn is_passable(&self) -> bool {
        self.connection_strength > 0.0
    }

    /// Get effective movement cost for this connection
    pub fn effective_movement_cost(&self, base_cost: f32) -> f32 {
        if !self.is_passable() {
            return f32::INFINITY;
        }
        base_cost * self.movement_cost_modifier / self.connection_strength
    }

    /// Create reverse adjacency (for bidirectional connections)
    pub fn reverse(&self) -> Self {
        Self {
            from_tile: self.to_tile,
            to_tile: self.from_tile,
            direction: self.direction.opposite(),
            connection_strength: self.connection_strength,
            movement_cost_modifier: self.movement_cost_modifier,
            bidirectional: self.bidirectional,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_direction() {
        assert_eq!(HexDirection::East.opposite(), HexDirection::West);
        assert_eq!(HexDirection::Northeast.opposite(), HexDirection::Southwest);
        
        let east_offset = HexDirection::East.offset();
        assert_eq!(east_offset.q, 1);
        assert_eq!(east_offset.r, 0);
    }

    #[test]
    fn test_tile_adjacency() {
        let adj = TileAdjacency::new(TileId(1), TileId(2), HexDirection::East);
        assert_eq!(adj.from_tile, TileId(1));
        assert_eq!(adj.to_tile, TileId(2));
        assert_eq!(adj.direction, HexDirection::East);
        assert!(adj.is_passable());
        
        let reverse = adj.reverse();
        assert_eq!(reverse.from_tile, TileId(2));
        assert_eq!(reverse.to_tile, TileId(1));
        assert_eq!(reverse.direction, HexDirection::West);
    }
}
