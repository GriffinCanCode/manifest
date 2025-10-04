//! Zig SIMD optimizations for Manifest Game Engine
//!
//! Provides deterministic high-performance math operations for cross-platform
//! reproducible game simulations with SIMD acceleration.

const std = @import("std");

pub const climate = @import("climate/mod.zig");
pub const culling = @import("culling/mod.zig");
pub const hydrology = @import("hydrology/mod.zig");
pub const math = @import("math/mod.zig");
pub const hex = math.hex;
pub const precise = math.precise;
pub const noise = @import("noise/mod.zig");
pub const simd = @import("simd/mod.zig");
pub const tectonics = @import("tectonics/mod.zig");
pub const geometry = tectonics.geometry;
pub const plates = tectonics.plates;
pub const stress = tectonics.stress;
pub const volcanic = tectonics.volcanic;

// Legacy compatibility - these will still work but are deprecated
// Individual module access is now available through the main modules above
// Tectonics modules
// Export main modules
// C exports for Rust FFI
pub export fn manifest_det_add_f32(a: f32, b: f32) f32 {
    return math.precise.detAdd(a, b);
}

pub export fn manifest_det_mul_f32(a: f32, b: f32) f32 {
    return math.precise.detMul(a, b);
}

pub export fn manifest_det_div_f32(a: f32, b: f32) f32 {
    return math.precise.detDiv(a, b);
}

export fn manifest_det_sqrt_f32(a: f32) f32 {
    return math.precise.detSqrt(a);
}

// SIMD vector operations
pub export fn manifest_simd_add_4_f32(a: *const f32, b: *const f32, result: *f32) void {
    const a_arr: *const [4]f32 = @ptrCast(a);
    const b_arr: *const [4]f32 = @ptrCast(b);
    const result_arr: *[4]f32 = @ptrCast(result);
    result_arr.* = simd.addVec4(a_arr.*, b_arr.*);
}

export fn manifest_simd_mul_4_f32(a: *const f32, b: *const f32, result: *f32) void {
    const a_arr: *const [4]f32 = @ptrCast(a);
    const b_arr: *const [4]f32 = @ptrCast(b);
    const result_arr: *[4]f32 = @ptrCast(result);
    result_arr.* = simd.mulVec4(a_arr.*, b_arr.*);
}

export fn manifest_simd_dot_4_f32(a: *const f32, b: *const f32) f32 {
    const a_arr: *const [4]f32 = @ptrCast(a);
    const b_arr: *const [4]f32 = @ptrCast(b);
    return simd.dotVec4(a_arr.*, b_arr.*);
}

// Hex grid operations
pub export fn manifest_hex_distance(q1: i32, r1: i32, q2: i32, r2: i32) u32 {
    return math.hex.distance(q1, r1, q2, r2);
}

pub export fn manifest_hex_to_pixel(q: i32, r: i32, size: f32, x: *f32, y: *f32) void {
    const pos = math.hex.toPixel(q, r, size);
    x.* = pos.x;
    y.* = pos.y;
}

pub export fn manifest_hex_from_pixel(x: f32, y: f32, size: f32, q: *i32, r: *i32) void {
    const coord = math.hex.fromPixel(x, y, size);
    q.* = coord.q;
    r.* = coord.r;
}

pub export fn manifest_hex_get_neighbors(q: i32, r: i32, neighbors: *[6]math.hex.HexCoord) void {
    const coord = math.hex.HexCoord.init(q, r);
    const result = math.hex.getNeighbors(coord);
    neighbors.* = result;
}

export fn manifest_hex_get_neighbor(q: i32, r: i32, direction: u8, out_q: *i32, out_r: *i32) void {
    const coord = math.hex.HexCoord.init(q, r);
    const neighbor = math.hex.getNeighbor(coord, @intCast(direction));
    out_q.* = neighbor.q;
    out_r.* = neighbor.r;
}

export fn manifest_hex_batch_to_pixel(coords: [*]const math.hex.HexCoord, size: f32, pixels: [*]math.hex.PixelPos, count: usize) void {
    const coord_slice = coords[0..count];
    const pixel_slice = pixels[0..count];
    math.hex.batchToPixel(coord_slice, size, pixel_slice);
}

export fn manifest_hex_round_to_hex(q_f: f32, r_f: f32, q: *i32, r: *i32) void {
    const coord = math.hex.roundToHex(q_f, r_f);
    q.* = coord.q;
    r.* = coord.r;
}

