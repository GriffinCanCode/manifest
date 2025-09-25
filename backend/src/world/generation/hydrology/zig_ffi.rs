//! Zig FFI Integration for Hydrology Calculations
//!
//! High-performance SIMD-optimized hydrological calculations using Zig functions
//! for performance-critical water flow, watershed analysis, and hydraulic computations.

use nalgebra::Vector2;
use std::os::raw::{c_uint, c_uchar};

// External Zig function declarations
extern "C" {
    // Flow grid operations
    fn hydrologyCreateFlowGrid(
        width: usize,
        height: usize,
        cell_size: f64,
        elevation_data: *const f64,
    ) -> *mut FlowGridC;

    fn hydrologyCalculateFlowDirections(grid_ptr: *mut FlowGridC);

    fn hydrologyCalculateFlowAccumulation(grid_ptr: *mut FlowGridC) -> bool;

    fn hydrologyDestroyFlowGrid(grid_ptr: *mut FlowGridC);

    // Manning's equation and hydraulics
    fn manifest_hydraulics_manning(
        area: f64,
        wetted_perimeter: f64,
        slope: f64,
        manning_n: f64,
        velocity_result: *mut f64,
        discharge_result: *mut f64,
        hydraulic_radius_result: *mut f64,
    );

    fn manifest_hydraulics_critical_depth(
        discharge: f64,
        width: f64,
        gravity: f64,
    ) -> f64;

    fn manifest_hydraulics_normal_depth(
        discharge: f64,
        width: f64,
        slope: f64,
        manning_n: f64,
        channel_type: u8,
        side_slope: f64,
    ) -> f64;

    fn manifest_hydraulics_froude_number(velocity: f64, depth: f64) -> f64;

    fn manifest_hydraulics_reynolds_number(
        velocity: f64,
        hydraulic_radius: f64,
        kinematic_viscosity: f64,
    ) -> f64;

    // Watershed operations
    fn manifest_watershed_delineate(
        flow_grid: *mut FlowGridC,
        outlet_x: usize,
        outlet_y: usize,
        watershed_id: u32,
        boundary_points_x: *mut f64,
        boundary_points_y: *mut f64,
        boundary_points_elevation: *mut f64,
        max_boundary_points: usize,
        boundary_count: *mut usize,
        area: *mut f64,
        perimeter: *mut f64,
        relief: *mut f64,
    ) -> bool;

    fn manifest_watershed_calculate_morphometrics(
        boundary_points_x: *const f64,
        boundary_points_y: *const f64,
        boundary_points_elevation: *const f64,
        boundary_count: usize,
        area: *mut f64,
        perimeter: *mut f64,
        shape_factor: *mut f64,
        mean_elevation: *mut f64,
        relief: *mut f64,
    );

    fn manifest_watershed_time_of_concentration(
        stream_length: f64,
        relief: f64,
    ) -> f64;

    // Groundwater and aquifer operations
    fn manifest_aquifer_darcy_velocity(
        hydraulic_conductivity: f64,
        head_gradient_x: f64,
        head_gradient_y: f64,
        velocity_x: *mut f64,
        velocity_y: *mut f64,
        magnitude: *mut f64,
    );

    fn manifest_aquifer_seepage_velocity(
        darcy_velocity_x: f64,
        darcy_velocity_y: f64,
        porosity: f64,
        seepage_x: *mut f64,
        seepage_y: *mut f64,
    );

    fn manifest_aquifer_theis_solution(
        distance: f64,
        time: f64,
        pumping_rate: f64,
        transmissivity: f64,
        storativity: f64,
    ) -> f64;

    fn manifest_spring_discharge(
        head_difference: f64,
        aquifer_type: u8,
    ) -> f64;

    fn manifest_spring_seasonal_discharge(
        base_discharge: f64,
        seasonal_variation: f64,
        day_of_year: u32,
    ) -> f64;

    // Batch operations
    fn manifest_batch_manning_calculations(
        areas: *const f64,
        wetted_perimeters: *const f64,
        slopes: *const f64,
        manning_ns: *const f64,
        velocities: *mut f64,
        discharges: *mut f64,
        hydraulic_radii: *mut f64,
        count: usize,
    );

    fn manifest_batch_slope_calculations(
        elevations: *const f64,
        width: usize,
        height: usize,
        cell_size: f64,
        slopes: *mut f64,
    );

    fn manifest_batch_point_distances(
        points1_x: *const f64,
        points1_y: *const f64,
        points2_x: *const f64,
        points2_y: *const f64,
        count1: usize,
        count2: usize,
        distances: *mut f64,
    );

    // Flood simulation operations
    fn manifest_flood_fill_inundation(
        start_x: usize,
        start_y: usize,
        width: usize,
        height: usize,
        flood_level: f32,
        elevation_data: *const f32,
        inundated_cells_x: *mut usize,
        inundated_cells_y: *mut usize,
        max_cells: usize,
        cell_count: *mut usize,
    ) -> bool;

    fn manifest_flood_risk_assessment(
        positions_x: *const f64,
        positions_y: *const f64,
        count: usize,
        elevation_data: *const f32,
        flow_data: *const f32,
        world_bounds: *const f64, // [min_x, min_y, max_x, max_y]
        world_size: *const u32,   // [width, height]
        risk_levels: *mut f32,
        return_periods: *mut f32,
        max_depths: *mut f32,
    );

    // Lake detection operations
    fn manifest_lakes_find_local_minima(
        elevation_data: *const f32,
        width: u32,
        height: u32,
        minima_x: *mut u32,
        minima_y: *mut u32,
        minima_elevation: *mut f32,
        max_minima: usize,
        minima_count: *mut usize,
    );

    fn manifest_lakes_union_find_basins(
        elevation_data: *const f32,
        width: u32,
        height: u32,
        elevation_threshold: f32,
        basin_cells: *mut usize,
        basin_ids: *mut u32,
        total_cells: usize,
        basin_count: *mut usize,
    ) -> bool;

    fn manifest_lakes_calculate_volume(
        basin_cells: *const usize,
        cell_count: usize,
        elevation_data: *const f32,
        max_elevation: f32,
        cell_area: f64,
    ) -> f64;

    // River pathfinding operations
    fn manifest_rivers_astar_pathfinding(
        start_x: i32,
        start_y: i32,
        goal_x: i32,
        goal_y: i32,
        flow_data: *const f32,
        elevation_data: *const f32,
        world_bounds: *const f64,
        grid_resolution: f64,
        path_x: *mut i32,
        path_y: *mut i32,
        max_path_points: usize,
        path_length: *mut usize,
    ) -> bool;

    fn manifest_rivers_find_sources(
        flow_accumulation: *const f32,
        width: usize,
        height: usize,
        threshold: f64,
        sample_step: usize,
        sources_x: *mut usize,
        sources_y: *mut usize,
        source_priorities: *mut i32,
        max_sources: usize,
        source_count: *mut usize,
    );

    fn manifest_rivers_calculate_segments(
        path_x: *const i32,
        path_y: *const i32,
        path_length: usize,
        flow_data: *const f32,
        elevation_data: *const f32,
        world_bounds: *const f64,
        segments_x: *mut f64,
        segments_y: *mut f64,
        segments_width: *mut f32,
        segments_depth: *mut f32,
        segments_flow: *mut f32,
        segments_elevation: *mut f32,
    );

    // Spatial indexing operations for wetlands
    fn manifest_spatial_kdtree_create() -> *mut SpatialTreeC;
    fn manifest_spatial_kdtree_add_point(tree: *mut SpatialTreeC, x: f64, y: f64, id: usize);
    fn manifest_spatial_kdtree_nearest(
        tree: *mut SpatialTreeC,
        x: f64,
        y: f64,
        k: usize,
        result_ids: *mut usize,
        result_distances: *mut f64,
    ) -> usize;
    fn manifest_spatial_kdtree_within_radius(
        tree: *mut SpatialTreeC,
        x: f64,
        y: f64,
        radius: f64,
        result_ids: *mut usize,
        max_results: usize,
    ) -> usize;
    fn manifest_spatial_kdtree_destroy(tree: *mut SpatialTreeC);

    fn manifest_wetlands_evaluate_candidates(
        positions_x: *const f64,
        positions_y: *const f64,
        elevations: *const f32,
        flow_values: *const f32,
        candidate_count: usize,
        water_bodies_tree: *mut SpatialTreeC,
        suitability_scores: *mut f32,
        wetland_types: *mut u8,
    );

    // Polygon and geometry operations
    fn manifest_geometry_point_in_polygon(
        point_x: f64,
        point_y: f64,
        polygon_x: *const f64,
        polygon_y: *const f64,
        vertex_count: usize,
    ) -> bool;

    fn manifest_geometry_polygon_area(
        polygon_x: *const f64,
        polygon_y: *const f64,
        vertex_count: usize,
    ) -> f64;

    fn manifest_geometry_convex_hull(
        points_x: *const f64,
        points_y: *const f64,
        point_count: usize,
        hull_x: *mut f64,
        hull_y: *mut f64,
        max_hull_points: usize,
        hull_size: *mut usize,
    );

    // Advanced elevation analysis
    fn manifest_elevation_gradient_analysis(
        elevation_data: *const f32,
        width: usize,
        height: usize,
        cell_size: f64,
        gradients_x: *mut f32,
        gradients_y: *mut f32,
        gradients_magnitude: *mut f32,
    );

    fn manifest_elevation_local_statistics(
        elevation_data: *const f32,
        positions_x: *const usize,
        positions_y: *const usize,
        position_count: usize,
        width: usize,
        height: usize,
        window_size: i32,
        mean_values: *mut f32,
        std_values: *mut f32,
        min_values: *mut f32,
        max_values: *mut f32,
    );
}

