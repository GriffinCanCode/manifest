//! Hydrological type definitions
//!
//! Core types used throughout the hydrological simulation system with enhanced Zig integration

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use nalgebra::Vector2;
use crate::core::scheduler::SchedulerError;
use std::fmt;

/// Unique identifier for watersheds
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WatershedId(pub u32);

/// Unique identifier for rivers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RiverId(pub u32);

/// Unique identifier for lakes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LakeId(pub u32);

/// Unique identifier for wetlands
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WetlandId(pub u32);

/// Unique identifier for aquifers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AquiferId(pub u32);

/// Unique identifier for springs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpringId(pub u32);

/// Water flow direction vectors
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FlowDirectionVector {
    pub direction: Vector2<f32>,
    pub magnitude: f32,
}

/// Enhanced flow accumulation data structure with Zig backend integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowAccumulation {
    width: u32,
    height: u32,
    world_bounds: (f64, f64, f64, f64),
    cell_size: f64,
    // Flow data storage for high-performance access
    flow_values: Option<Vec<f32>>,
    flow_directions: Option<Vec<FlowDirectionVector>>,
}

impl FlowAccumulation {
    pub fn new(
        width: u32,
        height: u32,
        world_bounds: (f64, f64, f64, f64),
    ) -> Result<Self, SchedulerError> {
        let cell_size = (world_bounds.2 - world_bounds.0) / width as f64;
        Ok(Self {
            width,
            height,
            world_bounds,
            cell_size,
            flow_values: None,
            flow_directions: None,
        })
    }

    /// Create flow accumulation with pre-computed data for performance
    pub fn with_data(
        width: u32,
        height: u32,
        world_bounds: (f64, f64, f64, f64),
        flow_values: Vec<f32>,
        flow_directions: Vec<FlowDirectionVector>,
    ) -> Result<Self, SchedulerError> {
        let cell_size = (world_bounds.2 - world_bounds.0) / width as f64;
        Ok(Self {
            width,
            height,
            world_bounds,
            cell_size,
            flow_values: Some(flow_values),
            flow_directions: Some(flow_directions),
        })
    }

    pub fn get_flow_value(&self, x: f64, y: f64) -> f32 {
        if let Some(ref flow_values) = self.flow_values {
            let grid_pos = self.world_to_grid(x, y);
            let index = grid_pos.1 * self.width as usize + grid_pos.0;
            flow_values.get(index).copied().unwrap_or(0.0)
        } else {
            // Fallback for compatibility
            0.0
        }
    }

    pub fn get_flow_direction(&self, x: f64, y: f64) -> Vector2<f32> {
        if let Some(ref flow_directions) = self.flow_directions {
            let grid_pos = self.world_to_grid(x, y);
            let index = grid_pos.1 * self.width as usize + grid_pos.0;
            flow_directions.get(index)
                .map(|fv| fv.direction)
                .unwrap_or_else(|| Vector2::new(0.0, 0.0))
        } else {
            // Fallback for compatibility
            Vector2::new(0.0, 0.0)
        }
    }

    /// Get raw flow data for Zig backend processing
    pub fn get_flow_data(&self) -> Option<&Vec<f32>> {
        self.flow_values.as_ref()
    }

    /// Get raw direction data for Zig backend processing
    pub fn get_direction_data(&self) -> Option<&Vec<FlowDirectionVector>> {
        self.flow_directions.as_ref()
    }

