//! Scheduled noise generation using the core task scheduler
//!
//! Provides coordinated, high-performance noise generation with
//! resource management, task scheduling, and performance monitoring.

use super::types::*;
use super::core::*;
use super::{NoiseResult, NoiseConfig};
use crate::core::scheduler::{Scheduler, TaskBatch, Stage, Resource, SchedulerError};
use crate::core::hashing::HashStrategies;
use ordered_float::OrderedFloat;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

/// Resource types for noise generation scheduling
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NoiseResource {
    SimplexGenerator,
    PerlinGenerator,
    VoronoiGenerator,
    WorleyGenerator,
    HeightMap,
    TemperatureMap,
    MoistureMap,
    TerrainCache,
}

/// Scheduled noise generation coordinator
#[derive(Debug)]
pub struct ScheduledNoiseGenerator {
    scheduler: Arc<Scheduler>,
    simplex: SimplexGenerator,
    perlin: PerlinGenerator,
    voronoi: VoronoiGenerator,
    worley: WorleyGenerator,
    config: NoiseConfig,
}

impl ScheduledNoiseGenerator {
    /// Create new scheduled noise generator
    pub fn new(config: NoiseConfig, scheduler: Arc<Scheduler>) -> Self {
        Self {
            scheduler,
            simplex: SimplexGenerator::new(&config.simplex, config.seed),
            perlin: PerlinGenerator::new(&config.perlin, config.seed.wrapping_add(1001)),
            voronoi: VoronoiGenerator::new(&config.voronoi, config.seed.wrapping_add(2002)),
            worley: WorleyGenerator::new(&config.worley, config.seed.wrapping_add(3003)),
            config,
        }
    }

    /// Generate terrain heightmap using coordinated scheduling
    pub fn generate_heightmap_scheduled(
        &self,
        width: u32,
        height: u32,
        chunk_size: u32,
    ) -> Result<Vec<Vec<f32>>, SchedulerError> {
        let total_points = (width * height) as usize;
        let coordinates = self.generate_coordinate_grid(width, height);
        
        // Prepare result storage
        let heightmap = Arc::new(Mutex::new(vec![vec![0.0f32; width as usize]; height as usize]));
        
        // Create task batch for heightmap generation
        let mut batch = TaskBatch::new(Stage::Update);
        
        // Process terrain in chunks for better cache locality
        let chunks_x = (width + chunk_size - 1) / chunk_size;
        let chunks_y = (height + chunk_size - 1) / chunk_size;
        
        for chunk_y in 0..chunks_y {
            for chunk_x in 0..chunks_x {
                let heightmap_clone = Arc::clone(&heightmap);
                let simplex_clone = self.simplex.clone();
                let perlin_clone = self.perlin.clone();
                
                // Calculate chunk bounds
                let x_start = chunk_x * chunk_size;
                let y_start = chunk_y * chunk_size;
                let x_end = (width).min(x_start + chunk_size);
                let y_end = (height).min(y_start + chunk_size);
                
                batch.add_task_with_resources(
                    format!("heightmap_chunk_{}_{}", chunk_x, chunk_y),
                    vec![Resource::write::<NoiseResource>()], 
                    move || -> Result<(), SchedulerError> {
                        // Generate heightmap for this chunk
                        for y in y_start..y_end {
                            for x in x_start..x_end {
                                let world_x = x as f64;
                                let world_y = y as f64;
                                
                                // Combine multiple noise sources for realistic terrain
                                let base_height = simplex_clone.sample_uncached(world_x * 0.01, world_y * 0.01);
                                let detail = perlin_clone.sample_uncached(world_x * 0.05, world_y * 0.05) * 0.3;
                                let fine_detail = simplex_clone.sample_uncached(world_x * 0.2, world_y * 0.2) * 0.1;
                                
                                let final_height = base_height + detail + fine_detail;
                                
                                // Write to heightmap
                                let mut map = heightmap_clone.lock().unwrap();
                                map[y as usize][x as usize] = final_height;
                            }
                        }
                        Ok(())
                    },
                );
            }
        }
        
        // Execute heightmap generation
        self.scheduler.add_batch(batch);
        self.scheduler.run_stage(Stage::Update).map_err(|errors| {
            errors.into_iter().next().unwrap_or_else(|| SchedulerError::TaskFailed("Unknown scheduler error".to_string()))
        })?;
        
        // Extract final heightmap
        let final_heightmap = heightmap.lock().unwrap().clone();
        Ok(final_heightmap)
    }

