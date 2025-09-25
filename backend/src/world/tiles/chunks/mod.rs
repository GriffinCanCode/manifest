//! Chunk storage system with ndarray for high-performance 2D tile arrays
//!
//! Provides memory-efficient chunk-based storage for large worlds using
//! optimized ndarray operations with SIMD acceleration where possible.

pub mod types;
pub mod errors;
pub mod stats;
pub mod chunk;
pub mod manager;

// Re-export commonly used types and constants
pub use types::{CHUNK_SIZE, ChunkCoord, TileId, INVALID_TILE};
pub use errors::ChunkError;
pub use stats::ChunkMemoryStats;
pub use chunk::TileChunk;
pub use manager::ChunkManager;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::zig_ffi::HexCoord;

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
        assert!(chunk.set_tile(10, 20, TileId::new(42)).is_ok());
        assert_eq!(chunk.get_tile(10, 20), Some(TileId::new(42)));
        assert!(chunk.is_dirty());
        
        // Test bounds
        assert!(chunk.set_tile(CHUNK_SIZE, 0, TileId::new(1)).is_err());
        assert!(chunk.set_tile(0, CHUNK_SIZE, TileId::new(1)).is_err());
    }

    #[test]
    fn test_chunk_manager() {
        let manager = ChunkManager::new(64); // 64MB budget
        let hex = HexCoord { q: 100, r: 200 };
        
        // Set and get tile
        assert!(manager.set_tile(hex, TileId::new(123)).is_ok());
        assert_eq!(manager.get_tile(hex), Some(TileId::new(123)));
        
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
