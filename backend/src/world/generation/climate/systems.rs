//! Climate ECS Systems
//!
//! Integrates climate generation with existing ECS architecture.

use bevy_ecs::prelude::*;
use tracing::{debug, instrument};

use crate::{
    core::{scheduler::Scheduler, logging::game_logging, hashing::FastHashMap},
    ecs::{components::{Position, Name}, resources::GameTime},
    world::{
        generation::noise::NoiseGenerator,
        tiles::{
            chunks::TileId,
            components::Tile,
            properties::EnhancedClimate,
        },
    },
};

use super::{
    core::ClimateGenerator,
    patterns::{WindPatterns, OceanCurrents, SeasonalVariation},
    effects::ClimateModifier,
};

/// Climate generation system - integrates with existing ECS patterns
#[instrument(name = "climate_generation", skip_all)]
pub fn climate_generation_system(
    mut commands: Commands,
    climate_generator: Res<ClimateGenerator>,
    noise_generator: Res<NoiseGenerator>,
    scheduler: Res<Scheduler>,
    
    // Query for tiles that need climate generation
    tiles_without_climate: Query<(Entity, &TileId, &Position), (With<Tile>, Without<EnhancedClimate>)>,
    
    // Optional elevation data
    elevation_query: Query<&crate::world::tiles::properties::Elevation>,
) {
    if tiles_without_climate.is_empty() {
        return;
    }
    
    // Collect tiles for batch processing
    let tile_data: Vec<_> = tiles_without_climate
        .iter()
        .filter_map(|(entity, tile_id, position)| {
            let elevation = elevation_query.get(entity)
                .map(|e| e.final_elevation)
                .unwrap_or(0.0);
            
            Some((entity, *tile_id, position.q() as f64, position.r() as f64, elevation))
        })
        .collect();
    
    if tile_data.is_empty() {
        return;
    }
    
    // Generate climate data in batch using scheduler
    let batch_data: Vec<_> = tile_data.iter()
        .map(|(_, tile_id, x, y, elevation)| (*tile_id, *x, *y, *elevation))
        .collect();
    
    // Prioritize Zig SIMD optimized methods with multiple fallback levels
    let climate_results = if batch_data.len() <= 256 {
        // Prefer Zig SIMD optimized method for smaller batches
        climate_generator.generate_batch_optimized(batch_data.clone(), &*noise_generator)
            .or_else(|e| {
                tracing::warn!("Zig optimized batch failed ({}), falling back to scheduler method", e);
                climate_generator.generate_batch(batch_data, NoiseGenerator::new(noise_generator.config()), &*scheduler)
            })
    } else if batch_data.len() <= 1024 {
        // Use scheduler-based method for medium batches
        climate_generator.generate_batch(batch_data.clone(), NoiseGenerator::new(noise_generator.config()), &*scheduler)
            .or_else(|e| {
                tracing::warn!("Scheduler batch failed ({}), falling back to Zig optimized", e);
                // Split into smaller batches for Zig processing
                let chunk_results: Result<std::collections::HashMap<_, _>, String> = batch_data.chunks(256)
                    .map(|chunk| climate_generator.generate_batch_optimized(chunk.to_vec(), &*noise_generator))
                    .collect::<Result<Vec<_>, _>>()
                    .map(|results| {
                        let mut combined = std::collections::HashMap::new();
                        for result in results {
                            combined.extend(result);
                        }
                        combined
                    });
                chunk_results
            })
    } else {
        // For very large batches, chunk into Zig-optimized sizes
        let chunk_results: Result<std::collections::HashMap<_, _>, String> = batch_data.chunks(256)
            .map(|chunk| climate_generator.generate_batch_optimized(chunk.to_vec(), &*noise_generator))
            .collect::<Result<Vec<_>, _>>()
            .map(|results| {
                let mut combined = std::collections::HashMap::new();
                for result in results {
                    combined.extend(result);
                }
                combined
            });
        chunk_results
    };
    
    match climate_results {
        Ok(climate_results) => {
            // Apply generated climate to entities
            for (entity, tile_id, _, _, _) in tile_data {
                if let Some(climate) = climate_results.get(&tile_id) {
                    commands.entity(entity).insert(climate.clone());
                }
            }
            
            debug!("🌡️ Generated climate for {} tiles", climate_results.len());
        }
        Err(e) => {
            tracing::error!("Climate generation failed: {}", e);
        }
    }
}

