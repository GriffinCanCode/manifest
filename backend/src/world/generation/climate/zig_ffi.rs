//! Climate FFI bindings to Zig SIMD optimizations
//!
//! Provides safe Rust wrappers around Zig's high-performance climate calculations
//! including orographic effects, continental effects, and seasonal variations.

use std::os::raw::{c_float, c_int};

// External C function declarations from Zig climate module
extern "C" {
    fn manifest_climate_orographic_effects(
        positions_x: *const c_float, positions_y: *const c_float,
        elevations: *const c_float,
        wind_directions: *const c_float,
        max_orographic_bonus: c_float,
        rain_shadow_factor: c_float,
        count: usize,
        results: *mut c_float,
    );
    fn manifest_climate_continental_effects(
        positions_x: *const c_float, positions_y: *const c_float,
        base_temperatures: *const i8,
        base_humidity: *const u8,
        temperature_amplification: c_float,
        humidity_reduction: c_float,
        world_width: c_float,
        world_height: c_float,
        count: usize,
        temperature_results: *mut i8,
        humidity_results: *mut u8,
    );
    fn manifest_climate_seasonal_temperature(
        base_temperatures: *const i8,
        climate_zones: *const u8,
        latitudes: *const c_float,
        current_season: c_float,
        temperature_variations: *const c_float,
        count: usize,
        results: *mut i8,
    );
    fn manifest_climate_seasonal_rainfall(
        base_rainfall: *const u16,
        climate_zones: *const u8,
        latitudes: *const c_float,
        current_season: c_float,
        rainfall_variations: *const c_float,
        count: usize,
        results: *mut u16,
    );
    fn manifest_climate_process_all(
        positions_x: *const c_float, positions_y: *const c_float,
        elevations: *const c_float,
        base_temperatures: *const i8,
        base_rainfall: *const c_float,
        base_humidity: *const u8,
        wind_directions: *const c_float,
        count: usize,
        temperature_results: *mut i8,
        rainfall_results: *mut c_float,
        humidity_results: *mut u8,
    );
    fn manifest_climate_ocean_proximity(
        positions_x: *const c_float, positions_y: *const c_float,
        world_width: c_float,
        world_height: c_float,
        count: usize,
        results: *mut c_float,
    );
    fn manifest_climate_rain_shadow(
        positions_x: *const c_float, positions_y: *const c_float,
        elevations: *const c_float,
        mountain_centers_x: *const c_float,
        mountain_centers_y: *const c_float,
        mountain_widths: *const c_float,
        mountain_heights: *const c_float,
        mountain_orientations: *const c_float,
        wind_direction: c_float,
        shadow_factor: c_float,
        count: usize,
        mountain_count: usize,
        results: *mut c_float,
    );
    fn manifest_climate_interpolate_batch(
        center_positions_x: *const c_float,
        center_positions_y: *const c_float,
        center_temperatures: *const c_float,
        center_rainfall: *const c_float,
        center_humidity: *const c_float,
        center_wind_strength: *const c_float,
        neighbor_positions_x: *const c_float,
        neighbor_positions_y: *const c_float,
        neighbor_temperatures: *const c_float,
        neighbor_rainfall: *const c_float,
        neighbor_humidity: *const c_float,
        neighbor_wind_strength: *const c_float,
        neighbor_counts: *const u32,
        neighbor_offsets: *const u32,
        temperature_weight: c_float,
        rainfall_weight: c_float,
        humidity_weight: c_float,
        wind_weight: c_float,
        distance_falloff: c_float,
        max_influence_distance: c_float,
        center_count: usize,
        neighbor_count: usize,
        result_temperatures: *mut c_float,
        result_rainfall: *mut c_float,
        result_humidity: *mut c_float,
        result_wind_strength: *mut c_float,
    );
    fn manifest_climate_monsoon_effects(
        latitudes: *const c_float,
        longitudes: *const c_float,
        current_season: c_float,
        year_progress: c_float,
        hemisphere_modifier: c_float,
        monsoon_strength: c_float,
        count: usize,
        results: *mut c_float,
    );
    fn manifest_climate_maritime_influence(
        positions_x: *const c_float,
        positions_y: *const c_float,
        world_width: c_float,
        world_height: c_float,
        count: usize,
        results: *mut c_float,
    );
    fn manifest_climate_gaussian_smoothing(
        positions_x: *const c_float,
        positions_y: *const c_float,
        temperatures: *const c_float,
        rainfall: *const c_float,
        humidity: *const c_float,
        wind_strength: *const c_float,
        kernel_size: u32,
        sigma: c_float,
        count: usize,
        result_temperatures: *mut c_float,
        result_rainfall: *mut c_float,
        result_humidity: *mut c_float,
        result_wind_strength: *mut c_float,
    );
}

/// Climate zone enum matching Zig definition
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClimateZone {
    Equatorial = 0,
    Tropical = 1,
    Temperate = 2,
    Polar = 3,
    Desert = 4,
    Mediterranean = 5,
}

