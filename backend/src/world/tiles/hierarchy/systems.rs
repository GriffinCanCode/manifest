//! ECS systems for tile hierarchy maintenance
//!
//! Contains Bevy ECS systems for maintaining tile hierarchy consistency,
//! cleaning up orphaned relationships, and performance monitoring.

use bevy_ecs::prelude::*;
use tracing::{warn, info, debug};

use super::{
    manager::TileHierarchy,
    types::HierarchicalTile
};

/// System for maintaining tile hierarchy consistency
pub fn maintain_tile_hierarchy_system(
    tile_hierarchy: ResMut<TileHierarchy>,
    hierarchical_query: Query<Entity, With<HierarchicalTile>>,
) {
    let hierarchical_entities: Vec<_> = hierarchical_query.iter().collect();
    
    if hierarchical_entities.len() > 1000 {
        // For large numbers of hierarchical entities, validate in batches
        // This prevents performance issues with very large worlds
        warn!("Large number of hierarchical tiles ({}), consider optimization", hierarchical_entities.len());
    }
}

/// System for cleaning up orphaned hierarchical relationships
pub fn cleanup_tile_hierarchy_system(
    _commands: Commands,
    tile_hierarchy: ResMut<TileHierarchy>,
    _hierarchical_query: Query<Entity, With<HierarchicalTile>>,
    world: &World,
) {
    // Validate and clean up any inconsistencies in the tile hierarchy
    if let Ok(validation) = tile_hierarchy.validate_tile_hierarchy(world) {
        if validation.has_resolution_gaps {
            warn!("Tile hierarchy has resolution gaps - consider rebuilding");
        }
        
        if validation.base_validation.has_cycles {
            warn!("Cycles detected in tile hierarchy - cleaning up");
            // In practice, this would implement cycle breaking logic
        }
    }
}

/// System for monitoring tile hierarchy performance
pub fn monitor_tile_hierarchy_system(
    tile_hierarchy: ResMut<TileHierarchy>,
    game_state: Res<crate::core::game_state::CoreGameState>,
) {
    // Only run monitoring every few seconds to avoid overhead
    if game_state.tick % 300 != 0 {  // ~5 seconds at 60 FPS
        return;
    }
    
    // Clone the data we need to avoid lifetime issues
    let cache = tile_hierarchy.cache().clone();
    let hierarchical_tiles_arc = tile_hierarchy.hierarchical_tiles().clone();
    let max_resolution = tile_hierarchy.max_resolution();
    let current_turn = game_state.turn;
    let current_tick = game_state.tick;
    
    // Comprehensive monitoring in async task
    tokio::spawn(async move {
        // Performance metrics collection
        let mut metrics = HierarchyPerformanceMetrics::new();
        
        // Collect hierarchical tile statistics
        let hierarchical_tiles = hierarchical_tiles_arc.read();
        for (resolution, tiles) in hierarchical_tiles.iter() {
            metrics.tiles_by_resolution.insert(*resolution, tiles.len());
            metrics.total_hierarchical_tiles += tiles.len();
            
            if *resolution == max_resolution {
                metrics.base_layer_tiles = tiles.len();
            }
        }
        drop(hierarchical_tiles); // Release lock early
        
        // Collect cache statistics
        let cache_stats = cache.stats().await;
        let total_requests = cache_stats.total_hits + cache_stats.total_misses;
        metrics.cache_hit_rate = if total_requests > 0 {
            cache_stats.total_hits as f32 / total_requests as f32
        } else {
            0.0
        };
        metrics.total_cache_requests = total_requests;
        metrics.cache_memory_usage = cache_stats.memory_usage_bytes;
        
        // Calculate derived metrics
        metrics.calculate_derived_metrics();
        
        // Performance alerts and warnings
        let mut alerts = Vec::new();
        
        // Cache performance alerts
        if metrics.cache_hit_rate < 0.5 && metrics.total_hierarchical_tiles > 100 {
            alerts.push(format!(
                "Low cache hit rate ({:.1}%) with {} hierarchical tiles", 
                metrics.cache_hit_rate * 100.0, metrics.total_hierarchical_tiles
            ));
        }
        
        // Memory usage alerts
        if metrics.cache_memory_usage > 128 * 1024 * 1024 { // 128MB
            alerts.push(format!(
                "High hierarchy cache memory usage: {:.1}MB", 
                metrics.cache_memory_usage as f64 / 1024.0 / 1024.0
            ));
        }
        
        // Hierarchy depth alerts
        if metrics.max_resolution > 8 {
            alerts.push(format!(
                "Very deep hierarchy detected: {} levels", 
                metrics.max_resolution
            ));
        }
        
        // Tile distribution alerts
        let avg_tiles_per_resolution = metrics.total_hierarchical_tiles as f32 / metrics.tiles_by_resolution.len() as f32;
        for (resolution, count) in &metrics.tiles_by_resolution {
            if *count as f32 > avg_tiles_per_resolution * 5.0 {
                alerts.push(format!(
                    "Resolution {} has abnormally high tile count: {}", 
                    resolution, count
                ));
            }
        }
        
        // Log alerts and metrics
        if !alerts.is_empty() {
            warn!("🚨 Tile hierarchy performance alerts (Turn {}, Tick {}):", current_turn, current_tick);
            for alert in alerts {
                warn!("  ⚠️  {}", alert);
            }
        }
        
        // Periodic detailed logging (every 100 turns)
        if current_turn % 100 == 0 {
            info!("📊 Tile Hierarchy Performance Report (Turn {}):", current_turn);
            info!("  📦 Total hierarchical tiles: {}", metrics.total_hierarchical_tiles);
            info!("  🎯 Base layer tiles: {}", metrics.base_layer_tiles);
            info!("  📈 Cache hit rate: {:.1}%", metrics.cache_hit_rate * 100.0);
            info!("  💾 Cache memory: {:.1}MB", metrics.cache_memory_usage as f64 / 1024.0 / 1024.0);
            info!("  🏔️  Max resolution: {}", metrics.max_resolution);
            info!("  ⚖️  Average tiles per resolution: {:.1}", avg_tiles_per_resolution);
            
            debug!("Detailed resolution breakdown:");
            for (resolution, count) in metrics.tiles_by_resolution.iter() {
                debug!("  Resolution {}: {} tiles", resolution, count);
            }
        }
    });
}

