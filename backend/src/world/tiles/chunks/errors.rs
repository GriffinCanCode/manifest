//! Error types for the chunk system
//!
//! Defines all error conditions that can occur during chunk operations.

use super::types::ChunkCoord;

/// Chunk system errors
#[derive(Debug, thiserror::Error)]
pub enum ChunkError {
    #[error("Invalid coordinate ({x}, {y}) - must be within chunk bounds")]
    InvalidCoordinate { x: usize, y: usize },
    
    #[error("Invalid region ({x}, {y}) + ({width}, {height}) - exceeds chunk bounds")]
    InvalidRegion { x: usize, y: usize, width: usize, height: usize },
    
    #[error("Chunk not found at coordinate {coord:?}")]
    ChunkNotFound { coord: ChunkCoord },
    
    #[error("Memory budget exceeded - cannot allocate more chunks")]
    MemoryBudgetExceeded,
}
