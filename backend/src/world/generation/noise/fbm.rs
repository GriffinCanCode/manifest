//! Fractal Brownian Motion (FBM) noise generator
//!
//! Provides sophisticated fractal noise with multiple octaves,
//! lacunarity control, and domain warping capabilities.

use super::types::*;
use super::core::{SimplexGenerator, PerlinGenerator};
use ordered_float::OrderedFloat;

/// Fractal Brownian Motion generator
#[derive(Debug)]
pub struct FbmGenerator {
    config: FbmConfig,
    base_generator: Box<dyn NoiseGenerator>,
}

/// Trait for base noise generators used in FBM
trait NoiseGenerator: Send + Sync + std::fmt::Debug {
    fn sample(&self, x: f64, y: f64) -> f32;
}

impl NoiseGenerator for SimplexGenerator {
    fn sample(&self, x: f64, y: f64) -> f32 {
        self.sample_uncached(x, y)
    }
}

impl NoiseGenerator for PerlinGenerator {
    fn sample(&self, x: f64, y: f64) -> f32 {
        self.sample_uncached(x, y)
    }
}

impl FbmGenerator {
    /// Create new FBM generator
    pub fn new(config: &FbmConfig) -> Self {
        let base_generator: Box<dyn NoiseGenerator> = match config.base_type {
            NoiseType::Simplex => {
                let simplex_config = SimplexConfig {
                    frequency: config.frequency,
                    amplitude: OrderedFloat(1.0),
                    octaves: 1, // FBM handles octaves
                    lacunarity: config.lacunarity,
                    persistence: config.persistence,
                    quality: NoiseQuality::Medium,
                };
                Box::new(SimplexGenerator::new(&simplex_config, 12345))
            }
            NoiseType::Perlin => {
                let perlin_config = PerlinConfig {
                    frequency: config.frequency,
                    amplitude: OrderedFloat(1.0),
                    octaves: 1, // FBM handles octaves
                    lacunarity: config.lacunarity,
                    persistence: config.persistence,
                    quality: NoiseQuality::Medium,
                    interpolation: Interpolation::Quintic,
                };
                Box::new(PerlinGenerator::new(&perlin_config, 12345))
            }
            _ => {
                // Fallback to Simplex
                let simplex_config = SimplexConfig::default();
                Box::new(SimplexGenerator::new(&simplex_config, 12345))
            }
        };

        Self {
            config: config.clone(),
            base_generator,
        }
    }

    /// Sample standard FBM
    pub fn sample(&self, x: f64, y: f64) -> f32 {
        self.sample_uncached(x, y)
    }

    /// Sample FBM without caching
    pub fn sample_uncached(&self, x: f64, y: f64) -> f32 {
        let mut value = 0.0;
        let mut amplitude = 1.0;
        let mut frequency = *self.config.frequency;
        let mut max_value = 0.0; // For normalization

        for _ in 0..self.config.octaves {
            let sample = self.base_generator.sample(x * frequency, y * frequency) as f64;
            value += sample * amplitude;
            max_value += amplitude;

            amplitude *= *self.config.persistence;
            frequency *= *self.config.lacunarity;
        }

        // Normalize to [-1, 1] range
        if max_value > 0.0 {
            (value / max_value) as f32
        } else {
            0.0
        }
    }

    /// Sample ridged FBM (inverted and squared)
    pub fn sample_ridged(&self, x: f64, y: f64) -> f32 {
        let mut value = 0.0;
        let mut amplitude = 1.0;
        let mut frequency = *self.config.frequency;
        let mut max_value = 0.0;

        for _ in 0..self.config.octaves {
            let mut sample = self.base_generator.sample(x * frequency, y * frequency) as f64;
            
            // Ridged transformation
            sample = sample.abs();
            sample = 1.0 - sample;
            sample = sample * sample;

            value += sample * amplitude;
            max_value += amplitude;

            amplitude *= *self.config.persistence;
            frequency *= *self.config.lacunarity;
        }

        if max_value > 0.0 {
            (value / max_value) as f32
        } else {
            0.0
        }
    }

    /// Sample billow FBM (absolute value)
    pub fn sample_billow(&self, x: f64, y: f64) -> f32 {
        let mut value = 0.0;
        let mut amplitude = 1.0;
        let mut frequency = *self.config.frequency;
        let mut max_value = 0.0;

        for _ in 0..self.config.octaves {
            let sample = self.base_generator.sample(x * frequency, y * frequency) as f64;
            let billow_sample = sample.abs() * 2.0 - 1.0;
            
            value += billow_sample * amplitude;
            max_value += amplitude;

            amplitude *= *self.config.persistence;
            frequency *= *self.config.lacunarity;
        }

        if max_value > 0.0 {
            (value / max_value) as f32
        } else {
            0.0
        }
    }