/// Opaque pointer to Zig FlowGrid structure
#[repr(C)]
struct FlowGridC {
    _private: [u8; 0],
}

/// Opaque pointer to Zig spatial tree structure
#[repr(C)]
struct SpatialTreeC {
    _private: [u8; 0],
}

/// Flow direction enumeration matching Zig implementation
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowDirection {
    East = 1,
    Southeast = 2,
    South = 4,
    Southwest = 8,
    West = 16,
    Northwest = 32,
    North = 64,
    Northeast = 128,
    None = 0,
}

impl FlowDirection {
    pub fn to_offset(&self) -> (i32, i32) {
        match self {
            FlowDirection::East => (1, 0),
            FlowDirection::Southeast => (1, 1),
            FlowDirection::South => (0, 1),
            FlowDirection::Southwest => (-1, 1),
            FlowDirection::West => (-1, 0),
            FlowDirection::Northwest => (-1, -1),
            FlowDirection::North => (0, -1),
            FlowDirection::Northeast => (1, -1),
            FlowDirection::None => (0, 0),
        }
    }

    pub fn to_vector2(&self) -> Vector2<f32> {
        let (dx, dy) = self.to_offset();
        Vector2::new(dx as f32, dy as f32)
    }
}

/// Channel types for hydraulic calculations
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelType {
    Rectangular = 0,
    Trapezoidal = 1,
    Triangular = 2,
    Circular = 3,
    Parabolic = 4,
    Irregular = 5,
}

