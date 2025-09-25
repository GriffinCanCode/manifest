//! Modular tile adjacency system
//!
//! This module has been refactored from a large monolithic file into focused submodules:
//! - `types`: Core types, enums, and adjacency relationships (HexDirection, TileAdjacency, etc.)
//! - `core`: Main TileAdjacencyGraph struct and implementation
//! - `stats`: Statistics, results, and error types
//! - `systems`: Bevy ECS systems for adjacency management

pub mod types;
pub mod core;
pub mod stats;
pub mod systems;

// Re-export commonly used types and functions
pub use types::{
    HexDirection, TileAdjacency
};

pub use core::TileAdjacencyGraph;

pub use stats::{
    AdjacencyStats, AdjacencyError, AdjacencyResult
};

pub use systems::{
    maintain_adjacency_system, update_adjacency_system
};

// Convenient type aliases
pub type AdjacencyManagerResult<T> = Result<T, AdjacencyError>;

/// Trait for objects that can have adjacency relationships
pub trait Adjacent {
    /// Get adjacent tiles
    fn get_adjacent_tiles(&self) -> Vec<crate::world::tiles::chunks::TileId>;
    
    /// Check if two objects are adjacent
    fn is_adjacent_to(&self, other: &Self) -> bool;
    
    /// Get adjacency strength/connection quality
    fn adjacency_strength(&self, other: &Self) -> Option<f32>;
}

/// Trait for pathfinding through adjacency graphs
pub trait Pathfinder {
    /// Find shortest path between two points
    fn find_path(&self, from: crate::world::tiles::chunks::TileId, to: crate::world::tiles::chunks::TileId) -> Option<Vec<crate::world::tiles::chunks::TileId>>;
    
    /// Find path with maximum distance constraint
    fn find_path_bounded(&self, from: crate::world::tiles::chunks::TileId, to: crate::world::tiles::chunks::TileId, max_distance: u32) -> Option<Vec<crate::world::tiles::chunks::TileId>>;
    
    /// Check if two points are connected
    fn is_connected(&self, from: crate::world::tiles::chunks::TileId, to: crate::world::tiles::chunks::TileId) -> bool;
}

impl Pathfinder for TileAdjacencyGraph {
    fn find_path(&self, from: crate::world::tiles::chunks::TileId, to: crate::world::tiles::chunks::TileId) -> Option<Vec<crate::world::tiles::chunks::TileId>> {
        // Use async runtime to call the async method
        tokio::runtime::Handle::try_current()
            .ok()
            .and_then(|handle| {
                handle.block_on(self.find_path(from, to, 100))
            })
    }
    
    fn find_path_bounded(&self, from: crate::world::tiles::chunks::TileId, to: crate::world::tiles::chunks::TileId, max_distance: u32) -> Option<Vec<crate::world::tiles::chunks::TileId>> {
        tokio::runtime::Handle::try_current()
            .ok()
            .and_then(|handle| {
                handle.block_on(self.find_path(from, to, max_distance))
            })
    }
    
    fn is_connected(&self, from: crate::world::tiles::chunks::TileId, to: crate::world::tiles::chunks::TileId) -> bool {
        // Use a reasonable default max depth for connectivity checks
        let runtime = tokio::runtime::Handle::try_current()
            .map(|handle| handle.block_on(self.find_path(from, to, 100)))
            .unwrap_or(None);
        runtime.is_some()
    }
}

/// Utility functions for adjacency management
pub mod utils {
    use super::*;
    use crate::core::zig_ffi::HexCoord;

    /// Calculate distance between two hex coordinates
    pub fn hex_distance(a: HexCoord, b: HexCoord) -> i32 {
        (
            (a.q - b.q).abs() + 
            (a.q + a.r - b.q - b.r).abs() + 
            (a.r - b.r).abs()
        ) / 2
    }

    /// Get all hex coordinates within a given radius
    pub fn hex_coordinates_in_radius(center: HexCoord, radius: i32) -> Vec<HexCoord> {
        let mut coordinates = Vec::new();
        
        for q in -radius..=radius {
            let r1 = (-radius - q).max(-radius);
            let r2 = (-radius - q).min(radius);
            
            for r in r1..=r2 {
                coordinates.push(HexCoord { 
                    q: center.q + q, 
                    r: center.r + r 
                });
            }
        }
        
        coordinates
    }

    /// Get hex coordinates in a ring at specific distance
    pub fn hex_ring(center: HexCoord, radius: i32) -> Vec<HexCoord> {
        if radius == 0 {
            return vec![center];
        }
        
        let mut ring = Vec::new();
        let mut hex = HexCoord {
            q: center.q - radius,
            r: center.r + radius,
        };
        
        for direction in types::HexDirection::ALL {
            for _ in 0..radius {
                ring.push(hex);
                let offset = direction.offset();
                hex.q += offset.q;
                hex.r += offset.r;
            }
        }
        
        ring
    }

    /// Check if hex coordinate is valid (within reasonable bounds)
    pub fn is_valid_hex_coordinate(coord: HexCoord) -> bool {
        const MAX_COORDINATE: i32 = 10000;
        coord.q.abs() <= MAX_COORDINATE && coord.r.abs() <= MAX_COORDINATE
    }

