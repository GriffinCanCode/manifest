//! Tile components with hecs sparse storage integration
//!
//! This module provides efficient sparse component storage for tiles using hecs ECS,
//! integrated with the main bevy_ecs world for optimal performance.
//!
//! The large components.rs file has been refactored into focused submodules:
//! - `core`: Core Tile struct and TerrainType enum
//! - `resources`: Resource-related components (TileResource, ResourceType)
//! - `environmental`: Climate, Fertility, and environmental components
//! - `movement`: MovementCost and pathfinding components
//! - `visibility`: Visibility and PlayerVisibilityFlags with bitfield storage
//! - `river`: River and water flow components with bitfield directions
//! - `manager`: High-performance TileComponentManager for ECS integration
//! - `errors`: Error types and statistics structures

pub mod core;
pub mod resources;
pub mod environmental;
pub mod movement;
pub mod visibility;
pub mod river;
pub mod manager;
pub mod errors;

// Re-export core tile types
pub use core::{Tile, TerrainType};

// Re-export resource types
pub use resources::{TileResource, ResourceType};

// Re-export environmental components
pub use environmental::{Climate, Fertility};

// Re-export movement components
pub use movement::MovementCost;

// Re-export visibility components
pub use visibility::{Visibility, PlayerVisibilityFlags};

// Re-export river components
pub use river::{River, RiverFlowDirections};

// Re-export manager and error types
pub use manager::TileComponentManager;
pub use errors::{TileError, TileComponentStats};

// Re-export for compatibility with existing code
pub use core::Tile as TileComponent;
pub use manager::TileComponentManager as ComponentManager;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::zig_ffi::HexCoord;
    use crate::world::tiles::chunks::ChunkCoord;

    #[test]
    fn test_tile_creation() {
        let manager = TileComponentManager::new();
        let hex = HexCoord { q: 10, r: 20 };
        let chunk = ChunkCoord::new(1, 2);
        
        let tile_id = manager.create_tile(hex, chunk, 10, 20, TerrainType::Grassland);
        assert_ne!(tile_id, 0);
        
        let tile = manager.get_component::<Tile>(tile_id).expect("Tile component should exist after creation");
        assert_eq!(tile.hex, hex);
        assert_eq!(tile.terrain_type, TerrainType::Grassland);
    }

    #[test]
    fn test_component_operations() {
        let manager = TileComponentManager::new();
        let hex = HexCoord { q: 0, r: 0 };
        let chunk = ChunkCoord::new(0, 0);
        
        let tile_id = manager.create_tile(hex, chunk, 0, 0, TerrainType::Forest);
        
        // Add resource component
        let resource = TileResource {
            resource_type: ResourceType::Iron,
            quantity: 100,
            discovered: false,
            depletion_rate: 0.1,
        };
        
        assert!(manager.add_component(tile_id, resource.clone()).is_ok());
        
        // Get resource component
        let retrieved = manager.get_component::<TileResource>(tile_id).expect("TileResource component should exist after being added");
        assert_eq!(retrieved.resource_type, ResourceType::Iron);
        assert_eq!(retrieved.quantity, 100);
    }

    #[test]
    fn test_tile_queries() {
        let manager = TileComponentManager::new();
        
        // Create tiles with different terrain types
        let hex1 = HexCoord { q: 0, r: 0 };
        let hex2 = HexCoord { q: 1, r: 0 };
        let chunk = ChunkCoord::new(0, 0);
        
        let tile1 = manager.create_tile(hex1, chunk, 0, 0, TerrainType::Forest);
        let tile2 = manager.create_tile(hex2, chunk, 1, 0, TerrainType::Mountain);
        
        // Query all tiles
        let all_tiles = manager.query_tiles::<&Tile>();
        assert_eq!(all_tiles.len(), 2);
    }
}
