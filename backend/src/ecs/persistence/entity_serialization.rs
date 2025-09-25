//! Entity serialization utilities
//!
//! Provides functions for serializing and deserializing entities with their components.

use bevy_ecs::prelude::*;
use crate::ecs::{
    world_state::SerializedEntity,
    components::{Position, Movement, Health, Renderable, Name, Owner},
    hierarchy::{StableEntityId, Relationships}
};
use crate::simulation::snapshots::EntityData;

/// Serialize an entity with its components
pub fn serialize_entity(world: &World, entity: Entity) -> Option<SerializedEntity> {
    // Generate a stable ID for this entity
    let stable_id = StableEntityId::from_entity_id(entity.index(), entity.generation());
    
    let mut serialized = SerializedEntity::new(stable_id);
    
    // Serialize components if they exist
    if let Some(position) = world.get::<Position>(entity) {
        serialized.position = Some(position.clone());
    }
    
    if let Some(movement) = world.get::<Movement>(entity) {
        serialized.movement = Some(movement.clone());
    }
    
    if let Some(health) = world.get::<Health>(entity) {
        serialized.health = Some(health.clone());
    }
    
    if let Some(renderable) = world.get::<Renderable>(entity) {
        serialized.renderable = Some(renderable.clone());
    }
    
    if let Some(name) = world.get::<Name>(entity) {
        serialized.name = Some(name.clone());
    }
    
    if let Some(owner) = world.get::<Owner>(entity) {
        serialized.owner = Some(*owner);
    }
    
    if let Some(relationships) = world.get::<Relationships>(entity) {
        serialized.relationships = Some(relationships.clone());
    }
    
    // Check if entity has hierarchical marker
    serialized.hierarchical = world.get::<crate::ecs::hierarchy::Hierarchical>(entity).is_some();
    
    // Only return the serialized entity if it has at least one component
    if serialized.position.is_some() || 
       serialized.movement.is_some() || 
       serialized.health.is_some() || 
       serialized.renderable.is_some() || 
       serialized.name.is_some() || 
       serialized.owner.is_some() || 
       serialized.relationships.is_some() || 
       serialized.hierarchical {
        Some(serialized)
    } else {
        None
    }
}

/// Deserialize an entity with its components
pub fn deserialize_entity(world: &mut World, serialized: &SerializedEntity) -> Entity {
    let mut entity_commands = world.spawn_empty();
    let entity = entity_commands.id();
    
    // Add components if they exist in serialized data
    if let Some(position) = &serialized.position {
        entity_commands.insert(position.clone());
    }
    
    if let Some(movement) = &serialized.movement {
        entity_commands.insert(movement.clone());
    }
    
    if let Some(health) = &serialized.health {
        entity_commands.insert(health.clone());
    }
    
    if let Some(renderable) = &serialized.renderable {
        entity_commands.insert(renderable.clone());
    }
    
    if let Some(name) = &serialized.name {
        entity_commands.insert(name.clone());
    }
    
    if let Some(owner) = &serialized.owner {
        entity_commands.insert(*owner);
    }
    
    if let Some(relationships) = &serialized.relationships {
        entity_commands.insert(relationships.clone());
    }
    
    if serialized.hierarchical {
        entity_commands.insert(crate::ecs::hierarchy::Hierarchical);
    }
    
    entity
}

/// Serialize an entity with its components (for snapshot system)
pub fn serialize_entity_with_components(world: &World, entity: Entity) -> Option<EntityData> {
    // Get entity location information
    let entity_location = world.entities().get(entity)?;
    let archetype_id = entity_location.archetype_id;
    
    // Create EntityData for snapshot system
    let entity_data = EntityData {
        entity_id: entity.index(),
        generation: entity.generation(),
        archetype_id: archetype_id.index() as u32,
        component_indices: Vec::new(), // Component indices will be populated by the snapshot system
    };
    
    Some(entity_data)
}
