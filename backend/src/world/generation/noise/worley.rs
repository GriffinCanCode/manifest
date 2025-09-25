//! Worley (cellular) noise implementation
//!
//! High-performance Worley noise using optimized cellular patterns and
//! configurable distance functions for organic terrain generation.

use super::types::*;
use crate::core::hashing::{CoordinateHasher, HashStrategies, FastHasher};
use ordered_float::OrderedFloat;
use rayon::prelude::*;
use std::hash::Hash;

/// High-performance Worley (cellular) noise generator
#[derive(Debug)]
pub struct WorleyGenerator {
    config: WorleyConfig,
    point_cache: std::collections::HashMap<(i32, i32), Vec<(f64, f64)>>,
}

impl WorleyGenerator {
    /// Create new Worley generator with configuration
    pub fn new(config: &WorleyConfig, seed: u64) -> Self {
        Self {
            config: config.clone(),
            point_cache: std::collections::HashMap::new(),
        }
    }

    /// Sample Worley noise using cellular pattern generation
    pub fn sample(&self, x: f64, y: f64) -> f32 {
        let cell_size = 1.0 / *self.config.density;
        
        // Find the containing cell
        let cell_x = (x / cell_size).floor() as i32;
        let cell_y = (y / cell_size).floor() as i32;

        let mut distances = Vec::new();

        // Check surrounding cells (3x3 grid)
        for dx in -1..=1 {
            for dy in -1..=1 {
                let check_cell_x = cell_x + dx;
                let check_cell_y = cell_y + dy;
                
                let cell_points = self.generate_cell_points(check_cell_x, check_cell_y, cell_size);
                
                for (px, py) in cell_points {
                    let distance = self.calculate_distance(x, y, px, py);
                    distances.push(distance);
                }
            }
        }

        // Sort distances to get the Nth closest
        distances.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let distance_index = (self.config.distance_order as usize).saturating_sub(1);
        if distance_index < distances.len() {
            let raw_distance = distances[distance_index];
            
            // Normalize to [0, 1] range and convert to [-1, 1]
            let normalized = (raw_distance / (cell_size * 2.0)).clamp(0.0, 1.0);
            (normalized * 2.0 - 1.0) as f32
        } else {
            1.0
        }
    }

    /// Generate deterministic points within a cell using hash-based distribution
    fn generate_cell_points(&self, cell_x: i32, cell_y: i32, cell_size: f64) -> Vec<(f64, f64)> {
        // Use cell coordinates as seed for deterministic point generation
        let cell_hash = CoordinateHasher::hash_hex(cell_x, cell_y);
        
        // Generate 1-4 points per cell based on density
        let base_density = (*self.config.density * cell_size * cell_size).max(1.0);
        let point_count = self.hash_to_point_count(cell_hash, base_density as u32);
        
        let mut points = Vec::new();
        
        for i in 0..point_count {
            // Generate hash for this point
            let point_seed = HashStrategies::combine_hashes(&[cell_hash, i as u64]);
            
            // Extract two independent random values from hash
            let rand1 = ((point_seed & 0xFFFFFFFF) as f64) / (0xFFFFFFFFu64 as f64);
            let rand2 = ((point_seed >> 32) as f64) / (0xFFFFFFFFu64 as f64);
            
            // Position within cell
            let local_x = rand1;
            let local_y = rand2;
            
            // Convert to world coordinates
            let world_x = (cell_x as f64 + local_x) * cell_size;
            let world_y = (cell_y as f64 + local_y) * cell_size;
            
            points.push((world_x, world_y));
        }
        
        points
    }

    /// Convert hash to point count with Poisson-like distribution
    fn hash_to_point_count(&self, hash: u64, max_points: u32) -> u32 {
        // Use hash to generate Poisson-like distribution
        let normalized = (hash % 1000) as f64 / 1000.0;
        let lambda = max_points as f64;
        
        // Simple approximation of Poisson distribution
        let k = (-lambda * (1.0 - normalized).ln()).max(1.0).min(max_points as f64);
        k as u32
    }

