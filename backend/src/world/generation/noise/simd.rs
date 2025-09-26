//! SIMD-optimized noise operations using Zig FFI
//!
//! Provides high-performance batch noise calculations through
//! Zig SIMD implementation with fallback to Rust implementations.

use super::{NoiseConfig, NoiseResult};
use crate::core::zig_ffi;
use std::os::raw::{c_float, c_int, c_uint};

/// C struct for noise parameters passed to Zig
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NoiseParamsC {
    pub frequency: c_float,
    pub amplitude: c_float,
    pub octaves: c_uint,
    pub lacunarity: c_float,
    pub persistence: c_float,
    pub seed: c_uint,
}

/// C struct for coordinate pairs
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CoordPairC {
    pub x: c_float,
    pub y: c_float,
}

/// C struct for noise results
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NoiseResultC {
    pub height: c_float,
    pub temperature: c_float,
    pub moisture: c_float,
}

// External Zig SIMD functions
extern "C" {
    /// Batch simplex noise calculation using SIMD
    fn manifest_noise_simplex_batch_simd(
        coords: *const CoordPairC,
        params: *const NoiseParamsC,
        results: *mut c_float,
        count: c_uint,
    );

    /// Batch Perlin noise calculation using SIMD
    fn manifest_noise_perlin_batch_simd(
        coords: *const CoordPairC,
        params: *const NoiseParamsC,
        results: *mut c_float,
        count: c_uint,
    );

    /// Batch FBM calculation using SIMD
    fn manifest_noise_fbm_batch_simd(
        coords: *const CoordPairC,
        params: *const NoiseParamsC,
        results: *mut c_float,
        count: c_uint,
    );

    /// Batch domain warping using SIMD
    fn manifest_noise_domain_warp_batch_simd(
        coords: *const CoordPairC,
        warp_params: *const NoiseParamsC,
        warped_coords: *mut CoordPairC,
        count: c_uint,
    );

    /// Batch ridged noise calculation
    fn manifest_noise_ridged_batch_simd(
        coords: *const CoordPairC,
        params: *const NoiseParamsC,
        results: *mut c_float,
        count: c_uint,
    );

    /// Batch noise mixing operations
    fn manifest_noise_mix_batch_simd(
        noise1: *const c_float,
        noise2: *const c_float,
        weights: *const c_float,
        results: *mut c_float,
        count: c_uint,
        operation: c_int, // 0=add, 1=mul, 2=max, 3=min, 4=blend
    );

    /// Multi-layered noise sampling (height, temperature, moisture)
    fn manifest_noise_multilayer_batch_simd(
        coords: *const CoordPairC,
        height_params: *const NoiseParamsC,
        temp_params: *const NoiseParamsC,
        moisture_params: *const NoiseParamsC,
        results: *mut NoiseResultC,
        count: c_uint,
    );
}

/// High-performance SIMD batch noise sampling
pub fn batch_noise_sample(coords: &[(f64, f64)], config: &NoiseConfig) -> Vec<NoiseResult> {
    if coords.is_empty() {
        return Vec::new();
    }

    #[cfg(not(feature = "no_zig"))]
    {
        batch_noise_sample_zig(coords, config)
    }

    #[cfg(feature = "no_zig")]
    {
        batch_noise_sample_fallback(coords, config)
    }
}

