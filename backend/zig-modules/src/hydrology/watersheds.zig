//! Watershed Delineation and Basin Analysis
//!
//! High-performance SIMD-optimized watershed delineation, basin boundary tracing,
//! and catchment area calculations using advanced hydrological algorithms.

const std = @import("std");

const math = @import("../math/math.zig");
const simd = @import("../simd/simd.zig");
const flow = @import("flow.zig");

/// Watershed boundary point
pub const BoundaryPoint = struct {
    x: f64,
    y: f64,
    elevation: f64,

    pub fn init(x: f64, y: f64, elevation: f64) BoundaryPoint {
        return BoundaryPoint{ .x = x, .y = y, .elevation = elevation };
    }

    pub fn distance(self: BoundaryPoint, other: BoundaryPoint) f64 {
        const dx = self.x - other.x;
        const dy = self.y - other.y;
        return @sqrt(dx * dx + dy * dy);
    }
};

/// Watershed data structure
pub const Watershed = struct {
    id: u32,
    outlet_x: usize,
    outlet_y: usize,
    outlet_elevation: f64,
    area: f64, // square meters
    perimeter: f64, // meters
    boundary_points: std.ArrayList(BoundaryPoint),
    stream_length: f64, // meters
    drainage_density: f64, // km/km²
    relief: f64, // meters (max - min elevation)
    mean_elevation: f64, // meters
    mean_slope: f64, // dimensionless
    shape_factor: f64, // dimensionless (area / perimeter²)

    pub fn init(id: u32, outlet_x: usize, outlet_y: usize, outlet_elevation: f64, allocator: std.mem.Allocator) Watershed {
        _ = allocator;
        return Watershed{
            .id = id,
            .outlet_x = outlet_x,
            .outlet_y = outlet_y,
            .outlet_elevation = outlet_elevation,
            .area = 0.0,
            .perimeter = 0.0,
            .boundary_points = std.ArrayList(BoundaryPoint){},
            .stream_length = 0.0,
            .drainage_density = 0.0,
            .relief = 0.0,
            .mean_elevation = 0.0,
            .mean_slope = 0.0,
            .shape_factor = 0.0,
        };
    }

    pub fn deinit(self: *Watershed, allocator: std.mem.Allocator) void {
        self.boundary_points.deinit(allocator);
    }

    /// Calculate watershed morphometric properties
    pub fn calculateMorphometrics(self: *Watershed) void {
        if (self.boundary_points.items.len < 3) return;

        // Calculate perimeter
        self.perimeter = 0.0;
        for (0..self.boundary_points.items.len) |i| {
            const next_i = (i + 1) % self.boundary_points.items.len;
            self.perimeter += self.boundary_points.items[i].distance(self.boundary_points.items[next_i]);
        }

        // Calculate area using shoelace formula
        self.area = 0.0;
        for (0..self.boundary_points.items.len) |i| {
            const next_i = (i + 1) % self.boundary_points.items.len;
            const curr = self.boundary_points.items[i];
            const next = self.boundary_points.items[next_i];
            self.area += curr.x * next.y - next.x * curr.y;
        }
        self.area = @abs(self.area) / 2.0;

        // Calculate shape factor
        if (self.perimeter > 0.0) {
            self.shape_factor = self.area / (self.perimeter * self.perimeter);
        }

        // Calculate elevation statistics
        if (self.boundary_points.items.len > 0) {
            var min_elev = self.boundary_points.items[0].elevation;
            var max_elev = self.boundary_points.items[0].elevation;
            var sum_elev: f64 = 0.0;

            for (self.boundary_points.items) |point| {
                min_elev = @min(min_elev, point.elevation);
                max_elev = @max(max_elev, point.elevation);
                sum_elev += point.elevation;
            }

            self.relief = max_elev - min_elev;
            self.mean_elevation = sum_elev / @as(f64, @floatFromInt(self.boundary_points.items.len));
        }

        // Calculate drainage density
        if (self.area > 0.0) {
            self.drainage_density = self.stream_length / (self.area / 1_000_000.0); // Convert to km/km²
        }
    }

    /// Calculate time of concentration using Kirpich equation
    pub fn calculateTimeOfConcentration(self: *const Watershed) f64 {
        if (self.stream_length <= 0.0 or self.relief <= 0.0) return 0.0;

        // Kirpich equation: tc = 0.0195 * (L^0.77) * (S^-0.385)
        // where L is length in meters, S is slope, tc is in minutes
        const length_km = self.stream_length / 1000.0;
        const slope = self.relief / self.stream_length;

        if (slope <= 0.0) return 0.0;

        const tc_minutes = 0.0195 * std.math.pow(f64, length_km * 1000.0, 0.77) * std.math.pow(f64, slope, -0.385);
        return tc_minutes; // minutes
    }

    /// Calculate lag time (approximately 0.6 * time of concentration)
    pub fn calculateLagTime(self: *const Watershed) f64 {
        return self.calculateTimeOfConcentration() * 0.6;
    }
};

