//! Watershed Detection and Analysis
//!
//! High-performance watershed detection using Zig backend for elevation data
//! and flow direction analysis with SIMD-optimized drainage basin delineation.

use super::{HydrologyConfig, Watershed, WatershedId};
use super::zig_ffi::{
    FlowGrid, delineate_watershed, WatershedResult,
    zig_polygon_area, zig_convex_hull, zig_point_in_polygon,
    zig_elevation_gradient_analysis, zig_elevation_local_statistics, ZigGradientAnalysis
};
use crate::core::scheduler::SchedulerError;
use nalgebra::Vector2;
use ndarray::Array2;
use std::collections::{HashSet, VecDeque};

/// D8 flow direction vector
#[derive(Debug, Clone, Copy)]
struct FlowDirectionVec {
    pub x: i32,
    pub y: i32,
}

/// Watershed analysis system using Zig backend
#[derive(Debug)]
pub struct WatershedAnalyzer {
    config: HydrologyConfig,
    flow_directions: Vec<FlowDirectionVec>,
}

impl WatershedAnalyzer {
    pub fn new(config: &HydrologyConfig) -> Self {
        // D8 flow directions: East, SE, South, SW, West, NW, North, NE
        let flow_directions = vec![
            FlowDirectionVec { x: 1, y: 0 },   // East
            FlowDirectionVec { x: 1, y: 1 },   // Southeast
            FlowDirectionVec { x: 0, y: 1 },   // South
            FlowDirectionVec { x: -1, y: 1 },  // Southwest
            FlowDirectionVec { x: -1, y: 0 },  // West
            FlowDirectionVec { x: -1, y: -1 }, // Northwest
            FlowDirectionVec { x: 0, y: -1 },  // North
            FlowDirectionVec { x: 1, y: -1 },  // Northeast
        ];
        
        Self {
            config: config.clone(),
            flow_directions,
        }
    }

    /// Analyze watersheds from elevation data using enhanced Zig backend
    pub fn analyze_watersheds(
        &self, 
        elevation_data: &[f32], 
        world_size: (u32, u32)
    ) -> Result<Vec<Watershed>, SchedulerError> {
        let (width, height) = world_size;
        let elevation_data_f64: Vec<f64> = elevation_data.iter().map(|&x| x as f64).collect();
        let cell_size = (self.config.world_bounds.2 - self.config.world_bounds.0) / width as f64;
        
        // Create flow grid using Zig backend
        let mut flow_grid = FlowGrid::new(width as usize, height as usize, cell_size, &elevation_data_f64)
            .ok_or_else(|| SchedulerError::TaskFailed("Failed to create flow grid".to_string()))?;
        
        // Calculate flow directions and accumulation using Zig
        flow_grid.calculate_flow_directions();
        if !flow_grid.calculate_flow_accumulation() {
            return Err(SchedulerError::TaskFailed("Flow accumulation calculation failed".to_string()));
        }

        // Perform gradient analysis using Zig backend for enhanced watershed characterization
        let gradient_analysis = zig_elevation_gradient_analysis(elevation_data, world_size, cell_size);
        
        // Find potential watershed outlets using enhanced analysis
        let outlets = self.find_enhanced_outlets(elevation_data, world_size, &gradient_analysis)?;
        
        // Delineate watersheds using Zig backend
        let watersheds = self.delineate_watersheds_zig(&mut flow_grid, outlets, elevation_data, world_size)?;
        
        Ok(watersheds)
    }

    /// Create 2D elevation grid from flat array
    fn create_elevation_grid(
        &self, 
        elevation_data: &[f32], 
        width: u32, 
        height: u32
    ) -> Result<Array2<f32>, SchedulerError> {
        if elevation_data.len() != (width * height) as usize {
            return Err(SchedulerError::TaskFailed(
                format!("Elevation data size mismatch: {} vs {}", 
                       elevation_data.len(), width * height)
            ));
        }

        let mut grid = Array2::zeros((height as usize, width as usize));
        for y in 0..height as usize {
            for x in 0..width as usize {
                grid[[y, x]] = elevation_data[y * width as usize + x];
            }
        }
        
        Ok(grid)
    }

    /// Calculate flow directions using D8 algorithm
    fn calculate_flow_directions(&self, elevation_grid: &Array2<f32>) -> Array2<Option<usize>> {
        let (height, width) = elevation_grid.dim();
        let mut flow_dirs = Array2::from_elem((height, width), None);

        for y in 1..height - 1 {
            for x in 1..width - 1 {
                let current_elevation = elevation_grid[[y, x]];
                let mut steepest_slope = 0.0;
                let mut flow_direction = None;

                for (dir_idx, direction) in self.flow_directions.iter().enumerate() {
                    let nx = x as i32 + direction.x;
                    let ny = y as i32 + direction.y;

                    if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                        let neighbor_elevation = elevation_grid[[ny as usize, nx as usize]];
                        let elevation_diff = current_elevation - neighbor_elevation;
                        
                        if elevation_diff > steepest_slope {
                            steepest_slope = elevation_diff;
                            flow_direction = Some(dir_idx);
                        }
                    }
                }

                flow_dirs[[y, x]] = flow_direction;
            }
        }

