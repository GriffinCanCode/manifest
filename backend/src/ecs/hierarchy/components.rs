//! Hierarchy relationship components with strong validation
//!
//! Provides parent-child relationships, ownership chains, and dependency tracking
//! for entities with efficient graph-based operations.

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use crate::ecs::components::{ComponentError, Validate};
use crate::core::hashing::{FastHashSet, HashStrategies};
use super::graph::{EntityGraph, HierarchyError};

/// Stable entity identifier for serialization
/// Uses the entity's index and generation for a stable ID across save/load
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StableEntityId {
    pub index: u32,
    pub generation: u32,
}

impl From<Entity> for StableEntityId {
    fn from(entity: Entity) -> Self {
        Self {
            index: entity.index(),
            generation: entity.generation(),
        }
    }
}

impl From<StableEntityId> for Entity {
    fn from(stable_id: StableEntityId) -> Self {
        // Bevy 0.12 format: combine index and generation into bits
        // Generation is in the high 32 bits, index is in the low 32 bits
        let bits = ((stable_id.generation as u64) << 32) | (stable_id.index as u64);
        Entity::from_bits(bits)
    }
}

/// Relationship types between entities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RelationshipType {
    /// Parent-child relationship (city contains districts)
    Parent,
    /// Child-parent relationship (district belongs to city)
    Child,
    /// Ownership relationship (player owns units)
    Owner,
    /// Dependency relationship (building requires resource)
    Dependency,
    /// Attachment relationship (unit carries equipment)
    Attachment,
}

impl RelationshipType {
    /// Get the inverse relationship type
    pub fn inverse(self) -> Self {
        match self {
            Self::Parent => Self::Child,
            Self::Child => Self::Parent,
            Self::Owner => Self::Dependency,
            Self::Dependency => Self::Owner,
            Self::Attachment => Self::Attachment, // Symmetric
        }
    }

    /// Check if this relationship type is symmetric
    pub fn is_symmetric(self) -> bool {
        matches!(self, Self::Attachment)
    }
}

/// Single relationship edge between two entities
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Relationship {
    /// Target entity in this relationship
    target: Entity,
    /// Type of relationship
    relationship_type: RelationshipType,
    /// Optional metadata for the relationship
    metadata: Option<String>,
}

/// Serializable version of Relationship using stable entity IDs
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct SerializableRelationship {
    target: StableEntityId,
    relationship_type: RelationshipType,
    metadata: Option<String>,
}

impl serde::Serialize for Relationship {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let serializable = SerializableRelationship {
            target: self.target.into(),
            relationship_type: self.relationship_type,
            metadata: self.metadata.clone(),
        };
        serializable.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for Relationship {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let serializable = SerializableRelationship::deserialize(deserializer)?;
        Ok(Self {
            target: serializable.target.into(),
            relationship_type: serializable.relationship_type,
            metadata: serializable.metadata,
        })
    }
}

impl Relationship {
    /// Create new relationship with validation
    pub fn new(target: Entity, relationship_type: RelationshipType) -> Self {
        Self {
            target,
            relationship_type,
            metadata: None,
        }
    }

    /// Create relationship with metadata
    pub fn with_metadata(target: Entity, relationship_type: RelationshipType, metadata: String) -> Self {
        Self {
            target,
            relationship_type,
            metadata: Some(metadata),
        }
    }

    /// Get target entity
    pub fn target(&self) -> Entity {
        self.target
    }

    /// Get relationship type
    pub fn relationship_type(&self) -> RelationshipType {
        self.relationship_type
    }

    /// Get optional metadata
    pub fn metadata(&self) -> Option<&str> {
        self.metadata.as_deref()
    }
}

/// Component storing all outgoing relationships for an entity
#[derive(Component, Debug, Clone)]
pub struct Relationships {
    /// Set of outgoing relationships (optimized for fast lookups)
    relationships: FastHashSet<Relationship>,
    /// Cached hash for quick equality checks (not serialized)
    hash_cache: Option<u64>,
}

