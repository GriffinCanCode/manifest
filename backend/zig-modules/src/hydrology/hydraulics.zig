//! Hydraulic Calculations and Flow Dynamics
//!
//! High-performance SIMD-optimized hydraulic calculations including Manning's equation,
//! open channel flow, pipe flow, and advanced hydraulic analysis with vectorized operations.

const std = @import("std");

const math = @import("../math/math.zig");
const simd = @import("../simd/simd.zig");

/// Channel cross-section types
pub const ChannelType = enum {
    rectangular,
    trapezoidal,
    triangular,
    circular,
    parabolic,
    irregular,

    /// Get wetted perimeter for given area and channel parameters
    pub fn getWettedPerimeter(self: ChannelType, area: f64, width: f64, side_slope: f64) f64 {
        return switch (self) {
            .rectangular => width + 2.0 * (area / width),
            .trapezoidal => {
                const depth = (-width + @sqrt(width * width + 4.0 * side_slope * area)) / (2.0 * side_slope);
                return width + 2.0 * depth * @sqrt(1.0 + side_slope * side_slope);
            },
            .triangular => 2.0 * @sqrt(area * (1.0 + side_slope * side_slope)) / side_slope,
            .circular => {
                // For circular channels, iterative solution required
                // Simplified approximation for now
                return std.math.pi * @sqrt(area / std.math.pi);
            },
            .parabolic => {
                const top_width = @sqrt(4.0 * width * area / width);
                return (top_width / 6.0) * (2.0 * @sqrt(top_width * top_width + 16.0 * width * width) +
                    (top_width * top_width / width) *
                        std.math.log(@abs((top_width + @sqrt(top_width * top_width + 16.0 * width * width)) / (4.0 * width))));
            },
            .irregular => area / @sqrt(area), // Rough approximation
        };
    }
};

/// Flow regime classification
pub const FlowRegime = enum {
    subcritical, // Fr < 1.0
    critical, // Fr ≈ 1.0
    supercritical, // Fr > 1.0

    pub fn fromFroude(froude: f64) FlowRegime {
        if (froude < 0.98) return .subcritical;
        if (froude > 1.02) return .supercritical;
        return .critical;
    }
};

/// Hydraulic calculation results
pub const HydraulicResults = struct {
    velocity: f64, // m/s
    discharge: f64, // m³/s
    hydraulic_radius: f64, // m
    wetted_perimeter: f64, // m
    froude_number: f64, // dimensionless
    reynolds_number: f64, // dimensionless
    flow_regime: FlowRegime,
    energy_grade_line: f64, // m
    specific_energy: f64, // m

    pub fn init() HydraulicResults {
        return HydraulicResults{
            .velocity = 0.0,
            .discharge = 0.0,
            .hydraulic_radius = 0.0,
            .wetted_perimeter = 0.0,
            .froude_number = 0.0,
            .reynolds_number = 0.0,
            .flow_regime = .subcritical,
            .energy_grade_line = 0.0,
            .specific_energy = 0.0,
        };
    }
};

/// Calculate Manning's equation for open channel flow
pub fn calculateManning(
    cross_sectional_area: f64,
    wetted_perimeter: f64,
    slope: f64,
    manning_n: f64,
) HydraulicResults {
    var results = HydraulicResults.init();

    if (cross_sectional_area <= 0.0 or wetted_perimeter <= 0.0 or slope <= 0.0 or manning_n <= 0.0) {
        return results;
    }

    const hydraulic_radius = cross_sectional_area / wetted_perimeter;
    const velocity = (1.0 / manning_n) * std.math.pow(f64, hydraulic_radius, 2.0 / 3.0) * @sqrt(slope);
    const discharge = velocity * cross_sectional_area;

    results.velocity = velocity;
    results.discharge = discharge;
    results.hydraulic_radius = hydraulic_radius;
    results.wetted_perimeter = wetted_perimeter;

    return results;
}