/// Zig SIMD implementation with safety checks
#[cfg(not(feature = "no_zig"))]
fn batch_noise_sample_zig(coords: &[(f64, f64)], config: &NoiseConfig) -> Vec<NoiseResult> {
    // Safety check: empty input
    if coords.is_empty() {
        return Vec::new();
    }
    
    // Safety check: reasonable upper limit to prevent memory exhaustion
    const MAX_BATCH_SIZE: usize = 1_000_000;
    if coords.len() > MAX_BATCH_SIZE {
        panic!("Batch size {} exceeds maximum allowed size {}", coords.len(), MAX_BATCH_SIZE);
    }
    
    let count = coords.len() as c_uint;
    
    // Convert coordinates to C format with bounds checking
    let c_coords: Vec<CoordPairC> = coords
        .iter()
        .map(|(x, y)| {
            // Check for valid floating point values
            if !x.is_finite() || !y.is_finite() {
                panic!("Invalid coordinate values: x={}, y={}", x, y);
            }
            CoordPairC {
                x: *x as c_float,
                y: *y as c_float,
            }
        })
        .collect();

    // Prepare noise parameters with validation
    let height_params = NoiseParamsC {
        frequency: (*config.simplex.frequency).max(0.0001) as c_float, // Prevent division by zero
        amplitude: (*config.simplex.amplitude).abs() as c_float, // Use absolute value
        octaves: config.simplex.octaves.min(32) as c_uint, // Reasonable octave limit
        lacunarity: (*config.simplex.lacunarity).max(1.0001) as c_float, // Must be > 1
        persistence: (*config.simplex.persistence).clamp(0.0, 1.0) as c_float, // 0-1 range
        seed: config.seed as c_uint,
    };

    let temp_params = NoiseParamsC {
        frequency: (*config.fbm.frequency).max(0.0001) as c_float,
        amplitude: 1.0,
        octaves: config.fbm.octaves.min(32) as c_uint,
        lacunarity: (*config.fbm.lacunarity).max(1.0001) as c_float,
        persistence: (*config.fbm.persistence).clamp(0.0, 1.0) as c_float,
        seed: (config.seed + 1234) as c_uint,
    };

    let moisture_params = NoiseParamsC {
        frequency: ((*config.domain_warp.frequency * 2.0).max(0.0001)) as c_float,
        amplitude: 1.0,
        octaves: 3,
        lacunarity: 2.0,
        persistence: 0.6,
        seed: (config.seed + 5678) as c_uint,
    };

    // Allocate result buffer - explicitly zero initialize for safety
    let mut results = vec![NoiseResultC { height: 0.0, temperature: 0.0, moisture: 0.0 }; coords.len()];
    
    // Safety assertion: ensure buffer sizes match
    debug_assert_eq!(c_coords.len(), results.len());
    debug_assert_eq!(c_coords.len(), coords.len());

    // Call Zig SIMD function with additional safety checks
    unsafe {
        // Verify pointers are not null
        debug_assert!(!c_coords.as_ptr().is_null());
        debug_assert!(!results.as_mut_ptr().is_null());
        
        manifest_noise_multilayer_batch_simd(
            c_coords.as_ptr(),
            &height_params,
            &temp_params,
            &moisture_params,
            results.as_mut_ptr(),
            count,
        );
    }

    // Convert back to Rust format with validation
    results
        .into_iter()
        .map(|r| {
            // Validate results from Zig
            let height = if r.height.is_finite() { r.height } else { 0.0 };
            let temperature = if r.temperature.is_finite() { r.temperature.clamp(0.0, 1.0) } else { 0.5 };
            let moisture = if r.moisture.is_finite() { r.moisture.clamp(0.0, 1.0) } else { 0.5 };
            
            NoiseResult {
                height,
                temperature,
                moisture,
            }
        })
        .collect()
}

/// Fallback implementation when Zig is not available
#[cfg(feature = "no_zig")]
fn batch_noise_sample_fallback(coords: &[(f64, f64)], _config: &NoiseConfig) -> Vec<NoiseResult> {
    // Simple fallback - in a real implementation this would use simdnoise
    coords
        .iter()
        .map(|(x, y)| {
            let hash = ((x * 73856093.0) + (y * 19349663.0)) as u32;
            let height = ((hash % 2000) as f32 / 1000.0) - 1.0;
            let temperature = ((hash >> 8 % 1000) as f32 / 1000.0);
            let moisture = ((hash >> 16 % 1000) as f32 / 1000.0);

            NoiseResult {
                height,
                temperature,
                moisture,
            }
        })
        .collect()
}

/// SIMD-optimized domain warping
pub fn batch_domain_warp(coords: &[(f64, f64)], config: &NoiseConfig) -> Vec<(f64, f64)> {
    if coords.is_empty() {
        return Vec::new();
    }

    #[cfg(not(feature = "no_zig"))]
    {
        batch_domain_warp_zig(coords, config)
    }

    #[cfg(feature = "no_zig")]
    {
        batch_domain_warp_fallback(coords, config)
    }
}