impl serde::Serialize for Relationships {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Convert to Vec for serialization to ensure deterministic ordering
        let mut relationships_vec: Vec<&Relationship> = self.relationships.iter().collect();
        relationships_vec.sort_by(|a, b| {
            // Sort by target entity index, then by relationship type, then by metadata
            (a.target.index(), &a.relationship_type, &a.metadata)
                .cmp(&(b.target.index(), &b.relationship_type, &b.metadata))
        });
        relationships_vec.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for Relationships {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let relationships_vec: Vec<Relationship> = Vec::deserialize(deserializer)?;
        let mut relationships = FastHashSet::default();
        for relationship in relationships_vec {
            relationships.insert(relationship);
        }
        
        Ok(Self {
            relationships,
            hash_cache: None, // Will be computed on first access
        })
    }
}

impl Relationships {
    /// Create empty relationships component
    pub fn new() -> Self {
        Self {
            relationships: FastHashSet::default(),
            hash_cache: None,
        }
    }

    /// Add a relationship with validation and cycle detection
    pub fn add(&mut self, relationship: Relationship) -> Result<bool, ComponentError> {
        // Validate the relationship doesn't create cycles for hierarchical types
        if matches!(relationship.relationship_type, RelationshipType::Parent | RelationshipType::Child) {
            // Check for immediate cycles - don't allow self-references or reciprocal parent-child relationships
            self.validate_no_cycle(&relationship)?;
        }

        let inserted = self.relationships.insert(relationship);
        if inserted {
            self.invalidate_cache();
        }
        Ok(inserted)
    }

    /// Validate that adding a relationship won't create a cycle
    fn validate_no_cycle(&self, new_relationship: &Relationship) -> Result<(), ComponentError> {
        // Self-reference check
        if new_relationship.target == Entity::from_raw(0) {
            return Err(ComponentError::InvalidHierarchy(
                "Cannot create relationship with invalid entity".to_string()
            ));
        }

        // Check for reciprocal relationships that would create cycles
        match new_relationship.relationship_type {
            RelationshipType::Parent => {
                // If we're adding A -> B as Parent, check if B -> A already exists as Parent
                if self.relationships.iter().any(|rel| 
                    rel.target == new_relationship.target && 
                    rel.relationship_type == RelationshipType::Child) {
                    return Err(ComponentError::InvalidHierarchy(
                        format!("Cannot add parent relationship to {:?}: child relationship already exists", 
                               new_relationship.target)
                    ));
                }
            }
            RelationshipType::Child => {
                // If we're adding A -> B as Child, check if B -> A already exists as Child  
                if self.relationships.iter().any(|rel| 
                    rel.target == new_relationship.target && 
                    rel.relationship_type == RelationshipType::Parent) {
                    return Err(ComponentError::InvalidHierarchy(
                        format!("Cannot add child relationship to {:?}: parent relationship already exists", 
                               new_relationship.target)
                    ));
                }
            }
            _ => {} // Other relationship types don't need cycle detection
        }

        Ok(())
    }

    /// Remove a relationship (ignores metadata)
    pub fn remove(&mut self, target: Entity, relationship_type: RelationshipType) -> bool {
        // Find the relationship to remove (ignores metadata)
        let to_remove = self.relationships
            .iter()
            .find(|r| r.target == target && r.relationship_type == relationship_type)
            .cloned();
        
        if let Some(relationship) = to_remove {
            let removed = self.relationships.remove(&relationship);
            if removed {
                self.invalidate_cache();
            }
            removed
        } else {
            false
        }
    }

    /// Check if relationship exists (ignores metadata)
    pub fn has_relationship(&self, target: Entity, relationship_type: RelationshipType) -> bool {
        self.relationships
            .iter()
            .any(|r| r.target == target && r.relationship_type == relationship_type)
    }

    /// Get all relationships of a specific type
    pub fn get_by_type(&self, relationship_type: RelationshipType) -> Vec<Entity> {
        self.relationships
            .iter()
            .filter(|r| r.relationship_type == relationship_type)
            .map(|r| r.target)
            .collect()
    }

    /// Get all parent entities
    pub fn parents(&self) -> Vec<Entity> {
        self.get_by_type(RelationshipType::Parent)
    }

    /// Get all child entities
    pub fn children(&self) -> Vec<Entity> {
        self.get_by_type(RelationshipType::Child)
    }

