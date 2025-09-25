//! Climate Interpolation
//!
//! SIMD-optimized climate smoothing and interpolation algorithms.
//! Used for creating smooth climate transitions between adjacent tiles.

const std = @import("std");

const precise = @import("../math/precise.zig");
const simd = @import("../simd/simd.zig");

/// Climate data structure for interpolation
pub const ClimateData = struct {
    temperature: f32,
    rainfall: f32,
    humidity: f32,
    wind_strength: f32,
};

/// Interpolation parameters
pub const InterpolationParams = struct {
    temperature_weight: f32, // Weight for temperature smoothing
    rainfall_weight: f32, // Weight for rainfall smoothing
    humidity_weight: f32, // Weight for humidity smoothing
    wind_weight: f32, // Weight for wind smoothing
    distance_falloff: f32, // Distance falloff factor
    max_influence_distance: f32, // Maximum distance for influence
};

/// Calculate weighted average of climate values
pub fn weightedAverage(
    center_climate: ClimateData,
    neighbor_climates: []ClimateData,
    weights: []f32,
    params: InterpolationParams,
) ClimateData {
    std.debug.assert(neighbor_climates.len == weights.len);

    var total_weight: f32 = 1.0; // Center always has weight 1
    var temp_sum = center_climate.temperature;
    var rain_sum = center_climate.rainfall;
    var hum_sum = center_climate.humidity;
    var wind_sum = center_climate.wind_strength;

    for (neighbor_climates, weights) |neighbor, weight| {
        const adjusted_temp_weight = precise.detMul(weight, params.temperature_weight);
        const adjusted_rain_weight = precise.detMul(weight, params.rainfall_weight);
        const adjusted_hum_weight = precise.detMul(weight, params.humidity_weight);
        const adjusted_wind_weight = precise.detMul(weight, params.wind_weight);

        temp_sum = precise.detAdd(temp_sum, precise.detMul(neighbor.temperature, adjusted_temp_weight));
        rain_sum = precise.detAdd(rain_sum, precise.detMul(neighbor.rainfall, adjusted_rain_weight));
        hum_sum = precise.detAdd(hum_sum, precise.detMul(neighbor.humidity, adjusted_hum_weight));
        wind_sum = precise.detAdd(wind_sum, precise.detMul(neighbor.wind_strength, adjusted_wind_weight));

        total_weight = precise.detAdd(total_weight, weight);
    }

    // Normalize by total weight
    const inv_weight = precise.detDiv(1.0, total_weight);

    return ClimateData{
        .temperature = precise.detMul(temp_sum, inv_weight),
        .rainfall = precise.detMul(rain_sum, inv_weight),
        .humidity = precise.detMul(hum_sum, inv_weight),
        .wind_strength = precise.detMul(wind_sum, inv_weight),
    };
}

/// Calculate distance-based weight for interpolation
pub fn calculateDistanceWeight(distance: f32, params: InterpolationParams) f32 {
    if (distance > params.max_influence_distance) {
        return 0.0;
    }

    // Exponential falloff: weight = e^(-distance * falloff_factor)
    const falloff_distance = precise.detMul(distance, params.distance_falloff);
    return precise.detExp(precise.detNeg(falloff_distance));
}

/// SIMD batch interpolation of climate data
pub fn batchInterpolateClimate(
    center_positions: [][2]f32,
    center_climates: []ClimateData,
    neighbor_positions: [][2]f32,
    neighbor_climates: []ClimateData,
    neighbor_counts: []u32, // Number of neighbors for each center
    neighbor_offsets: []u32, // Starting index in neighbor arrays for each center
    params: InterpolationParams,
    results: []ClimateData,
) void {
    std.debug.assert(center_positions.len == center_climates.len);
    std.debug.assert(center_positions.len == results.len);
    std.debug.assert(center_positions.len == neighbor_counts.len);
    std.debug.assert(center_positions.len == neighbor_offsets.len);

    for (center_positions, center_climates, neighbor_counts, neighbor_offsets, results) |center_pos, center_climate, neighbor_count, neighbor_offset, *result| {
        if (neighbor_count == 0) {
            result.* = center_climate;
            continue;
        }

        // Get neighbors for this center
        const start_idx = neighbor_offset;
        const end_idx = start_idx + neighbor_count;
        const current_neighbor_positions = neighbor_positions[start_idx..end_idx];
        const current_neighbor_climates = neighbor_climates[start_idx..end_idx];

        // Calculate weights based on distance
        var weights = std.ArrayList(f32).initCapacity(std.heap.page_allocator, neighbor_count) catch unreachable;
        defer weights.deinit();
        weights.resize(neighbor_count) catch unreachable;

        for (current_neighbor_positions, weights.items) |neighbor_pos, *weight| {
            const dx = precise.detSub(center_pos[0], neighbor_pos[0]);
            const dy = precise.detSub(center_pos[1], neighbor_pos[1]);
            const distance = precise.detSqrt(precise.detAdd(precise.detMul(dx, dx), precise.detMul(dy, dy)));

            weight.* = calculateDistanceWeight(distance, params);
        }

        // Perform weighted interpolation
        result.* = weightedAverage(center_climate, current_neighbor_climates, weights.items, params);
    }
}