/// Climate processing parameters
#[derive(Debug, Clone)]
pub struct ClimateParams {
    pub max_orographic_bonus: f32,
    pub rain_shadow_factor: f32,
    pub temperature_amplification: f32,
    pub humidity_reduction: f32,
    pub world_width: f32,
    pub world_height: f32,
}

impl Default for ClimateParams {
    fn default() -> Self {
        Self {
            max_orographic_bonus: 200.0,
            rain_shadow_factor: 0.6,
            temperature_amplification: 1.5,
            humidity_reduction: 0.8,
            world_width: 256.0,
            world_height: 256.0,
        }
    }
}

/// SIMD batch orographic effects calculation
#[cfg(not(feature = "no_zig"))]
pub fn climate_orographic_effects(
    positions: &[(f32, f32)],
    elevations: &[f32],
    wind_directions: &[f32],
    params: &ClimateParams,
) -> Result<Vec<f32>, String> {
    if positions.len() != elevations.len() || positions.len() != wind_directions.len() {
        return Err("Input arrays must have same length".to_string());
    }
    
    let len = positions.len().min(256); // Zig side limits to 256
    let mut results = vec![0.0f32; len];
    
    let positions_x: Vec<f32> = positions.iter().map(|(x, _)| *x).collect();
    let positions_y: Vec<f32> = positions.iter().map(|(_, y)| *y).collect();
    
    unsafe {
        manifest_climate_orographic_effects(
            positions_x.as_ptr(),
            positions_y.as_ptr(),
            elevations.as_ptr(),
            wind_directions.as_ptr(),
            params.max_orographic_bonus,
            params.rain_shadow_factor,
            len,
            results.as_mut_ptr(),
        );
    }
    
    Ok(results)
}

#[cfg(feature = "no_zig")]
pub fn climate_orographic_effects(
    positions: &[(f32, f32)],
    elevations: &[f32],
    wind_directions: &[f32],
    params: &ClimateParams,
) -> Result<Vec<f32>, String> {
    // Fallback implementation
    let results: Vec<f32> = elevations.iter().zip(wind_directions.iter())
        .map(|(&elevation, &wind_dir)| {
            let elevation_factor = (elevation / 1000.0).min(2.0);
            let wind_effect = (wind_dir.cos() + 1.0) * 0.5;
            1.0 + (elevation_factor * wind_effect * params.max_orographic_bonus)
        })
        .collect();
    Ok(results)
}

/// SIMD batch continental effects calculation
#[cfg(not(feature = "no_zig"))]
pub fn climate_continental_effects(
    positions: &[(f32, f32)],
    base_temperatures: &[i8],
    base_humidity: &[u8],
    params: &ClimateParams,
) -> Result<(Vec<i8>, Vec<u8>), String> {
    if positions.len() != base_temperatures.len() || positions.len() != base_humidity.len() {
        return Err("Input arrays must have same length".to_string());
    }
    
    let len = positions.len().min(256);
    let mut temp_results = vec![0i8; len];
    let mut humidity_results = vec![0u8; len];
    
    let positions_x: Vec<f32> = positions.iter().map(|(x, _)| *x).collect();
    let positions_y: Vec<f32> = positions.iter().map(|(_, y)| *y).collect();
    
    unsafe {
        manifest_climate_continental_effects(
            positions_x.as_ptr(),
            positions_y.as_ptr(),
            base_temperatures.as_ptr(),
            base_humidity.as_ptr(),
            params.temperature_amplification,
            params.humidity_reduction,
            params.world_width,
            params.world_height,
            len,
            temp_results.as_mut_ptr(),
            humidity_results.as_mut_ptr(),
        );
    }
    
    Ok((temp_results, humidity_results))
}

#[cfg(feature = "no_zig")]
pub fn climate_continental_effects(
    positions: &[(f32, f32)],
    base_temperatures: &[i8],
    base_humidity: &[u8],
    params: &ClimateParams,
) -> Result<(Vec<i8>, Vec<u8>), String> {
    // Fallback implementation
    let temp_results: Vec<i8> = positions.iter().zip(base_temperatures.iter())
        .map(|((x, y), &base_temp)| {
            let edge_dist_x = (x / params.world_width).min((params.world_width - x) / params.world_width);
            let edge_dist_y = (y / params.world_height).min((params.world_height - y) / params.world_height);
            let continentality = 1.0 - (1.0 - edge_dist_x.min(edge_dist_y) * 2.0).clamp(0.0, 1.0);
            
            let temp_modifier = if base_temp > 10 {
                continentality * params.temperature_amplification * 5.0
            } else {
                -continentality * params.temperature_amplification * 8.0
            };
            
            ((base_temp as f32) + temp_modifier).clamp(-50.0, 50.0) as i8
        })
        .collect();
        
    let humidity_results: Vec<u8> = positions.iter().zip(base_humidity.iter())
        .map(|((x, y), &base_hum)| {
            let edge_dist_x = (x / params.world_width).min((params.world_width - x) / params.world_width);
            let edge_dist_y = (y / params.world_height).min((params.world_height - y) / params.world_height);
            let continentality = 1.0 - (1.0 - edge_dist_x.min(edge_dist_y) * 2.0).clamp(0.0, 1.0);
            
            let humidity_reduction = continentality * params.humidity_reduction * 20.0;
            ((base_hum as f32) - humidity_reduction).clamp(0.0, 100.0) as u8
        })
        .collect();
    
    Ok((temp_results, humidity_results))
}

