//! Modular component system
//!
//! This module has been refactored from a large monolithic file into focused submodules:
//! - `validation`: Component validation traits and error types
//! - `core`: Basic components (Position, Movement, Health, Owner, Name)
//! - `rendering`: Rendering-related components (Renderable, Color)
//! - `interpolation`: Interpolated components for smooth animations

pub mod validation;
pub mod core;
pub mod rendering;
pub mod interpolation;
pub mod entities;

// Re-export commonly used types and traits
pub use validation::{Validate, ComponentError, ComponentResult};

// Re-export core components
pub use core::{
    Position, Movement, MovementType, Health, Owner, Name, GameSelection,
    hex_distance
};

// Re-export rendering components
pub use rendering::{Renderable, Color};

// Re-export interpolation components
pub use interpolation::{
    InterpolatedPosition, InterpolatedHealth, InterpolatedRenderable,
    DamageFlash, ColorPulse
};

// Re-export entity creation utilities
pub use entities::{
    EntityFactory, EntityQueries, MovableEntityBundle, LivingEntityBundle,
    TileBundle, UnitBundle
};

// Convenient type aliases
pub type ComponentValidationResult<T> = Result<T, ComponentError>;

/// Trait for components that can be reset to default state
pub trait Resettable {
    /// Reset component to default/initial state
    fn reset(&mut self);
}

impl Resettable for Movement {
    fn reset(&mut self) {
        self.reset_for_turn();
    }
}

impl Resettable for Health {
    fn reset(&mut self) {
        self.current = self.max;
    }
}

/// Trait for components that provide memory usage information
pub trait MemoryFootprint {
    /// Get estimated memory usage in bytes
    fn memory_footprint(&self) -> usize;
}

impl MemoryFootprint for Position {
    fn memory_footprint(&self) -> usize {
        std::mem::size_of::<Self>()
    }
}

impl MemoryFootprint for Movement {
    fn memory_footprint(&self) -> usize {
        std::mem::size_of::<Self>()
    }
}

impl MemoryFootprint for Health {
    fn memory_footprint(&self) -> usize {
        std::mem::size_of::<Self>()
    }
}

impl MemoryFootprint for Owner {
    fn memory_footprint(&self) -> usize {
        std::mem::size_of::<Self>()
    }
}

impl MemoryFootprint for Name {
    fn memory_footprint(&self) -> usize {
        std::mem::size_of::<Self>() + self.value().len()
    }
}

impl MemoryFootprint for Renderable {
    fn memory_footprint(&self) -> usize {
        std::mem::size_of::<Self>() + self.sprite.len()
    }
}

/// Utility functions for component management
pub mod utils {
    use super::*;
    use bevy_ecs::prelude::*;

    /// Validate all components on an entity
    pub fn validate_entity_components(world: &World, entity: Entity) -> Result<(), Vec<ComponentError>> {
        let mut errors = Vec::new();

        // Check Position
        if let Some(position) = world.get::<Position>(entity) {
            if let Err(e) = position.validate() {
                errors.push(e);
            }
        }

        // Check Movement
        if let Some(movement) = world.get::<Movement>(entity) {
            if let Err(e) = movement.validate() {
                errors.push(e);
            }
        }

        // Check Health
        if let Some(health) = world.get::<Health>(entity) {
            if let Err(e) = health.validate() {
                errors.push(e);
            }
        }

        // Check Name
        if let Some(name) = world.get::<Name>(entity) {
            if let Err(e) = name.validate() {
                errors.push(e);
            }
        }

        // Check Renderable
        if let Some(renderable) = world.get::<Renderable>(entity) {
            if let Err(e) = renderable.validate() {
                errors.push(e);
            }
        }

        // Check interpolated components
        if let Some(interp_pos) = world.get::<InterpolatedPosition>(entity) {
            if let Err(e) = interp_pos.validate() {
                errors.push(e);
            }
        }

        if let Some(interp_health) = world.get::<InterpolatedHealth>(entity) {
            if let Err(e) = interp_health.validate() {
                errors.push(e);
            }
        }

        if let Some(interp_renderable) = world.get::<InterpolatedRenderable>(entity) {
            if let Err(e) = interp_renderable.validate() {
                errors.push(e);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Calculate total memory footprint for an entity's components
    pub fn entity_memory_footprint(world: &World, entity: Entity) -> usize {
        let mut total = 0;

        if let Some(position) = world.get::<Position>(entity) {
            total += position.memory_footprint();
        }
        if let Some(movement) = world.get::<Movement>(entity) {
            total += movement.memory_footprint();
        }
        if let Some(health) = world.get::<Health>(entity) {
            total += health.memory_footprint();
        }
        if let Some(owner) = world.get::<Owner>(entity) {
            total += owner.memory_footprint();
        }
        if let Some(name) = world.get::<Name>(entity) {
            total += name.memory_footprint();
        }
        if let Some(renderable) = world.get::<Renderable>(entity) {
            total += renderable.memory_footprint();
        }

        total
    }

    /// Reset all resettable components on an entity
    pub fn reset_entity_components(world: &mut World, entity: Entity) {
        if let Some(mut movement) = world.get_mut::<Movement>(entity) {
            movement.reset();
        }
        if let Some(mut health) = world.get_mut::<Health>(entity) {
            health.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::world::World;

    #[test]
    fn test_position_validation() {
        let pos = Position::new(100, 200).unwrap();
        assert!(pos.validate().is_ok());

        let invalid_pos = Position::new(20000, 20000);
        assert!(invalid_pos.is_err());
    }

    #[test]
    fn test_health_validation() {
        let health = Health::new(100.0).unwrap();
        assert!(health.validate().is_ok());

        let invalid_health = Health::with_values(-10.0, 100.0, 0.0, 0.0);
        assert!(invalid_health.is_err());
    }

    #[test]
    fn test_name_validation() {
        let name = Name::new("Valid Name".to_string()).unwrap();
        assert!(name.validate().is_ok());

        let empty_name = Name::new("".to_string());
        assert!(empty_name.is_err());
    }

    #[test]
    fn test_movement_operations() {
        let mut movement = Movement::new(2.0, 4, MovementType::Land).unwrap();
        assert!(movement.can_move(2));
        assert!(movement.use_moves(2).is_ok());
        assert_eq!(movement.remaining_moves, 2);
        assert!(!movement.can_move(3));
    }

    #[test]
    fn test_health_operations() {
        let mut health = Health::new(100.0).unwrap();
        assert!(health.take_damage(30.0).is_ok());
        assert_eq!(health.current, 70.0);
        assert!(health.heal(20.0).is_ok());
        assert_eq!(health.current, 90.0);
    }

    #[test]
    fn test_color_operations() {
        let red = Color::RED;
        let blue = Color::BLUE;
        let purple = red.lerp(&blue, 0.5);
        
        assert_eq!(purple.r, 0.5);
        assert_eq!(purple.b, 0.5);
    }
}