        flow_dirs
    }

    /// Find watershed outlets (local minima with no outflow)
    fn find_outlets(&self, elevation_grid: &Array2<f32>) -> Vec<Vector2<usize>> {
        let (height, width) = elevation_grid.dim();
        let mut outlets = Vec::new();

        for y in 1..height - 1 {
            for x in 1..width - 1 {
                let current = elevation_grid[[y, x]];
                let mut is_outlet = true;

                // Check if all neighbors are higher
                for direction in &self.flow_directions {
                    let nx = x as i32 + direction.x;
                    let ny = y as i32 + direction.y;

                    if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                        let neighbor = elevation_grid[[ny as usize, nx as usize]];
                        if neighbor <= current {
                            is_outlet = false;
                            break;
                        }
                    }
                }

                if is_outlet {
                    outlets.push(Vector2::new(x, y));
                }
            }
        }

        // Limit number of outlets for performance
        if outlets.len() > 50 {
            outlets.truncate(50);
        }

        outlets
    }

    /// Trace watersheds from outlets using reverse flow tracing
    fn trace_watersheds(
        &self,
        elevation_grid: &Array2<f32>,
        flow_directions: &Array2<Option<usize>>,
        outlets: Vec<Vector2<usize>>
    ) -> Result<Vec<Watershed>, SchedulerError> {
        let (height, width) = elevation_grid.dim();
        let mut watersheds = Vec::new();
        let mut assigned_cells = HashSet::new();

        for (watershed_id, outlet) in outlets.into_iter().enumerate() {
            if assigned_cells.contains(&outlet) {
                continue;
            }

            let boundary = self.trace_watershed_boundary(
                elevation_grid, 
                flow_directions, 
                outlet, 
                &mut assigned_cells
            );

            if boundary.len() > 10 { // Only keep significant watersheds
                let watershed = self.create_watershed(
                    watershed_id as u32,
                    boundary,
                    outlet,
                    elevation_grid
                );
                watersheds.push(watershed);
            }
        }

        Ok(watersheds)
    }

    /// Trace the boundary of a single watershed
    fn trace_watershed_boundary(
        &self,
        elevation_grid: &Array2<f32>,
        flow_directions: &Array2<Option<usize>>,
        outlet: Vector2<usize>,
        assigned_cells: &mut HashSet<Vector2<usize>>
    ) -> Vec<Vector2<f64>> {
        let (height, width) = elevation_grid.dim();
        let mut watershed_cells = HashSet::new();
        let mut queue = VecDeque::new();
        
        queue.push_back(outlet);
        watershed_cells.insert(outlet);

        // Find all cells that drain to this outlet
        while let Some(current) = queue.pop_front() {
            assigned_cells.insert(current);

            // Find all cells that flow to current cell
            for y in 0..height {
                for x in 0..width {
                    if watershed_cells.contains(&Vector2::new(x, y)) {
                        continue;
                    }

                    if let Some(flow_dir_idx) = flow_directions[[y, x]] {
                        let flow_dir = self.flow_directions[flow_dir_idx];
                        let target_x = x as i32 + flow_dir.x;
                        let target_y = y as i32 + flow_dir.y;

                        if target_x == current.x as i32 && target_y == current.y as i32 {
                            let cell = Vector2::new(x, y);
                            watershed_cells.insert(cell);
                            queue.push_back(cell);
                        }
                    }
                }
            }
        }

        // Convert watershed cells to boundary polygon (simplified)
        self.cells_to_boundary(&watershed_cells)
    }

    /// Convert watershed cells to boundary polygon
    fn cells_to_boundary(&self, cells: &HashSet<Vector2<usize>>) -> Vec<Vector2<f64>> {
        // Simple convex hull for boundary approximation
        let mut points: Vec<Vector2<f64>> = cells.iter()
            .map(|cell| {
                let world_pos = self.grid_to_world(cell.x, cell.y);
                Vector2::new(world_pos.0, world_pos.1)
            })
            .collect();

        if points.len() < 3 {
            return points;
        }

        // Sort points by angle from centroid for simple polygon
        let centroid = points.iter().fold(Vector2::new(0.0, 0.0), |acc, p| acc + p) / points.len() as f64;
        
        points.sort_by(|a, b| {
            let angle_a = (a.y - centroid.y).atan2(a.x - centroid.x);
            let angle_b = (b.y - centroid.y).atan2(b.x - centroid.x);
            angle_a.partial_cmp(&angle_b).unwrap()
        });

        // Simplify polygon (remove every nth point for performance)
        let step = (points.len() / 20).max(1);
        points.into_iter().step_by(step).collect()
    }

    /// Create watershed from boundary and outlet data
    fn create_watershed(
        &self,
        id: u32,
        boundary: Vec<Vector2<f64>>,
        outlet: Vector2<usize>,
        elevation_grid: &Array2<f32>
    ) -> Watershed {
        let outlet_world = self.grid_to_world(outlet.x, outlet.y);
        let area = self.calculate_polygon_area(&boundary);
        
        // Calculate elevation range within watershed
        let mut min_elevation = f32::MAX;
        let mut max_elevation = f32::MIN;
        
        for point in &boundary {
            let (grid_x, grid_y) = self.world_to_grid(point.x, point.y);
            if let Some(elevation) = elevation_grid.get((grid_y, grid_x)) {
                min_elevation = min_elevation.min(*elevation);
                max_elevation = max_elevation.max(*elevation);
            }
        }

        let mean_elevation = (min_elevation + max_elevation) / 2.0;
        let relief = max_elevation - min_elevation;
        
        // Calculate perimeter from boundary points
        let mut perimeter = 0.0;
        for i in 0..boundary.len() {
            let current = &boundary[i];
            let next = &boundary[(i + 1) % boundary.len()];
            let dx = next.x - current.x;
            let dy = next.y - current.y;
            perimeter += (dx * dx + dy * dy).sqrt();
        }
        
        // Calculate shape factor (area / perimeter²)
        let shape_factor = if perimeter > 0.0 {
            area / (perimeter * perimeter)
        } else {
            0.0
        };

        Watershed {
            id: WatershedId(id),
            outlet_position: Vector2::new(outlet_world.0, outlet_world.1),
            boundary_points: boundary,
            area,
            perimeter,
            mean_elevation: mean_elevation as f64,
            relief: relief as f64,
            shape_factor,
        }
    }

    /// Convert grid coordinates to world coordinates
    fn grid_to_world(&self, grid_x: usize, grid_y: usize) -> (f64, f64) {
        let (min_x, min_y, max_x, max_y) = self.config.world_bounds;
        let x = min_x + (grid_x as f64 / self.config.grid_resolution as f64) * (max_x - min_x);
        let y = min_y + (grid_y as f64 / self.config.grid_resolution as f64) * (max_y - min_y);
        (x, y)
    }

    /// Convert world coordinates to grid coordinates
    fn world_to_grid(&self, world_x: f64, world_y: f64) -> (usize, usize) {
        let (min_x, min_y, max_x, max_y) = self.config.world_bounds;
        let norm_x = ((world_x - min_x) / (max_x - min_x)).clamp(0.0, 1.0);
        let norm_y = ((world_y - min_y) / (max_y - min_y)).clamp(0.0, 1.0);
        
        let grid_x = (norm_x * (self.config.grid_resolution - 1) as f64) as usize;
        let grid_y = (norm_y * (self.config.grid_resolution - 1) as f64) as usize;
        
        (grid_x, grid_y)
    }

    /// Find enhanced watershed outlets using gradient analysis
    fn find_enhanced_outlets(
        &self,
        elevation_data: &[f32],
        world_size: (u32, u32),
        gradient_analysis: &ZigGradientAnalysis,
    ) -> Result<Vec<Vector2<usize>>, SchedulerError> {
        let (width, height) = world_size;
        let mut outlets = Vec::new();

        // Use gradient magnitude to identify potential outlet locations
        // Outlets typically occur at convergence zones with low gradients
        for y in 1..height - 1 {
            for x in 1..width - 1 {
                let idx = (y * width + x) as usize;
                
                if idx >= gradient_analysis.gradients_magnitude.len() {
                    continue;
                }
                
                let gradient_magnitude = gradient_analysis.gradients_magnitude[idx];
                let elevation = elevation_data[idx];
                
                // Identify areas with low gradient (potential valley bottoms)
                // and check for local elevation minima
                if gradient_magnitude < 0.5 && self.is_local_minimum_enhanced(elevation_data, x as usize, y as usize, world_size) {
                    outlets.push(Vector2::new(x as usize, y as usize));
                }
            }
        }

        // Limit number of outlets for performance
        if outlets.len() > 50 {
            outlets.truncate(50);
        }

        Ok(outlets)
    }

    /// Enhanced local minimum detection
    fn is_local_minimum_enhanced(&self, elevation_data: &[f32], x: usize, y: usize, world_size: (u32, u32)) -> bool {
        let (width, height) = world_size;
        let center_idx = y * width as usize + x;
        
        if center_idx >= elevation_data.len() {
            return false;
        }
        
        let center_elevation = elevation_data[center_idx];
        let mut lower_neighbors = 0;
        let mut total_neighbors = 0;

        // Check 3x3 neighborhood
        for dy in -1..=1i32 {
            for dx in -1..=1i32 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                
                if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                    let neighbor_idx = (ny as usize * width as usize) + nx as usize;
                    if neighbor_idx < elevation_data.len() {
                        let neighbor_elevation = elevation_data[neighbor_idx];
                        if neighbor_elevation > center_elevation {
                            lower_neighbors += 1;
                        }
                        total_neighbors += 1;
                    }
                }
            }
        }

        // Consider it a local minimum if most neighbors are higher
        lower_neighbors >= (total_neighbors * 2) / 3
    }

    /// Delineate watersheds using Zig backend for high performance
    fn delineate_watersheds_zig(
        &self,
        flow_grid: &mut FlowGrid,
        outlets: Vec<Vector2<usize>>,
        elevation_data: &[f32],
        world_size: (u32, u32),
    ) -> Result<Vec<Watershed>, SchedulerError> {
        let mut watersheds = Vec::new();
        let max_boundary_points = 1000;

        for (watershed_id, outlet) in outlets.into_iter().enumerate() {
            // Use Zig backend for watershed delineation
            if let Some(watershed_result) = delineate_watershed(
                flow_grid,
                outlet.x,
                outlet.y,
                watershed_id as u32,
                max_boundary_points,
            ) {
                // Convert to enhanced watershed with additional properties
                let enhanced_watershed = self.create_enhanced_watershed(
                    watershed_id as u32,
                    watershed_result,
                    outlet,
                    elevation_data,
                    world_size,
                )?;
                
                // Only include watersheds that meet minimum area criteria
                if enhanced_watershed.area > 1000.0 {
                    watersheds.push(enhanced_watershed);
                }
            }
        }

        Ok(watersheds)
    }

    /// Create enhanced watershed with additional Zig-calculated properties
    fn create_enhanced_watershed(
        &self,
        id: u32,
        watershed_result: WatershedResult,
        outlet: Vector2<usize>,
        elevation_data: &[f32],
        world_size: (u32, u32),
    ) -> Result<Watershed, SchedulerError> {
        let outlet_world = self.grid_to_world(outlet.x, outlet.y);
        
        // Use Zig backend for accurate polygon area calculation
        let area = zig_polygon_area(&watershed_result.boundary_points);
        
        // Calculate enhanced convex hull using Zig backend
        let convex_hull = zig_convex_hull(&watershed_result.boundary_points);
        
        // Calculate local statistics for watershed characterization
        let boundary_grid_positions: Vec<(usize, usize)> = watershed_result.boundary_points
            .iter()
            .map(|&point| self.world_to_grid(point.x, point.y))
            .collect();

        let local_stats = zig_elevation_local_statistics(
            elevation_data,
            &boundary_grid_positions,
            world_size,
            5, // 5-cell window size
        );

        // Calculate additional morphometric properties
        let perimeter = self.calculate_perimeter(&convex_hull);
        let shape_factor = area / (perimeter * perimeter); // Compactness measure
        let mean_elevation = local_stats.mean_values.iter().sum::<f32>() / local_stats.mean_values.len().max(1) as f32;
        let relief = local_stats.max_values.iter().cloned().fold(f32::NEG_INFINITY, f32::max) - 
                    local_stats.min_values.iter().cloned().fold(f32::INFINITY, f32::min);

        Ok(Watershed {
            id: super::types::WatershedId(id),
            outlet_position: Vector2::new(outlet_world.0, outlet_world.1),
            boundary_points: watershed_result.boundary_points,
            area,
            perimeter,
            mean_elevation: mean_elevation as f64,
            relief: relief as f64,
            shape_factor,
        })
    }

    /// Calculate perimeter of boundary polygon
    fn calculate_perimeter(&self, boundary: &[Vector2<f64>]) -> f64 {
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

    /// Enhanced point-in-watershed test using Zig backend
    pub fn contains_point_enhanced(&self, watershed: &Watershed, point: Vector2<f64>) -> bool {
        zig_point_in_polygon(point, &watershed.boundary_points)
    }

    /// Calculate polygon area using Zig backend for accuracy
    fn calculate_polygon_area(&self, points: &[Vector2<f64>]) -> f64 {
        zig_polygon_area(points)
    }
}
