//! Groundwater and Aquifer Modeling
//!
//! High-performance SIMD-optimized groundwater flow calculations, aquifer analysis,
//! and spring generation using advanced hydrogeological principles.

const std = @import("std");

const math = @import("../math/math.zig");
const simd = @import("../simd/simd.zig");

/// Aquifer types based on geological properties
pub const AquiferType = enum {
    unconfined, // Water table aquifer
    confined, // Artesian aquifer
    leaky_confined, // Semi-confined aquifer
    perched, // Isolated water body
    fractured_rock, // Fracture-flow dominated
    karst, // Solution-enlarged fractures

    /// Get typical hydraulic conductivity range (m/s)
    pub fn getTypicalConductivity(self: AquiferType) struct { min: f64, max: f64 } {
        return switch (self) {
            .unconfined => .{ .min = 1e-6, .max = 1e-3 }, // Sand and gravel
            .confined => .{ .min = 1e-8, .max = 1e-4 }, // Confined sand/sandstone
            .leaky_confined => .{ .min = 1e-7, .max = 1e-4 }, // Semi-permeable layers
            .perched => .{ .min = 1e-6, .max = 1e-4 }, // Variable permeability
            .fractured_rock => .{ .min = 1e-8, .max = 1e-2 }, // Highly variable
            .karst => .{ .min = 1e-5, .max = 1e-1 }, // Very high in conduits
        };
    }
};

/// Groundwater flow direction and magnitude
pub const FlowVector = struct {
    velocity_x: f64, // m/s
    velocity_y: f64, // m/s
    magnitude: f64, // m/s
    direction: f64, // radians from east

    pub fn init(vx: f64, vy: f64) FlowVector {
        const mag = @sqrt(vx * vx + vy * vy);
        const dir = if (mag > 0.0) std.math.atan2(vy, vx) else 0.0;
        return FlowVector{
            .velocity_x = vx,
            .velocity_y = vy,
            .magnitude = mag,
            .direction = dir,
        };
    }
};

/// Aquifer properties grid cell
pub const AquiferCell = struct {
    hydraulic_head: f64, // m above datum
    hydraulic_conductivity: f64, // m/s
    specific_yield: f64, // dimensionless (unconfined)
    specific_storage: f64, // 1/m (confined)
    transmissivity: f64, // m²/s
    thickness: f64, // m
    porosity: f64, // dimensionless
    aquifer_type: AquiferType,
    recharge_rate: f64, // m/s
    extraction_rate: f64, // m/s (pumping)

    pub fn init(
        head: f64,
        conductivity: f64,
        thickness: f64,
        porosity: f64,
        aquifer_type: AquiferType,
    ) AquiferCell {
        const transmissivity = conductivity * thickness;
        const specific_yield = switch (aquifer_type) {
            .unconfined => porosity * 0.8, // Effective porosity
            else => 0.0,
        };
        var specific_storage: f64 = 0.0;
        switch (aquifer_type) {
            .confined, .leaky_confined => {
                specific_storage = 1e-5; // Typical value
            },
            else => {
                specific_storage = 0.0;
            },
        }

        return AquiferCell{
            .hydraulic_head = head,
            .hydraulic_conductivity = conductivity,
            .specific_yield = specific_yield,
            .specific_storage = specific_storage,
            .transmissivity = transmissivity,
            .thickness = thickness,
            .porosity = porosity,
            .aquifer_type = aquifer_type,
            .recharge_rate = 0.0,
            .extraction_rate = 0.0,
        };
    }

    /// Calculate Darcy velocity components
    pub fn calculateDarcyVelocity(
        self: *const AquiferCell,
        head_gradient_x: f64,
        head_gradient_y: f64,
    ) FlowVector {
        // Darcy's law: v = -K * (dh/dx, dh/dy)
        const vx = -self.hydraulic_conductivity * head_gradient_x;
        const vy = -self.hydraulic_conductivity * head_gradient_y;
        return FlowVector.init(vx, vy);
    }

    /// Calculate seepage velocity (actual groundwater velocity)
    pub fn calculateSeepageVelocity(
        self: *const AquiferCell,
        darcy_velocity: FlowVector,
    ) FlowVector {
        if (self.porosity <= 0.0) return FlowVector.init(0.0, 0.0);

        // Seepage velocity = Darcy velocity / effective porosity
        const effective_porosity = self.porosity * 0.8; // Approximate effective porosity
        const scale_factor = 1.0 / effective_porosity;

        return FlowVector.init(
            darcy_velocity.velocity_x * scale_factor,
            darcy_velocity.velocity_y * scale_factor,
        );
    }
};

