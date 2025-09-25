//! Statistics and error types for adjacency system
//!
//! Contains error handling and statistics for the adjacency graph.

use serde::{Deserialize, Serialize};
use crate::world::tiles::chunks::TileId;
use super::types::HexDirection;

/// Statistics for adjacency graph performance monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdjacencyStats {
    pub total_tiles: usize,
    pub total_adjacencies: usize,
    pub passable_adjacencies: usize,
    pub avg_neighbors_per_tile: f32,
    pub reverse_lookup_size: usize,
}

/// Errors that can occur in adjacency operations
#[derive(Debug, thiserror::Error)]
pub enum AdjacencyError {
    #[error("Inconsistent reverse lookup: tile {from} -> {to}")]
    InconsistentReverseLookup { from: TileId, to: TileId },
    
    #[error("Missing bidirectional adjacency: tile {from} -> {to} in direction {direction:?}")]
    MissingBidirectionalAdjacency { from: TileId, to: TileId, direction: HexDirection },
    
    #[error("Invalid tile ID: {tile_id}")]
    InvalidTileId { tile_id: TileId },
    
    #[error("Pathfinding failed: no path from {from} to {to}")]
    PathfindingFailed { from: TileId, to: TileId },
}

/// Result type for adjacency operations
pub type AdjacencyResult<T> = Result<T, AdjacencyError>;