/// Climate interpolation system - smooths climate between adjacent tiles using Zig SIMD
#[instrument(name = "climate_interpolation", skip_all)]
pub fn climate_interpolation_system(
    mut query_set: ParamSet<(
        Query<(Entity, &mut EnhancedClimate, &Position, Option<&Name>)>,
        Query<(&EnhancedClimate, &Position)>,
    )>,
) {
    
    // First, collect all entities and their data for batch processing
    let entities: Vec<_> = query_set.p0().iter().map(|(entity, climate, position, name)| {
        (entity, climate.clone(), *position, name.cloned())
    }).collect();
    
    if entities.is_empty() {
        return;
    }
    
    // Prepare data for Zig batch interpolation
    let mut center_positions = Vec::new();
    let mut center_climates = Vec::new();
    let mut neighbor_positions_all = Vec::new();
    let mut neighbor_climates_all = Vec::new();
    let mut neighbor_counts = Vec::new();
    let mut neighbor_offsets = Vec::new();
    let mut entity_mapping = Vec::new();
    
    let mut current_neighbor_offset = 0;
    
    for (entity, climate, position, name) in &entities {
        // Find neighboring climates within interpolation distance
        let neighbor_query = query_set.p1();
        let neighbors: Vec<_> = neighbor_query
            .iter()
            .filter(|(_, neighbor_pos)| {
                let dx = position.q() - neighbor_pos.q();
                let dy = position.r() - neighbor_pos.r();
                let distance = ((dx * dx + dy * dy) as f32).sqrt();
                distance > 0.1 && distance < 2.0 // Adjacent tiles
            })
            .collect();
        
        if !neighbors.is_empty() {
            center_positions.push((position.pixel().x, position.pixel().y));
            center_climates.push(super::zig_ffi::ClimateData {
                temperature: climate.temperature as f32,
                rainfall: climate.rainfall as f32,
                humidity: climate.humidity as f32,
                wind_strength: climate.wind_strength as f32,
            });
            
            // Add neighbors to the global neighbor arrays
            for (neighbor_climate, neighbor_pos) in &neighbors {
                neighbor_positions_all.push((neighbor_pos.pixel().x, neighbor_pos.pixel().y));
                neighbor_climates_all.push(super::zig_ffi::ClimateData {
                    temperature: neighbor_climate.temperature as f32,
                    rainfall: neighbor_climate.rainfall as f32,
                    humidity: neighbor_climate.humidity as f32,
                    wind_strength: neighbor_climate.wind_strength as f32,
                });
            }
            
            neighbor_counts.push(neighbors.len() as u32);
            neighbor_offsets.push(current_neighbor_offset);
            current_neighbor_offset += neighbors.len() as u32;
            
            entity_mapping.push(*entity);
        }
    }
    
    // Use Zig SIMD batch interpolation if we have data to process
    if !center_positions.is_empty() {
        let interpolation_params = super::zig_ffi::InterpolationParams {
            temperature_weight: 0.8,
            rainfall_weight: 0.7,
            humidity_weight: 0.6,
            wind_weight: 0.4,
            distance_falloff: 1.5,
            max_influence_distance: 2.0,
        };
        
        match super::zig_ffi::climate_interpolate_batch(
            &center_positions,
            &center_climates,
            &neighbor_positions_all,
            &neighbor_climates_all,
            &neighbor_counts,
            &neighbor_offsets,
            interpolation_params,
        ) {
            Ok(interpolated_results) => {
                // Apply interpolated results back to entities
                for (i, &entity) in entity_mapping.iter().enumerate() {
                    if i < interpolated_results.len() {
                        if let Ok((_, mut climate, _, name)) = query_set.p0().get_mut(entity) {
                            let interpolated = &interpolated_results[i];
                            
                            climate.temperature = interpolated.temperature as i8;
                            climate.rainfall = interpolated.rainfall.clamp(0.0, 500.0) as u16;
                            climate.humidity = interpolated.humidity.clamp(0.0, 100.0) as u8;
                            climate.wind_strength = interpolated.wind_strength.clamp(0.0, 255.0) as u8;
                            
                            if let Some(name) = name {
                                debug!("🌡️ Zig-interpolated climate for {}", name.value());
                            }
                        }
                    }
                }
                
                debug!("🚀 Zig SIMD interpolated {} climate tiles", interpolated_results.len());
            }
            Err(e) => {
                tracing::warn!("Zig interpolation failed ({}), falling back to Rust implementation", e);
                
                // Fallback to original Rust implementation
                let mut updates: Vec<_> = Vec::new();
                
                for (entity, climate, position, name) in entities {
                    let neighbor_query = query_set.p1();
                    let neighbors: Vec<_> = neighbor_query
                        .iter()
                        .filter(|(_, neighbor_pos)| {
                            let dx = position.q() - neighbor_pos.q();
                            let dy = position.r() - neighbor_pos.r();
                            let distance = ((dx * dx + dy * dy) as f32).sqrt();
                            distance > 0.1 && distance < 2.0
                        })
                        .map(|(neighbor_climate, _)| neighbor_climate.clone())
                        .collect();
                    
                    if !neighbors.is_empty() {
                        let mut updated_climate = climate.clone();
                        updated_climate.update_interpolation(&neighbors);
                        updates.push((entity, updated_climate, name));
                    }
                }
                
                // Apply fallback updates
                for (entity, updated_climate, name) in updates {
                    if let Ok((_, mut climate, _, _)) = query_set.p0().get_mut(entity) {
                        *climate = updated_climate;
                        
                        if let Some(name) = name {
                            debug!("🌡️ Rust-interpolated climate for {}", name.value());
                        }
                    }
                }
            }
        }
    }
}

