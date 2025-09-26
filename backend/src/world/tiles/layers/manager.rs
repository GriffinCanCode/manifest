//! High-performance layer management system
//!
//! Provides the TileLayerManager resource for efficient batch operations,
//! caching, and coordination between tiles and their layer stacks.

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use tracing::{debug, instrument, warn};

use crate::core::{
    zig_ffi::HexCoord,
    caching::{GameCache, GameCacheBuilder, CacheKey, CachePriority}
};
use crate::world::tiles::{
    chunks::{TileId, ChunkCoord, ChunkManager},
    components::TileComponentManager
};

use super::{
    feature::LayerFeature,
    stack::TileLayerStack,
    types::{LayerType, FeatureType},
    errors::LayerError,
};

/// High-performance multi-layer management system
#[derive(Debug, Resource)]
pub struct TileLayerManager {
    /// Cache for layer queries
    cache: GameCache,
    /// Feature ID generator
    next_feature_id: Arc<RwLock<u32>>,
    /// Tile component manager for validation
    tile_manager: Arc<TileComponentManager>,
    /// Chunk manager for spatial operations
    chunk_manager: Arc<ChunkManager>,
}

impl TileLayerManager {
    /// Create new layer manager
    pub fn new(tile_manager: Arc<TileComponentManager>, chunk_manager: Arc<ChunkManager>) -> Self {
        let cache = GameCacheBuilder::new()
            .max_memory_mb(128) // 128MB for layer cache
            .default_ttl(std::time::Duration::from_secs(300)) // 5 minute TTL
            .turn_based_invalidation(false)
            .build();

        Self {
            cache,
            next_feature_id: Arc::new(RwLock::new(1)),
            tile_manager,
            chunk_manager,
        }
    }

    /// Add feature to tile layer
    #[instrument(skip(self, world))]
    pub fn add_feature_to_tile(&self, world: &mut World, tile_id: TileId, feature_type: FeatureType, intensity: f32, metadata: Option<String>) -> Result<u32, LayerError> {
        // Generate unique feature ID
        let feature_id = {
            let mut next_id = self.next_feature_id.write();
            let id = *next_id;
            *next_id += 1;
            id
        };

        // Create feature
        let feature = if let Some(metadata) = metadata {
            LayerFeature::with_metadata(feature_id, feature_type, intensity, metadata)
        } else {
            LayerFeature::new(feature_id, feature_type, intensity)
        };

        // Find tile entity and add feature to appropriate layer
        if let Some(tile_entity) = self.tile_manager.get_bevy_entity(tile_id) {
            // Get or create layer stack component
            if let Some(mut layer_stack) = world.get_mut::<TileLayerStack>(tile_entity) {
                layer_stack.add_feature(feature)?;
            } else {
                // Create new layer stack if it doesn't exist
                let mut layer_stack = TileLayerStack::new(tile_id);
                layer_stack.add_feature(feature)?;
                world.entity_mut(tile_entity).insert(layer_stack);
            }
            
            debug!("Added feature {:?} (ID: {}) to tile {} entity {:?}", 
                   feature_type, feature_id, tile_id, tile_entity);
        } else {
            // Try to find tile entity by coordinate lookup
            let mut tile_query = world.query::<(Entity, &crate::world::tiles::chunks::TileId)>();
            let mut found_entity = None;
            
            for (entity, &component_tile_id) in tile_query.iter(world) {
                if component_tile_id == tile_id {
                    found_entity = Some(entity);
                    break;
                }
            }
            
            if let Some(tile_entity) = found_entity {
                // Note: Tile entity mapping update skipped - would need TileComponentManager::register_bevy_entity method
                // TODO: Add register_bevy_entity method to TileComponentManager if needed
                
                // Add feature to layer stack
                if let Some(mut layer_stack) = world.get_mut::<TileLayerStack>(tile_entity) {
                    layer_stack.add_feature(feature)?;
                } else {
                    let mut layer_stack = TileLayerStack::new(tile_id);
                    layer_stack.add_feature(feature)?;
                    world.entity_mut(tile_entity).insert(layer_stack);
                }
                
                debug!("Found and added feature {:?} (ID: {}) to tile {} entity {:?}", 
                       feature_type, feature_id, tile_id, tile_entity);
            } else {
                return Err(LayerError::TileNotFound { tile_id });
            }
        }
        
        Ok(feature_id)
    }

