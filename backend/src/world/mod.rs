//! World-level systems and data structures
//!
//! This module contains systems that operate on the entire game world,
//! including tiles, terrain, procedural generation, and large-scale spatial organization.

pub mod tiles;
pub mod generation;

// Re-export commonly used types
pub use tiles::*;
pub use generation::*;
