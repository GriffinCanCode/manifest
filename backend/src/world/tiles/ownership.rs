//! Ownership layers with bitvec flags for memory-efficient tile ownership tracking
//!
//! Provides highly optimized ownership tracking for tiles using bitvec for
//! compact storage and fast bitwise operations on ownership information.

use bitvec::prelude::*;
use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize, Deserializer, Serializer};
use std::collections::HashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use rayon::prelude::*;

use crate::core::{
    zig_ffi::HexCoord,
    hashing::{FastHashMap, FastHashSet},
    caching::{GameCache, GameCacheBuilder, CacheKey, CachePriority}
};
use crate::world::tiles::{
    chunks::{TileId, ChunkCoord, ChunkManager, CHUNK_SIZE},
    components::{Tile, TileComponentManager},
    spatial::TileSpatialIndex
};
use tracing::{debug, instrument, warn};

/// Maximum number of players supported (for bitvec sizing)
pub const MAX_PLAYERS: usize = 64;

/// Player identifier type
pub type PlayerId = u8;

/// Ownership status for a single tile
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OwnershipStatus {
    /// Tile is unowned (neutral)
    Unowned,
    /// Tile is owned by a specific player
    Owned(PlayerId),
    /// Tile is contested by multiple players
    Contested,
    /// Tile ownership is disputed (recent conflict)
    Disputed,
}

impl Default for OwnershipStatus {
    fn default() -> Self {
        Self::Unowned
    }
}

/// Ownership strength levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum OwnershipStrength {
    None = 0,
    Weak = 1,      // Recently claimed, easily lost
    Moderate = 2,  // Established presence
    Strong = 3,    // Well-defended territory
    Absolute = 4,  // Core territory, very hard to take
}

impl Default for OwnershipStrength {
    fn default() -> Self {
        Self::None
    }
}

impl OwnershipStrength {
    /// Convert strength to multiplier for various game mechanics
    pub fn as_multiplier(self) -> f32 {
        match self {
            Self::None => 0.0,
            Self::Weak => 0.25,
            Self::Moderate => 0.5,
            Self::Strong => 0.75,
            Self::Absolute => 1.0,
        }
    }

    /// Check if ownership strength allows certain actions
    pub fn allows_action(self, required_strength: OwnershipStrength) -> bool {
        (self as u8) >= (required_strength as u8)
    }
}

/// Ownership claims for a specific tile with bitvec flags
#[derive(Debug, Clone)]
pub struct TileOwnershipClaims {
    /// Bitfield for which players have claims (up to 64 players)
    claims: BitArr!(for MAX_PLAYERS, in bitvec::order::LocalBits, bitvec::store::usize),
    /// Strength of each player's claim
    claim_strengths: [OwnershipStrength; MAX_PLAYERS],
    /// Primary owner (strongest claim)
    primary_owner: Option<PlayerId>,
    /// Whether ownership is currently disputed
    is_disputed: bool,
    /// Last update timestamp for claim aging
    last_updated: u64,
}

impl Default for TileOwnershipClaims {
    fn default() -> Self {
        Self {
            claims: BitArray::ZERO,
            claim_strengths: [OwnershipStrength::None; MAX_PLAYERS],
            primary_owner: None,
            is_disputed: false,
            last_updated: 0,
        }
    }
}

impl TileOwnershipClaims {
    /// Create new empty claims
    pub fn new() -> Self {
        Self::default()
    }

    /// Add or update claim for a player
    pub fn set_claim(&mut self, player_id: PlayerId, strength: OwnershipStrength) {
        if (player_id as usize) < MAX_PLAYERS {
            let idx = player_id as usize;
            
            // Set claim bit
            self.claims.set(idx, strength != OwnershipStrength::None);
            
            // Update strength
            self.claim_strengths[idx] = strength;
            
            // Update primary owner
            self.update_primary_owner();
            
            // Check for disputes
            self.check_disputed_status();
            
            self.last_updated += 1; // Simple increment for testing
        }
    }

    /// Remove claim for a player
    pub fn remove_claim(&mut self, player_id: PlayerId) {
        if (player_id as usize) < MAX_PLAYERS {
            let idx = player_id as usize;
            
            self.claims.set(idx, false);
            self.claim_strengths[idx] = OwnershipStrength::None;
            
            self.update_primary_owner();
            self.check_disputed_status();
            
            self.last_updated += 1;
        }
    }

