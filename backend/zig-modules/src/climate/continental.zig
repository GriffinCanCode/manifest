//! Continental Climate Effects
//!
//! SIMD-optimized calculations for continental vs oceanic climate effects.
//! Handles temperature amplification and humidity reduction inland.

const std = @import("std");

const precise = @import("../math/precise.zig");
const simd = @import("../simd/simd.zig");

/// Continental effect parameters
pub const ContinentalParams = struct {
    temperature_amplification: f32, // Continental temperature amplification factor
    humidity_reduction: f32, // Continental humidity reduction factor
    world_width: f32, // World width for edge distance calculations
    world_height: f32, // World height for edge distance calculations
};

/// Calculate ocean proximity (0.0 = continental, 1.0 = oceanic)
pub fn calculateOceanProximity(x: f32, y: f32, params: ContinentalParams) f32 {
    // Calculate distance to nearest edge (simplified ocean proximity)
    const edge_dist_x = precise.detMin(precise.detDiv(x, params.world_width), precise.detDiv(precise.detSub(params.world_width, x), params.world_width));
    const edge_dist_y = precise.detMin(precise.detDiv(y, params.world_height), precise.detDiv(precise.detSub(params.world_height, y), params.world_height));

    const edge_distance = precise.detMin(edge_dist_x, edge_dist_y);

    // Convert to ocean proximity (inverse of distance from edge)
    const proximity = precise.detSub(1.0, precise.detMul(edge_distance, 2.0));
    return precise.detClamp(proximity, 0.0, 1.0);
}

/// Calculate continentality (inverse of ocean proximity)
pub fn calculateContinentality(x: f32, y: f32, params: ContinentalParams) f32 {
    return precise.detSub(1.0, calculateOceanProximity(x, y, params));
}

/// Apply continental effect to temperature
pub fn applyTemperatureEffect(base_temp: i8, continentality: f32, params: ContinentalParams) i8 {
    const continental_effect = precise.detMul(continentality, params.temperature_amplification);

    // Continental areas have more extreme temperatures
    const temp_modifier = if (base_temp > 10)
        precise.detMul(continental_effect, 5.0) // Hotter summers
    else
        precise.detMul(-continental_effect, 8.0); // Colder winters

    const new_temp = precise.detAdd(@as(f32, @floatFromInt(base_temp)), temp_modifier);
    return @as(i8, @intFromFloat(precise.detClamp(new_temp, -50.0, 50.0)));
}

/// Apply continental effect to humidity
pub fn applyHumidityEffect(base_humidity: u8, continentality: f32, params: ContinentalParams) u8 {
    const humidity_reduction = precise.detMul(precise.detMul(continentality, params.humidity_reduction), 20.0);
    const new_humidity = precise.detSub(@as(f32, @floatFromInt(base_humidity)), humidity_reduction);
    return @as(u8, @intFromFloat(precise.detClamp(new_humidity, 0.0, 100.0)));
}

