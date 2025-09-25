//! Slotmap-based archetype storage for optimal memory layout
//!
//! Provides efficient storage and retrieval of archetypes using slotmap's
//! generational indices for safe, fast access with minimal memory overhead.

use super::types::{Archetype, ArchetypeId, ComponentSignature, ArchetypeError, ArchetypeResult, ArchetypeStats};
use slotmap::SlotMap;
use bevy_ecs::prelude::*;
use parking_lot::RwLock;
use crate::core::hashing::{collections, FastHashMap};

/// Thread-safe archetype storage using slotmap for generational safety
#[derive(Debug)]
pub struct ArchetypeStorage {
    /// Primary storage for archetypes with generational keys
    archetypes: RwLock<SlotMap<ArchetypeId, Archetype>>,
    /// Fast lookup from component signature to archetype ID
    signature_lookup: RwLock<FastHashMap<u64, ArchetypeId>>,
    /// Entity to archetype mapping for quick queries
    entity_archetype: RwLock<FastHashMap<Entity, ArchetypeId>>,
}

impl Default for ArchetypeStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for ArchetypeStorage {
    fn clone(&self) -> Self {
        // Create new storage and copy data manually
        let new_storage = Self::new();
        
        // Copy archetype data
        {
            let source_archetypes = self.archetypes.read();
            let mut dest_archetypes = new_storage.archetypes.write();
            for (id, archetype) in source_archetypes.iter() {
                let new_archetype = Archetype::new(id, archetype.signature.clone());
                // Copy entities
                let archetype_id = dest_archetypes.insert(new_archetype);
                if let Some(dest_arch) = dest_archetypes.get_mut(archetype_id) {
                    dest_arch.entities = archetype.entities.clone();
                }
            }
        }
        
        // Copy signature lookup
        {
            let source_lookup = self.signature_lookup.read();
            let mut dest_lookup = new_storage.signature_lookup.write();
            for (&hash, &archetype_id) in source_lookup.iter() {
                dest_lookup.insert(hash, archetype_id);
            }
        }
        
        // Copy entity mapping
        {
            let source_mapping = self.entity_archetype.read();
            let mut dest_mapping = new_storage.entity_archetype.write();
            for (&entity, &archetype_id) in source_mapping.iter() {
                dest_mapping.insert(entity, archetype_id);
            }
        }
        
        new_storage
    }
}

impl ArchetypeStorage {
    /// Create new archetype storage
    pub fn new() -> Self {
        Self {
            archetypes: RwLock::new(SlotMap::new()),
            signature_lookup: RwLock::new(collections::fast_hash_map()),
            entity_archetype: RwLock::new(collections::fast_hash_map()),
        }
    }
    
    /// Get or create archetype for given component signature
    pub fn get_or_create_archetype(&self, signature: ComponentSignature) -> ArchetypeId {
        let signature_hash = signature.hash();
        
        // Fast path: check if archetype already exists
        {
            let lookup = self.signature_lookup.read();
            if let Some(&archetype_id) = lookup.get(&signature_hash) {
                return archetype_id;
            }
        }
        
        // Slow path: create new archetype
        let mut archetypes = self.archetypes.write();
        let mut lookup = self.signature_lookup.write();
        
        // Double-check in case another thread created it
        if let Some(&archetype_id) = lookup.get(&signature_hash) {
            return archetype_id;
        }
        
        // Create new archetype
        let archetype_id = archetypes.insert(Archetype::new(ArchetypeId::default(), signature.clone()));
        
        // Update the archetype's ID to match the slot
        if let Some(archetype) = archetypes.get_mut(archetype_id) {
            archetype.id = archetype_id;
        }
        
        lookup.insert(signature_hash, archetype_id);
        archetype_id
    }
    