const QueueItem = struct { x: usize, y: usize };

/// Watershed delineation algorithm
pub const WatershedDelineator = struct {
    flow_grid: *flow.FlowGrid,
    watershed_id_grid: []u32,
    watersheds: std.ArrayList(Watershed),
    allocator: std.mem.Allocator,

    pub fn init(flow_grid: *flow.FlowGrid, allocator: std.mem.Allocator) !WatershedDelineator {
        const watershed_id_grid = try allocator.alloc(u32, flow_grid.width * flow_grid.height);

        // Initialize with no watershed (0)
        for (watershed_id_grid) |*id| {
            id.* = 0;
        }

        return WatershedDelineator{
            .flow_grid = flow_grid,
            .watershed_id_grid = watershed_id_grid,
            .watersheds = std.ArrayList(Watershed){},
            .allocator = allocator,
        };
    }

    pub fn deinit(self: *WatershedDelineator) void {
        for (self.watersheds.items) |*watershed| {
            watershed.deinit(self.allocator);
        }
        self.watersheds.deinit(self.allocator);
        self.allocator.free(self.watershed_id_grid);
    }

    /// Delineate watershed from a pour point (outlet)
    pub fn delineateWatershed(
        self: *WatershedDelineator,
        outlet_x: usize,
        outlet_y: usize,
        watershed_id: u32,
    ) !void {
        if (outlet_x >= self.flow_grid.width or outlet_y >= self.flow_grid.height) return;

        const outlet_elevation = self.flow_grid.getElevation(outlet_x, outlet_y);
        var watershed = Watershed.init(watershed_id, outlet_x, outlet_y, outlet_elevation, self.allocator);

        // Use flood-fill algorithm to trace upstream
        var visited = try self.allocator.alloc(bool, self.flow_grid.width * self.flow_grid.height);
        defer self.allocator.free(visited);

        // Initialize visited array
        for (visited) |*v| {
            v.* = false;
        }

        // Flood fill from outlet
        var queue = std.ArrayList(QueueItem){};
        defer queue.deinit(self.allocator);

        try queue.append(self.allocator, QueueItem{ .x = outlet_x, .y = outlet_y });

        while (queue.items.len > 0) {
            const current = queue.pop() orelse break;
            const index = self.flow_grid.getIndex(current.x, current.y);

            if (visited[index]) continue;
            visited[index] = true;
            self.watershed_id_grid[index] = watershed_id;

            // Find all cells that flow into this cell
            try self.findUpstreamCells(current.x, current.y, &queue, visited);
        }

        // Trace watershed boundary
        try self.traceBoundary(&watershed);

        // Calculate watershed properties
        try self.calculateWatershedProperties(&watershed);

        try self.watersheds.append(self.allocator, watershed);
    }

    /// Find all cells that flow into the given cell
    fn findUpstreamCells(
        self: *WatershedDelineator,
        target_x: usize,
        target_y: usize,
        queue: *std.ArrayList(QueueItem),
        visited: []bool,
    ) !void {
        const directions = [_]flow.FlowDirection{ .east, .southeast, .south, .southwest, .west, .northwest, .north, .northeast };

        for (directions) |direction| {
            const offset = direction.getOffset();
            const nx = @as(i32, @intCast(target_x)) - offset.dx; // Reverse direction to find upstream
            const ny = @as(i32, @intCast(target_y)) - offset.dy;

            if (!self.flow_grid.inBounds(nx, ny)) continue;

            const upstream_x = @as(usize, @intCast(nx));
            const upstream_y = @as(usize, @intCast(ny));
            const upstream_index = self.flow_grid.getIndex(upstream_x, upstream_y);

            if (visited[upstream_index]) continue;

            // Check if upstream cell flows into target cell
            const upstream_flow = self.flow_grid.flow_direction[upstream_index];
            if (upstream_flow == direction) {
                try queue.append(self.allocator, QueueItem{ .x = upstream_x, .y = upstream_y });
            }
        }
    }

    /// Trace watershed boundary using Moore neighborhood algorithm
    fn traceBoundary(self: *WatershedDelineator, watershed: *Watershed) !void {
        // Find boundary cells (cells that have at least one neighbor not in watershed)
        var boundary_cells = std.ArrayList(QueueItem){};
        defer boundary_cells.deinit(self.allocator);

        for (0..self.flow_grid.height) |y| {
            for (0..self.flow_grid.width) |x| {
                const index = self.flow_grid.getIndex(x, y);

                if (self.watershed_id_grid[index] == watershed.id) {
                    // Check if this cell is on the boundary
                    if (self.isBoundaryCell(x, y, watershed.id)) {
                        try boundary_cells.append(self.allocator, QueueItem{ .x = x, .y = y });
                    }
                }
            }
        }

        // Convert boundary cells to world coordinates and add to watershed
        for (boundary_cells.items) |cell| {
            const world_x = (@as(f64, @floatFromInt(cell.x)) + 0.5) * self.flow_grid.cell_size;
            const world_y = (@as(f64, @floatFromInt(cell.y)) + 0.5) * self.flow_grid.cell_size;
            const elevation = self.flow_grid.getElevation(cell.x, cell.y);

            try watershed.boundary_points.append(self.allocator, BoundaryPoint.init(world_x, world_y, elevation));
        }
    }

    /// Check if a cell is on the watershed boundary
    fn isBoundaryCell(self: *WatershedDelineator, x: usize, y: usize, watershed_id: u32) bool {
        const directions = [_]struct { dx: i32, dy: i32 }{
            .{ .dx = -1, .dy = -1 }, .{ .dx = 0, .dy = -1 }, .{ .dx = 1, .dy = -1 },
            .{ .dx = -1, .dy = 0 },  .{ .dx = 1, .dy = 0 },  .{ .dx = -1, .dy = 1 },
            .{ .dx = 0, .dy = 1 },   .{ .dx = 1, .dy = 1 },
        };

        for (directions) |dir| {
            const nx = @as(i32, @intCast(x)) + dir.dx;
            const ny = @as(i32, @intCast(y)) + dir.dy;

            if (!self.flow_grid.inBounds(nx, ny)) return true; // Edge of grid is boundary

            const neighbor_index = self.flow_grid.getIndex(@intCast(nx), @intCast(ny));
            if (self.watershed_id_grid[neighbor_index] != watershed_id) {
                return true; // Different watershed or no watershed
            }
        }

        return false;
    }

    /// Calculate comprehensive watershed properties
    fn calculateWatershedProperties(self: *WatershedDelineator, watershed: *Watershed) !void {
        var cell_count: usize = 0;
        var elevation_sum: f64 = 0.0;
        var slope_sum: f64 = 0.0;
        var min_elevation: f64 = std.math.inf(f64);
        var max_elevation: f64 = -std.math.inf(f64);

        // Calculate basic properties
        for (0..self.flow_grid.height) |y| {
            for (0..self.flow_grid.width) |x| {
                const index = self.flow_grid.getIndex(x, y);

                if (self.watershed_id_grid[index] == watershed.id) {
                    cell_count += 1;
                    const elevation = self.flow_grid.getElevation(x, y);
                    elevation_sum += elevation;
                    min_elevation = @min(min_elevation, elevation);
                    max_elevation = @max(max_elevation, elevation);

                    // Calculate local slope
                    const slope = self.calculateLocalSlope(x, y);
                    slope_sum += slope;
                }
            }
        }

        if (cell_count > 0) {
            watershed.area = @as(f64, @floatFromInt(cell_count)) * self.flow_grid.cell_size * self.flow_grid.cell_size;
            watershed.mean_elevation = elevation_sum / @as(f64, @floatFromInt(cell_count));
            watershed.mean_slope = slope_sum / @as(f64, @floatFromInt(cell_count));
            watershed.relief = max_elevation - min_elevation;
        }

        // Calculate stream length within watershed
        watershed.stream_length = self.calculateStreamLength(watershed.id, 100.0); // 100 cells threshold for streams

        watershed.calculateMorphometrics();
    }

    /// Calculate local slope at a cell
    fn calculateLocalSlope(self: *WatershedDelineator, x: usize, y: usize) f64 {
        const center_elevation = self.flow_grid.getElevation(x, y);
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
                const neighbor_elevation = self.flow_grid.getElevation(@intCast(nx), @intCast(ny));
                const elevation_diff = @abs(center_elevation - neighbor_elevation);
                const distance = dir.mult * self.flow_grid.cell_size;
                const slope = elevation_diff / distance;
                max_slope = @max(max_slope, slope);
            }
        }

        return max_slope;
    }

    /// Calculate total stream length within watershed
    fn calculateStreamLength(self: *WatershedDelineator, watershed_id: u32, stream_threshold: f64) f64 {
        var total_length: f64 = 0.0;

        for (0..self.flow_grid.height) |y| {
            for (0..self.flow_grid.width) |x| {
                const index = self.flow_grid.getIndex(x, y);

                if (self.watershed_id_grid[index] == watershed_id) {
                    const accumulation = self.flow_grid.flow_accumulation[index];

                    if (accumulation >= stream_threshold) {
                        const flow_dir = self.flow_grid.flow_direction[index];
                        if (flow_dir != .none) {
                            const distance = flow_dir.getDistanceMultiplier() * self.flow_grid.cell_size;
                            total_length += distance;
                        }
                    }
                }
            }
        }

        return total_length;
    }
};

