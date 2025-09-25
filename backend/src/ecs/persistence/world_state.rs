//! World state serialization types
//!
//! Contains the main WorldState type and supporting serialization structures.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::ecs::hierarchy::{StableEntityId, Relationships};
use crate::ecs::resources::{GameTime, Players};

/// Complete serializable world state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldState {
    /// Game timing information
    pub game_time: GameTime,
    /// Player information
    pub players: Players,
    /// Camera position for UI state
    pub camera_position: (f32, f32),
    /// Camera zoom level
    pub camera_zoom: f32,
    /// Total number of entities
    pub entity_count: u32,
    /// Serialized entities
    pub entities: Vec<SerializedEntity>,
    /// Entity hierarchical relationships (legacy format)
    pub entity_relationships: HashMap<StableEntityId, Relationships>,
    /// List of hierarchical entities (legacy format)  
    pub hierarchical_entities: Vec<StableEntityId>,
}

impl Default for WorldState {
    fn default() -> Self {
        Self {
            game_time: GameTime::default(),
            players: Players::default(),
            camera_position: (0.0, 0.0),
            camera_zoom: 1.0,
            entity_count: 0,
            entities: Vec::new(),
            entity_relationships: HashMap::new(),
            hierarchical_entities: Vec::new(),
        }
    }
}

/// Serialized entity with optional components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedEntity {
    /// Entity ID (stable across saves)
    pub stable_id: StableEntityId,
    /// Position component
    pub position: Option<crate::ecs::components::Position>,
    /// Movement component
    pub movement: Option<crate::ecs::components::Movement>,
    /// Health component
    pub health: Option<crate::ecs::components::Health>,
    /// Renderable component
    pub renderable: Option<crate::ecs::components::Renderable>,
    /// Name component
    pub name: Option<crate::ecs::components::Name>,
    /// Owner component
    pub owner: Option<crate::ecs::components::Owner>,
    /// Relationships component
    pub relationships: Option<Relationships>,
    /// Whether entity is hierarchical
    pub hierarchical: bool,
}

impl SerializedEntity {
    pub fn new(stable_id: StableEntityId) -> Self {
        Self {
            stable_id,
            position: None,
            movement: None,
            health: None,
            renderable: None,
            name: None,
            owner: None,
            relationships: None,
            hierarchical: false,
        }
    }
}