    /// Convert hex coordinate to array index (if using flat array storage)
    pub fn hex_to_array_index(coord: HexCoord, map_width: i32) -> Option<usize> {
        if coord.q < 0 || coord.r < 0 {
            return None;
        }
        
        let index = coord.r * map_width + coord.q;
        if index >= 0 {
            Some(index as usize)
        } else {
            None
        }
    }

    /// Get neighbor coordinates for a hex
    pub fn get_hex_neighbors(coord: HexCoord) -> [HexCoord; 6] {
        types::HexDirection::ALL.map(|dir| {
            let offset = dir.offset();
            HexCoord {
                q: coord.q + offset.q,
                r: coord.r + offset.r,
            }
        })
    }

    /// Calculate movement cost between adjacent tiles based on terrain
    pub fn calculate_terrain_movement_cost(
        from_terrain: crate::world::tiles::components::TerrainType, 
        to_terrain: crate::world::tiles::components::TerrainType
    ) -> f32 {
        use crate::world::tiles::components::TerrainType;
        
        let base_cost = match to_terrain {
            TerrainType::Grassland => 1.0,
            TerrainType::Plains => 1.0,
            TerrainType::Hills => 2.0,
            TerrainType::Mountains => 3.0,
            TerrainType::Forest => 2.0,
            TerrainType::Jungle => 3.0,
            TerrainType::Desert => 1.5,
            TerrainType::Tundra => 2.0,
            TerrainType::Coast => 1.0,
            TerrainType::Ocean => f32::INFINITY, // Cannot move into ocean without ship
            TerrainType::River => 1.0,
            TerrainType::Snow => 3.0,
            TerrainType::Mountain => 3.0,
        };
        
        // Apply modifiers based on source terrain
        match from_terrain {
            TerrainType::Mountains | TerrainType::Hills => base_cost * 0.9, // Mountain units move better in rough terrain
            TerrainType::Desert => if matches!(to_terrain, TerrainType::Desert) { base_cost * 0.8 } else { base_cost },
            _ => base_cost,
        }
    }

    /// Check if two terrain types are compatible for easy movement
    pub fn terrain_compatibility(terrain1: crate::world::tiles::components::TerrainType, terrain2: crate::world::tiles::components::TerrainType) -> f32 {
        use crate::world::tiles::components::TerrainType;
        
        match (terrain1, terrain2) {
            // Same terrain types are always compatible
            (a, b) if a == b => 1.0,
            
            // Water compatibility
            (TerrainType::Coast, TerrainType::Ocean) | (TerrainType::Ocean, TerrainType::Coast) => 0.8,
            (TerrainType::River, TerrainType::Coast) | (TerrainType::Coast, TerrainType::River) => 0.9,
            
            // Land compatibility
            (TerrainType::Grassland, TerrainType::Plains) | (TerrainType::Plains, TerrainType::Grassland) => 0.9,
            (TerrainType::Hills, TerrainType::Mountains) | (TerrainType::Mountains, TerrainType::Hills) => 0.8,
            (TerrainType::Forest, TerrainType::Jungle) | (TerrainType::Jungle, TerrainType::Forest) => 0.7,
            
            // Harsh terrain transitions
            (TerrainType::Desert, TerrainType::Tundra) | (TerrainType::Tundra, TerrainType::Desert) => 0.3,
            (TerrainType::Ocean, _) | (_, TerrainType::Ocean) => 0.1, // Ocean requires special handling
            
            // Default compatibility
            _ => 0.6,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_distance() {
        let a = crate::core::zig_ffi::HexCoord { q: 0, r: 0 };
        let b = crate::core::zig_ffi::HexCoord { q: 3, r: -1 };
        let distance = utils::hex_distance(a, b);
        assert_eq!(distance, 3);
    }

    #[test]
    fn test_hex_neighbors() {
        let center = crate::core::zig_ffi::HexCoord { q: 0, r: 0 };
        let neighbors = utils::get_hex_neighbors(center);
        assert_eq!(neighbors.len(), 6);
        
        // Check that east neighbor is correct
        assert_eq!(neighbors[0], crate::core::zig_ffi::HexCoord { q: 1, r: 0 });
    }

    #[test]
    fn test_hex_ring() {
        let center = crate::core::zig_ffi::HexCoord { q: 0, r: 0 };
        let ring = utils::hex_ring(center, 1);
        assert_eq!(ring.len(), 6);
        
        let ring0 = utils::hex_ring(center, 0);
        assert_eq!(ring0, vec![center]);
    }

    #[test]
    fn test_terrain_movement_cost() {
        use crate::world::tiles::components::TerrainType;
        
        let cost = utils::calculate_terrain_movement_cost(
            TerrainType::Grassland,
            TerrainType::Mountains
        );
        assert!(cost > 1.0);
        
        let ocean_cost = utils::calculate_terrain_movement_cost(
            TerrainType::Grassland,
            TerrainType::Ocean
        );
        assert!(ocean_cost.is_infinite());
    }
}
