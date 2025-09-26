//! Volcanic Activity Calculations
//!
//! High-performance SIMD-optimized volcanic influence calculations,
//! magma chamber modeling, and eruption probability assessments.

const std = @import("std");

const simd = @import("../simd/simd.zig");

/// Volcanic hazard calculation result
pub const VolcanicHazard = struct {
    pyroclastic_flow_hazard: f64,
    ash_fall_hazard: f64,
    lava_flow_hazard: f64,
    lahar_hazard: f64,
    gas_hazard: f64,
    combined_hazard: f64,

    pub fn init() VolcanicHazard {
        return VolcanicHazard{
            .pyroclastic_flow_hazard = 0.0,
            .ash_fall_hazard = 0.0,
            .lava_flow_hazard = 0.0,
            .lahar_hazard = 0.0,
            .gas_hazard = 0.0,
            .combined_hazard = 0.0,
        };
    }

    pub fn calculateCombined(self: *VolcanicHazard) void {
        // Weighted combination of hazard types
        const weights = [_]f64{ 0.3, 0.25, 0.2, 0.15, 0.1 };
        const hazards = [_]f64{
            self.pyroclastic_flow_hazard,
            self.ash_fall_hazard,
            self.lava_flow_hazard,
            self.lahar_hazard,
            self.gas_hazard,
        };

        var total: f64 = 0.0;
        for (hazards, weights) |hazard, weight| {
            total += hazard * weight;
        }
        self.combined_hazard = total;
    }
};

/// Volcano data structure
pub const Volcano = struct {
    x: f64,
    y: f64,
    elevation: f64,
    vei_scale: u32, // Volcanic Explosivity Index
    hazard_radius: f64,
    magma_chamber_depth: f64,
    last_eruption_years_ago: f64,
    eruption_probability: f64,

    /// Calculate eruption probability based on various factors
    pub fn updateEruptionProbability(self: *Volcano, _: f64, regional_stress: f64) void {
        // Base probability based on VEI and repose time
        const base_prob = switch (self.vei_scale) {
            0...1 => 0.8, // Very active
            2...3 => 0.5, // Moderately active
            4...5 => 0.2, // Less active
            6...8 => 0.05, // Rarely active
            else => 0.01,
        };

        // Time factor - probability increases with time since last eruption
        const repose_factor = @min(2.0, self.last_eruption_years_ago / 1000.0);

        // Stress factor - higher regional stress increases probability
        const stress_factor = 1.0 + @min(1.0, regional_stress / 1e6);

        // Depth factor - shallower chambers more likely to erupt
        const depth_factor = @max(0.5, 2.0 - self.magma_chamber_depth / 20.0);

        self.eruption_probability = base_prob * repose_factor * stress_factor * depth_factor;
        self.eruption_probability = @min(1.0, self.eruption_probability);
    }
};

/// Magma chamber properties
pub const MagmaChamber = struct {
    volume: f64, // m³
    pressure: f64, // Pa
    temperature: f64, // K
    viscosity: f64, // Pa·s
    gas_content: f64, // weight fraction
    crystal_content: f64, // weight fraction
    depth: f64, // m below surface

    pub fn init(volume: f64, depth: f64, temperature: f64) MagmaChamber {
        return MagmaChamber{
            .volume = volume,
            .pressure = calculateLithostaticPressure(depth),
            .temperature = temperature,
            .viscosity = calculateMagmaViscosity(temperature, 0.05), // 5% crystals
            .gas_content = 0.03, // 3% gas
            .crystal_content = 0.05, // 5% crystals
            .depth = depth,
        };
    }

    /// Calculate overpressure that could trigger eruption
    pub fn calculateOverpressure(self: *const MagmaChamber) f64 {
        const lithostatic = calculateLithostaticPressure(self.depth);
        return @max(0.0, self.pressure - lithostatic);
    }

    /// Update chamber properties based on time evolution
    pub fn evolve(self: *MagmaChamber, dt: f64, heat_loss_rate: f64) void {
        // Cool the magma
        self.temperature -= heat_loss_rate * dt;
        self.temperature = @max(900.0, self.temperature); // Solidification temperature

        // Update viscosity based on temperature and crystal content
        self.viscosity = calculateMagmaViscosity(self.temperature, self.crystal_content);

        // Gas exsolution with cooling
        if (self.temperature < 1000.0) {
            self.gas_content += 0.001 * dt; // Simplified gas exsolution
            self.gas_content = @min(0.1, self.gas_content);
        }

        // Crystallization
        if (self.temperature < 1100.0) {
            self.crystal_content += 0.002 * dt;
            self.crystal_content = @min(0.6, self.crystal_content);
        }
    }
};

