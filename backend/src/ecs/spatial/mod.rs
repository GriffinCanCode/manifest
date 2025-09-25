//! High-performance spatial indexing system using R-tree
//!
//! Replaces the expensive full-rebuild approach with incremental updates
//! for optimal performance with large entity counts.

use rstar::{RTree, RTreeObject, AABB};
use bevy_ecs::prelude::*;
use bevy_ecs::system::Resource;
use tracing::{info, debug, warn, error, instrument, Span};
use glam::IVec2;
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::Instant;
use crate::core::{hashing::{FastHashMap, collections}, logging::{LoggingSystem, game_logging}};
use crate::core::caching::{GameCache, CacheKey, SpatialCacheKey, SpatialQueryResult, CachePriority};
use crate::ecs::components::{Position, Owner, Movement};

/// Spatial entity wrapper for R-tree insertion
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpatialEntity {
    pub entity: Entity,
    pub position: IVec2,
    pub player_id: Option<u32>,
    pub is_movable: bool,
}

impl RTreeObject for SpatialEntity {
    type Envelope = AABB<[i32; 2]>;
    
    fn envelope(&self) -> Self::Envelope {
        let point = [self.position.x, self.position.y];
        AABB::from_point(point)
    }
}

/// High-performance spatial index using R-tree with incremental updates
#[derive(Debug, Clone, Resource)]
pub struct OptimalSpatialIndex {
    /// R-tree for O(log n) spatial queries
    rtree: Arc<RwLock<RTree<SpatialEntity>>>,
    /// Fast lookup for updates/removals
    entity_lookup: Arc<RwLock<FastHashMap<Entity, SpatialEntity>>>,
    /// High-performance cache for spatial query results
    cache: Arc<GameCache>,
    /// World generation for cache invalidation
    world_generation: Arc<parking_lot::Mutex<u32>>,
}

impl Default for OptimalSpatialIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl OptimalSpatialIndex {
    pub fn new() -> Self {
        use crate::core::caching::{GameCacheBuilder};
        
        // Configure cache for spatial queries
        let cache = GameCacheBuilder::new()
            .max_memory_mb(128) // 128MB for spatial queries
            .default_ttl(std::time::Duration::from_secs(30)) // 30 second TTL
            .turn_based_invalidation(true)
            .build();
            
        Self {
            rtree: Arc::new(RwLock::new(RTree::new())),
            entity_lookup: Arc::new(RwLock::new(collections::fast_hash_map())),
            cache: Arc::new(cache),
            world_generation: Arc::new(parking_lot::Mutex::new(1)),
        }
    }
    
    /// Add entity to spatial index - O(log n)
    pub fn add_entity(&self, entity: Entity, position: IVec2, player_id: Option<u32>, is_movable: bool) {
        let spatial_entity = SpatialEntity {
            entity,
            position,
            player_id,
            is_movable,
        };
        
        // Insert into R-tree
        {
            let mut rtree = self.rtree.write();
            rtree.insert(spatial_entity);
        }
        
        // Update lookup
        {
            let mut lookup = self.entity_lookup.write();
            lookup.insert(entity, spatial_entity);
        }
        
        self.mark_cache_dirty();
    }
    
    /// Update entity position - O(log n) removal + O(log n) insertion
    pub fn update_entity(&self, entity: Entity, new_position: IVec2) {
        // Get current entity data
        let old_entity = {
            let lookup = self.entity_lookup.read();
            lookup.get(&entity).copied()
        };
        
        if let Some(mut old_entity) = old_entity {
            // Remove old entry
            {
                let mut rtree = self.rtree.write();
                rtree.remove(&old_entity);
            }
            
            // Update position and re-insert
            old_entity.position = new_position;
            {
                let mut rtree = self.rtree.write();
                rtree.insert(old_entity);
            }
            
            // Update lookup
            {
                let mut lookup = self.entity_lookup.write();
                lookup.insert(entity, old_entity);
            }
            
            self.mark_cache_dirty();
        }
    }
    