    /// Get archetype by ID
    pub fn get(&self, id: ArchetypeId) -> Option<parking_lot::MappedRwLockReadGuard<'_, Archetype>> {
        let archetypes = self.archetypes.read();
        if archetypes.contains_key(id) {
            Some(parking_lot::RwLockReadGuard::map(archetypes, |a| &a[id]))
        } else {
            None
        }
    }
    
    /// Get mutable archetype by ID  
    pub fn get_mut(&self, id: ArchetypeId) -> Option<parking_lot::MappedRwLockWriteGuard<'_, Archetype>> {
        let archetypes = self.archetypes.write();
        if archetypes.contains_key(id) {
            Some(parking_lot::RwLockWriteGuard::map(archetypes, |a| &mut a[id]))
        } else {
            None
        }
    }
    
    /// Find archetype by component signature
    pub fn find_by_signature(&self, signature: &ComponentSignature) -> Option<ArchetypeId> {
        let lookup = self.signature_lookup.read();
        lookup.get(&signature.hash()).copied()
    }
    
    /// Add entity to archetype
    pub fn add_entity_to_archetype(&self, entity: Entity, archetype_id: ArchetypeId) -> ArchetypeResult<()> {
        // Update entity mapping
        {
            let mut entity_mapping = self.entity_archetype.write();
            entity_mapping.insert(entity, archetype_id);
        }
        
        // Add to archetype
        if let Some(mut archetype) = self.get_mut(archetype_id) {
            archetype.add_entity(entity);
            Ok(())
        } else {
            Err(ArchetypeError::NotFound(archetype_id))
        }
    }
    
    /// Remove entity from archetype
    pub fn remove_entity_from_archetype(&self, entity: Entity) -> ArchetypeResult<ArchetypeId> {
        // Get current archetype
        let archetype_id = {
            let mut entity_mapping = self.entity_archetype.write();
            entity_mapping.remove(&entity).ok_or(ArchetypeError::EntityNotFound(entity))?
        };
        
        // Remove from archetype
        if let Some(mut archetype) = self.get_mut(archetype_id) {
            if archetype.remove_entity(entity) {
                Ok(archetype_id)
            } else {
                Err(ArchetypeError::EntityNotFound(entity))
            }
        } else {
            Err(ArchetypeError::NotFound(archetype_id))
        }
    }
    
    /// Move entity between archetypes
    pub fn move_entity(&self, entity: Entity, new_archetype_id: ArchetypeId) -> ArchetypeResult<ArchetypeId> {
        let old_archetype_id = self.remove_entity_from_archetype(entity)?;
        self.add_entity_to_archetype(entity, new_archetype_id)?;
        Ok(old_archetype_id)
    }
    
    /// Get archetype ID for entity
    pub fn get_entity_archetype(&self, entity: Entity) -> Option<ArchetypeId> {
        let entity_mapping = self.entity_archetype.read();
        entity_mapping.get(&entity).copied()
    }
    
    /// Get all archetype IDs
    pub fn all_archetype_ids(&self) -> Vec<ArchetypeId> {
        let archetypes = self.archetypes.read();
        archetypes.keys().collect()
    }
    
    /// Get archetype count
    pub fn archetype_count(&self) -> usize {
        let archetypes = self.archetypes.read();
        archetypes.len()
    }
    
    /// Get total entity count across all archetypes
    pub fn total_entity_count(&self) -> usize {
        let entity_mapping = self.entity_archetype.read();
        entity_mapping.len()
    }
    
    /// Remove empty archetypes to free memory
    pub fn cleanup_empty_archetypes(&self) -> usize {
        let mut archetypes = self.archetypes.write();
        let mut lookup = self.signature_lookup.write();
        
        let mut removed_count = 0;
        let empty_ids: Vec<_> = archetypes
            .iter()
            .filter(|(_, archetype)| archetype.is_empty())
            .map(|(id, _)| id)
            .collect();
        
        for id in empty_ids {
            if let Some(archetype) = archetypes.remove(id) {
                lookup.remove(&archetype.signature.hash());
                removed_count += 1;
            }
        }
        
        removed_count
    }
    
    /// Get storage statistics
    pub fn stats(&self) -> ArchetypeStats {
        let archetypes = self.archetypes.read();
        ArchetypeStats::calculate(&archetypes)
    }
    
    /// Clear all archetypes and entities
    pub fn clear(&self) {
        let mut archetypes = self.archetypes.write();
        let mut lookup = self.signature_lookup.write();
        let mut entity_mapping = self.entity_archetype.write();
        
        archetypes.clear();
        lookup.clear();
        entity_mapping.clear();
    }
    
    /// Validate storage consistency (for debugging)
    pub fn validate(&self) -> Result<(), String> {
        let archetypes = self.archetypes.read();
        let lookup = self.signature_lookup.read();
        let entity_mapping = self.entity_archetype.read();
        
        // Check signature lookup consistency
        for (hash, &archetype_id) in lookup.iter() {
            if let Some(archetype) = archetypes.get(archetype_id) {
                if archetype.signature.hash() != *hash {
                    return Err(format!("Hash mismatch for archetype {:?}", archetype_id));
                }
            } else {
                return Err(format!("Archetype {:?} referenced in lookup but not found", archetype_id));
            }
        }
        
        // Check entity mapping consistency  
        for (&entity, &archetype_id) in entity_mapping.iter() {
            if let Some(archetype) = archetypes.get(archetype_id) {
                if !archetype.entities.contains(&entity) {
                    return Err(format!("Entity {:?} mapped to archetype {:?} but not in archetype", entity, archetype_id));
                }
            } else {
                return Err(format!("Entity {:?} mapped to non-existent archetype {:?}", entity, archetype_id));
            }
        }
        
        // Check archetype entity consistency
        for (archetype_id, archetype) in archetypes.iter() {
            for &entity in &archetype.entities {
                if entity_mapping.get(&entity) != Some(&archetype_id) {
                    return Err(format!("Entity {:?} in archetype {:?} but not mapped correctly", entity, archetype_id));
                }
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::any::TypeId;

    #[test]
    fn test_archetype_creation_and_retrieval() {
        let storage = ArchetypeStorage::new();
        
        let mut types = HashSet::new();
        types.insert(TypeId::of::<u32>());
        let signature = ComponentSignature::new(types);
        
        let id = storage.get_or_create_archetype(signature.clone());
        assert!(storage.get(id).is_some());
        
        // Should return same archetype for same signature
        let id2 = storage.get_or_create_archetype(signature);
        assert_eq!(id, id2);
    }
    
    #[test]
    fn test_entity_management() {
        let storage = ArchetypeStorage::new();
        
        let mut types = HashSet::new();
        types.insert(TypeId::of::<String>());
        let signature = ComponentSignature::new(types);
        
        let archetype_id = storage.get_or_create_archetype(signature);
        let entity = Entity::from_raw(123);
        
        // Add entity
        assert!(storage.add_entity_to_archetype(entity, archetype_id).is_ok());
        assert_eq!(storage.get_entity_archetype(entity), Some(archetype_id));
        
        // Remove entity
        assert_eq!(storage.remove_entity_from_archetype(entity).unwrap(), archetype_id);
        assert_eq!(storage.get_entity_archetype(entity), None);
    }
    
    #[test]
    fn test_entity_movement() {
        let storage = ArchetypeStorage::new();
        
        let mut types1 = HashSet::new();
        types1.insert(TypeId::of::<u32>());
        let sig1 = ComponentSignature::new(types1);
        
        let mut types2 = HashSet::new();
        types2.insert(TypeId::of::<String>());
        let sig2 = ComponentSignature::new(types2);
        
        let arch1 = storage.get_or_create_archetype(sig1);
        let arch2 = storage.get_or_create_archetype(sig2);
        let entity = Entity::from_raw(456);
        
        // Add to first archetype
        storage.add_entity_to_archetype(entity, arch1).unwrap();
        
        // Move to second archetype
        let old_id = storage.move_entity(entity, arch2).unwrap();
        assert_eq!(old_id, arch1);
        assert_eq!(storage.get_entity_archetype(entity), Some(arch2));
    }
    
    #[test]
    fn test_cleanup() {
        let storage = ArchetypeStorage::new();
        
        let mut types = HashSet::new();
        types.insert(TypeId::of::<bool>());
        let signature = ComponentSignature::new(types);
        
        let archetype_id = storage.get_or_create_archetype(signature);
        let entity = Entity::from_raw(789);
        
        storage.add_entity_to_archetype(entity, archetype_id).unwrap();
        storage.remove_entity_from_archetype(entity).unwrap();
        
        // Should clean up empty archetype
        assert_eq!(storage.cleanup_empty_archetypes(), 1);
        assert!(storage.get(archetype_id).is_none());
    }
    
    #[test]
    fn test_validation() {
        let storage = ArchetypeStorage::new();
        assert!(storage.validate().is_ok());
        
        // Add some data and validate again
        let mut types = HashSet::new();
        types.insert(TypeId::of::<f32>());
        let signature = ComponentSignature::new(types);
        
        let archetype_id = storage.get_or_create_archetype(signature);
        let entity = Entity::from_raw(999);
        storage.add_entity_to_archetype(entity, archetype_id).unwrap();
        
        assert!(storage.validate().is_ok());
    }
}
