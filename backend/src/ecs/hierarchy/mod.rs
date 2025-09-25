//! Entity hierarchy and relationship system using petgraph
//!
//! Provides parent-child relationships, entity ownership chains, and graph-based
//! entity organization with fast lookups and traversal operations.

pub mod components;
pub mod graph;
pub mod queries;

#[cfg(test)]
pub mod tests;

// Re-export commonly used types
pub use components::*;
pub use graph::*;
pub use queries::*;