#[cfg(not(feature = "no_zig"))]
fn batch_domain_warp_zig(coords: &[(f64, f64)], config: &NoiseConfig) -> Vec<(f64, f64)> {
    // Safety check: empty input
    if coords.is_empty() {
        return Vec::new();
    }
    
    // Safety check: reasonable upper limit
    const MAX_BATCH_SIZE: usize = 1_000_000;
    if coords.len() > MAX_BATCH_SIZE {
        return coords.iter().map(|&(x, y)| (x, y)).collect(); // Return unchanged if too large
    }
    
    let count = coords.len() as c_uint;
    
    let c_coords: Vec<CoordPairC> = coords
        .iter()
        .map(|(x, y)| {
            // Validate input coordinates
            if !x.is_finite() || !y.is_finite() {
                return CoordPairC { x: 0.0, y: 0.0 }; // Use origin for invalid coordinates
            }
            CoordPairC {
                x: *x as c_float,
                y: *y as c_float,
            }
        })
        .collect();

    let warp_params = NoiseParamsC {
        frequency: (*config.domain_warp.frequency).max(0.0001) as c_float, // Prevent zero frequency
        amplitude: (*config.domain_warp.amplitude).abs().min(10000.0) as c_float, // Reasonable amplitude limit
        octaves: config.domain_warp.iterations.min(16) as c_uint, // Limit iterations
        lacunarity: 2.0,
        persistence: 0.5,
        seed: config.seed as c_uint,
    };

    let mut warped_coords = vec![CoordPairC { x: 0.0, y: 0.0 }; coords.len()];
    
    // Safety assertions
    debug_assert_eq!(c_coords.len(), warped_coords.len());
    debug_assert_eq!(c_coords.len(), coords.len());

    unsafe {
        // Verify pointers are not null
        debug_assert!(!c_coords.as_ptr().is_null());
        debug_assert!(!warped_coords.as_mut_ptr().is_null());
        
        manifest_noise_domain_warp_batch_simd(
            c_coords.as_ptr(),
            &warp_params,
            warped_coords.as_mut_ptr(),
            count,
        );
    }

    warped_coords
        .into_iter()
        .enumerate()
        .map(|(i, c)| {
            // Validate warped coordinates and fallback to original if invalid
            let warped_x = if c.x.is_finite() { c.x as f64 } else { coords[i].0 };
            let warped_y = if c.y.is_finite() { c.y as f64 } else { coords[i].1 };
            (warped_x, warped_y)
        })
        .collect()
}

#[cfg(feature = "no_zig")]
fn batch_domain_warp_fallback(coords: &[(f64, f64)], config: &NoiseConfig) -> Vec<(f64, f64)> {
    // Simple domain warping fallback
    coords
        .iter()
        .map(|(x, y)| {
            let warp_x = *config.domain_warp.amplitude * 0.01 * (x * 0.02).sin();
            let warp_y = *config.domain_warp.amplitude * 0.01 * (y * 0.02).cos();
            (x + warp_x, y + warp_y)
        })
        .collect()
}

/// SIMD-optimized noise mixing
pub fn batch_noise_mix(
    noise1: &[f32],
    noise2: &[f32],
    weights: &[f32],
    operation: MixOperation,
) -> Vec<f32> {
    if noise1.len() != noise2.len() || noise1.len() != weights.len() {
        return Vec::new();
    }

    #[cfg(not(feature = "no_zig"))]
    {
        batch_noise_mix_zig(noise1, noise2, weights, operation)
    }

    #[cfg(feature = "no_zig")]
    {
        batch_noise_mix_fallback(noise1, noise2, weights, operation)
    }
}

/// Mix operation enum for SIMD
pub enum MixOperation {
    Add = 0,
    Multiply = 1,
    Max = 2,
    Min = 3,
    Blend = 4,
}

#[cfg(not(feature = "no_zig"))]
fn batch_noise_mix_zig(
    noise1: &[f32],
    noise2: &[f32],
    weights: &[f32],
    operation: MixOperation,
) -> Vec<f32> {
    // Safety check: arrays must have the same length
    if noise1.len() != noise2.len() || noise1.len() != weights.len() {
        panic!("Array length mismatch: noise1={}, noise2={}, weights={}", 
               noise1.len(), noise2.len(), weights.len());
    }
    
    // Safety check: empty input
    if noise1.is_empty() {
        return Vec::new();
    }
    
    // Safety check: reasonable size limit
    const MAX_BATCH_SIZE: usize = 10_000_000;
    if noise1.len() > MAX_BATCH_SIZE {
        return batch_noise_mix_fallback(noise1, noise2, weights, operation);
    }
    
    // Validate input arrays contain finite values
    let valid_noise1 = noise1.iter().all(|&x| x.is_finite());
    let valid_noise2 = noise2.iter().all(|&x| x.is_finite());
    let valid_weights = weights.iter().all(|&x| x.is_finite());
    
    if !valid_noise1 || !valid_noise2 || !valid_weights {
        // Use fallback for invalid input
        return batch_noise_mix_fallback(noise1, noise2, weights, operation);
    }
    
    let count = noise1.len() as c_uint;
    let mut results = vec![0.0f32; noise1.len()];
    
    // Safety assertions
    debug_assert_eq!(noise1.len(), noise2.len());
    debug_assert_eq!(noise1.len(), weights.len());
    debug_assert_eq!(noise1.len(), results.len());

    unsafe {
        // Verify pointers are not null
        debug_assert!(!noise1.as_ptr().is_null());
        debug_assert!(!noise2.as_ptr().is_null());
        debug_assert!(!weights.as_ptr().is_null());
        debug_assert!(!results.as_mut_ptr().is_null());
        
        manifest_noise_mix_batch_simd(
            noise1.as_ptr(),
            noise2.as_ptr(),
            weights.as_ptr(),
            results.as_mut_ptr(),
            count,
            operation as c_int,
        );
    }

    // Validate results and clamp to reasonable ranges
    results
        .into_iter()
        .map(|x| if x.is_finite() { x.clamp(-100.0, 100.0) } else { 0.0 })
        .collect()
}

