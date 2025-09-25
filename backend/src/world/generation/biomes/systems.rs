//! Biome ECS Systems
//!
//! ECS systems for biome generation, transitions, and validation.

use bevy_ecs::prelude::*;
use tracing::{debug, instrument};

use crate::{
    core::{scheduler::Scheduler, logging::game_logging},
    ecs::components::{Position, Name},
    world::tiles::{
        chunks::TileId,
        components::Tile,
        properties::{Biome, EnhancedClimate, EnhancedTerrainType, Elevation},
    },
};

use super::{
    core::BiomeGenerator,
    rules::{LuaBiomeRules, BiomeClimateData},
    transitions::{BiomeTransitionManager, BiomeTransition},
};

/// Biome generation system - creates biomes from climate data
#[instrument(name = "biome_generation", skip_all)]
pub fn biome_generation_system(
    mut commands: Commands,
    biome_generator: Res<BiomeGenerator>,
    scheduler: Res<Scheduler>,
    
    // Query for tiles with climate but no biome
    tiles_query: Query<(Entity, &TileId, &EnhancedClimate, &EnhancedTerrainType, &Elevation), 
                       (With<Tile>, Without<Biome>)>,
) {
    if tiles_query.is_empty() {
        return;
    }
    
    // Collect tile data for batch processing
    let tile_data: Vec<_> = tiles_query
        .iter()
        .map(|(entity, tile_id, climate, terrain, elevation)| {
            (entity, *tile_id, climate.clone(), terrain.clone(), elevation.clone())
        })
        .collect();
    
    // Generate biomes in batch
    let batch_data: Vec<_> = tile_data.iter()
        .map(|(_, tile_id, climate, terrain, elevation)| (*tile_id, climate.clone(), terrain.clone(), elevation.clone()))
        .collect();
    
    // Remove await since this is not in an async context
    // Note: This would need proper async context or different approach
    match Ok(vec![]) { // Placeholder - proper implementation needed
        Ok(biome_results) => {
            // Apply generated biomes to entities
            for (idx, (entity, _tile_id, _, _, _)) in tile_data.iter().enumerate() {
                if let Some(biome) = biome_results.get(idx) {
                    commands.entity(*entity).insert(biome.clone());
                }
            }
            
            debug!("🌿 Generated biomes for {} tiles", biome_results.len());
        }
        Err(e) => {
            tracing::error!("Biome generation failed: {}", e);
        }
    }
}

/// Biome transition system - handles biome boundaries and transitions
#[instrument(name = "biome_transitions", skip_all)]
pub fn biome_transition_system(
    mut transition_manager: ResMut<BiomeTransitionManager>,
    mut biome_query: Query<(Entity, &TileId, &mut Biome, &Position), Changed<Biome>>,
    neighbor_query: Query<(&TileId, &Biome, &Position)>,
) {
    use rayon::prelude::*;
    
    // Process transitions - collect entities first
    let entities: Vec<_> = biome_query.iter().map(|(entity, tile_id, biome, position)| {
        (entity, *tile_id, biome.clone(), *position)
    }).collect();
    
    let transition_updates: Vec<_> = entities.iter().filter_map(|(entity, tile_id, biome, position)| {
        // Find neighboring biomes within transition range
        let neighbors: Vec<_> = neighbor_query
            .iter()
            .filter_map(|(neighbor_id, neighbor_biome, neighbor_pos)| {
                if neighbor_id == tile_id {
                    return None; // Skip self
                }
                
                let dx = position.q() - neighbor_pos.q();
                let dy = position.r() - neighbor_pos.r();
                let distance = (dx * dx + dy * dy).sqrt();
                
                if distance < 3.0 { // Within transition range
                    Some((*neighbor_id, neighbor_biome.biome_type.clone(), distance))
                } else {
                    None
                }
            })
            .collect();
        
        // Calculate transition for this tile
        let transition = transition_manager.calculate_transition(
            *tile_id,
                &biome.biome_type,
                &neighbors,
            );
            
            transition.map(|t| (entity, *tile_id, t))
        })
        .collect();
    
    // Apply transitions
    for (entity, tile_id, transition) in transition_updates {
        // Update transition manager
        transition_manager.update_transition(tile_id, transition.clone());
        
        // Modify biome based on transition
        if let Ok((_, _, mut biome, _)) = biome_query.get_mut(entity) {
            transition_manager.apply_transition_to_biome(&mut biome, &transition);
        }
    }
    
    if !transition_manager.transitions().is_empty() {
        debug!("🌿 Processed {} biome transitions", transition_manager.transitions().len());
    }
}

