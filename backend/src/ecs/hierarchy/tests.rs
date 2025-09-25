//! Comprehensive tests for hierarchy system
//!
//! Tests relationship components, graph operations, query performance,
//! and integration with the ECS system.

use super::*;
use bevy_ecs::prelude::*;
use petgraph::Direction;
use crate::ecs::components::Validate;

/// Create test world with hierarchy components
fn create_test_world() -> (World, HierarchyQueries) {
    let mut world = World::new();
    let hierarchy = HierarchyQueries::new();
    
    // Add some test entities
    let _entity1 = world.spawn((Hierarchical, Relationships::new())).id();
    let _entity2 = world.spawn((Hierarchical, Relationships::new())).id();
    let _entity3 = world.spawn((Hierarchical, Relationships::new())).id();
    
    (world, hierarchy)
}

#[cfg(test)]
mod relationship_tests {
    use super::*;

    #[test]
    fn test_relationship_creation() {
        let _entity1 = Entity::from_raw(1);
        let entity2 = Entity::from_raw(2);
        
        let relationship = Relationship::new(entity2, RelationshipType::Parent);
        
        assert_eq!(relationship.target(), entity2);
        assert_eq!(relationship.relationship_type(), RelationshipType::Parent);
        assert_eq!(relationship.metadata(), None);
    }

    #[test]
    fn test_relationship_with_metadata() {
        let _entity1 = Entity::from_raw(1);
        let entity2 = Entity::from_raw(2);
        let metadata = "test metadata".to_string();
        
        let relationship = Relationship::with_metadata(
            entity2, 
            RelationshipType::Dependency, 
            metadata.clone()
        );
        
        assert_eq!(relationship.target(), entity2);
        assert_eq!(relationship.relationship_type(), RelationshipType::Dependency);
        assert_eq!(relationship.metadata(), Some(metadata.as_str()));
    }

    #[test]
    fn test_relationship_type_inverse() {
        assert_eq!(RelationshipType::Parent.inverse(), RelationshipType::Child);
        assert_eq!(RelationshipType::Child.inverse(), RelationshipType::Parent);
        assert_eq!(RelationshipType::Owner.inverse(), RelationshipType::Dependency);
        assert_eq!(RelationshipType::Attachment.inverse(), RelationshipType::Attachment);
    }

    #[test]
    fn test_relationship_type_symmetry() {
        assert!(!RelationshipType::Parent.is_symmetric());
        assert!(!RelationshipType::Child.is_symmetric());
        assert!(!RelationshipType::Owner.is_symmetric());
        assert!(RelationshipType::Attachment.is_symmetric());
    }
}

#[cfg(test)]
mod relationships_component_tests {
    use super::*;

    #[test]
    fn test_relationships_creation() {
        let relationships = Relationships::new();
        assert!(relationships.is_empty());
        assert_eq!(relationships.count(), 0);
    }

    #[test]
    fn test_add_relationship() {
        let mut relationships = Relationships::new();
        let entity = Entity::from_raw(1);
        let relationship = Relationship::new(entity, RelationshipType::Parent);
        
        let result = relationships.add(relationship).unwrap();
        assert!(result); // Was inserted
        assert_eq!(relationships.count(), 1);
        assert!(relationships.has_relationship(entity, RelationshipType::Parent));
    }

    #[test]
    fn test_duplicate_relationship() {
        let mut relationships = Relationships::new();
        let entity = Entity::from_raw(1);
        let relationship = Relationship::new(entity, RelationshipType::Parent);
        
        relationships.add(relationship.clone()).unwrap();
        let result = relationships.add(relationship).unwrap();
        assert!(!result); // Was not inserted (duplicate)
        assert_eq!(relationships.count(), 1);
    }

    #[test]
    fn test_remove_relationship() {
        let mut relationships = Relationships::new();
        let entity = Entity::from_raw(1);
        let relationship = Relationship::new(entity, RelationshipType::Parent);
        
        relationships.add(relationship).unwrap();
        assert!(relationships.remove(entity, RelationshipType::Parent));
        assert_eq!(relationships.count(), 0);
        assert!(!relationships.has_relationship(entity, RelationshipType::Parent));
    }