    /// Get claim strength for a player
    pub fn get_claim_strength(&self, player_id: PlayerId) -> OwnershipStrength {
        if (player_id as usize) < MAX_PLAYERS {
            self.claim_strengths[player_id as usize]
        } else {
            OwnershipStrength::None
        }
    }

    /// Check if player has any claim
    pub fn has_claim(&self, player_id: PlayerId) -> bool {
        if (player_id as usize) < MAX_PLAYERS {
            self.claims[player_id as usize]
        } else {
            false
        }
    }

    /// Get all players with claims
    pub fn get_claimants(&self) -> Vec<PlayerId> {
        self.claims.iter()
            .enumerate()
            .filter_map(|(idx, has_claim)| {
                if *has_claim && idx < MAX_PLAYERS {
                    Some(idx as PlayerId)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get primary owner (strongest claim)
    pub fn primary_owner(&self) -> Option<PlayerId> {
        self.primary_owner
    }

    /// Check if ownership is disputed
    pub fn is_disputed(&self) -> bool {
        self.is_disputed
    }

    /// Get number of active claims
    pub fn claim_count(&self) -> usize {
        self.claims.count_ones()
    }

    /// Get ownership status
    pub fn status(&self) -> OwnershipStatus {
        match self.claim_count() {
            0 => OwnershipStatus::Unowned,
            1 => {
                if let Some(owner) = self.primary_owner {
                    OwnershipStatus::Owned(owner)
                } else {
                    OwnershipStatus::Unowned
                }
            }
            _ => {
                if self.is_disputed {
                    OwnershipStatus::Disputed
                } else {
                    OwnershipStatus::Contested
                }
            }
        }
    }

    /// Update primary owner based on claim strengths
    fn update_primary_owner(&mut self) {
        let mut strongest_player = None;
        let mut strongest_strength = OwnershipStrength::None;
        
        for (idx, &strength) in self.claim_strengths.iter().enumerate() {
            if strength != OwnershipStrength::None && (strength as u8) > (strongest_strength as u8) {
                strongest_strength = strength;
                strongest_player = Some(idx as PlayerId);
            }
        }
        
        self.primary_owner = strongest_player;
    }

    /// Check if ownership is disputed (multiple strong claims)
    fn check_disputed_status(&mut self) {
        let strong_claimants = self.claim_strengths.iter()
            .filter(|&&strength| (strength as u8) >= (OwnershipStrength::Moderate as u8))
            .count();
        
        self.is_disputed = strong_claimants > 1;
    }

    /// Apply claim decay over time
    pub fn apply_decay(&mut self, decay_factor: f32) -> bool {
        let mut changed = false;
        
        for (idx, strength) in self.claim_strengths.iter_mut().enumerate() {
            if *strength != OwnershipStrength::None {
                // Simple decay mechanism - in practice would be more sophisticated
                if rand::random::<f32>() < decay_factor {
                    let new_strength = match *strength {
                        OwnershipStrength::Absolute => OwnershipStrength::Strong,
                        OwnershipStrength::Strong => OwnershipStrength::Moderate,
                        OwnershipStrength::Moderate => OwnershipStrength::Weak,
                        OwnershipStrength::Weak => OwnershipStrength::None,
                        OwnershipStrength::None => OwnershipStrength::None,
                    };
                    
                    if new_strength != *strength {
                        *strength = new_strength;
                        self.claims.set(idx, new_strength != OwnershipStrength::None);
                        changed = true;
                    }
                }
            }
        }
        
        if changed {
            self.update_primary_owner();
            self.check_disputed_status();
            self.last_updated += 1;
        }
        
        changed
    }

    /// Merge claims from another tile (for influence spread)
    pub fn merge_claims(&mut self, other: &TileOwnershipClaims, influence_factor: f32) {
        for (idx, &other_strength) in other.claim_strengths.iter().enumerate() {
            if other_strength != OwnershipStrength::None {
                let influenced_strength = match other_strength {
                    OwnershipStrength::Absolute => {
                        if influence_factor > 0.8 { OwnershipStrength::Strong }
                        else if influence_factor > 0.5 { OwnershipStrength::Moderate }
                        else { OwnershipStrength::Weak }
                    }
                    OwnershipStrength::Strong => {
                        if influence_factor > 0.6 { OwnershipStrength::Moderate }
                        else { OwnershipStrength::Weak }
                    }
                    OwnershipStrength::Moderate => {
                        if influence_factor > 0.4 { OwnershipStrength::Weak }
                        else { OwnershipStrength::None }
                    }
                    _ => OwnershipStrength::None,
                };
                
                // Only apply if it would strengthen the claim
                if (influenced_strength as u8) > (self.claim_strengths[idx] as u8) {
                    self.set_claim(idx as PlayerId, influenced_strength);
                }
            }
        }
    }
}

/// Serialize TileOwnershipClaims as compact binary data
impl Serialize for TileOwnershipClaims {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        
        let mut state = serializer.serialize_struct("TileOwnershipClaims", 4)?;
        
        // Convert bitvec to bytes for serialization
        let claims_bytes: Vec<u8> = self.claims.as_raw_slice().iter()
            .flat_map(|&word| word.to_le_bytes())
            .collect();
        
        state.serialize_field("claims", &claims_bytes)?;
        state.serialize_field("claim_strengths", &self.claim_strengths)?;
        state.serialize_field("primary_owner", &self.primary_owner)?;
        state.serialize_field("is_disputed", &self.is_disputed)?;
        state.end()
    }
}

/// Deserialize TileOwnershipClaims from compact binary data
impl<'de> Deserialize<'de> for TileOwnershipClaims {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};
        use std::fmt;
        
        #[derive(Deserialize)]
        #[serde(field_identifier)]
        enum Field { Claims, ClaimStrengths, PrimaryOwner, IsDisputed }
        
        struct TileOwnershipClaimsVisitor;
        
        impl<'de> Visitor<'de> for TileOwnershipClaimsVisitor {
            type Value = TileOwnershipClaims;
            
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct TileOwnershipClaims")
            }
            
            fn visit_map<V>(self, mut map: V) -> Result<TileOwnershipClaims, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut claims_bytes: Option<Vec<u8>> = None;
                let mut claim_strengths: Option<[OwnershipStrength; MAX_PLAYERS]> = None;
                let mut primary_owner: Option<Option<PlayerId>> = None;
                let mut is_disputed: Option<bool> = None;
                
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Claims => {
                            claims_bytes = Some(map.next_value()?);
                        }
                        Field::ClaimStrengths => {
                            claim_strengths = Some(map.next_value()?);
                        }
                        Field::PrimaryOwner => {
                            primary_owner = Some(map.next_value()?);
                        }
                        Field::IsDisputed => {
                            is_disputed = Some(map.next_value()?);
                        }
                    }
                }
                