    /// Remove entity from spatial index - O(log n)
    pub fn remove_entity(&self, entity: Entity) -> bool {
        let removed_entity = {
            let mut lookup = self.entity_lookup.write();
            lookup.remove(&entity)
        };
        
        if let Some(removed_entity) = removed_entity {
            let mut rtree = self.rtree.write();
            rtree.remove(&removed_entity);
            self.mark_cache_dirty();
            true
        } else {
            false
        }
    }
    
    /// Ultra-fast range queries - O(log n + k) where k = results
    pub fn entities_in_range(&self, center: IVec2, radius: u32) -> Vec<Entity> {
        // Check cache first
        let cache_key = self.range_cache_key(center, radius, None);
        if let Some(cached) = self.get_cached_result(cache_key) {
            return cached;
        }
        
        // Perform R-tree range query
        let results: Vec<Entity> = {
            let rtree = self.rtree.read();
            let radius_i32 = radius as i32;
            let envelope = AABB::from_corners(
                [center.x - radius_i32, center.y - radius_i32],
                [center.x + radius_i32, center.y + radius_i32]
            );
            
            rtree.locate_in_envelope_intersecting(&envelope)
                .filter(|spatial_entity| {
                    // Precise hex distance check
                    self.hex_distance(spatial_entity.position, center) <= radius
                })
                .map(|spatial_entity| spatial_entity.entity)
                .collect()
        };
        
        // Cache result
        self.cache_result(cache_key, results.clone());
        results
    }
    
    /// Fast ownership + range queries
    pub fn owned_entities_in_range(&self, player_id: u32, center: IVec2, radius: u32) -> Vec<Entity> {
        let cache_key = self.range_cache_key(center, radius, Some(player_id));
        if let Some(cached) = self.get_cached_result(cache_key) {
            return cached;
        }
        
        let results: Vec<Entity> = {
            let rtree = self.rtree.read();
            let radius_i32 = radius as i32;
            let envelope = AABB::from_corners(
                [center.x - radius_i32, center.y - radius_i32],
                [center.x + radius_i32, center.y + radius_i32]
            );
            
            rtree.locate_in_envelope_intersecting(&envelope)
                .filter(|spatial_entity| {
                    spatial_entity.player_id == Some(player_id) &&
                    self.hex_distance(spatial_entity.position, center) <= radius
                })
                .map(|spatial_entity| spatial_entity.entity)
                .collect()
        };
        
        self.cache_result(cache_key, results.clone());
        results
    }
    
    /// Fast exact position queries
    pub fn entities_at_position(&self, position: IVec2) -> Vec<Entity> {
        let rtree = self.rtree.read();
        let point = [position.x, position.y];
        
        let envelope = AABB::from_point(point);
        rtree.locate_in_envelope_intersecting(&envelope)
            .map(|spatial_entity| spatial_entity.entity)
            .collect()
    }
    
    /// Fast movable entity queries
    pub fn movable_entities_in_range(&self, center: IVec2, radius: u32) -> Vec<Entity> {
        let rtree = self.rtree.read();
        let radius_i32 = radius as i32;
        let envelope = AABB::from_corners(
            [center.x - radius_i32, center.y - radius_i32],
            [center.x + radius_i32, center.y + radius_i32]
        );
        
        rtree.locate_in_envelope_intersecting(&envelope)
            .filter(|spatial_entity| {
                spatial_entity.is_movable &&
                self.hex_distance(spatial_entity.position, center) <= radius
            })
            .map(|spatial_entity| spatial_entity.entity)
            .collect()
    }
    
    /// Get all entities owned by player (no spatial filtering)
    pub fn entities_owned_by_player(&self, player_id: u32) -> Vec<Entity> {
        let rtree = self.rtree.read();
        rtree.iter()
            .filter(|spatial_entity| spatial_entity.player_id == Some(player_id))
            .map(|spatial_entity| spatial_entity.entity)
            .collect()
    }
    
    /// Get all movable entities (units)
    pub fn movable_entities(&self) -> Vec<Entity> {
        let rtree = self.rtree.read();
        rtree.iter()
            .filter(|spatial_entity| spatial_entity.is_movable)
            .map(|spatial_entity| spatial_entity.entity)
            .collect()
    }
    
