//! Cache management and invalidation
//!
//! Contains methods for managing query caches and cache invalidation strategies.

use bevy_ecs::prelude::*;
use glam::IVec2;

use crate::core::caching::{broadcast_cache_invalidation, global_cache_events, SubsystemStats};
use crate::ecs::archetypes::ArchetypeManager;

use super::core::GameWorld;

impl GameWorld {
    /// Advance world generation and invalidate all caches
    pub async fn advance_world_generation(&mut self) {
        self.increment_world_generation();
        self.query_cache().clear().await;
        
        // Notify subsystems
        tokio::spawn(async move {
            broadcast_cache_invalidation(crate::core::caching::events::CacheInvalidationEvent::TileUpdated { 
                tile_id: 0, 
                batch_size: 1 
            }).await;
        });
    }

    /// Invalidate caches when entity is modified
    pub async fn invalidate_entity_caches(&self, entity: Entity, _archetype_changed: bool, _position_changed: Option<IVec2>) {
        tokio::spawn(async move {
            broadcast_cache_invalidation(crate::core::caching::events::CacheInvalidationEvent::TileUpdated { 
                tile_id: 0, 
                batch_size: 1 
            }).await;
        });
    }

    /// Report all cache metrics to unified system
    pub async fn report_all_cache_metrics(&self) {
        // Report archetype cache metrics
        if let Some(archetype_manager) = self.world.get_resource::<ArchetypeManager>() {
            archetype_manager.report_metrics().await;
        }
        
        // Report main query cache metrics
        let stats = self.query_cache().stats().await;
        let subsystem_stats = SubsystemStats {
            hits: stats.total_hits,
            misses: stats.total_misses,
            entries: stats.cache_count,
            memory_usage_bytes: stats.memory_usage_bytes,
            avg_access_time_micros: stats.avg_access_time_micros,
            last_updated: std::time::Instant::now(),
        };
        global_cache_events().register_subsystem_metrics("world_query", subsystem_stats).await;
    }

    /// Clear all caches manually
    pub async fn clear_all_caches(&mut self) {
        self.query_cache().clear().await;
        
        // Clear archetype caches
        if let Some(archetype_manager) = self.world.get_resource::<ArchetypeManager>() {
            archetype_manager.clear_caches().await;
        }
        
        // Increment world generation
        self.increment_world_generation();
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> CacheStatistics {
        let query_stats = self.query_cache().stats().await;
        let archetype_stats = if let Some(archetype_manager) = self.world.get_resource::<ArchetypeManager>() {
            Some(archetype_manager.cache_stats().await)
        } else {
            None
        };

        CacheStatistics {
            query_cache: QueryCacheStats {
                hits: query_stats.total_hits,
                misses: query_stats.total_misses,
                entries: query_stats.cache_count as u64,
                memory_usage_bytes: query_stats.memory_usage_bytes,
                hit_rate: if query_stats.total_hits + query_stats.total_misses > 0 {
                    query_stats.total_hits as f32 / (query_stats.total_hits + query_stats.total_misses) as f32
                } else {
                    0.0
                },
            },
            archetype_stats: archetype_stats.map(|stats| ArchetypeCacheStats {
                entities_cached: stats.entries,
                archetypes_cached: stats.hits as usize, // Using hits as a proxy for archetype count
                memory_usage_bytes: stats.memory_usage_bytes,
            }),
            world_generation: self.world_generation(),
        }
    }
}

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