// Tectonic plate physics calculations
export fn manifest_calculate_ridge_push(plate_center_x: f64, plate_center_y: f64, plate_vel_x: f64, plate_vel_y: f64, age_million_years: f64, area: f64, movement_speed: f64, result_x: *f64, result_y: *f64) void {
    const plate_data = tectonics.plates.TectonicPlate{
        .id = 0,
        .center = tectonics.plates.Vec2.init(plate_center_x, plate_center_y),
        .velocity = tectonics.plates.Vec2.init(plate_vel_x, plate_vel_y),
        .age_million_years = age_million_years,
        .density = 2700.0,
        .area = area,
    };

    const force = tectonics.plates.calculateRidgePush(&plate_data, movement_speed);
    result_x.* = force.x;
    result_y.* = force.y;
}

export fn manifest_calculate_basal_drag(plate_vel_x: f64, plate_vel_y: f64, area: f64, result_x: *f64, result_y: *f64) void {
    const plate_data = tectonics.plates.TectonicPlate{
        .id = 0,
        .center = tectonics.plates.Vec2.init(0.0, 0.0),
        .velocity = tectonics.plates.Vec2.init(plate_vel_x, plate_vel_y),
        .age_million_years = 50.0,
        .density = 2700.0,
        .area = area,
    };

    const force = tectonics.plates.calculateBasalDrag(&plate_data);
    result_x.* = force.x;
    result_y.* = force.y;
}

export fn manifest_calculate_mantle_convection(plate_center_x: f64, plate_center_y: f64, area: f64, movement_speed: f64, result_x: *f64, result_y: *f64) void {
    const plate_data = tectonics.plates.TectonicPlate{
        .id = 0,
        .center = tectonics.plates.Vec2.init(plate_center_x, plate_center_y),
        .velocity = tectonics.plates.Vec2.init(0.0, 0.0),
        .age_million_years = 50.0,
        .density = 2700.0,
        .area = area,
    };

    const force = tectonics.plates.calculateMantelConvection(&plate_data, movement_speed);
    result_x.* = force.x;
    result_y.* = force.y;
}

// Geometric calculations
export fn manifest_point_to_segment_distance(point_x: f64, point_y: f64, seg_start_x: f64, seg_start_y: f64, seg_end_x: f64, seg_end_y: f64) f64 {
    const point = tectonics.geometry.Point2D.init(point_x, point_y);
    const segment = tectonics.geometry.LineSegment.init(tectonics.geometry.Point2D.init(seg_start_x, seg_start_y), tectonics.geometry.Point2D.init(seg_end_x, seg_end_y));

    return tectonics.geometry.pointToSegmentDistance(point, segment);
}

export fn manifest_polygon_contains_point(vertices_x: [*]const f64, vertices_y: [*]const f64, vertex_count: usize, point_x: f64, point_y: f64) bool {
    var vertices: [32]tectonics.geometry.Point2D = undefined; // Stack allocation for small polygons
    const count = @min(vertex_count, 32);

    for (0..count) |i| {
        vertices[i] = tectonics.geometry.Point2D.init(vertices_x[i], vertices_y[i]);
    }

    const polygon = tectonics.geometry.Polygon.init(vertices[0..count]);
    const point = tectonics.geometry.Point2D.init(point_x, point_y);

    return polygon.containsPoint(point);
}

export fn manifest_polygon_area(vertices_x: [*]const f64, vertices_y: [*]const f64, vertex_count: usize) f64 {
    var vertices: [32]tectonics.geometry.Point2D = undefined;
    const count = @min(vertex_count, 32);

    for (0..count) |i| {
        vertices[i] = tectonics.geometry.Point2D.init(vertices_x[i], vertices_y[i]);
    }

    const polygon = tectonics.geometry.Polygon.init(vertices[0..count]);
    return polygon.area();
}

// Stress field calculations
export fn manifest_stress_von_mises(stress_xx: f64, stress_yy: f64, stress_xy: f64) f64 {
    const tensor = tectonics.stress.StressTensor.init(stress_xx, stress_yy, stress_xy);
    return tensor.vonMisesStress();
}

export fn manifest_stress_max_principal(stress_xx: f64, stress_yy: f64, stress_xy: f64) f64 {
    const tensor = tectonics.stress.StressTensor.init(stress_xx, stress_yy, stress_xy);
    return tensor.maxPrincipalStress();
}

export fn manifest_stress_principal_angle(stress_xx: f64, stress_yy: f64, stress_xy: f64) f64 {
    const tensor = tectonics.stress.StressTensor.init(stress_xx, stress_yy, stress_xy);
    return tensor.principalStressAngle();
}