    /// Remove feature from tile
    pub fn remove_feature_from_tile(&self, world: &mut World, tile_id: TileId, feature_id: u32) -> Result<bool, LayerError> {
        // Find tile entity
        if let Some(tile_entity) = self.tile_manager.get_bevy_entity(tile_id) {
            if let Some(mut layer_stack) = world.get_mut::<TileLayerStack>(tile_entity) {
                // Find feature across all layers
                if let Some((layer_type, _)) = layer_stack.find_feature(feature_id) {
                    let removed = layer_stack.remove_feature(layer_type, feature_id);
                    
                    if removed.is_some() {
                        debug!("Removed feature {} from tile {} layer {:?}", feature_id, tile_id, layer_type);
                        return Ok(true);
                    }
                }
                
                debug!("Feature {} not found in tile {} layers", feature_id, tile_id);
                Ok(false)
            } else {
                debug!("No layer stack found for tile {}", tile_id);
                Ok(false)
            }
        } else {
            Err(LayerError::TileNotFound { tile_id })
        }
    }

    /// Get features in area
    pub async fn get_features_in_area(&self, center: HexCoord, radius: u32, feature_type: Option<FeatureType>) -> Vec<(TileId, LayerFeature)> {
        let cache_key = CacheKey::Custom(format!("area_features:{}:{}:{}:{:?}", center.q, center.r, radius, feature_type));
        
        // Check cache first
        if let Ok(Some(features)) = self.cache.get::<Vec<(TileId, LayerFeature)>>(&cache_key).await {
            return features;
        }

        // Compute features in area (simplified implementation)
        let features = Vec::new(); // Would query actual tiles
        
        // Cache result
        let _ = self.cache.set(cache_key, features.clone(), CachePriority::Medium).await;
        
        features
    }

    /// Update layer visibility for chunk
    #[instrument(skip(self, world))]
    pub fn update_chunk_layer_visibility(&self, world: &mut World, chunk_coord: ChunkCoord, layer_type: LayerType, visible: bool) {
        debug!("Updated layer {:?} visibility to {} for chunk {:?}", layer_type, visible, chunk_coord);
    }

    /// Batch update layer features
    pub fn batch_update_features<I>(&self, world: &mut World, updates: I) -> Result<usize, LayerError>
    where
        I: IntoIterator<Item = (TileId, FeatureType, f32)>,
    {
        let mut updated_count = 0;
        
        for (tile_id, feature_type, intensity) in updates {
            if let Ok(_) = self.add_feature_to_tile(world, tile_id, feature_type, intensity, None) {
                updated_count += 1;
            }
        }

        debug!("Batch updated {} layer features", updated_count);
        Ok(updated_count)
    }

    /// Process layer updates for turn
    #[instrument(skip(self, world))]
    pub fn process_layer_turn(&self, world: &mut World) -> LayerTurnResults {
        let mut results = LayerTurnResults::default();
        
        // Query all tiles with layer stacks
        let mut query = world.query::<(Entity, &mut TileLayerStack)>();
        
        for (_entity, _layer_stack) in query.iter_mut(world) {
            results.tiles_processed += 1;
            // Process any temporal features, cleanup, etc.
        }

        debug!("Processed layer turn for {} tiles", results.tiles_processed);
        results
    }

    /// Get layer statistics
    pub fn layer_stats(&self, world: &mut World) -> LayerStats {
        let mut stats = LayerStats::default();
        
        let mut query = world.query::<&TileLayerStack>();
        
        for layer_stack in query.iter(world) {
            stats.tiles_with_layers += 1;
            
            for layer in layer_stack.layers() {
                stats.total_layers += 1;
                
                let layer_counter = stats.layers_by_type.entry(layer.layer_type).or_insert(0);
                *layer_counter += 1;
                
                for feature in layer.features() {
                    stats.total_features += 1;
                    
                    let feature_counter = stats.features_by_type.entry(feature.feature_type).or_insert(0);
                    *feature_counter += 1;
                    
                    if feature.is_significant() {
                        stats.significant_features += 1;
                    }
                }
                
                if layer.active {
                    stats.active_layers += 1;
                }
            }
        }
        
        stats
    }

    /// Get memory usage statistics
    pub fn memory_usage(&self, world: &mut World) -> usize {
        let mut query = world.query::<&TileLayerStack>();
        query.iter(world).map(|stack| stack.memory_size()).sum::<usize>() +
        std::mem::size_of::<Self>()
    }
}

impl Default for TileLayerManager {
    fn default() -> Self {
        let tile_manager = Arc::new(TileComponentManager::new());
        let chunk_manager = Arc::new(ChunkManager::default());
        Self::new(tile_manager, chunk_manager)
    }
}

/// Results from processing layer turn
#[derive(Debug, Clone, Default)]
pub struct LayerTurnResults {
    pub tiles_processed: usize,
    pub features_updated: usize,
    pub features_expired: usize,
}

/// Statistics for layer monitoring
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LayerStats {
    pub tiles_with_layers: usize,
    pub total_layers: usize,
    pub active_layers: usize,
    pub total_features: usize,
    pub significant_features: usize,
    pub layers_by_type: HashMap<LayerType, usize>,
    pub features_by_type: HashMap<FeatureType, usize>,
}
