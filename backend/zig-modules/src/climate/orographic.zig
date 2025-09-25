//! Orographic Climate Effects
//!
//! SIMD-optimized calculations for mountain-induced precipitation and rain shadow effects.
//! Used by Rust climate generation for high-performance batch processing.

const std = @import("std");

const precise = @import("../math/precise.zig");
const simd = @import("../simd/simd.zig");

/// Orographic effect calculation parameters
pub const OrographicParams = struct {
    max_orographic_bonus: f32, // Maximum precipitation bonus (mm)
    rain_shadow_factor: f32, // Rain shadow reduction factor (0.0-1.0)
    elevation_scale: f32, // Elevation scaling factor
    wind_effect_scale: f32, // Wind direction effect scaling
};

/// Mountain range data structure
pub const MountainRange = struct {
    center_x: f32,
    center_y: f32,
    width: f32,
    height: f32,
    orientation: f32, // Radians
};

/// Calculate orographic precipitation effect for a single position
pub fn calculateOrographicEffect(
    _: f32, // x - not used in current implementation
    _: f32, // y - not used in current implementation
    elevation: f32,
    wind_direction: f32,
    params: OrographicParams,
) f32 {
    // Calculate elevation factor (capped at 2x effect)
    const elevation_factor = precise.detMin(precise.detDiv(elevation, 1000.0), 2.0);

    // Calculate wind effect (0.0 to 1.0)
    const wind_cos = precise.detCos(wind_direction);
    const wind_effect = precise.detMul(precise.detAdd(wind_cos, 1.0), 0.5);

    // Combine effects
    const orographic_multiplier = precise.detAdd(1.0, precise.detMul(precise.detMul(elevation_factor, wind_effect), params.max_orographic_bonus));

    return orographic_multiplier;
}

/// SIMD batch calculation of orographic effects
pub fn batchOrographicEffects(
    positions: [][2]f32,
    elevations: []f32,
    wind_directions: []f32,
    params: OrographicParams,
    results: []f32,
) void {
    std.debug.assert(positions.len == elevations.len);
    std.debug.assert(positions.len == wind_directions.len);
    std.debug.assert(positions.len == results.len);

    const len = positions.len;
    const simd_len = len / 4;

    // Process 4 elements at a time with SIMD
    var i: usize = 0;
    while (i < simd_len * 4) : (i += 4) {
        // Load elevation data into SIMD vectors
        const elevs = [4]f32{ elevations[i], elevations[i + 1], elevations[i + 2], elevations[i + 3] };

        // Load wind directions
        const winds = [4]f32{ wind_directions[i], wind_directions[i + 1], wind_directions[i + 2], wind_directions[i + 3] };

        // Calculate elevation factors (elevation / 1000.0, capped at 2.0)
        const thousand = [4]f32{ 1000.0, 1000.0, 1000.0, 1000.0 };
        const two = [4]f32{ 2.0, 2.0, 2.0, 2.0 };
        const elevation_factors = simd.minVec4(simd.divVec4(elevs, thousand), two);

        // Calculate wind effects
        const wind_cos = [4]f32{ precise.detCos(winds[0]), precise.detCos(winds[1]), precise.detCos(winds[2]), precise.detCos(winds[3]) };
        const one = [4]f32{ 1.0, 1.0, 1.0, 1.0 };
        const half = [4]f32{ 0.5, 0.5, 0.5, 0.5 };
        const wind_effects = simd.mulVec4(simd.addVec4(wind_cos, one), half);

        // Combine effects
        const bonus = [4]f32{ params.max_orographic_bonus, params.max_orographic_bonus, params.max_orographic_bonus, params.max_orographic_bonus };
        const multipliers = simd.addVec4(one, simd.mulVec4(simd.mulVec4(elevation_factors, wind_effects), bonus));

        // Store results
        results[i] = multipliers[0];
        results[i + 1] = multipliers[1];
        results[i + 2] = multipliers[2];
        results[i + 3] = multipliers[3];
    }

    // Handle remaining elements
    while (i < len) : (i += 1) {
        results[i] = calculateOrographicEffect(positions[i][0], positions[i][1], elevations[i], wind_directions[i], params);
    }
}