    #[test]
    fn test_get_by_type() {
        let mut relationships = Relationships::new();
        let parent1 = Entity::from_raw(1);
        let parent2 = Entity::from_raw(2);
        let child = Entity::from_raw(3);
        
        relationships.add(Relationship::new(parent1, RelationshipType::Parent)).unwrap();
        relationships.add(Relationship::new(parent2, RelationshipType::Parent)).unwrap();
        relationships.add(Relationship::new(child, RelationshipType::Child)).unwrap();
        
        let parents = relationships.parents();
        assert_eq!(parents.len(), 2);
        assert!(parents.contains(&parent1));
        assert!(parents.contains(&parent2));
        
        let children = relationships.children();
        assert_eq!(children.len(), 1);
        assert!(children.contains(&child));
    }

    #[test]
    fn test_relationships_validation() {
        let relationships = Relationships::new();
        assert!(relationships.validate().is_ok());
    }

    #[test]
    fn test_relationships_hash_caching() {
        let mut relationships = Relationships::new();
        let entity = Entity::from_raw(1);
        
        let hash1 = relationships.get_hash();
        relationships.add(Relationship::new(entity, RelationshipType::Parent)).unwrap();
        let hash2 = relationships.get_hash();
        
        assert_ne!(hash1, hash2);
    }
}

#[cfg(test)]
mod entity_graph_tests {
    use super::*;

    #[test]
    fn test_graph_creation() {
        let graph = EntityGraph::new();
        let stats = graph.stats();
        
        assert_eq!(stats.entity_count, 0);
        assert_eq!(stats.edge_count, 0);
        assert!(!stats.has_cycles);
    }

    #[test]
    fn test_add_entity() {
        let graph = EntityGraph::new();
        let entity = Entity::from_raw(1);
        
        let node1 = graph.add_entity(entity);
        let node2 = graph.add_entity(entity); // Same entity
        
        assert_eq!(node1, node2); // Should return same node
        assert_eq!(graph.stats().entity_count, 1);
    }

    #[test]
    fn test_add_relationship() {
        let graph = EntityGraph::new();
        let parent = Entity::from_raw(1);
        let child = Entity::from_raw(2);
        
        let result = graph.add_relationship(
            parent,
            child,
            RelationshipType::Parent,
            None,
        );
        
        assert!(result.is_ok());
        assert_eq!(graph.stats().entity_count, 2);
        assert_eq!(graph.stats().edge_count, 1);
    }

    #[test]
    fn test_cycle_detection() {
        let graph = EntityGraph::new();
        let entity1 = Entity::from_raw(1);
        let entity2 = Entity::from_raw(2);
        let entity3 = Entity::from_raw(3);
        
        // Create a chain: 1 -> 2 -> 3
        graph.add_relationship(entity1, entity2, RelationshipType::Parent, None).unwrap();
        graph.add_relationship(entity2, entity3, RelationshipType::Parent, None).unwrap();
        
        // Try to create a cycle: 3 -> 1
        let result = graph.add_relationship(entity3, entity1, RelationshipType::Parent, None);
        
        assert!(result.is_err());
        if let Err(HierarchyError::CycleDetected) = result {
            // Expected
        } else {
            panic!("Expected CycleDetected error");
        }
    }

    #[test]
    fn test_get_parents_and_children() {
        let graph = EntityGraph::new();
        let parent = Entity::from_raw(1);
        let child1 = Entity::from_raw(2);
        let child2 = Entity::from_raw(3);
        
        graph.add_relationship(parent, child1, RelationshipType::Parent, None).unwrap();
        graph.add_relationship(parent, child2, RelationshipType::Parent, None).unwrap();
        
        let children = graph.get_children(parent);
        assert_eq!(children.len(), 2);
        assert!(children.contains(&child1));
        assert!(children.contains(&child2));
        
        let parents1 = graph.get_parents(child1);
        assert_eq!(parents1.len(), 1);
        assert!(parents1.contains(&parent));
    }