/// SIMD batch calculation of ocean proximity for multiple positions
pub fn batchOceanProximity(
    positions: [][2]f32,
    params: ContinentalParams,
    results: []f32,
) void {
    std.debug.assert(positions.len == results.len);

    const len = positions.len;
    const simd_len = len / 4;

    // SIMD parameters
    const world_width_vec = [4]f32{ params.world_width, params.world_width, params.world_width, params.world_width };
    const world_height_vec = [4]f32{ params.world_height, params.world_height, params.world_height, params.world_height };
    const one_vec = [4]f32{ 1.0, 1.0, 1.0, 1.0 };
    const two_vec = [4]f32{ 2.0, 2.0, 2.0, 2.0 };
    const zero_vec = [4]f32{ 0.0, 0.0, 0.0, 0.0 };

    // Process 4 positions at a time
    var i: usize = 0;
    while (i < simd_len * 4) : (i += 4) {
        // Load positions
        const x_vals = [4]f32{ positions[i][0], positions[i + 1][0], positions[i + 2][0], positions[i + 3][0] };
        const y_vals = [4]f32{ positions[i][1], positions[i + 1][1], positions[i + 2][1], positions[i + 3][1] };

        // Calculate edge distances for X
        const x_from_left = simd.divVec4(x_vals, world_width_vec);
        const x_from_right = simd.divVec4(simd.subVec4(world_width_vec, x_vals), world_width_vec);
        const edge_dist_x = simd.minVec4(x_from_left, x_from_right);

        // Calculate edge distances for Y
        const y_from_top = simd.divVec4(y_vals, world_height_vec);
        const y_from_bottom = simd.divVec4(simd.subVec4(world_height_vec, y_vals), world_height_vec);
        const edge_dist_y = simd.minVec4(y_from_top, y_from_bottom);

        // Get minimum edge distance
        const edge_distance = simd.minVec4(edge_dist_x, edge_dist_y);

        // Calculate ocean proximity
        const proximity = simd.subVec4(one_vec, simd.mulVec4(edge_distance, two_vec));
        const clamped_proximity = simd.maxVec4(zero_vec, simd.minVec4(proximity, one_vec));

        // Store results
        results[i] = clamped_proximity[0];
        results[i + 1] = clamped_proximity[1];
        results[i + 2] = clamped_proximity[2];
        results[i + 3] = clamped_proximity[3];
    }

    // Handle remaining elements
    while (i < len) : (i += 1) {
        results[i] = calculateOceanProximity(positions[i][0], positions[i][1], params);
    }
}

/// SIMD batch calculation of continental effects on temperature and humidity
pub fn batchContinentalEffects(
    positions: [][2]f32,
    base_temperatures: []i8,
    base_humidity: []u8,
    params: ContinentalParams,
    temperature_results: []i8,
    humidity_results: []u8,
) void {
    std.debug.assert(positions.len == base_temperatures.len);
    std.debug.assert(positions.len == base_humidity.len);
    std.debug.assert(positions.len == temperature_results.len);
    std.debug.assert(positions.len == humidity_results.len);

    const len = positions.len;

    // First calculate continentality for all positions
    var continentality = std.ArrayList(f32).initCapacity(std.heap.page_allocator, len) catch unreachable;
    defer continentality.deinit();
    continentality.resize(len) catch unreachable;

    for (positions, continentality.items) |pos, *cont| {
        cont.* = calculateContinentality(pos[0], pos[1], params);
    }

    // Apply continental effects
    const simd_len = len / 4;
    var i: usize = 0;

    while (i < simd_len * 4) : (i += 4) {
        // Load data
        const continentality_vals = [4]f32{ continentality.items[i], continentality.items[i + 1], continentality.items[i + 2], continentality.items[i + 3] };

        const base_temps = [4]f32{ @as(f32, @floatFromInt(base_temperatures[i])), @as(f32, @floatFromInt(base_temperatures[i + 1])), @as(f32, @floatFromInt(base_temperatures[i + 2])), @as(f32, @floatFromInt(base_temperatures[i + 3])) };

        const base_hum = [4]f32{ @as(f32, @floatFromInt(base_humidity[i])), @as(f32, @floatFromInt(base_humidity[i + 1])), @as(f32, @floatFromInt(base_humidity[i + 2])), @as(f32, @floatFromInt(base_humidity[i + 3])) };

        // Calculate continental effects
        const temp_amp_vec = [4]f32{ params.temperature_amplification, params.temperature_amplification, params.temperature_amplification, params.temperature_amplification };
        const continental_effects = simd.mulVec4(continentality_vals, temp_amp_vec);

        // Apply temperature effects
        for (0..4) |j| {
            const idx = i + j;
            const continental_effect = continental_effects[j];

            // Continental areas have more extreme temperatures
            const temp_modifier = if (base_temperatures[idx] > 10)
                precise.detMul(continental_effect, 5.0) // Hotter summers
            else
                precise.detMul(-continental_effect, 8.0); // Colder winters

            const new_temp = precise.detAdd(base_temps[j], temp_modifier);
            temperature_results[idx] = @as(i8, @intFromFloat(precise.detClamp(new_temp, -50.0, 50.0)));

            // Apply humidity effects
            const humidity_reduction = precise.detMul(precise.detMul(continental_effect, params.humidity_reduction), 20.0);
            const new_humidity = precise.detSub(base_hum[j], humidity_reduction);
            humidity_results[idx] = @as(u8, @intFromFloat(precise.detClamp(new_humidity, 0.0, 100.0)));
        }
    }

    // Handle remaining elements
    while (i < len) : (i += 1) {
        const continentality_val = continentality.items[i];
        temperature_results[i] = applyTemperatureEffect(base_temperatures[i], continentality_val, params);
        humidity_results[i] = applyHumidityEffect(base_humidity[i], continentality_val, params);
    }
}

