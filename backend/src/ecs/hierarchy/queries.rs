//! High-performance hierarchy queries with rayon parallelization
//!
//! Provides optimized queries for entity relationships, ancestors, descendants,
//! and complex hierarchy traversals with parallel execution.

use bevy_ecs::prelude::*;
use rayon::prelude::*;
use petgraph::Direction;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::{
    components::{Relationships, RelationshipType, Hierarchical},
    graph::{EntityGraph, HierarchyError, HierarchyResult},
};
use crate::core::{
    hashing::{FastHashMap, FastHashSet},
    caching::{GameCache, GameCacheBuilder, CacheKey, CachePriority, global_cache_events, SubsystemStats}
};

/// High-performance hierarchy query system that integrates with ECS
#[derive(Debug, Resource)]
pub struct HierarchyQueries {
    /// Core entity relationship graph
    graph: Arc<EntityGraph>,
    /// Unified game cache for hierarchy operations
    cache: GameCache,
    /// World generation for cache invalidation
    world_generation: u32,
}

impl Default for HierarchyQueries {
    fn default() -> Self {
        Self::new()
    }
}


impl HierarchyQueries {
    /// Create new hierarchy query system
    pub fn new() -> Self {
        let cache = GameCacheBuilder::new()
            .max_memory_mb(32) // 32MB for hierarchy data
            .default_ttl(std::time::Duration::from_secs(180)) // 3 minute TTL
            .turn_based_invalidation(false) // Hierarchy persists across turns
            .build();

        Self {
            graph: Arc::new(EntityGraph::new()),
            cache,
            world_generation: 1,
        }
    }

    /// Synchronize graph state with ECS world relationships
    /// This should be called regularly to keep the graph in sync
    pub async fn sync_with_world(&self, world: &mut World) -> HierarchyResult<()> {
        // Collect all entities with Relationships components
        let mut query = world.query::<(Entity, &Relationships)>();
        let updates: Vec<_> = query.iter(world)
            .map(|(entity, relationships)| (entity, relationships.clone()))
            .collect();

        // Batch update the graph
        self.graph.batch_update_relationships(updates)?;
        
        // Invalidate cache after sync
        self.invalidate_cache().await;
        
        Ok(())
    }

    /// Find all parent entities of the given entity
    pub fn parents(&self, entity: Entity) -> Vec<Entity> {
        self.graph.get_parents(entity)
    }

    /// Find all child entities of the given entity
    pub fn children(&self, entity: Entity) -> Vec<Entity> {
        self.graph.get_children(entity)
    }

    /// Find all ancestor entities (recursive parents) with caching
    pub async fn ancestors(&self, entity: Entity) -> Vec<Entity> {
        // Create cache key for ancestor query
        let cache_key = CacheKey::Custom(format!("ancestors:{}", entity.index()));
        
        // Check cache first
        if let Ok(Some(ancestors)) = self.cache.get::<Vec<Entity>>(&cache_key).await {
            return ancestors;
        }

        // Cache miss - compute ancestors
        let ancestors = self.graph.get_ancestors(entity);
        
        // Cache the result
        let _ = self.cache.set(cache_key, ancestors.clone(), CachePriority::Normal).await;

        ancestors
    }

    /// Find all descendant entities (recursive children) with caching
    pub async fn descendants(&self, entity: Entity) -> Vec<Entity> {
        // Create cache key for descendant query
        let cache_key = CacheKey::Custom(format!("descendants:{}", entity.index()));
        
        // Check cache first
        if let Ok(Some(descendants)) = self.cache.get::<Vec<Entity>>(&cache_key).await {
            return descendants;
        }

        // Cache miss - compute descendants
        let descendants = self.graph.get_descendants(entity);
        
        // Cache the result
        let _ = self.cache.set(cache_key, descendants.clone(), CachePriority::Normal).await;

        descendants
    }

    /// Find all entities owned by the given entity
    pub fn owned_entities(&self, entity: Entity) -> Vec<Entity> {
        self.graph.get_owned_entities(entity)
    }

    /// Find the owner of the given entity
    pub fn owner(&self, entity: Entity) -> Option<Entity> {
        self.graph.get_owner(entity)
    }

    /// Check if there's a path between two entities
    pub fn has_path(&self, from: Entity, to: Entity) -> bool {
        self.graph.has_path(from, to)
    }

    /// Find root entities (entities with no parents) in parallel
    pub fn find_roots(&self, world: &mut World) -> Vec<Entity> {
        let mut query = world.query_filtered::<Entity, With<Hierarchical>>();
        let entities: Vec<Entity> = query.iter(world).collect();
        
        entities
            .par_iter()
            .filter(|&&entity| self.parents(entity).is_empty())
            .copied()
            .collect()
    }

