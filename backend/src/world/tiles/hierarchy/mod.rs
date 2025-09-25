//! Hierarchical tiles module
//!
//! Extends the existing hierarchy system to support tile-specific relationships,
//! multi-resolution tile organization, and spatial hierarchy queries using the
//! established ECS relationship components and graph infrastructure.

pub mod types;
pub mod manager;
pub mod queries;
pub mod systems;

// Re-export commonly used types and functions
pub use types::{
    TileRelationshipType, HierarchicalTile, HexBounds,
    TileHierarchyStats, TileHierarchyValidation
};

pub use manager::TileHierarchy;

pub use systems::{
    maintain_tile_hierarchy_system,
    cleanup_tile_hierarchy_system,
    monitor_tile_hierarchy_system,
    validate_tile_hierarchy_system,
    TileHierarchySystemSet,
};

// Convenient type aliases
pub type HierarchyResult<T> = crate::ecs::hierarchy::HierarchyResult<T>;
pub type HierarchyError = crate::ecs::hierarchy::HierarchyError;