/// Aquifer types for groundwater modeling
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AquiferType {
    Unconfined = 0,
    Confined = 1,
    LeakyConfined = 2,
    Perched = 3,
    FracturedRock = 4,
    Karst = 5,
}

/// High-performance flow grid using Zig implementation
pub struct FlowGrid {
    ptr: *mut FlowGridC,
    width: usize,
    height: usize,
    cell_size: f64,
}

impl FlowGrid {
    /// Create new flow grid with elevation data
    pub fn new(width: usize, height: usize, cell_size: f64, elevation_data: &[f64]) -> Option<Self> {
        if elevation_data.len() != width * height {
            return None;
        }

        let ptr = unsafe {
            hydrologyCreateFlowGrid(width, height, cell_size, elevation_data.as_ptr())
        };

        if ptr.is_null() {
            return None;
        }

        Some(FlowGrid {
            ptr,
            width,
            height,
            cell_size,
        })
    }

    /// Calculate flow directions using D8 algorithm
    pub fn calculate_flow_directions(&mut self) {
        unsafe {
            hydrologyCalculateFlowDirections(self.ptr);
        }
    }

    /// Calculate flow accumulation
    pub fn calculate_flow_accumulation(&mut self) -> bool {
        unsafe {
            hydrologyCalculateFlowAccumulation(self.ptr)
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn cell_size(&self) -> f64 {
        self.cell_size
    }
}

impl Drop for FlowGrid {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe {
                hydrologyDestroyFlowGrid(self.ptr);
            }
        }
    }
}

/// Hydraulic calculation results
#[derive(Debug, Clone, PartialEq)]
pub struct HydraulicResults {
    pub velocity: f64,         // m/s
    pub discharge: f64,        // m³/s
    pub hydraulic_radius: f64, // m
    pub froude_number: f64,    // dimensionless
    pub reynolds_number: f64,  // dimensionless
}

/// Manning coefficients for different channel types
pub struct ManningCoefficients;

impl ManningCoefficients {
    pub const CONCRETE_LINED: f64 = 0.012;
    pub const EARTH_STRAIGHT: f64 = 0.030;
    pub const EARTH_WINDING: f64 = 0.035;
    pub const ROCK_CUT: f64 = 0.025;
    pub const NATURAL_CLEAN: f64 = 0.030;
    pub const NATURAL_WEEDS: f64 = 0.050;
    pub const NATURAL_STONES: f64 = 0.040;
    pub const FLOODPLAIN: f64 = 0.035;
    pub const FOREST_LIGHT: f64 = 0.080;
    pub const FOREST_HEAVY: f64 = 0.120;
}

/// Safe Rust wrapper for Manning's equation calculation
pub fn calculate_manning_flow(
    area: f64,
    wetted_perimeter: f64,
    slope: f64,
    manning_n: f64,
) -> HydraulicResults {
    let mut velocity = 0.0;
    let mut discharge = 0.0;
    let mut hydraulic_radius = 0.0;

    unsafe {
        manifest_hydraulics_manning(
            area,
            wetted_perimeter,
            slope,
            manning_n,
            &mut velocity,
            &mut discharge,
            &mut hydraulic_radius,
        );
    }

    // Calculate dimensionless numbers
    let depth = area / wetted_perimeter; // Approximation for hydraulic depth
    let froude_number = unsafe { manifest_hydraulics_froude_number(velocity, depth) };
    let kinematic_viscosity = 1.004e-6; // Water at 20°C
    let reynolds_number = unsafe {
        manifest_hydraulics_reynolds_number(velocity, hydraulic_radius, kinematic_viscosity)
    };

    HydraulicResults {
        velocity,
        discharge,
        hydraulic_radius,
        froude_number,
        reynolds_number,
    }
}

