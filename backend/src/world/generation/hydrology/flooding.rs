//! Flooding Simulation System
//!
//! Implements flood simulation using floodfill algorithms and hydraulic modeling.
//! Provides flood risk assessment and inundation mapping.

use super::{HydrologyConfig, FlowAccumulation};
use super::zig_ffi::{zig_flood_fill_inundation, zig_batch_flood_risk_assessment, FloodInundationResult};
use crate::core::scheduler::SchedulerError;
use nalgebra::Vector2;

/// Flood simulation system
#[derive(Debug)]
pub struct FloodSimulator {
    config: HydrologyConfig,
}

/// Flood event representation
#[derive(Debug, Clone)]
pub struct FloodEvent {
    pub id: u32,
    pub source_position: Vector2<f64>,
    pub intensity: f32,
    pub inundated_area: Vec<Vector2<f64>>,
    pub max_depth: f32,
    pub duration_hours: f32,
}

/// Flood risk assessment for a location
#[derive(Debug, Clone)]
pub struct FloodRisk {
    pub position: Vector2<f64>,
    pub risk_level: f32,      // 0.0 to 1.0
    pub return_period: f32,   // Years
    pub max_expected_depth: f32,
}

impl FloodSimulator {
    pub fn new(config: &HydrologyConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    /// Simulate flood event using floodfill algorithm
    pub fn simulate_flood(
        &self,
        source: Vector2<f64>,
        intensity: f32,
        elevation_data: &[f32],
        flow_accumulation: &FlowAccumulation,
        world_size: (u32, u32)
    ) -> Result<FloodEvent, SchedulerError> {
        let (width, height) = world_size;
        
        // Convert source to grid coordinates
        let source_grid = self.world_to_grid(source, world_size);
        
        // Perform floodfill to find inundated area
        let inundated_cells = self.floodfill_inundation(
            source_grid,
            intensity,
            elevation_data,
            flow_accumulation,
            world_size
        );
        
        // Convert grid cells back to world coordinates
        let inundated_area: Vec<Vector2<f64>> = inundated_cells.into_iter()
            .map(|(x, y)| self.grid_to_world((x, y), world_size))
            .collect();
        
        // Calculate flood properties
        let max_depth = self.calculate_max_flood_depth(intensity, &inundated_area, elevation_data, world_size);
        let duration = self.estimate_flood_duration(intensity, inundated_area.len());
        
        Ok(FloodEvent {
            id: 0, // Will be assigned by caller
            source_position: source,
            intensity,
            inundated_area,
            max_depth,
            duration_hours: duration,
        })
    }

    /// Perform floodfill algorithm for flood inundation using Zig backend
    fn floodfill_inundation(
        &self,
        start: (usize, usize),
        intensity: f32,
        elevation_data: &[f32],
        _flow_accumulation: &FlowAccumulation,
        world_size: (u32, u32)
    ) -> Vec<(usize, usize)> {
        // Calculate flood level
        let start_elevation = self.get_elevation_at_grid(start, elevation_data, world_size);
        let flood_level = start_elevation + intensity;
        
        // Use Zig backend for high-performance flood fill
        let result = zig_flood_fill_inundation(start, world_size, flood_level, elevation_data);
        
        if result.max_cells_reached {
            // Log warning about potential incomplete flood simulation
            log::warn!("Flood simulation may be incomplete - reached maximum cell limit");
        }
        
        result.inundated_cells
    }

    /// Calculate flood risk assessment for multiple points using Zig backend
    pub fn assess_flood_risk(
        &self,
        points: &[Vector2<f64>],
        elevation_data: &[f32],
        flow_accumulation: &FlowAccumulation,
        world_size: (u32, u32)
    ) -> Result<Vec<FloodRisk>, SchedulerError> {
        if points.is_empty() {
            return Ok(Vec::new());
        }
        
        // Convert flow accumulation to raw data for Zig processing
        // Note: This is a simplified approach - in practice we'd need to extract
        // the actual flow data from the FlowAccumulation structure
        let flow_data = self.extract_flow_data_for_zig(flow_accumulation, world_size);
        
        // Use Zig batch processing for high performance
        let risk_results = zig_batch_flood_risk_assessment(
            points,
            elevation_data,
            &flow_data,
            self.config.world_bounds,
            world_size,
        );
        
        // Convert results to FloodRisk structures
        let flood_risks: Vec<FloodRisk> = points.iter()
            .zip(risk_results.iter())
            .map(|(&position, &(risk_level, return_period, max_depth))| {
                FloodRisk {
                    position,
                    risk_level,
                    return_period,
                    max_expected_depth: max_depth,
                }
            })
            .collect();
        
        Ok(flood_risks)
    }

    /// Extract flow data for Zig processing (placeholder implementation)
    fn extract_flow_data_for_zig(
        &self,
        flow_accumulation: &FlowAccumulation,
        world_size: (u32, u32),
    ) -> Vec<f32> {
        let (width, height) = world_size;
        let total_cells = (width * height) as usize;
        let mut flow_data = vec![0.0f32; total_cells];
        
        // Sample flow values across the grid
        // Note: This is a placeholder - the actual implementation would depend
        // on how FlowAccumulation stores its data internally
        for y in 0..height {
            for x in 0..width {
                let world_pos = self.grid_to_world((x as usize, y as usize), world_size);
                let flow_value = flow_accumulation.get_flow_value(world_pos.x, world_pos.y);
                let idx = (y * width + x) as usize;
                flow_data[idx] = flow_value;
            }
        }
        
        flow_data
    }

    /// Calculate flood risk for a single point
    fn calculate_flood_risk_for_point(
        &self,
        position: Vector2<f64>,
        elevation_data: &[f32],
        flow_accumulation: &FlowAccumulation,
        world_size: (u32, u32)
    ) -> FloodRisk {
        let grid_pos = self.world_to_grid(position, world_size);
        let elevation = self.get_elevation_at_grid(grid_pos, elevation_data, world_size);
        let flow_value = flow_accumulation.get_flow_value(position.x, position.y);
        
        // Risk factors
        let elevation_risk = self.calculate_elevation_risk(elevation);
        let flow_risk = self.calculate_flow_risk(flow_value);
        let topographic_risk = self.calculate_topographic_risk(grid_pos, elevation_data, world_size);
        
        // Combined risk score
        let risk_level = (elevation_risk + flow_risk + topographic_risk) / 3.0;
        
        // Estimate return period (inverse relationship with risk)
        let return_period = if risk_level > 0.01 {
            1.0 / risk_level
        } else {
            1000.0 // Very low risk = very high return period
        };
        
        // Estimate maximum expected flood depth
        let max_expected_depth = risk_level * 5.0; // Up to 5m in highest risk areas
        
        FloodRisk {
            position,
            risk_level: risk_level.min(1.0),
            return_period,
            max_expected_depth,
        }
    }

    /// Calculate elevation-based flood risk
    fn calculate_elevation_risk(&self, elevation: f32) -> f32 {
        // Lower elevation = higher risk
        let normalized_elevation = (elevation / 100.0).min(1.0); // Assume max elevation of 100m
        (1.0 - normalized_elevation).max(0.0)
    }

    /// Calculate flow-based flood risk
    fn calculate_flow_risk(&self, flow_value: f32) -> f32 {
        // Higher flow accumulation = higher risk
        (flow_value / 100.0).min(1.0) // Normalize by expected max flow
    }

    /// Calculate topographic flood risk (slope and surroundings)
    fn calculate_topographic_risk(
        &self,
        grid_pos: (usize, usize),
        elevation_data: &[f32],
        world_size: (u32, u32)
    ) -> f32 {
        let (width, height) = world_size;
        let (x, y) = grid_pos;
        let center_elevation = self.get_elevation_at_grid(grid_pos, elevation_data, world_size);
        
        let mut higher_neighbors = 0;
        let mut total_neighbors = 0;
        
        // Check surrounding area
        for dy in -2..=2i32 {
            for dx in -2..=2i32 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                
                if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                    let neighbor_elevation = self.get_elevation_at_grid(
                        (nx as usize, ny as usize), 
                        elevation_data, 
                        world_size
                    );
                    
                    if neighbor_elevation > center_elevation {
                        higher_neighbors += 1;
                    }
                    total_neighbors += 1;
                }
            }
        }
        
