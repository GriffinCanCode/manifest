//! ECS Systems module
//!
//! Contains all system implementations and scheduling infrastructure for the game.
//! This module is organized into:
//! - `changes`: Change detection and unified change processing system
//! - `schedule`: ECS scheduler and resource management
//! - `core`: Core game systems (time, interpolation, camera, etc.)

pub mod changes;
pub mod schedule; 
pub mod core;

// Re-export commonly used types and functions
pub use changes::{
    ChangeMonitor, ChangeStats, ChangeDetectionExt, ChangeDetectionUtils,
    unified_change_system, configure_change_detection
};

pub use schedule::{
    EcsScheduler, EcsTask, ResourceAccess, Access, ResourceSpecBuilder, SystemResources
};

pub use core::{
    time_system, interpolation_system, selection_validation_system,
    turn_advancement_system, camera_system, CameraInputState, GameSystemSet,
    configure_systems, configure_parallel_systems
};