/// Seasonal climate variation system using Zig SIMD batch processing
#[instrument(name = "seasonal_climate", skip_all)]
pub fn seasonal_climate_system(
    mut seasonal: ResMut<SeasonalVariation>,
    game_time: Res<GameTime>,
    mut climate_query: Query<(&mut EnhancedClimate, &Position)>,
) {
    // Update current season based on game time
    let season = (game_time.turn as f32 / 365.0) % 1.0; // Assuming 365 turns per year
    seasonal.update_season(season);
    
    // Collect all climate data for batch processing
    let climate_data: Vec<_> = climate_query.iter().map(|(climate, position)| {
        let latitude = ((position.r() as f32 / 256.0) - 0.5) * 180.0; // Assuming 256 world height
        let zone = &climate.interpolated.climate_zone;
        (climate.clone(), latitude, zone.clone())
    }).collect();
    
    if climate_data.is_empty() {
        return;
    }
    
    // Prepare data for Zig batch processing
    let base_temps: Vec<i8> = climate_data.iter().map(|(climate, _, _)| climate.temperature).collect();
    let base_rainfall: Vec<u16> = climate_data.iter().map(|(climate, _, _)| climate.rainfall).collect();
    let latitudes: Vec<f32> = climate_data.iter().map(|(_, latitude, _)| *latitude).collect();
    let climate_zones: Vec<&str> = climate_data.iter().map(|(_, _, zone)| zone.as_str()).collect();
    
    // Process temperatures and rainfall in batches using Zig SIMD
    let chunk_size = 256; // Zig SIMD limit
    let mut all_seasonal_temps = Vec::new();
    let mut all_seasonal_rain = Vec::new();
    
    for chunk_start in (0..climate_data.len()).step_by(chunk_size) {
        let chunk_end = (chunk_start + chunk_size).min(climate_data.len());
        
        let temp_chunk = &base_temps[chunk_start..chunk_end];
        let rain_chunk = &base_rainfall[chunk_start..chunk_end];
        let lat_chunk = &latitudes[chunk_start..chunk_end];
        let zone_chunk = &climate_zones[chunk_start..chunk_end];
        
        // Use Zig SIMD for seasonal temperature processing
        match seasonal.apply_temperature_variation_batch(temp_chunk, zone_chunk, lat_chunk) {
            Ok(seasonal_temps) => {
                all_seasonal_temps.extend(seasonal_temps);
            }
            Err(e) => {
                tracing::warn!("Zig seasonal temperature failed ({}), using fallback", e);
                // Fallback to individual processing
                for ((&base_temp, &latitude), &zone) in temp_chunk.iter().zip(lat_chunk.iter()).zip(zone_chunk.iter()) {
                    let seasonal_temp = seasonal.apply_temperature_variation(base_temp, zone, latitude);
                    all_seasonal_temps.push(seasonal_temp);
                }
            }
        }
        
        // Use Zig SIMD for seasonal rainfall processing
        match seasonal.apply_rainfall_variation_batch(rain_chunk, zone_chunk, lat_chunk) {
            Ok(seasonal_rain) => {
                all_seasonal_rain.extend(seasonal_rain);
            }
            Err(e) => {
                tracing::warn!("Zig seasonal rainfall failed ({}), using fallback", e);
                // Fallback to individual processing
                for (&base_rain, &zone) in rain_chunk.iter().zip(zone_chunk.iter()) {
                    let seasonal_rain = seasonal.apply_rainfall_variation(base_rain, zone);
                    all_seasonal_rain.push(seasonal_rain);
                }
            }
        }
    }
    
    // Apply results back to climate query
    let mut update_count = 0;
    for (i, (mut climate, _)) in climate_query.iter_mut().enumerate() {
        if i < all_seasonal_temps.len() && i < all_seasonal_rain.len() {
            let new_temp = all_seasonal_temps[i];
            let new_rain = all_seasonal_rain[i];
            
            // Only update if there's a significant change to avoid unnecessary updates
            if (new_temp - climate.temperature).abs() >= 1 {
                climate.temperature = new_temp;
                update_count += 1;
            }
            
            if (new_rain as i32 - climate.rainfall as i32).abs() >= 5 {
                climate.rainfall = new_rain;
                update_count += 1;
            }
        }
    }
    
    if update_count > 0 {
        debug!("🌡️ Updated {} climate values using Zig SIMD seasonal processing", update_count);
    }
}

