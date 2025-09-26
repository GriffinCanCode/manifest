//! Bevy systems for layer processing and management
//!
//! Provides game systems for processing layer updates, temporal effects,
//! and visibility management within the Bevy ECS framework.

use bevy_ecs::prelude::*;
use tracing::{debug, instrument};

use super::{
    stack::TileLayerStack,
    manager::TileLayerManager,
    types::{LayerType, FeatureType},
};

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
        for layer in layer_stack.layers_mut().iter_mut() {
            let mut expired_features = Vec::new();
            
            // Check for expired features (if they have duration metadata)
            for (index, feature) in layer.features().iter().enumerate() {
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
                if index < layer.features().len() {
                    // Use the remove_feature method instead of direct access
                    if let Some(feature) = layer.features().get(index) {
                        let feature_id = feature.id;
                        layer.remove_feature(feature_id);
                        features_expired += 1;
                    }
                }
            }
            
            // Update feature intensities for temporal effects
            let layer_generation = layer.generation();
            let mut layer_changed = false;
            
            for feature in layer.features_mut().iter_mut() {
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
                    layer_changed = true;
                    features_updated += 1;
                }
            }
            
            // Update generation after the loop if any changes were made
            if layer_changed {
                layer.set_generation(layer_generation + 1);
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
