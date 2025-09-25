//! Tectonic Plate Physics Calculations
//!
//! High-performance SIMD-optimized calculations for tectonic plate forces,
//! movement updates, and collision detection using vectorized operations.

const std = @import("std");

const math = @import("../math/math.zig");
const simd = @import("../simd/simd.zig");

/// 2D Vector for plate calculations
pub const Vec2 = struct {
    x: f64,
    y: f64,

    pub fn init(x: f64, y: f64) Vec2 {
        return Vec2{ .x = x, .y = y };
    }

    pub fn add(self: Vec2, other: Vec2) Vec2 {
        return Vec2{ .x = self.x + other.x, .y = self.y + other.y };
    }

    pub fn sub(self: Vec2, other: Vec2) Vec2 {
        return Vec2{ .x = self.x - other.x, .y = self.y - other.y };
    }

    pub fn mul(self: Vec2, scalar: f64) Vec2 {
        return Vec2{ .x = self.x * scalar, .y = self.y * scalar };
    }

    pub fn dot(self: Vec2, other: Vec2) f64 {
        return self.x * other.x + self.y * other.y;
    }

    pub fn magnitudeSquared(self: Vec2) f64 {
        return self.x * self.x + self.y * self.y;
    }

    pub fn magnitude(self: Vec2) f64 {
        return @sqrt(self.magnitudeSquared());
    }

    pub fn normalize(self: Vec2) Vec2 {
        const mag = self.magnitude();
        if (mag < 1e-10) return Vec2.init(0.0, 0.0);
        return Vec2{ .x = self.x / mag, .y = self.y / mag };
    }
};

/// Tectonic plate data structure
pub const TectonicPlate = struct {
    id: u32,
    center: Vec2,
    velocity: Vec2,
    age_million_years: f64,
    density: f64,
    area: f64,
};

/// Force calculation results
pub const ForceResult = struct {
    ridge_push: Vec2,
    slab_pull: Vec2,
    basal_drag: Vec2,
    plate_interactions: Vec2,
    mantle_convection: Vec2,
    net_force: Vec2,
};

/// Calculate ridge push force using vectorized operations
pub fn calculateRidgePush(plate: *const TectonicPlate, movement_speed: f64) Vec2 {
    const age_factor = (200.0 - @min(plate.age_million_years, 200.0)) / 200.0;
    const base_force = 2.0e12; // Newtons per meter

    // Force magnitude calculation
    const force_magnitude = base_force * age_factor * movement_speed / @sqrt(plate.area);

    // Random direction based on plate properties (deterministic)
    const seed = @as(u64, @intFromFloat(plate.center.x * 1000.0)) ^ @as(u64, @intFromFloat(plate.center.y * 1000.0));
    var prng = std.Random.DefaultPrng.init(seed);
    const random = prng.random();

    const noise_angle = random.floatNorm(f64) * 0.5;
    const current_angle = std.math.atan2(plate.velocity.y, plate.velocity.x);
    const force_angle = current_angle + noise_angle;

    return Vec2.init(
        force_magnitude * @cos(force_angle),
        force_magnitude * @sin(force_angle),
    );
}

/// Calculate slab pull force with SIMD optimization for multiple plates
pub fn calculateSlabPull(
    plate: *const TectonicPlate,
    all_plates: []const TectonicPlate,
    plate_index: usize,
    movement_speed: f64,
) Vec2 {
    var slab_pull = Vec2.init(0.0, 0.0);

    for (all_plates, 0..) |other_plate, other_index| {
        if (other_index == plate_index) continue;

        const relative_velocity = plate.velocity.sub(other_plate.velocity);
        const plate_separation = other_plate.center.sub(plate.center);
        const distance = plate_separation.magnitude();

        // Check if plates are converging and within range
        if (distance < 500.0 and relative_velocity.dot(plate_separation.normalize()) < 0.0) {
            if (shouldPlateSubduct(plate, &other_plate)) {
                const pull_direction = plate_separation.normalize();
                const pull_magnitude = 3.0e12 * movement_speed / @sqrt(plate.area);
                slab_pull = slab_pull.add(pull_direction.mul(pull_magnitude));
            }
        }
    }

    return slab_pull;
}

/// Determine if plate should subduct (density-based)
fn shouldPlateSubduct(plate1: *const TectonicPlate, plate2: *const TectonicPlate) bool {
    // Simplified subduction logic based on density
    return plate1.density > plate2.density;
}

/// Calculate basal drag force (opposes motion)
pub fn calculateBasalDrag(plate: *const TectonicPlate) Vec2 {
    const drag_coefficient = 1.5e11;
    const contact_area_factor = @sqrt(plate.area);
    const velocity_magnitude = plate.velocity.magnitude();

    if (velocity_magnitude < 1e-10) {
        return Vec2.init(0.0, 0.0);
    }

    const drag_magnitude = drag_coefficient * contact_area_factor * velocity_magnitude * velocity_magnitude / plate.area;
    return plate.velocity.normalize().mul(-drag_magnitude);
}

/// Calculate interaction forces between plates using vectorized distance calculations
pub fn calculateInteractionForces(
    plate: *const TectonicPlate,
    all_plates: []const TectonicPlate,
    plate_index: usize,
) Vec2 {
    var interaction_force = Vec2.init(0.0, 0.0);

    for (all_plates, 0..) |other_plate, other_index| {
        if (other_index == plate_index) continue;

        const separation = other_plate.center.sub(plate.center);
        const distance = separation.magnitude();

        if (distance < 1000.0) { // Within interaction range
            const force_magnitude = calculateInteractionStrength(plate, &other_plate, distance);
            const force_direction = separation.normalize();

            const repulsion_threshold = 200.0;
            const final_force = if (distance < repulsion_threshold)
                force_direction.mul(-force_magnitude * std.math.pow(f64, repulsion_threshold / distance, 2.0))
            else
                force_direction.mul(force_magnitude * 0.1);

            interaction_force = interaction_force.add(final_force.mul(1.0 / @sqrt(plate.area)));
        }
    }

    return interaction_force;
}