    /// Convert world coordinates to grid indices
    fn world_to_grid(&self, x: f64, y: f64) -> (usize, usize) {
        let (min_x, min_y, max_x, max_y) = self.world_bounds;
        let norm_x = ((x - min_x) / (max_x - min_x)).clamp(0.0, 1.0);
        let norm_y = ((y - min_y) / (max_y - min_y)).clamp(0.0, 1.0);
        
        let grid_x = (norm_x * (self.width - 1) as f64) as usize;
        let grid_y = (norm_y * (self.height - 1) as f64) as usize;
        
        (grid_x, grid_y)
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn world_bounds(&self) -> (f64, f64, f64, f64) {
        self.world_bounds
    }

    pub fn cell_size(&self) -> f64 {
        self.cell_size
    }
}

/// Watershed representation using Zig backend results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Watershed {
    pub id: WatershedId,
    pub outlet_position: Vector2<f64>,
    pub boundary_points: Vec<Vector2<f64>>,
    pub area: f64,          // m²
    pub perimeter: f64,     // m
    pub mean_elevation: f64, // m
    pub relief: f64,        // m (max - min elevation)
    pub shape_factor: f64,  // dimensionless (area / perimeter²)
}

impl Watershed {
    pub fn contains_point(&self, x: f64, y: f64) -> bool {
        // Point-in-polygon test using ray casting
        let point = Vector2::new(x, y);
        let mut inside = false;
        let n = self.boundary_points.len();
        
        if n < 3 {
            return false;
        }
        
        for i in 0..n {
            let j = if i == 0 { n - 1 } else { i - 1 };
            let pi = self.boundary_points[i];
            let pj = self.boundary_points[j];
            
            if ((pi.y > point.y) != (pj.y > point.y)) &&
               (point.x < (pj.x - pi.x) * (point.y - pi.y) / (pj.y - pi.y) + pi.x) {
                inside = !inside;
            }
        }
        
        inside
    }
}

/// River segment for detailed river representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiverSegment {
    pub position: Vector2<f64>,
    pub width: f32,         // m
    pub depth: f32,         // m
    pub flow_rate: f32,     // m³/s
    pub elevation: f32,     // m
}

/// River representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct River {
    pub id: RiverId,
    pub segments: Vec<RiverSegment>,
    pub length: f64,        // m
    pub discharge: f32,     // m³/s
}

/// Enhanced lake representation with Zig-calculated properties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lake {
    pub id: LakeId,
    pub center: Vector2<f64>,
    pub radius: f32,        // m
    pub depth: f32,         // m
    pub volume: f32,        // m³ (calculated by Zig backend)
    pub water_level: f32,   // m elevation
    pub surface_elevation: f32, // m elevation of water surface
    pub drainage_rivers: Vec<RiverId>, // Connected rivers
}

/// Enhanced wetland representation with ecological modeling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wetland {
    pub id: WetlandId,
    pub center: Vector2<f64>,
    pub radius: f32,        // m
    pub water_depth: f32,   // m
    pub vegetation_density: f32, // 0.0 - 1.0
    pub wetland_type: WetlandType,
    pub biodiversity_index: f32, // 0.0 - 1.0 (calculated by Zig backend)
    pub seasonal_variation: f32, // 0.0 - 1.0
}

/// Types of wetlands
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WetlandType {
    Marsh,
    Swamp,
    Bog,
    Fen,
    Delta,
}

/// Enhanced aquifer representation with advanced hydrogeology
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Aquifer {
    pub id: AquiferId,
    pub center: Vector2<f64>,
    pub extent: f64,        // m (radius of influence)
    pub depth: f32,         // m
    pub permeability: f64,  // m/s
    pub porosity: f32,      // dimensionless
    pub hydraulic_head: f32, // m
    pub water_table_elevation: f32, // m elevation
    pub recharge_rate: f32, // m/year
    pub boundary: Vec<Vector2<f64>>, // Aquifer boundary polygon
    pub aquifer_type: crate::world::generation::hydrology::zig_ffi::AquiferType,
}

/// Enhanced spring representation with detailed characteristics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spring {
    pub id: SpringId,
    pub position: Vector2<f64>,
    pub flow_rate: f32,     // m³/s (calculated by Zig backend)
    pub temperature: f32,   // °C
    pub aquifer_id: Option<AquiferId>, // Optional aquifer connection
    pub mineral_content: f32, // 0.0 - 1.0 (relative mineral concentration)
    pub spring_type: SpringType,
}

