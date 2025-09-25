//! ECS (Entity Component System) architecture using bevy_ecs
//! 
//! This module provides the core ECS infrastructure for the Manifest game engine.

pub mod archetypes;
pub mod changes;
pub mod components;
pub mod entities;
pub mod hierarchy;
pub mod resources;
pub mod saves;
pub mod schedule;
pub mod spatial;
pub mod systems;
pub mod world;

#[cfg(test)]
pub mod tests;
#[cfg(test)]
pub mod archetype_integration_test;

// Re-export commonly used ECS types for convenience
pub use bevy_ecs::prelude::*;

// Re-export our custom types
pub use archetypes::*;
pub use changes::*;
pub use components::*;
pub use entities::*;
pub use hierarchy::*;
pub use resources::*;
pub use saves::*;
pub use schedule::*;
pub use spatial::*;
pub use systems::*;
pub use world::*;
