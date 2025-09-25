//! Hydrology Module - Comprehensive Water Flow Analysis
//!
//! This module provides high-performance SIMD-optimized hydrological calculations
//! including surface water flow, groundwater modeling, watershed analysis, and
//! hydraulic engineering computations for game world generation.
//!
//! ## Features
//! - D8 flow analysis and flow accumulation
//! - Watershed delineation and morphometric analysis
//! - Manning's equation and open channel flow
//! - Groundwater flow modeling and spring generation
//! - Advanced hydraulic calculations
//!
//! ## Usage
//! ```zig
//! const hydrology = @import("hydrology/mod.zig");
//!
//! // Create flow grid for surface water analysis
//! var flow_grid = try hydrology.flow.FlowGrid.init(width, height, cell_size, elevation_data, allocator);
//! defer flow_grid.deinit(allocator);
//!
//! // Calculate flow directions and accumulation
//! flow_grid.calculateFlowDirections();
//! try flow_grid.calculateFlowAccumulation(allocator);
//!
//! // Delineate watersheds
//! var delineator = try hydrology.watersheds.WatershedDelineator.init(&flow_grid, allocator);
//! defer delineator.deinit();
//! try delineator.delineateWatershed(outlet_x, outlet_y, watershed_id);
//!
//! // Hydraulic calculations
//! const results = hydrology.hydraulics.calculateManning(area, wetted_perimeter, slope, manning_n);
//!
//! // Groundwater modeling
//! var gw_grid = try hydrology.aquifers.GroundwaterGrid.init(width, height, cell_size, allocator);
//! defer gw_grid.deinit(allocator);
//! gw_grid.solveStreadyState(1000, 0.001);
//! ```

const std = @import("std");

const math = @import("../math/math.zig");
const simd = @import("../simd/simd.zig");
pub const aquifers = @import("aquifers.zig");
pub const flow = @import("flow.zig");
pub const hydraulics = @import("hydraulics.zig");
pub const watersheds = @import("watersheds.zig");

// Re-export all hydrology modules
// Import dependencies
/// Comprehensive hydrological analysis results
pub const HydrologicalAnalysis = struct {
    // Surface water analysis
    total_stream_length: f64, // Total stream network length (m)
    drainage_density: f64, // Stream length per unit area (km/km²)
    watershed_count: usize, // Number of delineated watersheds
    largest_watershed_area: f64, // Area of largest watershed (m²)

    // Flow statistics
    mean_flow_accumulation: f64, // Average flow accumulation
    max_flow_accumulation: f64, // Maximum flow accumulation
    stream_threshold: f64, // Accumulation threshold for streams

    // Hydraulic properties
    mean_velocity: f64, // Average stream velocity (m/s)
    mean_discharge: f64, // Average stream discharge (m³/s)
    peak_discharge: f64, // Peak discharge in network (m³/s)

    // Groundwater analysis
    spring_count: usize, // Number of generated springs
    total_spring_discharge: f64, // Total spring discharge (m³/s)
    mean_hydraulic_head: f64, // Average hydraulic head (m)
    groundwater_flow_magnitude: f64, // Average groundwater flow velocity (m/s)

    pub fn init() HydrologicalAnalysis {
        return HydrologicalAnalysis{
            .total_stream_length = 0.0,
            .drainage_density = 0.0,
            .watershed_count = 0,
            .largest_watershed_area = 0.0,
            .mean_flow_accumulation = 0.0,
            .max_flow_accumulation = 0.0,
            .stream_threshold = 100.0,
            .mean_velocity = 0.0,
            .mean_discharge = 0.0,
            .peak_discharge = 0.0,
            .spring_count = 0,
            .total_spring_discharge = 0.0,
            .mean_hydraulic_head = 0.0,
            .groundwater_flow_magnitude = 0.0,
        };
    }
};