/// Calculate critical depth for open channel flow
pub fn calculate_critical_depth(discharge: f64, width: f64) -> f64 {
    let gravity = 9.81;
    unsafe { manifest_hydraulics_critical_depth(discharge, width, gravity) }
}

/// Calculate normal depth using Manning's equation
pub fn calculate_normal_depth(
    discharge: f64,
    width: f64,
    slope: f64,
    manning_n: f64,
    channel_type: ChannelType,
    side_slope: f64,
) -> f64 {
    unsafe {
        manifest_hydraulics_normal_depth(
            discharge,
            width,
            slope,
            manning_n,
            channel_type as u8,
            side_slope,
        )
    }
}

/// Watershed delineation results
#[derive(Debug, Clone)]
pub struct WatershedResult {
    pub boundary_points: Vec<Vector2<f64>>,
    pub boundary_elevations: Vec<f64>,
    pub area: f64,            // m²
    pub perimeter: f64,       // m
    pub relief: f64,          // m
    pub shape_factor: f64,    // dimensionless
    pub mean_elevation: f64,  // m
}

/// Delineate watershed from outlet point
pub fn delineate_watershed(
    flow_grid: &mut FlowGrid,
    outlet_x: usize,
    outlet_y: usize,
    watershed_id: u32,
    max_boundary_points: usize,
) -> Option<WatershedResult> {
    let mut boundary_points_x = vec![0.0; max_boundary_points];
    let mut boundary_points_y = vec![0.0; max_boundary_points];
    let mut boundary_points_elevation = vec![0.0; max_boundary_points];
    let mut boundary_count = 0;
    let mut area = 0.0;
    let mut perimeter = 0.0;
    let mut relief = 0.0;

    let success = unsafe {
        manifest_watershed_delineate(
            flow_grid.ptr,
            outlet_x,
            outlet_y,
            watershed_id,
            boundary_points_x.as_mut_ptr(),
            boundary_points_y.as_mut_ptr(),
            boundary_points_elevation.as_mut_ptr(),
            max_boundary_points,
            &mut boundary_count,
            &mut area,
            &mut perimeter,
            &mut relief,
        )
    };

    if !success || boundary_count == 0 {
        return None;
    }

    // Calculate morphometrics
    let mut shape_factor = 0.0;
    let mut mean_elevation = 0.0;
    unsafe {
        manifest_watershed_calculate_morphometrics(
            boundary_points_x.as_ptr(),
            boundary_points_y.as_ptr(),
            boundary_points_elevation.as_ptr(),
            boundary_count,
            &mut area,
            &mut perimeter,
            &mut shape_factor,
            &mut mean_elevation,
            &mut relief,
        );
    }

    // Convert to Vec<Vector2<f64>>
    let mut boundary_points = Vec::with_capacity(boundary_count);
    let mut boundary_elevations = Vec::with_capacity(boundary_count);

    for i in 0..boundary_count {
        boundary_points.push(Vector2::new(boundary_points_x[i], boundary_points_y[i]));
        boundary_elevations.push(boundary_points_elevation[i]);
    }

    Some(WatershedResult {
        boundary_points,
        boundary_elevations,
        area,
        perimeter,
        relief,
        shape_factor,
        mean_elevation,
    })
}

/// Calculate time of concentration using Kirpich equation
pub fn calculate_time_of_concentration(stream_length: f64, relief: f64) -> f64 {
    unsafe { manifest_watershed_time_of_concentration(stream_length, relief) }
}

/// Groundwater flow vector
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GroundwaterFlow {
    pub velocity: Vector2<f64>, // m/s
    pub magnitude: f64,         // m/s
}

/// Calculate Darcy velocity for groundwater flow
pub fn calculate_darcy_velocity(
    hydraulic_conductivity: f64,
    head_gradient: Vector2<f64>,
) -> GroundwaterFlow {
    let mut velocity_x = 0.0;
    let mut velocity_y = 0.0;
    let mut magnitude = 0.0;

    unsafe {
        manifest_aquifer_darcy_velocity(
            hydraulic_conductivity,
            head_gradient.x,
            head_gradient.y,
            &mut velocity_x,
            &mut velocity_y,
            &mut magnitude,
        );
    }

    GroundwaterFlow {
        velocity: Vector2::new(velocity_x, velocity_y),
        magnitude,
    }
}

/// Calculate seepage velocity (actual groundwater velocity)
pub fn calculate_seepage_velocity(darcy_velocity: Vector2<f64>, porosity: f64) -> Vector2<f64> {
    let mut seepage_x = 0.0;
    let mut seepage_y = 0.0;

    unsafe {
        manifest_aquifer_seepage_velocity(
            darcy_velocity.x,
            darcy_velocity.y,
            porosity,
            &mut seepage_x,
            &mut seepage_y,
        );
    }

    Vector2::new(seepage_x, seepage_y)
}