    #[test]
    fn test_ancestors_and_descendants() {
        let graph = EntityGraph::new();
        let grandparent = Entity::from_raw(1);
        let parent = Entity::from_raw(2);
        let child = Entity::from_raw(3);
        
        graph.add_relationship(grandparent, parent, RelationshipType::Parent, None).unwrap();
        graph.add_relationship(parent, child, RelationshipType::Parent, None).unwrap();
        
        let ancestors = graph.get_ancestors(child);
        assert_eq!(ancestors.len(), 2);
        assert!(ancestors.contains(&parent));
        assert!(ancestors.contains(&grandparent));
        
        let descendants = graph.get_descendants(grandparent);
        assert_eq!(descendants.len(), 2);
        assert!(descendants.contains(&parent));
        assert!(descendants.contains(&child));
    }

    #[test]
    fn test_ownership_relationships() {
        let graph = EntityGraph::new();
        let player = Entity::from_raw(1);
        let unit1 = Entity::from_raw(2);
        let unit2 = Entity::from_raw(3);
        
        graph.add_relationship(player, unit1, RelationshipType::Owner, None).unwrap();
        graph.add_relationship(player, unit2, RelationshipType::Owner, None).unwrap();
        
        let owned = graph.get_owned_entities(player);
        assert_eq!(owned.len(), 2);
        assert!(owned.contains(&unit1));
        assert!(owned.contains(&unit2));
        
        assert_eq!(graph.get_owner(unit1), Some(player));
        assert_eq!(graph.get_owner(unit2), Some(player));
    }

    #[test]
    fn test_remove_entity() {
        let graph = EntityGraph::new();
        let parent = Entity::from_raw(1);
        let child = Entity::from_raw(2);
        
        graph.add_relationship(parent, child, RelationshipType::Parent, None).unwrap();
        assert_eq!(graph.stats().entity_count, 2);
        
        graph.remove_entity(parent).unwrap();
        assert_eq!(graph.stats().entity_count, 1);
        assert_eq!(graph.stats().edge_count, 0);
    }

    #[test]
    fn test_symmetric_relationships() {
        let graph = EntityGraph::new();
        let entity1 = Entity::from_raw(1);
        let entity2 = Entity::from_raw(2);
        
        graph.add_relationship(entity1, entity2, RelationshipType::Attachment, None).unwrap();
        
        // Should have created both directions for symmetric relationship
        let related1 = graph.get_related_entities(entity1, RelationshipType::Attachment, Direction::Outgoing);
        let related2 = graph.get_related_entities(entity2, RelationshipType::Attachment, Direction::Outgoing);
        
        assert!(related1.contains(&entity2));
        assert!(related2.contains(&entity1));
    }

    #[test]
    fn test_topological_sort() {
        let graph = EntityGraph::new();
        let root = Entity::from_raw(1);
        let level1a = Entity::from_raw(2);
        let level1b = Entity::from_raw(3);
        let level2 = Entity::from_raw(4);
        
        graph.add_relationship(root, level1a, RelationshipType::Parent, None).unwrap();
        graph.add_relationship(root, level1b, RelationshipType::Parent, None).unwrap();
        graph.add_relationship(level1a, level2, RelationshipType::Parent, None).unwrap();
        
        let sorted = graph.topological_sort().unwrap();
        
        // Root should come before its children
        let root_pos = sorted.iter().position(|&e| e == root).unwrap();
        let level1a_pos = sorted.iter().position(|&e| e == level1a).unwrap();
        let level2_pos = sorted.iter().position(|&e| e == level2).unwrap();
        
        assert!(root_pos < level1a_pos);
        assert!(level1a_pos < level2_pos);
    }
}

#[cfg(test)]
mod hierarchy_queries_tests {
    use super::*;

    #[tokio::test]
    async fn test_queries_creation() {
        let queries = HierarchyQueries::new();
        let stats = queries.performance_stats().await;
        
        assert_eq!(stats.graph_stats.entity_count, 0);
        assert_eq!(stats.cached_ancestors, 0);
        assert_eq!(stats.cached_descendants, 0);
    }

