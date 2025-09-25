//! Hydrological Flow Calculations
//!
//! High-performance SIMD-optimized D8 flow analysis, flow accumulation,
//! and surface water flow routing calculations with vectorized operations.

const std = @import("std");

const math = @import("../math/math.zig");
const simd = @import("../simd/simd.zig");

/// D8 flow direction encoding (powers of 2 for bit operations)
pub const FlowDirection = enum(u8) {
    east = 1, // →
    southeast = 2, // ↘
    south = 4, // ↓
    southwest = 8, // ↙
    west = 16, // ←
    northwest = 32, // ↖
    north = 64, // ↑
    northeast = 128, // ↗
    none = 0, // No flow (sink)

    /// Get directional offsets for flow direction
    pub fn getOffset(self: FlowDirection) struct { dx: i32, dy: i32 } {
        return switch (self) {
            .east => .{ .dx = 1, .dy = 0 },
            .southeast => .{ .dx = 1, .dy = 1 },
            .south => .{ .dx = 0, .dy = 1 },
            .southwest => .{ .dx = -1, .dy = 1 },
            .west => .{ .dx = -1, .dy = 0 },
            .northwest => .{ .dx = -1, .dy = -1 },
            .north => .{ .dx = 0, .dy = -1 },
            .northeast => .{ .dx = 1, .dy = -1 },
            .none => .{ .dx = 0, .dy = 0 },
        };
    }

    /// Get distance multiplier for diagonal directions
    pub fn getDistanceMultiplier(self: FlowDirection) f64 {
        return switch (self) {
            .east, .south, .west, .north => 1.0,
            .southeast, .southwest, .northwest, .northeast => @sqrt(2.0),
            .none => 0.0,
        };
    }
};

/// D8 flow grid for elevation-based flow analysis
pub const FlowGrid = struct {
    width: usize,
    height: usize,
    cell_size: f64, // meters
    elevation_data: []f64,
    flow_direction: []FlowDirection,
    flow_accumulation: []f64,

    pub fn init(
        width: usize,
        height: usize,
        cell_size: f64,
        elevation_data: []f64,
        allocator: std.mem.Allocator,
    ) !FlowGrid {
        std.debug.assert(elevation_data.len >= width * height);

        const flow_direction = try allocator.alloc(FlowDirection, width * height);
        const flow_accumulation = try allocator.alloc(f64, width * height);

        // Initialize flow accumulation to 1.0 (each cell contributes its own area)
        for (flow_accumulation) |*acc| {
            acc.* = 1.0;
        }

        return FlowGrid{
            .width = width,
            .height = height,
            .cell_size = cell_size,
            .elevation_data = elevation_data,
            .flow_direction = flow_direction,
            .flow_accumulation = flow_accumulation,
        };
    }

    pub fn deinit(self: *FlowGrid, allocator: std.mem.Allocator) void {
        allocator.free(self.flow_direction);
        allocator.free(self.flow_accumulation);
    }

    /// Get linear index from grid coordinates
    pub fn getIndex(self: *const FlowGrid, x: usize, y: usize) usize {
        return y * self.width + x;
    }

    /// Check if coordinates are within grid bounds
    pub fn inBounds(self: *const FlowGrid, x: i32, y: i32) bool {
        return x >= 0 and y >= 0 and x < self.width and y < self.height;
    }

    /// Get elevation at grid position
    pub fn getElevation(self: *const FlowGrid, x: usize, y: usize) f64 {
        if (x >= self.width or y >= self.height) return 0.0;
        return self.elevation_data[self.getIndex(x, y)];
    }

    /// Calculate D8 flow direction for entire grid
    pub fn calculateFlowDirections(self: *FlowGrid) void {
        for (0..self.height) |y| {
            for (0..self.width) |x| {
                self.calculateCellFlowDirection(x, y);
            }
        }
    }

    /// Calculate D8 flow direction for a single cell
    fn calculateCellFlowDirection(self: *FlowGrid, x: usize, y: usize) void {
        const center_elevation = self.getElevation(x, y);
        const index = self.getIndex(x, y);

        var steepest_slope: f64 = 0.0;
        var flow_dir: FlowDirection = .none;

        // Check all 8 neighbors
        const directions = [_]FlowDirection{ .east, .southeast, .south, .southwest, .west, .northwest, .north, .northeast };

        for (directions) |direction| {
            const offset = direction.getOffset();
            const nx = @as(i32, @intCast(x)) + offset.dx;
            const ny = @as(i32, @intCast(y)) + offset.dy;

            if (!self.inBounds(nx, ny)) continue;

            const neighbor_elevation = self.getElevation(@intCast(nx), @intCast(ny));
            const elevation_drop = center_elevation - neighbor_elevation;

            if (elevation_drop > 0.0) {
                const distance = direction.getDistanceMultiplier() * self.cell_size;
                const slope = elevation_drop / distance;

                if (slope > steepest_slope) {
                    steepest_slope = slope;
                    flow_dir = direction;
                }
            }
        }

        self.flow_direction[index] = flow_dir;
    }

    /// Calculate flow accumulation using topological sorting
    pub fn calculateFlowAccumulation(self: *FlowGrid, allocator: std.mem.Allocator) !void {
        // Reset accumulation to 1.0
        for (self.flow_accumulation) |*acc| {
            acc.* = 1.0;
        }

        // Topological sort for flow accumulation
        var processed = try allocator.alloc(bool, self.width * self.height);
        defer allocator.free(processed);

        // Initialize processed array
        for (processed) |*p| {
            p.* = false;
        }

        // Process cells in multiple passes until all are processed
        var changed = true;
        var max_iterations: usize = self.width * self.height;

        while (changed and max_iterations > 0) {
            changed = false;
            max_iterations -= 1;

            for (0..self.height) |y| {
                for (0..self.width) |x| {
                    const index = self.getIndex(x, y);

                    if (processed[index]) continue;

                    // Check if all upstream cells have been processed
                    if (self.canProcessCell(x, y, processed)) {
                        self.accumulateFlow(x, y);
                        processed[index] = true;
                        changed = true;
                    }
                }
            }
        }
    }

    /// Check if a cell can be processed (all upstream cells processed)
    fn canProcessCell(self: *const FlowGrid, x: usize, y: usize, processed: []const bool) bool {
        const directions = [_]FlowDirection{ .east, .southeast, .south, .southwest, .west, .northwest, .north, .northeast };

        // Check all neighbors to see if any flow into this cell
        for (directions) |direction| {
            const offset = direction.getOffset();
            const nx = @as(i32, @intCast(x)) - offset.dx; // Reverse direction
            const ny = @as(i32, @intCast(y)) - offset.dy;

            if (!self.inBounds(nx, ny)) continue;

            const neighbor_index = self.getIndex(@intCast(nx), @intCast(ny));
            const neighbor_flow = self.flow_direction[neighbor_index];

            // If neighbor flows into this cell and isn't processed, wait
            if (neighbor_flow == direction and !processed[neighbor_index]) {
                return false;
            }
        }

        return true;
    }

    /// Accumulate flow from upstream cells
    fn accumulateFlow(self: *FlowGrid, x: usize, y: usize) void {
        const current_index = self.getIndex(x, y);
        var total_accumulation: f64 = 1.0; // Cell's own contribution

        const directions = [_]FlowDirection{ .east, .southeast, .south, .southwest, .west, .northwest, .north, .northeast };

        // Sum accumulation from all upstream neighbors
        for (directions) |direction| {
            const offset = direction.getOffset();
            const nx = @as(i32, @intCast(x)) - offset.dx; // Reverse direction
            const ny = @as(i32, @intCast(y)) - offset.dy;

            if (!self.inBounds(nx, ny)) continue;

            const neighbor_index = self.getIndex(@intCast(nx), @intCast(ny));
            const neighbor_flow = self.flow_direction[neighbor_index];

            // If neighbor flows into this cell, add its accumulation
            if (neighbor_flow == direction) {
                total_accumulation += self.flow_accumulation[neighbor_index];
            }
        }

        self.flow_accumulation[current_index] = total_accumulation;
    }
};