    /// Generate complete terrain data (height, temperature, moisture) with scheduling
    pub fn generate_terrain_data_scheduled(
        &self,
        coordinates: &[(f64, f64)],
    ) -> Result<Vec<NoiseResult>, SchedulerError> {
        if coordinates.is_empty() {
            return Ok(Vec::new());
        }

        let results = Arc::new(Mutex::new(vec![NoiseResult { height: 0.0, temperature: 0.0, moisture: 0.0 }; coordinates.len()]));
        let coords_arc = Arc::new(coordinates.to_vec());
        
        // Create coordinated task batch for all noise types
        let mut batch = TaskBatch::new(Stage::Update);
        let chunk_size = 256.min(coordinates.len().max(64));
        
        // Generate height data
        let height_results = Arc::clone(&results);
        let height_coords = Arc::clone(&coords_arc);
        let simplex_height = self.simplex.clone();
        
        for (chunk_idx, chunk) in coordinates.chunks(chunk_size).enumerate() {
            let results_clone = Arc::clone(&height_results);
            let coords_clone = Arc::clone(&height_coords);
            let generator_clone = simplex_height.clone();
            let start_idx = chunk_idx * chunk_size;
            let chunk_len = chunk.len();

            batch.add_task_with_resources(
                format!("terrain_height_chunk_{}", chunk_idx),
                vec![Resource::read::<NoiseResource>()],
                move || -> Result<(), SchedulerError> {
                    let mut results_guard = results_clone.lock().unwrap();
                    
                    for i in 0..chunk_len {
                        let (x, y) = coords_clone[start_idx + i];
                        let base = generator_clone.sample_uncached(x * 0.01, y * 0.01);
                        let detail = generator_clone.sample_uncached(x * 0.05, y * 0.05) * 0.4;
                        results_guard[start_idx + i].height = base + detail;
                    }
                    
                    Ok(())
                },
            );
        }

        // Generate temperature data
        let temp_results = Arc::clone(&results);
        let temp_coords = Arc::clone(&coords_arc);
        let perlin_temp = self.perlin.clone();
        
        for (chunk_idx, chunk) in coordinates.chunks(chunk_size).enumerate() {
            let results_clone = Arc::clone(&temp_results);
            let coords_clone = Arc::clone(&temp_coords);
            let generator_clone = perlin_temp.clone();
            let start_idx = chunk_idx * chunk_size;
            let chunk_len = chunk.len();

            batch.add_task_with_resources(
                format!("terrain_temperature_chunk_{}", chunk_idx),
                vec![Resource::read::<NoiseResource>()],
                move || -> Result<(), SchedulerError> {
                    let mut results_guard = results_clone.lock().unwrap();
                    
                    for i in 0..chunk_len {
                        let (x, y) = coords_clone[start_idx + i];
                        // Temperature influenced by latitude and elevation
                        let base_temp = generator_clone.sample_uncached(x * 0.02, y * 0.02);
                        let latitude_effect = ((y * 0.001) as f64).cos() as f32 * 0.3; // Cooler toward poles
                        let final_temp = ((base_temp + latitude_effect) * 0.5 + 0.5).clamp(0.0, 1.0);
                        results_guard[start_idx + i].temperature = final_temp;
                    }
                    
                    Ok(())
                },
            );
        }

        // Generate moisture data
        let moisture_results = Arc::clone(&results);
        let moisture_coords = Arc::clone(&coords_arc);
        let worley_moisture = self.worley.clone();
        
        for (chunk_idx, chunk) in coordinates.chunks(chunk_size).enumerate() {
            let results_clone = Arc::clone(&moisture_results);
            let coords_clone = Arc::clone(&moisture_coords);
            let generator_clone = worley_moisture.clone();
            let start_idx = chunk_idx * chunk_size;
            let chunk_len = chunk.len();

            batch.add_task_with_resources(
                format!("terrain_moisture_chunk_{}", chunk_idx),
                vec![Resource::read::<NoiseResource>()],
                move || -> Result<(), SchedulerError> {
                    let mut results_guard = results_clone.lock().unwrap();
                    
                    for i in 0..chunk_len {
                        let (x, y) = coords_clone[start_idx + i];
                        let base_moisture = generator_clone.sample_uncached(x, y);
                        let ocean_proximity = ((x * x + y * y) as f64).sqrt() as f32 * 0.0001; // Simple ocean distance
                        let final_moisture = ((base_moisture + 1.0) * 0.5 - ocean_proximity * 0.1).clamp(0.0, 1.0);
                        results_guard[start_idx + i].moisture = final_moisture;
                    }
                    
                    Ok(())
                },
            );
        }

        // Execute all terrain generation tasks
        self.scheduler.add_batch(batch);
        self.scheduler.run_stage(Stage::Update).map_err(|errors| {
            errors.into_iter().next().unwrap_or_else(|| SchedulerError::TaskFailed("Unknown scheduler error".to_string()))
        })?;
        
        // Return final results
        let final_results = results.lock().unwrap().clone();
        Ok(final_results)
    }

