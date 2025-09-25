//! Error types for the layer system
//!
//! Defines all error conditions that can occur during layer operations
//! including feature management, capacity limits, and validation errors.

use super::types::{LayerType, FeatureType};
use crate::world::tiles::chunks::TileId;

/// Maximum number of features per layer
pub const MAX_LAYER_FEATURES: usize = 16;

/// Layer system errors
#[derive(Debug, thiserror::Error)]
pub enum LayerError {
    #[error("Feature type {feature_type:?} does not match layer type {layer_type:?}")]
    FeatureMismatch { feature_type: FeatureType, layer_type: LayerType },
    
    #[error("Layer capacity exceeded (max {MAX_LAYER_FEATURES} features per layer)")]
    LayerCapacityExceeded,
    
    #[error("Duplicate feature ID: {id}")]
    DuplicateFeature { id: u32 },
    
    #[error("Feature not found: {id}")]
    FeatureNotFound { id: u32 },
    
    #[error("Tile not found: {tile_id}")]
    TileNotFound { tile_id: TileId },
    
    #[error("Invalid intensity value: {intensity} (must be 0.0 to 1.0)")]
    InvalidIntensity { intensity: f32 },
}