// Volcanic hazard calculations
export fn manifest_volcanic_pyroclastic_hazard(volcano_x: f64, volcano_y: f64, vei_scale: u32, hazard_radius: f64, target_x: f64, target_y: f64, wind_direction: f64, wind_speed: f64) f64 {
    const volcano_data = tectonics.volcanic.Volcano{
        .x = volcano_x,
        .y = volcano_y,
        .elevation = 2000.0,
        .vei_scale = vei_scale,
        .hazard_radius = hazard_radius,
        .magma_chamber_depth = 10000.0,
        .last_eruption_years_ago = 100.0,
        .eruption_probability = 0.5,
    };

    return tectonics.volcanic.calculatePyroclasticFlowHazard(&volcano_data, target_x, target_y, wind_direction, wind_speed);
}

export fn manifest_volcanic_ash_hazard(volcano_x: f64, volcano_y: f64, vei_scale: u32, target_x: f64, target_y: f64, wind_direction: f64, wind_speed: f64, column_height: f64) f64 {
    const volcano_data = tectonics.volcanic.Volcano{
        .x = volcano_x,
        .y = volcano_y,
        .elevation = 2000.0,
        .vei_scale = vei_scale,
        .hazard_radius = 50000.0,
        .magma_chamber_depth = 10000.0,
        .last_eruption_years_ago = 100.0,
        .eruption_probability = 0.5,
    };

    return tectonics.volcanic.calculateAshFallHazard(&volcano_data, target_x, target_y, wind_direction, wind_speed, column_height);
}

// Climate calculations
export fn manifest_climate_orographic_effects(
    positions_x: [*]const f32,
    positions_y: [*]const f32,
    elevations: [*]const f32,
    wind_directions: [*]const f32,
    max_orographic_bonus: f32,
    rain_shadow_factor: f32,
    count: usize,
    results: [*]f32,
) void {
    var position_data: [256][2]f32 = undefined; // Stack allocation for small batches
    const actual_count = @min(count, 256);

    for (0..actual_count) |i| {
        position_data[i] = .{ positions_x[i], positions_y[i] };
    }

    const params = climate.OrographicParams{
        .max_orographic_bonus = max_orographic_bonus,
        .rain_shadow_factor = rain_shadow_factor,
        .elevation_scale = 1.0,
        .wind_effect_scale = 1.0,
    };

    climate.orographic.batchOrographicEffects(
        position_data[0..actual_count],
        @as([]f32, @constCast(elevations[0..actual_count])),
        @as([]f32, @constCast(wind_directions[0..actual_count])),
        params,
        results[0..actual_count],
    );
}

export fn manifest_climate_continental_effects(
    positions_x: [*]const f32,
    positions_y: [*]const f32,
    base_temperatures: [*]const i8,
    base_humidity: [*]const u8,
    temperature_amplification: f32,
    humidity_reduction: f32,
    world_width: f32,
    world_height: f32,
    count: usize,
    temperature_results: [*]i8,
    humidity_results: [*]u8,
) void {
    var position_data: [256][2]f32 = undefined; // Stack allocation
    const actual_count = @min(count, 256);

    for (0..actual_count) |i| {
        position_data[i] = .{ positions_x[i], positions_y[i] };
    }

    const params = climate.ContinentalParams{
        .temperature_amplification = temperature_amplification,
        .humidity_reduction = humidity_reduction,
        .world_width = world_width,
        .world_height = world_height,
    };

    climate.continental.batchContinentalEffects(
        position_data[0..actual_count],
        @as([]i8, @constCast(base_temperatures[0..actual_count])),
        @as([]u8, @constCast(base_humidity[0..actual_count])),
        params,
        temperature_results[0..actual_count],
        humidity_results[0..actual_count],
    );
}

export fn manifest_climate_seasonal_temperature(
    base_temperatures: [*]const i8,
    climate_zones: [*]const u8, // ClimateZone as u8
    latitudes: [*]const f32,
    current_season: f32,
    temperature_variations: [*]const f32, // Array of 6 values for each climate zone
    count: usize,
    results: [*]i8,
) void {
    const actual_count = @min(count, 256);

    var zone_data: [256]climate.ClimateZone = undefined;
    for (0..actual_count) |i| {
        zone_data[i] = @enumFromInt(climate_zones[i]);
    }

    const seasonal_state = climate.SeasonalState{
        .current_season = current_season,
        .year_progress = current_season,
        .hemisphere_modifier = 1.0,
    };

    var params = climate.SeasonalParams.default();
    // Copy temperature variations
    for (0..6) |i| {
        params.temperature_variation[i] = temperature_variations[i];
    }

    climate.seasonal.batchSeasonalTemperature(
        @as([]i8, @constCast(base_temperatures[0..actual_count])),
        zone_data[0..actual_count],
        @as([]f32, @constCast(latitudes[0..actual_count])),
        seasonal_state,
        params,
        results[0..actual_count],
    );
}