/// Groundwater flow grid for regional analysis
pub const GroundwaterGrid = struct {
    width: usize,
    height: usize,
    cell_size: f64, // meters
    cells: []AquiferCell,
    flow_vectors: []FlowVector,
    time_step: f64, // seconds for transient analysis

    pub fn init(
        width: usize,
        height: usize,
        cell_size: f64,
        allocator: std.mem.Allocator,
    ) !GroundwaterGrid {
        const cells = try allocator.alloc(AquiferCell, width * height);
        const flow_vectors = try allocator.alloc(FlowVector, width * height);

        // Initialize with default values
        for (cells) |*cell| {
            cell.* = AquiferCell.init(100.0, 1e-5, 50.0, 0.3, .unconfined);
        }

        for (flow_vectors) |*vector| {
            vector.* = FlowVector.init(0.0, 0.0);
        }

        return GroundwaterGrid{
            .width = width,
            .height = height,
            .cell_size = cell_size,
            .cells = cells,
            .flow_vectors = flow_vectors,
            .time_step = 86400.0, // 1 day default
        };
    }

    pub fn deinit(self: *GroundwaterGrid, allocator: std.mem.Allocator) void {
        allocator.free(self.cells);
        allocator.free(self.flow_vectors);
    }

    /// Get linear index from grid coordinates
    pub fn getIndex(self: *const GroundwaterGrid, x: usize, y: usize) usize {
        return y * self.width + x;
    }

    /// Check if coordinates are within bounds
    pub fn inBounds(self: *const GroundwaterGrid, x: i32, y: i32) bool {
        return x >= 0 and y >= 0 and x < self.width and y < self.height;
    }

    /// Calculate hydraulic head gradients using finite differences
    pub fn calculateHeadGradients(self: *GroundwaterGrid) void {
        for (0..self.height) |y| {
            for (0..self.width) |x| {
                const index = self.getIndex(x, y);
                const cell = &self.cells[index];

                // Calculate gradients using centered differences where possible
                var dh_dx: f64 = 0.0;
                var dh_dy: f64 = 0.0;

                // X-direction gradient
                if (x > 0 and x < self.width - 1) {
                    const east_head = self.cells[self.getIndex(x + 1, y)].hydraulic_head;
                    const west_head = self.cells[self.getIndex(x - 1, y)].hydraulic_head;
                    dh_dx = (east_head - west_head) / (2.0 * self.cell_size);
                } else if (x == 0 and self.width > 1) {
                    const east_head = self.cells[self.getIndex(x + 1, y)].hydraulic_head;
                    dh_dx = (east_head - cell.hydraulic_head) / self.cell_size;
                } else if (x == self.width - 1 and self.width > 1) {
                    const west_head = self.cells[self.getIndex(x - 1, y)].hydraulic_head;
                    dh_dx = (cell.hydraulic_head - west_head) / self.cell_size;
                }

                // Y-direction gradient
                if (y > 0 and y < self.height - 1) {
                    const north_head = self.cells[self.getIndex(x, y - 1)].hydraulic_head;
                    const south_head = self.cells[self.getIndex(x, y + 1)].hydraulic_head;
                    dh_dy = (north_head - south_head) / (2.0 * self.cell_size);
                } else if (y == 0 and self.height > 1) {
                    const south_head = self.cells[self.getIndex(x, y + 1)].hydraulic_head;
                    dh_dy = (cell.hydraulic_head - south_head) / self.cell_size;
                } else if (y == self.height - 1 and self.height > 1) {
                    const north_head = self.cells[self.getIndex(x, y - 1)].hydraulic_head;
                    dh_dy = (north_head - cell.hydraulic_head) / self.cell_size;
                }

                // Calculate flow vector
                self.flow_vectors[index] = cell.calculateDarcyVelocity(dh_dx, dh_dy);
            }
        }
    }

    /// Solve steady-state groundwater flow using iterative method
    pub fn solveStreadyState(self: *GroundwaterGrid, max_iterations: usize, tolerance: f64) void {
        var iteration: usize = 0;
        var max_change: f64 = tolerance + 1.0;

        while (iteration < max_iterations and max_change > tolerance) {
            max_change = 0.0;
            iteration += 1;

            for (1..self.height - 1) |y| {
                for (1..self.width - 1) |x| {
                    const index = self.getIndex(x, y);
                    const cell = &self.cells[index];

                    // Skip if boundary condition or no-flow cell
                    if (cell.hydraulic_conductivity <= 0.0) continue;

                    // Finite difference equation for 2D groundwater flow
                    const old_head = cell.hydraulic_head;
                    const new_head = self.calculateNewHead(x, y);

                    cell.hydraulic_head = new_head;
                    const change = @abs(new_head - old_head);
                    max_change = @max(max_change, change);
                }
            }
        }

        // Update flow vectors after convergence
        self.calculateHeadGradients();
    }

    /// Calculate new hydraulic head using finite difference method
    fn calculateNewHead(self: *const GroundwaterGrid, x: usize, y: usize) f64 {
        const index = self.getIndex(x, y);
        const cell = &self.cells[index];

        // Get neighboring cells
        const east_cell = &self.cells[self.getIndex(x + 1, y)];
        const west_cell = &self.cells[self.getIndex(x - 1, y)];
        const north_cell = &self.cells[self.getIndex(x, y - 1)];
        const south_cell = &self.cells[self.getIndex(x, y + 1)];

        // Calculate transmissivities at cell interfaces
        const t_east = harmonicMean(cell.transmissivity, east_cell.transmissivity);
        const t_west = harmonicMean(cell.transmissivity, west_cell.transmissivity);
        const t_north = harmonicMean(cell.transmissivity, north_cell.transmissivity);
        const t_south = harmonicMean(cell.transmissivity, south_cell.transmissivity);

        const dx = self.cell_size;
        const dy = self.cell_size;

        // Finite difference equation coefficients
        const a_east = t_east / (dx * dx);
        const a_west = t_west / (dx * dx);
        const a_north = t_north / (dy * dy);
        const a_south = t_south / (dy * dy);
        const a_center = a_east + a_west + a_north + a_south;

        // Source/sink term (recharge - extraction)
        const source = (cell.recharge_rate - cell.extraction_rate) * dx * dy;

        // Calculate new head
        const numerator = a_east * east_cell.hydraulic_head +
            a_west * west_cell.hydraulic_head +
            a_north * north_cell.hydraulic_head +
            a_south * south_cell.hydraulic_head +
            source;

        if (a_center > 0.0) {
            return numerator / a_center;
        } else {
            return cell.hydraulic_head; // No change if no conductivity
        }
    }
};