/// Types of springs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpringType {
    Gravity,      // Topographic springs
    Artesian,     // Pressure springs
    Contact,      // Geological contact springs
    Depression,   // Springs in valleys
    Joint,        // Fracture springs
    Thermal,      // Hot springs
}

/// Performance-oriented hydrological data cache for Zig integration
#[derive(Debug, Clone)]
pub struct HydrologyDataCache {
    pub elevation_data: Vec<f32>,
    pub flow_data: Vec<f32>,
    pub gradient_data: Option<Vec<f32>>,
    pub world_size: (u32, u32),
    pub world_bounds: (f64, f64, f64, f64),
    pub cell_size: f64,
}

impl HydrologyDataCache {
    pub fn new(
        elevation_data: Vec<f32>,
        world_size: (u32, u32),
        world_bounds: (f64, f64, f64, f64),
    ) -> Self {
        let cell_size = (world_bounds.2 - world_bounds.0) / world_size.0 as f64;
        Self {
            elevation_data,
            flow_data: Vec::new(),
            gradient_data: None,
            world_size,
            world_bounds,
            cell_size,
        }
    }

    pub fn with_flow_data(mut self, flow_data: Vec<f32>) -> Self {
        self.flow_data = flow_data;
        self
    }

    pub fn with_gradient_data(mut self, gradient_data: Vec<f32>) -> Self {
        self.gradient_data = Some(gradient_data);
        self
    }

    pub fn total_cells(&self) -> usize {
        (self.world_size.0 * self.world_size.1) as usize
    }
}

/// High-performance spatial point for Zig backend operations
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpatialPoint {
    pub x: f64,
    pub y: f64,
    pub data: f32, // Associated data value (elevation, flow, etc.)
}

impl SpatialPoint {
    pub fn new(x: f64, y: f64, data: f32) -> Self {
        Self { x, y, data }
    }

    pub fn distance_to(&self, other: &SpatialPoint) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

/// Batch processing result for Zig operations
#[derive(Debug, Clone)]
pub struct BatchProcessingResult<T> {
    pub results: Vec<T>,
    pub success_count: usize,
    pub error_count: usize,
    pub processing_time_ms: u64,
}

impl<T> BatchProcessingResult<T> {
    pub fn new(results: Vec<T>) -> Self {
        let success_count = results.len();
        Self {
            results,
            success_count,
            error_count: 0,
            processing_time_ms: 0,
        }
    }

    pub fn with_timing(mut self, processing_time_ms: u64) -> Self {
        self.processing_time_ms = processing_time_ms;
        self
    }

    pub fn success_rate(&self) -> f32 {
        if self.success_count + self.error_count == 0 {
            1.0
        } else {
            self.success_count as f32 / (self.success_count + self.error_count) as f32
        }
    }
}

/// Zig integration utility traits for type conversions
pub trait ZigConvertible {
    type ZigRepr;
    
    fn to_zig_format(&self) -> Self::ZigRepr;
    fn from_zig_format(zig_data: Self::ZigRepr) -> Self;
}

/// Performance metrics for Zig-integrated operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HydrologyPerformanceMetrics {
    pub flood_fill_time_ms: u64,
    pub pathfinding_time_ms: u64,
    pub spatial_queries_time_ms: u64,
    pub gradient_analysis_time_ms: u64,
    pub total_zig_operations: u32,
    pub memory_usage_mb: f32,
}

impl Default for HydrologyPerformanceMetrics {
    fn default() -> Self {
        Self {
            flood_fill_time_ms: 0,
            pathfinding_time_ms: 0,
            spatial_queries_time_ms: 0,
            gradient_analysis_time_ms: 0,
            total_zig_operations: 0,
            memory_usage_mb: 0.0,
        }
    }
}

impl fmt::Display for HydrologyPerformanceMetrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Hydrology Performance: {}ms total, {} operations, {:.1}MB memory",
            self.flood_fill_time_ms + self.pathfinding_time_ms + self.spatial_queries_time_ms + self.gradient_analysis_time_ms,
            self.total_zig_operations,
            self.memory_usage_mb
        )
    }
}