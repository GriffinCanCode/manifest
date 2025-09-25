//! Archetype-based entity storage system using slotmap for performance
//!
//! This module provides sophisticated archetype management for optimal ECS
//! performance with strong typing and minimal memory overhead.

pub mod index;
pub mod manager; 
pub mod query;
pub mod storage;
pub mod types;

// Re-export core types for convenience
pub use index::*;
pub use manager::*;
pub use query::*;
pub use storage::*;
pub use types::*;
