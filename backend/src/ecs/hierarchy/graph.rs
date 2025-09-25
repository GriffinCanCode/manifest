//! Entity relationship graph using petgraph with ECS integration
//!
//! Provides high-performance graph operations for entity hierarchies with
//! cycle detection, path finding, and batch relationship updates.

use bevy_ecs::prelude::*;
use petgraph::{
    Graph, Direction,
    graph::{NodeIndex, EdgeIndex},
    algo::{has_path_connecting, is_cyclic_directed, toposort},
    visit::EdgeRef,
};
use parking_lot::RwLock;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

use super::components::{Relationship, RelationshipType, Relationships};
use crate::core::hashing::{FastHashMap, EntityHasher, HashStrategies};

/// Errors that can occur in hierarchy operations
#[derive(Error, Debug, Clone)]
pub enum HierarchyError {
    #[error("Cycle detected in hierarchy")]
    CycleDetected,
    #[error("Entity not found in hierarchy: {0:?}")]
    EntityNotFound(Entity),
    #[error("Invalid relationship: {0}")]
    InvalidRelationship(String),
    #[error("Graph operation failed: {0}")]
    GraphError(String),
}

pub type HierarchyResult<T> = Result<T, HierarchyError>;

/// Edge data in the entity graph
#[derive(Debug, Clone)]
struct EdgeData {
    relationship_type: RelationshipType,
    metadata: Option<String>,
}

/// High-performance entity relationship graph
/// Thread-safe with optimized lookups and batch operations
#[derive(Debug)]
pub struct EntityGraph {
    /// Directed graph where nodes are entities and edges are relationships
    graph: Arc<RwLock<Graph<Entity, EdgeData>>>,
    /// Fast entity-to-node mapping using optimized hasher
    entity_to_node: Arc<RwLock<FastHashMap<Entity, NodeIndex>>>,
    /// Reverse mapping for node-to-entity lookups
    node_to_entity: Arc<RwLock<FastHashMap<NodeIndex, Entity>>>,
}

impl Default for EntityGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl EntityGraph {
    /// Create new empty entity graph
    pub fn new() -> Self {
        Self {
            graph: Arc::new(RwLock::new(Graph::new())),
            entity_to_node: Arc::new(RwLock::new(FastHashMap::default())),
            node_to_entity: Arc::new(RwLock::new(FastHashMap::default())),
        }
    }

    /// Add entity to graph if not already present
    pub fn add_entity(&self, entity: Entity) -> NodeIndex {
        let mut graph = self.graph.write();
        let mut entity_to_node = self.entity_to_node.write();
        let mut node_to_entity = self.node_to_entity.write();

        if let Some(&node) = entity_to_node.get(&entity) {
            return node;
        }

        let node = graph.add_node(entity);
        entity_to_node.insert(entity, node);
        node_to_entity.insert(node, entity);
        node
    }

    /// Remove entity and all its relationships from graph
    pub fn remove_entity(&self, entity: Entity) -> HierarchyResult<()> {
        let mut graph = self.graph.write();
        let mut entity_to_node = self.entity_to_node.write();
        let mut node_to_entity = self.node_to_entity.write();

        if let Some(node) = entity_to_node.remove(&entity) {
            node_to_entity.remove(&node);
            graph.remove_node(node);
        }

        Ok(())
    }

    /// Add relationship between entities with cycle detection
    pub fn add_relationship(
        &self,
        from: Entity,
        to: Entity,
        relationship_type: RelationshipType,
        metadata: Option<String>,
    ) -> HierarchyResult<EdgeIndex> {
        // Add entities if they don't exist
        let from_node = self.add_entity(from);
        let to_node = self.add_entity(to);

        let edge_data = EdgeData {
            relationship_type,
            metadata: metadata.clone(),
        };

        // Check for cycles before adding parent-child relationships
        if matches!(relationship_type, RelationshipType::Parent | RelationshipType::Child) {
            let graph = self.graph.read();
            if has_path_connecting(&*graph, to_node, from_node, None) {
                return Err(HierarchyError::CycleDetected);
            }
        }

        // Add the edge
        let mut graph = self.graph.write();
        let edge = graph.add_edge(from_node, to_node, edge_data);
        
        // For symmetric relationships, add the reverse edge
        if relationship_type.is_symmetric() {
            let reverse_data = EdgeData {
                relationship_type,
                metadata,
            };
            graph.add_edge(to_node, from_node, reverse_data);
        }

        Ok(edge)
    }