/// Calculate well pumping effects using Theis solution
pub fn calculate_theis_solution(
    distance: f64,
    time: f64,
    pumping_rate: f64,
    transmissivity: f64,
    storativity: f64,
) -> f64 {
    unsafe {
        manifest_aquifer_theis_solution(distance, time, pumping_rate, transmissivity, storativity)
    }
}

/// Calculate spring discharge based on aquifer conditions
pub fn calculate_spring_discharge(head_difference: f64, aquifer_type: AquiferType) -> f64 {
    unsafe { manifest_spring_discharge(head_difference, aquifer_type as u8) }
}

/// Calculate seasonal variation in spring discharge
pub fn calculate_seasonal_spring_discharge(
    base_discharge: f64,
    seasonal_variation: f64,
    day_of_year: u32,
) -> f64 {
    unsafe {
        manifest_spring_seasonal_discharge(base_discharge, seasonal_variation, day_of_year)
    }
}

/// Batch calculate Manning's equation for multiple channels
pub fn batch_manning_calculations(
    areas: &[f64],
    wetted_perimeters: &[f64],
    slopes: &[f64],
    manning_ns: &[f64],
) -> Option<Vec<HydraulicResults>> {
    let count = areas.len();
    if count != wetted_perimeters.len() || count != slopes.len() || count != manning_ns.len() {
        return None;
    }

    let mut velocities = vec![0.0; count];
    let mut discharges = vec![0.0; count];
    let mut hydraulic_radii = vec![0.0; count];

    unsafe {
        manifest_batch_manning_calculations(
            areas.as_ptr(),
            wetted_perimeters.as_ptr(),
            slopes.as_ptr(),
            manning_ns.as_ptr(),
            velocities.as_mut_ptr(),
            discharges.as_mut_ptr(),
            hydraulic_radii.as_mut_ptr(),
            count,
        );
    }

    let results: Vec<HydraulicResults> = (0..count)
        .map(|i| HydraulicResults {
            velocity: velocities[i],
            discharge: discharges[i],
            hydraulic_radius: hydraulic_radii[i],
            froude_number: 0.0, // Would need separate calculation
            reynolds_number: 0.0, // Would need separate calculation
        })
        .collect();

    Some(results)
}

/// Batch calculate slopes from elevation grid
pub fn batch_slope_calculations(
    elevation_data: &[f64],
    width: usize,
    height: usize,
    cell_size: f64,
) -> Vec<f64> {
    let count = width * height;
    let mut slopes = vec![0.0; count];

    unsafe {
        manifest_batch_slope_calculations(
            elevation_data.as_ptr(),
            width,
            height,
            cell_size,
            slopes.as_mut_ptr(),
        );
    }

    slopes
}

/// Flood simulation results
#[derive(Debug, Clone)]
pub struct FloodInundationResult {
    pub inundated_cells: Vec<(usize, usize)>,
    pub max_cells_reached: bool,
}

/// Perform flood fill using Zig backend
pub fn zig_flood_fill_inundation(
    start: (usize, usize),
    world_size: (u32, u32),
    flood_level: f32,
    elevation_data: &[f32],
) -> FloodInundationResult {
    let (width, height) = world_size;
    let max_cells = (width * height / 4) as usize; // Reasonable limit
    let mut inundated_x = vec![0usize; max_cells];
    let mut inundated_y = vec![0usize; max_cells];
    let mut cell_count = 0;

    let success = unsafe {
        manifest_flood_fill_inundation(
            start.0,
            start.1,
            width as usize,
            height as usize,
            flood_level,
            elevation_data.as_ptr(),
            inundated_x.as_mut_ptr(),
            inundated_y.as_mut_ptr(),
            max_cells,
            &mut cell_count,
        )
    };

    if success && cell_count > 0 {
        inundated_x.truncate(cell_count);
        inundated_y.truncate(cell_count);
        let inundated_cells = inundated_x
            .into_iter()
            .zip(inundated_y.into_iter())
            .collect();

        FloodInundationResult {
            inundated_cells,
            max_cells_reached: cell_count >= max_cells,
        }
    } else {
        FloodInundationResult {
            inundated_cells: Vec::new(),
            max_cells_reached: false,
        }
    }
}

/// Batch flood risk assessment using Zig backend
pub fn zig_batch_flood_risk_assessment(
    positions: &[Vector2<f64>],
    elevation_data: &[f32],
    flow_data: &[f32],
    world_bounds: (f64, f64, f64, f64),
    world_size: (u32, u32),
) -> Vec<(f32, f32, f32)> {
    let count = positions.len();
    let positions_x: Vec<f64> = positions.iter().map(|p| p.x).collect();
    let positions_y: Vec<f64> = positions.iter().map(|p| p.y).collect();
    
    let mut risk_levels = vec![0.0f32; count];
    let mut return_periods = vec![0.0f32; count];
    let mut max_depths = vec![0.0f32; count];
    
    let bounds = [world_bounds.0, world_bounds.1, world_bounds.2, world_bounds.3];
    let size = [world_size.0, world_size.1];

    unsafe {
        manifest_flood_risk_assessment(
            positions_x.as_ptr(),
            positions_y.as_ptr(),
            count,
            elevation_data.as_ptr(),
            flow_data.as_ptr(),
            bounds.as_ptr(),
            size.as_ptr(),
            risk_levels.as_mut_ptr(),
            return_periods.as_mut_ptr(),
            max_depths.as_mut_ptr(),
        );
    }

    risk_levels
        .into_iter()
        .zip(return_periods.into_iter())
        .zip(max_depths.into_iter())
        .map(|((risk, period), depth)| (risk, period, depth))
        .collect()
}