/// Complete hydrological analysis workflow
pub const HydrologicalSystem = struct {
    flow_grid: flow.FlowGrid,
    watershed_delineator: watersheds.WatershedDelineator,
    groundwater_grid: aquifers.GroundwaterGrid,
    springs: std.ArrayList(aquifers.Spring),
    analysis_results: HydrologicalAnalysis,
    allocator: std.mem.Allocator,

    pub fn init(
        width: usize,
        height: usize,
        cell_size: f64,
        elevation_data: []f64,
        allocator: std.mem.Allocator,
    ) !HydrologicalSystem {
        var flow_grid = try flow.FlowGrid.init(width, height, cell_size, elevation_data, allocator);
        const watershed_delineator = try watersheds.WatershedDelineator.init(&flow_grid, allocator);
        const groundwater_grid = try aquifers.GroundwaterGrid.init(width, height, cell_size, allocator);
        const springs = std.ArrayList(aquifers.Spring).init(allocator);

        return HydrologicalSystem{
            .flow_grid = flow_grid,
            .watershed_delineator = watershed_delineator,
            .groundwater_grid = groundwater_grid,
            .springs = springs,
            .analysis_results = HydrologicalAnalysis.init(),
            .allocator = allocator,
        };
    }

    pub fn deinit(self: *HydrologicalSystem) void {
        self.flow_grid.deinit(self.allocator);
        self.watershed_delineator.deinit();
        self.groundwater_grid.deinit(self.allocator);
        self.springs.deinit();
    }

    /// Run complete hydrological analysis
    pub fn runCompleteAnalysis(
        self: *HydrologicalSystem,
        stream_threshold: f64,
        min_spring_discharge: f64,
    ) !void {
        // 1. Calculate surface water flow
        self.flow_grid.calculateFlowDirections();
        try self.flow_grid.calculateFlowAccumulation(self.allocator);

        // 2. Delineate major watersheds based on high accumulation points
        try self.delineateWatershedsFromPeaks(stream_threshold * 10.0);

        // 3. Initialize groundwater system based on surface elevations
        self.initializeGroundwaterFromSurface();

        // 4. Solve groundwater flow
        self.groundwater_grid.solveStreadyState(1000, 0.001);

        // 5. Generate springs
        try aquifers.generateSprings(
            &self.groundwater_grid,
            self.flow_grid.elevation_data,
            &self.springs,
            min_spring_discharge,
        );

        // 6. Calculate comprehensive analysis
        self.calculateAnalysisResults(stream_threshold);
    }

    /// Automatically delineate watersheds from accumulation peaks
    fn delineateWatershedsFromPeaks(self: *HydrologicalSystem, peak_threshold: f64) !void {
        var watershed_id: u32 = 1;

        for (0..self.flow_grid.height) |y| {
            for (0..self.flow_grid.width) |x| {
                const index = self.flow_grid.getIndex(x, y);
                const accumulation = self.flow_grid.flow_accumulation[index];

                // Check if this is a significant accumulation peak
                if (accumulation >= peak_threshold) {
                    // Check if it's a local maximum (pour point candidate)
                    if (self.isLocalAccumulationMaximum(x, y)) {
                        try self.watershed_delineator.delineateWatershed(x, y, watershed_id);
                        watershed_id += 1;
                    }
                }
            }
        }
    }

    /// Check if point is local accumulation maximum
    fn isLocalAccumulationMaximum(self: *const HydrologicalSystem, x: usize, y: usize) bool {
        const center_index = self.flow_grid.getIndex(x, y);
        const center_acc = self.flow_grid.flow_accumulation[center_index];

        const directions = [_]struct { dx: i32, dy: i32 }{
            .{ .dx = -1, .dy = -1 }, .{ .dx = 0, .dy = -1 }, .{ .dx = 1, .dy = -1 },
            .{ .dx = -1, .dy = 0 },  .{ .dx = 1, .dy = 0 },  .{ .dx = -1, .dy = 1 },
            .{ .dx = 0, .dy = 1 },   .{ .dx = 1, .dy = 1 },
        };

        for (directions) |dir| {
            const nx = @as(i32, @intCast(x)) + dir.dx;
            const ny = @as(i32, @intCast(y)) + dir.dy;

            if (self.flow_grid.inBounds(nx, ny)) {
                const neighbor_index = self.flow_grid.getIndex(@intCast(nx), @intCast(ny));
                const neighbor_acc = self.flow_grid.flow_accumulation[neighbor_index];

                if (neighbor_acc > center_acc) {
                    return false; // Not a maximum
                }
            }
        }

        return true;
    }

    /// Initialize groundwater heads based on surface topography
    fn initializeGroundwaterFromSurface(self: *HydrologicalSystem) void {
        for (0..self.groundwater_grid.height) |y| {
            for (0..self.groundwater_grid.width) |x| {
                const index = self.groundwater_grid.getIndex(x, y);
                const surface_elevation = self.flow_grid.elevation_data[index];

                // Set hydraulic head to surface elevation minus depth to water table
                const depth_to_water = 10.0; // Default 10m depth to water table
                self.groundwater_grid.cells[index].hydraulic_head = surface_elevation - depth_to_water;

                // Set aquifer properties based on elevation and accumulation
                const accumulation = self.flow_grid.flow_accumulation[index];

                if (accumulation > 1000.0) {
                    // High accumulation areas - more permeable alluvial deposits
                    self.groundwater_grid.cells[index].hydraulic_conductivity = 1e-4; // High K
                    self.groundwater_grid.cells[index].aquifer_type = .unconfined;
                } else if (surface_elevation > 2000.0) {
                    // High elevation areas - fractured rock
                    self.groundwater_grid.cells[index].hydraulic_conductivity = 1e-6; // Medium K
                    self.groundwater_grid.cells[index].aquifer_type = .fractured_rock;
                } else {
                    // Default sedimentary conditions
                    self.groundwater_grid.cells[index].hydraulic_conductivity = 1e-5; // Low K
                    self.groundwater_grid.cells[index].aquifer_type = .unconfined;
                }

                // Set recharge based on slope and accumulation
                const local_slope = self.calculateLocalSlope(x, y);
                const base_recharge = 1e-8; // 1mm/day base recharge
                self.groundwater_grid.cells[index].recharge_rate = base_recharge * (1.0 + accumulation / 10000.0) * (1.0 / (1.0 + local_slope * 100.0));
            }
        }
    }

    /// Calculate local slope at a grid cell
    fn calculateLocalSlope(self: *const HydrologicalSystem, x: usize, y: usize) f64 {
        const center_elev = self.flow_grid.getElevation(x, y);
        var max_slope: f64 = 0.0;

        const directions = [_]struct { dx: i32, dy: i32, mult: f64 }{
            .{ .dx = 1, .dy = 0, .mult = 1.0 },
            .{ .dx = 1, .dy = 1, .mult = @sqrt(2.0) },
            .{ .dx = 0, .dy = 1, .mult = 1.0 },
            .{ .dx = -1, .dy = 1, .mult = @sqrt(2.0) },
            .{ .dx = -1, .dy = 0, .mult = 1.0 },
            .{ .dx = -1, .dy = -1, .mult = @sqrt(2.0) },
            .{ .dx = 0, .dy = -1, .mult = 1.0 },
            .{ .dx = 1, .dy = -1, .mult = @sqrt(2.0) },
        };

        for (directions) |dir| {
            const nx = @as(i32, @intCast(x)) + dir.dx;
            const ny = @as(i32, @intCast(y)) + dir.dy;

            if (self.flow_grid.inBounds(nx, ny)) {
                const neighbor_elev = self.flow_grid.getElevation(@intCast(nx), @intCast(ny));
                const elevation_diff = @abs(center_elev - neighbor_elev);
                const distance = dir.mult * self.flow_grid.cell_size;
                const slope = elevation_diff / distance;
                max_slope = @max(max_slope, slope);
            }
        }

        return max_slope;
    }

    /// Calculate comprehensive analysis results
    fn calculateAnalysisResults(self: *HydrologicalSystem, stream_threshold: f64) void {
        self.analysis_results.stream_threshold = stream_threshold;

        // Surface water statistics
        var total_accumulation: f64 = 0.0;
        var stream_length: f64 = 0.0;
        var max_accumulation: f64 = 0.0;

        for (0..self.flow_grid.height) |y| {
            for (0..self.flow_grid.width) |x| {
                const index = self.flow_grid.getIndex(x, y);
                const accumulation = self.flow_grid.flow_accumulation[index];

                total_accumulation += accumulation;
                max_accumulation = @max(max_accumulation, accumulation);

                if (accumulation >= stream_threshold) {
                    const flow_dir = self.flow_grid.flow_direction[index];
                    if (flow_dir != .none) {
                        stream_length += flow_dir.getDistanceMultiplier() * self.flow_grid.cell_size;
                    }
                }
            }
        }

        const total_cells = @as(f64, @floatFromInt(self.flow_grid.width * self.flow_grid.height));
        const total_area = total_cells * self.flow_grid.cell_size * self.flow_grid.cell_size;

        self.analysis_results.total_stream_length = stream_length;
        self.analysis_results.drainage_density = stream_length / (total_area / 1_000_000.0); // km/km²
        self.analysis_results.mean_flow_accumulation = total_accumulation / total_cells;
        self.analysis_results.max_flow_accumulation = max_accumulation;

        // Watershed statistics
        self.analysis_results.watershed_count = self.watershed_delineator.watersheds.items.len;

        var largest_area: f64 = 0.0;
        for (self.watershed_delineator.watersheds.items) |watershed| {
            largest_area = @max(largest_area, watershed.area);
        }
        self.analysis_results.largest_watershed_area = largest_area;

        // Groundwater statistics
        var total_head: f64 = 0.0;
        var total_flow_magnitude: f64 = 0.0;

        for (0..self.groundwater_grid.height) |y| {
            for (0..self.groundwater_grid.width) |x| {
                const index = self.groundwater_grid.getIndex(x, y);
                const cell = &self.groundwater_grid.cells[index];
                const flow_vector = self.groundwater_grid.flow_vectors[index];

                total_head += cell.hydraulic_head;
                total_flow_magnitude += flow_vector.magnitude;
            }
        }

        self.analysis_results.mean_hydraulic_head = total_head / total_cells;
        self.analysis_results.groundwater_flow_magnitude = total_flow_magnitude / total_cells;

        // Spring statistics
        self.analysis_results.spring_count = self.springs.items.len;

        var total_spring_discharge: f64 = 0.0;
        for (self.springs.items) |spring| {
            total_spring_discharge += spring.discharge;
        }
        self.analysis_results.total_spring_discharge = total_spring_discharge;

        // Hydraulic calculations for representative streams
        var total_velocity: f64 = 0.0;
        var total_discharge: f64 = 0.0;
        var peak_discharge: f64 = 0.0;
        var stream_count: usize = 0;

        for (0..self.flow_grid.height) |y| {
            for (0..self.flow_grid.width) |x| {
                const index = self.flow_grid.getIndex(x, y);
                const accumulation = self.flow_grid.flow_accumulation[index];

                if (accumulation >= stream_threshold) {
                    const contributing_area = flow.calculateContributingArea(accumulation, self.flow_grid.cell_size);
                    const estimated_discharge = contributing_area * 1e-6; // Rough estimate: 1mm/day runoff

                    // Estimate channel dimensions
                    const width = flow.RiverSegment.estimateWidthFromDischarge(estimated_discharge);
                    const depth = flow.RiverSegment.estimateDepthFromDischarge(estimated_discharge);
                    const area = width * depth;
                    const wetted_perimeter = width + 2.0 * depth;

                    // Calculate slope
                    const slope = self.calculateLocalSlope(x, y);

                    if (area > 0.0 and slope > 0.0) {
                        const manning_n = hydraulics.ManningCoefficients.natural_clean;
                        const results = hydraulics.calculateManning(area, wetted_perimeter, slope, manning_n);

                        total_velocity += results.velocity;
                        total_discharge += results.discharge;
                        peak_discharge = @max(peak_discharge, results.discharge);
                        stream_count += 1;
                    }
                }
            }
        }

        if (stream_count > 0) {
            self.analysis_results.mean_velocity = total_velocity / @as(f64, @floatFromInt(stream_count));
            self.analysis_results.mean_discharge = total_discharge / @as(f64, @floatFromInt(stream_count));
        }
        self.analysis_results.peak_discharge = peak_discharge;
    }
};

