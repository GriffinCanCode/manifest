//! Subsystem registry and coordination
//!
//! Coordinates between specialized subsystems like spatial indexing, caching,
//! archetypes, hierarchy, and hot reload functionality.

use bevy_ecs::prelude::*;
use std::time::Instant;
use tracing::{info, warn, error, debug};

use crate::core::{
    caching::{GameCache, GameCacheBuilder, broadcast_cache_invalidation, global_cache_events, SubsystemStats, events::CacheInvalidationEvent},
    logging::{LoggingSystem, game_logging},
    reloader::ReloadManager
};
use crate::ecs::{
    spatial::OptimalSpatialIndex,
    hierarchy::HierarchyQueries,
    resources::GameTime,
    world::caching::{CacheStatistics, QueryCacheStats, ArchetypeCacheStats}
};

/// Registry and coordinator for all specialized subsystems
#[derive(Debug)]
pub struct SubsystemRegistry {
    /// High-performance spatial indexing using R-tree
    spatial_index: OptimalSpatialIndex,
    /// High-performance query result cache
    query_cache: GameCache,
    /// Hot reload manager for live development
    #[cfg(debug_assertions)]
    reload_manager: Option<ReloadManager>,
}

impl SubsystemRegistry {
    /// Create new subsystem registry with all subsystems initialized
    pub fn new(world: &mut World) -> Self {
        // Create and insert spatial index
        let spatial_index = OptimalSpatialIndex::new();
        world.insert_resource(spatial_index.clone());
        info!("🏗️ Spatial indexing system initialized");

        // Create high-performance cache for query results
        let query_cache = GameCacheBuilder::new()
            .max_memory_mb(256)
            .default_ttl(std::time::Duration::from_secs(30))
            .turn_based_invalidation(true)
            .build();

        // Setup hot reload in debug builds
        #[cfg(debug_assertions)]
        let reload_manager = Self::setup_hot_reload();

        Self {
            spatial_index,
            query_cache,
            #[cfg(debug_assertions)]
            reload_manager,
        }
    }

    /// Access the high-performance spatial index
    pub fn spatial_index(&self) -> &OptimalSpatialIndex {
        &self.spatial_index
    }

    /// Get reference to the query cache
    pub fn query_cache(&self) -> &GameCache {
        &self.query_cache
    }

    /// Setup hot reload system for development builds
    #[cfg(debug_assertions)]
    fn setup_hot_reload() -> Option<ReloadManager> {
        use std::path::Path;
        use crate::core::reloader::{LuaHandler, ConfigHandler, AssetHandler};
        
        match ReloadManager::new() {
            Ok(mut manager) => {
                // Add default handlers
                manager.add_handler(Box::new(LuaHandler::new().expect("Failed to initialize Lua handler for hot reload")));
                manager.add_handler(Box::new(ConfigHandler::new()));
                manager.add_handler(Box::new(AssetHandler::new()));

                // Watch common script/config directories
                let watch_dirs = [
                    "lua-scripts",
                    "configs", 
                    "assets",
                    "backend/src",  // For system files (informational only)
                ];

                for dir in &watch_dirs {
                    let path = Path::new(dir);
                    if path.exists() {
                        Self::watch_directory_recursive(&mut manager, path);
                    }
                }

                // Start the reloader
                if manager.start().is_ok() {
                    info!("🔥 Hot reload system activated");
                    Some(manager)
                } else {
                    warn!("Failed to start hot reload system");
                    None
                }
            }
            Err(e) => {
                warn!("Hot reload system disabled: {}", e);
                None
            }
        }
    }

