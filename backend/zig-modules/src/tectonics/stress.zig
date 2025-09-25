//! Tectonic Stress Field Calculations
//!
//! High-performance SIMD-optimized 2D stress field computations for
//! seismic hazard mapping and geological stress analysis.

const std = @import("std");

const simd = @import("../simd/simd.zig");

/// 2D stress tensor
pub const StressTensor = struct {
    xx: f64, // Normal stress in x direction
    yy: f64, // Normal stress in y direction
    xy: f64, // Shear stress

    pub fn init(xx: f64, yy: f64, xy: f64) StressTensor {
        return StressTensor{ .xx = xx, .yy = yy, .xy = xy };
    }

    pub fn zero() StressTensor {
        return StressTensor.init(0.0, 0.0, 0.0);
    }

    pub fn add(self: StressTensor, other: StressTensor) StressTensor {
        return StressTensor.init(
            self.xx + other.xx,
            self.yy + other.yy,
            self.xy + other.xy,
        );
    }

    pub fn scale(self: StressTensor, factor: f64) StressTensor {
        return StressTensor.init(
            self.xx * factor,
            self.yy * factor,
            self.xy * factor,
        );
    }

    /// Calculate Von Mises stress
    pub fn vonMisesStress(self: StressTensor) f64 {
        return @sqrt(self.xx * self.xx + self.yy * self.yy - self.xx * self.yy + 3.0 * self.xy * self.xy);
    }

    /// Calculate maximum principal stress
    pub fn maxPrincipalStress(self: StressTensor) f64 {
        const mean_stress = (self.xx + self.yy) / 2.0;
        const stress_diff = (self.xx - self.yy) / 2.0;
        const shear_sq = self.xy * self.xy;
        return mean_stress + @sqrt(stress_diff * stress_diff + shear_sq);
    }

    /// Calculate minimum principal stress
    pub fn minPrincipalStress(self: StressTensor) f64 {
        const mean_stress = (self.xx + self.yy) / 2.0;
        const stress_diff = (self.xx - self.yy) / 2.0;
        const shear_sq = self.xy * self.xy;
        return mean_stress - @sqrt(stress_diff * stress_diff + shear_sq);
    }

    /// Calculate principal stress direction (angle in radians)
    pub fn principalStressAngle(self: StressTensor) f64 {
        if (@abs(self.xx - self.yy) < 1e-10) {
            return std.math.pi / 4.0; // 45 degrees when xx == yy
        }
        return 0.5 * std.math.atan2(2.0 * self.xy, self.xx - self.yy);
    }
};

