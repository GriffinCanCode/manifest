//! Lake Detection System
//!
//! Detects lakes using union-find data structure and flood-fill algorithms
//! for identifying closed basins and water bodies.

use super::{HydrologyConfig, Lake, FlowAccumulation};
use super::zig_ffi::{zig_find_local_minima, zig_union_find_basins, zig_calculate_lake_volume, ZigBasin};
use crate::core::scheduler::SchedulerError;
use nalgebra::Vector2;

/// Lake detection system
#[derive(Debug)]
pub struct LakeDetector {
    config: HydrologyConfig,
}

impl LakeDetector {
    pub fn new(config: &HydrologyConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    /// Detect lakes from elevation and flow data using Zig backend
    pub fn detect_lakes(
        &self,
        elevation_data: &[f32],
        _flow_accumulation: &FlowAccumulation,
        world_size: (u32, u32)
    ) -> Result<Vec<Lake>, SchedulerError> {
        let (width, height) = world_size;
        
        // Use Zig backend for high-performance local minima detection
        let local_minima = zig_find_local_minima(elevation_data, world_size);
        
        if local_minima.is_empty() {
            return Ok(Vec::new());
        }
        
        // Use elevation threshold based on configuration
        let elevation_threshold = self.config.lake_min_depth;
        
        // Use Zig backend for union-find basin detection
        let zig_basins = zig_union_find_basins(elevation_data, world_size, elevation_threshold);
        
        // Convert Zig basins to lakes
        let lakes = self.create_lakes_from_zig_basins(zig_basins, elevation_data, width, height)?;
        
        Ok(lakes)
    }

    /// Create lake objects from Zig basins using high-performance backend
    fn create_lakes_from_zig_basins(
        &self,
        zig_basins: Vec<ZigBasin>,
        elevation_data: &[f32],
        width: u32,
        height: u32
    ) -> Result<Vec<Lake>, SchedulerError> {
        let mut lakes = Vec::new();
        
        for (lake_id, zig_basin) in zig_basins.into_iter().enumerate() {
            if zig_basin.cells.len() < 10 {
                continue; // Skip small basins
            }
            
            // Calculate basin properties from cells
            let basin_stats = self.calculate_basin_statistics(&zig_basin.cells, elevation_data, width);
            let depth = basin_stats.max_elevation - basin_stats.min_elevation;
            
            // Only create lakes for basins that meet criteria
            if depth < self.config.lake_min_depth {
                continue;
            }

            // Convert basin center from grid to world coordinates
            let world_center = self.grid_to_world(
                basin_stats.center_x as usize, 
                basin_stats.center_y as usize
            );
            
            // Calculate lake volume using Zig backend for performance
            let cell_area = (self.grid_cell_size() as f64).powi(2);
            let volume = zig_calculate_lake_volume(
                &zig_basin.cells,
                elevation_data,
                basin_stats.max_elevation,
                cell_area,
            );
            
            let surface_elevation = basin_stats.min_elevation + depth * 0.8; // 80% filled
            let radius = basin_stats.radius * self.grid_cell_size();

            lakes.push(Lake {
                id: lake_id as u32,
                center: world_center,
                radius,
                depth,
                surface_elevation,
                volume,
                drainage_rivers: Vec::new(), // Will be populated later
            });
        }

        Ok(lakes)
    }

    /// Calculate statistical properties of a basin from its cells
    fn calculate_basin_statistics(
        &self,
        cells: &[usize],
        elevation_data: &[f32],
        width: u32
    ) -> BasinStatistics {
        if cells.is_empty() {
            return BasinStatistics::default();
        }

        let mut min_elevation = f32::MAX;
        let mut max_elevation = f32::MIN;
        let mut center_x = 0.0;
        let mut center_y = 0.0;

        for &cell_idx in cells {
            let x = (cell_idx % width as usize) as f32;
            let y = (cell_idx / width as usize) as f32;
            let elevation = elevation_data.get(cell_idx).copied().unwrap_or(0.0);
            
            center_x += x;
            center_y += y;
            min_elevation = min_elevation.min(elevation);
            max_elevation = max_elevation.max(elevation);
        }

        center_x /= cells.len() as f32;
        center_y /= cells.len() as f32;

        // Calculate approximate radius
        let radius = (cells.len() as f32 / std::f32::consts::PI).sqrt();

        BasinStatistics {
            center_x,
            center_y,
            min_elevation,
            max_elevation,
            radius,
        }
    }

    /// Convert grid coordinates to world coordinates
    fn grid_to_world(&self, grid_x: usize, grid_y: usize) -> Vector2<f64> {
        let (min_x, min_y, max_x, max_y) = self.config.world_bounds;
        let x = min_x + (grid_x as f64 / self.config.grid_resolution as f64) * (max_x - min_x);
        let y = min_y + (grid_y as f64 / self.config.grid_resolution as f64) * (max_y - min_y);
        Vector2::new(x, y)
    }

    /// Calculate size of each grid cell in world units
    fn grid_cell_size(&self) -> f32 {
        let (min_x, _min_y, max_x, _max_y) = self.config.world_bounds;
        ((max_x - min_x) / self.config.grid_resolution as f64) as f32
    }
}

/// Basin statistics calculated from cells
#[derive(Debug, Clone)]
struct BasinStatistics {
    center_x: f32,
    center_y: f32,
    min_elevation: f32,
    max_elevation: f32,
    radius: f32,
}

impl Default for BasinStatistics {
    fn default() -> Self {
        Self {
            center_x: 0.0,
            center_y: 0.0,
            min_elevation: 0.0,
            max_elevation: 0.0,
            radius: 0.0,
        }
    }
}
