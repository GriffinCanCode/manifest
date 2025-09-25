//! Chunk-based ownership management for memory efficiency
//!
//! Provides OwnershipChunk for managing ownership claims within spatial chunks,
//! using sparse storage and bitvec for efficient player tracking.

use std::collections::HashMap;

use super::{
    claims::TileOwnershipClaims,
    types::{OwnershipStrength, PlayerId, MAX_PLAYERS},
};
use crate::world::tiles::chunks::{ChunkCoord, CHUNK_SIZE};

/// Chunk-based ownership layer for memory efficiency
#[derive(Debug)]
pub struct OwnershipChunk {
    /// Chunk coordinate
    chunk_coord: ChunkCoord,
    /// Ownership claims for each tile in chunk (sparse storage)
    pub tile_claims: HashMap<(usize, usize), TileOwnershipClaims>,
    /// Quick lookup for which players have claims in this chunk
    players_in_chunk: bitvec::vec::BitVec,
    /// Generation counter for change tracking
    generation: u64,
}

impl OwnershipChunk {
    /// Create new ownership chunk
    pub fn new(chunk_coord: ChunkCoord) -> Self {
        Self {
            chunk_coord,
            tile_claims: HashMap::new(),
            players_in_chunk: bitvec::vec::BitVec::new(),
            generation: 1,
        }
    }

    /// Set ownership claim for tile within chunk
    pub fn set_tile_claim(&mut self, local_x: usize, local_y: usize, player_id: PlayerId, strength: OwnershipStrength) {
        if local_x < CHUNK_SIZE && local_y < CHUNK_SIZE && (player_id as usize) < MAX_PLAYERS {
            let claims = self.tile_claims.entry((local_x, local_y)).or_insert_with(TileOwnershipClaims::new);
            claims.set_claim(player_id, strength);
            
            // Update chunk-level player tracking
            if strength != OwnershipStrength::None {
                self.players_in_chunk.set(player_id as usize, true);
            } else {
                // Check if player has any other claims in this chunk
                let has_other_claims = self.tile_claims.values()
                    .any(|claim| claim.has_claim(player_id));
                if !has_other_claims {
                    self.players_in_chunk.set(player_id as usize, false);
                }
            }
            
            self.generation += 1;
        }
    }

    /// Get ownership claims for tile within chunk
    pub fn get_tile_claims(&self, local_x: usize, local_y: usize) -> Option<&TileOwnershipClaims> {
        if local_x < CHUNK_SIZE && local_y < CHUNK_SIZE {
            self.tile_claims.get(&(local_x, local_y))
        } else {
            None
        }
    }

    /// Check if player has any claims in this chunk
    pub fn player_has_claims(&self, player_id: PlayerId) -> bool {
        if (player_id as usize) < MAX_PLAYERS {
            self.players_in_chunk[player_id as usize]
        } else {
            false
        }
    }

    /// Get all players with claims in this chunk
    pub fn get_players_in_chunk(&self) -> Vec<PlayerId> {
        self.players_in_chunk.iter()
            .enumerate()
            .filter_map(|(idx, has_claims)| {
                if *has_claims && idx < MAX_PLAYERS {
                    Some(idx as PlayerId)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Apply decay to all claims in chunk
    pub fn apply_decay(&mut self, decay_factor: f32) -> bool {
        let mut chunk_changed = false;
        
        // Use retain to remove empty claims while processing
        self.tile_claims.retain(|_pos, claims| {
            let changed = claims.apply_decay(decay_factor);
            if changed {
                chunk_changed = true;
            }
            
            // Keep claim if it has any active claims
            claims.claim_count() > 0
        });
        
        if chunk_changed {
            // Rebuild player presence tracking
            self.players_in_chunk = bitvec::vec::BitVec::new();
            for claims in self.tile_claims.values() {
                for player in claims.get_claimants() {
                    self.players_in_chunk.set(player as usize, true);
                }
            }
            
            self.generation += 1;
        }
        
        chunk_changed
    }

    /// Get memory usage in bytes
    pub fn memory_size(&self) -> usize {
        std::mem::size_of::<Self>() +
        self.tile_claims.len() * (std::mem::size_of::<(usize, usize)>() + std::mem::size_of::<TileOwnershipClaims>())
    }

    /// Get generation counter
    pub fn generation(&self) -> u64 {
        self.generation
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::types::OwnershipStrength;

    #[test]
    fn test_ownership_chunk() {
        let mut chunk = OwnershipChunk::new(ChunkCoord::new(0, 0));
        
        // Test setting tile claims
        chunk.set_tile_claim(10, 20, 1, OwnershipStrength::Strong);
        assert!(chunk.player_has_claims(1));
        
        let claims = chunk.get_tile_claims(10, 20).unwrap();
        assert!(claims.has_claim(1));
        assert_eq!(claims.get_claim_strength(1), OwnershipStrength::Strong);
        
        // Test player tracking
        let players = chunk.get_players_in_chunk();
        assert_eq!(players.len(), 1);
        assert!(players.contains(&1));
    }
}