/// Calculate lithostatic pressure at depth
fn calculateLithostaticPressure(depth: f64) f64 {
    const rock_density = 2700.0; // kg/m³
    const gravity = 9.81; // m/s²
    return rock_density * gravity * depth;
}

/// Calculate magma viscosity based on temperature and crystal content
fn calculateMagmaViscosity(temperature: f64, crystal_content: f64) f64 {
    // Simplified viscosity model
    const base_viscosity = @exp(18000.0 / temperature - 10.0); // Temperature dependence
    const crystal_factor = @exp(2.5 * crystal_content / (1.0 - crystal_content)); // Crystal effect
    return base_viscosity * crystal_factor;
}

/// Calculate pyroclastic flow hazard at distance from volcano
pub fn calculatePyroclasticFlowHazard(
    volcano: *const Volcano,
    target_x: f64,
    target_y: f64,
    _: f64, // wind_direction (unused for pyroclastic flows)
    _: f64, // wind_speed (unused for pyroclastic flows)
) f64 {
    const dx = target_x - volcano.x;
    const dy = target_y - volcano.y;
    const distance = @sqrt(dx * dx + dy * dy);

    if (distance > volcano.hazard_radius) return 0.0;

    // Base hazard based on VEI scale
    const base_hazard: f64 = switch (volcano.vei_scale) {
        0...1 => 0.1,
        2...3 => 0.3,
        4...5 => 0.85, // Increased base hazard for VEI 4-5
        6...8 => 0.95,
        else => 1.0,
    };

    // Distance decay (pyroclastic flows are gravity-driven)
    // For very close targets (< 1km), hazard should be very high
    const distance_factor = if (distance < 1000.0)
        1.0 - (distance / 1000.0) * 0.05 // 95-100% hazard within 1km
    else
        @exp(-distance / (volcano.hazard_radius * 0.3));

    // Topographic channeling (simplified - flows follow valleys)
    const elevation_factor = 1.0; // Would need DEM for proper calculation

    // Wind has minimal effect on pyroclastic flows
    const wind_factor = 1.0;

    return base_hazard * distance_factor * elevation_factor * wind_factor;
}

/// Calculate ash fall hazard at distance from volcano
pub fn calculateAshFallHazard(
    volcano: *const Volcano,
    target_x: f64,
    target_y: f64,
    wind_direction: f64,
    wind_speed: f64,
    eruption_column_height: f64,
) f64 {
    const dx = target_x - volcano.x;
    const dy = target_y - volcano.y;
    const distance = @sqrt(dx * dx + dy * dy);

    // Wind direction effect (ash drifts downwind)
    const target_bearing = std.math.atan2(dy, dx);
    const wind_alignment = @cos(target_bearing - wind_direction);

    // Ash dispersal model (simplified Tephra2)
    const column_height_km = eruption_column_height / 1000.0;
    const max_distance = column_height_km * 20.0 * wind_speed; // Empirical relationship

    if (distance > max_distance) return 0.0;

    // Base hazard from VEI
    const base_hazard: f64 = switch (volcano.vei_scale) {
        0...1 => 0.05,
        2...3 => 0.2,
        4...5 => 0.6,
        6...8 => 0.9,
        else => 1.0,
    };

    // Distance and wind effects
    const distance_factor = @exp(-distance / (max_distance * 0.4));
    const wind_factor = @max(0.1, (wind_alignment + 1.0) / 2.0);
    const wind_speed_factor = @min(2.0, wind_speed / 10.0);

    return base_hazard * distance_factor * wind_factor * wind_speed_factor;
}