export fn manifest_climate_seasonal_rainfall(
    base_rainfall: [*]const u16,
    climate_zones: [*]const u8,
    latitudes: [*]const f32,
    current_season: f32,
    rainfall_variations: [*]const f32, // Array of 6 values for each climate zone
    count: usize,
    results: [*]u16,
) void {
    const actual_count = @min(count, 256);

    var zone_data: [256]climate.ClimateZone = undefined;
    for (0..actual_count) |i| {
        zone_data[i] = @enumFromInt(climate_zones[i]);
    }

    const seasonal_state = climate.SeasonalState{
        .current_season = current_season,
        .year_progress = current_season,
        .hemisphere_modifier = 1.0,
    };

    var params = climate.SeasonalParams.default();
    // Copy rainfall variations
    for (0..6) |i| {
        params.rainfall_variation[i] = rainfall_variations[i];
    }

    climate.seasonal.batchSeasonalRainfall(
        @as([]u16, @constCast(base_rainfall[0..actual_count])),
        zone_data[0..actual_count],
        @as([]f32, @constCast(latitudes[0..actual_count])),
        seasonal_state,
        params,
        results[0..actual_count],
    );
}

pub export fn manifest_climate_process_all(
    positions_x: [*]const f32,
    positions_y: [*]const f32,
    elevations: [*]const f32,
    base_temperatures: [*]const i8,
    base_rainfall: [*]const f32,
    base_humidity: [*]const u8,
    wind_directions: [*]const f32,
    count: usize,
    temperature_results: [*]i8,
    rainfall_results: [*]f32,
    humidity_results: [*]u8,
) void {
    const actual_count = @min(count, 256);

    var position_data: [256][2]f32 = undefined;
    for (0..actual_count) |i| {
        position_data[i] = .{ positions_x[i], positions_y[i] };
    }

    climate.simpleClimateProcessing(
        position_data[0..actual_count],
        @as([]f32, @constCast(elevations[0..actual_count])),
        @as([]i8, @constCast(base_temperatures[0..actual_count])),
        @as([]f32, @constCast(base_rainfall[0..actual_count])),
        @as([]u8, @constCast(base_humidity[0..actual_count])),
        @as([]f32, @constCast(wind_directions[0..actual_count])),
        temperature_results[0..actual_count],
        rainfall_results[0..actual_count],
        humidity_results[0..actual_count],
    );
}

export fn manifest_climate_ocean_proximity(
    positions_x: [*]const f32,
    positions_y: [*]const f32,
    world_width: f32,
    world_height: f32,
    count: usize,
    results: [*]f32,
) void {
    const actual_count = @min(count, 256);

    var position_data: [256][2]f32 = undefined;
    for (0..actual_count) |i| {
        position_data[i] = .{ positions_x[i], positions_y[i] };
    }

    const params = climate.ContinentalParams{
        .temperature_amplification = 1.0, // Not used for proximity calculation
        .humidity_reduction = 1.0, // Not used for proximity calculation
        .world_width = world_width,
        .world_height = world_height,
    };

    climate.continental.batchOceanProximity(
        position_data[0..actual_count],
        params,
        results[0..actual_count],
    );
}

export fn manifest_climate_rain_shadow(
    positions_x: [*]const f32,
    positions_y: [*]const f32,
    elevations: [*]const f32,
    mountain_centers_x: [*]const f32,
    mountain_centers_y: [*]const f32,
    mountain_widths: [*]const f32,
    mountain_heights: [*]const f32,
    mountain_orientations: [*]const f32,
    wind_direction: f32,
    shadow_factor: f32,
    count: usize,
    mountain_count: usize,
    results: [*]f32,
) void {
    const actual_count = @min(count, 256);
    const actual_mountain_count = @min(mountain_count, 32);

    var position_data: [256][2]f32 = undefined;
    for (0..actual_count) |i| {
        position_data[i] = .{ positions_x[i], positions_y[i] };
    }

    var mountain_data: [32]climate.orographic.MountainRange = undefined;
    for (0..actual_mountain_count) |i| {
        mountain_data[i] = climate.orographic.MountainRange{
            .center_x = mountain_centers_x[i],
            .center_y = mountain_centers_y[i],
            .width = mountain_widths[i],
            .height = mountain_heights[i],
            .orientation = mountain_orientations[i],
        };
    }

    climate.orographic.calculateRainShadowEffect(
        position_data[0..actual_count],
        @as([]f32, @constCast(elevations[0..actual_count])),
        mountain_data[0..actual_mountain_count],
        wind_direction,
        shadow_factor,
        results[0..actual_count],
    );
}