    /// Remove specific relationship between entities
    pub fn remove_relationship(
        &self,
        from: Entity,
        to: Entity,
        relationship_type: RelationshipType,
    ) -> HierarchyResult<bool> {
        let entity_to_node = self.entity_to_node.read();
        let from_node = entity_to_node.get(&from)
            .ok_or(HierarchyError::EntityNotFound(from))?;
        let to_node = entity_to_node.get(&to)
            .ok_or(HierarchyError::EntityNotFound(to))?;

        let mut graph = self.graph.write();
        let mut removed = false;

        // Find and remove the edge
        let edges_to_remove: Vec<EdgeIndex> = graph
            .edges_connecting(*from_node, *to_node)
            .filter(|edge_ref| edge_ref.weight().relationship_type == relationship_type)
            .map(|edge_ref| edge_ref.id())
            .collect();

        for edge_id in edges_to_remove {
            graph.remove_edge(edge_id);
            removed = true;
        }

        // Remove symmetric relationships
        if relationship_type.is_symmetric() {
            let reverse_edges: Vec<EdgeIndex> = graph
                .edges_connecting(*to_node, *from_node)
                .filter(|edge_ref| edge_ref.weight().relationship_type == relationship_type)
                .map(|edge_ref| edge_ref.id())
                .collect();

            for edge_id in reverse_edges {
                graph.remove_edge(edge_id);
            }
        }

        Ok(removed)
    }

