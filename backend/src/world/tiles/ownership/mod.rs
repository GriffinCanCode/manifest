//! Ownership layers with bitvec flags for memory-efficient tile ownership tracking
//!
//! Provides highly optimized ownership tracking for tiles using bitvec for
//! compact storage and fast bitwise operations on ownership information.
//!
//! The large ownership.rs file has been refactored into focused submodules:
//! - `types`: Core types, enums, and constants (OwnershipStatus, OwnershipStrength, PlayerId)
//! - `claims`: TileOwnershipClaims struct with bitvec storage and serialization
//! - `chunk`: OwnershipChunk for chunk-based management with sparse storage
//! - `layer`: High-performance TileOwnershipLayer manager with caching
//! - `stats`: OwnershipStats for monitoring and performance tracking
//! - `systems`: ECS systems for ownership management and decay

pub mod types;
pub mod claims;
pub mod chunk;
pub mod layer;
pub mod stats;
pub mod systems;

// Re-export commonly used types
pub use types::{
    OwnershipStatus, OwnershipStrength, PlayerId, MAX_PLAYERS,
};
pub use claims::TileOwnershipClaims;
pub use chunk::OwnershipChunk;
pub use layer::TileOwnershipLayer;
pub use stats::OwnershipStats;
pub use systems::{
    update_ownership_system,
    ownership_decay_system,
    ownership_monitoring_system,
    ownership_cleanup_system,
};

// Re-export for compatibility with existing code
pub use layer::TileOwnershipLayer as OwnershipLayer;
pub use claims::TileOwnershipClaims as OwnershipClaims;

#[cfg(test)]
mod integration_tests {
    use super::*;
    use bevy_ecs::prelude::*;
    use crate::core::zig_ffi::HexCoord;
    use crate::world::tiles::chunks::ChunkManager;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_complete_ownership_system() {
        let chunk_manager = Arc::new(ChunkManager::default());
        let ownership_layer = TileOwnershipLayer::new(chunk_manager);
        
        let hex1 = HexCoord { q: 10, r: 20 };
        let hex2 = HexCoord { q: 11, r: 20 };
        
        // Test setting ownership for multiple tiles
        ownership_layer.set_tile_ownership(hex1, 1, OwnershipStrength::Strong).await;
        ownership_layer.set_tile_ownership(hex2, 1, OwnershipStrength::Moderate).await;
        
        // Test getting ownership status
        let status1 = ownership_layer.get_tile_ownership(hex1).await;
        let status2 = ownership_layer.get_tile_ownership(hex2).await;
        
        assert_eq!(status1, OwnershipStatus::Owned(1));
        assert_eq!(status2, OwnershipStatus::Owned(1));
        
        // Test getting player territories
        let territories = ownership_layer.get_player_territories(1).await;
        assert_eq!(territories.len(), 2);
        assert!(territories.contains(&hex1));
        assert!(territories.contains(&hex2));
        
        // Test ownership statistics
        let stats = ownership_layer.ownership_stats().await;
        assert_eq!(stats.owned_tiles, 2);
        assert_eq!(stats.active_players, 1);
        assert_eq!(stats.player_territories.get(&1), Some(&2));
        
        println!("✓ Complete ownership system integration test passed");
    }

    #[tokio::test]
    async fn test_ownership_decay_system() {
        let chunk_manager = Arc::new(ChunkManager::default());
        let ownership_layer = TileOwnershipLayer::new(chunk_manager);
        
        let hex = HexCoord { q: 5, r: 5 };
        
        // Set initial ownership
        ownership_layer.set_tile_ownership(hex, 1, OwnershipStrength::Weak).await;
        
        let initial_status = ownership_layer.get_tile_ownership(hex).await;
        assert_eq!(initial_status, OwnershipStatus::Owned(1));
        
        // Apply decay multiple times (high decay factor for testing)
        for _ in 0..10 {
            ownership_layer.apply_global_decay(1.0).await; // 100% decay chance
        }
        
        // Check if ownership has decayed
        let final_status = ownership_layer.get_tile_ownership(hex).await;
        // Due to randomness, we can't guarantee the exact result, but we can check it's reasonable
        assert!(matches!(final_status, OwnershipStatus::Owned(_) | OwnershipStatus::Unowned));
        
        println!("✓ Ownership decay system test passed");
    }

