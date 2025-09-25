//! High-performance chunk manager with spatial partitioning
//!
//! Manages chunks in memory with LRU eviction and coordinate conversion utilities.

use std::collections::HashMap;
use parking_lot::RwLock;
use tracing::{debug, instrument};
use crate::core::zig_ffi::HexCoord;
use super::{
    chunk::TileChunk,
    types::{CHUNK_SIZE, ChunkCoord, TileId},
    errors::ChunkError,
    stats::ChunkMemoryStats,
};

/// High-performance chunk manager with spatial partitioning
#[derive(Debug)]
pub struct ChunkManager {
    /// Active chunks stored in memory
    chunks: RwLock<HashMap<ChunkCoord, TileChunk>>,
    /// World bounds for validation
    world_bounds: RwLock<Option<(ChunkCoord, ChunkCoord)>>,
    /// Memory budget for chunk caching
    max_memory_mb: usize,
    /// Current generation for cache invalidation
    global_generation: RwLock<u64>,
}

impl ChunkManager {
    /// Create new chunk manager with memory budget
    pub fn new(max_memory_mb: usize) -> Self {
        Self {
            chunks: RwLock::new(HashMap::new()),
            world_bounds: RwLock::new(None),
            max_memory_mb,
            global_generation: RwLock::new(1),
        }
    }

    /// Convert world hex coordinate to chunk coordinate
    #[inline]
    pub fn hex_to_chunk(hex: HexCoord) -> ChunkCoord {
        ChunkCoord {
            x: hex.q / CHUNK_SIZE as i32,
            y: hex.r / CHUNK_SIZE as i32,
        }
    }

    /// Convert chunk coordinate and local offset to hex coordinate
    #[inline]
    pub fn chunk_to_hex(chunk: ChunkCoord, local_x: usize, local_y: usize) -> HexCoord {
        HexCoord {
            q: chunk.x * CHUNK_SIZE as i32 + local_x as i32,
            r: chunk.y * CHUNK_SIZE as i32 + local_y as i32,
        }
    }

    /// Get or create chunk at coordinate
    #[instrument(skip(self))]
    pub fn get_or_create_chunk(&self, coord: ChunkCoord) -> Result<(), ChunkError> {
        let mut chunks = self.chunks.write();
        
        if !chunks.contains_key(&coord) {
            // Check memory budget before creating
            if chunks.len() >= self.max_chunks() {
                self.evict_old_chunks(&mut chunks)?;
            }
            
            chunks.insert(coord, TileChunk::new(coord));
            debug!("Created new chunk at {:?}", coord);
        }
        
        Ok(())
    }

    /// Get tile ID at world hex coordinate
    pub fn get_tile(&self, hex: HexCoord) -> Option<TileId> {
        let chunk_coord = Self::hex_to_chunk(hex);
        let local_x = (hex.q - chunk_coord.x * CHUNK_SIZE as i32) as usize;
        let local_y = (hex.r - chunk_coord.y * CHUNK_SIZE as i32) as usize;

        let chunks = self.chunks.read();
        chunks.get(&chunk_coord)?.get_tile(local_x, local_y)
    }

    /// Set tile ID at world hex coordinate
    #[instrument(skip(self))]
    pub fn set_tile(&self, hex: HexCoord, tile_id: TileId) -> Result<(), ChunkError> {
        let chunk_coord = Self::hex_to_chunk(hex);
        let local_x = (hex.q - chunk_coord.x * CHUNK_SIZE as i32) as usize;
        let local_y = (hex.r - chunk_coord.y * CHUNK_SIZE as i32) as usize;

        // Ensure chunk exists
        self.get_or_create_chunk(chunk_coord)?;

        let mut chunks = self.chunks.write();
        if let Some(chunk) = chunks.get_mut(&chunk_coord) {
            chunk.set_tile(local_x, local_y, tile_id)?;
            
            // Update global generation
            let mut gen = self.global_generation.write();
            *gen += 1;
            
            Ok(())
        } else {
            Err(ChunkError::ChunkNotFound { coord: chunk_coord })
        }
    }

    /// Get all loaded chunk coordinates
    pub fn loaded_chunks(&self) -> Vec<ChunkCoord> {
        self.chunks.read().keys().copied().collect()
    }

    /// Get memory usage statistics
    pub fn memory_stats(&self) -> ChunkMemoryStats {
        let chunks = self.chunks.read();
        let chunk_count = chunks.len();
        let total_memory = chunks.values().map(|c| c.memory_size()).sum::<usize>();
        
        ChunkMemoryStats {
            chunk_count,
            total_memory_bytes: total_memory,
            memory_mb: total_memory / (1024 * 1024),
            max_memory_mb: self.max_memory_mb,
        }
    }

    /// Calculate maximum chunks based on memory budget
    fn max_chunks(&self) -> usize {
        let chunk_size = CHUNK_SIZE * CHUNK_SIZE * std::mem::size_of::<TileId>() + std::mem::size_of::<TileChunk>();
        (self.max_memory_mb * 1024 * 1024) / chunk_size
    }

    /// Evict oldest unused chunks to free memory
    fn evict_old_chunks(&self, chunks: &mut HashMap<ChunkCoord, TileChunk>) -> Result<(), ChunkError> {
        if chunks.is_empty() {
            return Ok(());
        }

        // Simple LRU: remove chunks with lowest generation
        let mut chunk_ages: Vec<_> = chunks.iter().map(|(coord, chunk)| (*coord, chunk.generation())).collect();
        chunk_ages.sort_by_key(|(_, gen)| *gen);

        // Remove oldest 25% of chunks
        let remove_count = (chunks.len() / 4).max(1);
        for (coord, _) in chunk_ages.iter().take(remove_count) {
            chunks.remove(coord);
            debug!("Evicted chunk at {:?}", coord);
        }

        Ok(())
    }
}

impl Default for ChunkManager {
    fn default() -> Self {
        Self::new(512) // 512MB default memory budget
    }
}