/// Lake detection results from Zig backend
#[derive(Debug, Clone)]
pub struct ZigLakeDetectionResult {
    pub local_minima: Vec<(u32, u32, f32)>,
    pub basins: Vec<ZigBasin>,
}

#[derive(Debug, Clone)]
pub struct ZigBasin {
    pub id: u32,
    pub cells: Vec<usize>,
}

/// Find local minima using Zig backend
pub fn zig_find_local_minima(
    elevation_data: &[f32],
    world_size: (u32, u32),
) -> Vec<(u32, u32, f32)> {
    let (width, height) = world_size;
    let max_minima = 1000; // Reasonable limit
    let mut minima_x = vec![0u32; max_minima];
    let mut minima_y = vec![0u32; max_minima];
    let mut minima_elevation = vec![0.0f32; max_minima];
    let mut minima_count = 0;

    unsafe {
        manifest_lakes_find_local_minima(
            elevation_data.as_ptr(),
            width,
            height,
            minima_x.as_mut_ptr(),
            minima_y.as_mut_ptr(),
            minima_elevation.as_mut_ptr(),
            max_minima,
            &mut minima_count,
        );
    }

    if minima_count > 0 {
        minima_x.truncate(minima_count);
        minima_y.truncate(minima_count);
        minima_elevation.truncate(minima_count);
        
        minima_x
            .into_iter()
            .zip(minima_y.into_iter())
            .zip(minima_elevation.into_iter())
            .map(|((x, y), elev)| (x, y, elev))
            .collect()
    } else {
        Vec::new()
    }
}

/// Perform union-find basin detection using Zig backend
pub fn zig_union_find_basins(
    elevation_data: &[f32],
    world_size: (u32, u32),
    elevation_threshold: f32,
) -> Vec<ZigBasin> {
    let (width, height) = world_size;
    let total_cells = (width * height) as usize;
    let mut basin_cells = vec![0usize; total_cells];
    let mut basin_ids = vec![0u32; total_cells];
    let mut basin_count = 0;

    let success = unsafe {
        manifest_lakes_union_find_basins(
            elevation_data.as_ptr(),
            width,
            height,
            elevation_threshold,
            basin_cells.as_mut_ptr(),
            basin_ids.as_mut_ptr(),
            total_cells,
            &mut basin_count,
        )
    };

    if success && basin_count > 0 {
        // Group cells by basin ID
        let mut basins = std::collections::HashMap::new();
        for i in 0..total_cells {
            let basin_id = basin_ids[i];
            if basin_id > 0 {
                basins.entry(basin_id).or_insert_with(Vec::new).push(basin_cells[i]);
            }
        }

        basins
            .into_iter()
            .map(|(id, cells)| ZigBasin { id, cells })
            .collect()
    } else {
        Vec::new()
    }
}

/// Calculate lake volume using Zig backend
pub fn zig_calculate_lake_volume(
    basin_cells: &[usize],
    elevation_data: &[f32],
    max_elevation: f32,
    cell_area: f64,
) -> f64 {
    unsafe {
        manifest_lakes_calculate_volume(
            basin_cells.as_ptr(),
            basin_cells.len(),
            elevation_data.as_ptr(),
            max_elevation,
            cell_area,
        )
    }
}

/// River pathfinding result
#[derive(Debug, Clone)]
pub struct ZigRiverPath {
    pub path: Vec<(i32, i32)>,
    pub success: bool,
}

/// Perform A* pathfinding for rivers using Zig backend
pub fn zig_river_astar_pathfinding(
    start: (i32, i32),
    goal: (i32, i32),
    flow_data: &[f32],
    elevation_data: &[f32],
    world_bounds: (f64, f64, f64, f64),
    grid_resolution: f64,
) -> ZigRiverPath {
    let max_path_points = 10000; // Reasonable limit
    let mut path_x = vec![0i32; max_path_points];
    let mut path_y = vec![0i32; max_path_points];
    let mut path_length = 0;
    let bounds = [world_bounds.0, world_bounds.1, world_bounds.2, world_bounds.3];

    let success = unsafe {
        manifest_rivers_astar_pathfinding(
            start.0,
            start.1,
            goal.0,
            goal.1,
            flow_data.as_ptr(),
            elevation_data.as_ptr(),
            bounds.as_ptr(),
            grid_resolution,
            path_x.as_mut_ptr(),
            path_y.as_mut_ptr(),
            max_path_points,
            &mut path_length,
        )
    };

    if success && path_length > 0 {
        path_x.truncate(path_length);
        path_y.truncate(path_length);
        let path = path_x.into_iter().zip(path_y.into_iter()).collect();
        ZigRiverPath { path, success: true }
    } else {
        ZigRiverPath {
            path: Vec::new(),
            success: false,
        }
    }
}