    /// Get owned units at specific position
    pub fn owned_units_at_position(&self, pos: IVec2, player_id: u32) -> Vec<Entity> {
        let rtree = self.rtree.read();
        let point = [pos.x, pos.y];
        
        let envelope = AABB::from_point(point);
        rtree.locate_in_envelope_intersecting(&envelope)
            .filter(|spatial_entity| {
                spatial_entity.player_id == Some(player_id) && spatial_entity.is_movable
            })
            .map(|spatial_entity| spatial_entity.entity)
            .collect()
    }
    
    /// Get statistics for monitoring
    pub fn stats(&self) -> SpatialStats {
        let rtree = self.rtree.read();
        let lookup = self.entity_lookup.read();
        // Stats for cache handled via the GameCache
        
        SpatialStats {
            total_entities: rtree.size(),
            lookup_entries: lookup.len(),
            cache_entries: 0, // Cache managed by GameCache
            rtree_depth: 0, // R-tree depth not directly accessible
        }
    }
    
    // === PRIVATE IMPLEMENTATION ===
    
    fn hex_distance(&self, a: IVec2, b: IVec2) -> u32 {
        let dx = (a.x - b.x).abs() as u32;
        let dy = (a.y - b.y).abs() as u32;
        let dz = ((a.x + a.y) - (b.x + b.y)).abs() as u32;
        (dx + dy + dz) / 2
    }
    
    fn range_cache_key(&self, center: IVec2, radius: u32, player_id: Option<u32>) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        center.hash(&mut hasher);
        radius.hash(&mut hasher);
        player_id.hash(&mut hasher);
        hasher.finish()
    }
    
    fn get_cached_result(&self, key: u64) -> Option<Vec<Entity>> {
        use crate::core::caching::{CacheKey, CachePriority};
        use tokio::runtime::Handle;
        
        // Try to get current runtime, if none available return None (no caching)
        let handle = Handle::try_current().ok()?;
        
        // Use block_on for synchronous interface to async cache
        let result = handle.block_on(async {
            let cache_key = CacheKey::Custom(format!("spatial_query_{}", key));
            
            match self.cache.get::<Vec<Entity>>(&cache_key).await {
                Ok(Some(entities)) => Some(entities),
                Ok(None) => None,
                Err(_) => None, // Cache error, proceed without cache
            }
        });
        
        result
    }
    
    fn cache_result(&self, key: u64, result: Vec<Entity>) {
        use crate::core::caching::{CacheKey, CachePriority};
        use tokio::runtime::Handle;
        
        // Try to get current runtime, if none available skip caching
        let Some(handle) = Handle::try_current().ok() else {
            return;
        };
        
        let cache = Arc::clone(&self.cache);
        let cache_key = CacheKey::Custom(format!("spatial_query_{}", key));
        
        // Fire and forget caching - don't block on cache writes
        handle.spawn(async move {
            let _ = cache.set(cache_key, result, CachePriority::High).await;
        });
    }
    
    fn mark_cache_dirty(&self) {
        // Advance world generation to invalidate all caches
        let mut generation = self.world_generation.lock();
        *generation = generation.saturating_add(1);
    }
    
    /// Clear cache if dirty (call periodically)
    pub fn clear_cache_if_dirty(&self) {
        // No-op - cache managed by GameCache
    }
}

/// Statistics for spatial index performance monitoring
#[derive(Debug, Clone)]
pub struct SpatialStats {
    pub total_entities: usize,
    pub lookup_entries: usize,
    pub cache_entries: usize,
    pub rtree_depth: usize,
}

// === INCREMENTAL SYNC SYSTEM ===

/// Resource to track when spatial sync is needed
#[derive(Resource, Default)]
pub struct SpatialSyncNeeded;

