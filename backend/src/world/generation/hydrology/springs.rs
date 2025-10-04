//! Spring Generation System
//!
//! Generates springs based on aquifer systems and topographical conditions.
//! Uses probabilistic models and geological analysis for realistic spring placement.

use super::{HydrologyConfig, Spring, Aquifer};
use super::types::{SpringId, SpringType};
use crate::core::scheduler::SchedulerError;
use nalgebra::Vector2;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// Spring generation system
#[derive(Debug)]
pub struct SpringGenerator {
    config: HydrologyConfig,
    rng: ChaCha8Rng,
}

impl SpringGenerator {
    pub fn new(config: &HydrologyConfig) -> Self {
        Self {
            config: config.clone(),
            rng: ChaCha8Rng::seed_from_u64(config.seed + 3000), // Different seed offset
        }
    }

    /// Generate springs from aquifer systems
    pub fn generate_springs(
        &mut self,
        aquifers: &[Aquifer],
        elevation_data: &[f32],
        world_size: (u32, u32)
    ) -> Result<Vec<Spring>, SchedulerError> {
        if aquifers.is_empty() {
            return Ok(Vec::new());
        }

        // Generate springs for each aquifer
        let mut springs = Vec::new();
        for (base_id, aquifer) in aquifers.iter().enumerate() {
            let aquifer_springs = self.generate_springs_for_aquifer(
                base_id * 100, // Ensure unique IDs
                aquifer, 
                elevation_data, 
                world_size
            );
            springs.push(aquifer_springs);
        }

        // Flatten and return
        Ok(springs.into_iter().flatten().collect())
    }

    /// Generate springs for a single aquifer
    fn generate_springs_for_aquifer(
        &self,
        base_id: usize,
        aquifer: &Aquifer,
        elevation_data: &[f32],
        world_size: (u32, u32)
    ) -> Vec<Spring> {
        let mut springs = Vec::new();
        let mut local_rng = ChaCha8Rng::seed_from_u64(self.config.seed + aquifer.id.0 as u64);
        
        // Determine number of springs based on aquifer properties
        let num_springs = self.calculate_spring_count(aquifer);
        
        for spring_idx in 0..num_springs {
            if let Some(spring_position) = self.find_spring_location(aquifer, elevation_data, world_size, &mut local_rng) {
                let spring = self.create_spring(
                    (base_id + spring_idx) as u32,
                    spring_position,
                    aquifer,
                    elevation_data,
                    world_size,
                    &mut local_rng
                );
                springs.push(spring);
            }
        }
        
        springs
    }

    /// Calculate number of springs for an aquifer
    fn calculate_spring_count(&self, aquifer: &Aquifer) -> usize {
        // Base count on aquifer size and recharge rate
        let boundary_length = self.calculate_aquifer_perimeter(&aquifer.boundary);
        let size_factor = (boundary_length / 1000.0) as usize; // ~1 spring per km of perimeter
        let recharge_factor = (aquifer.recharge_rate * 10.0) as usize;
        
        let base_count = size_factor.max(1) + recharge_factor;
        
        // Apply randomness
        let variation = (base_count as f32 * 0.3) as usize;
        if variation > 0 {
            base_count.saturating_sub(variation / 2) + (self.config.seed as usize % variation)
        } else {
            base_count
        }.min(10) // Reasonable maximum per aquifer
    }

    /// Calculate aquifer perimeter
    fn calculate_aquifer_perimeter(&self, boundary: &[Vector2<f64>]) -> f64 {
        if boundary.len() < 2 {
            return 0.0;
        }
        
        let mut perimeter = 0.0;
        for i in 0..boundary.len() {
            let current = boundary[i];
            let next = boundary[(i + 1) % boundary.len()];
            
            let dx = next.x - current.x;
            let dy = next.y - current.y;
            perimeter += (dx * dx + dy * dy).sqrt();
        }
        
        perimeter
    }

    /// Find suitable spring location near aquifer boundary
    fn find_spring_location(
        &self,
        aquifer: &Aquifer,
        elevation_data: &[f32],
        world_size: (u32, u32),
        rng: &mut ChaCha8Rng
    ) -> Option<Vector2<f64>> {
        const MAX_ATTEMPTS: usize = 20;
        
        for _ in 0..MAX_ATTEMPTS {
            // Select random point on aquifer boundary
            let boundary_point = self.select_boundary_point(&aquifer.boundary, rng);
            
            // Add some offset to place spring near but not exactly on boundary
            let offset_distance = rng.gen_range(50.0..200.0);
            let offset_angle = rng.gen_range(0.0..2.0 * std::f64::consts::PI);
            
            let spring_position = Vector2::new(
                boundary_point.x + offset_distance * offset_angle.cos(),
                boundary_point.y + offset_distance * offset_angle.sin()
            );
            
            // Check if position is suitable
            if self.is_suitable_spring_location(spring_position, aquifer, elevation_data, world_size) {
                return Some(spring_position);
            }
        }
        
        None
    }

    /// Select a point on aquifer boundary
    fn select_boundary_point(&self, boundary: &[Vector2<f64>], rng: &mut ChaCha8Rng) -> Vector2<f64> {
        if boundary.len() <= 2 {
            return boundary.get(0).copied().unwrap_or(Vector2::new(0.0, 0.0));
        }
        
        // Select random segment and interpolate
        let segment_idx = rng.gen_range(0..boundary.len());
        let next_idx = (segment_idx + 1) % boundary.len();
        
        let start = boundary[segment_idx];
        let end = boundary[next_idx];
        let t = rng.gen::<f64>();
        
        Vector2::new(
            start.x + t * (end.x - start.x),
            start.y + t * (end.y - start.y)
        )
    }