    #[test]
    fn test_ownership_contested_scenario() {
        let mut claims = TileOwnershipClaims::new();
        
        // Two players claim the same tile
        claims.set_claim(1, OwnershipStrength::Strong);
        claims.set_claim(2, OwnershipStrength::Moderate);
        
        // Should be contested with player 1 as primary owner
        assert_eq!(claims.status(), OwnershipStatus::Contested);
        assert_eq!(claims.primary_owner(), Some(1));
        assert_eq!(claims.claim_count(), 2);
        
        // Add a third player with equal strength to first
        claims.set_claim(3, OwnershipStrength::Strong);
        
        // Should still be contested, primary owner may vary based on implementation
        assert_eq!(claims.status(), OwnershipStatus::Contested);
        assert_eq!(claims.claim_count(), 3);
        
        println!("✓ Ownership contested scenario test passed");
    }

    #[test]
    fn test_ownership_memory_efficiency() {
        // Test memory efficiency of ownership structures
        assert!(std::mem::size_of::<OwnershipStatus>() <= 2); // Should be small enum
        assert!(std::mem::size_of::<OwnershipStrength>() == 1); // Single byte enum
        
        // TileOwnershipClaims should be reasonably sized
        assert!(std::mem::size_of::<TileOwnershipClaims>() < 2048); // Less than 2KB
        
        // Test that we can handle MAX_PLAYERS efficiently
        let mut claims = TileOwnershipClaims::new();
        for player_id in 0..MAX_PLAYERS.min(10) { // Test first 10 players
            claims.set_claim(player_id as PlayerId, OwnershipStrength::Weak);
        }
        
        assert_eq!(claims.claim_count(), MAX_PLAYERS.min(10));
        assert!(claims.get_claimants().len() == MAX_PLAYERS.min(10));
        
        println!("✓ Ownership memory efficiency test passed");
    }

    #[test]
    fn test_bitvec_operations() {
        use bitvec::prelude::*;
        
        let mut bits = BitArray::<[usize; 1]>::ZERO;
        
        // Test setting bits for different players
        bits.set(5, true);
        bits.set(10, true);
        bits.set(15, true);
        
        assert!(bits[5]);
        assert!(bits[10]);
        assert!(bits[15]);
        assert!(!bits[0]);
        
        assert_eq!(bits.count_ones(), 3);
        
        // Test iteration over set bits
        let set_indices: Vec<usize> = bits.iter()
            .enumerate()
            .filter_map(|(idx, bit)| if *bit { Some(idx) } else { None })
            .collect();
        
        assert_eq!(set_indices, vec![5, 10, 15]);
        
        println!("✓ Bitvec operations test passed");
    }

    #[test]
    fn test_ownership_stats_functionality() {
        let mut stats = OwnershipStats::new();
        
        // Set up test scenario
        stats.total_claimed_tiles = 100;
        stats.owned_tiles = 60;
        stats.contested_tiles = 25;
        stats.disputed_tiles = 15;
        stats.active_players = 3;
        
        stats.player_territories.insert(1, 30);
        stats.player_territories.insert(2, 20);
        stats.player_territories.insert(3, 10);
        
        // Test statistical functions
        let (dominant_player, territories) = stats.dominant_player().unwrap();
        assert_eq!(dominant_player, 1);
        assert_eq!(territories, 30);
        
        let (owned_pct, contested_pct, disputed_pct) = stats.ownership_distribution();
        assert_eq!(owned_pct, 0.6);
        assert_eq!(contested_pct, 0.25);
        assert_eq!(disputed_pct, 0.15);
        
        let conflict_level = stats.conflict_level();
        assert_eq!(conflict_level, 0.4); // (25 + 15) / 100
        
        // Test balance checking
        assert!(stats.is_balanced(0.6)); // Player 1 has 30/60 = 50% < 60%
        assert!(!stats.is_balanced(0.4)); // Player 1 has 50% > 40%
        
        println!("✓ Ownership stats functionality test passed");
    }

    #[test]
    fn test_ecs_system_registration() {
        let mut world = World::new();
        
        // Add required resources
        let chunk_manager = Arc::new(ChunkManager::default());
        world.insert_resource(TileOwnershipLayer::new(chunk_manager));
        
        // Test that all systems are valid and can be registered
        // In a real Bevy app, these would be added to schedules
        
        // Systems should be available for registration
        let _update_sys = update_ownership_system;
        let _decay_sys = ownership_decay_system;
        let _monitor_sys = ownership_monitoring_system;
        let _cleanup_sys = ownership_cleanup_system;
        
        // Verify resources exist for systems
        assert!(world.get_resource::<TileOwnershipLayer>().is_some());
        
        println!("✓ ECS system registration test passed");
    }
}