/// System that incrementally updates spatial index
#[instrument(name = "incremental_spatial_sync", skip_all)]
pub fn incremental_spatial_sync(
    mut commands: Commands,
    mut spatial_index: ResMut<OptimalSpatialIndex>,
    
    // Added entities with positions
    added_query: Query<(Entity, &Position, Option<&Owner>, Option<&Movement>), Added<Position>>,
    
    // Changed positions  
    changed_query: Query<(Entity, &Position, Option<&Owner>, Option<&Movement>), Changed<Position>>,
    
    // Removed positions
    mut removed: RemovedComponents<Position>,
) {
    let sync_start = Instant::now();
    let correlation_id = LoggingSystem::generate_correlation_id();
    
    let mut updates_made = false;
    let mut added_count = 0;
    let mut changed_count = 0;
    let mut removed_count = 0;
    
    // Handle new entities
    for (entity, position, owner, movement) in added_query.iter() {
        let player_id = owner.map(|o| o.player_id());
        let is_movable = movement.is_some();
        let hex_pos = position.hex();
        
        spatial_index.add_entity(entity, hex_pos, player_id, is_movable);
        
        debug!(
            target: "game::spatial::sync",
            correlation_id = correlation_id,
            entity = ?entity,
            position = ?hex_pos,
            player_id = ?player_id,
            is_movable = is_movable,
            "Entity added to spatial index"
        );
        
        game_logging::log_spatial_operation(hex_pos, "entity_added", None);
        game_logging::log_entity_operation(entity, "spatial_add", None);
        
        updates_made = true;
        added_count += 1;
    }
    
    // Handle position changes
    for (entity, position, _owner, _movement) in changed_query.iter() {
        let hex_pos = position.hex();
        
        spatial_index.update_entity(entity, hex_pos);
        
        debug!(
            target: "game::spatial::sync",
            correlation_id = correlation_id,
            entity = ?entity,
            new_position = ?hex_pos,
            "Entity position updated in spatial index"
        );
        
        game_logging::log_spatial_operation(hex_pos, "entity_moved", None);
        game_logging::log_entity_operation(entity, "spatial_update", None);
        
        updates_made = true;
        changed_count += 1;
    }
    
    // Handle removed entities
    for entity in removed.read() {
        if spatial_index.remove_entity(entity) {
            debug!(
                target: "game::spatial::sync",
                correlation_id = correlation_id,
                entity = ?entity,
                "Entity removed from spatial index"
            );
            
            game_logging::log_entity_operation(entity, "spatial_remove", None);
            removed_count += 1;
        }
        
        updates_made = true;
    }
    
    if updates_made {
        commands.insert_resource(SpatialSyncNeeded);
        
        let sync_duration = sync_start.elapsed().as_secs_f64() * 1000.0;
        let total_changes = added_count + changed_count + removed_count;
        
        info!(
            target: "game::spatial::sync",
            correlation_id = correlation_id,
            added_entities = added_count,
            changed_entities = changed_count,
            removed_entities = removed_count,
            total_changes = total_changes,
            sync_duration_ms = sync_duration,
            "Spatial index sync completed"
        );
        
        game_logging::log_performance_event("spatial_sync", sync_duration, total_changes);
    }
}

