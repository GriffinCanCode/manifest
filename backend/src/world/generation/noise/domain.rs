//! Domain warping for organic noise distortion
//!
//! Advanced domain warping using multiple warp iterations and
//! configurable distortion patterns for natural-looking terrain.

use super::types::*;
use super::core::SimplexGenerator;
use crate::core::hashing::HashStrategies;
use ordered_float::OrderedFloat;
use rayon::prelude::*;

/// Advanced domain warping generator with multi-iteration support
#[derive(Debug)]
pub struct DomainWarpGenerator {
    config: DomainWarpConfig,
    warp_x: SimplexGenerator,
    warp_y: SimplexGenerator,
    warp_z: SimplexGenerator, // For 3D warping and rotation effects
}

impl DomainWarpGenerator {
    /// Create new domain warp generator
    pub fn new(config: &DomainWarpConfig) -> Self {
        // Create specialized simplex configs for warping
        let warp_config = SimplexConfig {
            frequency: config.frequency,
            amplitude: OrderedFloat(1.0),
            octaves: 3,
            lacunarity: OrderedFloat(2.0),
            persistence: OrderedFloat(0.5),
            quality: NoiseQuality::Medium,
        };
        
        // Use different seeds for each axis to avoid correlation
        let base_seed = HashStrategies::hash_bytes(b"domain_warp_base");
        
        Self {
            config: config.clone(),
            warp_x: SimplexGenerator::new(&warp_config, base_seed),
            warp_y: SimplexGenerator::new(&warp_config, base_seed.wrapping_add(1001)),
            warp_z: SimplexGenerator::new(&warp_config, base_seed.wrapping_add(2002)),
        }
    }

    /// Apply domain warping to coordinates
    pub fn warp(&self, x: f64, y: f64) -> (f64, f64) {
        let mut current_x = x;
        let mut current_y = y;

        // Apply multiple warp iterations for more complex distortion
        for i in 0..self.config.iterations {
            let iteration_scale = 1.0 + i as f64 * 0.1;
            let freq_scale = *self.config.frequency * iteration_scale;
            
            // Apply rotation to avoid grid artifacts
            let (rot_x, rot_y) = self.rotate_coordinates(current_x, current_y, *self.config.rotation);
            
            // Sample warp vectors
            let warp_x = self.warp_x.sample_uncached(rot_x * freq_scale, rot_y * freq_scale) as f64;
            let warp_y = self.warp_y.sample_uncached(
                (rot_x + 5.12) * freq_scale, 
                (rot_y + 1.93) * freq_scale
            ) as f64;
            
            // Apply warping with amplitude scaling
            let amplitude_scale = *self.config.amplitude / (i + 1) as f64;
            current_x += warp_x * amplitude_scale;
            current_y += warp_y * amplitude_scale;
        }

        (current_x, current_y)
    }

    /// Apply curl-based warping for fluid-like effects
    pub fn warp_curl(&self, x: f64, y: f64) -> (f64, f64) {
        let freq = *self.config.frequency;
        let amplitude = *self.config.amplitude;
        let epsilon = 0.001;
        
        // Sample potential field
        let potential = self.warp_x.sample_uncached(x * freq, y * freq) as f64;
        let potential_dx = self.warp_x.sample_uncached((x + epsilon) * freq, y * freq) as f64;
        let potential_dy = self.warp_x.sample_uncached(x * freq, (y + epsilon) * freq) as f64;
        
        // Calculate curl (perpendicular gradient)
        let curl_x = (potential_dy - potential) / epsilon;
        let curl_y = -(potential_dx - potential) / epsilon;
        
        (
            x + curl_x * amplitude,
            y + curl_y * amplitude,
        )
    }

    /// Batch warping with parallel processing
    pub fn warp_batch(&self, coordinates: &[(f64, f64)]) -> Vec<(f64, f64)> {
        if coordinates.len() > 100 {
            coordinates.par_iter()
                .map(|(x, y)| self.warp(*x, *y))
                .collect()
        } else {
            coordinates.iter()
                .map(|(x, y)| self.warp(*x, *y))
                .collect()
        }
    }

    /// Rotate coordinates by angle
    fn rotate_coordinates(&self, x: f64, y: f64, angle: f64) -> (f64, f64) {
        if angle.abs() < f64::EPSILON {
            return (x, y);
        }
        
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        
        (
            x * cos_a - y * sin_a,
            x * sin_a + y * cos_a,
        )
    }

    /// Get configuration
    pub fn config(&self) -> &DomainWarpConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_warp_creation() {
        let config = DomainWarpConfig::default();
        let warp = DomainWarpGenerator::new(&config);
        
        assert_eq!(warp.config.iterations, 1);
        assert_eq!(warp.config.warp_type, NoiseType::Simplex);
    }

    #[test]
    fn test_basic_warping() {
        let config = DomainWarpConfig::default();
        let warp = DomainWarpGenerator::new(&config);
        
        let (warped_x, warped_y) = warp.warp(0.0, 0.0);
        
        // Warped coordinates should be different from original
        assert_ne!(warped_x, 0.0);
        assert_ne!(warped_y, 0.0);
        
        // Test determinism
        let (warped_x2, warped_y2) = warp.warp(0.0, 0.0);
        assert_eq!(warped_x, warped_x2);
        assert_eq!(warped_y, warped_y2);
    }
}