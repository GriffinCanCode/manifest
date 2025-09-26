//! High-Performance Noise Generation System
//!
//! Provides deterministic, SIMD-optimized noise generation for procedural
//! world creation with comprehensive caching and extensible noise types.

pub mod types;
pub mod core;
pub mod cache;
pub mod simd;
pub mod voronoi;
pub mod worley;
pub mod fbm;
pub mod domain;
pub mod ridge;
pub mod mixers;
pub mod scheduler;

// Re-export public API
pub use types::*;
pub use core::*;
pub use cache::*;
pub use fbm::*;
pub use voronoi::VoronoiGenerator as VoronoiGen;
pub use worley::WorleyGenerator as WorleyGen;
pub use domain::*;
pub use ridge::*;
pub use mixers::*;
pub use scheduler::*;

use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString};
use ordered_float::OrderedFloat;
use rand_chacha::ChaCha8Rng;
use bevy_ecs::system::Resource as BevyResource;
use cached::proc_macro::cached;
use std::sync::Arc;

/// Comprehensive noise configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoiseConfig {
    /// Base noise types configuration
    pub simplex: SimplexConfig,
    pub perlin: PerlinConfig,
    pub voronoi: VoronoiConfig,
    pub worley: WorleyConfig,
    
    /// Advanced noise configuration
    pub fbm: FbmConfig,
    pub domain_warp: DomainWarpConfig,
    pub ridge: RidgeConfig,
    
    /// Performance settings
    pub use_simd: bool,
    pub cache_size: usize,
    pub batch_size: usize,
    
    /// Deterministic settings
    pub seed: u64,
}

impl Default for NoiseConfig {
    fn default() -> Self {
        Self {
            simplex: SimplexConfig::default(),
            perlin: PerlinConfig::default(),
            voronoi: VoronoiConfig::default(),
            worley: WorleyConfig::default(),
            fbm: FbmConfig::default(),
            domain_warp: DomainWarpConfig::default(),
            ridge: RidgeConfig::default(),
            use_simd: true,
            cache_size: 1000,
            batch_size: 64,
            seed: 12345,
        }
    }
}

/// Main noise generator with comprehensive caching
#[derive(Debug, BevyResource)]
pub struct NoiseGenerator {
    config: NoiseConfig,
    rng: ChaCha8Rng,
    cache: Arc<NoiseCache>,
    
    // Core generators
    simplex: SimplexGenerator,
    perlin: PerlinGenerator,
    voronoi: VoronoiGen,
    worley: WorleyGen,
    
    // Advanced generators
    fbm: FbmGenerator,
    domain_warp: DomainWarpGenerator,
    ridge: RidgeGenerator,
    mixer: NoiseMixer,
}

impl NoiseGenerator {
    /// Create new noise generator with configuration
    pub fn new(config: &NoiseConfig) -> Self {
        use rand::SeedableRng;
        
        let rng = ChaCha8Rng::seed_from_u64(config.seed);
        let cache = Arc::new(NoiseCache::new(config.cache_size));
        
        Self {
            config: config.clone(),
            rng,
            cache: cache.clone(),
            simplex: SimplexGenerator::new(&config.simplex, config.seed),
            perlin: PerlinGenerator::new(&config.perlin, config.seed),
            voronoi: VoronoiGen::new(&config.voronoi, config.seed),
            worley: WorleyGen::new(&config.worley, config.seed),
            fbm: FbmGenerator::new(&config.fbm),
            domain_warp: DomainWarpGenerator::new(&config.domain_warp),
            ridge: RidgeGenerator::new(&config.ridge),
            mixer: NoiseMixer::new(),
        }
    }

    /// Sample height using composite noise
    pub fn sample_height(&self, x: f64, y: f64) -> f32 {
        let base = self.simplex.sample(x, y);
        let detail = self.perlin.sample(x * 4.0, y * 4.0) * 0.5;
        let ridge = self.ridge.sample(x * 2.0, y * 2.0) * 0.3;
        
        (base + detail + ridge).clamp(-1.0, 1.0)
    }

    /// Sample temperature using FBM
    pub fn sample_temperature(&self, x: f64, y: f64) -> f32 {
        self.fbm.sample_temperature(x, y)
    }

    /// Get the noise configuration
    pub fn config(&self) -> &NoiseConfig {
        &self.config
    }

    /// Sample moisture using domain warping
    pub fn sample_moisture(&self, x: f64, y: f64) -> f32 {
        let warped = self.domain_warp.warp(x, y);
        self.simplex.sample(warped.0 as f64, warped.1 as f64)
    }

    /// Sample 2D noise - compatibility method
    pub fn sample_2d(&self, x: f64, y: f64) -> f32 {
        self.sample_height(x, y)
    }

    /// Sample multiple noise values in batch for SIMD optimization
    pub fn sample_batch(&self, coords: &[(f64, f64)]) -> Vec<NoiseResult> {
        if self.config.use_simd && coords.len() >= self.config.batch_size {
            self.sample_batch_simd(coords)
        } else {
            coords.iter()
                .map(|(x, y)| NoiseResult {
                    height: self.sample_height(*x, *y),
                    temperature: self.sample_temperature(*x, *y), 
                    moisture: self.sample_moisture(*x, *y),
                })
                .collect()
        }
    }

    /// Sample multiple noise values using the scheduler for coordinated execution
    pub fn sample_batch_scheduled(
        &self, 
        coords: &[(f64, f64)], 
        scheduler: &std::sync::Arc<crate::core::scheduler::Scheduler>
    ) -> Result<Vec<NoiseResult>, crate::core::scheduler::SchedulerError> {
        // Create scheduled noise generator for coordinated execution
        let scheduled_generator = ScheduledNoiseGenerator::new(self.config.clone(), scheduler.clone());
        scheduled_generator.generate_terrain_data_scheduled(coords)
    }

    /// SIMD-optimized batch sampling
    fn sample_batch_simd(&self, coords: &[(f64, f64)]) -> Vec<NoiseResult> {
        // Use Zig SIMD functions for batch processing
        simd::batch_noise_sample(coords, &self.config)
    }

    /// Generate Voronoi diagram points
    pub fn generate_voronoi(&mut self, width: u32, height: u32) -> Vec<VoronoiPoint> {
        self.voronoi.generate_points(width, height)
    }

    /// Generate Worley noise pattern
    pub fn generate_worley(&self, x: f64, y: f64) -> f32 {
        self.worley.sample(x, y)
    }
}

/// Noise sampling result
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NoiseResult {
    pub height: f32,
    pub temperature: f32,
    pub moisture: f32,
}

/// Voronoi point for diagram generation
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VoronoiPoint {
    pub x: f64,
    pub y: f64,
    pub value: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noise_generator_creation() {
        let config = NoiseConfig::default();
        let generator = NoiseGenerator::new(&config);
        
        // Test deterministic output
        let sample1 = generator.sample_height(0.0, 0.0);
        let sample2 = generator.sample_height(0.0, 0.0);
        assert_eq!(sample1, sample2);
    }

    #[test]
    fn test_batch_sampling() {
        let config = NoiseConfig::default();
        let generator = NoiseGenerator::new(&config);
        
        let coords = vec![(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)];
        let results = generator.sample_batch(&coords);
        
        assert_eq!(results.len(), 3);
        assert!(results[0].height >= -1.0 && results[0].height <= 1.0);
    }
}