/// Calculate maritime influence (smooth transition from continental to oceanic)
pub fn calculateMaritimeInfluence(
    positions: [][2]f32,
    params: ContinentalParams,
    results: []f32,
) void {
    std.debug.assert(positions.len == results.len);

    for (positions, results) |pos, *result| {
        const ocean_proximity = calculateOceanProximity(pos[0], pos[1], params);

        // Smooth maritime influence curve (stronger near coasts)
        const maritime_influence = precise.detMul(ocean_proximity, precise.detAdd(1.0, precise.detMul(ocean_proximity, 0.5)));

        result.* = maritime_influence;
    }
}

// Tests
test "ocean proximity calculation" {
    const testing = std.testing;

    const params = ContinentalParams{
        .temperature_amplification = 1.5,
        .humidity_reduction = 0.8,
        .world_width = 256.0,
        .world_height = 256.0,
    };

    // Test coastal position
    const coastal_proximity = calculateOceanProximity(10.0, 10.0, params);
    try testing.expect(coastal_proximity > 0.5);

    // Test inland position
    const inland_proximity = calculateOceanProximity(128.0, 128.0, params);
    try testing.expect(inland_proximity < 0.5);

    // Coastal should be more oceanic than inland
    try testing.expect(coastal_proximity > inland_proximity);
}

test "continental temperature effects" {
    const testing = std.testing;

    const params = ContinentalParams{
        .temperature_amplification = 1.5,
        .humidity_reduction = 0.8,
        .world_width = 256.0,
        .world_height = 256.0,
    };

    // Test continental effect on hot temperature
    const continental_hot = applyTemperatureEffect(25, 0.8, params);
    try testing.expect(continental_hot > 25); // Should amplify heat

    // Test continental effect on cold temperature
    const continental_cold = applyTemperatureEffect(5, 0.8, params);
    try testing.expect(continental_cold < 5); // Should amplify cold
}

test "batch continental calculations" {
    const testing = std.testing;

    const positions = [_][2]f32{ .{ 10.0, 10.0 }, .{ 128.0, 128.0 }, .{ 200.0, 200.0 }, .{ 50.0, 50.0 } };
    const base_temps = [_]i8{ 20, 20, 20, 20 };
    const base_hum = [_]u8{ 60, 60, 60, 60 };
    var temp_results = [_]i8{ 0, 0, 0, 0 };
    var hum_results = [_]u8{ 0, 0, 0, 0 };

    const params = ContinentalParams{
        .temperature_amplification = 1.5,
        .humidity_reduction = 0.8,
        .world_width = 256.0,
        .world_height = 256.0,
    };

    batchContinentalEffects(positions[0..], base_temps[0..], base_hum[0..], params, temp_results[0..], hum_results[0..]);

    // Results should vary based on continentality
    try testing.expect(temp_results[0] != temp_results[1]); // Coastal vs inland
    try testing.expect(hum_results[0] != hum_results[1]); // Different humidity effects
}
