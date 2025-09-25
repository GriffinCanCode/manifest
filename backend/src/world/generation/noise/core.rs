//! Core noise generation implementations
//!
//! Deterministic noise generators using multiple libraries for
//! cross-platform reproducible results with performance optimization.

use super::types::*;
use noise::{NoiseFn, Simplex, Perlin, OpenSimplex};
use bracket_noise::prelude::*;
use ordered_float::OrderedFloat;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use cached::proc_macro::cached;
use rayon::prelude::*;
use std::sync::Arc;
use crate::core::scheduler::{Scheduler, TaskBatch, Stage, Resource, Access, SchedulerError};

/// High-performance Simplex noise generator
pub struct SimplexGenerator {
    config: SimplexConfig,
    generator: Arc<Simplex>,
    bracket_generator: FastNoise,
    seed: u64,
}

impl std::fmt::Debug for SimplexGenerator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SimplexGenerator")
            .field("config", &self.config)
            .field("generator", &"Arc<Simplex>")
            .field("seed", &self.seed)
            .finish()
    }
}

impl Clone for SimplexGenerator {
    fn clone(&self) -> Self {
        let mut bracket_generator = FastNoise::seeded(self.seed);
        bracket_generator.set_noise_type(bracket_noise::prelude::NoiseType::SimplexFractal);
        bracket_generator.set_frequency(*self.config.frequency as f32);
        
        Self {
            config: self.config.clone(),
            generator: self.generator.clone(),
            bracket_generator,
            seed: self.seed,
        }
    }
}

impl SimplexGenerator {
    /// Create new simplex generator with deterministic seed
    pub fn new(config: &SimplexConfig, seed: u64) -> Self {
        let generator = Arc::new(Simplex::new(seed as u32));
        let mut bracket_generator = FastNoise::seeded(seed);
        bracket_generator.set_noise_type(bracket_noise::prelude::NoiseType::SimplexFractal);
        bracket_generator.set_frequency(*config.frequency as f32);
        
        Self {
            config: config.clone(),
            generator,
            bracket_generator,
            seed,
        }
    }

    /// Sample simplex noise at coordinates
    pub fn sample(&self, x: f64, y: f64) -> f32 {
        self.sample_uncached(x, y)
    }

    /// Sample without caching for batch operations
    pub fn sample_uncached(&self, x: f64, y: f64) -> f32 {
        let scaled_x = x * *self.config.frequency;
        let scaled_y = y * *self.config.frequency;

        match self.config.quality {
            NoiseQuality::Low | NoiseQuality::Medium => {
                // Use standard noise library
                let mut value = 0.0;
                let mut amplitude = *self.config.amplitude;
                let mut frequency = 1.0;

                for _ in 0..self.config.octaves {
                    value += self.generator.get([scaled_x * frequency, scaled_y * frequency]) as f64 * amplitude;
                    frequency *= *self.config.lacunarity;
                    amplitude *= *self.config.persistence;
                }

                value as f32
            }
            NoiseQuality::High | NoiseQuality::Ultra => {
                // Use bracket-noise for high quality with optimizations
                self.bracket_generator.get_noise(scaled_x as f32, scaled_y as f32) * *self.config.amplitude as f32
            }
        }
    }

    /// Sample multiple points efficiently using basic parallel processing
    pub fn sample_batch(&self, points: &[(f64, f64)]) -> Vec<f32> {
        // Use parallel processing for better performance on large batches
        if points.len() > 100 {
            points.par_iter().map(|(x, y)| self.sample_uncached(*x, *y)).collect()
        } else {
            points.iter().map(|(x, y)| self.sample_uncached(*x, *y)).collect()
        }
    }

    /// Sample multiple points using the scheduler for coordinated task execution
    pub fn sample_batch_scheduled(
        &self, 
        points: &[(f64, f64)], 
        scheduler: &Scheduler
    ) -> Result<Vec<f32>, SchedulerError> {
        if points.is_empty() {
            return Ok(Vec::new());
        }

        // Create a task batch for noise generation
        let mut batch = TaskBatch::new(Stage::Update);
        let chunk_size = 64.max(points.len() / scheduler.active_count().max(1));
        
        let results = Arc::new(std::sync::Mutex::new(vec![0.0f32; points.len()]));
        let points_arc = Arc::new(points.to_vec());

        // Split work into chunks and create tasks
        for (chunk_idx, chunk) in points.chunks(chunk_size).enumerate() {
            let results_clone = Arc::clone(&results);
            let points_clone = Arc::clone(&points_arc);
            let generator_clone = (*self).clone(); // We'll need to implement Clone
            let start_idx = chunk_idx * chunk_size;
            let chunk_len = chunk.len();

            batch.add_task_with_resources(
                format!("simplex_noise_chunk_{}", chunk_idx),
                vec![Resource::write::<Vec<f32>>()],
                move || -> Result<(), SchedulerError> {
                    let mut local_results = Vec::with_capacity(chunk_len);
                    
                    for i in 0..chunk_len {
                        let (x, y) = points_clone[start_idx + i];
                        local_results.push(generator_clone.sample_uncached(x, y));
                    }

                    // Write results back to shared buffer
                    {
                        let mut results_guard = results_clone.lock().unwrap();
                        for (i, result) in local_results.into_iter().enumerate() {
                            results_guard[start_idx + i] = result;
                        }
                    }

                    Ok(())
                },
            );
        }

        // Execute the batch
        scheduler.add_batch(batch);
        scheduler.run_stage(Stage::Update).map_err(|errors| {
            errors.into_iter().next().unwrap_or_else(|| SchedulerError::TaskFailed("Unknown scheduler error".to_string()))
        })?;

        // Extract results
        let final_results = results.lock().unwrap().clone();
        Ok(final_results)
    }
}

