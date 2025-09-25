//! ECS Persistence module  
//!
//! Contains all functionality related to saving, loading, and serializing game state.
//! This module is organized into:
//! - `saves`: Main save/load system with high-performance serialization
//! - `entity_serialization`: Utilities for serializing individual entities
//! - `world_state`: World state representation for persistence

pub mod saves;
pub mod entity_serialization;
pub mod world_state;

// Re-export commonly used types and functions
pub use saves::{
    SaveSystem, SaveFile, SaveMetadata, SaveInfo, SaveError,
    save_world_state_to_file, load_world_state_from_file,
    list_available_saves, delete_save_file, GameTimeExt
};

pub use entity_serialization::{
    serialize_entity, deserialize_entity, serialize_entity_with_components
};

pub use world_state::{
    WorldState, SerializedEntity
};
