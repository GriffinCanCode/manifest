//! Movement and pathfinding components
//!
//! Contains components related to movement costs and pathfinding mechanics.

use bevy_ecs::prelude::Component;
use serde::{Deserialize, Serialize};

/// Movement cost component for pathfinding
#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub struct MovementCost {
    /// Base movement cost for this tile
    pub base_cost: f32,
    /// Current modified cost (affected by improvements, weather, etc.)
    pub current_cost: f32,
    /// Whether tile blocks movement entirely
    pub impassable: bool,
}

impl Default for MovementCost {
    fn default() -> Self {
        Self {
            base_cost: 1.0,
            current_cost: 1.0,
            impassable: false,
        }
    }
}