/// Calculate lava flow hazard using simplified flow model
pub fn calculateLavaFlowHazard(
    volcano: *const Volcano,
    target_x: f64,
    target_y: f64,
    slope_angle: f64,
    effusion_rate: f64,
) f64 {
    const dx = target_x - volcano.x;
    const dy = target_y - volcano.y;
    const distance = @sqrt(dx * dx + dy * dy);

    // Lava flow range based on effusion rate and slope
    const base_range = 10000.0; // 10 km base range
    const slope_factor = @max(0.1, @sin(slope_angle));
    const effusion_factor = @sqrt(effusion_rate / 100.0); // m³/s
    const max_range = base_range * slope_factor * effusion_factor;

    if (distance > max_range) return 0.0;

    // Hazard decreases with distance and increases with slope towards volcano
    const distance_factor = @exp(-distance / (max_range * 0.5));

    // Check if target is downhill from volcano (simplified)
    const elevation_diff = volcano.elevation - 1000.0; // Assume target at lower elevation
    const downhill_factor = if (elevation_diff > 0) 1.5 else 0.5;

    return 0.8 * distance_factor * downhill_factor;
}

/// Calculate lahar (mudflow) hazard
pub fn calculateLaharHazard(
    volcano: *const Volcano,
    target_x: f64,
    target_y: f64,
    rainfall_rate: f64,
    channel_distance: f64,
) f64 {
    const dx = target_x - volcano.x;
    const dy = target_y - volcano.y;
    const distance = @sqrt(dx * dx + dy * dy);

    // Lahars follow river channels and can travel very far
    const max_range = 50000.0; // 50 km maximum range

    if (distance > max_range) return 0.0;

    // Base hazard depends on recent volcanic activity and rainfall
    const activity_factor = @exp(-volcano.last_eruption_years_ago / 50.0);
    const rainfall_factor = @min(2.0, rainfall_rate / 10.0); // mm/hr

    // Channel proximity is crucial for lahar hazard
    var channel_factor: f64 = 1.0;
    if (channel_distance < 1000.0) {
        channel_factor = 1.0;
    } else if (channel_distance < 5000.0) {
        channel_factor = @exp(-(channel_distance - 1000.0) / 2000.0);
    } else {
        channel_factor = 0.1;
    }

    const distance_factor = @exp(-distance / (max_range * 0.3));

    return 0.6 * activity_factor * rainfall_factor * channel_factor * distance_factor;
}

/// Calculate volcanic gas hazard
pub fn calculateGasHazard(
    volcano: *const Volcano,
    target_x: f64,
    target_y: f64,
    wind_direction: f64,
    wind_speed: f64,
    atmospheric_stability: f64,
) f64 {
    const dx = target_x - volcano.x;
    const dy = target_y - volcano.y;
    const distance = @sqrt(dx * dx + dy * dy);

    const max_range = 20000.0; // 20 km for gas hazard

    if (distance > max_range) return 0.0;

    // Gas dispersion is highly dependent on wind and atmospheric conditions
    const target_bearing = std.math.atan2(dy, dx);
    const wind_alignment = @cos(target_bearing - wind_direction);

    // Base hazard from volcanic activity level
    const base_hazard = if (volcano.eruption_probability > 0.5) 0.4 else 0.1;

    // Wind and atmospheric effects
    const wind_factor = @max(0.1, (wind_alignment + 1.0) / 2.0);
    const dispersion_factor = @max(0.2, wind_speed / 20.0) * atmospheric_stability;
    const distance_factor = @exp(-distance / (max_range * 0.6));

    return base_hazard * wind_factor * dispersion_factor * distance_factor;
}

/// Calculate complete volcanic hazard at target location
pub fn calculateVolcanicHazard(
    volcano: *const Volcano,
    target_x: f64,
    target_y: f64,
    wind_direction: f64,
    wind_speed: f64,
    slope_angle: f64,
    rainfall_rate: f64,
    channel_distance: f64,
    atmospheric_stability: f64,
) VolcanicHazard {
    var hazard = VolcanicHazard.init();

    // Only calculate hazards if volcano is potentially active
    if (volcano.eruption_probability > 0.01) {
        const eruption_column_height = switch (volcano.vei_scale) {
            0...1 => 3000.0, // 3 km
            2...3 => 10000.0, // 10 km
            4...5 => 25000.0, // 25 km
            6...8 => 40000.0, // 40 km
            else => 50000.0, // 50 km
        };

        const effusion_rate = switch (volcano.vei_scale) {
            0...2 => 100.0, // 100 m³/s
            3...4 => 1000.0, // 1000 m³/s
            else => 500.0, // 500 m³/s (explosive eruptions have lower effusion)
        };

        hazard.pyroclastic_flow_hazard = calculatePyroclasticFlowHazard(volcano, target_x, target_y, wind_direction, wind_speed);

        hazard.ash_fall_hazard = calculateAshFallHazard(volcano, target_x, target_y, wind_direction, wind_speed, eruption_column_height);

        hazard.lava_flow_hazard = calculateLavaFlowHazard(volcano, target_x, target_y, slope_angle, effusion_rate);

        hazard.lahar_hazard = calculateLaharHazard(volcano, target_x, target_y, rainfall_rate, channel_distance);

        hazard.gas_hazard = calculateGasHazard(volcano, target_x, target_y, wind_direction, wind_speed, atmospheric_stability);

        // Scale all hazards by eruption probability
        const prob_factor = volcano.eruption_probability;
        hazard.pyroclastic_flow_hazard *= prob_factor;
        hazard.ash_fall_hazard *= prob_factor;
        hazard.lava_flow_hazard *= prob_factor;
        hazard.lahar_hazard *= prob_factor;
        hazard.gas_hazard *= prob_factor;

        hazard.calculateCombined();
    }

    return hazard;
}