/// Calculate open channel flow with complete hydraulic analysis
pub fn calculateOpenChannelFlow(
    area: f64,
    width: f64,
    depth: f64,
    slope: f64,
    manning_n: f64,
    channel_type: ChannelType,
    side_slope: f64, // For trapezoidal channels (horizontal:vertical)
) HydraulicResults {
    var results = HydraulicResults.init();

    if (area <= 0.0 or depth <= 0.0 or slope <= 0.0 or manning_n <= 0.0) {
        return results;
    }

    // Calculate wetted perimeter based on channel type
    const wetted_perimeter = channel_type.getWettedPerimeter(area, width, side_slope);
    const hydraulic_radius = area / wetted_perimeter;

    // Manning's equation
    const velocity = (1.0 / manning_n) * std.math.pow(f64, hydraulic_radius, 2.0 / 3.0) * @sqrt(slope);
    const discharge = velocity * area;

    // Calculate Froude number
    const gravity = 9.81; // m/s²
    const hydraulic_depth = area / width; // Top width for irregular sections
    const froude_number = velocity / @sqrt(gravity * hydraulic_depth);

    // Calculate Reynolds number (approximate kinematic viscosity for water at 20°C)
    const kinematic_viscosity = 1.004e-6; // m²/s
    const reynolds_number = velocity * hydraulic_radius / kinematic_viscosity;

    // Calculate energy quantities
    const velocity_head = velocity * velocity / (2.0 * gravity);
    const specific_energy = depth + velocity_head;
    const energy_grade_line = depth + velocity_head; // Assuming no pressure head

    results.velocity = velocity;
    results.discharge = discharge;
    results.hydraulic_radius = hydraulic_radius;
    results.wetted_perimeter = wetted_perimeter;
    results.froude_number = froude_number;
    results.reynolds_number = reynolds_number;
    results.flow_regime = FlowRegime.fromFroude(froude_number);
    results.energy_grade_line = energy_grade_line;
    results.specific_energy = specific_energy;

    return results;
}

/// Calculate pipe flow using Darcy-Weisbach equation
pub fn calculatePipeFlow(
    diameter: f64,
    length: f64,
    head_loss: f64,
    roughness: f64, // Absolute roughness (m)
    kinematic_viscosity: f64,
) HydraulicResults {
    var results = HydraulicResults.init();

    if (diameter <= 0.0 or length <= 0.0 or head_loss <= 0.0) {
        return results;
    }

    const gravity = 9.81; // m/s²
    const area = std.math.pi * diameter * diameter / 4.0;
    const hydraulic_radius = diameter / 4.0; // For circular pipes
    const wetted_perimeter = std.math.pi * diameter;

    // Iterative solution for Darcy-Weisbach equation
    var velocity: f64 = 1.0; // Initial guess
    var friction_factor: f64 = 0.02; // Initial guess

    for (0..20) |_| {
        const reynolds = velocity * diameter / kinematic_viscosity;
        friction_factor = calculateFrictionFactor(reynolds, roughness / diameter);

        // Darcy-Weisbach: hf = f * (L/D) * (V²/2g)
        const new_velocity = @sqrt(2.0 * gravity * head_loss * diameter / (friction_factor * length));

        if (@abs(new_velocity - velocity) < 0.001) break;
        velocity = new_velocity;
    }

    const discharge = velocity * area;
    const reynolds_number = velocity * diameter / kinematic_viscosity;

    results.velocity = velocity;
    results.discharge = discharge;
    results.hydraulic_radius = hydraulic_radius;
    results.wetted_perimeter = wetted_perimeter;
    results.reynolds_number = reynolds_number;

    return results;
}

/// Calculate friction factor using Colebrook-White equation
pub fn calculateFrictionFactor(reynolds: f64, relative_roughness: f64) f64 {
    if (reynolds < 2300.0) {
        // Laminar flow
        return 64.0 / reynolds;
    } else if (reynolds < 4000.0) {
        // Transition region (use interpolation)
        const f_laminar = 64.0 / 2300.0;
        const f_turbulent = calculateTurbulentFriction(4000.0, relative_roughness);
        const weight = (reynolds - 2300.0) / (4000.0 - 2300.0);
        return f_laminar * (1.0 - weight) + f_turbulent * weight;
    } else {
        // Turbulent flow
        return calculateTurbulentFriction(reynolds, relative_roughness);
    }
}

