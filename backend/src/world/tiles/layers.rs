//! Multi-layer system with arrayvec fixed arrays for efficient layer management
//!
//! Provides a sophisticated multi-layer system for tiles using fixed-capacity
//! arrays via arrayvec for memory efficiency and cache-friendly access patterns.

use arrayvec::ArrayVec;
use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use indexmap::IndexMap;

use crate::core::{
    zig_ffi::HexCoord,
    caching::{GameCache, GameCacheBuilder, CacheKey, CachePriority}
};
use crate::world::tiles::{
    chunks::{TileId, ChunkCoord, ChunkManager},
    components::TileComponentManager
};
use tracing::{debug, instrument, warn};

/// Maximum number of layers per tile
pub const MAX_LAYERS: usize = 8;

/// Maximum number of features per layer
pub const MAX_LAYER_FEATURES: usize = 16;

/// Layer types for organizing different aspects of tile data
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum LayerType {
    /// Base terrain and elevation
    Terrain = 0,
    /// Natural resources and deposits
    Resources = 1,
    /// Political boundaries and ownership
    Political = 2,
    /// Cultural influences and zones
    Cultural = 3,
    /// Religious presence and holy sites
    Religious = 4,
    /// Military presence and fortifications
    Military = 5,
    /// Economic networks and trade
    Economic = 6,
    /// Environmental effects and climate
    Environmental = 7,
}

impl LayerType {
    /// Get all layer types
    pub const fn all() -> &'static [LayerType] {
        &[
            LayerType::Terrain, LayerType::Resources, LayerType::Political,
            LayerType::Cultural, LayerType::Religious, LayerType::Military,
            LayerType::Economic, LayerType::Environmental,
        ]
    }

    /// Get layer priority (lower values render first)
    pub fn render_priority(self) -> u8 {
        self as u8
    }

    /// Check if layer affects gameplay mechanics
    pub fn is_gameplay_layer(self) -> bool {
        match self {
            LayerType::Terrain | LayerType::Resources | LayerType::Political | LayerType::Military => true,
            _ => false,
        }
    }

    /// Check if layer is purely visual/informational
    pub fn is_visual_layer(self) -> bool {
        !self.is_gameplay_layer()
    }
}

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

/// Types of features that can exist in layers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u16)]
pub enum FeatureType {
    // Terrain features
    River = 0,
    Forest = 1,
    Mountain = 2,
    Hill = 3,
    Desert = 4,
    Oasis = 5,
    Volcano = 6,
    Canyon = 7,
    
    // Resource features
    IronDeposit = 100,
    GoldVein = 101,
    OilField = 102,
    CoalMine = 103,
    Quarry = 104,
    FertileSoil = 105,
    FishingGrounds = 106,
    HuntingGrounds = 107,
    
    // Political features
    NationalBorder = 200,
    ProvinceBorder = 201,
    CityLimits = 202,
    MilitaryZone = 203,
    DemilitarizedZone = 204,
    TradeZone = 205,
    NaturalPark = 206,
    
    // Cultural features
    CulturalSite = 300,
    HistoricalSite = 301,
    ArtisticCenter = 302,
    EducationalHub = 303,
    CulturalBoundary = 304,
    LanguageZone = 305,
    TraditionArea = 306,
    
    // Religious features
    HolySite = 400,
    Temple = 401,
    Shrine = 402,
    Pilgrimage = 403,
    ReligiousBoundary = 404,
    Monastery = 405,
    Cemetery = 406,
    
    // Military features
    Fortress = 500,
    Barracks = 501,
    Watchtower = 502,
    Battlefield = 503,
    StrategicPoint = 504,
    SupplyDepot = 505,
    DefensiveLine = 506,
    
    // Economic features
    TradeRoute = 600,
    Market = 601,
    TradingPost = 602,
    Caravan = 603,
    Port = 604,
    Workshop = 605,
    Guild = 606,
    
    // Environmental features
    Pollution = 700,
    Radiation = 701,
    Disease = 702,
    ClimateZone = 703,
    Weather = 704,
    Disaster = 705,
    Restoration = 706,
}