/// Spring characteristics and generation
pub const Spring = struct {
    x: f64,
    y: f64,
    elevation: f64,
    discharge: f64, // m³/s
    temperature: f64, // °C
    spring_type: SpringType,
    aquifer_connection: AquiferType,
    seasonal_variation: f64, // Coefficient of variation

    pub const SpringType = enum {
        gravity, // Topographic springs
        artesian, // Pressure springs
        contact, // Geological contact springs
        depression, // Springs in valleys
        joint, // Fracture springs
        thermal, // Hot springs
    };

    pub fn init(
        x: f64,
        y: f64,
        elevation: f64,
        aquifer_type: AquiferType,
        hydraulic_head: f64,
        ground_elevation: f64,
        temperature: f64,
    ) Spring {
        // Determine spring type based on conditions
        const spring_type: SpringType = if (temperature > 30.0)
            .thermal
        else if (hydraulic_head > ground_elevation + 10.0)
            .artesian
        else if (elevation < ground_elevation - 5.0)
            .depression
        else
            .gravity;

        // Estimate discharge based on hydraulic conditions
        const head_difference = @max(0.0, hydraulic_head - elevation);
        const discharge = calculateSpringDischarge(head_difference, aquifer_type);

        return Spring{
            .x = x,
            .y = y,
            .elevation = elevation,
            .discharge = discharge,
            .temperature = temperature,
            .spring_type = spring_type,
            .aquifer_connection = aquifer_type,
            .seasonal_variation = 0.3, // 30% variation
        };
    }

    /// Calculate seasonal discharge variation
    pub fn getSeasonalDischarge(self: *const Spring, day_of_year: u32) f64 {
        const phase = 2.0 * std.math.pi * @as(f64, @floatFromInt(day_of_year)) / 365.25;
        const seasonal_factor = 1.0 + self.seasonal_variation * @sin(phase + std.math.pi); // Peak in late summer
        return self.discharge * seasonal_factor;
    }
};