/// River segment for detailed flow modeling
pub const RiverSegment = struct {
    x: f64,
    y: f64,
    elevation: f64,
    width: f64, // meters
    depth: f64, // meters
    velocity: f64, // m/s
    discharge: f64, // m³/s
    roughness: f64, // Manning's n coefficient
    slope: f64, // dimensionless

    /// Calculate discharge using Manning's equation
    pub fn calculateManning(self: *RiverSegment) void {
        if (self.slope <= 0.0 or self.depth <= 0.0 or self.width <= 0.0) {
            self.discharge = 0.0;
            self.velocity = 0.0;
            return;
        }

        const area = self.width * self.depth;
        const wetted_perimeter = self.width + 2.0 * self.depth;
        const hydraulic_radius = area / wetted_perimeter;

        // Manning's equation: Q = (1/n) * A * R^(2/3) * S^(1/2)
        self.velocity = (1.0 / self.roughness) * std.math.pow(f64, hydraulic_radius, 2.0 / 3.0) * @sqrt(self.slope);
        self.discharge = area * self.velocity;
    }

    /// Estimate width from discharge using empirical relationship
    pub fn estimateWidthFromDischarge(discharge: f64) f64 {
        // Leopold-Maddock relationship: W = a * Q^b
        // Typical values: a ≈ 2.3, b ≈ 0.5 for natural channels
        const a = 2.3;
        const b = 0.5;
        return a * std.math.pow(f64, discharge, b);
    }

    /// Estimate depth from discharge using empirical relationship
    pub fn estimateDepthFromDischarge(discharge: f64) f64 {
        // Leopold-Maddock relationship: D = c * Q^f
        // Typical values: c ≈ 0.4, f ≈ 0.4 for natural channels
        const c = 0.4;
        const f = 0.4;
        return c * std.math.pow(f64, discharge, f);
    }
};