    /// Get all entities related to the given entity by relationship type
    pub fn get_related_entities(
        &self,
        entity: Entity,
        relationship_type: RelationshipType,
        direction: Direction,
    ) -> Vec<Entity> {
        let entity_to_node = self.entity_to_node.read();
        let graph = self.graph.read();

        if let Some(&node) = entity_to_node.get(&entity) {
            graph
                .edges_directed(node, direction)
                .filter(|edge_ref| edge_ref.weight().relationship_type == relationship_type)
                .map(|edge_ref| {
                    let target_node = match direction {
                        Direction::Outgoing => edge_ref.target(),
                        Direction::Incoming => edge_ref.source(),
                    };
                    graph[target_node]
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get all parent entities (incoming Parent relationships)
    pub fn get_parents(&self, entity: Entity) -> Vec<Entity> {
        self.get_related_entities(entity, RelationshipType::Parent, Direction::Incoming)
    }

    /// Get all child entities (outgoing Parent relationships)
    pub fn get_children(&self, entity: Entity) -> Vec<Entity> {
        self.get_related_entities(entity, RelationshipType::Parent, Direction::Outgoing)
    }

    /// Get all entities this entity owns (outgoing Owner relationships)
    pub fn get_owned_entities(&self, entity: Entity) -> Vec<Entity> {
        self.get_related_entities(entity, RelationshipType::Owner, Direction::Outgoing)
    }

    /// Get the owner of this entity (incoming Owner relationship)
    pub fn get_owner(&self, entity: Entity) -> Option<Entity> {
        self.get_related_entities(entity, RelationshipType::Owner, Direction::Incoming)
            .into_iter()
            .next()
    }

    /// Get all ancestors of an entity (recursive parent traversal)
    pub fn get_ancestors(&self, entity: Entity) -> Vec<Entity> {
        let mut ancestors = Vec::new();
        let mut current_parents = self.get_parents(entity);
        
        while !current_parents.is_empty() {
            ancestors.extend(&current_parents);
            current_parents = current_parents
                .iter()
                .flat_map(|&parent| self.get_parents(parent))
                .collect();
        }

        ancestors
    }

    /// Get all descendants of an entity (recursive child traversal)
    pub fn get_descendants(&self, entity: Entity) -> Vec<Entity> {
        let mut descendants = Vec::new();
        let mut current_children = self.get_children(entity);
        
        while !current_children.is_empty() {
            descendants.extend(&current_children);
            current_children = current_children
                .iter()
                .flat_map(|&child| self.get_children(child))
                .collect();
        }

        descendants
    }

    /// Check if there's a path between two entities
    pub fn has_path(&self, from: Entity, to: Entity) -> bool {
        let entity_to_node = self.entity_to_node.read();
        let graph = self.graph.read();

        if let (Some(&from_node), Some(&to_node)) = 
            (entity_to_node.get(&from), entity_to_node.get(&to)) {
            has_path_connecting(&*graph, from_node, to_node, None)
        } else {
            false
        }
    }

    /// Check if the graph has cycles (useful for validation)
    pub fn has_cycles(&self) -> bool {
        let graph = self.graph.read();
        is_cyclic_directed(&*graph)
    }

    /// Get topological sort of entities (useful for dependency resolution)
    pub fn topological_sort(&self) -> Result<Vec<Entity>, HierarchyError> {
        let graph = self.graph.read();
        let node_to_entity = self.node_to_entity.read();

        match toposort(&*graph, None) {
            Ok(sorted_nodes) => {
                let entities = sorted_nodes
                    .into_iter()
                    .filter_map(|node| node_to_entity.get(&node).copied())
                    .collect();
                Ok(entities)
            }
            Err(_) => Err(HierarchyError::CycleDetected),
        }
    }

    /// Batch update relationships for multiple entities (parallelized)
    pub fn batch_update_relationships<I>(&self, updates: I) -> HierarchyResult<()>
    where
        I: IntoIterator<Item = (Entity, Relationships)> + Send,
        I::IntoIter: Send,
    {
        // Collect updates to avoid holding locks during iteration
        let updates: Vec<_> = updates.into_iter().collect();
        
        // Process in parallel batches for large updates
        if updates.len() > 100 {
            updates
                .par_chunks(100)
                .try_for_each(|chunk| {
                    for (entity, relationships) in chunk {
                        self.sync_entity_relationships(*entity, relationships)?;
                    }
                    Ok::<(), HierarchyError>(())
                })?;
        } else {
            for (entity, relationships) in updates {
                self.sync_entity_relationships(entity, &relationships)?;
            }
        }

        Ok(())
    }

    /// Synchronize entity's relationships with its Relationships component
    fn sync_entity_relationships(
        &self,
        entity: Entity,
        relationships: &Relationships,
    ) -> HierarchyResult<()> {
        // Ensure entity exists in graph
        self.add_entity(entity);
        
        // Remove only the outgoing relationships from this entity
        self.remove_outgoing_relationships(entity)?;

        // Add all relationships from the component
        for relationship in relationships.iter() {
            self.add_relationship(
                entity,
                relationship.target(),
                relationship.relationship_type(),
                relationship.metadata().map(|s| s.to_string()),
            )?;
        }

        Ok(())
    }

    /// Remove all outgoing relationships from an entity without removing the entity itself
    fn remove_outgoing_relationships(&self, entity: Entity) -> HierarchyResult<()> {
        let entity_to_node = self.entity_to_node.read();
        let node = entity_to_node.get(&entity);
        
        if let Some(&from_node) = node {
            let mut graph = self.graph.write();
            
            // Collect outgoing edges to remove
            let edges_to_remove: Vec<_> = graph.edges_directed(from_node, petgraph::Direction::Outgoing)
                .map(|edge_ref| edge_ref.id())
                .collect();
                
            // Remove the edges
            for edge_id in edges_to_remove {
                graph.remove_edge(edge_id);
            }
        }
        
        Ok(())
    }

    /// Get graph statistics for monitoring and debugging
    pub fn stats(&self) -> GraphStats {
        let graph = self.graph.read();
        let entity_count = self.entity_to_node.read().len();
        
        GraphStats {
            entity_count,
            edge_count: graph.edge_count(),
            has_cycles: is_cyclic_directed(&*graph),
        }
    }
}

/// Statistics about the entity graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStats {
    pub entity_count: usize,
    pub edge_count: usize,
    pub has_cycles: bool,
}

// Make EntityGraph thread-safe
unsafe impl Send for EntityGraph {}
unsafe impl Sync for EntityGraph {}
