//! Memory usage statistics for the chunk system
//!
//! Provides metrics and monitoring for chunk memory consumption.

use serde::{Deserialize, Serialize};

/// Memory usage statistics for chunk manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMemoryStats {
    pub chunk_count: usize,
    pub total_memory_bytes: usize,
    pub memory_mb: usize,
    pub max_memory_mb: usize,
}