/// High-performance Perlin noise generator
pub struct PerlinGenerator {
    config: PerlinConfig,
    generator: Arc<Perlin>,
    bracket_generator: FastNoise,
    seed: u64,
}

impl std::fmt::Debug for PerlinGenerator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PerlinGenerator")
            .field("config", &self.config)
            .field("generator", &"Arc<Perlin>")
            .field("seed", &self.seed)
            .finish()
    }
}

impl Clone for PerlinGenerator {
    fn clone(&self) -> Self {
        let mut bracket_generator = FastNoise::seeded(self.seed);
        bracket_generator.set_noise_type(bracket_noise::prelude::NoiseType::Perlin);
        bracket_generator.set_frequency(*self.config.frequency as f32);
        
        Self {
            config: self.config.clone(),
            generator: self.generator.clone(),
            bracket_generator,
            seed: self.seed,
        }
    }
}

impl PerlinGenerator {
    /// Create new Perlin generator
    pub fn new(config: &PerlinConfig, seed: u64) -> Self {
        let generator = Arc::new(Perlin::new(seed as u32));
        let mut bracket_generator = FastNoise::seeded(seed);
        bracket_generator.set_noise_type(bracket_noise::prelude::NoiseType::Perlin);
        bracket_generator.set_frequency(*config.frequency as f32);
        
        Self {
            config: config.clone(),
            generator,
            bracket_generator,
            seed,
        }
    }

    /// Sample Perlin noise at coordinates
    pub fn sample(&self, x: f64, y: f64) -> f32 {
        self.sample_uncached(x, y)
    }

    /// Sample without caching
    pub fn sample_uncached(&self, x: f64, y: f64) -> f32 {
        let scaled_x = x * *self.config.frequency;
        let scaled_y = y * *self.config.frequency;

        match self.config.quality {
            NoiseQuality::Low | NoiseQuality::Medium => {
                // Use standard noise library for better interpolation control
                let mut value = 0.0;
                let mut amplitude = *self.config.amplitude;
                let mut frequency = 1.0;

                for _ in 0..self.config.octaves {
                    value += self.generator.get([scaled_x * frequency, scaled_y * frequency]) as f64 * amplitude;
                    frequency *= *self.config.lacunarity;
                    amplitude *= *self.config.persistence;
                }

                value as f32
            }
            NoiseQuality::High | NoiseQuality::Ultra => {
                // Use bracket-noise for high quality Perlin
                self.bracket_generator.get_noise(scaled_x as f32, scaled_y as f32)
            }
        }
    }

    /// Generate fractal Perlin noise
    pub fn sample_fractal(&self, x: f64, y: f64, octaves: u32) -> f32 {
        let mut bracket_gen = FastNoise::seeded(self.seed);
        bracket_gen.set_noise_type(bracket_noise::prelude::NoiseType::SimplexFractal);
        bracket_gen.set_frequency(*self.config.frequency as f32);
        bracket_gen.set_fractal_octaves(octaves as i32);
        bracket_gen.set_fractal_lacunarity(*self.config.lacunarity as f32);
        bracket_gen.set_fractal_gain(*self.config.persistence as f32);
        
        bracket_gen.get_noise(x as f32, y as f32)
    }
}

/// Voronoi diagram generator using fast_poisson for point distribution
#[derive(Debug, Clone)]
pub struct VoronoiGenerator {
    config: VoronoiConfig,
    seed: u64,
    rng: ChaCha8Rng,
}

impl VoronoiGenerator {
    /// Create new Voronoi generator
    pub fn new(config: &VoronoiConfig, seed: u64) -> Self {
        let rng = ChaCha8Rng::seed_from_u64(config.point_seed);
        
        Self {
            config: config.clone(),
            seed,
            rng,
        }
    }

    /// Generate Voronoi points using Poisson disk sampling
    pub fn generate_points(&self, width: u32, height: u32) -> Vec<super::VoronoiPoint> {
        use fast_poisson::Poisson2D;
        
        let radius = (width as f64 * height as f64 / self.config.point_count as f64).sqrt() * 0.5;
        let poisson = Poisson2D::new()
            .with_dimensions([width as f64, height as f64], radius)
            .with_seed(self.config.point_seed);

        let points = poisson.iter().collect::<Vec<_>>();
        
        points.into_iter()
            .map(|p| super::VoronoiPoint {
                x: p[0],
                y: p[1],
                value: self.calculate_cell_value(p[0], p[1]),
            })
            .collect()
    }