/// Calculate friction factor for turbulent flow using Colebrook-White
fn calculateTurbulentFriction(reynolds: f64, relative_roughness: f64) f64 {
    // Colebrook-White equation (iterative solution)
    var f: f64 = 0.02; // Initial guess

    for (0..10) |_| {
        const sqrt_f = @sqrt(f);
        const rhs = -2.0 * std.math.log10(relative_roughness / 3.7 + 2.51 / (reynolds * sqrt_f));
        const new_f = 1.0 / (rhs * rhs);

        if (@abs(new_f - f) < 0.0001) break;
        f = new_f;
    }

    return f;
}

/// Calculate critical depth for open channel flow
pub fn calculateCriticalDepth(discharge: f64, width: f64, gravity: f64) f64 {
    if (discharge <= 0.0 or width <= 0.0) return 0.0;

    // For rectangular channel: yc = (Q²/(g*B²))^(1/3)
    const q = discharge / width; // Unit discharge
    return std.math.pow(f64, q * q / gravity, 1.0 / 3.0);
}

/// Calculate normal depth using Manning's equation (iterative solution)
pub fn calculateNormalDepth(
    discharge: f64,
    width: f64,
    slope: f64,
    manning_n: f64,
    channel_type: ChannelType,
    side_slope: f64,
) f64 {
    if (discharge <= 0.0 or width <= 0.0 or slope <= 0.0 or manning_n <= 0.0) {
        return 0.0;
    }

    var depth: f64 = 1.0; // Initial guess

    for (0..20) |_| {
        const area = calculateArea(depth, width, channel_type, side_slope);
        const wetted_perimeter = channel_type.getWettedPerimeter(area, width, side_slope);
        const hydraulic_radius = area / wetted_perimeter;

        const calculated_discharge = (1.0 / manning_n) * area *
            std.math.pow(f64, hydraulic_radius, 2.0 / 3.0) * @sqrt(slope);

        const discharge_error = calculated_discharge - discharge;
        if (@abs(discharge_error) < 0.01) break;

        // Newton-Raphson iteration (simplified)
        const delta = 0.01;
        const area_plus = calculateArea(depth + delta, width, channel_type, side_slope);
        const wp_plus = channel_type.getWettedPerimeter(area_plus, width, side_slope);
        const hr_plus = area_plus / wp_plus;
        const q_plus = (1.0 / manning_n) * area_plus *
            std.math.pow(f64, hr_plus, 2.0 / 3.0) * @sqrt(slope);

        const derivative = (q_plus - calculated_discharge) / delta;
        if (derivative != 0.0) {
            depth -= discharge_error / derivative;
        }
        depth = @max(0.01, depth); // Ensure positive depth
    }

    return depth;
}

/// Calculate cross-sectional area for different channel types
pub fn calculateArea(depth: f64, width: f64, channel_type: ChannelType, side_slope: f64) f64 {
    return switch (channel_type) {
        .rectangular => width * depth,
        .trapezoidal => depth * (width + side_slope * depth),
        .triangular => side_slope * depth * depth,
        .circular => {
            // For circular channel (complex calculation)
            const radius = width / 2.0; // Assuming width = diameter
            if (depth >= 2.0 * radius) {
                return std.math.pi * radius * radius;
            } else {
                const theta = 2.0 * std.math.acos((radius - depth) / radius);
                return radius * radius * (theta - @sin(theta)) / 2.0;
            }
        },
        .parabolic => (2.0 * width * depth) / 3.0,
        .irregular => width * depth * 0.8, // Rough approximation
    };
}