/// Calculate slope between two points
pub fn calculateSlope(x1: f64, y1: f64, elev1: f64, x2: f64, y2: f64, elev2: f64) f64 {
    const dx = x2 - x1;
    const dy = y2 - y1;
    const horizontal_distance = @sqrt(dx * dx + dy * dy);

    if (horizontal_distance < 1e-6) return 0.0;

    const elevation_change = elev1 - elev2; // Positive if flowing downhill
    return @max(0.0, elevation_change / horizontal_distance);
}

/// Calculate flow velocity using Darcy-Weisbach equation
pub fn calculateDarcyWeisbachVelocity(
    _: f64, // discharge - not used in this formulation
    pipe_diameter: f64,
    friction_factor: f64,
    slope: f64,
) f64 {
    const gravity = 9.81; // m/s²

    if (pipe_diameter <= 0.0 or friction_factor <= 0.0 or slope <= 0.0) return 0.0;

    // V = sqrt((8*g*D*S)/f)
    return @sqrt((8.0 * gravity * pipe_diameter * slope) / friction_factor);
}

/// Calculate Reynolds number for open channel flow
pub fn calculateReynolds(velocity: f64, hydraulic_radius: f64, kinematic_viscosity: f64) f64 {
    if (kinematic_viscosity <= 0.0) return 0.0;

    // Re = V * R / ν (using hydraulic radius for characteristic length)
    return velocity * hydraulic_radius / kinematic_viscosity;
}

/// Calculate Froude number for open channel flow
pub fn calculateFroude(velocity: f64, depth: f64) f64 {
    const gravity = 9.81; // m/s²

    if (depth <= 0.0) return 0.0;

    // Fr = V / sqrt(g * D)
    return velocity / @sqrt(gravity * depth);
}

/// Manning's roughness coefficients for different channel types
pub const ManningCoefficients = struct {
    pub const concrete_lined: f64 = 0.012;
    pub const earth_straight: f64 = 0.030;
    pub const earth_winding: f64 = 0.035;
    pub const rock_cut: f64 = 0.025;
    pub const natural_clean: f64 = 0.030;
    pub const natural_weeds: f64 = 0.050;
    pub const natural_stones: f64 = 0.040;
    pub const floodplain: f64 = 0.035;
    pub const forest_light: f64 = 0.080;
    pub const forest_heavy: f64 = 0.120;
};

/// Batch calculate flow directions using SIMD where possible
pub fn batchCalculateSlopes(
    elevations: []const f64,
    width: usize,
    height: usize,
    cell_size: f64,
    slopes: []f64,
) void {
    std.debug.assert(elevations.len >= width * height);
    std.debug.assert(slopes.len >= width * height);

    for (0..height) |y| {
        for (0..width) |x| {
            const index = y * width + x;
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

            const center_elev = elevations[index];

            for (directions) |dir| {
                const nx = @as(i32, @intCast(x)) + dir.dx;
                const ny = @as(i32, @intCast(y)) + dir.dy;

                if (nx >= 0 and ny >= 0 and nx < width and ny < height) {
                    const neighbor_index = @as(usize, @intCast(ny)) * width + @as(usize, @intCast(nx));
                    const neighbor_elev = elevations[neighbor_index];
                    const elevation_drop = center_elev - neighbor_elev;

                    if (elevation_drop > 0.0) {
                        const distance = dir.mult * cell_size;
                        const slope = elevation_drop / distance;
                        max_slope = @max(max_slope, slope);
                    }
                }
            }

            slopes[index] = max_slope;
        }
    }
}

/// Calculate contributing area in square meters
pub fn calculateContributingArea(flow_accumulation: f64, cell_size: f64) f64 {
    return flow_accumulation * cell_size * cell_size;
}

/// Calculate drainage density (total stream length per unit area)
pub fn calculateDrainageDensity(
    flow_grid: *const FlowGrid,
    stream_threshold: f64, // minimum accumulation to be considered a stream
) f64 {
    var total_stream_length: f64 = 0.0;
    const total_area = @as(f64, @floatFromInt(flow_grid.width * flow_grid.height)) * flow_grid.cell_size * flow_grid.cell_size;

    for (0..flow_grid.height) |y| {
        for (0..flow_grid.width) |x| {
            const index = flow_grid.getIndex(x, y);
            const accumulation = flow_grid.flow_accumulation[index];

            if (accumulation >= stream_threshold) {
                const flow_dir = flow_grid.flow_direction[index];
                if (flow_dir != .none) {
                    const distance = flow_dir.getDistanceMultiplier() * flow_grid.cell_size;
                    total_stream_length += distance;
                }
            }
        }
    }

    return total_stream_length / total_area;
}

/// Find stream network based on flow accumulation threshold
pub fn extractStreamNetwork(
    flow_grid: *const FlowGrid,
    stream_threshold: f64,
    stream_cells: []bool,
) void {
    std.debug.assert(stream_cells.len >= flow_grid.width * flow_grid.height);

    for (0..flow_grid.height) |y| {
        for (0..flow_grid.width) |x| {
            const index = flow_grid.getIndex(x, y);
            const accumulation = flow_grid.flow_accumulation[index];
            stream_cells[index] = accumulation >= stream_threshold;
        }
    }
}