/// Calculate drainage network ordering (Strahler ordering)
pub fn calculateStrahlerOrder(
    flow_grid: *const flow.FlowGrid,
    stream_threshold: f64,
    strahler_order: []u32,
    allocator: std.mem.Allocator,
) !void {
    std.debug.assert(strahler_order.len >= flow_grid.width * flow_grid.height);

    // Initialize order array
    for (strahler_order) |*order| {
        order.* = 0;
    }

    // Identify stream cells
    var stream_cells = try allocator.alloc(bool, flow_grid.width * flow_grid.height);
    defer allocator.free(stream_cells);

    for (0..flow_grid.height) |y| {
        for (0..flow_grid.width) |x| {
            const index = flow_grid.getIndex(x, y);
            const accumulation = flow_grid.flow_accumulation[index];
            stream_cells[index] = accumulation >= stream_threshold;
        }
    }

    // Assign initial order (1) to source streams
    for (0..flow_grid.height) |y| {
        for (0..flow_grid.width) |x| {
            const index = flow_grid.getIndex(x, y);

            if (stream_cells[index]) {
                // Check if this is a source (no upstream streams)
                if (countUpstreamStreams(flow_grid, x, y, stream_cells) == 0) {
                    strahler_order[index] = 1;
                }
            }
        }
    }

    // Propagate orders downstream
    var changed = true;
    var max_iterations: usize = flow_grid.width * flow_grid.height;

    while (changed and max_iterations > 0) {
        changed = false;
        max_iterations -= 1;

        for (0..flow_grid.height) |y| {
            for (0..flow_grid.width) |x| {
                const index = flow_grid.getIndex(x, y);

                if (stream_cells[index] and strahler_order[index] == 0) {
                    const upstream_orders = getUpstreamOrders(flow_grid, x, y, stream_cells, strahler_order, allocator) catch continue;
                    defer allocator.free(upstream_orders);

                    if (upstream_orders.len > 0) {
                        const new_order = calculateStrahlerOrderFromUpstream(upstream_orders);
                        if (new_order > 0) {
                            strahler_order[index] = new_order;
                            changed = true;
                        }
                    }
                }
            }
        }
    }
}

