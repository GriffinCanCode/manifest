//! High-performance ownership layer manager
//!
//! Provides the main TileOwnershipLayer resource for managing tile ownership
//! across the game world with caching and parallel processing support.

use std::collections::HashMap;
use std::sync::Arc;
use bevy_ecs::prelude::*;
use parking_lot::RwLock;
use rayon::prelude::*;
use tracing::{debug, instrument};

use crate::core::{
    zig_ffi::HexCoord,
    caching::{GameCache, GameCacheBuilder, CacheKey, CachePriority}
};
use crate::world::tiles::chunks::{ChunkCoord, ChunkManager, CHUNK_SIZE};

use super::{
    claims::TileOwnershipClaims,
    chunk::OwnershipChunk,
    stats::OwnershipStats,
    types::{OwnershipStatus, OwnershipStrength, PlayerId},
};

/// High-performance ownership layer manager using bitvec for efficient storage
#[derive(Debug, Resource)]
pub struct TileOwnershipLayer {
    /// Ownership chunks indexed by chunk coordinate
    chunks: Arc<RwLock<HashMap<ChunkCoord, OwnershipChunk>>>,
    /// Cache for ownership queries
    cache: GameCache,
    /// Chunk manager for coordinate conversion
    chunk_manager: Arc<ChunkManager>,
    /// Global generation counter
    global_generation: Arc<RwLock<u64>>,
}

impl TileOwnershipLayer {
    /// Create new ownership layer
    pub fn new(chunk_manager: Arc<ChunkManager>) -> Self {
        let cache = GameCacheBuilder::new()
            .max_memory_mb(32) // 32MB for ownership cache
            .default_ttl(std::time::Duration::from_secs(60)) // 1 minute TTL
            .turn_based_invalidation(true)
            .build();

        Self {
            chunks: Arc::new(RwLock::new(HashMap::new())),
            cache,
            chunk_manager,
            global_generation: Arc::new(RwLock::new(1)),
        }
    }

    /// Set ownership for a tile
    #[instrument(skip(self))]
    pub async fn set_tile_ownership(&self, hex: HexCoord, player_id: PlayerId, strength: OwnershipStrength) {
        let chunk_coord = ChunkManager::hex_to_chunk(hex);
        let local_x = (hex.q - chunk_coord.x * CHUNK_SIZE as i32) as usize;
        let local_y = (hex.r - chunk_coord.y * CHUNK_SIZE as i32) as usize;

        // Ensure chunk exists
        {
            let mut chunks = self.chunks.write();
            if !chunks.contains_key(&chunk_coord) {
                chunks.insert(chunk_coord, OwnershipChunk::new(chunk_coord));
            }
        }

        // Update ownership
        {
            let mut chunks = self.chunks.write();
            if let Some(chunk) = chunks.get_mut(&chunk_coord) {
                chunk.set_tile_claim(local_x, local_y, player_id, strength);
            }
        }

        // Update global generation
        {
            let mut gen = self.global_generation.write();
            *gen += 1;
        }

        // Invalidate cache
        self.invalidate_tile_cache(hex).await;
        
        debug!("Set ownership for tile {:?} to player {} with strength {:?}", hex, player_id, strength);
    }

    /// Get ownership status for a tile
    #[instrument(skip(self))]
    pub async fn get_tile_ownership(&self, hex: HexCoord) -> OwnershipStatus {
        let cache_key = CacheKey::Custom(format!("ownership:{}:{}", hex.q, hex.r));
        
        // Check cache first
        if let Ok(Some(status)) = self.cache.get::<OwnershipStatus>(&cache_key).await {
            return status;
        }

        // Cache miss - compute ownership
        let status = {
            let chunk_coord = ChunkManager::hex_to_chunk(hex);
            let local_x = (hex.q - chunk_coord.x * CHUNK_SIZE as i32) as usize;
            let local_y = (hex.r - chunk_coord.y * CHUNK_SIZE as i32) as usize;

            let chunks = self.chunks.read();
            if let Some(chunk) = chunks.get(&chunk_coord) {
                if let Some(claims) = chunk.get_tile_claims(local_x, local_y) {
                    claims.status()
                } else {
                    OwnershipStatus::Unowned
                }
            } else {
                OwnershipStatus::Unowned
            }
        };

        // Cache the result
        let _ = self.cache.set(cache_key, status, CachePriority::High).await;
        status
    }

    /// Get detailed ownership claims for a tile
    pub fn get_tile_claims(&self, hex: HexCoord) -> Option<TileOwnershipClaims> {
        let chunk_coord = ChunkManager::hex_to_chunk(hex);
        let local_x = (hex.q - chunk_coord.x * CHUNK_SIZE as i32) as usize;
        let local_y = (hex.r - chunk_coord.y * CHUNK_SIZE as i32) as usize;

        let chunks = self.chunks.read();
        chunks.get(&chunk_coord)?
            .get_tile_claims(local_x, local_y)
            .cloned()
    }

