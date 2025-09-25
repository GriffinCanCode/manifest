//! World management subsystems
//!
//! This module contains focused manager structs that handle different aspects
//! of world management, allowing GameWorld to delegate specialized responsibilities.

pub mod world_manager;
pub mod system_coordinator;
pub mod subsystem_registry;

// Re-export for convenience
pub use world_manager::*;
pub use system_coordinator::*;
pub use subsystem_registry::*;