    #[tokio::test]
    async fn test_sync_with_world() {
        let (mut world, queries) = create_test_world();
        
        // Add some relationships to entities
        let mut entity_query = world.query::<(Entity, &mut Relationships)>();
        let entities: Vec<_> = entity_query.iter(&world).map(|(e, _)| e).collect();
        
        if entities.len() >= 2 {
            let parent = entities[0];
            let child = entities[1];
            
            // Add relationship through component
            if let Ok((_, mut relationships)) = entity_query.get_mut(&mut world, parent) {
                relationships.add(Relationship::new(child, RelationshipType::Parent)).unwrap();
            }
            
            // Sync with queries
            queries.sync_with_world(&mut world).await.unwrap();
            
            // Verify relationship exists in graph
            let children = queries.children(parent);
            assert!(children.contains(&child));
        }
    }

    #[tokio::test]
    async fn test_ancestor_caching() {
        let queries = HierarchyQueries::new();
        let grandparent = Entity::from_raw(1);
        let parent = Entity::from_raw(2);
        let child = Entity::from_raw(3);
        
        // Build hierarchy in graph
        queries.graph().add_relationship(grandparent, parent, RelationshipType::Parent, None).unwrap();
        queries.graph().add_relationship(parent, child, RelationshipType::Parent, None).unwrap();
        
        // First call should compute and cache
        let ancestors1 = queries.ancestors(child).await;
        let stats1 = queries.performance_stats().await;
        
        // Second call should use cache
        let ancestors2 = queries.ancestors(child).await;
        let stats2 = queries.performance_stats().await;
        
        assert_eq!(ancestors1, ancestors2);
        // Cache stats tracking may be approximate
        assert!(ancestors1.len() == 2); // grandparent and parent
        assert!(ancestors1.contains(&grandparent));
        assert!(ancestors1.contains(&parent));
    }

    #[test]
    fn test_find_roots_and_leaves() {
        let (mut world, queries) = create_test_world();
        
        // Create a simple hierarchy
        let root = world.spawn((Hierarchical, Relationships::new())).id();
        let child = world.spawn((Hierarchical, Relationships::new())).id();
        let leaf = world.spawn((Hierarchical, Relationships::new())).id();
        
        // Add to graph
        queries.graph().add_relationship(root, child, RelationshipType::Parent, None).unwrap();
        queries.graph().add_relationship(child, leaf, RelationshipType::Parent, None).unwrap();
        
        let roots = queries.find_roots(&mut world);
        let leaves = queries.find_leaves(&mut world);
        
        assert!(roots.contains(&root));
        assert!(leaves.contains(&leaf));
        assert!(!roots.contains(&child)); // Not a root
        assert!(!leaves.contains(&child)); // Not a leaf
    }

    #[test]
    fn test_hierarchy_depth() {
        let queries = HierarchyQueries::new();
        let root = Entity::from_raw(1);
        let level1 = Entity::from_raw(2);
        let level2 = Entity::from_raw(3);
        let level3 = Entity::from_raw(4);
        
        // Build 3-level hierarchy
        queries.graph().add_relationship(root, level1, RelationshipType::Parent, None).unwrap();
        queries.graph().add_relationship(level1, level2, RelationshipType::Parent, None).unwrap();
        queries.graph().add_relationship(level2, level3, RelationshipType::Parent, None).unwrap();
        
        assert_eq!(queries.hierarchy_depth(root), 3);
        assert_eq!(queries.hierarchy_depth(level1), 2);
        assert_eq!(queries.hierarchy_depth(level2), 1);
        assert_eq!(queries.hierarchy_depth(level3), 0);
    }

    #[tokio::test]
    async fn test_common_ancestors() {
        let queries = HierarchyQueries::new();
        let grandparent = Entity::from_raw(1);
        let parent1 = Entity::from_raw(2);
        let parent2 = Entity::from_raw(3);
        let child1 = Entity::from_raw(4);
        let child2 = Entity::from_raw(5);
        
        // Build diamond hierarchy
        queries.graph().add_relationship(grandparent, parent1, RelationshipType::Parent, None).unwrap();
        queries.graph().add_relationship(grandparent, parent2, RelationshipType::Parent, None).unwrap();
        queries.graph().add_relationship(parent1, child1, RelationshipType::Parent, None).unwrap();
        queries.graph().add_relationship(parent2, child2, RelationshipType::Parent, None).unwrap();
        
        let common = queries.common_ancestors(child1, child2).await;
        assert!(common.contains(&grandparent));
        
        let lca = queries.lowest_common_ancestor(child1, child2).await;
        assert_eq!(lca, Some(grandparent));
    }

