//! Ownership claims for individual tiles with efficient bitvec storage
//!
//! Provides TileOwnershipClaims struct for tracking ownership claims per tile
//! using bitvec for memory-efficient storage and fast bitwise operations.

use serde::{Deserialize, Serialize, Deserializer, Serializer};

use super::types::{OwnershipStatus, OwnershipStrength, PlayerId, MAX_PLAYERS};

/// Ownership claims for a specific tile with bitvec flags
#[derive(Debug, Clone)]
pub struct TileOwnershipClaims {
    /// Bitfield for which players have claims (up to 64 players)
    claims: bitvec::vec::BitVec,
    /// Strength of each player's claim
    claim_strengths: Vec<OwnershipStrength>,
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
            claims: bitvec::vec::BitVec::new(),
            claim_strengths: vec![OwnershipStrength::None; MAX_PLAYERS],
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
        state.serialize_field("claim_strengths", &self.claim_strengths.as_slice())?;
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
                let mut claim_strengths: Option<Vec<OwnershipStrength>> = None;
                let mut primary_owner: Option<Option<PlayerId>> = None;
                let mut is_disputed: Option<bool> = None;
                
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Claims => {
                            claims_bytes = Some(map.next_value()?);
                        }
                        Field::ClaimStrengths => {
                            let vec: Vec<OwnershipStrength> = map.next_value()?;
                            if vec.len() == MAX_PLAYERS {
                                let mut array = [OwnershipStrength::None; MAX_PLAYERS];
                                array.copy_from_slice(&vec);
                                claim_strengths = Some(array.to_vec());
                            }
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
                let mut claims = bitvec::vec::BitVec::new();
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