/// Find river sources using Zig backend
pub fn zig_find_river_sources(
    flow_accumulation: &[f32],
    world_size: (u32, u32),
    threshold: f64,
    sample_step: usize,
) -> Vec<(usize, usize, i32)> {
    let (width, height) = world_size;
    let max_sources = 1000; // Reasonable limit
    let mut sources_x = vec![0usize; max_sources];
    let mut sources_y = vec![0usize; max_sources];
    let mut source_priorities = vec![0i32; max_sources];
    let mut source_count = 0;

    unsafe {
        manifest_rivers_find_sources(
            flow_accumulation.as_ptr(),
            width as usize,
            height as usize,
            threshold,
            sample_step,
            sources_x.as_mut_ptr(),
            sources_y.as_mut_ptr(),
            source_priorities.as_mut_ptr(),
            max_sources,
            &mut source_count,
        );
    }

    if source_count > 0 {
        sources_x.truncate(source_count);
        sources_y.truncate(source_count);
        source_priorities.truncate(source_count);
        
        sources_x
            .into_iter()
            .zip(sources_y.into_iter())
            .zip(source_priorities.into_iter())
            .map(|((x, y), priority)| (x, y, priority))
            .collect()
    } else {
        Vec::new()
    }
}

/// Spatial tree wrapper for Zig KdTree
pub struct ZigSpatialTree {
    ptr: *mut SpatialTreeC,
}

impl ZigSpatialTree {
    /// Create new spatial tree
    pub fn new() -> Option<Self> {
        let ptr = unsafe { manifest_spatial_kdtree_create() };
        if ptr.is_null() {
            None
        } else {
            Some(ZigSpatialTree { ptr })
        }
    }

    /// Add point to spatial tree
    pub fn add_point(&mut self, x: f64, y: f64, id: usize) {
        unsafe {
            manifest_spatial_kdtree_add_point(self.ptr, x, y, id);
        }
    }

    /// Find k nearest neighbors
    pub fn nearest(&self, x: f64, y: f64, k: usize) -> Vec<(usize, f64)> {
        let mut result_ids = vec![0usize; k];
        let mut result_distances = vec![0.0f64; k];

        let found_count = unsafe {
            manifest_spatial_kdtree_nearest(
                self.ptr,
                x,
                y,
                k,
                result_ids.as_mut_ptr(),
                result_distances.as_mut_ptr(),
            )
        };

        if found_count > 0 {
            result_ids.truncate(found_count);
            result_distances.truncate(found_count);
            result_ids
                .into_iter()
                .zip(result_distances.into_iter())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Find all points within radius
    pub fn within_radius(&self, x: f64, y: f64, radius: f64) -> Vec<usize> {
        let max_results = 1000; // Reasonable limit
        let mut result_ids = vec![0usize; max_results];

        let found_count = unsafe {
            manifest_spatial_kdtree_within_radius(
                self.ptr,
                x,
                y,
                radius,
                result_ids.as_mut_ptr(),
                max_results,
            )
        };

        if found_count > 0 {
            result_ids.truncate(found_count);
            result_ids
        } else {
            Vec::new()
        }
    }
}

impl Drop for ZigSpatialTree {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe {
                manifest_spatial_kdtree_destroy(self.ptr);
            }
        }
    }
}

unsafe impl Send for ZigSpatialTree {}
unsafe impl Sync for ZigSpatialTree {}

/// Wetland candidate evaluation results
#[derive(Debug, Clone)]
pub struct ZigWetlandEvaluation {
    pub suitability_scores: Vec<f32>,
    pub wetland_types: Vec<u8>,
}

/// Evaluate wetland candidates using Zig backend
pub fn zig_evaluate_wetland_candidates(
    positions: &[Vector2<f64>],
    elevations: &[f32],
    flow_values: &[f32],
    water_bodies_tree: &mut ZigSpatialTree,
) -> ZigWetlandEvaluation {
    let candidate_count = positions.len();
    let positions_x: Vec<f64> = positions.iter().map(|p| p.x).collect();
    let positions_y: Vec<f64> = positions.iter().map(|p| p.y).collect();
    
    let mut suitability_scores = vec![0.0f32; candidate_count];
    let mut wetland_types = vec![0u8; candidate_count];

    unsafe {
        manifest_wetlands_evaluate_candidates(
            positions_x.as_ptr(),
            positions_y.as_ptr(),
            elevations.as_ptr(),
            flow_values.as_ptr(),
            candidate_count,
            water_bodies_tree.ptr,
            suitability_scores.as_mut_ptr(),
            wetland_types.as_mut_ptr(),
        );
    }

    ZigWetlandEvaluation {
        suitability_scores,
        wetland_types,
    }
}