/// SIMD batch seasonal temperature calculation
#[cfg(not(feature = "no_zig"))]
pub fn climate_seasonal_temperature(
    base_temperatures: &[i8],
    climate_zones: &[ClimateZone], 
    latitudes: &[f32],
    current_season: f32,
    temperature_variations: &[f32; 6],
) -> Result<Vec<i8>, String> {
    if base_temperatures.len() != climate_zones.len() || base_temperatures.len() != latitudes.len() {
        return Err("Input arrays must have same length".to_string());
    }
    
    let len = base_temperatures.len().min(256);
    let mut results = vec![0i8; len];
    
    let zones_u8: Vec<u8> = climate_zones.iter().map(|&z| z as u8).collect();
    
    unsafe {
        manifest_climate_seasonal_temperature(
            base_temperatures.as_ptr(),
            zones_u8.as_ptr(),
            latitudes.as_ptr(),
            current_season,
            temperature_variations.as_ptr(),
            len,
            results.as_mut_ptr(),
        );
    }
    
    Ok(results)
}

#[cfg(feature = "no_zig")]
pub fn climate_seasonal_temperature(
    base_temperatures: &[i8],
    climate_zones: &[ClimateZone],
    latitudes: &[f32],
    current_season: f32,
    temperature_variations: &[f32; 6],
) -> Result<Vec<i8>, String> {
    // Fallback implementation
    let results: Vec<i8> = base_temperatures.iter().zip(climate_zones.iter()).zip(latitudes.iter())
        .map(|((&base_temp, &zone), &latitude)| {
            let variation = temperature_variations[zone as usize];
            let hemisphere_offset = if latitude >= 0.0 { 0.0 } else { 0.5 };
            let season_phase = current_season + hemisphere_offset;
            let season_cycle = (season_phase * 2.0 * std::f32::consts::PI).sin();
            let latitude_factor = (latitude.abs() / 90.0).min(1.0);
            let temp_change = season_cycle * variation * latitude_factor;
            
            ((base_temp as f32) + temp_change).clamp(-50.0, 50.0) as i8
        })
        .collect();
    
    Ok(results)
}

/// Complete climate processing pipeline
#[cfg(not(feature = "no_zig"))]
pub fn climate_process_all(
    positions: &[(f32, f32)],
    elevations: &[f32],
    base_temperatures: &[i8],
    base_rainfall: &[f32],
    base_humidity: &[u8],
    wind_directions: &[f32],
) -> Result<(Vec<i8>, Vec<f32>, Vec<u8>), String> {
    if positions.len() != elevations.len() || 
       positions.len() != base_temperatures.len() ||
       positions.len() != base_rainfall.len() ||
       positions.len() != base_humidity.len() ||
       positions.len() != wind_directions.len() {
        return Err("All input arrays must have same length".to_string());
    }
    
    let len = positions.len().min(256);
    let mut temp_results = vec![0i8; len];
    let mut rain_results = vec![0.0f32; len];
    let mut humidity_results = vec![0u8; len];
    
    let positions_x: Vec<f32> = positions.iter().map(|(x, _)| *x).collect();
    let positions_y: Vec<f32> = positions.iter().map(|(_, y)| *y).collect();
    
    unsafe {
        manifest_climate_process_all(
            positions_x.as_ptr(),
            positions_y.as_ptr(),
            elevations.as_ptr(),
            base_temperatures.as_ptr(),
            base_rainfall.as_ptr(),
            base_humidity.as_ptr(),
            wind_directions.as_ptr(),
            len,
            temp_results.as_mut_ptr(),
            rain_results.as_mut_ptr(),
            humidity_results.as_mut_ptr(),
        );
    }
    
    Ok((temp_results, rain_results, humidity_results))
}

#[cfg(feature = "no_zig")]
pub fn climate_process_all(
    positions: &[(f32, f32)],
    elevations: &[f32],
    base_temperatures: &[i8],
    base_rainfall: &[f32],
    base_humidity: &[u8],
    wind_directions: &[f32],
) -> Result<(Vec<i8>, Vec<f32>, Vec<u8>), String> {
    // Fallback implementation combining multiple effects
    let params = ClimateParams::default();
    
    // Apply orographic effects to rainfall
    let orographic_multipliers = climate_orographic_effects(positions, elevations, wind_directions, &params)?;
    let modified_rainfall: Vec<f32> = base_rainfall.iter().zip(orographic_multipliers.iter())
        .map(|(&rain, &multiplier)| rain * multiplier)
        .collect();
    
    // Apply continental effects
    let (temp_results, humidity_results) = climate_continental_effects(
        positions, base_temperatures, base_humidity, &params
    )?;
    
    Ok((temp_results, modified_rainfall, humidity_results))
}