/// Wind and ocean current update system
#[instrument(name = "climate_patterns", skip_all)]
pub fn climate_patterns_system(
    wind_patterns: Res<WindPatterns>,
    mut ocean_currents: ResMut<OceanCurrents>,
    climate_query: Query<(&EnhancedClimate, &Position, &TileId), Changed<EnhancedClimate>>,
    tile_query: Query<&crate::world::tiles::properties::terrain::EnhancedTerrainType>,
) {
    // Update ocean currents for water tiles with changed climate
    for (climate, position, tile_id) in climate_query.iter() {
        // Note: TileId cannot be directly converted to Entity
        // This needs proper tile lookup system
        // For now, skip terrain check
        if false { // TODO: Implement proper tile lookup
            // Check if this is a water tile
            if false { // terrain.is_water() - TODO: Implement terrain water check
                // Simple current calculation based on climate and position
                let latitude_index = ((position.r() as f32 / 256.0) * 180.0) as u32;
                
                if let Some(wind_belt) = wind_patterns.get_wind(latitude_index.clamp(0, 179)) {
                    let current_strength = (wind_belt.base_speed / 100.0).clamp(0.1, 1.0);
                    let current_direction = wind_belt.direction;
                    
                    // Determine current type based on temperature and location
                    let current_type = if climate.temperature > 20 {
                        super::patterns::CurrentType::WarmWesternBoundary
                    } else if climate.temperature < 10 {
                        super::patterns::CurrentType::ColdEasternBoundary
                    } else {
                        super::patterns::CurrentType::SubtropicalGyre
                    };
                    
                    ocean_currents.update_current(*tile_id, current_strength, current_direction, current_type);
                }
            }
        }
    }
}

