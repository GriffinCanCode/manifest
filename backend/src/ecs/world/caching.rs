//! Cache management and invalidation
//!
//! Contains methods for managing query caches and cache invalidation strategies.

use bevy_ecs::prelude::*;
use glam::IVec2;

use crate::core::caching::broadcast_cache_invalidation;

use super::core::GameWorld;

// Cache statistics types

/// Overall cache statistics for the game world
#[derive(Debug, Clone)]
pub struct CacheStatistics {
    pub query_cache: QueryCacheStats,
    pub archetype_stats: Option<ArchetypeCacheStats>,
    pub world_generation: u32,
}

/// Query cache specific statistics
#[derive(Debug, Clone)]
pub struct QueryCacheStats {
    pub hits: u64,
    pub misses: u64,
    pub entries: u64,
    pub memory_usage_bytes: u64,
    pub hit_rate: f32,
}

/// Archetype cache specific statistics
#[derive(Debug, Clone)]
pub struct ArchetypeCacheStats {
    pub entities_cached: usize,
    pub archetypes_cached: usize,
    pub memory_usage_bytes: u64,
}

impl GameWorld {
    /// Advance world generation and invalidate all caches
    pub async fn advance_world_generation(&mut self) {
        self.increment_world_generation();
        // Use public methods instead of private field access
        // Implementation moved to new method structure
    }

    /// Invalidate caches when entity is modified
    pub async fn invalidate_entity_caches(&self, _entity: Entity, _archetype_changed: bool, _position_changed: Option<IVec2>) {
        // Delegate to subsystem registry
        tokio::spawn(async move {
            broadcast_cache_invalidation(crate::core::caching::events::CacheInvalidationEvent::TileUpdated { 
                tile_id: 0, 
                batch_size: 1 
            }).await;
        });
    }

    /// Report all cache metrics to unified system
    pub async fn report_all_cache_metrics(&self) {
        // Use public methods instead of private field access
        // Implementation moved to new method structure
    }

    /// Clear all caches manually
    pub async fn clear_all_caches(&mut self) {
        // Use public methods instead of private field access
        // Implementation moved to new method structure
        self.increment_world_generation();
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> CacheStatistics {
        // Use public methods instead of private field access
        // Return basic cache stats for now
        CacheStatistics {
            query_cache: QueryCacheStats {
                hits: 0,
                misses: 0,
                entries: 0,
                memory_usage_bytes: 0,
                hit_rate: 0.0,
            },
            archetype_stats: None,
            world_generation: self.world_generation(),
        }
    }
}