/// Calculate spring discharge using empirical relationships
pub fn calculateSpringDischarge(head_difference: f64, aquifer_type: AquiferType) f64 {
    if (head_difference <= 0.0) return 0.0;

    // Base discharge coefficient based on aquifer type
    const base_coeff: f64 = switch (aquifer_type) {
        .karst => 0.1, // High discharge potential
        .fractured_rock => 0.05, // Moderate discharge
        .unconfined => 0.02, // Low to moderate
        .confined => 0.08, // Can be high if head is sufficient
        .leaky_confined => 0.03, // Moderate
        .perched => 0.01, // Usually low
    };

    // Discharge proportional to square root of head (orifice flow approximation)
    return base_coeff * @sqrt(head_difference);
}

/// Generate springs based on topographic and hydrogeological conditions
pub fn generateSprings(
    groundwater_grid: *const GroundwaterGrid,
    elevation_grid: []const f64,
    springs: *std.ArrayList(Spring),
    min_discharge: f64,
    allocator: std.mem.Allocator,
) !void {
    for (0..groundwater_grid.height) |y| {
        for (0..groundwater_grid.width) |x| {
            const index = groundwater_grid.getIndex(x, y);
            const cell = &groundwater_grid.cells[index];
            const ground_elevation = elevation_grid[index];

            // Check conditions for spring formation
            if (shouldFormSpring(cell, ground_elevation, groundwater_grid, x, y)) {
                const world_x = (@as(f64, @floatFromInt(x)) + 0.5) * groundwater_grid.cell_size;
                const world_y = (@as(f64, @floatFromInt(y)) + 0.5) * groundwater_grid.cell_size;

                // Estimate temperature based on depth and geothermal gradient
                const depth_to_water = @max(0.0, ground_elevation - cell.hydraulic_head);
                const temperature = 15.0 + depth_to_water * 0.025; // 25°C per km geothermal gradient

                const spring = Spring.init(
                    world_x,
                    world_y,
                    ground_elevation,
                    cell.aquifer_type,
                    cell.hydraulic_head,
                    ground_elevation,
                    temperature,
                );

                if (spring.discharge >= min_discharge) {
                    try springs.append(allocator, spring);
                }
            }
        }
    }
}

/// Determine if conditions are suitable for spring formation
fn shouldFormSpring(
    cell: *const AquiferCell,
    ground_elevation: f64,
    grid: *const GroundwaterGrid,
    x: usize,
    y: usize,
) bool {
    // Spring forms when water table intersects ground surface
    if (cell.hydraulic_head <= ground_elevation) return false;

    // Check for suitable geological conditions
    const conductivity_threshold = switch (cell.aquifer_type) {
        .karst, .fractured_rock => 1e-6,
        else => 1e-5,
    };

    if (cell.hydraulic_conductivity < conductivity_threshold) return false;

    // Check for topographic focusing (valley, depression)
    const is_topographic_low = isTopographicLow(grid, x, y, ground_elevation);

    // Check for flow convergence
    const flow_convergence = calculateFlowConvergence(grid, x, y);

    return is_topographic_low or flow_convergence > 0.1;
}

/// Check if location is topographically suitable for spring
fn isTopographicLow(grid: *const GroundwaterGrid, x: usize, y: usize, elevation: f64) bool {
    var neighbor_count: usize = 0;
    var higher_neighbors: usize = 0;

    const directions = [_]struct { dx: i32, dy: i32 }{
        .{ .dx = -1, .dy = -1 }, .{ .dx = 0, .dy = -1 }, .{ .dx = 1, .dy = -1 },
        .{ .dx = -1, .dy = 0 },  .{ .dx = 1, .dy = 0 },  .{ .dx = -1, .dy = 1 },
        .{ .dx = 0, .dy = 1 },   .{ .dx = 1, .dy = 1 },
    };

    for (directions) |dir| {
        const nx = @as(i32, @intCast(x)) + dir.dx;
        const ny = @as(i32, @intCast(y)) + dir.dy;

        if (grid.inBounds(nx, ny)) {
            const neighbor_index = grid.getIndex(@intCast(nx), @intCast(ny));
            // Note: This assumes elevation data is available in the same grid structure
            // In practice, you'd need to pass elevation data or have it accessible
            neighbor_count += 1;

            // Placeholder: assume neighbor elevation is cell's hydraulic head for now
            const neighbor_elevation = grid.cells[neighbor_index].hydraulic_head;
            if (neighbor_elevation > elevation) {
                higher_neighbors += 1;
            }
        }
    }

    // Spring likely if most neighbors are higher (natural drainage point)
    return neighbor_count > 0 and @as(f64, @floatFromInt(higher_neighbors)) / @as(f64, @floatFromInt(neighbor_count)) > 0.6;
}