impl FeatureType {
    /// Get the layer type this feature belongs to
    pub fn layer_type(self) -> LayerType {
        match (self as u16) / 100 {
            0 => LayerType::Terrain,
            1 => LayerType::Resources,
            2 => LayerType::Political,
            3 => LayerType::Cultural,
            4 => LayerType::Religious,
            5 => LayerType::Military,
            6 => LayerType::Economic,
            7 => LayerType::Environmental,
            _ => LayerType::Environmental, // Default fallback
        }
    }

    /// Check if feature affects tile properties
    pub fn affects_tile_properties(self) -> bool {
        match self.layer_type() {
            LayerType::Terrain | LayerType::Resources | LayerType::Environmental => true,
            _ => false,
        }
    }

    /// Get base influence radius for this feature type
    pub fn influence_radius(self) -> u8 {
        match self {
            // Local features
            FeatureType::River | FeatureType::Forest | FeatureType::IronDeposit => 1,
            
            // Regional features
            FeatureType::Mountain | FeatureType::Volcano | FeatureType::CityLimits => 2,
            
            // Large-scale features
            FeatureType::Desert | FeatureType::NationalBorder | FeatureType::TradeRoute => 3,
            
            // Minimal influence
            _ => 0,
        }
    }
}

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
    generation: u64,
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
                // Update tile manager mapping
                self.tile_manager.register_bevy_entity(tile_id, tile_entity);
                
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

/// System for processing layer turns
pub fn process_layers_system(
    layer_manager: Res<TileLayerManager>,
    mut layers_query: Query<(Entity, &mut TileLayerStack)>,
    game_state: Res<crate::core::game_state::CoreGameState>,
) {
    let current_turn = game_state.turn;
    let mut processed_tiles = 0;
    let mut features_updated = 0;
    let mut features_expired = 0;
    
    // Process each tile's layer stack
    for (entity, mut layer_stack) in layers_query.iter_mut() {
        processed_tiles += 1;
        let stack_generation_before = layer_stack.generation();
        
        // Process temporal features and cleanup
        for layer in layer_stack.layers.iter_mut() {
            let mut expired_features = Vec::new();
            
            // Check for expired features (if they have duration metadata)
            for (index, feature) in layer.features.iter().enumerate() {
                if let Some(ref metadata) = feature.metadata {
                    // Parse expiration from metadata (format: "expires:turn_number")
                    if let Some(expires_str) = metadata.strip_prefix("expires:") {
                        if let Ok(expire_turn) = expires_str.parse::<u32>() {
                            if current_turn >= expire_turn {
                                expired_features.push(index);
                            }
                        }
                    }
                }
                
                // Check for feature intensity decay (temporal features fade)
                if feature.feature_type.affects_tile_properties() && feature.intensity < 0.1 {
                    expired_features.push(index);
                }
            }
            
            // Remove expired features (in reverse order to preserve indices)
            for &index in expired_features.iter().rev() {
                if index < layer.features.len() {
                    layer.features.swap_remove(index);
                    layer.generation += 1;
                    features_expired += 1;
                }
            }
            
            // Update feature intensities for temporal effects
            for feature in layer.features.iter_mut() {
                let old_intensity = feature.intensity;
                
                // Apply time-based intensity decay for certain feature types
                match feature.feature_type {
                    FeatureType::Pollution | FeatureType::Radiation | FeatureType::Disease => {
                        // Environmental hazards decay over time
                        feature.intensity *= 0.95; // 5% decay per turn
                        if feature.intensity < 0.01 {
                            feature.intensity = 0.0;
                        }
                    },
                    FeatureType::Weather => {
                        // Weather patterns change
                        feature.intensity *= 0.8; // 20% decay per turn for weather
                    },
                    FeatureType::Caravan | FeatureType::Pilgrimage => {
                        // Moving features have dynamic intensity
                        let turn_age = current_turn.saturating_sub(feature.last_modified);
                        if turn_age > 5 {
                            feature.intensity *= 0.9; // Fade after 5 turns
                        }
                    },
                    _ => {
                        // Most features are stable
                    }
                }
                
                if old_intensity != feature.intensity {
                    feature.last_modified = current_turn;
                    layer.generation += 1;
                    features_updated += 1;
                }
            }
        }
        
        // Update stack generation if any layers changed
        if layer_stack.generation() != stack_generation_before {
            layer_stack.generation += 1;
        }
    }
    
    // Log processing results periodically
    if current_turn % 10 == 0 || features_expired > 0 {
        debug!("Layer processing (Turn {}): {} tiles, {} features updated, {} expired", 
               current_turn, processed_tiles, features_updated, features_expired);
    }
}