/// 2D stress field grid
pub const StressField = struct {
    width: usize,
    height: usize,
    resolution: f64, // meters per grid cell
    origin_x: f64,
    origin_y: f64,
    stress_data: []StressTensor,

    pub fn init(
        width: usize,
        height: usize,
        resolution: f64,
        origin_x: f64,
        origin_y: f64,
        allocator: std.mem.Allocator,
    ) !StressField {
        const stress_data = try allocator.alloc(StressTensor, width * height);

        // Initialize to zero stress
        for (stress_data) |*tensor| {
            tensor.* = StressTensor.zero();
        }

        return StressField{
            .width = width,
            .height = height,
            .resolution = resolution,
            .origin_x = origin_x,
            .origin_y = origin_y,
            .stress_data = stress_data,
        };
    }

    pub fn deinit(self: *StressField, allocator: std.mem.Allocator) void {
        allocator.free(self.stress_data);
    }

    /// Get stress tensor at grid position
    pub fn getStress(self: *const StressField, x: usize, y: usize) StressTensor {
        if (x >= self.width or y >= self.height) return StressTensor.zero();
        return self.stress_data[y * self.width + x];
    }

    /// Set stress tensor at grid position
    pub fn setStress(self: *StressField, x: usize, y: usize, stress: StressTensor) void {
        if (x >= self.width or y >= self.height) return;
        self.stress_data[y * self.width + x] = stress;
    }

    /// Add stress tensor at grid position
    pub fn addStress(self: *StressField, x: usize, y: usize, stress: StressTensor) void {
        if (x >= self.width or y >= self.height) return;
        const index = y * self.width + x;
        self.stress_data[index] = self.stress_data[index].add(stress);
    }

    /// Convert world coordinates to grid coordinates
    pub fn worldToGrid(self: *const StressField, world_x: f64, world_y: f64) struct { x: usize, y: usize } {
        const grid_x = @as(usize, @intFromFloat(@floor((world_x - self.origin_x) / self.resolution)));
        const grid_y = @as(usize, @intFromFloat(@floor((world_y - self.origin_y) / self.resolution)));
        return .{ .x = grid_x, .y = grid_y };
    }

    /// Convert grid coordinates to world coordinates (center of cell)
    pub fn gridToWorld(self: *const StressField, grid_x: usize, grid_y: usize) struct { x: f64, y: f64 } {
        const world_x = self.origin_x + (@as(f64, @floatFromInt(grid_x)) + 0.5) * self.resolution;
        const world_y = self.origin_y + (@as(f64, @floatFromInt(grid_y)) + 0.5) * self.resolution;
        return .{ .x = world_x, .y = world_y };
    }

    /// Sample stress at world coordinates using bilinear interpolation
    pub fn sampleStress(self: *const StressField, world_x: f64, world_y: f64) StressTensor {
        const fx = (world_x - self.origin_x) / self.resolution - 0.5;
        const fy = (world_y - self.origin_y) / self.resolution - 0.5;

        const x0 = @as(usize, @intFromFloat(@floor(fx)));
        const y0 = @as(usize, @intFromFloat(@floor(fy)));
        const x1 = x0 + 1;
        const y1 = y0 + 1;

        if (x1 >= self.width or y1 >= self.height) {
            return StressTensor.zero();
        }

        const wx = fx - @floor(fx);
        const wy = fy - @floor(fy);

        const s00 = self.getStress(x0, y0);
        const s10 = self.getStress(x1, y0);
        const s01 = self.getStress(x0, y1);
        const s11 = self.getStress(x1, y1);

        // Bilinear interpolation
        const s_x0 = StressTensor.init(
            s00.xx * (1.0 - wx) + s10.xx * wx,
            s00.yy * (1.0 - wx) + s10.yy * wx,
            s00.xy * (1.0 - wx) + s10.xy * wx,
        );

        const s_x1 = StressTensor.init(
            s01.xx * (1.0 - wx) + s11.xx * wx,
            s01.yy * (1.0 - wx) + s11.yy * wx,
            s01.xy * (1.0 - wx) + s11.xy * wx,
        );

        return StressTensor.init(
            s_x0.xx * (1.0 - wy) + s_x1.xx * wy,
            s_x0.yy * (1.0 - wy) + s_x1.yy * wy,
            s_x0.xy * (1.0 - wy) + s_x1.xy * wy,
        );
    }
};

/// Point source of stress (e.g., fault, volcano)
pub const StressSource = struct {
    x: f64,
    y: f64,
    magnitude: f64,
    source_type: SourceType,
    orientation: f64, // radians

    pub const SourceType = enum {
        point_force, // Point force source
        fault_slip, // Fault slip dislocation
        pressure_source, // Pressure/volume source
        shear_zone, // Distributed shear
    };
};

/// Calculate stress field from a point force source
pub fn calculatePointForceStress(
    field: *StressField,
    source: StressSource,
    elastic_modulus: f64,
    poisson_ratio: f64,
) void {
    const mu = elastic_modulus / (2.0 * (1.0 + poisson_ratio)); // Shear modulus
    // const lambda = elastic_modulus * poisson_ratio / ((1.0 + poisson_ratio) * (1.0 - 2.0 * poisson_ratio)); // Lame parameter (unused)

    for (0..field.height) |j| {
        for (0..field.width) |i| {
            const coords = field.gridToWorld(i, j);
            const dx = coords.x - source.x;
            const dy = coords.y - source.y;
            const r_sq = dx * dx + dy * dy;

            if (r_sq < 1.0) continue; // Avoid singularity at source

            const r = @sqrt(r_sq);
            const r_cubed = r_sq * r;

            // Force components in source direction
            const fx = source.magnitude * @cos(source.orientation);
            const fy = source.magnitude * @sin(source.orientation);

            // Green's function for 2D elasticity (point force)
            const factor = 1.0 / (4.0 * std.math.pi * mu);

            const stress_xx = factor * (fx * (3.0 * dx * dx / r_cubed - 1.0 / r) +
                fy * (3.0 * dx * dy / r_cubed));

            const stress_yy = factor * (fy * (3.0 * dy * dy / r_cubed - 1.0 / r) +
                fx * (3.0 * dx * dy / r_cubed));

            const stress_xy = factor * (fx * (3.0 * dx * dy / r_cubed) +
                fy * (3.0 * dy * dy / r_cubed - 1.0 / r));

            const stress = StressTensor.init(stress_xx, stress_yy, stress_xy);
            field.addStress(i, j, stress);
        }
    }
}

