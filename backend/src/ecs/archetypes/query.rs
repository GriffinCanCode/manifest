//! Integration layer between archetypes and existing query system
//!
//! This module provides integration points that allow the existing
//! OptimalSpatialIndex system to leverage archetype organization.

use super::types::ArchetypeId;
use super::manager::ArchetypeManager;
use bevy_ecs::prelude::*;
use std::any::TypeId;
use std::collections::{HashMap, HashSet};

/// Trait to extend existing spatial system with archetype awareness  
/// This would be implemented on OptimalSpatialIndex
pub trait ArchetypeQueryIntegration {
    /// Limit queries to specific archetypes for better performance
    fn with_archetypes(&self, archetype_ids: Vec<ArchetypeId>) -> Self;
    
    /// Get archetype distribution of query results
    fn get_archetype_distribution(&self) -> HashMap<ArchetypeId, usize>;
    
    /// Pre-filter entities by archetype before spatial queries
    fn archetype_prefilter<T: Bundle>(&self) -> Vec<Entity>;
}

/// Helper to bridge archetype manager with query system
#[derive(Debug)]
pub struct QueryArchetypeBridge {
    archetype_manager: ArchetypeManager,
}

impl QueryArchetypeBridge {
    /// Create new bridge
    pub fn new(archetype_manager: ArchetypeManager) -> Self {
        Self { archetype_manager }
    }
    
    /// Get entities that have specific component layout
    /// This can pre-filter entities before expensive spatial queries
    pub fn entities_with_components(&self, component_types: HashSet<TypeId>) -> Vec<Entity> {
        let archetype_ids = self.archetype_manager
            .find_archetypes_with_components(&component_types);
        
        let mut entities = Vec::new();
        for archetype_id in archetype_ids {
            entities.extend(self.archetype_manager.get_archetype_entities(archetype_id));
        }
        
        entities
    }
    
    /// Get entities in same archetype as a reference entity
    /// Useful for batch operations on entities with same component layout  
    pub fn entities_like(&self, reference_entity: Entity) -> Vec<Entity> {
        self.archetype_manager.get_entities_with_same_layout(reference_entity)
    }
    
    /// Get archetype manager reference
    pub fn archetype_manager(&self) -> &ArchetypeManager {
        &self.archetype_manager
    }
}

/// Example of how OptimalSpatialIndex could be extended with archetypes
/// (This would be implemented in spatial/mod.rs if needed)
/*
impl ArchetypeQueryIntegration for crate::ecs::spatial::OptimalSpatialIndex {
    fn with_archetypes(&self, archetype_ids: Vec<ArchetypeId>) -> Self {
        // Create a filtered version of the spatial index that only
        // operates on entities from specific archetypes
        todo!()
    }
    
    fn get_archetype_distribution(&self) -> HashMap<ArchetypeId, usize> {
        // Analyze spatial query results to see which archetypes they come from
        todo!()
    }
    
    fn archetype_prefilter<T: Bundle>(&self) -> Vec<Entity> {
        // Use archetype system to quickly find all entities with bundle T
        // before applying spatial filters
        todo!()
    }
}
*/

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_creation() {
        let manager = ArchetypeManager::new();
        let bridge = QueryArchetypeBridge::new(manager);
        
        let empty_components = HashSet::new();
        let entities = bridge.entities_with_components(empty_components);
        assert!(entities.is_empty());
    }
    
    #[test] 
    fn test_entities_like() {
        let manager = ArchetypeManager::new();
        let bridge = QueryArchetypeBridge::new(manager);
        
        let entity = Entity::from_raw(123);
        let like_entities = bridge.entities_like(entity);
        assert!(like_entities.is_empty()); // No registered entities
    }
}