// Fallback implementation for when SIMD fails or no_zig feature is enabled
fn batch_noise_mix_fallback(
    noise1: &[f32],
    noise2: &[f32],
    weights: &[f32],
    operation: MixOperation,
) -> Vec<f32> {
    noise1
        .iter()
        .zip(noise2.iter())
        .zip(weights.iter())
        .map(|((n1, n2), weight)| match operation {
            MixOperation::Add => n1 + n2 * weight,
            MixOperation::Multiply => n1 * (1.0 + n2 * weight),
            MixOperation::Max => n1.max(*n2),
            MixOperation::Min => n1.min(*n2),
            MixOperation::Blend => n1 * (1.0 - weight) + n2 * weight,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::generation::noise::NoiseConfig;
    use std::mem;

    #[test]
    fn test_c_struct_alignment() {
        // Verify C struct sizes match expectations
        assert_eq!(mem::size_of::<NoiseParamsC>(), 6 * mem::size_of::<f32>());
        assert_eq!(mem::size_of::<CoordPairC>(), 2 * mem::size_of::<f32>());
        assert_eq!(mem::size_of::<NoiseResultC>(), 3 * mem::size_of::<f32>());
        
        // Verify alignment
        assert_eq!(mem::align_of::<NoiseParamsC>(), mem::align_of::<f32>());
        assert_eq!(mem::align_of::<CoordPairC>(), mem::align_of::<f32>());
        assert_eq!(mem::align_of::<NoiseResultC>(), mem::align_of::<f32>());
    }

    #[test]
    fn test_batch_noise_sample() {
        let config = NoiseConfig::default();
        let coords = vec![(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)];
        
        let results = batch_noise_sample(&coords, &config);
        assert_eq!(results.len(), 3);
        
        for result in results {
            assert!(result.height >= -1.0 && result.height <= 1.0);
            assert!(result.temperature >= 0.0 && result.temperature <= 1.0);
            assert!(result.moisture >= 0.0 && result.moisture <= 1.0);
        }
    }

    #[test]
    fn test_batch_noise_sample_empty() {
        let config = NoiseConfig::default();
        let coords = vec![];
        
        let results = batch_noise_sample(&coords, &config);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_batch_domain_warp() {
        let config = NoiseConfig::default();
        let coords = vec![(0.0, 0.0), (10.0, 10.0)];
        
        let warped = batch_domain_warp(&coords, &config);
        assert_eq!(warped.len(), 2);
        
        // Warped coordinates should be finite
        for (x, y) in warped {
            assert!(x.is_finite());
            assert!(y.is_finite());
        }
    }

    #[test]
    fn test_batch_noise_mix() {
        let noise1 = vec![0.5, 0.3, 0.8];
        let noise2 = vec![0.2, 0.7, 0.1];
        let weights = vec![0.5, 0.5, 0.5];
        
        let mixed = batch_noise_mix(&noise1, &noise2, &weights, MixOperation::Blend);
        assert_eq!(mixed.len(), 3);
        
        // Check blend operation results are finite
        for value in mixed {
            assert!(value.is_finite());
            assert!(value >= -100.0 && value <= 100.0);
        }
    }

    #[test]
    #[should_panic(expected = "Array length mismatch")]
    fn test_batch_noise_mix_length_mismatch() {
        let noise1 = vec![0.5, 0.3];
        let noise2 = vec![0.2, 0.7, 0.1];
        let weights = vec![0.5, 0.5, 0.5];
        
        batch_noise_mix(&noise1, &noise2, &weights, MixOperation::Add);
    }

    #[test]
    fn test_invalid_coordinates_handling() {
        let config = NoiseConfig::default();
        let coords = vec![(f64::NAN, 0.0), (f64::INFINITY, 1.0), (0.0, f64::NEG_INFINITY)];
        
        // Should not panic and return valid results
        let results = batch_noise_sample(&coords, &config);
        assert_eq!(results.len(), 3);
        
        for result in results {
            assert!(result.height.is_finite());
            assert!(result.temperature.is_finite());
            assert!(result.moisture.is_finite());
        }
    }
}