export fn manifest_climate_interpolate_batch(
    center_positions_x: [*]const f32,
    center_positions_y: [*]const f32,
    center_temperatures: [*]const f32,
    center_rainfall: [*]const f32,
    center_humidity: [*]const f32,
    center_wind_strength: [*]const f32,
    neighbor_positions_x: [*]const f32,
    neighbor_positions_y: [*]const f32,
    neighbor_temperatures: [*]const f32,
    neighbor_rainfall: [*]const f32,
    neighbor_humidity: [*]const f32,
    neighbor_wind_strength: [*]const f32,
    neighbor_counts: [*]const u32,
    neighbor_offsets: [*]const u32,
    temperature_weight: f32,
    rainfall_weight: f32,
    humidity_weight: f32,
    wind_weight: f32,
    distance_falloff: f32,
    max_influence_distance: f32,
    center_count: usize,
    neighbor_count: usize,
    result_temperatures: [*]f32,
    result_rainfall: [*]f32,
    result_humidity: [*]f32,
    result_wind_strength: [*]f32,
) void {
    const actual_center_count = @min(center_count, 256);
    const actual_neighbor_count = @min(neighbor_count, 1024);

    var center_pos_data: [256][2]f32 = undefined;
    var center_climate_data: [256]climate.interpolation.ClimateData = undefined;
    var neighbor_pos_data: [1024][2]f32 = undefined;
    var neighbor_climate_data: [1024]climate.interpolation.ClimateData = undefined;
    var results_data: [256]climate.interpolation.ClimateData = undefined;

    // Prepare center data
    for (0..actual_center_count) |i| {
        center_pos_data[i] = .{ center_positions_x[i], center_positions_y[i] };
        center_climate_data[i] = climate.interpolation.ClimateData{
            .temperature = center_temperatures[i],
            .rainfall = center_rainfall[i],
            .humidity = center_humidity[i],
            .wind_strength = center_wind_strength[i],
        };
    }

    // Prepare neighbor data
    for (0..actual_neighbor_count) |i| {
        neighbor_pos_data[i] = .{ neighbor_positions_x[i], neighbor_positions_y[i] };
        neighbor_climate_data[i] = climate.interpolation.ClimateData{
            .temperature = neighbor_temperatures[i],
            .rainfall = neighbor_rainfall[i],
            .humidity = neighbor_humidity[i],
            .wind_strength = neighbor_wind_strength[i],
        };
    }

    const params = climate.interpolation.InterpolationParams{
        .temperature_weight = temperature_weight,
        .rainfall_weight = rainfall_weight,
        .humidity_weight = humidity_weight,
        .wind_weight = wind_weight,
        .distance_falloff = distance_falloff,
        .max_influence_distance = max_influence_distance,
    };

    climate.interpolation.batchInterpolateClimate(
        center_pos_data[0..actual_center_count],
        center_climate_data[0..actual_center_count],
        neighbor_pos_data[0..actual_neighbor_count],
        neighbor_climate_data[0..actual_neighbor_count],
        @as([]u32, @constCast(neighbor_counts[0..actual_center_count])),
        @as([]u32, @constCast(neighbor_offsets[0..actual_center_count])),
        params,
        results_data[0..actual_center_count],
    );

    // Extract results
    for (0..actual_center_count) |i| {
        result_temperatures[i] = results_data[i].temperature;
        result_rainfall[i] = results_data[i].rainfall;
        result_humidity[i] = results_data[i].humidity;
        result_wind_strength[i] = results_data[i].wind_strength;
    }
}

export fn manifest_climate_monsoon_effects(
    latitudes: [*]const f32,
    longitudes: [*]const f32,
    current_season: f32,
    year_progress: f32,
    hemisphere_modifier: f32,
    monsoon_strength: f32,
    count: usize,
    results: [*]f32,
) void {
    const actual_count = @min(count, 256);

    const seasonal_state = climate.seasonal.SeasonalState{
        .current_season = current_season,
        .year_progress = year_progress,
        .hemisphere_modifier = hemisphere_modifier,
    };

    for (0..actual_count) |i| {
        results[i] = climate.seasonal.calculateMonsoonEffect(
            latitudes[i],
            longitudes[i],
            seasonal_state,
            monsoon_strength,
        );
    }
}

export fn manifest_climate_maritime_influence(
    positions_x: [*]const f32,
    positions_y: [*]const f32,
    world_width: f32,
    world_height: f32,
    count: usize,
    results: [*]f32,
) void {
    const actual_count = @min(count, 256);

    var position_data: [256][2]f32 = undefined;
    for (0..actual_count) |i| {
        position_data[i] = .{ positions_x[i], positions_y[i] };
    }

    const params = climate.ContinentalParams{
        .temperature_amplification = 1.5,
        .humidity_reduction = 0.8,
        .world_width = world_width,
        .world_height = world_height,
    };

    climate.continental.calculateMaritimeInfluence(
        position_data[0..actual_count],
        params,
        results[0..actual_count],
    );
}