/// Calculate ocean proximity for positions
#[cfg(not(feature = "no_zig"))]
pub fn climate_ocean_proximity(
    positions: &[(f32, f32)],
    world_width: f32,
    world_height: f32,
) -> Result<Vec<f32>, String> {
    let len = positions.len().min(256);
    let mut results = vec![0.0f32; len];
    
    let positions_x: Vec<f32> = positions.iter().map(|(x, _)| *x).collect();
    let positions_y: Vec<f32> = positions.iter().map(|(_, y)| *y).collect();
    
    unsafe {
        manifest_climate_ocean_proximity(
            positions_x.as_ptr(),
            positions_y.as_ptr(),
            world_width,
            world_height,
            len,
            results.as_mut_ptr(),
        );
    }
    
    Ok(results)
}

#[cfg(feature = "no_zig")]
pub fn climate_ocean_proximity(
    positions: &[(f32, f32)],
    world_width: f32,
    world_height: f32,
) -> Result<Vec<f32>, String> {
    let results: Vec<f32> = positions.iter()
        .map(|(x, y)| {
            let edge_dist_x = (x / world_width).min((world_width - x) / world_width);
            let edge_dist_y = (y / world_height).min((world_height - y) / world_height);
            let edge_distance = edge_dist_x.min(edge_dist_y);
            let proximity = 1.0 - edge_distance * 2.0;
            proximity.clamp(0.0, 1.0)
        })
        .collect();
    
    Ok(results)
}

/// Mountain range data for rain shadow calculations
#[derive(Debug, Clone)]
pub struct MountainRange {
    pub center: (f32, f32),
    pub width: f32,
    pub height: f32,
    pub orientation: f32,
}

/// Calculate rain shadow effects using Zig SIMD
#[cfg(not(feature = "no_zig"))]
pub fn climate_rain_shadow_effects(
    positions: &[(f32, f32)],
    elevations: &[f32],
    mountain_ranges: &[MountainRange],
    wind_direction: f32,
    shadow_factor: f32,
) -> Result<Vec<f32>, String> {
    if positions.len() != elevations.len() {
        return Err("Position and elevation arrays must have same length".to_string());
    }
    
    let len = positions.len().min(256);
    let mountain_count = mountain_ranges.len().min(32);
    let mut results = vec![0.0f32; len];
    
    let positions_x: Vec<f32> = positions.iter().map(|(x, _)| *x).collect();
    let positions_y: Vec<f32> = positions.iter().map(|(_, y)| *y).collect();
    
    // Extract mountain data
    let mountain_centers_x: Vec<f32> = mountain_ranges.iter().map(|m| m.center.0).collect();
    let mountain_centers_y: Vec<f32> = mountain_ranges.iter().map(|m| m.center.1).collect();
    let mountain_widths: Vec<f32> = mountain_ranges.iter().map(|m| m.width).collect();
    let mountain_heights: Vec<f32> = mountain_ranges.iter().map(|m| m.height).collect();
    let mountain_orientations: Vec<f32> = mountain_ranges.iter().map(|m| m.orientation).collect();
    
    unsafe {
        manifest_climate_rain_shadow(
            positions_x.as_ptr(),
            positions_y.as_ptr(),
            elevations.as_ptr(),
            mountain_centers_x.as_ptr(),
            mountain_centers_y.as_ptr(),
            mountain_widths.as_ptr(),
            mountain_heights.as_ptr(),
            mountain_orientations.as_ptr(),
            wind_direction,
            shadow_factor,
            len,
            mountain_count,
            results.as_mut_ptr(),
        );
    }
    
    Ok(results)
}

#[cfg(feature = "no_zig")]
pub fn climate_rain_shadow_effects(
    positions: &[(f32, f32)],
    elevations: &[f32],
    mountain_ranges: &[MountainRange],
    wind_direction: f32,
    shadow_factor: f32,
) -> Result<Vec<f32>, String> {
    // Fallback implementation
    let results: Vec<f32> = positions.iter().zip(elevations.iter())
        .map(|(&(x, y), &elevation)| {
            let mut shadow_effect = 1.0f32;
            let elevation_protection = (elevation / 2000.0).min(0.5);
            
            for mountain in mountain_ranges {
                let dx = x - mountain.center.0;
                let dy = y - mountain.center.1;
                let distance = (dx * dx + dy * dy).sqrt();
                
                let wind_dx = wind_direction.cos();
                let wind_dy = wind_direction.sin();
                let dot = dx * wind_dx + dy * wind_dy;
                
                if dot > 0.0 && distance < mountain.width {
                    let height_factor = mountain.height / 3000.0;
                    let distance_factor = 1.0 - (distance / mountain.width);
                    let mut shadow_strength = height_factor * distance_factor * shadow_factor;
                    shadow_strength *= 1.0 - elevation_protection;
                    shadow_effect *= 1.0 - shadow_strength;
                }
            }
            
            shadow_effect
        })
        .collect();
    
    Ok(results)
}

/// Climate interpolation parameters
#[derive(Debug, Clone)]
pub struct InterpolationParams {
    pub temperature_weight: f32,
    pub rainfall_weight: f32,
    pub humidity_weight: f32,
    pub wind_weight: f32,
    pub distance_falloff: f32,
    pub max_influence_distance: f32,
}

