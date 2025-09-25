//! Core archetype type definitions with strong typing
//!
//! Provides fundamental types for archetype identification and management
//! with minimal memory overhead and maximum type safety.

use slotmap::{SlotMap, DefaultKey as SlotKey};
use bevy_ecs::prelude::*;
use serde::{Serialize, Deserialize};
use std::collections::{HashMap, HashSet};
use std::any::TypeId;
use crate::core::hashing::{collections, HashStrategies, FastHashMap, FastHashSet};

/// Unique identifier for an archetype using slotmap's efficient keys
pub type ArchetypeId = SlotKey;

/// Component type signature for archetype identification
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentSignature {
    /// Set of component type IDs that define this archetype (optimized for TypeId)
    components: FastHashSet<TypeId>,
    /// Cached hash for performance
    hash: u64,
}

impl ComponentSignature {
    /// Create new component signature from type IDs
    pub fn new(component_types: HashSet<TypeId>) -> Self {
        // Sort for consistent hashing (using a stable sort key)
        let mut sorted_types: Vec<_> = component_types.iter().copied().collect();
        sorted_types.sort_by_key(|&id| {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            id.hash(&mut hasher);
            hasher.finish()
        });
        
        // Use our optimized TypeId signature hashing
        let hash = HashStrategies::hash_type_signature(&sorted_types);
        
        // Convert to our optimized FastHashSet
        let fast_components: FastHashSet<TypeId> = component_types.into_iter().collect();
        
        Self {
            components: fast_components,
            hash,
        }
    }
    
    /// Get component type IDs
    pub fn components(&self) -> &FastHashSet<TypeId> {
        &self.components
    }
    
    /// Check if signature contains component type
    pub fn has_component(&self, type_id: TypeId) -> bool {
        self.components.contains(&type_id)
    }
    
    /// Get number of components in signature
    pub fn len(&self) -> usize {
        self.components.len()
    }
    
    /// Check if signature is empty
    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }
    
    /// Get cached hash value
    pub fn hash(&self) -> u64 {
        self.hash
    }
}

/// Archetype metadata and entity storage reference
#[derive(Debug)]
pub struct Archetype {
    /// Unique identifier for this archetype
    pub id: ArchetypeId,
    /// Component signature defining this archetype
    pub signature: ComponentSignature,
    /// Entities belonging to this archetype
    pub entities: Vec<Entity>,
    /// Component storage indices for efficient access
    pub component_indices: FastHashMap<TypeId, usize>,
    /// Creation timestamp for debugging
    pub created_at: std::time::Instant,
}

impl Archetype {
    /// Create new archetype with given signature
    pub fn new(id: ArchetypeId, signature: ComponentSignature) -> Self {
        let mut component_indices = collections::fast_hash_map_with_capacity(signature.len());
        
        // Build component indices for efficient lookup
        for (index, &type_id) in signature.components().iter().enumerate() {
            component_indices.insert(type_id, index);
        }
        
        Self {
            id,
            signature,
            entities: Vec::new(),
            component_indices,
            created_at: std::time::Instant::now(),
        }
    }
    
    /// Add entity to this archetype
    pub fn add_entity(&mut self, entity: Entity) {
        self.entities.push(entity);
    }
    
    /// Remove entity from this archetype
    pub fn remove_entity(&mut self, entity: Entity) -> bool {
        if let Some(pos) = self.entities.iter().position(|&e| e == entity) {
            self.entities.swap_remove(pos);
            true
        } else {
            false
        }
    }
    
    /// Get entity count
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }
    
    /// Check if archetype is empty
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }
    
    /// Get component index for type
    pub fn component_index(&self, type_id: TypeId) -> Option<usize> {
        self.component_indices.get(&type_id).copied()
    }
    
    /// Check if archetype matches signature
    pub fn matches_signature(&self, other: &ComponentSignature) -> bool {
        self.signature == *other
    }
    
    /// Check if archetype contains all required components
    pub fn contains_all(&self, required: &HashSet<TypeId>) -> bool {
        required.iter().all(|&type_id| self.signature.has_component(type_id))
    }
    
    /// Check if archetype contains any of the components
    pub fn contains_any(&self, components: &HashSet<TypeId>) -> bool {
        components.iter().any(|&type_id| self.signature.has_component(type_id))
    }
}