    /// Sample with ping-pong effect
    pub fn sample_ping_pong(&self, x: f64, y: f64) -> f32 {
        let mut value = 0.0;
        let mut amplitude = 1.0;
        let mut frequency = *self.config.frequency;
        let ping_pong = *self.config.ping_pong_strength;

        for _ in 0..self.config.octaves {
            let sample = self.base_generator.sample(x * frequency, y * frequency) as f64;
            
            // Ping-pong effect
            let ping_ponged = if ping_pong != 0.0 {
                Self::ping_pong_transform(sample, ping_pong)
            } else {
                sample
            };

            value += ping_ponged * amplitude;

            amplitude *= *self.config.persistence;
            frequency *= *self.config.lacunarity;
        }

        value as f32
    }

    /// Sample weighted FBM with derivative control
    pub fn sample_weighted(&self, x: f64, y: f64) -> f32 {
        let mut value = 0.0;
        let mut amplitude = 1.0;
        let mut frequency = *self.config.frequency;
        let mut weight = 1.0;
        let weighted_strength = *self.config.weighted_strength;

        for _ in 0..self.config.octaves {
            let sample = self.base_generator.sample(x * frequency, y * frequency) as f64;
            
            // Apply weight
            let weighted_sample = sample * weight;
            value += weighted_sample * amplitude;

            // Update weight based on derivative
            weight = (weighted_sample + 1.0).clamp(0.0, 1.0);
            if weighted_strength > 0.0 {
                weight = weight.powf(weighted_strength);
            }

            amplitude *= *self.config.persistence;
            frequency *= *self.config.lacunarity;
        }

        value as f32
    }

    /// Sample for temperature generation
    pub fn sample_temperature(&self, x: f64, y: f64) -> f32 {
        // Use modified parameters for temperature
        let base = self.sample_uncached(x * 0.5, y * 0.5);
        let detail = self.base_generator.sample(x * 0.1, y * 0.1) * 0.3;
        
        // Temperature should be 0-1 range, with some variation
        let temp = (base as f64 + detail as f64) * 0.3 + 0.7;
        temp.clamp(0.0, 1.0) as f32
    }

    /// Ping-pong transformation function
    fn ping_pong_transform(value: f64, strength: f64) -> f64 {
        let t = 1.0 - ((value * strength).abs() % 2.0 - 1.0).abs();
        if value >= 0.0 { t } else { -t }
    }

    /// Sample with erosion effect for terrain
    pub fn sample_eroded(&self, x: f64, y: f64, erosion_strength: f32) -> f32 {
        let base = self.sample_uncached(x, y);
        let erosion = self.base_generator.sample(x * 3.0, y * 3.0) * erosion_strength;
        
        // Simulate erosion by reducing height in valleys
        if base < 0.0 {
            base + erosion.abs() * base.abs()
        } else {
            base - erosion.abs() * 0.1
        }
    }

    /// Sample with terrace effect
    pub fn sample_terraced(&self, x: f64, y: f64, terrace_count: u32) -> f32 {
        let base = self.sample_uncached(x, y);
        let steps = terrace_count as f32;
        
        // Create stepped/terraced effect
        (base * steps).floor() / steps
    }

    /// Batch sampling for performance
    pub fn sample_batch(&self, coords: &[(f64, f64)]) -> Vec<f32> {
        coords.iter()
            .map(|(x, y)| self.sample_uncached(*x, *y))
            .collect()
    }

    /// Get the configuration
    pub fn config(&self) -> &FbmConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fbm_creation() {
        let config = FbmConfig::default();
        let generator = FbmGenerator::new(&config);
        
        assert_eq!(generator.config.octaves, 6);
        assert_eq!(generator.config.base_type, NoiseType::Simplex);
    }

    #[test]
    fn test_fbm_sampling() {
        let config = FbmConfig::default();
        let generator = FbmGenerator::new(&config);
        
        let sample = generator.sample(0.0, 0.0);
        assert!(sample >= -1.0 && sample <= 1.0);
        
        // Test determinism
        let sample2 = generator.sample(0.0, 0.0);
        assert_eq!(sample, sample2);
    }

    #[test]
    fn test_fbm_variants() {
        let config = FbmConfig::default();
        let generator = FbmGenerator::new(&config);
        
        let ridged = generator.sample_ridged(0.0, 0.0);
        let billow = generator.sample_billow(0.0, 0.0);
        let ping_pong = generator.sample_ping_pong(0.0, 0.0);
        let weighted = generator.sample_weighted(0.0, 0.0);
        
        // All should be in valid range
        assert!(ridged >= -1.0 && ridged <= 1.0);
        assert!(billow >= -1.0 && billow <= 1.0);
        assert!(ping_pong >= -10.0 && ping_pong <= 10.0); // Ping-pong can be wider
        assert!(weighted >= -10.0 && weighted <= 10.0);
    }

    #[test]
    fn test_temperature_sampling() {
        let config = FbmConfig::default();
        let generator = FbmGenerator::new(&config);
        
        let temp = generator.sample_temperature(0.0, 0.0);
        assert!(temp >= 0.0 && temp <= 1.0);
    }

    #[test]
    fn test_batch_sampling() {
        let config = FbmConfig::default();
        let generator = FbmGenerator::new(&config);
        
        let coords = vec![(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)];
        let results = generator.sample_batch(&coords);
        
        assert_eq!(results.len(), 3);
        for result in results {
            assert!(result >= -1.0 && result <= 1.0);
        }
    }
}