impl Default for InterpolationParams {
    fn default() -> Self {
        Self {
            temperature_weight: 1.0,
            rainfall_weight: 1.0,
            humidity_weight: 1.0,
            wind_weight: 0.5,
            distance_falloff: 1.0,
            max_influence_distance: 5.0,
        }
    }
}

/// Climate data for interpolation
#[derive(Debug, Clone)]
pub struct ClimateData {
    pub temperature: f32,
    pub rainfall: f32,
    pub humidity: f32,
    pub wind_strength: f32,
}

/// Batch climate interpolation using Zig SIMD
#[cfg(not(feature = "no_zig"))]
pub fn climate_interpolate_batch(
    center_positions: &[(f32, f32)],
    center_climates: &[ClimateData],
    neighbor_positions: &[(f32, f32)],
    neighbor_climates: &[ClimateData],
    neighbor_counts: &[u32],
    neighbor_offsets: &[u32],
    params: InterpolationParams,
) -> Result<Vec<ClimateData>, String> {
    if center_positions.len() != center_climates.len() ||
       center_positions.len() != neighbor_counts.len() ||
       center_positions.len() != neighbor_offsets.len() {
        return Err("Center arrays must have same length".to_string());
    }
    
    let center_count = center_positions.len().min(256);
    let neighbor_count = neighbor_positions.len().min(1024);
    
    // Prepare center data
    let center_x: Vec<f32> = center_positions.iter().map(|(x, _)| *x).collect();
    let center_y: Vec<f32> = center_positions.iter().map(|(_, y)| *y).collect();
    let center_temps: Vec<f32> = center_climates.iter().map(|c| c.temperature).collect();
    let center_rain: Vec<f32> = center_climates.iter().map(|c| c.rainfall).collect();
    let center_hum: Vec<f32> = center_climates.iter().map(|c| c.humidity).collect();
    let center_wind: Vec<f32> = center_climates.iter().map(|c| c.wind_strength).collect();
    
    // Prepare neighbor data
    let neighbor_x: Vec<f32> = neighbor_positions.iter().map(|(x, _)| *x).collect();
    let neighbor_y: Vec<f32> = neighbor_positions.iter().map(|(_, y)| *y).collect();
    let neighbor_temps: Vec<f32> = neighbor_climates.iter().map(|c| c.temperature).collect();
    let neighbor_rain: Vec<f32> = neighbor_climates.iter().map(|c| c.rainfall).collect();
    let neighbor_hum: Vec<f32> = neighbor_climates.iter().map(|c| c.humidity).collect();
    let neighbor_wind: Vec<f32> = neighbor_climates.iter().map(|c| c.wind_strength).collect();
    
    // Prepare result buffers
    let mut result_temps = vec![0.0f32; center_count];
    let mut result_rain = vec![0.0f32; center_count];
    let mut result_hum = vec![0.0f32; center_count];
    let mut result_wind = vec![0.0f32; center_count];
    
    unsafe {
        manifest_climate_interpolate_batch(
            center_x.as_ptr(),
            center_y.as_ptr(),
            center_temps.as_ptr(),
            center_rain.as_ptr(),
            center_hum.as_ptr(),
            center_wind.as_ptr(),
            neighbor_x.as_ptr(),
            neighbor_y.as_ptr(),
            neighbor_temps.as_ptr(),
            neighbor_rain.as_ptr(),
            neighbor_hum.as_ptr(),
            neighbor_wind.as_ptr(),
            neighbor_counts.as_ptr(),
            neighbor_offsets.as_ptr(),
            params.temperature_weight,
            params.rainfall_weight,
            params.humidity_weight,
            params.wind_weight,
            params.distance_falloff,
            params.max_influence_distance,
            center_count,
            neighbor_count,
            result_temps.as_mut_ptr(),
            result_rain.as_mut_ptr(),
            result_hum.as_mut_ptr(),
            result_wind.as_mut_ptr(),
        );
    }
    
    let results: Vec<ClimateData> = (0..center_count).map(|i| ClimateData {
        temperature: result_temps[i],
        rainfall: result_rain[i],
        humidity: result_hum[i],
        wind_strength: result_wind[i],
    }).collect();
    
    Ok(results)
}

#[cfg(feature = "no_zig")]
pub fn climate_interpolate_batch(
    center_positions: &[(f32, f32)],
    center_climates: &[ClimateData],
    neighbor_positions: &[(f32, f32)],
    neighbor_climates: &[ClimateData],
    neighbor_counts: &[u32],
    neighbor_offsets: &[u32],
    params: InterpolationParams,
) -> Result<Vec<ClimateData>, String> {
    // Simple fallback implementation
    let results: Vec<ClimateData> = center_climates.iter().cloned().collect();
    Ok(results)
}

/// Monsoon effects calculation parameters
#[derive(Debug, Clone)]
pub struct SeasonalState {
    pub current_season: f32,
    pub year_progress: f32,
    pub hemisphere_modifier: f32,
}