/// Gaussian smoothing filter for climate data
pub fn gaussianSmoothing(
    positions: [][2]f32,
    climates: []ClimateData,
    kernel_size: u32,
    sigma: f32,
    results: []ClimateData,
) void {
    std.debug.assert(positions.len == climates.len);
    std.debug.assert(positions.len == results.len);

    const kernel_radius = @as(i32, @intCast(kernel_size)) / 2;

    for (positions, climates, results, 0..) |center_pos, center_climate, *result, i| {
        var total_weight: f32 = 0.0;
        var weighted_temp: f32 = 0.0;
        var weighted_rain: f32 = 0.0;
        var weighted_hum: f32 = 0.0;
        var weighted_wind: f32 = 0.0;

        // Sample neighboring positions
        for (positions, climates, 0..) |neighbor_pos, neighbor_climate, j| {
            if (i == j) continue;

            const dx = precise.detSub(center_pos[0], neighbor_pos[0]);
            const dy = precise.detSub(center_pos[1], neighbor_pos[1]);
            const distance_sq = precise.detAdd(precise.detMul(dx, dx), precise.detMul(dy, dy));

            // Skip neighbors too far away
            const max_distance_sq = precise.detMul(@as(f32, @floatFromInt(kernel_radius)), @as(f32, @floatFromInt(kernel_radius)));
            if (distance_sq > max_distance_sq) continue;

            // Calculate Gaussian weight
            const sigma_sq = precise.detMul(sigma, sigma);
            const exponent = precise.detDiv(distance_sq, precise.detMul(-2.0, sigma_sq));
            const weight = precise.detExp(exponent);

            total_weight = precise.detAdd(total_weight, weight);
            weighted_temp = precise.detAdd(weighted_temp, precise.detMul(neighbor_climate.temperature, weight));
            weighted_rain = precise.detAdd(weighted_rain, precise.detMul(neighbor_climate.rainfall, weight));
            weighted_hum = precise.detAdd(weighted_hum, precise.detMul(neighbor_climate.humidity, weight));
            weighted_wind = precise.detAdd(weighted_wind, precise.detMul(neighbor_climate.wind_strength, weight));
        }

        if (total_weight > 0.0) {
            const inv_weight = precise.detDiv(1.0, total_weight);
            result.* = ClimateData{
                .temperature = precise.detMul(weighted_temp, inv_weight),
                .rainfall = precise.detMul(weighted_rain, inv_weight),
                .humidity = precise.detMul(weighted_hum, inv_weight),
                .wind_strength = precise.detMul(weighted_wind, inv_weight),
            };
        } else {
            result.* = center_climate; // No neighbors found, keep original
        }
    }
}