export fn manifest_climate_gaussian_smoothing(
    positions_x: [*]const f32,
    positions_y: [*]const f32,
    temperatures: [*]const f32,
    rainfall: [*]const f32,
    humidity: [*]const f32,
    wind_strength: [*]const f32,
    kernel_size: u32,
    sigma: f32,
    count: usize,
    result_temperatures: [*]f32,
    result_rainfall: [*]f32,
    result_humidity: [*]f32,
    result_wind_strength: [*]f32,
) void {
    const actual_count = @min(count, 256);

    var position_data: [256][2]f32 = undefined;
    var climate_data: [256]climate.interpolation.ClimateData = undefined;
    var results_data: [256]climate.interpolation.ClimateData = undefined;

    for (0..actual_count) |i| {
        position_data[i] = .{ positions_x[i], positions_y[i] };
        climate_data[i] = climate.interpolation.ClimateData{
            .temperature = temperatures[i],
            .rainfall = rainfall[i],
            .humidity = humidity[i],
            .wind_strength = wind_strength[i],
        };
    }

    climate.interpolation.gaussianSmoothing(
        position_data[0..actual_count],
        climate_data[0..actual_count],
        kernel_size,
        sigma,
        results_data[0..actual_count],
    );

    // Extract results
    for (0..actual_count) |i| {
        result_temperatures[i] = results_data[i].temperature;
        result_rainfall[i] = results_data[i].rainfall;
        result_humidity[i] = results_data[i].humidity;
        result_wind_strength[i] = results_data[i].wind_strength;
    }
}

// Batch distance calculations
export fn manifest_batch_plate_distances(plates_x: [*]const f64, plates_y: [*]const f64, plate_count: usize, distances: [*]f64) void {
    var plate_data: [64]tectonics.plates.TectonicPlate = undefined; // Stack allocation
    const count = @min(plate_count, 64);

    for (0..count) |i| {
        plate_data[i] = tectonics.plates.TectonicPlate{
            .id = @intCast(i),
            .center = tectonics.plates.Vec2.init(plates_x[i], plates_y[i]),
            .velocity = tectonics.plates.Vec2.init(0.0, 0.0),
            .age_million_years = 50.0,
            .density = 2700.0,
            .area = 1000000.0,
        };
    }

    tectonics.plates.batchDistanceCalculations(plate_data[0..count], distances[0 .. count * count]);
}

// Hydrology FFI exports
export fn manifest_hydraulics_manning(
    area: f64,
    wetted_perimeter: f64,
    slope: f64,
    manning_n: f64,
    velocity_result: *f64,
    discharge_result: *f64,
    hydraulic_radius_result: *f64,
) void {
    const results = hydrology.hydraulics.calculateManning(area, wetted_perimeter, slope, manning_n);
    velocity_result.* = results.velocity;
    discharge_result.* = results.discharge;
    hydraulic_radius_result.* = results.hydraulic_radius;
}

export fn manifest_hydraulics_critical_depth(discharge: f64, width: f64, gravity: f64) f64 {
    return hydrology.hydraulics.calculateCriticalDepth(discharge, width, gravity);
}

export fn manifest_hydraulics_normal_depth(
    discharge: f64,
    width: f64,
    slope: f64,
    manning_n: f64,
    channel_type: u8,
    side_slope: f64,
) f64 {
    const channel_enum: hydrology.hydraulics.ChannelType = @enumFromInt(channel_type);
    return hydrology.hydraulics.calculateNormalDepth(discharge, width, slope, manning_n, channel_enum, side_slope);
}

export fn manifest_hydraulics_froude_number(velocity: f64, depth: f64) f64 {
    return hydrology.hydraulics.calculateFroude(velocity, depth);
}

export fn manifest_hydraulics_reynolds_number(
    velocity: f64,
    hydraulic_radius: f64,
    kinematic_viscosity: f64,
) f64 {
    return hydrology.hydraulics.calculateReynolds(velocity, hydraulic_radius, kinematic_viscosity);
}

export fn manifest_aquifer_darcy_velocity(
    hydraulic_conductivity: f64,
    head_gradient_x: f64,
    head_gradient_y: f64,
    velocity_x: *f64,
    velocity_y: *f64,
    magnitude: *f64,
) void {
    const cell_data = hydrology.aquifers.AquiferCell.init(0.0, hydraulic_conductivity, 10.0, 0.3, .unconfined);
    const flow_vector = cell_data.calculateDarcyVelocity(head_gradient_x, head_gradient_y);
    velocity_x.* = flow_vector.velocity_x;
    velocity_y.* = flow_vector.velocity_y;
    magnitude.* = flow_vector.magnitude;
}