/// Calculate monsoon effects using Zig SIMD
#[cfg(not(feature = "no_zig"))]
pub fn climate_monsoon_effects(
    positions: &[(f32, f32)],
    seasonal_state: SeasonalState,
    monsoon_strength: f32,
) -> Result<Vec<f32>, String> {
    let len = positions.len().min(256);
    let mut results = vec![0.0f32; len];
    
    let latitudes: Vec<f32> = positions.iter().map(|(_, y)| (y / 256.0 - 0.5) * 180.0).collect();
    let longitudes: Vec<f32> = positions.iter().map(|(x, _)| (x / 256.0 - 0.5) * 360.0).collect();
    
    unsafe {
        manifest_climate_monsoon_effects(
            latitudes.as_ptr(),
            longitudes.as_ptr(),
            seasonal_state.current_season,
            seasonal_state.year_progress,
            seasonal_state.hemisphere_modifier,
            monsoon_strength,
            len,
            results.as_mut_ptr(),
        );
    }
    
    Ok(results)
}

#[cfg(feature = "no_zig")]
pub fn climate_monsoon_effects(
    positions: &[(f32, f32)],
    seasonal_state: SeasonalState,
    monsoon_strength: f32,
) -> Result<Vec<f32>, String> {
    let results: Vec<f32> = positions.iter()
        .map(|(_, y)| {
            let latitude = (y / 256.0 - 0.5) * 180.0;
            let abs_lat = latitude.abs();
            
            // Monsoons strongest in tropical latitudes
            let monsoon_factor = if abs_lat >= 10.0 && abs_lat <= 30.0 {
                1.0 - (abs_lat - 20.0).abs() / 10.0
            } else {
                0.0
            };
            
            let monsoon_phase = seasonal_state.current_season + 0.3;
            let monsoon_cycle = (monsoon_phase * 2.0 * std::f32::consts::PI).sin();
            
            monsoon_factor * monsoon_cycle * monsoon_strength
        })
        .collect();
    
    Ok(results)
}

/// Calculate maritime influence using Zig SIMD
#[cfg(not(feature = "no_zig"))]
pub fn climate_maritime_influence(
    positions: &[(f32, f32)],
    world_width: f32,
    world_height: f32,
) -> Result<Vec<f32>, String> {
    let len = positions.len().min(256);
    let mut results = vec![0.0f32; len];
    
    let positions_x: Vec<f32> = positions.iter().map(|(x, _)| *x).collect();
    let positions_y: Vec<f32> = positions.iter().map(|(_, y)| *y).collect();
    
    unsafe {
        manifest_climate_maritime_influence(
            positions_x.as_ptr(),
            positions_y.as_ptr(),
            world_width,
            world_height,
            len,
            results.as_mut_ptr(),
        );
    }
    
    Ok(results)
}

#[cfg(feature = "no_zig")]
pub fn climate_maritime_influence(
    positions: &[(f32, f32)],
    world_width: f32,
    world_height: f32,
) -> Result<Vec<f32>, String> {
    let results: Vec<f32> = positions.iter()
        .map(|(x, y)| {
            // Calculate ocean proximity
            let edge_dist_x = (x / world_width).min((world_width - x) / world_width);
            let edge_dist_y = (y / world_height).min((world_height - y) / world_height);
            let ocean_proximity = 1.0 - edge_dist_x.min(edge_dist_y) * 2.0;
            let ocean_proximity = ocean_proximity.clamp(0.0, 1.0);
            
            // Maritime influence curve (stronger near coasts)
            ocean_proximity * (1.0 + ocean_proximity * 0.5)
        })
        .collect();
    
    Ok(results)
}

/// Gaussian smoothing for climate data using Zig SIMD
#[cfg(not(feature = "no_zig"))]
pub fn climate_gaussian_smoothing(
    positions: &[(f32, f32)],
    climates: &[ClimateData],
    kernel_size: u32,
    sigma: f32,
) -> Result<Vec<ClimateData>, String> {
    if positions.len() != climates.len() {
        return Err("Position and climate arrays must have same length".to_string());
    }
    
    let len = positions.len().min(256);
    
    let positions_x: Vec<f32> = positions.iter().map(|(x, _)| *x).collect();
    let positions_y: Vec<f32> = positions.iter().map(|(_, y)| *y).collect();
    let temps: Vec<f32> = climates.iter().map(|c| c.temperature).collect();
    let rain: Vec<f32> = climates.iter().map(|c| c.rainfall).collect();
    let hum: Vec<f32> = climates.iter().map(|c| c.humidity).collect();
    let wind: Vec<f32> = climates.iter().map(|c| c.wind_strength).collect();
    
    let mut result_temps = vec![0.0f32; len];
    let mut result_rain = vec![0.0f32; len];
    let mut result_hum = vec![0.0f32; len];
    let mut result_wind = vec![0.0f32; len];
    
    unsafe {
        manifest_climate_gaussian_smoothing(
            positions_x.as_ptr(),
            positions_y.as_ptr(),
            temps.as_ptr(),
            rain.as_ptr(),
            hum.as_ptr(),
            wind.as_ptr(),
            kernel_size,
            sigma,
            len,
            result_temps.as_mut_ptr(),
            result_rain.as_mut_ptr(),
            result_hum.as_mut_ptr(),
            result_wind.as_mut_ptr(),
        );
    }
    
    let results: Vec<ClimateData> = (0..len).map(|i| ClimateData {
        temperature: result_temps[i],
        rainfall: result_rain[i],
        humidity: result_hum[i],
        wind_strength: result_wind[i],
    }).collect();
    
    Ok(results)
}

