//! Individual chunk storage with ndarray for high-performance 2D tile arrays
//!
//! Provides memory-efficient chunk-based storage using optimized ndarray operations
//! with SIMD acceleration where possible.

use ndarray::{Array2, ArrayView2, ArrayViewMut2, s};
use serde::{Deserialize, Serialize};
use super::types::{CHUNK_SIZE, ChunkCoord, TileId};
use super::errors::ChunkError;

/// Chunk storage container using ndarray for optimal SIMD operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileChunk {
    /// Chunk coordinate in world space
    coord: ChunkCoord,
    /// 2D array of tile IDs (64x64 for cache-friendly access)
    tiles: Array2<TileId>,
    /// Dirty flag for change detection
    dirty: bool,
    /// Generation for cache invalidation
    generation: u64,
}

impl TileChunk {
    /// Create new empty chunk at specified coordinate
    pub fn new(coord: ChunkCoord) -> Self {
        Self {
            coord,
            tiles: Array2::zeros((CHUNK_SIZE, CHUNK_SIZE)),
            dirty: false,
            generation: 0,
        }
    }

    /// Get tile ID at local chunk coordinates
    #[inline]
    pub fn get_tile(&self, x: usize, y: usize) -> Option<TileId> {
        if x >= CHUNK_SIZE || y >= CHUNK_SIZE {
            return None;
        }
        Some(self.tiles[[x, y]])
    }

    /// Set tile ID at local chunk coordinates
    #[inline] 
    pub fn set_tile(&mut self, x: usize, y: usize, tile_id: TileId) -> Result<(), ChunkError> {
        if x >= CHUNK_SIZE || y >= CHUNK_SIZE {
            return Err(ChunkError::InvalidCoordinate { x, y });
        }
        
        self.tiles[[x, y]] = tile_id;
        self.dirty = true;
        self.generation += 1;
        Ok(())
    }

    /// Get read-only view of entire tile array
    #[inline]
    pub fn tiles(&self) -> ArrayView2<TileId> {
        self.tiles.view()
    }

    /// Get mutable view of entire tile array
    #[inline]
    pub fn tiles_mut(&mut self) -> ArrayViewMut2<TileId> {
        self.dirty = true;
        self.generation += 1;
        self.tiles.view_mut()
    }

    /// Check if chunk has been modified
    #[inline]
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Mark chunk as clean (after serialization/save)
    #[inline]
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    /// Get current generation for cache invalidation
    #[inline]
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Get chunk coordinate
    #[inline]
    pub fn coord(&self) -> ChunkCoord {
        self.coord
    }

    /// Efficiently fill rectangular region with tile ID
    pub fn fill_rect(&mut self, x: usize, y: usize, width: usize, height: usize, tile_id: TileId) -> Result<(), ChunkError> {
        if x + width > CHUNK_SIZE || y + height > CHUNK_SIZE {
            return Err(ChunkError::InvalidRegion { x, y, width, height });
        }

        let mut slice = self.tiles.slice_mut(s![x..x+width, y..y+height]);
        slice.fill(tile_id);
        
        self.dirty = true;
        self.generation += 1;
        Ok(())
    }

    /// Copy data from another chunk (for streaming/LOD)
    pub fn copy_from(&mut self, other: &TileChunk) {
        self.tiles.assign(&other.tiles);
        self.dirty = true;
        self.generation += 1;
    }

    /// Count occurrences of specific tile ID (SIMD optimized via ndarray)
    pub fn count_tile(&self, tile_id: TileId) -> usize {
        self.tiles.iter().filter(|&&id| id == tile_id).count()
    }

    /// Get memory usage in bytes
    pub fn memory_size(&self) -> usize {
        std::mem::size_of::<Self>() + (CHUNK_SIZE * CHUNK_SIZE * std::mem::size_of::<TileId>())
    }
}