    /// Generate Voronoi regions using scheduled task coordination
    pub fn generate_voronoi_scheduled(
        &self,
        width: u32,
        height: u32,
        region_size: u32,
    ) -> Result<Vec<Vec<f32>>, SchedulerError> {
        let voronoi_map = Arc::new(Mutex::new(vec![vec![0.0f32; width as usize]; height as usize]));
        let mut batch = TaskBatch::new(Stage::Update);
        
        // Process Voronoi in regions for better performance
        let regions_x = (width + region_size - 1) / region_size;
        let regions_y = (height + region_size - 1) / region_size;
        
        for region_y in 0..regions_y {
            for region_x in 0..regions_x {
                let map_clone = Arc::clone(&voronoi_map);
                let voronoi_clone = self.voronoi.clone();
                
                let x_start = region_x * region_size;
                let y_start = region_y * region_size;
                let x_end = width.min(x_start + region_size);
                let y_end = height.min(y_start + region_size);
                
                batch.add_task_with_resources(
                    format!("voronoi_region_{}_{}", region_x, region_y),
                    vec![Resource::write::<NoiseResource>()],
                    move || -> Result<(), SchedulerError> {
                        let mut map = map_clone.lock().unwrap();
                        
                        for y in y_start..y_end {
                            for x in x_start..x_end {
                                let distance = voronoi_clone.sample(x as f64, y as f64);
                                map[y as usize][x as usize] = distance;
                            }
                        }
                        
                        Ok(())
                    },
                );
            }
        }
        
        self.scheduler.add_batch(batch);
        self.scheduler.run_stage(Stage::Update).map_err(|errors| {
            errors.into_iter().next().unwrap_or_else(|| SchedulerError::TaskFailed("Unknown scheduler error".to_string()))
        })?;
        
        let final_map = voronoi_map.lock().unwrap().clone();
        Ok(final_map)
    }

    /// Generate coordinate grid for terrain generation
    fn generate_coordinate_grid(&self, width: u32, height: u32) -> Vec<(f64, f64)> {
        let mut coordinates = Vec::with_capacity((width * height) as usize);
        
        for y in 0..height {
            for x in 0..width {
                coordinates.push((x as f64, y as f64));
            }
        }
        
        coordinates
    }

    /// Get performance metrics from scheduler
    pub fn performance_metrics(&self) -> crate::core::scheduler::SchedulerMetrics {
        self.scheduler.metrics()
    }

    /// Clear scheduler state for testing
    pub fn clear_scheduler(&self) {
        self.scheduler.clear();
    }

    /// Get current active task count
    pub fn active_task_count(&self) -> usize {
        self.scheduler.active_count()
    }

    /// Check if noise generation is currently busy
    pub fn is_busy(&self) -> bool {
        self.scheduler.is_busy()
    }
}

/// Utility for resource management in noise generation
impl Resource {
    /// Create resource for specific noise generator type
    pub fn noise_generator(resource_type: NoiseResource) -> Self {
        Self {
            type_id: std::any::TypeId::of::<NoiseResource>(),
            name: format!("NoiseResource::{:?}", resource_type),
            access: crate::core::scheduler::Access::Read,
        }
    }

    /// Create writable resource for noise generation
    pub fn noise_generator_write(resource_type: NoiseResource) -> Self {
        Self {
            type_id: std::any::TypeId::of::<NoiseResource>(),
            name: format!("NoiseResource::{:?}", resource_type),
            access: crate::core::scheduler::Access::Write,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::scheduler::Scheduler;

    #[test]
    fn test_scheduled_noise_generator() {
        let scheduler = Arc::new(Scheduler::new(Some(4)).unwrap());
        let config = NoiseConfig::default();
        let generator = ScheduledNoiseGenerator::new(config, scheduler);
        
        assert_eq!(generator.active_task_count(), 0);
        assert!(!generator.is_busy());
    }

    #[test]
    fn test_heightmap_generation() {
        let scheduler = Arc::new(Scheduler::new(Some(2)).unwrap());
        let config = NoiseConfig::default();
        let generator = ScheduledNoiseGenerator::new(config, scheduler);
        
        let result = generator.generate_heightmap_scheduled(32, 32, 16);
        assert!(result.is_ok());
        
        let heightmap = result.unwrap();
        assert_eq!(heightmap.len(), 32);
        assert_eq!(heightmap[0].len(), 32);
    }

    #[test]
    fn test_terrain_data_generation() {
        let scheduler = Arc::new(Scheduler::new(Some(2)).unwrap());
        let config = NoiseConfig::default();
        let generator = ScheduledNoiseGenerator::new(config, scheduler);
        
        let coordinates = vec![(0.0, 0.0), (100.0, 100.0), (200.0, 200.0)];
        let result = generator.generate_terrain_data_scheduled(&coordinates);
        
        assert!(result.is_ok());
        let terrain_data = result.unwrap();
        assert_eq!(terrain_data.len(), 3);
        
        for data in &terrain_data {
            assert!(data.height >= -1.0 && data.height <= 1.0);
            assert!(data.temperature >= 0.0 && data.temperature <= 1.0);
            assert!(data.moisture >= 0.0 && data.moisture <= 1.0);
        }
    }

    #[test]
    fn test_performance_metrics() {
        let scheduler = Arc::new(Scheduler::new(Some(2)).unwrap());
        let config = NoiseConfig::default();
        let generator = ScheduledNoiseGenerator::new(config, scheduler);
        
        let metrics = generator.performance_metrics();
        assert_eq!(metrics.tasks_executed, 0);
    }
}
