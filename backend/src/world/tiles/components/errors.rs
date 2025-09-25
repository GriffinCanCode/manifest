//! Error types and statistics for tile components
//!
//! Contains error definitions and statistics structures for the tile component system.

use serde::{Deserialize, Serialize};

/// Tile system errors
#[derive(Debug, thiserror::Error)]
pub enum TileError {
    #[error("Tile not found")]
    TileNotFound,
    
    #[error("Component not found")]
    ComponentNotFound,
    
    #[error("Failed to add component")]
    ComponentAddFailed,
    
    #[error("Invalid tile coordinate")]
    InvalidCoordinate,
}

/// Memory usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileComponentStats {
    pub tile_count: usize,
    pub archetype_count: usize,
    pub estimated_memory_bytes: usize,
}