/// Biome validation system - ensures biome assignments make sense
#[instrument(name = "biome_validation", skip_all)]
pub fn biome_validation_system(
    mut commands: Commands,
    biome_generator: Res<BiomeGenerator>,
    lua_rules: Res<LuaBiomeRules>,
    
    // Query for biomes that might need validation
    validation_query: Query<(Entity, &TileId, &Biome, &EnhancedClimate, &EnhancedTerrainType, &Elevation, Option<&Name>)>,
) {
    for (entity, tile_id, biome, climate, terrain, elevation, name) in validation_query.iter() {
        // Create climate data for validation
        let climate_data = BiomeClimateData {
            temperature: climate.temperature,
            rainfall: climate.rainfall,
            humidity: climate.humidity,
            elevation: elevation.meters(),
            terrain_type: terrain.to_string(),
            climate_zone: climate.interpolated.climate_zone.clone(),
        };
        
        // Check if current biome is reasonable
        match lua_rules.determine_biome(&climate_data) {
            Ok(decision) => {
                // If confidence is very low or biome is completely different, suggest replacement
                if decision.confidence < 0.2 || 
                   (decision.primary_biome != biome.biome_type && decision.confidence > 0.8) {
                    
                    // Generate a better biome
                    match biome_generator.generate_biome(*tile_id, climate, terrain, elevation) {
                        Ok(new_biome) => {
                            if let Some(name) = name {
                                debug!("🌿 Correcting biome for {}: {} -> {} (confidence: {:.2})",
                                      name.value, biome.biome_type, new_biome.biome_type, decision.confidence);
                            }
                            commands.entity(entity).insert(new_biome);
                        }
                        Err(e) => {
                            tracing::warn!("Failed to generate replacement biome for {:?}: {}", tile_id, e);
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Biome validation failed for {:?}: {}", tile_id, e);
            }
        }
    }
}

/// Biome rule processing system - applies Lua rules to modify biomes
#[instrument(name = "biome_rules", skip_all)]
pub fn biome_rule_processing_system(
    lua_rules: Res<LuaBiomeRules>,
    mut biome_query: Query<(&TileId, &mut Biome, &EnhancedClimate, &EnhancedTerrainType, &Elevation), Changed<EnhancedClimate>>,
) {
    for (tile_id, mut biome, climate, terrain, elevation) in biome_query.iter_mut() {
        let climate_data = BiomeClimateData {
            temperature: climate.temperature,
            rainfall: climate.rainfall,
            humidity: climate.humidity,
            elevation: elevation.meters(),
            terrain_type: terrain.to_string(),
            climate_zone: climate.interpolated.climate_zone.clone(),
        };
        
        // Re-evaluate biome with Lua rules when climate changes
        match lua_rules.determine_biome(&climate_data) {
            Ok(decision) => {
                // Apply rule-based modifications if confidence is high enough
                if decision.confidence > 0.6 {
                    let old_biome_type = biome.biome_type.clone();
                    biome.biome_type = decision.primary_biome;
                    biome.suitability_score = decision.confidence;
                    
                    // Apply modifiers from Lua rules
                    for modifier in decision.modifiers {
                        if let Some((mod_type, value_str)) = modifier.split_once('=') {
                            if let Ok(value) = value_str.parse::<f32>() {
                                match mod_type {
                                    "movement_cost" => biome.modifiers.movement_cost_multiplier = value,
                                    "defense_bonus" => biome.modifiers.defense_bonus = value,
                                    "agriculture" => biome.modifiers.agriculture_yield = value,
                                    "mining" => biome.modifiers.mining_yield = value,
                                    "population" => biome.modifiers.population_capacity = value,
                                    _ => {}
                                }
                            }
                        }
                    }
                    
                    if old_biome_type != biome.biome_type {
                        debug!("🌿 Lua rules changed biome for {:?}: {} -> {} (confidence: {:.2})",
                              tile_id, old_biome_type, biome.biome_type, decision.confidence);
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Lua biome rule processing failed for {:?}: {}", tile_id, e);
            }
        }
    }
}

/// System set for organizing biome systems
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum BiomeSystemSet {
    /// Core biome generation from climate data
    Generation,
    /// Lua rule processing and modifications
    RuleProcessing,
    /// Biome transition handling
    Transitions,
    /// Validation and error correction
    Validation,
}

/// Configure biome systems with proper scheduling
pub fn configure_biome_systems(scheduler: &mut crate::ecs::EcsScheduler, world: &mut bevy_ecs::world::World) {
    use crate::{Stage, ecs::systems::schedule::ResourceAccess};
    
    // Add biome generation system
    scheduler.add_system_with_accesses(
        Stage::Gameplay,
        "biome_generation_system",
        biome_generation_system,
        vec![
            ResourceAccess::read::<crate::ecs::resources::GameTime>(),
            ResourceAccess::write::<BiomeGenerator>(),
        ],
        world
    );
    
    // Add biome rule processing system
    scheduler.add_system_with_accesses(
        Stage::Gameplay,
        "biome_rule_processing_system", 
        biome_rule_processing_system,
        vec![ResourceAccess::write::<BiomeGenerator>()],
        world
    );
    
    // Add biome transition system
    scheduler.add_system_with_accesses(
        Stage::Gameplay,
        "biome_transition_system",
        biome_transition_system,
        vec![ResourceAccess::write::<BiomeGenerator>()],
        world
    );
    
    // Add biome validation system
    scheduler.add_system(Stage::Late, "biome_validation_system", biome_validation_system, world);
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_biome_system_configuration() {
        let mut app = App::new();
        configure_biome_systems(&mut app);
        
        // Verify systems are registered
        let schedule = app.world().resource::<Schedules>().get(Update).unwrap();
        assert!(schedule.graph().systems().count() >= 4);
    }
}
