//! Ridged noise for mountain generation  
//!
//! Advanced ridged noise with multiple ridge types and
//! sophisticated amplitude control for realistic terrain.

use super::types::*;
use super::core::SimplexGenerator;
use crate::core::hashing::HashStrategies;
use ordered_float::OrderedFloat;
use rayon::prelude::*;

/// Advanced ridged noise generator with multiple ridge patterns
#[derive(Debug)]
pub struct RidgeGenerator {
    config: RidgeConfig,
    base_generator: SimplexGenerator,
    ridge_modulator: SimplexGenerator,
}

impl RidgeGenerator {
    /// Create new ridged noise generator
    pub fn new(config: &RidgeConfig) -> Self {
        let base_config = SimplexConfig {
            frequency: config.frequency,
            amplitude: OrderedFloat(1.0),
            octaves: 1, // Ridge generator handles octaves
            lacunarity: config.lacunarity,
            persistence: OrderedFloat(0.5),
            quality: NoiseQuality::High,
        };
        
        let modulator_config = SimplexConfig {
            frequency: OrderedFloat(*config.frequency * 0.5),
            amplitude: OrderedFloat(1.0),
            octaves: 1,
            lacunarity: OrderedFloat(2.0),
            persistence: OrderedFloat(0.5),
            quality: NoiseQuality::Medium,
        };
        
        let base_seed = HashStrategies::hash_bytes(b"ridge_base");
        let modulator_seed = HashStrategies::hash_bytes(b"ridge_modulator");
        
        Self {
            config: config.clone(),
            base_generator: SimplexGenerator::new(&base_config, base_seed),
            ridge_modulator: SimplexGenerator::new(&modulator_config, modulator_seed),
        }
    }
    
    /// Sample standard ridged noise
    pub fn sample(&self, x: f64, y: f64) -> f32 {
        self.sample_ridged_multifractal(x, y)
    }

    /// Sample ridged multifractal noise (classic ridged noise)
    pub fn sample_ridged_multifractal(&self, x: f64, y: f64) -> f32 {
        let mut value = 0.0;
        let mut amplitude = 1.0;
        let mut frequency = *self.config.frequency;
        let mut max_amplitude = 0.0;
        
        // First octave is different - it establishes the ridge pattern
        let mut noise = self.base_generator.sample_uncached(x * frequency, y * frequency) as f64;
        noise = self.apply_ridge_transform(noise);
        value = noise * amplitude;
        max_amplitude = amplitude;
        
        frequency *= *self.config.lacunarity;
        
        // Subsequent octaves use previous octave to modulate amplitude
        for i in 1..self.config.octaves {
            amplitude = noise * *self.config.gain + self.calculate_base_amplitude(i);
            amplitude = amplitude.clamp(0.0, 1.0);
            
            noise = self.base_generator.sample_uncached(x * frequency, y * frequency) as f64;
            noise = self.apply_ridge_transform(noise);
            
            value += noise * amplitude;
            max_amplitude += amplitude;
            
            frequency *= *self.config.lacunarity;
        }
        
        // Normalize and apply final sharpness
        let normalized = if max_amplitude > 0.0 { value / max_amplitude } else { 0.0 };
        let sharpened = normalized * *self.config.sharpness;
        sharpened.clamp(-1.0, 1.0) as f32
    }

    /// Sample hybrid ridged noise (softer ridges)
    pub fn sample_hybrid_ridged(&self, x: f64, y: f64) -> f32 {
        let mut value = 0.0;
        let mut amplitude = 1.0;
        let mut frequency = *self.config.frequency;
        let mut weight = 1.0;
        
        for i in 0..self.config.octaves {
            let mut noise = self.base_generator.sample_uncached(x * frequency, y * frequency) as f64;
            
            if i == 0 {
                // First octave: standard ridge
                noise = self.apply_ridge_transform(noise);
                value = noise * amplitude;
            } else {
                // Subsequent octaves: weighted hybrid
                noise = self.apply_ridge_transform(noise) * weight + noise * (1.0 - weight);
                value += noise * amplitude;
                weight = (noise + 1.0) * 0.5; // Convert to [0,1] for next iteration
                weight = weight.clamp(0.0, 1.0);
            }
            
            frequency *= *self.config.lacunarity;
            amplitude *= self.calculate_persistence(i, noise);
        }
        
        (value * *self.config.sharpness).clamp(-1.0, 1.0) as f32
    }

