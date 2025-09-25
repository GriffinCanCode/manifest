//! High-performance tile storage architecture
//!
//! This module implements a sophisticated multi-layer tile system with:
//! - Chunk-based storage using ndarray for memory efficiency
//! - Sparse tile components with hecs integration 
//! - Spatial indexing via rstar R-tree
//! - Hierarchical organization with petgraph DAG
//! - Adjacency graphs with indexmap
//! - Edge detection algorithms using image crate
//! - Ownership layers with bitvec flags
//! - Improvement slots via slotmap
//! - Bitfield modifiers for tile properties
//! - Multi-layer support with arrayvec

pub mod chunks;
pub mod components;
pub mod spatial;
pub mod hierarchy;
pub mod adjacency;  
pub mod edges;
pub mod ownership;
pub mod improvements;
pub mod modifiers;
pub mod layers;

// Re-export core types
pub use chunks::*;
pub use components::*;
pub use spatial::*;
pub use hierarchy::*;
pub use adjacency::*;
pub use edges::*;
pub use ownership::*;
pub use improvements::*;
pub use modifiers::*;
pub use layers::*;

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::sync::Arc;

    /// Integration test for the complete tile storage system
    #[tokio::test]
    async fn test_complete_tile_system() {
        // Initialize core managers
        let chunk_manager = Arc::new(ChunkManager::new(64)); // 64MB budget
        let tile_manager = Arc::new(TileComponentManager::new());
        let spatial_index = Arc::new(TileSpatialIndex::new(1.0));
        
        // Test basic tile creation and storage
        let hex = HexCoord { q: 10, r: 20 };
        let chunk_coord = ChunkManager::hex_to_chunk(hex);
        
        // Set up tile in chunk
        let tile_id = 123;
        chunk_manager.set_tile(hex, tile_id).unwrap();
        assert_eq!(chunk_manager.get_tile(hex), Some(tile_id));
        
        // Create tile component
        let tile_id = tile_manager.create_tile(hex, chunk_coord, 10, 20, TerrainType::Grassland);
        assert_ne!(tile_id, 0);
        
        // Add to spatial index
        spatial_index.add_tile(tile_id, hex, TerrainType::Grassland, chunk_coord);
        
        // Test spatial queries
        let nearby_tiles = spatial_index.tiles_in_radius(hex, 5.0).await;
        assert!(nearby_tiles.contains(&tile_id));
        
        // Test adjacency system
        let adjacency_graph = TileAdjacencyGraph::new(spatial_index.clone());
        let tiles_for_adjacency = vec![(tile_id, hex)];
        adjacency_graph.build_from_tiles(&tiles_for_adjacency).await.unwrap();
        
        // Test ownership system
        let ownership_layer = TileOwnershipLayer::new(chunk_manager.clone());
        ownership_layer.set_tile_ownership(hex, 1, OwnershipStrength::Strong).await;
        let status = ownership_layer.get_tile_ownership(hex).await;
        assert_eq!(status, OwnershipStatus::Owned(1));
        
        println!("✓ Complete tile storage system integration test passed");
    }
}
