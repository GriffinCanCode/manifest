//! Resource Distribution System
//!
//! Comprehensive resource distribution system using Lua configuration for:
//! - Resource types and properties
//! - Geological placement rules  
//! - Quality and quantity calculations
//! - Discovery and depletion mechanics
//! - Scarcity and market dynamics

pub mod core;
pub mod lua;
pub mod types;
pub mod distribution;
pub mod discovery;
pub mod depletion;

#[cfg(test)]
pub mod tests;

// Re-export commonly used types
pub use types::*;
pub use core::*;
pub use distribution::*;
pub use discovery::*;
pub use depletion::*;

use thiserror::Error;

/// Errors that can occur in the resource distribution system
#[derive(Error, Debug)]
pub enum ResourceDistributionError {
    #[error("Lua script error: {0}")]
    LuaScriptError(#[from] crate::scripting::ScriptError),
    
    #[error("Invalid resource type: {0}")]
    InvalidResourceType(String),
    
    #[error("Distribution rule not found: {0}")]
    RuleNotFound(String),
    
    #[error("Resource configuration error: {0}")]
    ConfigError(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Lua error: {0}")]
    LuaError(#[from] mlua::Error),
}

pub type ResourceResult<T> = Result<T, ResourceDistributionError>;