/// Bilinear interpolation for climate grids
pub fn bilinearInterpolation(
    grid_width: u32,
    grid_height: u32,
    climate_grid: []ClimateData,
    query_x: f32,
    query_y: f32,
) ClimateData {
    std.debug.assert(climate_grid.len == grid_width * grid_height);

    // Clamp coordinates to grid bounds
    const x = precise.detClamp(query_x, 0.0, @as(f32, @floatFromInt(grid_width - 1)));
    const y = precise.detClamp(query_y, 0.0, @as(f32, @floatFromInt(grid_height - 1)));

    // Get integer coordinates
    const x0 = @as(u32, @intFromFloat(@floor(x)));
    const y0 = @as(u32, @intFromFloat(@floor(y)));
    const x1 = @min(x0 + 1, grid_width - 1);
    const y1 = @min(y0 + 1, grid_height - 1);

    // Get fractional parts
    const fx = precise.detSub(x, @as(f32, @floatFromInt(x0)));
    const fy = precise.detSub(y, @as(f32, @floatFromInt(y0)));

    // Get corner values
    const c00 = climate_grid[y0 * grid_width + x0];
    const c10 = climate_grid[y0 * grid_width + x1];
    const c01 = climate_grid[y1 * grid_width + x0];
    const c11 = climate_grid[y1 * grid_width + x1];

    // Bilinear interpolation
    const temp = precise.detAdd(precise.detAdd(precise.detMul(c00.temperature, precise.detMul(precise.detSub(1.0, fx), precise.detSub(1.0, fy))), precise.detMul(c10.temperature, precise.detMul(fx, precise.detSub(1.0, fy)))), precise.detAdd(precise.detMul(c01.temperature, precise.detMul(precise.detSub(1.0, fx), fy)), precise.detMul(c11.temperature, precise.detMul(fx, fy))));

    const rain = precise.detAdd(precise.detAdd(precise.detMul(c00.rainfall, precise.detMul(precise.detSub(1.0, fx), precise.detSub(1.0, fy))), precise.detMul(c10.rainfall, precise.detMul(fx, precise.detSub(1.0, fy)))), precise.detAdd(precise.detMul(c01.rainfall, precise.detMul(precise.detSub(1.0, fx), fy)), precise.detMul(c11.rainfall, precise.detMul(fx, fy))));

    const hum = precise.detAdd(precise.detAdd(precise.detMul(c00.humidity, precise.detMul(precise.detSub(1.0, fx), precise.detSub(1.0, fy))), precise.detMul(c10.humidity, precise.detMul(fx, precise.detSub(1.0, fy)))), precise.detAdd(precise.detMul(c01.humidity, precise.detMul(precise.detSub(1.0, fx), fy)), precise.detMul(c11.humidity, precise.detMul(fx, fy))));

    const wind = precise.detAdd(precise.detAdd(precise.detMul(c00.wind_strength, precise.detMul(precise.detSub(1.0, fx), precise.detSub(1.0, fy))), precise.detMul(c10.wind_strength, precise.detMul(fx, precise.detSub(1.0, fy)))), precise.detAdd(precise.detMul(c01.wind_strength, precise.detMul(precise.detSub(1.0, fx), fy)), precise.detMul(c11.wind_strength, precise.detMul(fx, fy))));

    return ClimateData{
        .temperature = temp,
        .rainfall = rain,
        .humidity = hum,
        .wind_strength = wind,
    };
}

// Tests
test "weighted average calculation" {
    const testing = std.testing;

    const center = ClimateData{
        .temperature = 20.0,
        .rainfall = 100.0,
        .humidity = 50.0,
        .wind_strength = 10.0,
    };

    const neighbors = [_]ClimateData{
        .{ .temperature = 25.0, .rainfall = 120.0, .humidity = 60.0, .wind_strength = 15.0 },
        .{ .temperature = 15.0, .rainfall = 80.0, .humidity = 40.0, .wind_strength = 5.0 },
    };

    const weights = [_]f32{ 0.5, 0.5 };

    const params = InterpolationParams{
        .temperature_weight = 1.0,
        .rainfall_weight = 1.0,
        .humidity_weight = 1.0,
        .wind_weight = 1.0,
        .distance_falloff = 1.0,
        .max_influence_distance = 10.0,
    };

    const result = weightedAverage(center, neighbors[0..], weights[0..], params);

    // Result should be between center and neighbors
    try testing.expect(result.temperature > 15.0 and result.temperature < 25.0);
    try testing.expect(result.rainfall > 80.0 and result.rainfall < 120.0);
}

test "distance weight calculation" {
    const testing = std.testing;

    const params = InterpolationParams{
        .temperature_weight = 1.0,
        .rainfall_weight = 1.0,
        .humidity_weight = 1.0,
        .wind_weight = 1.0,
        .distance_falloff = 0.5,
        .max_influence_distance = 10.0,
    };

    // Test close distance
    const close_weight = calculateDistanceWeight(1.0, params);
    try testing.expect(close_weight > 0.5);

    // Test far distance
    const far_weight = calculateDistanceWeight(15.0, params);
    try testing.expect(far_weight == 0.0); // Beyond max influence

    // Test zero distance
    const zero_weight = calculateDistanceWeight(0.0, params);
    try testing.expect(zero_weight == 1.0); // Should be maximum weight
}

test "bilinear interpolation" {
    const testing = std.testing;

    // Create a 2x2 grid
    const grid = [_]ClimateData{
        .{ .temperature = 10.0, .rainfall = 100.0, .humidity = 50.0, .wind_strength = 5.0 }, // (0,0)
        .{ .temperature = 20.0, .rainfall = 120.0, .humidity = 60.0, .wind_strength = 10.0 }, // (1,0)
        .{ .temperature = 15.0, .rainfall = 110.0, .humidity = 55.0, .wind_strength = 7.0 }, // (0,1)
        .{ .temperature = 25.0, .rainfall = 130.0, .humidity = 65.0, .wind_strength = 12.0 }, // (1,1)
    };

    // Test center interpolation
    const center_result = bilinearInterpolation(2, 2, grid[0..], 0.5, 0.5);
    try testing.expect(center_result.temperature == 17.5); // Average of all corners
}