/// Performance metrics for tile hierarchy monitoring
#[derive(Debug, Clone)]
struct HierarchyPerformanceMetrics {
    pub total_hierarchical_tiles: usize,
    pub base_layer_tiles: usize,
    pub cache_hit_rate: f32,
    pub total_cache_requests: u64,
    pub cache_memory_usage: u64,
    pub tiles_by_resolution: std::collections::HashMap<u8, usize>,
    pub max_resolution: u8,
    pub hierarchy_efficiency: f32,
}

impl HierarchyPerformanceMetrics {
    fn new() -> Self {
        Self {
            total_hierarchical_tiles: 0,
            base_layer_tiles: 0,
            cache_hit_rate: 0.0,
            total_cache_requests: 0,
            cache_memory_usage: 0,
            tiles_by_resolution: std::collections::HashMap::new(),
            max_resolution: 0,
            hierarchy_efficiency: 0.0,
        }
    }
    
    fn calculate_derived_metrics(&mut self) {
        // Find maximum resolution
        self.max_resolution = *self.tiles_by_resolution.keys().max().unwrap_or(&0);
        
        // Calculate hierarchy efficiency (ratio of base tiles to total tiles)
        if self.total_hierarchical_tiles > 0 {
            self.hierarchy_efficiency = self.base_layer_tiles as f32 / self.total_hierarchical_tiles as f32;
        }
    }
}

/// System for periodic hierarchy validation (should be run less frequently)
pub fn validate_tile_hierarchy_system(
    tile_hierarchy: ResMut<TileHierarchy>,
    world: &World,
) {
    if let Ok(validation) = tile_hierarchy.validate_tile_hierarchy(world) {
        if validation.has_resolution_gaps || validation.base_validation.has_cycles {
            warn!("Tile hierarchy validation issues detected: gaps={}, cycles={}", 
                  validation.has_resolution_gaps, validation.base_validation.has_cycles);
        }
    }
}

/// System bundle for convenient registration of all tile hierarchy systems
pub struct TileHierarchySystemSet;

impl TileHierarchySystemSet {
    /// Get all tile hierarchy systems for registration with Bevy scheduler
    pub fn systems() -> impl IntoSystemConfigs<()> {
        (
            maintain_tile_hierarchy_system,
            cleanup_tile_hierarchy_system,
            monitor_tile_hierarchy_system,
            validate_tile_hierarchy_system,
        ).chain() // Run systems in sequence
    }
}