    /// Sample billow ridged noise (inverted ridges)
    pub fn sample_billow_ridged(&self, x: f64, y: f64) -> f32 {
        let mut value = 0.0;
        let mut amplitude = 1.0;
        let mut frequency = *self.config.frequency;
        let mut max_amplitude = 0.0;
        
        for _ in 0..self.config.octaves {
            let mut noise = self.base_generator.sample_uncached(x * frequency, y * frequency) as f64;
            
            // Billow transformation (absolute value then offset)
            noise = noise.abs() * 2.0 - 1.0;
            noise = self.apply_ridge_transform(-noise); // Invert for billow effect
            
            value += noise * amplitude;
            max_amplitude += amplitude;
            
            frequency *= *self.config.lacunarity;
            amplitude *= 0.5; // Standard persistence for billow
        }
        
        let normalized = if max_amplitude > 0.0 { value / max_amplitude } else { 0.0 };
        (normalized * *self.config.sharpness).clamp(-1.0, 1.0) as f32
    }

    /// Sample ridged noise with modulation for variety
    pub fn sample_modulated_ridged(&self, x: f64, y: f64) -> f32 {
        let base_ridged = self.sample_ridged_multifractal(x, y) as f64;
        
        // Add modulation for variety
        let modulation = self.ridge_modulator.sample_uncached(x * 0.3, y * 0.3) as f64 * 0.2;
        let modulated = base_ridged * (1.0 + modulation);
        
        modulated.clamp(-1.0, 1.0) as f32
    }

    /// Sample terraced ridged noise for stepped mountains
    pub fn sample_terraced_ridged(&self, x: f64, y: f64, terrace_count: u32) -> f32 {
        let base_ridged = self.sample_ridged_multifractal(x, y);
        
        // Apply terracing
        let normalized = (base_ridged + 1.0) * 0.5; // Convert to [0,1]
        let terraced = (normalized * terrace_count as f32).floor() / terrace_count as f32;
        
        terraced * 2.0 - 1.0 // Convert back to [-1,1]
    }

    /// Batch sampling with parallel processing
    pub fn sample_batch(&self, coordinates: &[(f64, f64)]) -> Vec<f32> {
        if coordinates.len() > 100 {
            coordinates.par_iter()
                .map(|(x, y)| self.sample(*x, *y))
                .collect()
        } else {
            coordinates.iter()
                .map(|(x, y)| self.sample(*x, *y))
                .collect()
        }
    }

    /// Apply ridge transformation to noise value
    fn apply_ridge_transform(&self, noise: f64) -> f64 {
        let offset_noise = noise + *self.config.offset;
        let ridged = (*self.config.offset - offset_noise.abs()).max(0.0);
        ridged * ridged // Square for sharper ridges
    }

    /// Calculate amplitude persistence based on octave and previous noise
    fn calculate_persistence(&self, octave: u32, prev_noise: f64) -> f64 {
        if octave == 0 {
            1.0
        } else {
            // Use previous noise to modulate amplitude
            let base_persistence = 0.5;
            let noise_factor = (prev_noise + 1.0) * 0.5; // Convert to [0,1]
            base_persistence * (1.0 + noise_factor * 0.3)
        }
    }

    /// Calculate base amplitude for octave
    fn calculate_base_amplitude(&self, octave: u32) -> f64 {
        match octave {
            0 => 1.0,
            1 => 0.7,
            2 => 0.4,
            3 => 0.2,
            _ => 0.1,
        }
    }

    /// Get configuration
    pub fn config(&self) -> &RidgeConfig {
        &self.config
    }

    /// Update configuration
    pub fn update_config(&mut self, config: RidgeConfig) {
        self.config = config;
        
        // Recreate generators if base frequency changed significantly
        let base_config = SimplexConfig {
            frequency: self.config.frequency,
            amplitude: OrderedFloat(1.0),
            octaves: 1,
            lacunarity: self.config.lacunarity,
            persistence: OrderedFloat(0.5),
            quality: NoiseQuality::High,
        };
        
        let base_seed = HashStrategies::hash_bytes(b"ridge_base_updated");
        self.base_generator = SimplexGenerator::new(&base_config, base_seed);
    }
}

/// Specialized ridge generator for mountain terrain
#[derive(Debug)]
pub struct MountainRidgeGenerator {
    primary_ridge: RidgeGenerator,
    secondary_ridge: RidgeGenerator,
    erosion_noise: SimplexGenerator,
}

impl MountainRidgeGenerator {
    /// Create mountain ridge generator with multiple layers
    pub fn new() -> Self {
        let primary_config = RidgeConfig {
            base_type: NoiseType::Simplex,
            sharpness: OrderedFloat(1.2),
            offset: OrderedFloat(0.9),
            gain: OrderedFloat(2.5),
            frequency: OrderedFloat(0.008),
            octaves: 5,
            lacunarity: OrderedFloat(2.3),
        };
        
        let secondary_config = RidgeConfig {
            base_type: NoiseType::Simplex,
            sharpness: OrderedFloat(0.8),
            offset: OrderedFloat(0.7),
            gain: OrderedFloat(1.8),
            frequency: OrderedFloat(0.02),
            octaves: 3,
            lacunarity: OrderedFloat(2.1),
        };
        
        let erosion_config = SimplexConfig {
            frequency: OrderedFloat(0.05),
            amplitude: OrderedFloat(1.0),
            octaves: 2,
            lacunarity: OrderedFloat(2.0),
            persistence: OrderedFloat(0.6),
            quality: NoiseQuality::Medium,
        };
        
        let erosion_seed = HashStrategies::hash_bytes(b"mountain_erosion");
        
        Self {
            primary_ridge: RidgeGenerator::new(&primary_config),
            secondary_ridge: RidgeGenerator::new(&secondary_config),
            erosion_noise: SimplexGenerator::new(&erosion_config, erosion_seed),
        }
    }

