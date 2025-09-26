//! Multi-layer stack for tiles
//!
//! Provides the TileLayerStack component which manages multiple layers
//! for a single tile with efficient lookups and layer management.

use arrayvec::ArrayVec;
use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use indexmap::IndexMap;

use crate::world::tiles::chunks::TileId;
use super::{
    feature::LayerFeature,
    layer::TileLayer,
    types::{LayerType, FeatureType},
    errors::LayerError,
};

/// Maximum number of layers per tile
pub const MAX_LAYERS: usize = 8;

/// Multi-layer system for a single tile
#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub struct TileLayerStack {
    /// All layers for this tile (fixed capacity)
    layers: ArrayVec<TileLayer, MAX_LAYERS>,
    /// Quick lookup by layer type
    layer_index: IndexMap<LayerType, usize>,
    /// Tile this stack belongs to
    tile_id: TileId,
    /// Global generation counter
    pub(crate) generation: u64,
}

impl TileLayerStack {
    /// Create new layer stack for tile
    pub fn new(tile_id: TileId) -> Self {
        Self {
            layers: ArrayVec::new(),
            layer_index: IndexMap::new(),
            tile_id,
            generation: 1,
        }
    }

    /// Add or get layer of specific type
    pub fn get_or_create_layer(&mut self, layer_type: LayerType) -> &mut TileLayer {
        if let Some(&index) = self.layer_index.get(&layer_type) {
            &mut self.layers[index]
        } else {
            let new_layer = TileLayer::new(layer_type);
            let index = self.layers.len();
            
            self.layers.push(new_layer);
            self.layer_index.insert(layer_type, index);
            self.generation += 1;
            
            &mut self.layers[index]
        }
    }

    /// Get layer by type
    pub fn get_layer(&self, layer_type: LayerType) -> Option<&TileLayer> {
        self.layer_index.get(&layer_type)
            .and_then(|&index| self.layers.get(index))
    }

    /// Get mutable layer by type
    pub fn get_layer_mut(&mut self, layer_type: LayerType) -> Option<&mut TileLayer> {
        if let Some(&index) = self.layer_index.get(&layer_type) {
            self.generation += 1;
            self.layers.get_mut(index)
        } else {
            None
        }
    }

    /// Add feature to appropriate layer
    pub fn add_feature(&mut self, feature: LayerFeature) -> Result<(), LayerError> {
        let layer_type = feature.feature_type.layer_type();
        let layer = self.get_or_create_layer(layer_type);
        layer.add_feature(feature)?;
        self.generation += 1;
        Ok(())
    }

    /// Remove feature by ID and layer type
    pub fn remove_feature(&mut self, layer_type: LayerType, feature_id: u32) -> Option<LayerFeature> {
        if let Some(layer) = self.get_layer_mut(layer_type) {
            let result = layer.remove_feature(feature_id);
            if result.is_some() {
                self.generation += 1;
            }
            result
        } else {
            None
        }
    }

    /// Find feature by ID across all layers
    pub fn find_feature(&self, feature_id: u32) -> Option<(LayerType, &LayerFeature)> {
        for layer in &self.layers {
            if let Some(feature) = layer.get_feature(feature_id) {
                return Some((layer.layer_type, feature));
            }
        }
        None
    }

    /// Get all layers
    pub fn layers(&self) -> &[TileLayer] {
        &self.layers
    }

    /// Get mutable reference to all layers
    pub fn layers_mut(&mut self) -> &mut [TileLayer] {
        &mut self.layers
    }

    /// Get layers sorted by render priority
    pub fn layers_by_priority(&self) -> Vec<&TileLayer> {
        let mut layers: Vec<_> = self.layers.iter().collect();
        layers.sort_by_key(|layer| layer.layer_type.render_priority());
        layers
    }

    /// Get active layers only
    pub fn active_layers(&self) -> impl Iterator<Item = &TileLayer> {
        self.layers.iter().filter(|layer| layer.active)
    }

    /// Check if any layer has significant features
    pub fn has_significant_features(&self) -> bool {
        self.layers.iter().any(|layer| layer.has_significant_features())
    }

    /// Get all features of a specific type across layers
    pub fn get_features_by_type(&self, feature_type: FeatureType) -> Vec<&LayerFeature> {
        let layer_type = feature_type.layer_type();
        if let Some(layer) = self.get_layer(layer_type) {
            layer.features_by_type(feature_type).collect()
        } else {
            Vec::new()
        }
    }

    /// Apply layer visibility settings
    pub fn set_layer_visibility(&mut self, layer_type: LayerType, visible: bool) {
        if let Some(layer) = self.get_layer_mut(layer_type) {
            layer.active = visible;
        }
    }

    /// Set layer opacity
    pub fn set_layer_opacity(&mut self, layer_type: LayerType, opacity: f32) {
        if let Some(layer) = self.get_layer_mut(layer_type) {
            layer.opacity = opacity.clamp(0.0, 1.0);
        }
    }

    /// Get combined layer intensity for a specific layer type
    pub fn get_layer_intensity(&self, layer_type: LayerType) -> f32 {
        self.get_layer(layer_type)
            .map(|layer| layer.total_intensity())
            .unwrap_or(0.0)
    }

    /// Get generation counter
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Get memory usage in bytes
    pub fn memory_size(&self) -> usize {
        std::mem::size_of::<Self>() +
        self.layers.iter().map(|l| l.memory_size()).sum::<usize>()
    }
}
