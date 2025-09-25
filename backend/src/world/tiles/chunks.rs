//! Chunk storage system with ndarray for high-performance 2D tile arrays
//!
//! Provides memory-efficient chunk-based storage for large worlds using
//! optimized ndarray operations with SIMD acceleration where possible.

use ndarray::{Array2, ArrayView2, ArrayViewMut2, Axis, s};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use glam::IVec2;
use parking_lot::RwLock;
use crate::core::zig_ffi::HexCoord;
use tracing::{debug, warn, instrument};

/// Chunk size for tile storage (power of 2 for optimal memory alignment)
pub const CHUNK_SIZE: usize = 64;

/// World coordinate type for chunk addressing
pub type ChunkCoord = IVec2;

/// Tile identifier within a chunk
pub type TileId = u32;

/// Invalid tile constant
pub const INVALID_TILE: TileId = 0;

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

/// Memory usage statistics for chunk manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMemoryStats {
    pub chunk_count: usize,
    pub total_memory_bytes: usize,
    pub memory_mb: usize,
    pub max_memory_mb: usize,
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_creation() {
        let coord = ChunkCoord::new(0, 0);
        let chunk = TileChunk::new(coord);
        
        assert_eq!(chunk.coord(), coord);
        assert!(!chunk.is_dirty());
        assert_eq!(chunk.get_tile(0, 0), Some(INVALID_TILE));
    }

    #[test]
    fn test_chunk_tile_operations() {
        let mut chunk = TileChunk::new(ChunkCoord::new(0, 0));
        
        // Set tile
        assert!(chunk.set_tile(10, 20, 42).is_ok());
        assert_eq!(chunk.get_tile(10, 20), Some(42));
        assert!(chunk.is_dirty());
        
        // Test bounds
        assert!(chunk.set_tile(CHUNK_SIZE, 0, 1).is_err());
        assert!(chunk.set_tile(0, CHUNK_SIZE, 1).is_err());
    }

    #[test]
    fn test_chunk_manager() {
        let manager = ChunkManager::new(64); // 64MB budget
        let hex = HexCoord { q: 100, r: 200 };
        
        // Set and get tile
        assert!(manager.set_tile(hex, 123).is_ok());
        assert_eq!(manager.get_tile(hex), Some(123));
        
        // Verify chunk was created
        let chunk_coord = ChunkManager::hex_to_chunk(hex);
        assert!(manager.loaded_chunks().contains(&chunk_coord));
    }

    #[test] 
    fn test_coordinate_conversion() {
        let hex = HexCoord { q: 150, r: 250 };
        let chunk_coord = ChunkManager::hex_to_chunk(hex);
        
        let local_x = (hex.q - chunk_coord.x * CHUNK_SIZE as i32) as usize;
        let local_y = (hex.r - chunk_coord.y * CHUNK_SIZE as i32) as usize;
        
        let converted_hex = ChunkManager::chunk_to_hex(chunk_coord, local_x, local_y);
        assert_eq!(hex.q, converted_hex.q);
        assert_eq!(hex.r, converted_hex.r);
    }
}
