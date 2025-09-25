//! Component validation traits and error types
//!
//! Contains validation infrastructure and error handling for all components.

/// Validation trait for component constraints and business rules
pub trait Validate {
    type Error: std::error::Error + Send + Sync + 'static;
    
    /// Validate component state
    fn validate(&self) -> Result<(), Self::Error>;
    
    /// Get validation constraints as human-readable text
    fn constraints() -> &'static str;
}

/// Component error types for strong error handling
#[derive(Debug, thiserror::Error)]
pub enum ComponentError {
    #[error("Invalid position: {0}")]
    InvalidPosition(String),
    #[error("Invalid movement: {0}")]
    InvalidMovement(String),
    #[error("Invalid health: {0}")]
    InvalidHealth(String),
    #[error("Invalid name: {0}")]
    InvalidName(String),
    #[error("Invalid hierarchy: {0}")]
    InvalidHierarchy(String),
    #[error("Invalid owner: {0}")]
    InvalidOwner(String),
    #[error("Invalid renderable: {0}")]
    InvalidRenderable(String),
    #[error("Invalid interpolation: {0}")]
    InvalidInterpolation(String),
}

/// Result type for component operations
pub type ComponentResult<T> = Result<T, ComponentError>;

/// Utility functions for component validation
pub mod utils {
    use super::ComponentError;

    /// Validate that a name is not empty and within length limits
    pub fn validate_name(name: &str) -> Result<(), ComponentError> {
        if name.is_empty() {
            return Err(ComponentError::InvalidName("Name cannot be empty".to_string()));
        }
        
        if name.len() > 100 {
            return Err(ComponentError::InvalidName(
                format!("Name too long: {} characters (max 100)", name.len())
            ));
        }
        
        if name.chars().any(|c| c.is_control()) {
            return Err(ComponentError::InvalidName(
                "Name cannot contain control characters".to_string()
            ));
        }
        
        Ok(())
    }

    /// Validate coordinates are within world bounds
    pub fn validate_coordinates(q: i32, r: i32) -> Result<(), ComponentError> {
        const MAX_BOUND: i32 = 10000;
        
        if q.abs() > MAX_BOUND || r.abs() > MAX_BOUND {
            return Err(ComponentError::InvalidPosition(
                format!("Coordinates ({}, {}) exceed world bounds (±{})", q, r, MAX_BOUND)
            ));
        }
        
        Ok(())
    }

    /// Validate health values
    pub fn validate_health(current: f32, max: f32) -> Result<(), ComponentError> {
        if max <= 0.0 {
            return Err(ComponentError::InvalidHealth(
                format!("Max health must be positive, got {}", max)
            ));
        }
        
        if current < 0.0 {
            return Err(ComponentError::InvalidHealth(
                format!("Current health cannot be negative, got {}", current)
            ));
        }
        
        if current > max {
            return Err(ComponentError::InvalidHealth(
                format!("Current health ({}) cannot exceed max health ({})", current, max)
            ));
        }
        
        Ok(())
    }

    /// Validate movement parameters
    pub fn validate_movement(speed: f32, remaining_moves: u32, max_moves: u32) -> Result<(), ComponentError> {
        if speed < 0.0 {
            return Err(ComponentError::InvalidMovement(
                format!("Speed cannot be negative, got {}", speed)
            ));
        }
        
        if speed > 1000.0 {
            return Err(ComponentError::InvalidMovement(
                format!("Speed too high: {} (max 1000)", speed)
            ));
        }
        
        if remaining_moves > max_moves {
            return Err(ComponentError::InvalidMovement(
                format!("Remaining moves ({}) cannot exceed max moves ({})", remaining_moves, max_moves)
            ));
        }
        
        Ok(())
    }
}