    /// Calculate distance using configured distance function
    fn calculate_distance(&self, x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
        let dx = x1 - x2;
        let dy = y1 - y2;

        match self.config.distance_function {
            VoronoiDistance::Euclidean => (dx * dx + dy * dy).sqrt(),
            VoronoiDistance::Manhattan => dx.abs() + dy.abs(),
            VoronoiDistance::Chebyshev => dx.abs().max(dy.abs()),
            VoronoiDistance::Minkowski => {
                // Using Minkowski distance with p=3
                (dx.abs().powf(3.0) + dy.abs().powf(3.0)).powf(1.0/3.0)
            }
        }
    }

    /// Sample fractal Worley noise by combining multiple octaves
    pub fn sample_fractal(&self, x: f64, y: f64) -> f32 {
        if !self.config.fractal {
            return self.sample(x, y);
        }

        let mut value = 0.0;
        let mut amplitude = 1.0;
        let mut frequency = *self.config.fractal_frequency;
        let mut max_amplitude = 0.0;

        for _ in 0..self.config.fractal_octaves {
            let sample = self.sample(x * frequency, y * frequency) as f64;
            value += sample * amplitude;
            max_amplitude += amplitude;

            amplitude *= 0.5; // Reduce amplitude each octave
            frequency *= 2.0; // Increase frequency each octave
        }

        // Normalize
        if max_amplitude > 0.0 {
            (value / max_amplitude) as f32
        } else {
            0.0
        }
    }

    /// Sample with specific distance order (1st, 2nd, 3rd closest point)
    pub fn sample_distance_order(&self, x: f64, y: f64, order: u32) -> f32 {
        let original_order = self.config.distance_order;
        
        // Temporarily modify config for this sample
        let mut temp_config = self.config.clone();
        temp_config.distance_order = order;
        
        let temp_generator = Self {
            config: temp_config,
            point_cache: std::collections::HashMap::new(),
        };
        
        temp_generator.sample(x, y)
    }

    /// Batch sampling with parallel processing
    pub fn sample_batch(&self, coordinates: &[(f64, f64)]) -> Vec<f32> {
        if coordinates.len() > 50 {
            coordinates.par_iter()
                .map(|(x, y)| self.sample(*x, *y))
                .collect()
        } else {
            coordinates.iter()
                .map(|(x, y)| self.sample(*x, *y))
                .collect()
        }
    }

    /// Get configuration
    pub fn config(&self) -> &WorleyConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worley_generator_creation() {
        let config = WorleyConfig::default();
        let generator = WorleyGenerator::new(&config, 12345);
        
        assert_eq!(generator.config.distance_order, 1);
        assert_eq!(generator.point_cache.len(), 0);
    }

    #[test]
    fn test_worley_sampling() {
        let config = WorleyConfig::default();
        let generator = WorleyGenerator::new(&config, 12345);
        
        let sample = generator.sample(0.0, 0.0);
        assert!(sample >= -1.0 && sample <= 1.0);
        
        // Test determinism
        let sample2 = generator.sample(0.0, 0.0);
        assert_eq!(sample, sample2);
    }

    #[test]
    fn test_distance_functions() {
        let mut config = WorleyConfig::default();
        
        // Test Euclidean distance
        config.distance_function = VoronoiDistance::Euclidean;
        let euclidean_gen = WorleyGenerator::new(&config, 12345);
        let euclidean_dist = euclidean_gen.calculate_distance(0.0, 0.0, 3.0, 4.0);
        assert!((euclidean_dist - 5.0).abs() < 1e-10);
        
        // Test Manhattan distance
        config.distance_function = VoronoiDistance::Manhattan;
        let manhattan_gen = WorleyGenerator::new(&config, 12345);
        let manhattan_dist = manhattan_gen.calculate_distance(0.0, 0.0, 3.0, 4.0);
        assert_eq!(manhattan_dist, 7.0);
    }
}