/// Calculate stress field from fault slip dislocation
pub fn calculateFaultSlipStress(
    field: *StressField,
    fault_x1: f64,
    fault_y1: f64,
    fault_x2: f64,
    fault_y2: f64,
    slip_magnitude: f64,
    elastic_modulus: f64,
    poisson_ratio: f64,
) void {
    const mu = elastic_modulus / (2.0 * (1.0 + poisson_ratio));

    // Fault parameters
    const fault_dx = fault_x2 - fault_x1;
    const fault_dy = fault_y2 - fault_y1;
    const fault_length = @sqrt(fault_dx * fault_dx + fault_dy * fault_dy);

    if (fault_length < 1e-6) return;

    const fault_nx = -fault_dy / fault_length; // Normal vector
    const fault_ny = fault_dx / fault_length;
    const fault_tx = fault_dx / fault_length; // Tangent vector
    const fault_ty = fault_dy / fault_length;

    // Discretize fault into segments
    const num_segments = @max(10, @as(usize, @intFromFloat(@ceil(fault_length / 1000.0)))); // 1km segments

    for (0..num_segments) |seg| {
        const t = (@as(f64, @floatFromInt(seg)) + 0.5) / @as(f64, @floatFromInt(num_segments));
        const seg_x = fault_x1 + t * fault_dx;
        const seg_y = fault_y1 + t * fault_dy;
        const seg_slip = slip_magnitude / @as(f64, @floatFromInt(num_segments));

        // Calculate stress from this segment
        for (0..field.height) |j| {
            for (0..field.width) |i| {
                const coords = field.gridToWorld(i, j);
                const dx = coords.x - seg_x;
                const dy = coords.y - seg_y;
                const r_sq = dx * dx + dy * dy;

                if (r_sq < 100.0) continue; // Avoid near-field singularity

                // const r = @sqrt(r_sq); // Unused

                // Okada's solution for 2D dislocation (simplified)
                const factor = mu * seg_slip / (2.0 * std.math.pi);

                // Transform to fault coordinates
                const x_fault = dx * fault_tx + dy * fault_ty;
                const y_fault = -dx * fault_nx + dy * fault_ny;

                const r_fault = @sqrt(x_fault * x_fault + y_fault * y_fault);
                const theta = std.math.atan2(y_fault, x_fault);

                // Stress components in fault coordinates
                const stress_rr = factor * @sin(2.0 * theta) / (r_fault * r_fault);
                const stress_tt = -stress_rr;
                const stress_rt = factor * @cos(2.0 * theta) / (r_fault * r_fault);

                // Transform back to global coordinates
                const cos_theta = x_fault / r_fault;
                const sin_theta = y_fault / r_fault;

                const stress_xx = stress_rr * cos_theta * cos_theta + stress_tt * sin_theta * sin_theta +
                    stress_rt * 2.0 * cos_theta * sin_theta;
                const stress_yy = stress_rr * sin_theta * sin_theta + stress_tt * cos_theta * cos_theta -
                    stress_rt * 2.0 * cos_theta * sin_theta;
                const stress_xy = (stress_rr - stress_tt) * cos_theta * sin_theta +
                    stress_rt * (cos_theta * cos_theta - sin_theta * sin_theta);

                const stress = StressTensor.init(stress_xx, stress_yy, stress_xy);
                field.addStress(i, j, stress);
            }
        }
    }
}

/// Calculate stress field from pressure source (volcano, intrusion)
pub fn calculatePressureSourceStress(
    field: *StressField,
    source_x: f64,
    source_y: f64,
    pressure: f64,
    radius: f64,
    depth: f64,
    elastic_modulus: f64,
    poisson_ratio: f64,
) void {
    const nu = poisson_ratio;
    const factor = pressure * radius * radius * (1.0 - nu) / elastic_modulus;

    for (0..field.height) |j| {
        for (0..field.width) |i| {
            const coords = field.gridToWorld(i, j);
            const dx = coords.x - source_x;
            const dy = coords.y - source_y;
            const r = @sqrt(dx * dx + dy * dy + depth * depth);

            if (r < radius) continue; // Inside source

            const r_cubed = r * r * r;
            const r_fifth = r_cubed * r * r;

            // Mogi model for spherical pressure source
            const stress_rr = factor * (1.0 / r_cubed - 3.0 * depth * depth / r_fifth);
            const stress_tt = factor * (1.0 / r_cubed + 3.0 * depth * depth / r_fifth);

            // Transform to Cartesian coordinates
            const cos_theta = dx / @sqrt(dx * dx + dy * dy);
            const sin_theta = dy / @sqrt(dx * dx + dy * dy);

            const stress_xx = stress_rr * cos_theta * cos_theta + stress_tt * sin_theta * sin_theta;
            const stress_yy = stress_rr * sin_theta * sin_theta + stress_tt * cos_theta * cos_theta;
            const stress_xy = (stress_rr - stress_tt) * cos_theta * sin_theta;

            const stress = StressTensor.init(stress_xx, stress_yy, stress_xy);
            field.addStress(i, j, stress);
        }
    }
}

