//! Modular ECS World management
//!
//! This module has been refactored from a large monolithic file into focused submodules:
//! - `core`: Main GameWorld struct and initialization
//! - `update`: Update loops and time management
//! - `entities`: Entity spawning, despawning, and lifecycle
//! - `spatial`: Spatial queries and indexing
//! - `hierarchy`: Hierarchical entity relationships
//! - `caching`: Cache management and invalidation
//! - `initialization`: Game setup and terrain generation
//! - `serialization`: Save/load world state
//! - `hot_reload`: Development hot reload functionality

pub mod core;
pub mod update;
pub mod entities;
pub mod spatial;
pub mod hierarchy;
pub mod caching;
pub mod initialization;
pub mod serialization;
pub mod hot_reload;
pub mod managers;

// Re-export the main GameWorld struct and commonly used types
pub use core::GameWorld;
pub use caching::{CacheStatistics, QueryCacheStats, ArchetypeCacheStats};
pub use managers::{WorldManager, SystemCoordinator, SubsystemRegistry};

// Re-export for convenience

// Convenient type aliases for frequently used error types
pub type WorldResult<T> = Result<T, WorldError>;

/// Errors that can occur during world operations
#[derive(Debug, thiserror::Error)]
pub enum WorldError {
    #[error("Entity not found: {0:?}")]
    EntityNotFound(bevy_ecs::entity::Entity),
    
    #[error("Component not found: {0}")]
    ComponentNotFound(String),
    
    #[error("Resource not found: {0}")]
    ResourceNotFound(String),
    
    #[error("Hierarchy error: {0}")]
    HierarchyError(#[from] crate::ecs::hierarchy::HierarchyError),
    
    #[error("Save/load error: {0}")]
    SaveError(#[from] crate::ecs::persistence::SaveError),
    
    #[error("Cache error: {0}")]
    CacheError(String),
    
    #[error("Spatial index error: {0}")]
    SpatialError(String),
    
    #[error("System execution error: {0}")]
    SystemError(String),
}