/// Climate effects application system
#[instrument(name = "climate_effects", skip_all)]
pub fn climate_effects_system(
    mut climate_modifiers: Query<&mut ClimateModifier>,
    mut climate_query: Query<(&mut EnhancedClimate, &Position), Changed<EnhancedClimate>>,
    elevation_query: Query<&crate::world::tiles::properties::Elevation>,
    wind_patterns: Res<WindPatterns>,
) {
    if climate_query.is_empty() || climate_modifiers.is_empty() {
        return;
    }
    
    // Get the first climate modifier (assuming single world modifier for now)
    if let Ok(mut modifier) = climate_modifiers.get_single_mut() {
        // Collect data for batch processing
        let batch_data: Vec<_> = climate_query
            .iter()
            .enumerate()
            .map(|(i, (climate, position))| {
                let elevation = elevation_query.get(Entity::from_raw(i as u32))
                    .map(|e| e.final_elevation)
                    .unwrap_or(0.0);
                
                (
                    (position.pixel().x, position.pixel().y),
                    elevation,
                    climate.temperature,
                    climate.rainfall as f32,
                    climate.humidity,
                )
            })
            .collect();
        
        if batch_data.is_empty() {
            return;
        }
        
        // Extract data for batch processing
        let positions: Vec<_> = batch_data.iter().map(|(pos, _, _, _, _)| *pos).collect();
        let elevations: Vec<_> = batch_data.iter().map(|(_, elev, _, _, _)| *elev).collect();
        let temperatures: Vec<_> = batch_data.iter().map(|(_, _, temp, _, _)| *temp).collect();
        let rainfall: Vec<_> = batch_data.iter().map(|(_, _, _, rain, _)| *rain).collect();
        let humidity: Vec<_> = batch_data.iter().map(|(_, _, _, _, hum)| *hum).collect();
        
        // Get prevailing wind direction (simplified)
        let wind_direction = wind_patterns.latitude_winds
            .values()
            .next()
            .map(|w| w.direction)
            .unwrap_or(0.0);
        
        // Apply climate effects in batch - prioritize ultra-optimized Zig method
        let effects_result = if positions.len() <= 128 {
            // Use ultra-optimized Zig method for smaller batches (highest performance)
            modifier.apply_effects_batch_ultra_optimized(
                &positions,
                &elevations,
                &temperatures,
                &rainfall,
                &humidity,
                wind_direction,
            ).or_else(|e| {
                tracing::warn!("Ultra-optimized batch failed ({}), falling back to optimized", e);
                modifier.apply_effects_batch_optimized(
                    &positions,
                    &elevations,
                    &temperatures,
                    &rainfall,
                    &humidity,
                    wind_direction,
                )
            }).or_else(|e| {
                tracing::warn!("Optimized batch failed ({}), falling back to enhanced", e);
                modifier.apply_effects_batch(
                    &positions,
                    &elevations,
                    &temperatures,
                    &rainfall,
                    &humidity,
                    wind_direction,
                )
            })
        } else if positions.len() <= 256 {
            // Use optimized Zig method for medium batches
            modifier.apply_effects_batch_optimized(
                &positions,
                &elevations,
                &temperatures,
                &rainfall,
                &humidity,
                wind_direction,
            ).or_else(|e| {
                tracing::warn!("Optimized batch failed ({}), falling back to enhanced", e);
                modifier.apply_effects_batch(
                    &positions,
                    &elevations,
                    &temperatures,
                    &rainfall,
                    &humidity,
                    wind_direction,
                )
            })
        } else {
            // For larger batches, chunk into ultra-optimized sizes
            let chunk_size = 128;
            let chunks: Result<Vec<_>, String> = positions.chunks(chunk_size)
                .zip(elevations.chunks(chunk_size))
                .zip(temperatures.chunks(chunk_size))
                .zip(rainfall.chunks(chunk_size))
                .zip(humidity.chunks(chunk_size))
                .map(|((((pos_chunk, elev_chunk), temp_chunk), rain_chunk), hum_chunk)| {
                    modifier.apply_effects_batch_ultra_optimized(
                        pos_chunk,
                        elev_chunk,
                        temp_chunk,
                        rain_chunk,
                        hum_chunk,
                        wind_direction,
                    )
                })
                .collect();
                
            chunks.map(|results| {
                let mut combined_temps = Vec::new();
                let mut combined_rain = Vec::new();
                let mut combined_hum = Vec::new();
                
                for (temps, rain, hum) in results {
                    combined_temps.extend(temps);
                    combined_rain.extend(rain);
                    combined_hum.extend(hum);
                }
                
                (combined_temps, combined_rain, combined_hum)
            })
        };
        
        match effects_result {
            Ok((modified_temps, modified_rain, modified_humidity)) => {
                // Apply results back to entities
                for ((mut climate, _), i) in climate_query.iter_mut().zip(0..) {
                    if let (Some(&temp), Some(&rain), Some(&hum)) = (
                        modified_temps.get(i),
                        modified_rain.get(i),
                        modified_humidity.get(i),
                    ) {
                        climate.temperature = temp;
                        climate.rainfall = rain as u16;
                        climate.humidity = hum;
                    }
                }
                
                debug!("🚀 Applied climate effects to {} tiles using Zig SIMD optimization", modified_temps.len());
            }
            Err(e) => {
                tracing::error!("Climate effects processing failed: {}", e);
            }
        }
    }
}