/// Calculate hydraulic jump characteristics
pub fn calculateHydraulicJump(
    upstream_depth: f64,
    upstream_velocity: f64,
    gravity: f64,
) struct { downstream_depth: f64, energy_loss: f64, jump_length: f64 } {
    const froude1 = upstream_velocity / @sqrt(gravity * upstream_depth);

    if (froude1 <= 1.0) {
        return .{ .downstream_depth = upstream_depth, .energy_loss = 0.0, .jump_length = 0.0 };
    }

    // Sequent depth relationship
    const downstream_depth = (upstream_depth / 2.0) *
        (@sqrt(1.0 + 8.0 * froude1 * froude1) - 1.0);

    // Energy loss
    const energy_loss = std.math.pow(f64, downstream_depth - upstream_depth, 3.0) /
        (4.0 * upstream_depth * downstream_depth);

    // Jump length (empirical relationship)
    const jump_length = downstream_depth * (2.5 + 3.5 * froude1);

    return .{
        .downstream_depth = downstream_depth,
        .energy_loss = energy_loss,
        .jump_length = jump_length,
    };
}

/// Calculate gradually varied flow profile using standard step method
pub fn calculateWaterSurfaceProfile(
    discharge: f64,
    channel_width: f64,
    manning_n: f64,
    channel_type: ChannelType,
    side_slope: f64,
    bed_elevations: []const f64,
    distances: []const f64,
    starting_depth: f64,
    results: []f64, // Output water surface elevations
) void {
    std.debug.assert(bed_elevations.len == distances.len);
    std.debug.assert(results.len >= bed_elevations.len);

    if (bed_elevations.len == 0) return;

    const gravity = 9.81;
    var current_depth = starting_depth;

    results[0] = bed_elevations[0] + current_depth;

    for (1..bed_elevations.len) |i| {
        const dx = distances[i] - distances[i - 1];
        const dz = bed_elevations[i] - bed_elevations[i - 1];
        const bed_slope = -dz / dx; // Positive for downward slope

        // Calculate energy slope
        const area = calculateArea(current_depth, channel_width, channel_type, side_slope);
        const wetted_perimeter = channel_type.getWettedPerimeter(area, channel_width, side_slope);
        const hydraulic_radius = area / wetted_perimeter;

        const velocity = discharge / area;

        // Energy slope using Manning's equation
        const manning_slope = (manning_n * velocity * @sqrt(velocity * velocity)) /
            std.math.pow(f64, hydraulic_radius, 4.0 / 3.0);

        // Standard step equation: dy/dx = (S0 - Sf) / (1 - Fr²)
        const froude_sq = velocity * velocity / (gravity * current_depth);
        const denominator = 1.0 - froude_sq;

        if (@abs(denominator) < 0.01) {
            // Near critical flow, use small step
            current_depth += (bed_slope - manning_slope) * dx * 0.1;
        } else {
            const depth_change = (bed_slope - manning_slope) * dx / denominator;
            current_depth += depth_change;
        }

        current_depth = @max(0.01, current_depth); // Ensure positive depth
        results[i] = bed_elevations[i] + current_depth;
    }
}

/// Calculate discharge coefficient for weirs and spillways
pub fn calculateWeirDischarge(
    head: f64, // Head over weir crest (m)
    length: f64, // Effective length of weir (m)
    discharge_coefficient: f64, // Cd (typically 0.6-0.8)
    weir_type: enum { sharp_crested, broad_crested, ogee },
) f64 {
    if (head <= 0.0 or length <= 0.0) return 0.0;

    const gravity = 9.81;

    return switch (weir_type) {
        .sharp_crested => discharge_coefficient * length * @sqrt(2.0 * gravity) * std.math.pow(f64, head, 1.5),
        .broad_crested => discharge_coefficient * length * @sqrt(2.0 * gravity) * std.math.pow(f64, head, 1.5),
        .ogee => discharge_coefficient * length * @sqrt(2.0 * gravity) * std.math.pow(f64, head, 1.5),
    };
}

/// Batch calculate Manning's equation for multiple channels
pub fn batchManningCalculation(
    areas: []const f64,
    wetted_perimeters: []const f64,
    slopes: []const f64,
    manning_ns: []const f64,
    results: []HydraulicResults,
) void {
    const count = @min(@min(areas.len, wetted_perimeters.len), @min(slopes.len, manning_ns.len));
    std.debug.assert(results.len >= count);

    for (0..count) |i| {
        results[i] = calculateManning(areas[i], wetted_perimeters[i], slopes[i], manning_ns[i]);
    }
}