    /// Get all owned entities
    pub fn owned(&self) -> Vec<Entity> {
        self.get_by_type(RelationshipType::Owner)
    }

    /// Get all dependencies
    pub fn dependencies(&self) -> Vec<Entity> {
        self.get_by_type(RelationshipType::Dependency)
    }

    /// Get count of relationships
    pub fn count(&self) -> usize {
        self.relationships.len()
    }

    /// Check if has any relationships
    pub fn is_empty(&self) -> bool {
        self.relationships.is_empty()
    }

    /// Get iterator over all relationships
    pub fn iter(&self) -> impl Iterator<Item = &Relationship> {
        self.relationships.iter()
    }

    /// Clear all relationships
    pub fn clear(&mut self) {
        self.relationships.clear();
        self.invalidate_cache();
    }

    /// Remap entity IDs in relationships (used during deserialization)
    pub fn remap_entities(&mut self, entity_mapping: &std::collections::HashMap<crate::ecs::hierarchy::StableEntityId, Entity>) {
        let mut updated_relationships = FastHashSet::default();
        
        for relationship in &self.relationships {
            let stable_target = crate::ecs::hierarchy::StableEntityId::from(relationship.target);
            if let Some(&new_target) = entity_mapping.get(&stable_target) {
                let updated_relationship = Relationship {
                    target: new_target,
                    relationship_type: relationship.relationship_type,
                    metadata: relationship.metadata.clone(),
                };
                updated_relationships.insert(updated_relationship);
            } else {
                // Keep original if no mapping found (shouldn't happen in well-formed saves)
                updated_relationships.insert(relationship.clone());
            }
        }
        
        self.relationships = updated_relationships;
        self.invalidate_cache();
    }

    /// Get or compute cached hash for fast equality checks
    pub fn get_hash(&mut self) -> u64 {
        if let Some(hash) = self.hash_cache {
            return hash;
        }

        // Create deterministic hash from all relationships
        let mut relationship_hashes: Vec<u64> = self.relationships
            .iter()
            .map(|r| {
                let combined = format!("{}:{:?}:{}", 
                    r.target.index(), 
                    r.relationship_type,
                    r.metadata.as_deref().unwrap_or("")
                );
                HashStrategies::hash_string(&combined)
            })
            .collect();
        
        // Sort for deterministic hashing
        relationship_hashes.sort_unstable();
        
        let hash = HashStrategies::combine_hashes(&relationship_hashes);
        self.hash_cache = Some(hash);
        hash
    }

    /// Invalidate cached hash
    fn invalidate_cache(&mut self) {
        self.hash_cache = None;
    }
}

impl Default for Relationships {
    fn default() -> Self {
        Self::new()
    }
}

impl Validate for Relationships {
    type Error = ComponentError;
    
    fn constraints() -> &'static str {
        "Relationships with max 10000 entries and valid targets"
    }
    
    fn validate(&self) -> Result<(), Self::Error> {
        // Validate relationship count isn't excessive
        if self.relationships.len() > 10000 {
            return Err(ComponentError::InvalidName(
                format!("Too many relationships: {} (max 10000)", self.relationships.len())
            ));
        }

        // Validate each relationship
        for relationship in &self.relationships {
            // Basic entity validation - ensure it's not null
            if relationship.target.index() == 0 {
                return Err(ComponentError::InvalidName(
                    "Invalid relationship target entity".to_string()
                ));
            }

            // Validate metadata length if present
            if let Some(metadata) = &relationship.metadata {
                if metadata.len() > 1000 {
                    return Err(ComponentError::InvalidName(
                        format!("Relationship metadata too long: {} chars (max 1000)", metadata.len())
                    ));
                }
            }
        }

        Ok(())
    }
}

/// Marker component for entities that participate in hierarchies
/// Enables fast filtering in hierarchy queries
#[derive(Component, Debug, Default, Clone, Copy, Serialize, Deserialize)]
pub struct Hierarchical;

impl Validate for Hierarchical {
    type Error = ComponentError;
    
    fn constraints() -> &'static str {
        "Marker component for hierarchical entities"
    }
    
    fn validate(&self) -> Result<(), Self::Error> {
        Ok(()) // Marker component always valid
    }
}