    /// Find leaf entities (entities with no children) in parallel
    pub fn find_leaves(&self, world: &mut World) -> Vec<Entity> {
        let mut query = world.query_filtered::<Entity, With<Hierarchical>>();
        let entities: Vec<Entity> = query.iter(world).collect();
        
        entities
            .par_iter()
            .filter(|&&entity| self.children(entity).is_empty())
            .copied()
            .collect()
    }

    /// Find all entities at a specific hierarchy depth from a root
    pub fn entities_at_depth(&self, root: Entity, depth: u32) -> Vec<Entity> {
        if depth == 0 {
            return vec![root];
        }

        let mut current_level = vec![root];
        
        for _ in 0..depth {
            current_level = current_level
                .par_iter()
                .flat_map(|&entity| self.children(entity))
                .collect();
            
            if current_level.is_empty() {
                break;
            }
        }

        current_level
    }

    /// Calculate hierarchy depth (maximum distance from root to leaf)
    pub fn hierarchy_depth(&self, root: Entity) -> u32 {
        let mut max_depth = 0;
        let mut current_level = vec![root];
        
        while !current_level.is_empty() {
            current_level = current_level
                .par_iter()
                .flat_map(|&entity| self.children(entity))
                .collect();
            
            if !current_level.is_empty() {
                max_depth += 1;
            }
        }

        max_depth
    }

    /// Find common ancestors between two entities
    pub async fn common_ancestors(&self, entity1: Entity, entity2: Entity) -> Vec<Entity> {
        let ancestors1: FastHashSet<_> = self.ancestors(entity1).await.into_iter().collect();
        let ancestors2 = self.ancestors(entity2).await;

        ancestors2
            .into_iter()
            .filter(|ancestor| ancestors1.contains(&ancestor))
            .collect()
    }

    /// Find the lowest common ancestor of two entities
    pub async fn lowest_common_ancestor(&self, entity1: Entity, entity2: Entity) -> Option<Entity> {
        let ancestors1: FastHashSet<_> = self.ancestors(entity1).await.into_iter().collect();
        
        // Walk up from entity2 until we find a common ancestor
        for ancestor in self.ancestors(entity2).await {
            if ancestors1.contains(&ancestor) {
                return Some(ancestor);
            }
        }

        None
    }

    /// Find all entities in a subtree rooted at the given entity
    pub async fn subtree(&self, root: Entity) -> Vec<Entity> {
        let mut subtree = vec![root];
        subtree.extend(self.descendants(root).await);
        subtree
    }

    /// Find entities by relationship type in parallel
    pub fn find_by_relationship(
        &self,
        world: &mut World,
        relationship_type: RelationshipType,
        direction: Direction,
    ) -> FastHashMap<Entity, Vec<Entity>> {
        let mut query = world.query_filtered::<Entity, With<Relationships>>();
        let entities: Vec<Entity> = query.iter(world).collect();
        
        entities
            .par_iter()
            .map(|&entity| {
                let related = self.graph.get_related_entities(entity, relationship_type, direction);
                (entity, related)
            })
            .filter(|(_, related)| !related.is_empty())
            .collect()
    }

    /// Validate hierarchy integrity (no cycles, valid relationships)
    pub fn validate_hierarchy(&self) -> HierarchyResult<HierarchyValidation> {
        let stats = self.graph.stats();
        
        Ok(HierarchyValidation {
            has_cycles: stats.has_cycles,
            entity_count: stats.entity_count,
            relationship_count: stats.edge_count,
            orphaned_entities: self.count_orphaned_entities(),
        })
    }

    /// Count entities that exist in components but not in graph
    fn count_orphaned_entities(&self) -> usize {
        // This would need access to the world to implement fully
        // For now, return 0 as placeholder
        0
    }

    /// Perform hierarchical query with custom traversal function
    pub fn traverse_hierarchy<F, R>(&self, root: Entity, mut visitor: F) -> Vec<R>
    where
        F: FnMut(Entity, u32) -> Option<R> + Send + Sync,
        R: Send,
    {
        let mut results = Vec::new();
        let mut to_visit = vec![(root, 0u32)];

        while let Some((entity, depth)) = to_visit.pop() {
            if let Some(result) = visitor(entity, depth) {
                results.push(result);
            }

            // Add children to visit queue
            for child in self.children(entity) {
                to_visit.push((child, depth + 1));
            }
        }

        results
    }

    /// Get entities organized by hierarchy levels (breadth-first)
    pub fn hierarchy_levels(&self, root: Entity) -> Vec<Vec<Entity>> {
        let mut levels = Vec::new();
        let mut current_level = vec![root];

        while !current_level.is_empty() {
            levels.push(current_level.clone());
            
            current_level = current_level
                .par_iter()
                .flat_map(|&entity| self.children(entity))
                .collect();
        }

        levels
    }

