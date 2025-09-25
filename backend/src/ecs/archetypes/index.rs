//! Archetype indexing for efficient component-based lookups
//!
//! Provides indexing structures that complement the existing spatial indexing
//! in queries.rs, focusing on component signature patterns.

use super::types::{ArchetypeId, ComponentSignature};
use serde::{Deserialize, Serialize};
use std::any::TypeId;
use std::collections::HashSet;
use parking_lot::RwLock;
use crate::core::hashing::{collections, FastHashMap, FastHashSet, HashStrategies};

/// Index for fast archetype lookups by component patterns
#[derive(Debug)]
pub struct ArchetypeIndex {
    /// Map component types to archetypes containing them (optimized for TypeId keys)
    component_to_archetypes: RwLock<FastHashMap<TypeId, FastHashSet<ArchetypeId>>>,
    /// Map archetype to its component signature (optimized for ArchetypeId keys)  
    archetype_signatures: RwLock<FastHashMap<ArchetypeId, ComponentSignature>>,
    /// Cached common queries for performance (optimized for u64 hash keys)
    query_cache: RwLock<FastHashMap<u64, Vec<ArchetypeId>>>,
}

impl Default for ArchetypeIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl ArchetypeIndex {
    /// Create new archetype index
    pub fn new() -> Self {
        Self {
            component_to_archetypes: RwLock::new(collections::fast_hash_map()),
            archetype_signatures: RwLock::new(collections::fast_hash_map()),
            query_cache: RwLock::new(collections::fast_hash_map()),
        }
    }
    
    /// Register archetype in the index
    pub fn register_archetype(&self, archetype_id: ArchetypeId, signature: ComponentSignature) {
        // Update component mapping
        {
            let mut component_map = self.component_to_archetypes.write();
            for &component_type in signature.components() {
                component_map.entry(component_type)
                    .or_insert_with(|| collections::fast_hash_set())
                    .insert(archetype_id);
            }
        }
        
        // Store signature
        {
            let mut signatures = self.archetype_signatures.write();
            signatures.insert(archetype_id, signature);
        }
        
        // Clear cache since index changed
        self.clear_cache();
    }
    
    /// Remove archetype from index
    pub fn unregister_archetype(&self, archetype_id: ArchetypeId) {
        // Get signature first
        let signature = {
            let mut signatures = self.archetype_signatures.write();
            signatures.remove(&archetype_id)
        };
        
        if let Some(signature) = signature {
            // Remove from component mapping
            let mut component_map = self.component_to_archetypes.write();
            for &component_type in signature.components() {
                if let Some(archetype_set) = component_map.get_mut(&component_type) {
                    archetype_set.remove(&archetype_id);
                    if archetype_set.is_empty() {
                        component_map.remove(&component_type);
                    }
                }
            }
        }
        
        // Clear cache
        self.clear_cache();
    }
    
    /// Find archetypes containing specific component
    pub fn archetypes_with_component(&self, component_type: TypeId) -> Vec<ArchetypeId> {
        let component_map = self.component_to_archetypes.read();
        component_map.get(&component_type)
            .map(|set| set.iter().copied().collect())
            .unwrap_or_default()
    }
    
    /// Find archetypes containing all specified components
    pub fn archetypes_with_all_components(&self, component_types: &HashSet<TypeId>) -> Vec<ArchetypeId> {
        if component_types.is_empty() {
            return Vec::new();
        }
        
        // Check cache first
        let cache_key = self.calculate_query_hash(component_types);
        {
            let cache = self.query_cache.read();
            if let Some(cached) = cache.get(&cache_key) {
                return cached.clone();
            }
        }
        
        let component_map = self.component_to_archetypes.read();
        
        // Start with archetypes containing the first component
        let mut iter = component_types.iter();
        let first_component = match iter.next() {
            Some(&comp) => comp,
            None => return Vec::new(),
        };
        
        let mut result: FastHashSet<ArchetypeId> = component_map.get(&first_component)
            .cloned()
            .unwrap_or_else(|| collections::fast_hash_set());
        
        // Intersect with archetypes containing other components
        for &component_type in iter {
            if let Some(archetypes) = component_map.get(&component_type) {
                result = result.intersection(archetypes).copied().collect();
            } else {
                result.clear();
                break;
            }
        }
        
        let result_vec: Vec<ArchetypeId> = result.into_iter().collect();
        
        // Cache the result
        {
            let mut cache = self.query_cache.write();
            cache.insert(cache_key, result_vec.clone());
        }
        
        result_vec
    }
    
    /// Find archetypes containing any of the specified components
    pub fn archetypes_with_any_component(&self, component_types: &HashSet<TypeId>) -> Vec<ArchetypeId> {
        let component_map = self.component_to_archetypes.read();
        let mut result: FastHashSet<ArchetypeId> = collections::fast_hash_set();
        
        for &component_type in component_types {
            if let Some(archetypes) = component_map.get(&component_type) {
                result.extend(archetypes);
            }
        }
        
        result.into_iter().collect()
    }
    
    /// Get signature for archetype
    pub fn get_signature(&self, archetype_id: ArchetypeId) -> Option<ComponentSignature> {
        let signatures = self.archetype_signatures.read();
        signatures.get(&archetype_id).cloned()
    }
    
    /// Get all registered archetypes
    pub fn all_archetypes(&self) -> Vec<ArchetypeId> {
        let signatures = self.archetype_signatures.read();
        signatures.keys().copied().collect()
    }
    
    /// Get component types present in the system
    pub fn all_component_types(&self) -> Vec<TypeId> {
        let component_map = self.component_to_archetypes.read();
        component_map.keys().copied().collect()
    }
    
