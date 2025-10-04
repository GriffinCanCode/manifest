//! Voronoi diagram generation for organic patterns
//!
//! High-performance Voronoi noise using optimized algorithms and
//! deterministic point generation with configurable distance metrics.

use super::types::*;
use super::VoronoiPoint;
use crate::core::hashing::CoordinateHasher;
use fast_poisson::Poisson2D;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;
use std::collections::HashMap;

/// High-performance Voronoi diagram generator
#[derive(Debug)]
pub struct VoronoiGenerator {
    config: VoronoiConfig,
    points: Vec<VoronoiPoint>,
    point_grid: HashMap<(i32, i32), Vec<VoronoiPoint>>,
    grid_size: f64,
    bounds: (f64, f64, f64, f64), // min_x, min_y, max_x, max_y
}

impl VoronoiGenerator {
    /// Create new Voronoi generator with optimal point distribution
    pub fn new(config: &VoronoiConfig, seed: u64) -> Self {
        Self {
            config: config.clone(),
            points: Vec::new(),
            point_grid: HashMap::new(),
            grid_size: 100.0, // Default grid cell size
            bounds: (0.0, 0.0, 1000.0, 1000.0),
        }
    }

    /// Generate Voronoi points using Poisson disk sampling for natural distribution
    pub fn generate_points(&mut self, width: u32, height: u32) -> Vec<VoronoiPoint> {
        self.bounds = (0.0, 0.0, width as f64, height as f64);
        
        // Calculate optimal radius for desired point count
        let area = width as f64 * height as f64;
        let target_density = self.config.point_count as f64 / area;
        let radius = (1.0 / (target_density * std::f64::consts::PI)).sqrt() * 0.8;

        // Generate points using Poisson disk sampling
        let mut poisson_builder = Poisson2D::new();
        let poisson = poisson_builder
            .with_dimensions([width as f64, height as f64], radius)
            .with_seed(self.config.point_seed);

        let mut rng = ChaCha8Rng::seed_from_u64(self.config.point_seed);

        let points_iter: Vec<_> = poisson.iter().collect();
        self.points = points_iter.into_iter()
            .map(|p| {
                let jittered_x = if *self.config.jitter > 0.0 {
                    p[0] + (rng.gen::<f64>() - 0.5) * *self.config.jitter * radius
                } else {
                    p[0]
                };
                let jittered_y = if *self.config.jitter > 0.0 {
                    p[1] + (rng.gen::<f64>() - 0.5) * *self.config.jitter * radius
                } else {
                    p[1]
                };

                // Generate deterministic value based on position
                let position_hash = CoordinateHasher::hash_hex(
                    (jittered_x * 1000.0) as i32,
                    (jittered_y * 1000.0) as i32,
                );
                let value = (position_hash % 1000) as f32 / 1000.0;

                VoronoiPoint {
                    x: jittered_x.clamp(0.0, width as f64),
                    y: jittered_y.clamp(0.0, height as f64),
                    value,
                }
            })
            .collect();

        // Build spatial grid for fast nearest neighbor queries
        self.build_spatial_grid();

        self.points.clone()
    }

    /// Build spatial grid for O(1) nearest neighbor queries
    fn build_spatial_grid(&mut self) {
        self.point_grid.clear();
        
        for point in &self.points {
            let grid_x = (point.x / self.grid_size) as i32;
            let grid_y = (point.y / self.grid_size) as i32;
            
            self.point_grid
                .entry((grid_x, grid_y))
                .or_insert_with(Vec::new)
                .push(*point);
        }
    }