export fn manifest_aquifer_seepage_velocity(
    darcy_velocity_x: f64,
    darcy_velocity_y: f64,
    porosity: f64,
    seepage_x: *f64,
    seepage_y: *f64,
) void {
    const cell_data = hydrology.aquifers.AquiferCell.init(0.0, 1e-5, 10.0, porosity, .unconfined);
    const darcy_flow = hydrology.aquifers.FlowVector.init(darcy_velocity_x, darcy_velocity_y);
    const seepage_flow = cell_data.calculateSeepageVelocity(darcy_flow);
    seepage_x.* = seepage_flow.velocity_x;
    seepage_y.* = seepage_flow.velocity_y;
}

export fn manifest_aquifer_theis_solution(
    distance: f64,
    time: f64,
    pumping_rate: f64,
    transmissivity: f64,
    storativity: f64,
) f64 {
    return hydrology.aquifers.calculateTheisSolution(distance, time, pumping_rate, transmissivity, storativity);
}

export fn manifest_spring_discharge(head_difference: f64, aquifer_type: u8) f64 {
    const aquifer_enum: hydrology.aquifers.AquiferType = @enumFromInt(aquifer_type);
    return hydrology.aquifers.calculateSpringDischarge(head_difference, aquifer_enum);
}

export fn manifest_spring_seasonal_discharge(
    base_discharge: f64,
    seasonal_variation: f64,
    day_of_year: u32,
) f64 {
    const spring_data = hydrology.aquifers.Spring{
        .x = 0.0,
        .y = 0.0,
        .elevation = 100.0,
        .discharge = base_discharge,
        .temperature = 15.0,
        .spring_type = .gravity,
        .aquifer_connection = .unconfined,
        .seasonal_variation = seasonal_variation,
    };
    return spring_data.getSeasonalDischarge(day_of_year);
}

export fn manifest_watershed_time_of_concentration(stream_length: f64, relief: f64) f64 {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    const allocator = gpa.allocator();

    var watershed_data = hydrology.watersheds.Watershed.init(1, 0, 0, 100.0, allocator);
    defer watershed_data.deinit(allocator);

    watershed_data.stream_length = stream_length;
    watershed_data.relief = relief;

    return watershed_data.calculateTimeOfConcentration();
}

pub export fn manifest_batch_manning_calculations(
    areas: [*]const f64,
    wetted_perimeters: [*]const f64,
    slopes: [*]const f64,
    manning_ns: [*]const f64,
    velocities: [*]f64,
    discharges: [*]f64,
    hydraulic_radii: [*]f64,
    count: usize,
) void {
    const actual_count = @min(count, 1024); // Limit for safety

    for (0..actual_count) |i| {
        const results = hydrology.hydraulics.calculateManning(
            areas[i],
            wetted_perimeters[i],
            slopes[i],
            manning_ns[i],
        );
        velocities[i] = results.velocity;
        discharges[i] = results.discharge;
        hydraulic_radii[i] = results.hydraulic_radius;
    }
}

export fn manifest_batch_slope_calculations(
    elevations: [*]const f64,
    width: usize,
    height: usize,
    cell_size: f64,
    slopes: [*]f64,
) void {
    const elevation_slice = elevations[0 .. width * height];
    const slopes_slice = slopes[0 .. width * height];
    hydrology.flow.batchCalculateSlopes(elevation_slice, width, height, cell_size, slopes_slice);
}

export fn manifest_batch_point_distances(
    points1_x: [*]const f64,
    points1_y: [*]const f64,
    points2_x: [*]const f64,
    points2_y: [*]const f64,
    count1: usize,
    count2: usize,
    distances: [*]f64,
) void {
    const max_points = 256;
    const actual_count1 = @min(count1, max_points);
    const actual_count2 = @min(count2, max_points);

    var points1_data: [max_points]tectonics.geometry.Point2D = undefined;
    var points2_data: [max_points]tectonics.geometry.Point2D = undefined;

    for (0..actual_count1) |i| {
        points1_data[i] = tectonics.geometry.Point2D.init(points1_x[i], points1_y[i]);
    }

    for (0..actual_count2) |i| {
        points2_data[i] = tectonics.geometry.Point2D.init(points2_x[i], points2_y[i]);
    }

    tectonics.geometry.batchPointDistances(
        points1_data[0..actual_count1],
        points2_data[0..actual_count2],
        distances[0 .. actual_count1 * actual_count2],
    );
}