    /// Check if location is suitable for spring
    fn is_suitable_spring_location(
        &self,
        position: Vector2<f64>,
        aquifer: &Aquifer,
        elevation_data: &[f32],
        world_size: (u32, u32)
    ) -> bool {
        // Check if within reasonable bounds
        let (min_x, min_y, max_x, max_y) = self.config.world_bounds;
        if position.x < min_x || position.x > max_x || position.y < min_y || position.y > max_y {
            return false;
        }
        
        let grid_pos = self.world_to_grid(position, world_size);
        let elevation = self.get_elevation_at_grid(grid_pos, elevation_data, world_size);
        
        // Springs typically form where water table intersects surface
        // This is a simplified check based on elevation and aquifer water table
        let elevation_suitable = elevation > aquifer.water_table_elevation - 5.0 && 
                                elevation < aquifer.water_table_elevation + 10.0;
        
        // Prefer areas with some topographic relief (slopes where springs naturally emerge)
        let has_topographic_relief = self.check_topographic_relief(grid_pos, elevation_data, world_size);
        
        elevation_suitable && has_topographic_relief
    }

    /// Check for topographic relief (slope) that favors spring formation
    fn check_topographic_relief(
        &self,
        grid_pos: (usize, usize),
        elevation_data: &[f32],
        world_size: (u32, u32)
    ) -> bool {
        let (x, y) = grid_pos;
        let center_elevation = self.get_elevation_at_grid(grid_pos, elevation_data, world_size);
        
        // Check for elevation gradient in neighborhood
        let mut max_gradient: f32 = 0.0;
        
        for dy in -2..=2i32 {
            for dx in -2..=2i32 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                
                if nx >= 0 && nx < world_size.0 as i32 && ny >= 0 && ny < world_size.1 as i32 {
                    let neighbor_elevation = self.get_elevation_at_grid(
                        (nx as usize, ny as usize),
                        elevation_data,
                        world_size
                    );
                    
                    let distance = ((dx * dx + dy * dy) as f32).sqrt();
                    let gradient = (center_elevation - neighbor_elevation).abs() / distance;
                    max_gradient = max_gradient.max(gradient);
                }
            }
        }
        
        // Prefer areas with moderate slope (not flat, not too steep)
        max_gradient > 0.5 && max_gradient < 10.0
    }

    /// Create spring with calculated properties
    fn create_spring(
        &self,
        spring_id: u32,
        position: Vector2<f64>,
        aquifer: &Aquifer,
        elevation_data: &[f32],
        world_size: (u32, u32),
        rng: &mut ChaCha8Rng
    ) -> Spring {
        let grid_pos = self.world_to_grid(position, world_size);
        let surface_elevation = self.get_elevation_at_grid(grid_pos, elevation_data, world_size);
        
        // Calculate flow rate based on aquifer properties and hydraulic head
        let hydraulic_head = aquifer.water_table_elevation - surface_elevation;
        let base_flow = aquifer.porosity * aquifer.recharge_rate * hydraulic_head.max(0.0);
        let flow_rate = base_flow * rng.gen_range(0.5..2.0); // Add variation
        
        // Calculate temperature (simplified model)
        let depth_factor = aquifer.depth / 100.0; // Geothermal gradient
        let base_temperature = 12.0; // Average groundwater temperature
        let temperature = base_temperature + depth_factor * 3.0; // ~3°C per 100m depth
        
        // Calculate mineral content based on geology and residence time
        let residence_time_factor = aquifer.depth / aquifer.recharge_rate;
        let mineral_content = (residence_time_factor * 0.01).min(1.0).max(0.1);
        
        Spring {
            id: SpringId(spring_id),
            position,
            flow_rate,
            temperature,
            aquifer_id: Some(aquifer.id),
            mineral_content,
            spring_type: self.determine_spring_type_zig(aquifer, flow_rate),
        }
    }

    /// Determine spring type based on aquifer properties
    fn determine_spring_type_zig(&self, aquifer: &Aquifer, flow_rate: f32) -> SpringType {
        use super::types::SpringType;
        use crate::world::generation::hydrology::zig_ffi::AquiferType;
        
        let hydraulic_head = aquifer.hydraulic_head;
        
        // Determine spring type based on aquifer characteristics
        match (aquifer.aquifer_type, hydraulic_head, flow_rate) {
            // High pressure springs (artesian)
            (AquiferType::Confined, head, _) if head > 20.0 => SpringType::Artesian,
            (AquiferType::LeakyConfined, head, _) if head > 15.0 => SpringType::Artesian,
            
            // Contact springs at geological boundaries
            (AquiferType::FracturedRock, _, _) => SpringType::Contact,
            (AquiferType::Karst, _, _) => SpringType::Joint,
            
            // High-flow thermal springs
            (_, _, flow) if flow > 0.1 && aquifer.depth > 200.0 => SpringType::Thermal,
            
            // Depression springs in low-lying areas
            (_, _, _) if hydraulic_head < 5.0 => SpringType::Depression,
            
            // Default gravity springs
            _ => SpringType::Gravity,
        }
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
            0.0
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
}
