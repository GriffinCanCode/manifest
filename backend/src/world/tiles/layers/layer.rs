//! Single tile layer implementation
//!
//! Provides the TileLayer struct which represents a single layer containing
//! features of a specific type with efficient fixed-capacity storage.

use arrayvec::ArrayVec;
use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

use super::{
    feature::LayerFeature,
    types::{LayerType, FeatureType},
    errors::{LayerError, MAX_LAYER_FEATURES},
};

/// Single layer containing features of a specific type
#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub struct TileLayer {
    /// Type of this layer
    pub layer_type: LayerType,
    /// Features in this layer (fixed capacity for performance)
    features: ArrayVec<LayerFeature, MAX_LAYER_FEATURES>,
    /// Whether layer is currently active/visible
    pub active: bool,
    /// Layer opacity for rendering (0.0 to 1.0)
    pub opacity: f32,
    /// Generation counter for change tracking
    generation: u64,
}

impl TileLayer {
    /// Create new empty layer
    pub fn new(layer_type: LayerType) -> Self {
        Self {
            layer_type,
            features: ArrayVec::new(),
            active: true,
            opacity: 1.0,
            generation: 1,
        }
    }

    /// Add feature to layer
    pub fn add_feature(&mut self, feature: LayerFeature) -> Result<(), LayerError> {
        // Check if feature type matches layer
        if feature.feature_type.layer_type() != self.layer_type {
            return Err(LayerError::FeatureMismatch { 
                feature_type: feature.feature_type, 
                layer_type: self.layer_type 
            });
        }

        // Check capacity
        if self.features.is_full() {
            return Err(LayerError::LayerCapacityExceeded);
        }

        // Check for existing feature with same ID
        if self.features.iter().any(|f| f.id == feature.id) {
            return Err(LayerError::DuplicateFeature { id: feature.id });
        }

        self.features.push(feature);
        self.generation += 1;
        Ok(())
    }

    /// Remove feature by ID
    pub fn remove_feature(&mut self, feature_id: u32) -> Option<LayerFeature> {
        if let Some(pos) = self.features.iter().position(|f| f.id == feature_id) {
            let feature = self.features.swap_remove(pos);
            self.generation += 1;
            Some(feature)
        } else {
            None
        }
    }

    /// Update existing feature
    pub fn update_feature(&mut self, feature_id: u32, intensity: f32, metadata: Option<String>) -> Result<(), LayerError> {
        if let Some(feature) = self.features.iter_mut().find(|f| f.id == feature_id) {
            feature.intensity = intensity.clamp(0.0, 1.0);
            feature.metadata = metadata;
            feature.last_modified += 1; // Would be set to actual turn in real implementation
            self.generation += 1;
            Ok(())
        } else {
            Err(LayerError::FeatureNotFound { id: feature_id })
        }
    }

    /// Get feature by ID
    pub fn get_feature(&self, feature_id: u32) -> Option<&LayerFeature> {
        self.features.iter().find(|f| f.id == feature_id)
    }

    /// Get all features
    pub fn features(&self) -> &[LayerFeature] {
        &self.features
    }

    /// Get mutable access to all features
    pub fn features_mut(&mut self) -> &mut [LayerFeature] {
        &mut self.features
    }

    /// Get features by type
    pub fn features_by_type(&self, feature_type: FeatureType) -> impl Iterator<Item = &LayerFeature> {
        self.features.iter().filter(move |f| f.feature_type == feature_type)
    }

    /// Get significant features (above threshold)
    pub fn significant_features(&self) -> impl Iterator<Item = &LayerFeature> {
        self.features.iter().filter(|f| f.is_significant())
    }

    /// Check if layer has any significant features
    pub fn has_significant_features(&self) -> bool {
        self.features.iter().any(|f| f.is_significant())
    }

    /// Get total intensity of all features
    pub fn total_intensity(&self) -> f32 {
        self.features.iter().map(|f| f.intensity).sum()
    }

    /// Get layer generation for change detection
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Set layer generation (for manual change tracking)
    pub fn set_generation(&mut self, generation: u64) {
        self.generation = generation;
    }

    /// Increment layer generation
    pub fn increment_generation(&mut self) {
        self.generation += 1;
    }

    /// Clear all features from layer
    pub fn clear(&mut self) {
        self.features.clear();
        self.generation += 1;
    }

    /// Get memory usage in bytes
    pub fn memory_size(&self) -> usize {
        std::mem::size_of::<Self>() +
        self.features.iter().map(|f| f.memory_size()).sum::<usize>()
    }
}