    /// Calculate distance to nearest Voronoi point
    pub fn sample(&self, x: f64, y: f64) -> f32 {
        // This would typically use the generated points, simplified for now
        let hash = ((x * 73856093.0) + (y * 19349663.0)) as u32;
        let normalized = (hash % 1000) as f32 / 1000.0;
        normalized * 2.0 - 1.0
    }

    /// Calculate cell value based on distance function
    fn calculate_cell_value(&self, x: f64, y: f64) -> f32 {
        match self.config.distance_function {
            VoronoiDistance::Euclidean => {
                ((x * x + y * y).sqrt() % 1.0) as f32
            }
            VoronoiDistance::Manhattan => {
                ((x.abs() + y.abs()) % 1.0) as f32
            }
            VoronoiDistance::Chebyshev => {
                (x.abs().max(y.abs()) % 1.0) as f32
            }
            VoronoiDistance::Minkowski => {
                ((x.abs().powf(3.0) + y.abs().powf(3.0)).powf(1.0/3.0) % 1.0) as f32
            }
        }
    }
}

/// Worley (cellular) noise generator
pub struct WorleyGenerator {
    config: WorleyConfig,
    bracket_generator: FastNoise,
    seed: u64,
}

impl std::fmt::Debug for WorleyGenerator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorleyGenerator")
            .field("config", &self.config)
            .field("seed", &self.seed)
            .finish()
    }
}

impl Clone for WorleyGenerator {
    fn clone(&self) -> Self {
        let mut bracket_generator = FastNoise::seeded(self.seed);
        bracket_generator.set_noise_type(bracket_noise::prelude::NoiseType::Cellular);
        bracket_generator.set_frequency(*self.config.density as f32);
        
        Self {
            config: self.config.clone(),
            bracket_generator,
            seed: self.seed,
        }
    }
}

impl WorleyGenerator {
    /// Create new Worley generator
    pub fn new(config: &WorleyConfig, seed: u64) -> Self {
        let mut bracket_generator = FastNoise::seeded(seed);
        bracket_generator.set_noise_type(bracket_noise::prelude::NoiseType::Cellular);
        bracket_generator.set_frequency(*config.density as f32);
        
        Self {
            config: config.clone(),
            bracket_generator,
            seed,
        }
    }

    /// Sample Worley noise
    pub fn sample(&self, x: f64, y: f64) -> f32 {
        self.sample_uncached(x, y)
    }

    /// Sample without caching
    pub fn sample_uncached(&self, x: f64, y: f64) -> f32 {
        if self.config.fractal {
            let mut generator = FastNoise::seeded(self.seed);
            generator.set_noise_type(bracket_noise::prelude::NoiseType::SimplexFractal);
            generator.set_fractal_octaves(self.config.fractal_octaves as i32);
            generator.set_fractal_frequency(*self.config.fractal_frequency as f32);
            generator.get_noise(x as f32, y as f32)
        } else {
            self.bracket_generator.get_noise(x as f32, y as f32)
        }
    }

    /// Sample with specific distance order (1st, 2nd, 3rd closest point)
    pub fn sample_distance_order(&self, x: f64, y: f64, order: u32) -> f32 {
        // Simplified implementation - would need more sophisticated cellular calculation
        let base = self.sample_uncached(x, y);
        let modifier = (order as f32 - 1.0) * 0.1;
        (base + modifier).clamp(-1.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simplex_deterministic() {
        let config = SimplexConfig::default();
        let gen1 = SimplexGenerator::new(&config, 12345);
        let gen2 = SimplexGenerator::new(&config, 12345);
        
        assert_eq!(gen1.sample(0.0, 0.0), gen2.sample(0.0, 0.0));
        assert_eq!(gen1.sample(1.0, 1.0), gen2.sample(1.0, 1.0));
    }

    #[test]
    fn test_perlin_octaves() {
        let config = PerlinConfig {
            octaves: 4,
            ..Default::default()
        };
        let gen = PerlinGenerator::new(&config, 12345);
        
        let sample = gen.sample(0.0, 0.0);
        assert!(sample >= -1.0 && sample <= 1.0);
    }

    #[test]
    fn test_voronoi_points() {
        let config = VoronoiConfig {
            point_count: 10,
            ..Default::default()
        };
        let gen = VoronoiGenerator::new(&config, 12345);
        
        let points = gen.generate_points(100, 100);
        assert!(!points.is_empty());
        assert!(points.len() <= config.point_count as usize + 5); // Allow some variance
    }

    #[test]
    fn test_batch_sampling() {
        let config = SimplexConfig::default();
        let gen = SimplexGenerator::new(&config, 12345);
        
        let points = vec![(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)];
        let results = gen.sample_batch(&points);
        
        assert_eq!(results.len(), 3);
        for result in results {
            assert!(result >= -1.0 && result <= 1.0);
        }
    }
}
