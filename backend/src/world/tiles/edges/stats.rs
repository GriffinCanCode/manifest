//! Statistics and error handling for edge detection

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::types::EdgeType;
use crate::world::tiles::chunks::ChunkCoord;

/// Statistics for edge detection monitoring
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EdgeDetectionStats {
    pub total_edges: usize,
    pub significant_edges: usize,
    pub chunks_processed: usize,
    pub edges_by_type: HashMap<EdgeType, usize>,
}

/// Edge detection errors
#[derive(Debug, thiserror::Error)]
pub enum EdgeDetectionError {
    #[error("Chunk data not available: {chunk:?}")]
    ChunkDataUnavailable { chunk: ChunkCoord },
    
    #[error("Image processing error: {message}")]
    ImageProcessingError { message: String },
    
    #[error("Invalid tile data")]
    InvalidTileData,
    
    #[error("Cache error: {message}")]
    CacheError { message: String },
}