/// Error types for archetype operations
#[derive(Debug, thiserror::Error)]
pub enum ArchetypeError {
    #[error("Archetype not found: {0:?}")]
    NotFound(ArchetypeId),
    #[error("Entity not in archetype: {0:?}")]
    EntityNotFound(Entity),
    #[error("Component type not in archetype: {0:?}")]
    ComponentNotFound(TypeId),
    #[error("Invalid archetype signature: {0}")]
    InvalidSignature(String),
}

/// Result type for archetype operations
pub type ArchetypeResult<T> = Result<T, ArchetypeError>;

/// Archetype statistics for monitoring and debugging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchetypeStats {
    /// Total number of archetypes
    pub total_archetypes: usize,
    /// Total entities across all archetypes
    pub total_entities: usize,
    /// Average entities per archetype
    pub avg_entities_per_archetype: f32,
    /// Largest archetype size
    pub max_archetype_size: usize,
    /// Number of empty archetypes
    pub empty_archetypes: usize,
    /// Most common component count
    pub common_component_count: usize,
}

impl ArchetypeStats {
    /// Calculate statistics from archetype storage
    pub fn calculate(archetypes: &SlotMap<ArchetypeId, Archetype>) -> Self {
        let total_archetypes = archetypes.len();
        let total_entities = archetypes.values().map(|a| a.entity_count()).sum();
        let avg_entities_per_archetype = if total_archetypes > 0 {
            total_entities as f32 / total_archetypes as f32
        } else {
            0.0
        };
        
        let max_archetype_size = archetypes.values()
            .map(|a| a.entity_count())
            .max()
            .unwrap_or(0);
            
        let empty_archetypes = archetypes.values()
            .filter(|a| a.is_empty())
            .count();
            
        // Find most common component count
        let mut component_count_freq = collections::fast_hash_map();
        for archetype in archetypes.values() {
            *component_count_freq.entry(archetype.signature.len()).or_insert(0) += 1;
        }
        
        let common_component_count = component_count_freq
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(size, _)| size)
            .unwrap_or(0);
        
        Self {
            total_archetypes,
            total_entities,
            avg_entities_per_archetype,
            max_archetype_size,
            empty_archetypes,
            common_component_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::TypeId;

    #[test]
    fn test_component_signature_creation() {
        let mut types = HashSet::new();
        types.insert(TypeId::of::<u32>());
        types.insert(TypeId::of::<String>());
        
        let sig = ComponentSignature::new(types.clone());
        assert_eq!(sig.len(), 2);
        assert!(sig.has_component(TypeId::of::<u32>()));
        assert!(sig.has_component(TypeId::of::<String>()));
        assert!(!sig.has_component(TypeId::of::<bool>()));
    }
    
    #[test]
    fn test_archetype_creation() {
        let mut types = HashSet::new();
        types.insert(TypeId::of::<u32>());
        
        let sig = ComponentSignature::new(types);
        let mut slot_map = SlotMap::new();
        let id = slot_map.insert(());
        let archetype = Archetype::new(id, sig);
        
        assert_eq!(archetype.entity_count(), 0);
        assert!(archetype.is_empty());
    }
    
    #[test]
    fn test_archetype_entity_management() {
        let mut types = HashSet::new();
        types.insert(TypeId::of::<u32>());
        
        let sig = ComponentSignature::new(types);
        let mut slot_map = SlotMap::new();
        let id = slot_map.insert(());
        let mut archetype = Archetype::new(id, sig);
        
        let entity = Entity::from_raw(42);
        archetype.add_entity(entity);
        assert_eq!(archetype.entity_count(), 1);
        assert!(!archetype.is_empty());
        
        assert!(archetype.remove_entity(entity));
        assert_eq!(archetype.entity_count(), 0);
        assert!(archetype.is_empty());
        
        assert!(!archetype.remove_entity(entity)); // Already removed
    }
}