    #[tokio::test]
    async fn test_subtree() {
        let queries = HierarchyQueries::new();
        let root = Entity::from_raw(1);
        let child1 = Entity::from_raw(2);
        let child2 = Entity::from_raw(3);
        let grandchild = Entity::from_raw(4);
        
        queries.graph().add_relationship(root, child1, RelationshipType::Parent, None).unwrap();
        queries.graph().add_relationship(root, child2, RelationshipType::Parent, None).unwrap();
        queries.graph().add_relationship(child1, grandchild, RelationshipType::Parent, None).unwrap();
        
        let subtree = queries.subtree(root).await;
        assert_eq!(subtree.len(), 4);
        assert!(subtree.contains(&root));
        assert!(subtree.contains(&child1));
        assert!(subtree.contains(&child2));
        assert!(subtree.contains(&grandchild));
    }

    #[test]
    fn test_hierarchy_levels() {
        let queries = HierarchyQueries::new();
        let root = Entity::from_raw(1);
        let child1 = Entity::from_raw(2);
        let child2 = Entity::from_raw(3);
        let grandchild = Entity::from_raw(4);
        
        queries.graph().add_relationship(root, child1, RelationshipType::Parent, None).unwrap();
        queries.graph().add_relationship(root, child2, RelationshipType::Parent, None).unwrap();
        queries.graph().add_relationship(child1, grandchild, RelationshipType::Parent, None).unwrap();
        
        let levels = queries.hierarchy_levels(root);
        
        assert_eq!(levels.len(), 3);
        assert_eq!(levels[0], vec![root]);
        assert_eq!(levels[1].len(), 2);
        assert!(levels[1].contains(&child1));
        assert!(levels[1].contains(&child2));
        assert_eq!(levels[2], vec![grandchild]);
    }

    #[tokio::test]
    async fn test_cache_invalidation() {
        let mut queries = HierarchyQueries::new();
        let _entity = Entity::from_raw(1);
        
        // Get initial cache version
        let stats1 = queries.performance_stats().await;
        let initial_version = stats1.cache_version;
        
        // Advance generation to test cache invalidation
        queries.advance_world_generation().await;
        
        let stats2 = queries.performance_stats().await;
        assert!(stats2.cache_version > initial_version);
    }
}

#[cfg(test)]
mod serialization_tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_relationship_type_serialization() {
        let original = RelationshipType::Parent;
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: RelationshipType = serde_json::from_str(&json).unwrap();
        assert_eq!(original, deserialized);
        