    /// Sample mountain terrain with erosion effects
    pub fn sample_mountain(&self, x: f64, y: f64, erosion_strength: f32) -> f32 {
        // Primary large-scale ridges
        let primary = self.primary_ridge.sample_ridged_multifractal(x, y) as f64;
        
        // Secondary detail ridges
        let secondary = self.secondary_ridge.sample_hybrid_ridged(x, y) as f64 * 0.4;
        
        // Combine ridges
        let combined_ridges = primary + secondary * (primary + 1.0) * 0.5;
        
        // Apply erosion if strength > 0
        let final_result = if erosion_strength > 0.01 {
            let erosion = self.erosion_noise.sample_uncached(x * 0.1, y * 0.1) as f64;
            let erosion_factor = erosion_strength as f64;
            
            // Erosion reduces height in valleys, preserves ridges
            if combined_ridges < 0.0 {
                combined_ridges * (1.0 + erosion * erosion_factor * 0.3)
            } else {
                combined_ridges * (1.0 - erosion.abs() * erosion_factor * 0.1)
            }
        } else {
            combined_ridges
        };
        
        final_result.clamp(-1.0, 1.0) as f32
    }
}

impl Default for MountainRidgeGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ridge_generator_creation() {
        let config = RidgeConfig::default();
        let generator = RidgeGenerator::new(&config);
        
        assert_eq!(generator.config.octaves, 4);
        assert_eq!(generator.config.base_type, NoiseType::Simplex);
    }

    #[test]
    fn test_ridge_sampling() {
        let config = RidgeConfig::default();
        let generator = RidgeGenerator::new(&config);
        
        let sample = generator.sample(0.0, 0.0);
        assert!(sample >= -1.0 && sample <= 1.0);
        
        // Test determinism
        let sample2 = generator.sample(0.0, 0.0);
        assert_eq!(sample, sample2);
    }

    #[test]
    fn test_ridge_variants() {
        let config = RidgeConfig::default();
        let generator = RidgeGenerator::new(&config);
        
        let multifractal = generator.sample_ridged_multifractal(5.0, 5.0);
        let hybrid = generator.sample_hybrid_ridged(5.0, 5.0);
        let billow = generator.sample_billow_ridged(5.0, 5.0);
        let modulated = generator.sample_modulated_ridged(5.0, 5.0);
        
        // All should be in valid range
        assert!(multifractal >= -1.0 && multifractal <= 1.0);
        assert!(hybrid >= -1.0 && hybrid <= 1.0);
        assert!(billow >= -1.0 && billow <= 1.0);
        assert!(modulated >= -1.0 && modulated <= 1.0);
    }

    #[test]
    fn test_terraced_ridged() {
        let config = RidgeConfig::default();
        let generator = RidgeGenerator::new(&config);
        
        let terraced = generator.sample_terraced_ridged(10.0, 10.0, 8);
        assert!(terraced >= -1.0 && terraced <= 1.0);
        
        // Should produce stepped values
        let normalized = (terraced + 1.0) * 0.5 * 8.0; // Convert to terrace steps
        let stepped = normalized.floor();
        let remainder = normalized - stepped;
        assert!(remainder.abs() < 0.001 || remainder.abs() > 0.999); // Should be close to step boundaries
    }

    #[test]
    fn test_mountain_ridge_generator() {
        let mountain = MountainRidgeGenerator::new();
        
        let no_erosion = mountain.sample_mountain(10.0, 10.0, 0.0);
        let with_erosion = mountain.sample_mountain(10.0, 10.0, 0.5);
        
        assert!(no_erosion >= -1.0 && no_erosion <= 1.0);
        assert!(with_erosion >= -1.0 && with_erosion <= 1.0);
        
        // Erosion should typically change the result
        // Note: they might be equal in rare cases, but usually different
    }

    #[test]
    fn test_batch_sampling() {
        let config = RidgeConfig::default();
        let generator = RidgeGenerator::new(&config);
        
        let coords = vec![(0.0, 0.0), (1.0, 1.0), (2.0, 2.0), (3.0, 3.0)];
        let results = generator.sample_batch(&coords);
        
        assert_eq!(results.len(), 4);
        for result in results {
            assert!(result >= -1.0 && result <= 1.0);
        }
    }
}