/// Utility function to create a simple hydrological analysis
pub fn createSimpleHydrologicalAnalysis(
    width: usize,
    height: usize,
    cell_size: f64,
    elevation_data: []f64,
    allocator: std.mem.Allocator,
) !HydrologicalSystem {
    var system = try HydrologicalSystem.init(width, height, cell_size, elevation_data, allocator);

    // Run analysis with default parameters
    try system.runCompleteAnalysis(100.0, 0.001); // 100 cells for streams, 0.001 m³/s min spring discharge

    return system;
}

/// Batch process multiple elevation grids
pub fn batchProcessElevationGrids(
    grids: []const []f64,
    width: usize,
    height: usize,
    cell_size: f64,
    results: []HydrologicalAnalysis,
    allocator: std.mem.Allocator,
) !void {
    std.debug.assert(results.len >= grids.len);

    for (grids, 0..) |elevation_data, i| {
        var system = try createSimpleHydrologicalAnalysis(width, height, cell_size, elevation_data, allocator);
        defer system.deinit();

        results[i] = system.analysis_results;
    }
}

/// Export functions for C FFI
export fn hydrologyCreateFlowGrid(
    width: usize,
    height: usize,
    cell_size: f64,
    elevation_data: [*]f64,
) ?*flow.FlowGrid {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    const allocator = gpa.allocator();

    const elevation_slice = elevation_data[0 .. width * height];
    const grid = flow.FlowGrid.init(width, height, cell_size, elevation_slice, allocator) catch return null;

    const grid_ptr = allocator.create(flow.FlowGrid) catch return null;
    grid_ptr.* = grid;
    return grid_ptr;
}

export fn hydrologyCalculateFlowDirections(grid_ptr: *flow.FlowGrid) void {
    grid_ptr.calculateFlowDirections();
}

export fn hydrologyCalculateFlowAccumulation(grid_ptr: *flow.FlowGrid) bool {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    const allocator = gpa.allocator();

    grid_ptr.calculateFlowAccumulation(allocator) catch return false;
    return true;
}

export fn hydrologyDestroyFlowGrid(grid_ptr: *flow.FlowGrid) void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    const allocator = gpa.allocator();

    grid_ptr.deinit(allocator);
    allocator.destroy(grid_ptr);
}
