//! ECS (Entity Component System) architecture using bevy_ecs
//! 
//! This module provides the core ECS infrastructure for the Manifest game engine.

pub mod components;
pub mod hierarchy;
pub mod persistence;
pub mod resources;
pub mod spatial;
pub mod systems;
pub mod world;

#[cfg(test)]
pub mod tests;

// Re-export commonly used ECS types for convenience
pub use bevy_ecs::prelude::*;

// Re-export our custom types
pub use components::*;
pub use hierarchy::*;
pub use persistence::*;
pub use resources::*;
pub use spatial::*;
pub use systems::*;
pub use world::*;