/// Batch calculate volcanic hazards for multiple points
pub fn batchCalculateVolcanicHazards(
    volcanoes: []const Volcano,
    target_points: []const struct { x: f64, y: f64 },
    hazards: []VolcanicHazard,
    wind_direction: f64,
    wind_speed: f64,
) void {
    std.debug.assert(hazards.len >= target_points.len);

    for (target_points, 0..) |point, i| {
        hazards[i] = VolcanicHazard.init();

        // Find the closest volcano or most hazardous combination
        var combined_hazard = VolcanicHazard.init();

        for (volcanoes) |volcano| {
            const hazard = calculateVolcanicHazard(
                &volcano,
                point.x,
                point.y,
                wind_direction,
                wind_speed,
                0.1, // Default slope
                5.0, // Default rainfall
                2000.0, // Default channel distance
                1.0, // Default atmospheric stability
            );

            // Combine hazards from multiple volcanoes
            combined_hazard.pyroclastic_flow_hazard = @max(combined_hazard.pyroclastic_flow_hazard, hazard.pyroclastic_flow_hazard);
            combined_hazard.ash_fall_hazard += hazard.ash_fall_hazard; // Ash can accumulate from multiple sources
            combined_hazard.lava_flow_hazard = @max(combined_hazard.lava_flow_hazard, hazard.lava_flow_hazard);
            combined_hazard.lahar_hazard = @max(combined_hazard.lahar_hazard, hazard.lahar_hazard);
            combined_hazard.gas_hazard = @max(combined_hazard.gas_hazard, hazard.gas_hazard);
        }

        // Clamp accumulated hazards
        combined_hazard.ash_fall_hazard = @min(1.0, combined_hazard.ash_fall_hazard);
        combined_hazard.calculateCombined();

        hazards[i] = combined_hazard;
    }
}

/// Calculate volcanic influence on elevation (constructive volcanism)
pub fn calculateVolcanicElevationContribution(
    volcano: *const Volcano,
    target_x: f64,
    target_y: f64,
    age_million_years: f64,
) f64 {
    const dx = target_x - volcano.x;
    const dy = target_y - volcano.y;
    const distance = @sqrt(dx * dx + dy * dy);

    // Volcanic edifice size based on VEI and age
    const edifice_radius = switch (volcano.vei_scale) {
        0...2 => 5000.0, // 5 km radius
        3...4 => 15000.0, // 15 km radius
        5...6 => 30000.0, // 30 km radius
        else => 50000.0, // 50 km radius
    };

    if (distance > edifice_radius) return 0.0;

    // Height contribution decreases with distance and age
    const max_height = volcano.elevation * 0.3; // Volcano contributes up to 30% of its elevation to surroundings
    const distance_factor = @exp(-distance * distance / (edifice_radius * edifice_radius * 0.5));
    const age_factor = @exp(-age_million_years / 10.0); // Erosion over time

    return max_height * distance_factor * age_factor;
}

/// Update all volcanoes in a region
pub fn updateVolcanoRegion(
    volcanoes: []Volcano,
    dt: f64,
    regional_stress: f64,
    current_time: f64,
) void {
    for (volcanoes) |*volcano| {
        volcano.updateEruptionProbability(current_time, regional_stress);

        // Simple aging
        volcano.last_eruption_years_ago += dt;
    }
}