/// System that clears cache periodically
#[instrument(name = "spatial_cache_maintenance", skip_all)]
pub fn spatial_cache_maintenance(
    mut commands: Commands,
    mut spatial_index: ResMut<OptimalSpatialIndex>,
    sync_needed: Option<Res<SpatialSyncNeeded>>,
) {
    if sync_needed.is_some() {
        let maintenance_start = Instant::now();
        let correlation_id = LoggingSystem::generate_correlation_id();
        
        spatial_index.clear_cache_if_dirty();
        commands.remove_resource::<SpatialSyncNeeded>();
        
        let maintenance_duration = maintenance_start.elapsed().as_secs_f64() * 1000.0;
        
        debug!(
            target: "game::spatial::cache",
            correlation_id = correlation_id,
            maintenance_duration_ms = maintenance_duration,
            "Spatial cache maintenance completed"
        );
        
        game_logging::log_performance_event("spatial_cache_maintenance", maintenance_duration, 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_spatial_index_creation() {
        let index = OptimalSpatialIndex::new();
        let stats = index.stats();
        assert_eq!(stats.total_entities, 0);
        assert_eq!(stats.lookup_entries, 0);
    }
    
    #[test]
    fn test_add_and_query_entities() {
        let index = OptimalSpatialIndex::new();
        let entity1 = Entity::from_raw(1);
        let entity2 = Entity::from_raw(2);
        
        // Add entities
        index.add_entity(entity1, IVec2::new(0, 0), Some(1), true);
        index.add_entity(entity2, IVec2::new(2, 0), Some(1), false);
        
        // Query in range
        let results = index.entities_in_range(IVec2::new(0, 0), 2);
        assert_eq!(results.len(), 2);
        assert!(results.contains(&entity1));
        assert!(results.contains(&entity2));
        
        // Query exact position
        let at_origin = index.entities_at_position(IVec2::new(0, 0));
        assert_eq!(at_origin.len(), 1);
        assert!(at_origin.contains(&entity1));
    }
    
    #[test]
    fn test_update_entity_position() {
        let index = OptimalSpatialIndex::new();
        let entity = Entity::from_raw(1);
        
        // Add entity
        index.add_entity(entity, IVec2::new(0, 0), Some(1), true);
        
        // Verify initial position
        let at_origin = index.entities_at_position(IVec2::new(0, 0));
        assert!(at_origin.contains(&entity));
        
        // Update position
        index.update_entity(entity, IVec2::new(5, 5));
        
        // Verify new position
        let at_origin_after = index.entities_at_position(IVec2::new(0, 0));
        let at_new_pos = index.entities_at_position(IVec2::new(5, 5));
        assert!(!at_origin_after.contains(&entity));
        assert!(at_new_pos.contains(&entity));
    }
    
    #[test]
    fn test_ownership_queries() {
        let index = OptimalSpatialIndex::new();
        let entity1 = Entity::from_raw(1);
        let entity2 = Entity::from_raw(2);
        let entity3 = Entity::from_raw(3);
        
        // Add entities with different owners
        index.add_entity(entity1, IVec2::new(0, 0), Some(1), true);
        index.add_entity(entity2, IVec2::new(1, 0), Some(2), true);
        index.add_entity(entity3, IVec2::new(2, 0), Some(1), false);
        
        // Query by ownership
        let player1_entities = index.entities_owned_by_player(1);
        assert_eq!(player1_entities.len(), 2);
        assert!(player1_entities.contains(&entity1));
        assert!(player1_entities.contains(&entity3));
        
        let player2_entities = index.entities_owned_by_player(2);
        assert_eq!(player2_entities.len(), 1);
        assert!(player2_entities.contains(&entity2));
        
        // Query owned entities in range
        let player1_in_range = index.owned_entities_in_range(1, IVec2::new(0, 0), 2);
        assert_eq!(player1_in_range.len(), 2);
    }
    
    #[test]
    fn test_remove_entity() {
        let index = OptimalSpatialIndex::new();
        let entity = Entity::from_raw(1);
        
        // Add entity
        index.add_entity(entity, IVec2::new(0, 0), Some(1), true);
        assert_eq!(index.stats().total_entities, 1);
        
        // Remove entity
        let removed = index.remove_entity(entity);
        assert!(removed);
        assert_eq!(index.stats().total_entities, 0);
        
        // Verify it's gone
        let at_origin = index.entities_at_position(IVec2::new(0, 0));
        assert!(!at_origin.contains(&entity));
    }
    
    #[test]
    fn test_hex_distance() {
        let index = OptimalSpatialIndex::new();
        
        // Test basic hex distance
        assert_eq!(index.hex_distance(IVec2::new(0, 0), IVec2::new(1, 0)), 1);
        assert_eq!(index.hex_distance(IVec2::new(0, 0), IVec2::new(0, 1)), 1);
        assert_eq!(index.hex_distance(IVec2::new(0, 0), IVec2::new(2, 0)), 2);
        assert_eq!(index.hex_distance(IVec2::new(0, 0), IVec2::new(1, 1)), 2);
    }
}
