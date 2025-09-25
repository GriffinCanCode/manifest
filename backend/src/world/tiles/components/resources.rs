//! Resource components for tiles
//!
//! Contains resource-related components for storing tile resources and their properties.

use bevy_ecs::prelude::Component;
use serde::{Deserialize, Serialize};

/// Resource component for tiles (sparse)
#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub struct TileResource {
    /// Resource type identifier  
    pub resource_type: ResourceType,
    /// Quantity available (0-255 for memory efficiency)
    pub quantity: u8,
    /// Whether resource is visible to players
    pub discovered: bool,
    /// Depletion rate over time
    pub depletion_rate: f32,
}

/// Resource type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum ResourceType {
    None = 0,
    Iron = 1,
    Coal = 2,
    Oil = 3,
    Gold = 4,
    Silver = 5,
    Copper = 6,
    Stone = 7,
    Wheat = 8,
    Fish = 9,
    Cattle = 10,
    // Add more resource types as needed
}

impl Default for ResourceType {
    fn default() -> Self {
        Self::None
    }
}