                let claims_bytes = claims_bytes.ok_or_else(|| de::Error::missing_field("claims"))?;
                let claim_strengths = claim_strengths.ok_or_else(|| de::Error::missing_field("claim_strengths"))?;
                let primary_owner = primary_owner.ok_or_else(|| de::Error::missing_field("primary_owner"))?;
                let is_disputed = is_disputed.ok_or_else(|| de::Error::missing_field("is_disputed"))?;
                
                // Reconstruct bitvec from bytes
                let mut claims = BitArray::ZERO;
                let words_needed = (MAX_PLAYERS + std::mem::size_of::<usize>() * 8 - 1) / (std::mem::size_of::<usize>() * 8);
                for (i, chunk) in claims_bytes.chunks(std::mem::size_of::<usize>()).take(words_needed).enumerate() {
                    let mut word_bytes = [0u8; std::mem::size_of::<usize>()];
                    word_bytes[..chunk.len()].copy_from_slice(chunk);
                    let word = usize::from_le_bytes(word_bytes);
                    claims.as_raw_mut_slice()[i] = word;
                }
                
                Ok(TileOwnershipClaims {
                    claims,
                    claim_strengths,
                    primary_owner,
                    is_disputed,
                    last_updated: 0, // Reset on deserialize
                })
            }
        }
        
        const FIELDS: &[&str] = &["claims", "claim_strengths", "primary_owner", "is_disputed"];
        deserializer.deserialize_struct("TileOwnershipClaims", FIELDS, TileOwnershipClaimsVisitor)
    }
}