/// System set for organizing climate systems
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum ClimateSystemSet {
    /// Core climate generation
    Generation,
    /// Pattern and effect processing
    Processing,
    /// Interpolation and smoothing
    Interpolation,
    /// Seasonal updates
    Seasonal,
}

/// Configure climate systems with proper scheduling
pub fn configure_climate_systems(scheduler: &mut crate::ecs::EcsScheduler, world: &mut bevy_ecs::world::World) {
    use crate::{Stage, ecs::systems::schedule::ResourceAccess};
    
    // Add climate generation system
    scheduler.add_system_with_accesses(
        Stage::Update, 
        "climate_generation_system", 
        climate_generation_system, 
        vec![
            ResourceAccess::read::<crate::ecs::resources::GameTime>(),
            ResourceAccess::write::<ClimateGenerator>(),
        ],
        world
    );
    
    // Add climate processing systems
    scheduler.add_system_with_accesses(
        Stage::Update,
        "climate_patterns_system",
        climate_patterns_system,
        vec![ResourceAccess::write::<ClimateGenerator>()],
        world
    );
    
    // Add climate effects system
    scheduler.add_system_with_accesses(
        Stage::Update,
        "climate_effects_system", 
        climate_effects_system,
        vec![ResourceAccess::write::<ClimateGenerator>()],
        world
    );
    
    // Add interpolation system
    scheduler.add_system(Stage::Update, "climate_interpolation_system", climate_interpolation_system, world);
    
    // Add seasonal system
    scheduler.add_system_with_accesses(
        Stage::Update,
        "seasonal_climate_system",
        seasonal_climate_system,
        vec![ResourceAccess::read::<crate::ecs::resources::GameTime>()],
        world
    );
}

use bevy_ecs::schedule::IntoSystemConfigs;

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::prelude::*;
    
    #[test]
    fn test_system_configuration() {
        // Create a simple bevy world for testing instead of App
        let mut world = bevy_ecs::prelude::World::new();
        let mut scheduler = crate::ecs::EcsScheduler::new(None).unwrap();
        configure_climate_systems(&mut scheduler, &mut world);
        
        // Test passes if function executes without errors
        assert!(true);
    }
}