// Watershed delineation exports
export fn manifest_watershed_delineate(
    flow_grid: *hydrology.flow.FlowGrid,
    outlet_x: usize,
    outlet_y: usize,
    watershed_id: u32,
    boundary_points_x: [*]f64,
    boundary_points_y: [*]f64,
    boundary_points_elevation: [*]f64,
    max_boundary_points: usize,
    boundary_count: *usize,
    area: *f64,
    perimeter: *f64,
    relief: *f64,
) bool {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    const allocator = gpa.allocator();
    defer _ = gpa.deinit();

    var delineator = hydrology.watersheds.WatershedDelineator.init(flow_grid, allocator) catch return false;
    defer delineator.deinit();

    delineator.delineateWatershed(outlet_x, outlet_y, watershed_id) catch return false;

    // Find the watershed we just created
    for (delineator.watersheds.items) |watershed| {
        if (watershed.id == watershed_id) {
            // Copy boundary points (limited by max_boundary_points)
            const actual_count = @min(watershed.boundary_points.items.len, max_boundary_points);
            boundary_count.* = actual_count;

            for (0..actual_count) |i| {
                boundary_points_x[i] = watershed.boundary_points.items[i].x;
                boundary_points_y[i] = watershed.boundary_points.items[i].y;
                boundary_points_elevation[i] = watershed.boundary_points.items[i].elevation;
            }

            // Set morphometric data
            area.* = watershed.area;
            perimeter.* = watershed.perimeter;
            relief.* = watershed.relief;

            return true;
        }
    }

    return false;
}

export fn manifest_watershed_calculate_morphometrics(
    boundary_points_x: [*]const f64,
    boundary_points_y: [*]const f64,
    boundary_points_elevation: [*]const f64,
    boundary_count: usize,
    area: *f64,
    perimeter: *f64,
    shape_factor: *f64,
    mean_elevation: *f64,
    relief: *f64,
) void {
    if (boundary_count == 0) return;

    // Calculate area using shoelace formula
    var calculated_area: f64 = 0.0;
    for (0..boundary_count) |i| {
        const next_i = (i + 1) % boundary_count;
        calculated_area += boundary_points_x[i] * boundary_points_y[next_i] - boundary_points_x[next_i] * boundary_points_y[i];
    }
    calculated_area = @abs(calculated_area) / 2.0;
    area.* = calculated_area;

    // Calculate perimeter
    var calculated_perimeter: f64 = 0.0;
    for (0..boundary_count) |i| {
        const next_i = (i + 1) % boundary_count;
        const dx = boundary_points_x[next_i] - boundary_points_x[i];
        const dy = boundary_points_y[next_i] - boundary_points_y[i];
        calculated_perimeter += @sqrt(dx * dx + dy * dy);
    }
    perimeter.* = calculated_perimeter;

    // Calculate shape factor (area / perimeter²)
    if (calculated_perimeter > 0.0) {
        shape_factor.* = calculated_area / (calculated_perimeter * calculated_perimeter);
    } else {
        shape_factor.* = 0.0;
    }

    // Calculate elevation statistics
    var total_elevation: f64 = 0.0;
    var min_elevation: f64 = boundary_points_elevation[0];
    var max_elevation: f64 = boundary_points_elevation[0];

    for (0..boundary_count) |i| {
        const elev = boundary_points_elevation[i];
        total_elevation += elev;
        min_elevation = @min(min_elevation, elev);
        max_elevation = @max(max_elevation, elev);
    }

    mean_elevation.* = total_elevation / @as(f64, @floatFromInt(boundary_count));
    relief.* = max_elevation - min_elevation;
}

// River segment calculation exports
export fn manifest_rivers_calculate_segments(
    points_x: [*]const f64,
    points_y: [*]const f64,
    point_count: usize,
    min_segment_length: f64,
    segments_x: [*]f64,
    segments_y: [*]f64,
    segment_lengths: [*]f64,
    max_segments: usize,
    segment_count: *usize,
) void {
    if (point_count < 2) {
        segment_count.* = 0;
        return;
    }

    var current_segment: usize = 0;
    var current_length: f64 = 0.0;
    var segment_start_x = points_x[0];
    var segment_start_y = points_y[0];

    for (1..point_count) |i| {
        const dx = points_x[i] - points_x[i - 1];
        const dy = points_y[i] - points_y[i - 1];
        const distance = @sqrt(dx * dx + dy * dy);
        current_length += distance;

        // If we've accumulated enough length or this is the last point, create a segment
        if (current_length >= min_segment_length or i == point_count - 1) {
            if (current_segment < max_segments) {
                segments_x[current_segment] = (segment_start_x + points_x[i]) / 2.0;
                segments_y[current_segment] = (segment_start_y + points_y[i]) / 2.0;
                segment_lengths[current_segment] = current_length;
                current_segment += 1;
            }

            // Start new segment
            segment_start_x = points_x[i];
            segment_start_y = points_y[i];
            current_length = 0.0;
        }
    }

    segment_count.* = current_segment;
}