/// Chunk-based ownership layer for memory efficiency
#[derive(Debug)]
pub struct OwnershipChunk {
    /// Chunk coordinate
    chunk_coord: ChunkCoord,
    /// Ownership claims for each tile in chunk (sparse storage)
    tile_claims: HashMap<(usize, usize), TileOwnershipClaims>,
    /// Quick lookup for which players have claims in this chunk
    players_in_chunk: BitArr!(for MAX_PLAYERS, in bitvec::order::LocalBits, bitvec::store::usize),
    /// Generation counter for change tracking
    generation: u64,
}

impl OwnershipChunk {
    /// Create new ownership chunk
    pub fn new(chunk_coord: ChunkCoord) -> Self {
        Self {
            chunk_coord,
            tile_claims: HashMap::new(),
            players_in_chunk: BitArray::ZERO,
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
            self.players_in_chunk = BitArray::ZERO;
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

/// High-performance ownership layer manager using bitvec for efficient storage
#[derive(Debug)]
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
                        let hex = ChunkManager::chunk_to_hex(**chunk_coord, *local_x, *local_y);
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

/// Ownership statistics for monitoring
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OwnershipStats {
    pub total_chunks: usize,
    pub total_claimed_tiles: usize,
    pub owned_tiles: usize,
    pub contested_tiles: usize,
    pub disputed_tiles: usize,
    pub active_players: u8,
    pub player_territories: HashMap<PlayerId, usize>,
}

/// System for updating ownership based on game events
pub fn update_ownership_system(
    ownership_layer: Res<TileOwnershipLayer>,
    // Would include event queries for ownership changes
) {
    // Process ownership change events
    // Implementation depends on event system
}

/// System for applying periodic ownership decay
pub fn ownership_decay_system(
    ownership_layer: Res<TileOwnershipLayer>,
    // Would include timing resources
) {
    // Apply decay at regular intervals
    // Implementation depends on game time system
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ownership_status() {
        assert_eq!(OwnershipStatus::default(), OwnershipStatus::Unowned);
    }

    #[test]
    fn test_ownership_strength() {
        assert!(OwnershipStrength::Strong.allows_action(OwnershipStrength::Moderate));
        assert!(!OwnershipStrength::Weak.allows_action(OwnershipStrength::Strong));
        
        assert_eq!(OwnershipStrength::Absolute.as_multiplier(), 1.0);
        assert_eq!(OwnershipStrength::None.as_multiplier(), 0.0);
    }

    #[test]
    fn test_tile_ownership_claims() {
        let mut claims = TileOwnershipClaims::new();
        
        // Test setting claims
        claims.set_claim(1, OwnershipStrength::Strong);
        assert!(claims.has_claim(1));
        assert_eq!(claims.get_claim_strength(1), OwnershipStrength::Strong);
        assert_eq!(claims.primary_owner(), Some(1));
        assert_eq!(claims.status(), OwnershipStatus::Owned(1));
        
        // Test multiple claims
        claims.set_claim(2, OwnershipStrength::Moderate);
        assert_eq!(claims.claim_count(), 2);
        assert_eq!(claims.primary_owner(), Some(1)); // Still strongest
        assert_eq!(claims.status(), OwnershipStatus::Contested);
        
        // Test removing claims
        claims.remove_claim(1);
        assert!(!claims.has_claim(1));
        assert_eq!(claims.primary_owner(), Some(2));
        assert_eq!(claims.status(), OwnershipStatus::Owned(2));
    }

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

    #[test]
    fn test_bitvec_operations() {
        let mut bits: BitArr!(for 64) = BitArray::ZERO;
        
        // Test setting bits
        bits.set(5, true);
        bits.set(10, true);
        bits.set(63, true);
        
        assert!(bits[5]);
        assert!(bits[10]);
        assert!(bits[63]);
        assert!(!bits[0]);
        
        assert_eq!(bits.count_ones(), 3);
        
        // Test iteration
        let set_indices: Vec<usize> = bits.iter()
            .enumerate()
            .filter_map(|(idx, bit)| if *bit { Some(idx) } else { None })
            .collect();
        
        assert_eq!(set_indices, vec![5, 10, 63]);
    }
}