/// Calculate rain shadow effect for positions downwind of mountains
pub fn calculateRainShadowEffect(
    positions: [][2]f32,
    elevations: []f32,
    mountain_ranges: []MountainRange,
    wind_direction: f32,
    shadow_factor: f32,
    results: []f32,
) void {
    std.debug.assert(positions.len == elevations.len);
    std.debug.assert(positions.len == results.len);

    for (positions, elevations, results) |pos, elevation, *result| {
        var shadow_effect: f32 = 1.0;

        // Elevation affects shadow strength - higher positions get less rain shadow
        const elevation_protection = precise.detMin(elevation / 2000.0, 0.5);

        // Check each mountain range for rain shadow effect
        for (mountain_ranges) |mountain| {
            const dx = precise.detSub(pos[0], mountain.center_x);
            const dy = precise.detSub(pos[1], mountain.center_y);
            const distance = precise.detSqrt(precise.detAdd(precise.detMul(dx, dx), precise.detMul(dy, dy)));

            // Check if position is in rain shadow (downwind of mountain)
            const wind_dx = precise.detCos(wind_direction);
            const wind_dy = precise.detSin(wind_direction);

            // Dot product to check if downwind
            const dot = precise.detAdd(precise.detMul(dx, wind_dx), precise.detMul(dy, wind_dy));

            if (dot > 0.0 and distance < mountain.width) {
                // Calculate shadow strength based on mountain height and distance
                const height_factor = precise.detDiv(mountain.height, 3000.0); // Normalize to typical mountain height
                const distance_factor = precise.detSub(1.0, precise.detDiv(distance, mountain.width));
                var shadow_strength = precise.detMul(precise.detMul(height_factor, distance_factor), shadow_factor);

                // Reduce shadow strength based on elevation protection
                shadow_strength = precise.detMul(shadow_strength, precise.detSub(1.0, elevation_protection));

                shadow_effect = precise.detMul(shadow_effect, precise.detSub(1.0, shadow_strength));
            }
        }

        result.* = shadow_effect;
    }
}

/// Apply orographic effects to base rainfall values
pub fn applyOrographicToRainfall(
    base_rainfall: []f32,
    orographic_multipliers: []f32,
    rain_shadow_effects: []f32,
    results: []f32,
) void {
    std.debug.assert(base_rainfall.len == orographic_multipliers.len);
    std.debug.assert(base_rainfall.len == rain_shadow_effects.len);
    std.debug.assert(base_rainfall.len == results.len);

    const len = base_rainfall.len;
    const simd_len = len / 4;

    // SIMD batch processing
    var i: usize = 0;
    while (i < simd_len * 4) : (i += 4) {
        const rainfall = [4]f32{ base_rainfall[i], base_rainfall[i + 1], base_rainfall[i + 2], base_rainfall[i + 3] };
        const orographic = [4]f32{ orographic_multipliers[i], orographic_multipliers[i + 1], orographic_multipliers[i + 2], orographic_multipliers[i + 3] };
        const shadows = [4]f32{ rain_shadow_effects[i], rain_shadow_effects[i + 1], rain_shadow_effects[i + 2], rain_shadow_effects[i + 3] };

        // Apply both orographic enhancement and rain shadow reduction
        const enhanced = simd.mulVec4(rainfall, orographic);
        const final_rainfall = simd.mulVec4(enhanced, shadows);

        results[i] = final_rainfall[0];
        results[i + 1] = final_rainfall[1];
        results[i + 2] = final_rainfall[2];
        results[i + 3] = final_rainfall[3];
    }

    // Handle remaining elements
    while (i < len) : (i += 1) {
        results[i] = precise.detMul(precise.detMul(base_rainfall[i], orographic_multipliers[i]), rain_shadow_effects[i]);
    }
}

// Tests
test "orographic effect calculation" {
    const testing = std.testing;

    const params = OrographicParams{
        .max_orographic_bonus = 200.0,
        .rain_shadow_factor = 0.6,
        .elevation_scale = 1.0,
        .wind_effect_scale = 1.0,
    };

    // Test single calculation
    const effect = calculateOrographicEffect(100.0, 100.0, 1500.0, 0.0, params);
    try testing.expect(effect > 1.0); // Should enhance precipitation

    // Test at sea level
    const sea_level_effect = calculateOrographicEffect(0.0, 0.0, 0.0, 0.0, params);
    try testing.expect(sea_level_effect == 1.0); // No enhancement at sea level
}

test "batch orographic calculations" {
    const testing = std.testing;

    const positions = [_][2]f32{ .{ 0.0, 0.0 }, .{ 100.0, 100.0 }, .{ 200.0, 200.0 }, .{ 300.0, 300.0 } };
    const elevations = [_]f32{ 0.0, 1000.0, 2000.0, 500.0 };
    const wind_directions = [_]f32{ 0.0, 0.0, 0.0, 0.0 };
    var results = [_]f32{ 0.0, 0.0, 0.0, 0.0 };

    const params = OrographicParams{
        .max_orographic_bonus = 200.0,
        .rain_shadow_factor = 0.6,
        .elevation_scale = 1.0,
        .wind_effect_scale = 1.0,
    };

    batchOrographicEffects(positions[0..], elevations[0..], wind_directions[0..], params, results[0..]);

    try testing.expect(results[0] == 1.0); // Sea level
    try testing.expect(results[1] > 1.0); // 1000m elevation
    try testing.expect(results[2] > results[1]); // 2000m > 1000m
}