    /// Sample Voronoi value at coordinates using optimized spatial queries
    pub fn sample(&self, x: f64, y: f64) -> f32 {
        if self.points.is_empty() {
            return 0.0;
        }

        let grid_x = (x / self.grid_size) as i32;
        let grid_y = (y / self.grid_size) as i32;

        let mut min_distance = f64::INFINITY;
        let mut closest_value = 0.0;

        // Check surrounding grid cells
        for dx in -1..=1 {
            for dy in -1..=1 {
                let check_x = grid_x + dx;
                let check_y = grid_y + dy;

                if let Some(cell_points) = self.point_grid.get(&(check_x, check_y)) {
                    for point in cell_points {
                        let distance = self.calculate_distance(x, y, point.x, point.y);
                        if distance < min_distance {
                            min_distance = distance;
                            closest_value = point.value;
                        }
                    }
                }
            }
        }

        // If no points found in grid, fallback to brute force
        if min_distance == f64::INFINITY {
            for point in &self.points {
                let distance = self.calculate_distance(x, y, point.x, point.y);
                if distance < min_distance {
                    min_distance = distance;
                    closest_value = point.value;
                }
            }
        }

        // Normalize distance to [0, 1] range
        let normalized_distance = (min_distance / (self.grid_size * 0.5)).clamp(0.0, 1.0);
        
        if self.config.cellular {
            // Cellular noise returns distance instead of value
            normalized_distance as f32
        } else {
            closest_value
        }
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

    /// Sample with multiple distance orders (1st, 2nd, 3rd closest points)
    pub fn sample_distance_order(&self, x: f64, y: f64, order: usize) -> f32 {
        if self.points.is_empty() || order == 0 {
            return 0.0;
        }

        let mut distances: Vec<f64> = self.points
            .iter()
            .map(|point| self.calculate_distance(x, y, point.x, point.y))
            .collect();

        distances.sort_by(|a, b| a.partial_cmp(b).unwrap());

        if order <= distances.len() {
            let distance = distances[order - 1];
            (distance / (self.grid_size * 0.5)).clamp(0.0, 1.0) as f32
        } else {
            1.0
        }
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

    /// Generate cellular automata pattern
    pub fn sample_cellular(&self, x: f64, y: f64, threshold: f32) -> f32 {
        let base_value = self.sample(x, y);
        if base_value > threshold { 1.0 } else { 0.0 }
    }

    /// Generate F1-F2 pattern (difference between 1st and 2nd closest)
    pub fn sample_f1_f2(&self, x: f64, y: f64) -> f32 {
        let f1 = self.sample_distance_order(x, y, 1);
        let f2 = self.sample_distance_order(x, y, 2);
        (f2 - f1).clamp(0.0, 1.0)
    }

    /// Generate crackle pattern using distance ratios
    pub fn sample_crackle(&self, x: f64, y: f64) -> f32 {
        let f1 = self.sample_distance_order(x, y, 1);
        let f2 = self.sample_distance_order(x, y, 2);
        
        if f2 > f32::EPSILON {
            (f1 / f2).clamp(0.0, 1.0)
        } else {
            f1
        }
    }

    /// Update configuration and regenerate if needed
    pub fn update_config(&mut self, config: VoronoiConfig, width: u32, height: u32) {
        let needs_regeneration = config.point_count != self.config.point_count ||
                               config.point_seed != self.config.point_seed ||
                               config.jitter != self.config.jitter;

        self.config = config;

        if needs_regeneration {
            self.generate_points(width, height);
        }
    }

    /// Get point count
    pub fn point_count(&self) -> usize {
        self.points.len()
    }

    /// Get bounds
    pub fn bounds(&self) -> (f64, f64, f64, f64) {
        self.bounds
    }

    /// Clear points and grid
    pub fn clear(&mut self) {
        self.points.clear();
        self.point_grid.clear();
    }
}

/// Specialized Voronoi implementation for procedural textures
#[derive(Debug)]
pub struct VoronoiTexture {
    generator: VoronoiGenerator,
    scale: f64,
    octaves: u32,
    lacunarity: f64,
    persistence: f64,
}

impl VoronoiTexture {
    /// Create texture generator with fractal properties
    pub fn new(config: &VoronoiConfig, seed: u64, scale: f64) -> Self {
        Self {
            generator: VoronoiGenerator::new(config, seed),
            scale,
            octaves: 4,
            lacunarity: 2.0,
            persistence: 0.5,
        }
    }

    /// Generate fractal Voronoi texture
    pub fn sample_fractal(&self, x: f64, y: f64) -> f32 {
        let mut value = 0.0;
        let mut amplitude = 1.0;
        let mut frequency = self.scale;

        for _ in 0..self.octaves {
            value += self.generator.sample(x * frequency, y * frequency) * amplitude;
            frequency *= self.lacunarity;
            amplitude *= self.persistence as f32;
        }

        value.clamp(0.0, 1.0)
    }

    /// Set fractal parameters
    pub fn set_fractal_params(&mut self, octaves: u32, lacunarity: f64, persistence: f64) {
        self.octaves = octaves;
        self.lacunarity = lacunarity;
        self.persistence = persistence;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voronoi_generator_creation() {
        let config = VoronoiConfig::default();
        let generator = VoronoiGenerator::new(&config, 12345);
        
        assert_eq!(generator.point_count(), 0);
    }

    #[test]
    fn test_point_generation() {
        let config = VoronoiConfig {
            point_count: 50,
            ..Default::default()
        };
        let mut generator = VoronoiGenerator::new(&config, 12345);
        
        let points = generator.generate_points(100, 100);
        assert!(!points.is_empty());
        assert!(points.len() <= config.point_count as usize + 10); // Allow some variance
        
        // Verify points are within bounds
        for point in &points {
            assert!(point.x >= 0.0 && point.x <= 100.0);
            assert!(point.y >= 0.0 && point.y <= 100.0);
            assert!(point.value >= 0.0 && point.value <= 1.0);
        }
    }

    #[test]
    fn test_distance_functions() {
        let config = VoronoiConfig::default();
        let generator = VoronoiGenerator::new(&config, 12345);
        
        let euclidean = generator.calculate_distance(0.0, 0.0, 3.0, 4.0);
        assert!((euclidean - 5.0).abs() < 1e-10);
        
        // Test other distance functions by changing config
        let mut manhattan_config = config.clone();
        manhattan_config.distance_function = VoronoiDistance::Manhattan;
        let manhattan_gen = VoronoiGenerator::new(&manhattan_config, 12345);
        let manhattan = manhattan_gen.calculate_distance(0.0, 0.0, 3.0, 4.0);
        assert_eq!(manhattan, 7.0);
    }

    #[test]
    fn test_sampling() {
        let config = VoronoiConfig {
            point_count: 10,
            ..Default::default()
        };
        let mut generator = VoronoiGenerator::new(&config, 12345);
        generator.generate_points(100, 100);
        
        let sample = generator.sample(50.0, 50.0);
        assert!(sample >= 0.0 && sample <= 1.0);
        
        // Test determinism
        let sample2 = generator.sample(50.0, 50.0);
        assert_eq!(sample, sample2);
    }

    #[test]
    fn test_batch_sampling() {
        let config = VoronoiConfig {
            point_count: 20,
            ..Default::default()
        };
        let mut generator = VoronoiGenerator::new(&config, 12345);
        generator.generate_points(100, 100);
        
        let coords = vec![(10.0, 10.0), (50.0, 50.0), (90.0, 90.0)];
        let results = generator.sample_batch(&coords);
        
        assert_eq!(results.len(), 3);
        for result in results {
            assert!(result >= 0.0 && result <= 1.0);
        }
    }

    #[test]
    fn test_distance_orders() {
        let config = VoronoiConfig {
            point_count: 25,
            ..Default::default()
        };
        let mut generator = VoronoiGenerator::new(&config, 12345);
        generator.generate_points(100, 100);
        
        let f1 = generator.sample_distance_order(50.0, 50.0, 1);
        let f2 = generator.sample_distance_order(50.0, 50.0, 2);
        
        // F2 should be greater than or equal to F1
        assert!(f2 >= f1);
        
        // Test F1-F2 pattern
        let f1_f2 = generator.sample_f1_f2(50.0, 50.0);
        assert!(f1_f2 >= 0.0 && f1_f2 <= 1.0);
    }

    #[test]
    fn test_voronoi_texture() {
        let config = VoronoiConfig::default();
        let mut texture = VoronoiTexture::new(&config, 12345, 0.1);
        texture.generator.generate_points(100, 100);
        
        let fractal_sample = texture.sample_fractal(50.0, 50.0);
        assert!(fractal_sample >= 0.0 && fractal_sample <= 1.0);
    }
}