/// System for updating layer visibility based on player settings
pub fn update_layer_visibility_system(
    layer_manager: Res<TileLayerManager>,
    mut layers_query: Query<&mut TileLayerStack>,
    game_state: Res<crate::core::game_state::CoreGameState>,
    // In a real implementation, would have player preference queries
) {
    // Only update visibility settings periodically to avoid overhead
    if game_state.tick % 600 != 0 {  // ~10 seconds at 60 FPS
        return;
    }
    
    let current_turn = game_state.turn;
    let mut updated_stacks = 0;
    
    // Update layer visibility based on game state and player preferences
    for mut layer_stack in layers_query.iter_mut() {
        let mut changed = false;
        
        // Apply automatic visibility rules based on game state
        for layer_type in LayerType::all() {
            let should_be_visible = match layer_type {
                LayerType::Terrain => true, // Always visible
                LayerType::Resources => true, // Always visible
                LayerType::Political => true, // Always visible for strategy games
                LayerType::Military => true, // Always visible for strategy games
                LayerType::Cultural => current_turn > 10, // Show after early game
                LayerType::Religious => current_turn > 20, // Show after religions develop
                LayerType::Economic => current_turn > 5, // Show after economy develops
                LayerType::Environmental => true, // Show environmental effects
            };
            
            // Update layer visibility and opacity based on importance
            if let Some(layer) = layer_stack.get_layer_mut(*layer_type) {
                let was_active = layer.active;
                layer.active = should_be_visible;
                
                // Set opacity based on layer type and game state
                let target_opacity = if should_be_visible {
                    match layer_type {
                        LayerType::Terrain | LayerType::Resources => 1.0,
                        LayerType::Political | LayerType::Military => 0.8,
                        LayerType::Cultural | LayerType::Religious => 0.6,
                        LayerType::Economic => 0.7,
                        LayerType::Environmental => 0.5,
                    }
                } else {
                    0.0
                };
                
                if (layer.opacity - target_opacity).abs() > 0.01 {
                    layer.opacity = target_opacity;
                    changed = true;
                }
                
                if was_active != layer.active {
                    changed = true;
                    debug!("Layer {:?} visibility changed to {} for tile", layer_type, layer.active);
                }
            }
        }
        
        if changed {
            updated_stacks += 1;
        }
    }
    
    if updated_stacks > 0 {
        debug!("Updated layer visibility for {} tile stacks on turn {}", 
               updated_stacks, current_turn);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layer_types() {
        assert_eq!(LayerType::Terrain.render_priority(), 0);
        assert_eq!(LayerType::Environmental.render_priority(), 7);
        
        assert!(LayerType::Terrain.is_gameplay_layer());
        assert!(LayerType::Cultural.is_visual_layer());
    }

    #[test]
    fn test_feature_types() {
        assert_eq!(FeatureType::River.layer_type(), LayerType::Terrain);
        assert_eq!(FeatureType::IronDeposit.layer_type(), LayerType::Resources);
        assert_eq!(FeatureType::NationalBorder.layer_type(), LayerType::Political);
        
        assert_eq!(FeatureType::River.influence_radius(), 1);
        assert_eq!(FeatureType::Mountain.influence_radius(), 2);
        assert_eq!(FeatureType::Desert.influence_radius(), 3);
    }

    #[test]
    fn test_layer_feature() {
        let feature = LayerFeature::new(123, FeatureType::River, 0.8);
        
        assert_eq!(feature.id, 123);
        assert_eq!(feature.feature_type, FeatureType::River);
        assert_eq!(feature.intensity, 0.8);
        assert!(feature.is_significant());
        
        // Test intensity clamping
        let clamped_feature = LayerFeature::new(456, FeatureType::Forest, 1.5);
        assert_eq!(clamped_feature.intensity, 1.0);
    }

    #[test]
    fn test_tile_layer() {
        let mut layer = TileLayer::new(LayerType::Terrain);
        
        // Test adding feature
        let feature = LayerFeature::new(1, FeatureType::River, 0.7);
        assert!(layer.add_feature(feature).is_ok());
        assert_eq!(layer.features().len(), 1);
        
        // Test feature mismatch
        let wrong_feature = LayerFeature::new(2, FeatureType::IronDeposit, 0.5); // Resources, not Terrain
        assert!(layer.add_feature(wrong_feature).is_err());
        
        // Test feature retrieval
        assert!(layer.get_feature(1).is_some());
        assert!(layer.get_feature(999).is_none());
        
        // Test removal
        let removed = layer.remove_feature(1);
        assert!(removed.is_some());
        assert_eq!(layer.features().len(), 0);
    }

    #[test]
    fn test_tile_layer_stack() {
        let mut stack = TileLayerStack::new(123);
        
        // Test adding features to different layers
        let terrain_feature = LayerFeature::new(1, FeatureType::River, 0.6);
        let resource_feature = LayerFeature::new(2, FeatureType::IronDeposit, 0.8);
        
        assert!(stack.add_feature(terrain_feature).is_ok());
        assert!(stack.add_feature(resource_feature).is_ok());
        
        // Should have created 2 layers
        assert_eq!(stack.layers().len(), 2);
        
        // Test layer retrieval
        assert!(stack.get_layer(LayerType::Terrain).is_some());
        assert!(stack.get_layer(LayerType::Resources).is_some());
        assert!(stack.get_layer(LayerType::Political).is_none());
        
        // Test feature finding
        let (layer_type, feature) = stack.find_feature(1).unwrap();
        assert_eq!(layer_type, LayerType::Terrain);
        assert_eq!(feature.id, 1);
        
        // Test layer priorities
        let layers_by_priority = stack.layers_by_priority();
        assert_eq!(layers_by_priority[0].layer_type, LayerType::Terrain);
        assert_eq!(layers_by_priority[1].layer_type, LayerType::Resources);
    }

    #[test]
    fn test_layer_capacity() {
        let mut layer = TileLayer::new(LayerType::Terrain);
        
        // Fill up to capacity
        for i in 0..MAX_LAYER_FEATURES {
            let feature = LayerFeature::new(i as u32, FeatureType::River, 0.5);
            assert!(layer.add_feature(feature).is_ok());
        }
        
        // Should be at capacity
        assert_eq!(layer.features().len(), MAX_LAYER_FEATURES);
        
        // Adding one more should fail
        let extra_feature = LayerFeature::new(999, FeatureType::Forest, 0.5);
        assert!(layer.add_feature(extra_feature).is_err());
    }

    #[test]
    fn test_layer_intensity() {
        let mut layer = TileLayer::new(LayerType::Resources);
        
        layer.add_feature(LayerFeature::new(1, FeatureType::IronDeposit, 0.3)).unwrap();
        layer.add_feature(LayerFeature::new(2, FeatureType::GoldVein, 0.7)).unwrap();
        
        assert_eq!(layer.total_intensity(), 1.0); // 0.3 + 0.7
        
        let significant: Vec<_> = layer.significant_features().collect();
        assert_eq!(significant.len(), 1); // Only gold vein is significant (>= 0.1 and substantially above)
    }

    #[test]
    fn test_layer_manager() {
        let tile_manager = Arc::new(TileComponentManager::new());
        let chunk_manager = Arc::new(ChunkManager::default());
        let manager = TileLayerManager::new(tile_manager, chunk_manager);
        
        // Test feature ID generation
        let id1 = {
            let next_id = manager.next_feature_id.read();
            *next_id
        };
        
        // After creating features, ID should increment
        let mut next_id = manager.next_feature_id.write();
        *next_id += 1;
        
        assert!(id1 < *next_id);
    }
}