        // Risk increases with more higher neighbors (basin-like areas)
        if total_neighbors > 0 {
            higher_neighbors as f32 / total_neighbors as f32
        } else {
            0.0
        }
    }

    /// Calculate maximum flood depth
    fn calculate_max_flood_depth(
        &self,
        intensity: f32,
        inundated_area: &[Vector2<f64>],
        elevation_data: &[f32],
        world_size: (u32, u32)
    ) -> f32 {
        if inundated_area.is_empty() {
            return 0.0;
        }
        
        let elevations: Vec<f32> = inundated_area.iter()
            .map(|&pos| {
                let grid_pos = self.world_to_grid(pos, world_size);
                self.get_elevation_at_grid(grid_pos, elevation_data, world_size)
            })
            .collect();
        
        let min_elevation = elevations.iter().cloned().fold(f32::INFINITY, f32::min);
        intensity.max(min_elevation) - min_elevation
    }

    /// Estimate flood duration based on intensity and area
    fn estimate_flood_duration(&self, intensity: f32, area_cells: usize) -> f32 {
        // Simple model: higher intensity and larger area = longer duration
        let base_duration = intensity * 2.0; // 2 hours per unit intensity
        let area_factor = (area_cells as f32).sqrt() / 100.0; // Area influence
        
        (base_duration + area_factor * 6.0).min(72.0) // Max 72 hours
    }

    /// Get elevation at grid coordinates
    fn get_elevation_at_grid(
        &self,
        grid_pos: (usize, usize),
        elevation_data: &[f32],
        world_size: (u32, u32)
    ) -> f32 {
        let (width, _height) = world_size;
        let (x, y) = grid_pos;
        let index = y * width as usize + x;
        
        if index < elevation_data.len() {
            elevation_data[index]
        } else {
            0.0 // Default elevation for out-of-bounds
        }
    }

    /// Convert world coordinates to grid coordinates
    fn world_to_grid(&self, world_pos: Vector2<f64>, world_size: (u32, u32)) -> (usize, usize) {
        let (min_x, min_y, max_x, max_y) = self.config.world_bounds;
        let norm_x = ((world_pos.x - min_x) / (max_x - min_x)).clamp(0.0, 1.0);
        let norm_y = ((world_pos.y - min_y) / (max_y - min_y)).clamp(0.0, 1.0);
        
        let grid_x = (norm_x * (world_size.0 - 1) as f64) as usize;
        let grid_y = (norm_y * (world_size.1 - 1) as f64) as usize;
        
        (grid_x, grid_y)
    }

    /// Convert grid coordinates to world coordinates
    fn grid_to_world(&self, grid_pos: (usize, usize), world_size: (u32, u32)) -> Vector2<f64> {
        let (min_x, min_y, max_x, max_y) = self.config.world_bounds;
        let (x, y) = grid_pos;
        
        let world_x = min_x + (x as f64 / world_size.0 as f64) * (max_x - min_x);
        let world_y = min_y + (y as f64 / world_size.1 as f64) * (max_y - min_y);
        
        Vector2::new(world_x, world_y)
    }
}