/// Count upstream stream cells
fn countUpstreamStreams(
    flow_grid: *const flow.FlowGrid,
    x: usize,
    y: usize,
    stream_cells: []const bool,
) usize {
    var count: usize = 0;
    const directions = [_]flow.FlowDirection{ .east, .southeast, .south, .southwest, .west, .northwest, .north, .northeast };

    for (directions) |direction| {
        const offset = direction.getOffset();
        const nx = @as(i32, @intCast(x)) - offset.dx; // Reverse to find upstream
        const ny = @as(i32, @intCast(y)) - offset.dy;

        if (!flow_grid.inBounds(nx, ny)) continue;

        const upstream_x = @as(usize, @intCast(nx));
        const upstream_y = @as(usize, @intCast(ny));
        const upstream_index = flow_grid.getIndex(upstream_x, upstream_y);

        if (stream_cells[upstream_index]) {
            const upstream_flow = flow_grid.flow_direction[upstream_index];
            if (upstream_flow == direction) {
                count += 1;
            }
        }
    }

    return count;
}

/// Get Strahler orders of upstream cells
fn getUpstreamOrders(
    flow_grid: *const flow.FlowGrid,
    x: usize,
    y: usize,
    stream_cells: []const bool,
    strahler_order: []const u32,
    allocator: std.mem.Allocator,
) ![]u32 {
    var orders = std.ArrayList(u32){};
    const directions = [_]flow.FlowDirection{ .east, .southeast, .south, .southwest, .west, .northwest, .north, .northeast };

    for (directions) |direction| {
        const offset = direction.getOffset();
        const nx = @as(i32, @intCast(x)) - offset.dx;
        const ny = @as(i32, @intCast(y)) - offset.dy;

        if (!flow_grid.inBounds(nx, ny)) continue;

        const upstream_x = @as(usize, @intCast(nx));
        const upstream_y = @as(usize, @intCast(ny));
        const upstream_index = flow_grid.getIndex(upstream_x, upstream_y);

        if (stream_cells[upstream_index] and strahler_order[upstream_index] > 0) {
            const upstream_flow = flow_grid.flow_direction[upstream_index];
            if (upstream_flow == direction) {
                try orders.append(allocator, strahler_order[upstream_index]);
            }
        }
    }

    return orders.toOwnedSlice();
}

/// Calculate Strahler order from upstream orders
fn calculateStrahlerOrderFromUpstream(upstream_orders: []const u32) u32 {
    if (upstream_orders.len == 0) return 0;
    if (upstream_orders.len == 1) return upstream_orders[0];

    // Sort orders
    var sorted_orders = std.ArrayList(u32){};
    defer sorted_orders.deinit(std.heap.page_allocator);

    for (upstream_orders) |order| {
        sorted_orders.append(std.heap.page_allocator, order) catch return 0;
    }

    std.sort.insertion(u32, sorted_orders.items, {}, comptime std.sort.desc(u32));

    // Strahler rules:
    // - If highest two orders are equal, increment by 1
    // - Otherwise, take the highest order
    if (sorted_orders.items.len >= 2 and sorted_orders.items[0] == sorted_orders.items[1]) {
        return sorted_orders.items[0] + 1;
    } else {
        return sorted_orders.items[0];
    }
}
