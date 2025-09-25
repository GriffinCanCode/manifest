//! Hierarchy operations and queries
//!
//! Contains methods for managing hierarchical entities and relationships.

use bevy_ecs::prelude::*;
use std::any::TypeId;
use tokio::runtime::Handle;

use crate::core::caching::{CacheKey, QueryCacheKey, QueryResult, QueryType, CachePriority};
use crate::ecs::hierarchy::{HierarchyQueries, Hierarchical, Relationships};

use super::core::GameWorld;

impl GameWorld {
    /// Find all hierarchical entities with relationships
    pub fn get_hierarchical_entities(&mut self) -> Vec<Entity> {
        // Try cache first if we have a tokio runtime
        if let Ok(handle) = Handle::try_current() {
            let cache_key = QueryCacheKey {
                component_signature: crate::core::hashing::HashStrategies::hash_type_signature(&[TypeId::of::<Hierarchical>()]),  
                filter_hash: None,
                player_id: None,
                world_generation: self.world_generation(),
                query_type: QueryType::ComponentQuery,
            };
            
            // Check cache
            if let Ok(Some(QueryResult::Entities(entities))) = handle.block_on(
                self.query_cache().get::<QueryResult>(&CacheKey::Query(cache_key.clone()))
            ) {
                return entities;
            }
            
            // Cache miss - perform query
            let mut query = self.world().query_filtered::<Entity, With<Hierarchical>>();
            let entities: Vec<Entity> = query.iter(self.world()).collect();
            
            // Cache result asynchronously
            let cache = self.query_cache().clone();
            let cache_key_clone = cache_key.clone();
            let entities_clone = entities.clone();
            handle.spawn(async move {
                let result = QueryResult::Entities(entities_clone);
                let _ = cache.set(CacheKey::Query(cache_key_clone), result, CachePriority::Normal).await;
            });
            
            entities
        } else {
            // No tokio runtime - fallback to uncached query
            let mut query = self.world().query_filtered::<Entity, With<Hierarchical>>();
            query.iter(self.world()).collect()
        }
    }

    /// Get hierarchy queries resource for advanced relationship operations
    pub fn hierarchy_queries(&self) -> Option<&HierarchyQueries> {
        self.world().get_resource::<HierarchyQueries>()
    }

    /// Find all entities with relationships
    pub fn entities_with_relationships(&mut self) -> Vec<(Entity, Relationships)> {
        // Try cache first if we have a tokio runtime
        if let Ok(handle) = Handle::try_current() {
            let cache_key = QueryCacheKey {
                component_signature: crate::core::hashing::HashStrategies::hash_type_signature(&[TypeId::of::<Relationships>()]),  
                filter_hash: None,
                player_id: None,
                world_generation: self.world_generation(),
                query_type: QueryType::ComponentQuery,
            };
            
            // Check cache - we'll store as serialized component data
            if let Ok(Some(QueryResult::EntitiesWithData { entities, component_data })) = handle.block_on(
                self.query_cache().get::<QueryResult>(&CacheKey::Query(cache_key.clone()))
            ) {
                // Deserialize relationships from component data
                let mut result = Vec::new();
                for (i, entity) in entities.iter().enumerate() {
                    if let Some(data) = component_data.get(i) {
                        if let crate::core::caching::ComponentData::Serialized { data, .. } = data {
                            if let Ok(relationships) = bincode::deserialize::<Relationships>(data) {
                                result.push((*entity, relationships));
                            }
                        }
                    }
                }
                return result;
            }
            
            // Cache miss - perform query
            let mut query = self.world().query::<(Entity, &Relationships)>();
            let results: Vec<(Entity, Relationships)> = query.iter(self.world())
                .map(|(entity, rel)| (entity, rel.clone()))
                .collect();
            
            // Cache result asynchronously with serialized component data
            let cache = self.query_cache().clone();
            let cache_key_clone = cache_key.clone();
            let results_clone = results.clone();
            handle.spawn(async move {
                let entities: Vec<Entity> = results_clone.iter().map(|(e, _)| *e).collect();
                let component_data: Vec<crate::core::caching::ComponentData> = results_clone.iter()
                    .map(|(_, rel)| {
                        let serialized = bincode::serialize(rel).unwrap_or_default();
                        crate::core::caching::ComponentData::Serialized {
                            data: serialized,
                            type_id: crate::core::hashing::TypeIdHasher::hash(TypeId::of::<Relationships>()),
                        }
                    })
                    .collect();
                
                let result = QueryResult::EntitiesWithData { entities, component_data };
                let _ = cache.set(CacheKey::Query(cache_key_clone), result, CachePriority::Normal).await;
            });
            
            results
        } else {
            // No tokio runtime - fallback to uncached query
            let mut query = self.world().query::<(Entity, &Relationships)>();
            query.iter(self.world())
                .map(|(entity, rel)| (entity, rel.clone()))
                .collect()
        }
    }

    /// Get all parent entities of a given entity
    pub fn get_entity_parents(&self, entity: Entity) -> Vec<Entity> {
        if let Some(hierarchy_queries) = self.hierarchy_queries() {
            hierarchy_queries.parents(entity)
        } else {
            Vec::new()
        }
    }

    /// Get all child entities of a given entity
    pub fn get_entity_children(&self, entity: Entity) -> Vec<Entity> {
        if let Some(hierarchy_queries) = self.hierarchy_queries() {
            hierarchy_queries.children(entity)
        } else {
            Vec::new()
        }
    }

    /// Get all ancestor entities (recursive parents)
    pub async fn get_entity_ancestors(&self, entity: Entity) -> Vec<Entity> {
        if let Some(hierarchy_queries) = self.hierarchy_queries() {
            hierarchy_queries.ancestors(entity).await
        } else {
            Vec::new()
        }
    }

    /// Get all descendant entities (recursive children)
    pub async fn get_entity_descendants(&self, entity: Entity) -> Vec<Entity> {
        if let Some(hierarchy_queries) = self.hierarchy_queries() {
            hierarchy_queries.descendants(entity).await
        } else {
            Vec::new()
        }
    }

    /// Check if one entity is an ancestor of another
    pub async fn is_ancestor(&self, ancestor: Entity, descendant: Entity) -> bool {
        if let Some(hierarchy_queries) = self.hierarchy_queries() {
            hierarchy_queries.is_ancestor(ancestor, descendant).await
        } else {
            false
        }
    }

    /// Get the root entities (entities with no parents) in the hierarchy
    pub fn get_root_entities(&mut self) -> Vec<Entity> {
        let mut query = self.world().query::<(Entity, &Relationships)>();
        query.iter(self.world())
            .filter(|(_, relationships)| relationships.parents().is_empty())
            .map(|(entity, _)| entity)
            .collect()
    }

    /// Get leaf entities (entities with no children) in the hierarchy
    pub fn get_leaf_entities(&mut self) -> Vec<Entity> {
        let mut query = self.world().query::<(Entity, &Relationships)>();
        query.iter(self.world())
            .filter(|(_, relationships)| relationships.children().is_empty())
            .map(|(entity, _)| entity)
            .collect()
    }
}