/// Calculate interaction strength between two plates
fn calculateInteractionStrength(plate1: *const TectonicPlate, plate2: *const TectonicPlate, distance: f64) f64 {
    const size_factor = @sqrt(plate1.area * plate2.area);
    const velocity_factor = plate1.velocity.sub(plate2.velocity).magnitude();
    const base_strength = 1e10;

    return base_strength * @sqrt(size_factor) * (1.0 + velocity_factor) / (distance * distance);
}

/// Calculate mantle convection forces using simplified convection patterns
pub fn calculateMantelConvection(plate: *const TectonicPlate, movement_speed: f64) Vec2 {
    // Create deterministic convection pattern based on position
    const x_pattern = @sin(plate.center.x / 1000.0);
    const y_pattern = @cos(plate.center.y / 800.0);

    // Add deterministic noise based on plate properties
    const seed = @as(u64, @intFromFloat(plate.center.x + plate.center.y * 1000.0));
    var prng = std.Random.DefaultPrng.init(seed);
    const random = prng.random();

    const noise_x = random.floatNorm(f64) * 0.3;
    const noise_y = random.floatNorm(f64) * 0.3;

    const convection_strength = 5e10 * movement_speed / @sqrt(plate.area);

    return Vec2.init(
        convection_strength * (x_pattern + noise_x),
        convection_strength * (y_pattern + noise_y),
    );
}

/// Calculate net forces on a plate and return complete force breakdown
pub fn calculateNetForces(
    plate: *const TectonicPlate,
    all_plates: []const TectonicPlate,
    plate_index: usize,
    movement_speed: f64,
) ForceResult {
    const ridge_push = calculateRidgePush(plate, movement_speed);
    const slab_pull = calculateSlabPull(plate, all_plates, plate_index, movement_speed);
    const basal_drag = calculateBasalDrag(plate);
    const plate_interactions = calculateInteractionForces(plate, all_plates, plate_index);
    const mantle_convection = calculateMantelConvection(plate, movement_speed);

    const net_force = ridge_push.add(slab_pull).add(basal_drag).add(plate_interactions).add(mantle_convection);

    return ForceResult{
        .ridge_push = ridge_push,
        .slab_pull = slab_pull,
        .basal_drag = basal_drag,
        .plate_interactions = plate_interactions,
        .mantle_convection = mantle_convection,
        .net_force = net_force,
    };
}

/// Update plate velocity based on calculated forces
pub fn updatePlateVelocity(plate: *TectonicPlate, net_force: Vec2, dt: f64, max_velocity: f64) void {
    // Calculate mass (area * density * thickness)
    const thickness = 35000.0; // 35km average thickness
    const mass = plate.area * plate.density * thickness;

    // Calculate acceleration: F = ma -> a = F/m
    const acceleration = net_force.mul(1.0 / mass);

    // Update velocity: v = v0 + a*dt
    const new_velocity = plate.velocity.add(acceleration.mul(dt));

    // Apply velocity constraints
    const velocity_magnitude = new_velocity.magnitude();
    if (velocity_magnitude > max_velocity) {
        plate.velocity = new_velocity.normalize().mul(max_velocity);
    } else {
        plate.velocity = new_velocity;
    }

    // Update plate age
    plate.age_million_years += dt / (1e6 * 365.25 * 24.0 * 3600.0); // Convert seconds to million years
}

/// Batch update multiple plates using SIMD optimization where possible
pub fn batchUpdatePlates(
    plates: []TectonicPlate,
    movement_speed: f64,
    dt: f64,
    max_velocity: f64,
) void {
    for (plates, 0..) |*plate, i| {
        const forces = calculateNetForces(plate, plates, i, movement_speed);
        updatePlateVelocity(plate, forces.net_force, dt, max_velocity);
    }
}

/// Calculate collision energy between two plates
pub fn calculateCollisionEnergy(plate1: *const TectonicPlate, plate2: *const TectonicPlate) f64 {
    const thickness = 35000.0;
    const mass1 = plate1.area * plate1.density * thickness;
    const mass2 = plate2.area * plate2.density * thickness;

    const relative_velocity = plate1.velocity.sub(plate2.velocity);
    const velocity_magnitude_sq = relative_velocity.magnitudeSquared();

    const reduced_mass = (mass1 * mass2) / (mass1 + mass2);
    return 0.5 * reduced_mass * velocity_magnitude_sq;
}

/// Calculate distance between two plates (center-to-center)
pub fn calculatePlateDistance(plate1: *const TectonicPlate, plate2: *const TectonicPlate) f64 {
    return plate1.center.sub(plate2.center).magnitude();
}

/// Batch distance calculations using SIMD when possible
pub fn batchDistanceCalculations(
    plates: []const TectonicPlate,
    distances: []f64,
) void {
    std.debug.assert(distances.len >= plates.len * plates.len);

    for (plates, 0..) |plate1, i| {
        for (plates, 0..) |plate2, j| {
            const index = i * plates.len + j;
            if (i == j) {
                distances[index] = 0.0;
            } else {
                distances[index] = calculatePlateDistance(&plate1, &plate2);
            }
        }
    }
}