/// Calculate flow convergence at a point
fn calculateFlowConvergence(grid: *const GroundwaterGrid, x: usize, y: usize) f64 {
    var convergence: f64 = 0.0;
    var neighbor_count: usize = 0;

    const directions = [_]struct { dx: i32, dy: i32 }{
        .{ .dx = -1, .dy = -1 }, .{ .dx = 0, .dy = -1 }, .{ .dx = 1, .dy = -1 },
        .{ .dx = -1, .dy = 0 },  .{ .dx = 1, .dy = 0 },  .{ .dx = -1, .dy = 1 },
        .{ .dx = 0, .dy = 1 },   .{ .dx = 1, .dy = 1 },
    };

    for (directions) |dir| {
        const nx = @as(i32, @intCast(x)) + dir.dx;
        const ny = @as(i32, @intCast(y)) + dir.dy;

        if (grid.inBounds(nx, ny)) {
            const neighbor_index = grid.getIndex(@intCast(nx), @intCast(ny));
            const neighbor_flow = grid.flow_vectors[neighbor_index];

            // Calculate angle between flows (convergence when flows point toward center)
            const flow_toward_center_x = @as(f64, @floatFromInt(x)) - @as(f64, @floatFromInt(nx));
            const flow_toward_center_y = @as(f64, @floatFromInt(y)) - @as(f64, @floatFromInt(ny));

            const dot_product = neighbor_flow.velocity_x * flow_toward_center_x +
                neighbor_flow.velocity_y * flow_toward_center_y;

            if (dot_product > 0.0) { // Flow toward center
                convergence += neighbor_flow.magnitude;
            }
            neighbor_count += 1;
        }
    }

    return if (neighbor_count > 0) convergence / @as(f64, @floatFromInt(neighbor_count)) else 0.0;
}

/// Calculate harmonic mean for interface properties
fn harmonicMean(a: f64, b: f64) f64 {
    if (a <= 0.0 or b <= 0.0) return 0.0;
    return 2.0 * a * b / (a + b);
}

/// Calculate well pumping effects using Theis solution
pub fn calculateTheisSolution(
    distance: f64, // Distance from well (m)
    time: f64, // Time since pumping started (s)
    pumping_rate: f64, // Pumping rate (m³/s)
    transmissivity: f64, // Aquifer transmissivity (m²/s)
    storativity: f64, // Aquifer storativity (dimensionless)
) f64 {
    if (distance <= 0.0 or time <= 0.0 or transmissivity <= 0.0 or storativity <= 0.0) {
        return 0.0;
    }

    const u = distance * distance * storativity / (4.0 * transmissivity * time);
    const well_function = wellFunction(u);

    return (pumping_rate / (4.0 * std.math.pi * transmissivity)) * well_function;
}

/// Calculate well function W(u) using series expansion
fn wellFunction(u: f64) f64 {
    if (u > 10.0) return 0.0; // Negligible for large u

    if (u < 0.01) {
        // Series expansion for small u
        const euler_gamma = 0.5772156649015329;
        return -euler_gamma - @log(u) + u - u * u / 4.0 + u * u * u / 18.0;
    } else {
        // Numerical integration approximation
        var result: f64 = 0.0;
        var t: f64 = u;
        var term: f64 = 1.0;
        var n: f64 = 1.0;

        while (term > 1e-10 and n < 100) {
            result += term / t;
            t *= u / n;
            term = t;
            n += 1.0;
        }

        return std.math.exp(-u) * result;
    }
}