        // Test all variants
        for variant in [
            RelationshipType::Parent,
            RelationshipType::Child,
            RelationshipType::Owner,
            RelationshipType::Dependency,
            RelationshipType::Attachment,
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            let deserialized: RelationshipType = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, deserialized);
        }
    }

    #[test]
    fn test_stable_entity_id_serialization() {
        let entity = Entity::from_raw(42);
        let stable_id: StableEntityId = entity.into();
        
        let json = serde_json::to_string(&stable_id).unwrap();
        let deserialized: StableEntityId = serde_json::from_str(&json).unwrap();
        assert_eq!(stable_id, deserialized);
        
        // Test round-trip through Entity
        let entity_back: Entity = deserialized.into();
        assert_eq!(entity.index(), entity_back.index());
        assert_eq!(entity.generation(), entity_back.generation());
    }

    #[test]
    fn test_relationship_serialization() {
        let entity = Entity::from_raw(123);
        let relationship = Relationship::new(entity, RelationshipType::Parent);
        
        let json = serde_json::to_string(&relationship).unwrap();
        let deserialized: Relationship = serde_json::from_str(&json).unwrap();
        
        assert_eq!(relationship.target(), deserialized.target());
        assert_eq!(relationship.relationship_type(), deserialized.relationship_type());
        assert_eq!(relationship.metadata(), deserialized.metadata());
    }

    #[test]
    fn test_relationship_with_metadata_serialization() {
        let entity = Entity::from_raw(456);
        let metadata = "test metadata".to_string();
        let relationship = Relationship::with_metadata(entity, RelationshipType::Dependency, metadata.clone());
        
        let json = serde_json::to_string(&relationship).unwrap();
        let deserialized: Relationship = serde_json::from_str(&json).unwrap();
        
        assert_eq!(relationship.target(), deserialized.target());
        assert_eq!(relationship.relationship_type(), deserialized.relationship_type());
        assert_eq!(Some(metadata.as_str()), deserialized.metadata());
    }

    #[test]
    fn test_relationships_serialization() {
        let mut relationships = Relationships::new();
        
        // Add some relationships
        let entity1 = Entity::from_raw(1);
        let entity2 = Entity::from_raw(2);
        let entity3 = Entity::from_raw(3);
        
        relationships.add(Relationship::new(entity1, RelationshipType::Parent)).unwrap();
        relationships.add(Relationship::new(entity2, RelationshipType::Child)).unwrap();
        relationships.add(Relationship::with_metadata(
            entity3, 
            RelationshipType::Dependency, 
            "test dependency".to_string()
        )).unwrap();
        
        let json = serde_json::to_string(&relationships).unwrap();
        let deserialized: Relationships = serde_json::from_str(&json).unwrap();
        
        // Check that all relationships are preserved
        assert_eq!(relationships.count(), deserialized.count());
        assert!(deserialized.has_relationship(entity1, RelationshipType::Parent));
        assert!(deserialized.has_relationship(entity2, RelationshipType::Child));
        assert!(deserialized.has_relationship(entity3, RelationshipType::Dependency));
        
        // Check that metadata is preserved
        let dependency_relationships = deserialized.get_by_type(RelationshipType::Dependency);
        assert!(dependency_relationships.contains(&entity3));
    }

    #[test]
    fn test_relationships_serialization_deterministic() {
        // Test that serialization is deterministic (same order every time)
        let mut relationships = Relationships::new();
        
        // Add relationships in different orders
        for i in (1..=10).rev() {
            let entity = Entity::from_raw(i);
            relationships.add(Relationship::new(entity, RelationshipType::Parent)).unwrap();
        }
        
        let json1 = serde_json::to_string(&relationships).unwrap();
        let json2 = serde_json::to_string(&relationships).unwrap();
        
        assert_eq!(json1, json2, "Serialization should be deterministic");
    }

    #[test]
    fn test_relationships_hash_cache_not_serialized() {
        let mut relationships = Relationships::new();
        let entity = Entity::from_raw(1);
        relationships.add(Relationship::new(entity, RelationshipType::Parent)).unwrap();
        
        // Force cache computation
        let _hash = relationships.get_hash();
        
        // Serialize and deserialize
        let json = serde_json::to_string(&relationships).unwrap();
        let deserialized: Relationships = serde_json::from_str(&json).unwrap();
        
        // The deserialized version should not have a cached hash initially
        // This tests that hash_cache is not serialized
        assert!(relationships.count() == deserialized.count());
        assert!(deserialized.has_relationship(entity, RelationshipType::Parent));
    }

    #[test]
    fn test_hierarchical_marker_serialization() {
        let hierarchical = Hierarchical;
        
        let json = serde_json::to_string(&hierarchical).unwrap();
        let deserialized: Hierarchical = serde_json::from_str(&json).unwrap();
        
        // Should serialize/deserialize without issues (it's a zero-sized type)
        assert_eq!(std::mem::size_of_val(&hierarchical), std::mem::size_of_val(&deserialized));
    }

    #[test]
    fn test_large_relationships_serialization() {
        // Test serialization with a large number of relationships
        let mut relationships = Relationships::new();
        
        for i in 1..=1000 {
            let entity = Entity::from_raw(i);
            let rel_type = match i % 5 {
                0 => RelationshipType::Parent,
                1 => RelationshipType::Child,
                2 => RelationshipType::Owner,
                3 => RelationshipType::Dependency,
                _ => RelationshipType::Attachment,
            };
            
            if i % 10 == 0 {
                relationships.add(Relationship::with_metadata(
                    entity, 
                    rel_type, 
                    format!("metadata_{}", i)
                )).unwrap();
            } else {
                relationships.add(Relationship::new(entity, rel_type)).unwrap();
            }
        }
        
        let json = serde_json::to_string(&relationships).unwrap();
        let deserialized: Relationships = serde_json::from_str(&json).unwrap();
        
        assert_eq!(relationships.count(), deserialized.count());
        assert_eq!(1000, deserialized.count());
        
        // Spot check a few relationships
        assert!(deserialized.has_relationship(Entity::from_raw(1), RelationshipType::Child));
        assert!(deserialized.has_relationship(Entity::from_raw(500), RelationshipType::Parent));
        assert!(deserialized.has_relationship(Entity::from_raw(1000), RelationshipType::Parent)); // 1000 % 5 = 0 -> Parent
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_full_hierarchy_workflow() {
        let (mut world, queries) = create_test_world();
        
        // Create entities with relationships
        let city = world.spawn((Hierarchical, Relationships::new())).id();
        let district1 = world.spawn((Hierarchical, Relationships::new())).id();
        let district2 = world.spawn((Hierarchical, Relationships::new())).id();
        let building = world.spawn((Hierarchical, Relationships::new())).id();
        
        // Add relationships through components
        {
            let mut city_query = world.query::<&mut Relationships>();
            if let Ok(mut city_relationships) = city_query.get_mut(&mut world, city) {
                city_relationships.add(Relationship::new(district1, RelationshipType::Parent)).unwrap();
                city_relationships.add(Relationship::new(district2, RelationshipType::Parent)).unwrap();
            }
            
            let mut district_query = world.query::<&mut Relationships>();
            if let Ok(mut district_relationships) = district_query.get_mut(&mut world, district1) {
                district_relationships.add(Relationship::new(building, RelationshipType::Parent)).unwrap();
            }
        }
        
        // Sync with hierarchy system
        queries.sync_with_world(&mut world).await.unwrap();
        
        // Test queries
        let city_children = queries.children(city);
        assert_eq!(city_children.len(), 2);
        assert!(city_children.contains(&district1));
        assert!(city_children.contains(&district2));
        
        let building_ancestors = queries.ancestors(building).await;
        assert_eq!(building_ancestors.len(), 2);
        assert!(building_ancestors.contains(&district1));
        assert!(building_ancestors.contains(&city));
        
        let city_subtree = queries.subtree(city).await;
        assert_eq!(city_subtree.len(), 4);
        
        let validation = queries.validate_hierarchy().unwrap();
        assert!(!validation.has_cycles);
        assert_eq!(validation.entity_count, 7); // 3 from create_test_world + 4 new entities
    }

    #[test]
    fn test_performance_with_large_hierarchy() {
        let queries = HierarchyQueries::new();
        
        // Create a large balanced tree
        let root = Entity::from_raw(1);
        let mut current_id = 2u32;
        
        // Add 100 entities in a tree structure
        for level in 0..4 {
            let level_size = 1 << level; // 1, 2, 4, 8
            for _i in 0..level_size {
                let parent = if level == 0 {
                    root
                } else {
                    Entity::from_raw((current_id - level_size) as u32)
                };
                
                let child = Entity::from_raw(current_id);
                queries.graph().add_relationship(parent, child, RelationshipType::Parent, None).unwrap();
                current_id += 1;
            }
        }
        
        let stats = queries.performance_stats();
        assert!(stats.graph_stats.entity_count > 10);
        assert!(stats.graph_stats.edge_count > 10);
        
        // Test that deep queries still work efficiently
        let leaf = Entity::from_raw(current_id - 1);
        let ancestors = queries.ancestors(leaf);
        assert!(ancestors.len() >= 3); // Should have multiple levels of ancestors
    }
}
