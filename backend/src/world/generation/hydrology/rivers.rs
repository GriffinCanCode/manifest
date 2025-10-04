//! River Generation System
//!
//! Generates rivers using flow accumulation data and pathfinding algorithms.
//! Uses priority queues for source generation and A* pathfinding for river routing.

use super::{HydrologyConfig, River, RiverSegment, FlowAccumulation, Watershed};
use super::zig_ffi::{zig_find_river_sources, zig_river_astar_pathfinding};
use crate::core::scheduler::SchedulerError;
use crate::world::{WatershedId, RiverId};
use nalgebra::Vector2;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// River generation system
#[derive(Debug)]
pub struct RiverGenerator {
    config: HydrologyConfig,
    rng: ChaCha8Rng,
}

impl RiverGenerator {
    pub fn new(config: &HydrologyConfig) -> Self {
        Self {
            config: config.clone(),
            rng: ChaCha8Rng::seed_from_u64(config.seed),
        }
    }

    /// Generate rivers from flow accumulation and watersheds
    pub fn generate_rivers(
        &mut self,
        flow_accumulation: &FlowAccumulation,
        watersheds: &[Watershed]
    ) -> Result<Vec<River>, SchedulerError> {
        // Find potential river sources using flow accumulation
        let sources = self.find_river_sources(flow_accumulation)?;
        
        // Generate rivers from sources using pathfinding
        let rivers = self.trace_rivers_from_sources(flow_accumulation, watersheds, sources)?;
        
        Ok(rivers)
    }

    /// Find river sources using Zig backend for high-performance source detection
    fn find_river_sources(&mut self, flow_accumulation: &FlowAccumulation) -> Result<Vec<Vector2<f64>>, SchedulerError> {
        let width = flow_accumulation.width();
        let height = flow_accumulation.height();
        let world_size = (width, height);
        
        // Extract flow data for Zig processing
        let flow_data = self.extract_flow_data_for_zig(flow_accumulation, world_size);
        
        // Sample every 8th point for performance
        let sample_step = 8;
        
        // Use Zig backend for high-performance source detection
        let zig_sources = zig_find_river_sources(
            &flow_data,
            world_size,
            self.config.river_threshold,
            sample_step,
        );
        
        // Convert grid coordinates to world coordinates and add randomness
        let selected_sources: Vec<Vector2<f64>> = zig_sources
            .into_iter()
            .take(100) // Limit for performance
            .map(|(grid_x, grid_y, priority)| {
                let mut world_pos = self.grid_to_world_from_indices(grid_x, grid_y, world_size);
                
                // Add small amount of randomness to avoid grid patterns
                let random_offset_x = (self.rng.gen::<f64>() - 0.5) * 10.0;
                let random_offset_y = (self.rng.gen::<f64>() - 0.5) * 10.0;
                world_pos.x += random_offset_x;
                world_pos.y += random_offset_y;
                
                world_pos
            })
            .collect();

        Ok(selected_sources)
    }

    /// Extract flow data for Zig processing
    fn extract_flow_data_for_zig(
        &self,
        flow_accumulation: &FlowAccumulation,
        world_size: (u32, u32),
    ) -> Vec<f32> {
        let (width, height) = world_size;
        let total_cells = (width * height) as usize;
        let mut flow_data = vec![0.0f32; total_cells];
        
        for y in 0..height {
            for x in 0..width {
                let world_pos = self.grid_to_world_from_indices(x as usize, y as usize, world_size);
                let flow_value = flow_accumulation.get_flow_value(world_pos.x, world_pos.y);
                let idx = (y * width + x) as usize;
                flow_data[idx] = flow_value;
            }
        }
        
        flow_data
    }

    /// Convert grid indices to world coordinates
    fn grid_to_world_from_indices(
        &self,
        grid_x: usize,
        grid_y: usize,
        world_size: (u32, u32),
    ) -> Vector2<f64> {
        let (min_x, min_y, max_x, max_y) = self.config.world_bounds;
        let (width, height) = world_size;
        
        let norm_x = grid_x as f64 / (width - 1) as f64;
        let norm_y = grid_y as f64 / (height - 1) as f64;
        
        let world_x = min_x + norm_x * (max_x - min_x);
        let world_y = min_y + norm_y * (max_y - min_y);
        
        Vector2::new(world_x, world_y)
    }

    /// Trace rivers from sources using sequential processing
    fn trace_rivers_from_sources(
        &mut self,
        flow_accumulation: &FlowAccumulation,
        watersheds: &[Watershed],
        sources: Vec<Vector2<f64>>
    ) -> Result<Vec<River>, SchedulerError> {
        let mut successful_rivers = Vec::new();
        
        for (river_id, source) in sources.into_iter().enumerate() {
            match self.trace_single_river(river_id as u32, source, flow_accumulation, watersheds) {
                Ok(river) => successful_rivers.push(river),
                Err(_) => continue, // Skip failed rivers
            }
        }

        Ok(successful_rivers)
    }

    /// Trace a single river from source to mouth
    fn trace_single_river(
        &self,
        river_id: u32,
        source: Vector2<f64>,
        flow_accumulation: &FlowAccumulation,
        watersheds: &[Watershed]
    ) -> Result<River, SchedulerError> {
        // Find the watershed containing this source
        let watershed_id = watersheds.iter()
            .find(|w| w.contains_point(source.x, source.y))
            .map(|w| w.id)
            .unwrap_or(WatershedId(0));

        // Use A* pathfinding to find path from source to watershed outlet
        let watershed = watersheds.iter().find(|w| w.id == watershed_id);
        let mouth = watershed.map(|w| w.outlet_position).unwrap_or(source);

        let path = self.find_river_path(source, mouth, flow_accumulation)?;
        
        // Convert path to river segments
        let segments = self.create_river_segments(&path, flow_accumulation);
        
        let length = self.calculate_river_length(&segments);
        let total_discharge = segments.iter().map(|s| s.flow_rate).sum::<f32>();

        Ok(River {
            id: RiverId(river_id),
            segments,
            length,
            discharge: total_discharge,
        })
    }

