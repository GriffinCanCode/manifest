//! World-level systems and data structures
//!
//! This module contains systems that operate on the entire game world,
//! including tiles, terrain, and large-scale spatial organization.

pub mod tiles;

// Re-export commonly used types
pub use tiles::*;