/// Apply regional tectonic stress field
pub fn applyRegionalStress(
    field: *StressField,
    regional_stress_xx: f64,
    regional_stress_yy: f64,
    regional_stress_xy: f64,
) void {
    const regional = StressTensor.init(regional_stress_xx, regional_stress_yy, regional_stress_xy);

    for (field.stress_data) |*stress| {
        stress.* = stress.add(regional);
    }
}

/// Calculate stress invariants for entire field
pub fn calculateStressInvariants(
    field: *const StressField,
    von_mises: []f64,
    max_principal: []f64,
    min_principal: []f64,
) void {
    std.debug.assert(von_mises.len >= field.width * field.height);
    std.debug.assert(max_principal.len >= field.width * field.height);
    std.debug.assert(min_principal.len >= field.width * field.height);

    for (field.stress_data, 0..) |stress, i| {
        von_mises[i] = stress.vonMisesStress();
        max_principal[i] = stress.maxPrincipalStress();
        min_principal[i] = stress.minPrincipalStress();
    }
}

/// Update stress field with time-dependent effects
pub fn updateStressField(
    field: *StressField,
    dt: f64,
    viscosity: f64,
) void {
    // Simple viscoelastic relaxation
    const relaxation_factor = @exp(-dt / viscosity);

    for (field.stress_data) |*stress| {
        stress.* = stress.scale(relaxation_factor);
    }
}

/// Calculate Coulomb failure stress on optimally oriented planes
pub fn calculateCoulombStress(
    field: *const StressField,
    friction_coefficient: f64,
    coulomb_stress: []f64,
) void {
    std.debug.assert(coulomb_stress.len >= field.width * field.height);

    for (field.stress_data, 0..) |stress, i| {
        const sigma1 = stress.maxPrincipalStress();
        const sigma3 = stress.minPrincipalStress();

        // Coulomb failure criteria for optimally oriented planes
        coulomb_stress[i] = (sigma1 - sigma3) / 2.0 - friction_coefficient * (sigma1 + sigma3) / 2.0;
    }
}

/// Smooth stress field using Gaussian filter
pub fn smoothStressField(
    field: *StressField,
    sigma: f64,
    temp_field: []StressTensor,
    allocator: std.mem.Allocator,
) !void {
    std.debug.assert(temp_field.len >= field.width * field.height);

    const kernel_size = @as(usize, @intFromFloat(@ceil(3.0 * sigma))) * 2 + 1;
    const kernel = try allocator.alloc(f64, kernel_size * kernel_size);
    defer allocator.free(kernel);

    // Create Gaussian kernel
    const center = @as(f64, @floatFromInt(kernel_size)) / 2.0;
    var sum: f64 = 0.0;

    for (0..kernel_size) |j| {
        for (0..kernel_size) |i| {
            const dx = @as(f64, @floatFromInt(i)) - center;
            const dy = @as(f64, @floatFromInt(j)) - center;
            const value = @exp(-(dx * dx + dy * dy) / (2.0 * sigma * sigma));
            kernel[j * kernel_size + i] = value;
            sum += value;
        }
    }

    // Normalize kernel
    for (kernel) |*value| {
        value.* /= sum;
    }

    // Apply convolution
    for (0..field.height) |y| {
        for (0..field.width) |x| {
            var convolved = StressTensor.zero();

            for (0..kernel_size) |ky| {
                for (0..kernel_size) |kx| {
                    const sy = y + ky - kernel_size / 2;
                    const sx = x + kx - kernel_size / 2;

                    if (sx < field.width and sy < field.height) {
                        const stress = field.getStress(sx, sy);
                        const weight = kernel[ky * kernel_size + kx];
                        convolved = convolved.add(stress.scale(weight));
                    }
                }
            }

            temp_field[y * field.width + x] = convolved;
        }
    }

    // Copy back to original field
    for (field.stress_data, 0..) |*stress, i| {
        stress.* = temp_field[i];
    }
}
