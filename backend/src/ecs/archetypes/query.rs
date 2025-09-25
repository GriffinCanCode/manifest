//! Integration layer between archetypes and existing query system
//!
//! This module provides integration points that allow the existing
//! OptimalSpatialIndex system to leverage archetype organization.

use super::types::ArchetypeId;
use super::manager::{ArchetypeManager, BundleComponentExtractor};
use bevy_ecs::prelude::*;
use std::any::TypeId;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use parking_lot::RwLock;

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

/// Helper to bridge archetype manager with spatial query system
#[derive(Debug)]
pub struct QueryArchetypeBridge {
    archetype_manager: Arc<RwLock<ArchetypeManager>>,
}

/// Bridge that combines archetype organization with spatial indexing
#[derive(Debug, bevy_ecs::system::Resource)]
pub struct ArchetypeSpatialBridge {
    archetype_manager: Arc<RwLock<ArchetypeManager>>,
    spatial_index: Arc<crate::ecs::spatial::OptimalSpatialIndex>,
}

impl QueryArchetypeBridge {
    /// Create new bridge
    pub fn new(archetype_manager: ArchetypeManager) -> Self {
        Self { 
            archetype_manager: Arc::new(RwLock::new(archetype_manager))
        }
    }
    
    /// Create bridge from shared archetype manager
    pub fn from_shared(archetype_manager: Arc<RwLock<ArchetypeManager>>) -> Self {
        Self { archetype_manager }
    }
    
    /// Get entities that have specific component layout
    /// This can pre-filter entities before expensive spatial queries
    pub fn entities_with_components(&self, component_types: HashSet<TypeId>) -> Vec<Entity> {
        let manager = self.archetype_manager.read();
        let archetype_ids = manager.find_archetypes_with_components(&component_types);
        
        let mut entities = Vec::new();
        for archetype_id in archetype_ids {
            entities.extend(manager.get_archetype_entities(archetype_id));
        }
        
        entities
    }
    
    /// Get entities in same archetype as a reference entity
    /// Useful for batch operations on entities with same component layout  
    pub fn entities_like(&self, reference_entity: Entity) -> Vec<Entity> {
        let manager = self.archetype_manager.read();
        manager.get_entities_with_same_layout(reference_entity)
    }
    
    /// Get archetype manager reference
    pub fn archetype_manager(&self) -> Arc<RwLock<ArchetypeManager>> {
        Arc::clone(&self.archetype_manager)
    }
}

impl ArchetypeSpatialBridge {
    /// Create new bridge combining archetype management and spatial indexing
    pub fn new(archetype_manager: ArchetypeManager, spatial_index: crate::ecs::spatial::OptimalSpatialIndex) -> Self {
        Self {
            archetype_manager: Arc::new(RwLock::new(archetype_manager)),
            spatial_index: Arc::new(spatial_index),
        }
    }
    
    /// Create from shared resources
    pub fn from_shared(
        archetype_manager: Arc<RwLock<ArchetypeManager>>,
        spatial_index: Arc<crate::ecs::spatial::OptimalSpatialIndex>
    ) -> Self {
        Self { archetype_manager, spatial_index }
    }
    
    /// Get entities with specific components that are also in spatial range
    pub fn entities_with_components_in_range(
        &self, 
        component_types: HashSet<TypeId>, 
        center: glam::IVec2, 
        radius: u32
    ) -> Vec<Entity> {
        // First get entities with the required components from archetype system
        let manager = self.archetype_manager.read();
        let archetype_ids = manager.find_archetypes_with_components(&component_types);
        
        let mut archetype_entities = HashSet::new();
        for archetype_id in archetype_ids {
            archetype_entities.extend(manager.get_archetype_entities(archetype_id));
        }
        
        // Then intersect with spatial results
        let spatial_entities: HashSet<Entity> = self.spatial_index
            .entities_in_range(center, radius)
            .into_iter()
            .collect();
        
        // Return intersection
        archetype_entities.intersection(&spatial_entities).copied().collect()
    }
    
    /// Get entities in same archetype as reference entity, within spatial range
    pub fn entities_like_in_range(&self, reference_entity: Entity, center: glam::IVec2, radius: u32) -> Vec<Entity> {
        let manager = self.archetype_manager.read();
        let archetype_entities: HashSet<Entity> = manager
            .get_entities_with_same_layout(reference_entity)
            .into_iter()
            .collect();
        
        let spatial_entities: HashSet<Entity> = self.spatial_index
            .entities_in_range(center, radius)
            .into_iter()
            .collect();
        
        archetype_entities.intersection(&spatial_entities).copied().collect()
    }
    
    /// Get archetype distribution of entities in spatial area
    pub fn archetype_distribution_in_range(&self, center: glam::IVec2, radius: u32) -> HashMap<ArchetypeId, usize> {
        let spatial_entities = self.spatial_index.entities_in_range(center, radius);
        let manager = self.archetype_manager.read();
        
        let mut distribution = HashMap::new();
        for entity in spatial_entities {
            if let Some(archetype_id) = manager.storage().get_entity_archetype(entity) {
                *distribution.entry(archetype_id).or_insert(0) += 1;
            }
        }
        
        distribution
    }
    
    /// Register entity in both archetype and spatial systems
    pub fn register_entity<T: BundleComponentExtractor>(
        &self,
        entity: Entity,
        position: glam::IVec2,
        player_id: Option<u32>,
        is_movable: bool
    ) -> Result<ArchetypeId, Box<dyn std::error::Error>> {
        // Register in archetype system
        let archetype_id = {
            let manager = self.archetype_manager.write();
            manager.register_entity::<T>(entity)?
        };
        
        // Register in spatial system
        self.spatial_index.add_entity(entity, position, player_id, is_movable);
        
        Ok(archetype_id)
    }
    
    /// Remove entity from both systems
    pub fn remove_entity(&self, entity: Entity) -> Result<(), Box<dyn std::error::Error>> {
        // Remove from spatial system
        self.spatial_index.remove_entity(entity);
        
        // Remove from archetype system
        let manager = self.archetype_manager.write();
        manager.unregister_entity(entity)?;
        
        Ok(())
    }
}

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
    
    #[test]
    fn test_archetype_spatial_bridge_creation() {
        let manager = ArchetypeManager::new();
        let spatial_index = crate::ecs::spatial::OptimalSpatialIndex::new();
        let bridge = ArchetypeSpatialBridge::new(manager, spatial_index);
        
        let empty_components = HashSet::new();
        let entities = bridge.entities_with_components_in_range(
            empty_components, 
            glam::IVec2::new(0, 0), 
            5
        );
        assert!(entities.is_empty());
    }
}