    /// Find optimal river path using Zig A* pathfinding
    fn find_river_path(
        &self,
        start: Vector2<f64>,
        goal: Vector2<f64>,
        flow_accumulation: &FlowAccumulation
    ) -> Result<Vec<Vector2<f64>>, SchedulerError> {
        // Define grid resolution for pathfinding
        let grid_resolution = 10.0; // World units per grid cell
        
        // Convert world coordinates to grid coordinates
        let start_grid = self.world_to_grid_pos(start, grid_resolution);
        let goal_grid = self.world_to_grid_pos(goal, grid_resolution);
        
        // Extract data for Zig pathfinding
        let world_size = (flow_accumulation.width(), flow_accumulation.height());
        let flow_data = self.extract_flow_data_for_zig(flow_accumulation, world_size);
        let elevation_data = self.create_elevation_placeholder(world_size);
        
        // Use Zig backend for high-performance A* pathfinding
        let zig_result = zig_river_astar_pathfinding(
            start_grid,
            goal_grid,
            &flow_data,
            &elevation_data,
            self.config.world_bounds,
            grid_resolution,
        );
        
        if zig_result.success && !zig_result.path.is_empty() {
            // Convert grid path back to world coordinates
            let world_path: Vec<Vector2<f64>> = zig_result.path.into_iter()
                .map(|(grid_x, grid_y)| self.grid_pos_to_world((grid_x, grid_y), grid_resolution))
                .collect();
            Ok(world_path)
        } else {
            // Fallback: straight line path if pathfinding fails
            tracing::warn!("River pathfinding failed, using straight line fallback");
            Ok(vec![start, goal])
        }
    }

    /// Create placeholder elevation data (would be replaced with actual elevation data)
    fn create_elevation_placeholder(&self, world_size: (u32, u32)) -> Vec<f32> {
        let (width, height) = world_size;
        let total_cells = (width * height) as usize;
        
        // Create simple elevation gradient as placeholder
        let mut elevation_data = vec![0.0f32; total_cells];
        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) as usize;
                elevation_data[idx] = 100.0 - (x + y) as f32 * 0.1; // Simple gradient
            }
        }
        elevation_data
    }

    /// Calculate movement cost for pathfinding
    fn calculate_movement_cost(&self, pos: Vector2<f64>, flow_accumulation: &FlowAccumulation) -> f32 {
        let flow_direction = flow_accumulation.get_flow_direction(pos.x, pos.y);
        
        // Prefer following flow direction (lower cost)
        // Rivers naturally follow the steepest descent
        let base_cost = 1.0;
        let flow_bonus = flow_direction.magnitude() * 0.1; // Bonus for following flow
        
        base_cost - flow_bonus.min(0.8) // Ensure cost stays positive
    }

    /// Convert world coordinates to grid position
    fn world_to_grid_pos(&self, world_pos: Vector2<f64>, grid_step: f64) -> (i32, i32) {
        let x = (world_pos.x / grid_step) as i32;
        let y = (world_pos.y / grid_step) as i32;
        (x, y)
    }

    /// Convert grid position to world coordinates
    fn grid_pos_to_world(&self, grid_pos: (i32, i32), grid_step: f64) -> Vector2<f64> {
        Vector2::new(grid_pos.0 as f64 * grid_step, grid_pos.1 as f64 * grid_step)
    }

    /// Create river segments from path
    fn create_river_segments(&self, path: &[Vector2<f64>], flow_accumulation: &FlowAccumulation) -> Vec<RiverSegment> {
        path.iter().enumerate().map(|(i, &position)| {
            let flow_value = flow_accumulation.get_flow_value(position.x, position.y);
            
            // River properties based on flow accumulation
            let width = (flow_value * 0.1).min(50.0).max(1.0); // 1-50 units wide
            let depth = (flow_value * 0.01).min(5.0).max(0.1); // 0.1-5 units deep
            let flow_rate = flow_value * 0.5; // Flow rate based on accumulation
            
            // Estimate elevation (would typically come from elevation data)
            let elevation = 100.0 - (i as f32 * 0.5); // Gradually decreasing

            RiverSegment {
                position,
                width,
                depth,
                flow_rate,
                elevation,
            }
        }).collect()
    }

    /// Calculate total river length
    fn calculate_river_length(&self, segments: &[RiverSegment]) -> f64 {
        if segments.len() < 2 {
            return 0.0;
        }

        let mut total_length = 0.0;
        for i in 1..segments.len() {
            let prev = segments[i - 1].position;
            let curr = segments[i].position;
            let dx = curr.x - prev.x;
            let dy = curr.y - prev.y;
            total_length += (dx * dx + dy * dy).sqrt();
        }
        
        total_length
    }

    /// Convert grid coordinates to world coordinates
    fn grid_to_world(&self, grid_x: usize, grid_y: usize, flow_accumulation: &FlowAccumulation) -> Vector2<f64> {
        let (min_x, min_y, max_x, max_y) = self.config.world_bounds;
        let width = flow_accumulation.width() as f64;
        let height = flow_accumulation.height() as f64;
        
        let x = min_x + (grid_x as f64 / width) * (max_x - min_x);
        let y = min_y + (grid_y as f64 / height) * (max_y - min_y);
        
        Vector2::new(x, y)
    }
}