    /// Watch directory recursively for file changes
    #[cfg(debug_assertions)]
    fn watch_directory_recursive(manager: &mut ReloadManager, path: &std::path::Path) {
        use crate::core::reloader::FileType;

        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if entry_path.is_file() {
                    let file_type = match entry_path.extension().and_then(|ext| ext.to_str()) {
                        Some("lua") => Some(FileType::Lua),
                        Some("toml" | "json" | "yaml" | "yml") => Some(FileType::Config),
                        Some("png" | "jpg" | "wav" | "glb") => Some(FileType::Asset),
                        _ => None,
                    };
                    
                    if let Some(ft) = file_type {
                        let _ = manager.watch_file(entry_path, ft);
                    }
                } else if entry_path.is_dir() {
                    Self::watch_directory_recursive(manager, &entry_path);
                }
            }
        }
    }

    /// Get reference to the reload manager (debug builds only)
    #[cfg(debug_assertions)]
    pub fn reload_manager(&self) -> &Option<ReloadManager> {
        &self.reload_manager
    }

    /// Get mutable reference to the reload manager (debug builds only)
    #[cfg(debug_assertions)]
    pub fn reload_manager_mut(&mut self) -> &mut Option<ReloadManager> {
        &mut self.reload_manager
    }

    /// Process hot reload events
    #[cfg(debug_assertions)]
    pub fn process_reload_events(&mut self) {
        use crate::core::reloader::ReloadEvent;
        
        if let Some(ref manager) = self.reload_manager {
            for event in manager.poll_events() {
                match event {
                    ReloadEvent::Reloaded { path, handler } => {
                        debug!("🔄 Reloaded {} with {}", path.display(), handler);
                    }
                    ReloadEvent::Failed { path, error } => {
                        warn!("❌ Reload failed for {}: {}", path.display(), error);
                    }
                    ReloadEvent::FileChanged { path } => {
                        debug!("📝 File changed: {}", path.display());
                    }
                }
            }
        }
    }

    /// Validate hierarchical data periodically
    pub fn validate_hierarchy(&self, world: &mut World, delta: f32) {
        // Validate hierarchical data periodically (every 5 seconds in debug, 30 seconds in release)
        let validation_interval = if cfg!(debug_assertions) { 5.0 } else { 30.0 };
        
        if let Some(game_time) = world.get_resource::<GameTime>() {
            // Use tick count as a rough approximation for total elapsed time
            let elapsed_time = game_time.tick as f32 * game_time.delta_time;
            if elapsed_time % validation_interval < delta {
                let correlation_id = LoggingSystem::generate_correlation_id();
                let validation_start = Instant::now();
                
                // Temporarily remove the resource to avoid borrow conflicts
                if let Some(hierarchy_queries) = world.remove_resource::<HierarchyQueries>() {
                    let validation_result = hierarchy_queries.validate_hierarchy(world);
                    // Put the resource back
                    world.insert_resource(hierarchy_queries);
                    
                    match validation_result {
                        Ok(validation) => {
                            let validation_duration = validation_start.elapsed().as_secs_f64() * 1000.0;
                            
                            if validation.has_cycles {
                                error!(
                                    target: "game::world::hierarchy",
                                    correlation_id = correlation_id,
                                    entity_count = validation.entity_count,
                                    relationship_count = validation.relationship_count,
                                    "Hierarchy cycles detected!"
                                );
                            }
                            
                            if validation.orphaned_entities > 0 {
                                warn!(
                                    target: "game::world::hierarchy",
                                    correlation_id = correlation_id,
                                    orphaned_entities = validation.orphaned_entities,
                                    "Found orphaned entities in hierarchy"
                                );
                            }
                            
                            debug!(
                                target: "game::world::hierarchy",
                                correlation_id = correlation_id,
                                entity_count = validation.entity_count,
                                relationship_count = validation.relationship_count,
                                orphaned_entities = validation.orphaned_entities,
                                has_cycles = validation.has_cycles,
                                validation_duration_ms = validation_duration,
                                "Hierarchy validation completed"
                            );
                            
                            game_logging::log_performance_event("hierarchy_validation", validation_duration, validation.entity_count);
                        }
                        Err(e) => {
                            error!(
                                target: "game::world::hierarchy",
                                correlation_id = correlation_id,
                                error = %e,
                                "Hierarchy validation failed"
                            );
                        }
                    }
                }
            }
        }
    }

    /// Advance world generation and invalidate all caches
    pub async fn advance_world_generation(&mut self) {
        self.query_cache.clear().await;
        
        // Notify subsystems
        tokio::spawn(async move {
            broadcast_cache_invalidation(CacheInvalidationEvent::TileUpdated { 
                tile_id: 0, 
                batch_size: 1 
            }).await;
        });
    }

    /// Report all cache metrics to unified system
    pub async fn report_all_cache_metrics(&self, world: &World) {
        // Archetype cache metrics would be reported here if available
        
        // Report main query cache metrics
        let stats = self.query_cache.stats().await;
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
    pub async fn clear_all_caches(&mut self, world: &World) {
        self.query_cache.clear().await;
        
        // Archetype caches would be cleared here if available
    }

    /// Get cache statistics with archetype information
    pub async fn cache_stats(&self, world: &World, world_generation: u32) -> CacheStatistics {
        let query_stats = self.query_cache.stats().await;
        
        // Calculate actual archetype statistics
        let archetype_stats = self.calculate_archetype_stats(world);

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
            archetype_stats: Some(archetype_stats),
            world_generation,
        }
    }

    /// Calculate archetype statistics for performance monitoring
    fn calculate_archetype_stats(&self, world: &World) -> ArchetypeCacheStats {
        let archetypes = world.archetypes();
        let total_archetypes = archetypes.len();
        let total_entities: usize = archetypes.iter().map(|a| a.len()).sum();
        
        // Calculate memory usage (rough estimate)
        let memory_usage_bytes = total_entities * 64 + total_archetypes * 1024; // Rough estimate
        
        ArchetypeCacheStats {
            entities_cached: total_entities,
            archetypes_cached: total_archetypes,
            memory_usage_bytes: memory_usage_bytes as u64,
        }
    }
}

impl Default for SubsystemRegistry {
    fn default() -> Self {
        // We can't create a default registry without a World reference,
        // so this implementation will panic if used directly.
        // The proper way is to call SubsystemRegistry::new(world).
        panic!("SubsystemRegistry::default() requires a World reference. Use SubsystemRegistry::new(world) instead.");
    }
}