    /// Clear query cache
    pub fn clear_cache(&self) {
        let mut cache = self.query_cache.write();
        cache.clear();
    }
    
    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, usize) {
        let cache = self.query_cache.read();
        let component_map = self.component_to_archetypes.read();
        (cache.len(), component_map.len())
    }
    
    // Helper to calculate hash for query caching using optimized TypeId hashing
    fn calculate_query_hash(&self, component_types: &HashSet<TypeId>) -> u64 {
        let sorted_types: Vec<_> = component_types.iter().copied().collect();
        HashStrategies::hash_type_signature(&sorted_types)
    }
}

/// Statistics for archetype index performance monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    pub total_archetypes: usize,
    pub total_component_types: usize,
    pub cache_entries: usize,
    pub avg_archetypes_per_component: f32,
}

impl ArchetypeIndex {
    /// Get performance statistics
    pub fn stats(&self) -> IndexStats {
        let signatures = self.archetype_signatures.read();
        let component_map = self.component_to_archetypes.read();
        let cache = self.query_cache.read();
        
        let total_archetypes = signatures.len();
        let total_component_types = component_map.len();
        let cache_entries = cache.len();
        
        let total_mappings: usize = component_map.values().map(|set| set.len()).sum();
        let avg_archetypes_per_component = if total_component_types > 0 {
            total_mappings as f32 / total_component_types as f32
        } else {
            0.0
        };
        
        IndexStats {
            total_archetypes,
            total_component_types,
            cache_entries,
            avg_archetypes_per_component,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::TypeId;
    use slotmap::SlotMap;
    use crate::ecs::archetypes::types::Archetype;

    #[test]
    fn test_index_creation() {
        let index = ArchetypeIndex::new();
        let stats = index.stats();
        assert_eq!(stats.total_archetypes, 0);
        assert_eq!(stats.total_component_types, 0);
    }
    
    #[test]
    fn test_archetype_registration() {
        let index = ArchetypeIndex::new();
        let mut slot_map = SlotMap::new();
        let archetype_id = slot_map.insert(());
        
        let mut components = HashSet::new();
        components.insert(TypeId::of::<u32>());
        components.insert(TypeId::of::<String>());
        let signature = ComponentSignature::new(components);
        
        index.register_archetype(archetype_id, signature);
        
        let stats = index.stats();
        assert_eq!(stats.total_archetypes, 1);
        assert_eq!(stats.total_component_types, 2);
    }
    
    #[test]
    fn test_component_lookup() {
        let index = ArchetypeIndex::new();
        let mut slot_map = SlotMap::new();
        let archetype_id = slot_map.insert(());
        
        let mut components = HashSet::new();
        components.insert(TypeId::of::<u32>());
        let signature = ComponentSignature::new(components);
        
        index.register_archetype(archetype_id, signature);
        
        let archetypes = index.archetypes_with_component(TypeId::of::<u32>());
        assert_eq!(archetypes.len(), 1);
        assert!(archetypes.contains(&archetype_id));
        
        let empty = index.archetypes_with_component(TypeId::of::<String>());
        assert!(empty.is_empty());
    }
    
    #[test]
    fn test_multi_component_query() {
        let index = ArchetypeIndex::new();
        let mut slot_map = SlotMap::new();
        
        // Create archetype with u32 and String
        let arch1 = slot_map.insert(());
        let mut comp1 = HashSet::new();
        comp1.insert(TypeId::of::<u32>());
        comp1.insert(TypeId::of::<String>());
        let sig1 = ComponentSignature::new(comp1.clone());
        index.register_archetype(arch1, sig1);
        
        // Create archetype with only u32
        let arch2 = slot_map.insert(());
        let mut comp2 = HashSet::new();
        comp2.insert(TypeId::of::<u32>());
        let sig2 = ComponentSignature::new(comp2);
        index.register_archetype(arch2, sig2);
        
        // Query for all components - should find arch1
        let all_result = index.archetypes_with_all_components(&comp1);
        assert_eq!(all_result.len(), 1);
        assert!(all_result.contains(&arch1));
        
        // Query for any component - should find both
        let any_result = index.archetypes_with_any_component(&comp1);
        assert_eq!(any_result.len(), 2);
        assert!(any_result.contains(&arch1));
        assert!(any_result.contains(&arch2));
    }
    
    #[test]
    fn test_archetype_unregistration() {
        let index = ArchetypeIndex::new();
        let mut slot_map = SlotMap::new();
        let archetype_id = slot_map.insert(());
        
        let mut components = HashSet::new();
        components.insert(TypeId::of::<bool>());
        let signature = ComponentSignature::new(components);
        
        index.register_archetype(archetype_id, signature);
        assert_eq!(index.stats().total_archetypes, 1);
        
        index.unregister_archetype(archetype_id);
        assert_eq!(index.stats().total_archetypes, 0);
        assert_eq!(index.stats().total_component_types, 0);
    }
    
    #[test]
    fn test_query_caching() {
        let index = ArchetypeIndex::new();
        let mut slot_map = SlotMap::new();
        let archetype_id = slot_map.insert(());
        
        let mut components = HashSet::new();
        components.insert(TypeId::of::<f32>());
        let signature = ComponentSignature::new(components.clone());
        
        index.register_archetype(archetype_id, signature);
        
        // First query should populate cache
        let result1 = index.archetypes_with_all_components(&components);
        let (cache_size_1, _) = index.cache_stats();
        
        // Second identical query should use cache
        let result2 = index.archetypes_with_all_components(&components);
        let (cache_size_2, _) = index.cache_stats();
        
        assert_eq!(result1, result2);
        assert_eq!(cache_size_1, cache_size_2); // Should be same cache entry
        assert!(cache_size_1 > 0);
    }
}