    /// Get all tiles owned by a player in parallel
    pub async fn get_player_territories(&self, player_id: PlayerId) -> Vec<HexCoord> {
        let chunks = self.chunks.read();
        
        // Process chunks in parallel
        let territories: Vec<HexCoord> = chunks.par_iter()
            .filter_map(|(chunk_coord, chunk)| {
                if chunk.player_has_claims(player_id) {
                    Some((chunk_coord, chunk))
                } else {
                    None
                }
            })
            .flat_map(|(chunk_coord, chunk)| {
                let mut tiles = Vec::new();
                for ((local_x, local_y), claims) in &chunk.tile_claims {
                    if claims.has_claim(player_id) {
                        let hex = ChunkManager::chunk_to_hex(*chunk_coord, *local_x, *local_y);
                        tiles.push(hex);
                    }
                }
                tiles
            })
            .collect();

        territories
    }

    /// Apply ownership decay across all chunks
    #[instrument(skip(self))]
    pub async fn apply_global_decay(&self, decay_factor: f32) -> usize {
        let mut chunks_changed = 0;
        
        // Apply decay in parallel
        let chunk_coords: Vec<_> = self.chunks.read().keys().copied().collect();
        
        for chunk_coord in chunk_coords {
            let changed = {
                let mut chunks = self.chunks.write();
                if let Some(chunk) = chunks.get_mut(&chunk_coord) {
                    chunk.apply_decay(decay_factor)
                } else {
                    false
                }
            };
            
            if changed {
                chunks_changed += 1;
            }
        }

        // Update global generation if any changes occurred
        if chunks_changed > 0 {
            let mut gen = self.global_generation.write();
            *gen += 1;
            
            // Clear cache after global changes
            let _ = self.cache.clear().await;
        }

        debug!("Applied decay to {} chunks", chunks_changed);
        chunks_changed
    }

    /// Get ownership statistics for monitoring
    pub async fn ownership_stats(&self) -> OwnershipStats {
        let chunks = self.chunks.read();
        let mut stats = OwnershipStats::default();
        
        stats.total_chunks = chunks.len();
        
        let mut player_tile_counts: HashMap<PlayerId, usize> = HashMap::new();
        
        for chunk in chunks.values() {
            stats.total_claimed_tiles += chunk.tile_claims.len();
            
            for claims in chunk.tile_claims.values() {
                match claims.status() {
                    OwnershipStatus::Owned(player) => {
                        *player_tile_counts.entry(player).or_insert(0) += 1;
                        stats.owned_tiles += 1;
                    }
                    OwnershipStatus::Contested => stats.contested_tiles += 1,
                    OwnershipStatus::Disputed => stats.disputed_tiles += 1,
                    OwnershipStatus::Unowned => {} // Shouldn't happen in claimed tiles
                }
            }
        }
        
        stats.active_players = player_tile_counts.len() as u8;
        stats.player_territories = player_tile_counts;
        
        stats
    }

    /// Get memory usage across all ownership data
    pub fn memory_usage(&self) -> usize {
        let chunks = self.chunks.read();
        chunks.values().map(|chunk| chunk.memory_size()).sum::<usize>() +
        std::mem::size_of::<Self>()
    }

    /// Clear ownership data for a chunk
    pub async fn clear_chunk(&self, chunk_coord: ChunkCoord) {
        {
            let mut chunks = self.chunks.write();
            chunks.remove(&chunk_coord);
        }
        
        // Update global generation
        {
            let mut gen = self.global_generation.write();
            *gen += 1;
        }
        
        debug!("Cleared ownership data for chunk {:?}", chunk_coord);
    }

    /// Invalidate cache for a specific tile
    async fn invalidate_tile_cache(&self, hex: HexCoord) {
        let cache_key = CacheKey::Custom(format!("ownership:{}:{}", hex.q, hex.r));
        let _ = self.cache.remove(&cache_key).await;
    }
}

impl Default for TileOwnershipLayer {
    fn default() -> Self {
        let chunk_manager = Arc::new(ChunkManager::default());
        Self::new(chunk_manager)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ownership_layer() {
        let chunk_manager = Arc::new(ChunkManager::default());
        let layer = TileOwnershipLayer::new(chunk_manager);
        
        let hex = HexCoord { q: 10, r: 20 };
        
        // Test setting ownership
        layer.set_tile_ownership(hex, 1, OwnershipStrength::Strong).await;
        
        let status = layer.get_tile_ownership(hex).await;
        assert_eq!(status, OwnershipStatus::Owned(1));
        
        // Test getting player territories
        let territories = layer.get_player_territories(1).await;
        assert_eq!(territories.len(), 1);
        assert!(territories.contains(&hex));
    }
}
