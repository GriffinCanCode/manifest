//! High-performance hierarchy queries with rayon parallelization
//!
//! Provides optimized queries for entity relationships, ancestors, descendants,
//! and complex hierarchy traversals with parallel execution.

use bevy_ecs::prelude::*;
use rayon::prelude::*;
use petgraph::Direction;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::{
    components::{Relationships, RelationshipType, Hierarchical},
    graph::{EntityGraph, HierarchyResult},
};
use crate::core::{
    hashing::{FastHashMap, FastHashSet},
    caching::{GameCache, GameCacheBuilder, CacheKey, CachePriority, SubsystemStats, global_cache_events, events::CacheInvalidationEvent}
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
        let cache_key = CacheKey::Custom(format!("hierarchy:ancestors:{}:{}", 
            entity.index(), self.world_generation));
        
        // Check cache first
        if let Ok(Some(ancestors)) = self.cache.get::<Vec<Entity>>(&cache_key).await {
            return ancestors;
        }

        // Cache miss - compute ancestors
        let ancestors = self.graph.get_ancestors(entity);
        
        // Cache the result with normal priority (hierarchies are moderately important)
        let _ = self.cache.set(cache_key, ancestors.clone(), CachePriority::Normal).await;

        ancestors
    }

    /// Find all descendant entities (recursive children) with caching
    pub async fn descendants(&self, entity: Entity) -> Vec<Entity> {
        let cache_key = CacheKey::Custom(format!("hierarchy:descendants:{}:{}", 
            entity.index(), self.world_generation));
        
        // Check cache first
        if let Ok(Some(descendants)) = self.cache.get::<Vec<Entity>>(&cache_key).await {
            return descendants;
        }

        // Cache miss - compute descendants
        let descendants = self.graph.get_descendants(entity);
        
        // Cache the result with normal priority
        let _ = self.cache.set(cache_key, descendants.clone(), CachePriority::Normal).await;

        descendants
    }

    /// Check if one entity is an ancestor of another
    pub async fn is_ancestor(&self, ancestor: Entity, descendant: Entity) -> bool {
        let ancestors = self.ancestors(descendant).await;
        ancestors.contains(&ancestor)
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

    /// Find common ancestors between two entities with caching
    pub async fn common_ancestors(&self, entity1: Entity, entity2: Entity) -> Vec<Entity> {
        let cache_key = CacheKey::Custom(format!("hierarchy:common_ancestors:{}:{}:{}", 
            entity1.index(), entity2.index(), self.world_generation));
        
        // Check cache first
        if let Ok(Some(common)) = self.cache.get::<Vec<Entity>>(&cache_key).await {
            return common;
        }

        // Cache miss - compute common ancestors
        let ancestors1: FastHashSet<_> = self.ancestors(entity1).await.into_iter().collect();
        let ancestors2 = self.ancestors(entity2).await;

        let common: Vec<Entity> = ancestors2
            .into_iter()
            .filter(|ancestor| ancestors1.contains(&ancestor))
            .collect();

        // Cache the result with normal priority
        let _ = self.cache.set(cache_key, common.clone(), CachePriority::Normal).await;
        common
    }

    /// Find the lowest common ancestor of two entities with caching
    pub async fn lowest_common_ancestor(&self, entity1: Entity, entity2: Entity) -> Option<Entity> {
        let cache_key = CacheKey::Custom(format!("hierarchy:lca:{}:{}:{}", 
            entity1.index(), entity2.index(), self.world_generation));
        
        // Check cache first
        if let Ok(Some(lca)) = self.cache.get::<Option<Entity>>(&cache_key).await {
            return lca;
        }

        // Cache miss - compute lowest common ancestor
        let ancestors1: FastHashSet<_> = self.ancestors(entity1).await.into_iter().collect();
        
        // Walk up from entity2 until we find a common ancestor
        let lca = self.ancestors(entity2).await
            .into_iter()
            .find(|ancestor| ancestors1.contains(ancestor));

        // Cache the result with normal priority
        let _ = self.cache.set(cache_key, lca, CachePriority::Normal).await;
        lca
    }

    /// Find all entities in a subtree rooted at the given entity with caching
    pub async fn subtree(&self, root: Entity) -> Vec<Entity> {
        let cache_key = CacheKey::Custom(format!("hierarchy:subtree:{}:{}", 
            root.index(), self.world_generation));
        
        // Check cache first
        if let Ok(Some(subtree)) = self.cache.get::<Vec<Entity>>(&cache_key).await {
            return subtree;
        }

        // Cache miss - compute subtree
        let mut subtree = vec![root];
        subtree.extend(self.descendants(root).await);
        
        // Cache the result with normal priority
        let _ = self.cache.set(cache_key, subtree.clone(), CachePriority::Normal).await;
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
    pub fn validate_hierarchy(&self, world: &mut World) -> HierarchyResult<HierarchyValidation> {
        let stats = self.graph.stats();
        
        Ok(HierarchyValidation {
            has_cycles: stats.has_cycles,
            entity_count: stats.entity_count,
            relationship_count: stats.edge_count,
            orphaned_entities: self.count_orphaned_entities(world),
        })
    }

    /// Count entities that exist in components but not in graph
    fn count_orphaned_entities(&self, world: &mut World) -> usize {
        let mut orphaned_count = 0;
        
        // Get all entities that have hierarchical components
        let hierarchical_entities: FastHashSet<Entity> = world
            .query_filtered::<Entity, With<Hierarchical>>()
            .iter(world)
            .collect();
        
        let relationships_entities: FastHashSet<Entity> = world
            .query_filtered::<Entity, With<Relationships>>()
            .iter(world)
            .collect();
        
        // Combine both sets of entities that should be in the graph
        let mut component_entities = hierarchical_entities;
        component_entities.extend(relationships_entities);
        
        // Count entities that exist in components but not tracked by the graph
        for entity in component_entities {
            // Check if the entity exists in the graph's entity tracking
            if !self.graph.contains_entity(entity) {
                orphaned_count += 1;
            }
        }
        
        orphaned_count
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
        // Clear all hierarchy-related cache entries
        self.cache.clear().await;
        
        // Broadcast cache invalidation event for hierarchy subsystem
        global_cache_events().broadcast(
            CacheInvalidationEvent::WorldGeneration(self.world_generation)
        ).await;
    }

    /// Synchronous cache invalidation for ECS systems
    pub fn invalidate_cache_sync(&self) {
        // Use tokio::task::block_in_place to handle async operations in sync context
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.block_on(async {
                self.invalidate_cache().await;
            });
        }
        // If no async runtime available, skip cache invalidation but log warning
        else {
            tracing::warn!("Cannot invalidate hierarchy cache: no async runtime available");
        }
    }

    /// Advance world generation and invalidate caches
    pub async fn advance_world_generation(&mut self) {
        self.world_generation += 1;
        self.invalidate_cache().await;
    }

    /// Invalidate cache entries for a specific entity
    pub async fn invalidate_entity(&self, entity: Entity) {
        // Remove specific cache entries related to this entity
        let entity_index = entity.index();
        let generation = self.world_generation;
        
        // Remove entity-specific caches
        let ancestors_key = CacheKey::Custom(format!("hierarchy:ancestors:{}:{}", entity_index, generation));
        let descendants_key = CacheKey::Custom(format!("hierarchy:descendants:{}:{}", entity_index, generation));
        let subtree_key = CacheKey::Custom(format!("hierarchy:subtree:{}:{}", entity_index, generation));
        
        self.cache.remove(&ancestors_key).await;
        self.cache.remove(&descendants_key).await;
        self.cache.remove(&subtree_key).await;
        
        // Also need to invalidate any common ancestor or LCA caches involving this entity
        // This is more expensive but necessary for correctness
        // For now, we'll do a broader invalidation for caches involving this entity
        
        // Broadcast entity-specific invalidation event
        global_cache_events().broadcast(
            CacheInvalidationEvent::EntityModified { 
                entity, 
                archetype_changed: false, 
                position_changed: None 
            }
        ).await;
    }

    /// Get access to the underlying EntityGraph for testing
    #[cfg(test)]
    pub fn graph(&self) -> &Arc<EntityGraph> {
        &self.graph
    }

    /// Update hierarchy with new relationship data (async interface)
    pub async fn update_relationships(&self, updates: Vec<(Entity, Relationships)>) -> HierarchyResult<()> {
        // Extract entities that will be affected for targeted cache invalidation
        let affected_entities: Vec<Entity> = updates.iter().map(|(entity, _)| *entity).collect();
        
        // Update the graph
        self.graph.batch_update_relationships(updates)?;
        
        // Invalidate cache entries for affected entities
        for entity in affected_entities {
            self.invalidate_entity(entity).await;
        }
        
        Ok(())
    }

    /// Update hierarchy with new relationship data (sync interface for non-async contexts)
    pub fn update_relationships_sync(&self, updates: Vec<(Entity, Relationships)>) -> HierarchyResult<()> {
        // Extract entities that will be affected for targeted cache invalidation
        let affected_entities: Vec<Entity> = updates.iter().map(|(entity, _)| *entity).collect();
        
        // Update the graph
        self.graph.batch_update_relationships(updates)?;
        
        // Invalidate cache entries for affected entities using runtime
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            for entity in affected_entities {
                handle.block_on(async {
                    self.invalidate_entity(entity).await;
                });
            }
        } else {
            tracing::warn!("Cannot invalidate hierarchy cache: no async runtime available");
        }
        
        Ok(())
    }

    /// Get performance statistics for the hierarchy system
    pub async fn performance_stats(&self) -> HierarchyPerformanceStats {
        let cache_stats = self.cache.stats().await;
        let graph_stats = self.graph.stats();

        // Estimate hierarchy-specific cache entries
        let total_entries = cache_stats.cache_count;
        let estimated_ancestors = total_entries / 4; // Rough estimate
        let estimated_descendants = total_entries / 4; // Rough estimate

        HierarchyPerformanceStats {
            graph_stats,
            cached_ancestors: estimated_ancestors,
            cached_descendants: estimated_descendants,
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

        // Report metrics to the global metrics system
        global_cache_events().register_subsystem_metrics("hierarchy", subsystem_stats).await;
        
        // Log hierarchy-specific performance info
        tracing::debug!(
            target: "hierarchy::performance",
            graph_entities = graph_stats.entity_count,
            graph_edges = graph_stats.edge_count,
            cache_hits = cache_stats.total_hits,
            cache_misses = cache_stats.total_misses,
            cache_entries = cache_stats.cache_count,
            "Hierarchy system performance stats"
        );
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
    
    // Invalidate cache after sync (sync version for ECS system)
    hierarchy_queries.invalidate_cache_sync();
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
