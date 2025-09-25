//! Individual layer features with metadata and intensity tracking
//!
//! Provides the LayerFeature struct which represents individual features
//! within layers, including their properties, metadata, and temporal aspects.

use serde::{Deserialize, Serialize};
use super::types::FeatureType;

/// Individual feature within a layer
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerFeature {
    /// Unique identifier for this feature
    pub id: u32,
    /// Type of feature
    pub feature_type: FeatureType,
    /// Intensity/strength of feature (0.0 to 1.0)
    pub intensity: f32,
    /// Optional metadata for the feature
    pub metadata: Option<String>,
    /// Turn when feature was created/last modified
    pub last_modified: u32,
}

impl LayerFeature {
    /// Create new layer feature
    pub fn new(id: u32, feature_type: FeatureType, intensity: f32) -> Self {
        Self {
            id,
            feature_type,
            intensity: intensity.clamp(0.0, 1.0),
            metadata: None,
            last_modified: 0, // Would be set by game logic
        }
    }

    /// Create feature with metadata
    pub fn with_metadata(id: u32, feature_type: FeatureType, intensity: f32, metadata: String) -> Self {
        Self {
            id,
            feature_type,
            intensity: intensity.clamp(0.0, 1.0),
            metadata: Some(metadata),
            last_modified: 0,
        }
    }

    /// Check if feature is significant (above threshold)
    pub fn is_significant(&self) -> bool {
        self.intensity >= 0.1
    }

    /// Get memory usage in bytes
    pub fn memory_size(&self) -> usize {
        std::mem::size_of::<Self>() + 
        self.metadata.as_ref().map_or(0, |s| s.len())
    }
}