#[cfg(feature = "no_zig")]
pub fn climate_gaussian_smoothing(
    positions: &[(f32, f32)],
    climates: &[ClimateData],
    kernel_size: u32,
    sigma: f32,
) -> Result<Vec<ClimateData>, String> {
    // Simple fallback - just return the input climates
    Ok(climates.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_climate_orographic_effects() {
        let positions = vec![(100.0, 100.0), (200.0, 200.0)];
        let elevations = vec![0.0, 1500.0];
        let wind_directions = vec![0.0, 0.0];
        let params = ClimateParams::default();
        
        let results = climate_orographic_effects(&positions, &elevations, &wind_directions, &params);
        assert!(results.is_ok());
        
        let effects = results.unwrap();
        assert_eq!(effects.len(), 2);
        
        // Higher elevation should have enhanced precipitation
        assert!(effects[1] > effects[0]);
    }

    #[test]  
    fn test_climate_continental_effects() {
        let positions = vec![(10.0, 10.0), (128.0, 128.0)]; // Coast vs inland
        let base_temps = vec![20i8, 20i8];
        let base_humidity = vec![60u8, 60u8];
        let params = ClimateParams::default();
        
        let results = climate_continental_effects(&positions, &base_temps, &base_humidity, &params);
        assert!(results.is_ok());
        
        let (temps, humidity) = results.unwrap();
        assert_eq!(temps.len(), 2);
        assert_eq!(humidity.len(), 2);
        
        // Continental (inland) should have different temperature than coastal
        assert!(temps[0] != temps[1] || humidity[0] != humidity[1]);
    }

    #[test]
    fn test_climate_seasonal_temperature() {
        let base_temps = vec![20i8, 15i8];
        let zones = vec![ClimateZone::Temperate, ClimateZone::Polar];
        let latitudes = vec![45.0, 70.0];
        let variations = [2.0, 5.0, 15.0, 25.0, 12.0, 8.0]; // Default values
        
        let summer = climate_seasonal_temperature(&base_temps, &zones, &latitudes, 0.25, &variations);
        let winter = climate_seasonal_temperature(&base_temps, &zones, &latitudes, 0.75, &variations);
        
        assert!(summer.is_ok() && winter.is_ok());
        
        let summer_temps = summer.unwrap();
        let winter_temps = winter.unwrap();
        
        // Seasonal variation should be present
        assert!(summer_temps != winter_temps);
    }

    #[test]
    fn test_climate_ocean_proximity() {
        let positions = vec![(10.0, 10.0), (128.0, 128.0)]; // Coast vs inland
        
        let results = climate_ocean_proximity(&positions, 256.0, 256.0);
        assert!(results.is_ok());
        
        let proximity = results.unwrap();
        assert_eq!(proximity.len(), 2);
        
        // Coastal should be more oceanic than inland
        assert!(proximity[0] > proximity[1]);
        assert!(proximity[0] >= 0.0 && proximity[0] <= 1.0);
        assert!(proximity[1] >= 0.0 && proximity[1] <= 1.0);
    }

    #[test]
    fn test_climate_process_all() {
        let positions = vec![(100.0, 100.0), (150.0, 150.0)];
        let elevations = vec![500.0, 1500.0];
        let base_temps = vec![20i8, 15i8];
        let base_rainfall = vec![100.0, 200.0];
        let base_humidity = vec![60u8, 70u8];
        let wind_directions = vec![0.0, 0.0];
        
        let results = climate_process_all(
            &positions, &elevations, &base_temps, 
            &base_rainfall, &base_humidity, &wind_directions
        );
        
        assert!(results.is_ok());
        let (temps, rainfall, humidity) = results.unwrap();
        
        assert_eq!(temps.len(), 2);
        assert_eq!(rainfall.len(), 2);  
        assert_eq!(humidity.len(), 2);
        
        // Results should be modified from base values
        assert!(temps != base_temps || rainfall != base_rainfall || humidity != base_humidity);
    }

    #[test]
    fn test_climate_params_default() {
        let params = ClimateParams::default();
        assert_eq!(params.max_orographic_bonus, 200.0);
        assert_eq!(params.rain_shadow_factor, 0.6);
        assert_eq!(params.temperature_amplification, 1.5);
        assert_eq!(params.humidity_reduction, 0.8);
        assert_eq!(params.world_width, 256.0);
        assert_eq!(params.world_height, 256.0);
    }

    #[test]
    fn test_climate_zone_enum() {
        assert_eq!(ClimateZone::Equatorial as u8, 0);
        assert_eq!(ClimateZone::Tropical as u8, 1);
        assert_eq!(ClimateZone::Temperate as u8, 2);
        assert_eq!(ClimateZone::Polar as u8, 3);
        assert_eq!(ClimateZone::Desert as u8, 4);
        assert_eq!(ClimateZone::Mediterranean as u8, 5);
    }

    #[test]
    fn test_climate_rain_shadow_effects() {
        let positions = vec![(100.0, 100.0), (200.0, 200.0)];
        let elevations = vec![500.0, 1000.0];
        let mountain_ranges = vec![
            MountainRange {
                center: (150.0, 150.0),
                width: 100.0,
                height: 2000.0,
                orientation: 0.0,
            }
        ];
        
        let results = climate_rain_shadow_effects(
            &positions, 
            &elevations, 
            &mountain_ranges, 
            0.0, 
            0.6
        );
        
        assert!(results.is_ok());
        let effects = results.unwrap();
        assert_eq!(effects.len(), 2);
        assert!(effects[0] >= 0.0 && effects[0] <= 1.0);
        assert!(effects[1] >= 0.0 && effects[1] <= 1.0);
    }

    #[test]
    fn test_climate_monsoon_effects() {
        let positions = vec![(100.0, 100.0), (200.0, 200.0)];
        let seasonal_state = SeasonalState {
            current_season: 0.4,
            year_progress: 0.4,
            hemisphere_modifier: 1.0,
        };
        
        let results = climate_monsoon_effects(&positions, seasonal_state, 100.0);
        
        assert!(results.is_ok());
        let effects = results.unwrap();
        assert_eq!(effects.len(), 2);
    }

    #[test]
    fn test_climate_maritime_influence() {
        let positions = vec![(10.0, 10.0), (128.0, 128.0)]; // Coast vs inland
        
        let results = climate_maritime_influence(&positions, 256.0, 256.0);
        
        assert!(results.is_ok());
        let influence = results.unwrap();
        assert_eq!(influence.len(), 2);
        
        // Coastal should have more maritime influence than inland
        assert!(influence[0] > influence[1]);
        assert!(influence[0] >= 0.0 && influence[0] <= 2.0); // Maritime influence can be > 1.0
        assert!(influence[1] >= 0.0 && influence[1] <= 2.0);
    }

    #[test]
    fn test_climate_interpolation_params_default() {
        let params = InterpolationParams::default();
        assert_eq!(params.temperature_weight, 1.0);
        assert_eq!(params.rainfall_weight, 1.0);
        assert_eq!(params.humidity_weight, 1.0);
        assert_eq!(params.wind_weight, 0.5);
        assert_eq!(params.distance_falloff, 1.0);
        assert_eq!(params.max_influence_distance, 5.0);
    }

    #[test]
    fn test_climate_gaussian_smoothing() {
        let positions = vec![(100.0, 100.0), (110.0, 110.0)];
        let climates = vec![
            ClimateData { temperature: 20.0, rainfall: 100.0, humidity: 50.0, wind_strength: 10.0 },
            ClimateData { temperature: 25.0, rainfall: 120.0, humidity: 60.0, wind_strength: 15.0 },
        ];
        
        let results = climate_gaussian_smoothing(&positions, &climates, 3, 1.0);
        
        assert!(results.is_ok());
        let smoothed = results.unwrap();
        assert_eq!(smoothed.len(), 2);
    }
}

/// Calculate seasonal rainfall variations using Zig SIMD optimizations
#[cfg(not(feature = "no_zig"))]
pub fn climate_seasonal_rainfall(
    base_rainfall: &[u16],
    climate_zones: &[u8], 
    latitudes: &[f32],
    current_season: f32,
    rainfall_variations: &[f32],
) -> Vec<u16> {
    let count = base_rainfall.len();
    let mut results = vec![0u16; count];
    
    unsafe {
        manifest_climate_seasonal_rainfall(
            base_rainfall.as_ptr(),
            climate_zones.as_ptr(),
            latitudes.as_ptr(),
            current_season,
            rainfall_variations.as_ptr(),
            count,
            results.as_mut_ptr(),
        );
    }
    
    results
}

/// Calculate seasonal rainfall variations using pure Rust fallback
#[cfg(feature = "no_zig")]
pub fn climate_seasonal_rainfall(
    base_rainfall: &[u16],
    climate_zones: &[u8], 
    latitudes: &[f32],
    current_season: f32,
    rainfall_variations: &[f32],
) -> Vec<u16> {
    // Simple fallback implementation
    let mut results = Vec::with_capacity(base_rainfall.len());
    
    for (i, &base_rain) in base_rainfall.iter().enumerate() {
        let zone = climate_zones.get(i).copied().unwrap_or(0) as usize;
        let latitude = latitudes.get(i).copied().unwrap_or(0.0);
        
        // Apply seasonal variation based on climate zone and latitude
        let zone_variation = if zone < rainfall_variations.len() {
            rainfall_variations[zone]
        } else {
            1.0
        };
        
        // Simple seasonal effect based on latitude and current season
        let seasonal_factor = 1.0 + (current_season * std::f32::consts::PI * 2.0 + latitude * 0.1).sin() * zone_variation * 0.3;
        
        let adjusted_rain = (base_rain as f32 * seasonal_factor.max(0.1)) as u16;
        results.push(adjusted_rain);
    }
    
    results
}
