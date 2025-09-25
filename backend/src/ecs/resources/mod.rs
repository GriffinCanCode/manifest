//! Resource management system for the ECS
//!
//! This module provides both resource definitions (GameTime, Players, etc.)
//! and a thread-safe resource manager for controlled access.

pub mod definitions;
// pub mod manager; // Temporarily disabled due to lifetime issues

// Re-export resource definitions for convenience
pub use definitions::*;

// Re-export resource manager types when fixed
// pub use manager::{ResourceManager, ResourceError, ResourceReadGuard, ResourceWriteGuard};