/// Check if point is in polygon using Zig backend
pub fn zig_point_in_polygon(
    point: Vector2<f64>,
    polygon: &[Vector2<f64>],
) -> bool {
    if polygon.len() < 3 {
        return false;
    }

    let polygon_x: Vec<f64> = polygon.iter().map(|p| p.x).collect();
    let polygon_y: Vec<f64> = polygon.iter().map(|p| p.y).collect();

    unsafe {
        manifest_geometry_point_in_polygon(
            point.x,
            point.y,
            polygon_x.as_ptr(),
            polygon_y.as_ptr(),
            polygon.len(),
        )
    }
}

/// Calculate polygon area using Zig backend
pub fn zig_polygon_area(polygon: &[Vector2<f64>]) -> f64 {
    if polygon.len() < 3 {
        return 0.0;
    }

    let polygon_x: Vec<f64> = polygon.iter().map(|p| p.x).collect();
    let polygon_y: Vec<f64> = polygon.iter().map(|p| p.y).collect();

    unsafe {
        manifest_geometry_polygon_area(
            polygon_x.as_ptr(),
            polygon_y.as_ptr(),
            polygon.len(),
        )
    }
}

/// Calculate convex hull using Zig backend
pub fn zig_convex_hull(points: &[Vector2<f64>]) -> Vec<Vector2<f64>> {
    if points.len() < 3 {
        return points.to_vec();
    }

    let points_x: Vec<f64> = points.iter().map(|p| p.x).collect();
    let points_y: Vec<f64> = points.iter().map(|p| p.y).collect();
    
    let max_hull_points = points.len();
    let mut hull_x = vec![0.0f64; max_hull_points];
    let mut hull_y = vec![0.0f64; max_hull_points];
    let mut hull_size = 0;

    unsafe {
        manifest_geometry_convex_hull(
            points_x.as_ptr(),
            points_y.as_ptr(),
            points.len(),
            hull_x.as_mut_ptr(),
            hull_y.as_mut_ptr(),
            max_hull_points,
            &mut hull_size,
        );
    }

    if hull_size > 0 {
        hull_x.truncate(hull_size);
        hull_y.truncate(hull_size);
        hull_x
            .into_iter()
            .zip(hull_y.into_iter())
            .map(|(x, y)| Vector2::new(x, y))
            .collect()
    } else {
        Vec::new()
    }
}

/// Gradient analysis results
#[derive(Debug, Clone)]
pub struct ZigGradientAnalysis {
    pub gradients_x: Vec<f32>,
    pub gradients_y: Vec<f32>,
    pub gradients_magnitude: Vec<f32>,
}

/// Perform gradient analysis using Zig backend
pub fn zig_elevation_gradient_analysis(
    elevation_data: &[f32],
    world_size: (u32, u32),
    cell_size: f64,
) -> ZigGradientAnalysis {
    let (width, height) = world_size;
    let total_cells = (width * height) as usize;
    let mut gradients_x = vec![0.0f32; total_cells];
    let mut gradients_y = vec![0.0f32; total_cells];
    let mut gradients_magnitude = vec![0.0f32; total_cells];

    unsafe {
        manifest_elevation_gradient_analysis(
            elevation_data.as_ptr(),
            width as usize,
            height as usize,
            cell_size,
            gradients_x.as_mut_ptr(),
            gradients_y.as_mut_ptr(),
            gradients_magnitude.as_mut_ptr(),
        );
    }

    ZigGradientAnalysis {
        gradients_x,
        gradients_y,
        gradients_magnitude,
    }
}

/// Local statistics results
#[derive(Debug, Clone)]
pub struct ZigLocalStatistics {
    pub mean_values: Vec<f32>,
    pub std_values: Vec<f32>,
    pub min_values: Vec<f32>,
    pub max_values: Vec<f32>,
}

/// Calculate local elevation statistics using Zig backend
pub fn zig_elevation_local_statistics(
    elevation_data: &[f32],
    positions: &[(usize, usize)],
    world_size: (u32, u32),
    window_size: i32,
) -> ZigLocalStatistics {
    let position_count = positions.len();
    let positions_x: Vec<usize> = positions.iter().map(|p| p.0).collect();
    let positions_y: Vec<usize> = positions.iter().map(|p| p.1).collect();
    
    let mut mean_values = vec![0.0f32; position_count];
    let mut std_values = vec![0.0f32; position_count];
    let mut min_values = vec![0.0f32; position_count];
    let mut max_values = vec![0.0f32; position_count];

    unsafe {
        manifest_elevation_local_statistics(
            elevation_data.as_ptr(),
            positions_x.as_ptr(),
            positions_y.as_ptr(),
            position_count,
            world_size.0 as usize,
            world_size.1 as usize,
            window_size,
            mean_values.as_mut_ptr(),
            std_values.as_mut_ptr(),
            min_values.as_mut_ptr(),
            max_values.as_mut_ptr(),
        );
    }

    ZigLocalStatistics {
        mean_values,
        std_values,
        min_values,
        max_values,
    }
}