    /// Batch query multiple entities for their relationships
    pub fn batch_relationship_query(
        &self,
        entities: &[Entity],
        relationship_type: RelationshipType,
        direction: Direction,
    ) -> FastHashMap<Entity, Vec<Entity>> {
        entities
            .par_iter()
            .map(|&entity| {
                let related = self.graph.get_related_entities(entity, relationship_type, direction);
                (entity, related)
            })
            .collect()
    }

    /// Clear all caches and invalidate cached results
    pub async fn invalidate_cache(&self) {
        self.cache.clear().await;
    }

    /// Advance world generation and invalidate caches
    pub async fn advance_world_generation(&mut self) {
        self.world_generation += 1;
        self.cache.clear().await;
    }

    /// Invalidate cache entries for a specific entity
    pub async fn invalidate_entity(&self, entity: Entity) {
        let entity_key = entity.index();
        
        // Remove all cache entries related to this entity
        let ancestor_key = CacheKey::Custom(format!("ancestors:{}", entity_key));
        let descendant_key = CacheKey::Custom(format!("descendants:{}", entity_key));
        
        self.cache.remove(&ancestor_key).await;
        self.cache.remove(&descendant_key).await;
        
        // Also invalidate any cached results that might include this entity
        // This is a simplified approach - a more sophisticated system would track dependencies
    }

    /// Get access to the underlying EntityGraph for testing
    #[cfg(test)]
    pub fn graph(&self) -> &Arc<EntityGraph> {
        &self.graph
    }

    /// Update hierarchy with new relationship data (public interface)
    pub fn update_relationships(&self, updates: Vec<(Entity, Relationships)>) -> HierarchyResult<()> {
        self.graph.batch_update_relationships(updates)?;
        self.invalidate_cache();
        Ok(())
    }

    /// Get performance statistics for the hierarchy system
    pub async fn performance_stats(&self) -> HierarchyPerformanceStats {
        let cache_stats = self.cache.stats().await;
        let graph_stats = self.graph.stats();

        HierarchyPerformanceStats {
            graph_stats,
            cached_ancestors: cache_stats.cache_count / 2, // Rough estimate for ancestor cache entries
            cached_descendants: cache_stats.cache_count / 2, // Rough estimate for descendant cache entries
            cache_version: self.world_generation as u64,
        }
    }

    /// Report cache metrics to the global metrics system
    pub async fn report_metrics(&self) {
        let cache_stats = self.cache.stats().await;
        let graph_stats = self.graph.stats();
        
        let subsystem_stats = SubsystemStats {
            hits: cache_stats.total_hits,
            misses: cache_stats.total_misses,
            entries: cache_stats.cache_count,
            memory_usage_bytes: cache_stats.memory_usage_bytes,
            avg_access_time_micros: cache_stats.avg_access_time_micros,
            last_updated: std::time::Instant::now(),
        };

        global_cache_events().register_subsystem_metrics("hierarchy", subsystem_stats).await;
    }
}

/// Hierarchy validation results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HierarchyValidation {
    pub has_cycles: bool,
    pub entity_count: usize,
    pub relationship_count: usize,
    pub orphaned_entities: usize,
}

/// Performance statistics for the hierarchy system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HierarchyPerformanceStats {
    pub graph_stats: super::graph::GraphStats,
    pub cached_ancestors: usize,
    pub cached_descendants: usize,
    pub cache_version: u64,
}

/// System for automatically syncing hierarchy with ECS world
pub fn sync_hierarchy_system(
    hierarchy_queries: Res<HierarchyQueries>,
    relationships_query: Query<(Entity, &Relationships)>,
) {
    // Collect all entities with Relationships components
    let updates: Vec<_> = relationships_query.iter()
        .map(|(entity, relationships)| (entity, relationships.clone()))
        .collect();

    // Batch update the graph
    if let Err(e) = hierarchy_queries.graph.batch_update_relationships(updates) {
        tracing::warn!("Failed to sync hierarchy with world: {}", e);
    }
    
    // Invalidate cache after sync
    hierarchy_queries.invalidate_cache();
}

/// System for cleaning up orphaned relationships
pub fn cleanup_hierarchy_system(
    mut commands: Commands,
    query: Query<(Entity, &Relationships), With<Hierarchical>>,
    hierarchy_queries: Res<HierarchyQueries>,
) {
    // Find entities with invalid relationships and clean them up
    for (entity, relationships) in query.iter() {
        let mut needs_cleanup = false;
        
        for relationship in relationships.iter() {
            // Check if target entity still exists in the world
            if !hierarchy_queries.has_path(entity, relationship.target()) {
                needs_cleanup = true;
                break;
            }
        }

        if needs_cleanup {
            // Remove the Hierarchical marker to trigger cleanup
            commands.entity(entity).remove::<Hierarchical>();
        }
    }
}
