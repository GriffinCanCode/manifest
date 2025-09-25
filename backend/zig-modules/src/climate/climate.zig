//! Climate System Main Module
//!
//! Combines all climate calculation submodules into a unified SIMD-optimized climate system.
//! Provides high-level functions for batch climate processing used by Rust.

const std = @import("std");

pub const continental = @import("continental.zig");
pub const ContinentalParams = continental.ContinentalParams;
pub const interpolation = @import("interpolation.zig");
pub const ClimateData = interpolation.ClimateData;
pub const InterpolationParams = interpolation.InterpolationParams;
pub const orographic = @import("orographic.zig");
pub const OrographicParams = orographic.OrographicParams;
pub const seasonal = @import("seasonal.zig");
pub const ClimateZone = seasonal.ClimateZone;
pub const SeasonalParams = seasonal.SeasonalParams;
pub const SeasonalState = seasonal.SeasonalState;

// Import all climate submodules
// Re-export key types for convenience
/// Comprehensive climate processing parameters
pub const ClimateProcessingParams = struct {
    orographic: OrographicParams,
    continental: ContinentalParams,
    interpolation: InterpolationParams,
    seasonal: SeasonalParams,
    seasonal_state: SeasonalState,

    pub fn default() ClimateProcessingParams {
        return ClimateProcessingParams{
            .orographic = OrographicParams{
                .max_orographic_bonus = 200.0,
                .rain_shadow_factor = 0.6,
                .elevation_scale = 1.0,
                .wind_effect_scale = 1.0,
            },
            .continental = ContinentalParams{
                .temperature_amplification = 1.5,
                .humidity_reduction = 0.8,
                .world_width = 256.0,
                .world_height = 256.0,
            },
            .interpolation = InterpolationParams{
                .temperature_weight = 0.3,
                .rainfall_weight = 0.4,
                .humidity_weight = 0.3,
                .wind_weight = 0.2,
                .distance_falloff = 0.5,
                .max_influence_distance = 3.0,
            },
            .seasonal = SeasonalParams.default(),
            .seasonal_state = SeasonalState{
                .current_season = 0.0,
                .year_progress = 0.0,
                .hemisphere_modifier = 1.0,
            },
        };
    }
};

/// Complete climate processing pipeline for a batch of tiles
pub fn processClimateEffects(
    positions: [][2]f32,
    elevations: []f32,
    base_temperatures: []i8,
    base_rainfall: []f32,
    base_humidity: []u8,
    climate_zones: []ClimateZone,
    latitudes: []f32,
    wind_directions: []f32,
    mountain_ranges: []orographic.MountainRange,
    params: ClimateProcessingParams,
    temperature_results: []i8,
    rainfall_results: []f32,
    humidity_results: []u8,
) void {
    std.debug.assert(positions.len == elevations.len);
    std.debug.assert(positions.len == base_temperatures.len);
    std.debug.assert(positions.len == base_rainfall.len);
    std.debug.assert(positions.len == base_humidity.len);
    std.debug.assert(positions.len == temperature_results.len);
    std.debug.assert(positions.len == rainfall_results.len);
    std.debug.assert(positions.len == humidity_results.len);

    const len = positions.len;

    // Allocate temporary buffers for intermediate calculations
    var orographic_multipliers = std.ArrayList(f32).initCapacity(std.heap.page_allocator, len) catch unreachable;
    defer orographic_multipliers.deinit();
    orographic_multipliers.resize(len) catch unreachable;

    var rain_shadow_effects = std.ArrayList(f32).initCapacity(std.heap.page_allocator, len) catch unreachable;
    defer rain_shadow_effects.deinit();
    rain_shadow_effects.resize(len) catch unreachable;

    var modified_rainfall = std.ArrayList(f32).initCapacity(std.heap.page_allocator, len) catch unreachable;
    defer modified_rainfall.deinit();
    modified_rainfall.resize(len) catch unreachable;

    var continental_temps = std.ArrayList(i8).initCapacity(std.heap.page_allocator, len) catch unreachable;
    defer continental_temps.deinit();
    continental_temps.resize(len) catch unreachable;

    var continental_humidity = std.ArrayList(u8).initCapacity(std.heap.page_allocator, len) catch unreachable;
    defer continental_humidity.deinit();
    continental_humidity.resize(len) catch unreachable;

    // Step 1: Apply orographic effects to rainfall
    orographic.batchOrographicEffects(
        positions,
        elevations,
        wind_directions,
        params.orographic,
        orographic_multipliers.items,
    );

    orographic.calculateRainShadowEffect(
        positions,
        elevations,
        mountain_ranges,
        0.0, // Default wind direction, could be parameterized
        params.orographic.rain_shadow_factor,
        rain_shadow_effects.items,
    );

    orographic.applyOrographicToRainfall(
        base_rainfall,
        orographic_multipliers.items,
        rain_shadow_effects.items,
        modified_rainfall.items,
    );

    // Step 2: Apply continental effects to temperature and humidity
    continental.batchContinentalEffects(
        positions,
        base_temperatures,
        base_humidity,
        params.continental,
        continental_temps.items,
        continental_humidity.items,
    );

    // Step 3: Apply seasonal variations
    seasonal.batchSeasonalTemperature(
        continental_temps.items,
        climate_zones,
        latitudes,
        params.seasonal_state,
        params.seasonal,
        temperature_results,
    );

    // Convert rainfall from f32 to u16 for seasonal processing
    var rainfall_u16 = std.ArrayList(u16).initCapacity(std.heap.page_allocator, len) catch unreachable;
    defer rainfall_u16.deinit();
    rainfall_u16.resize(len) catch unreachable;

    for (modified_rainfall.items, rainfall_u16.items) |rain_f32, *rain_u16| {
        rain_u16.* = @as(u16, @intFromFloat(@max(0.0, @min(500.0, rain_f32))));
    }

    var seasonal_rainfall = std.ArrayList(u16).initCapacity(std.heap.page_allocator, len) catch unreachable;
    defer seasonal_rainfall.deinit();
    seasonal_rainfall.resize(len) catch unreachable;

    seasonal.batchSeasonalRainfall(
        rainfall_u16.items,
        climate_zones,
        latitudes,
        params.seasonal_state,
        params.seasonal,
        seasonal_rainfall.items,
    );

    // Convert back to f32 for final results
    for (seasonal_rainfall.items, rainfall_results) |rain_u16, *rain_f32| {
        rain_f32.* = @as(f32, @floatFromInt(rain_u16));
    }

    // Copy humidity results (already processed by continental effects)
    for (continental_humidity.items, humidity_results) |hum, *result| {
        result.* = hum;
    }
}

/// Simplified climate processing for basic use cases
pub fn simpleClimateProcessing(
    positions: [][2]f32,
    elevations: []f32,
    base_temperatures: []i8,
    base_rainfall: []f32,
    base_humidity: []u8,
    wind_directions: []f32,
    temperature_results: []i8,
    rainfall_results: []f32,
    humidity_results: []u8,
) void {
    // Use default parameters
    const params = ClimateProcessingParams.default();

    // Create default climate zones (temperate for all)
    var climate_zones = std.ArrayList(ClimateZone).initCapacity(std.heap.page_allocator, positions.len) catch unreachable;
    defer climate_zones.deinit();
    climate_zones.resize(positions.len) catch unreachable;

    for (climate_zones.items) |*zone| {
        zone.* = ClimateZone.Temperate;
    }

    // Create default latitudes based on Y position
    var latitudes = std.ArrayList(f32).initCapacity(std.heap.page_allocator, positions.len) catch unreachable;
    defer latitudes.deinit();
    latitudes.resize(positions.len) catch unreachable;

    for (positions, latitudes.items) |pos, *lat| {
        // Convert Y position to latitude (-90 to +90)
        lat.* = (pos[1] / params.continental.world_height - 0.5) * 180.0;
    }

    // No mountain ranges for simple processing
    const mountain_ranges: []orographic.MountainRange = &.{};

    processClimateEffects(
        positions,
        elevations,
        base_temperatures,
        base_rainfall,
        base_humidity,
        climate_zones.items,
        latitudes.items,
        wind_directions,
        mountain_ranges,
        params,
        temperature_results,
        rainfall_results,
        humidity_results,
    );
}

/// Process only orographic effects (for testing or specialized use)
pub fn processOrographicOnly(
    positions: [][2]f32,
    elevations: []f32,
    base_rainfall: []f32,
    wind_directions: []f32,
    mountain_ranges: []orographic.MountainRange,
    params: OrographicParams,
    rainfall_results: []f32,
) void {
    var orographic_multipliers = std.ArrayList(f32).initCapacity(std.heap.page_allocator, positions.len) catch unreachable;
    defer orographic_multipliers.deinit();
    orographic_multipliers.resize(positions.len) catch unreachable;

    var rain_shadow_effects = std.ArrayList(f32).initCapacity(std.heap.page_allocator, positions.len) catch unreachable;
    defer rain_shadow_effects.deinit();
    rain_shadow_effects.resize(positions.len) catch unreachable;

    orographic.batchOrographicEffects(
        positions,
        elevations,
        wind_directions,
        params,
        orographic_multipliers.items,
    );

    orographic.calculateRainShadowEffect(
        positions,
        elevations,
        mountain_ranges,
        0.0, // Default wind direction
        params.rain_shadow_factor,
        rain_shadow_effects.items,
    );

    orographic.applyOrographicToRainfall(
        base_rainfall,
        orographic_multipliers.items,
        rain_shadow_effects.items,
        rainfall_results,
    );
}

/// Process only continental effects (for testing or specialized use)
pub fn processContinentalOnly(
    positions: [][2]f32,
    base_temperatures: []i8,
    base_humidity: []u8,
    params: ContinentalParams,
    temperature_results: []i8,
    humidity_results: []u8,
) void {
    continental.batchContinentalEffects(
        positions,
        base_temperatures,
        base_humidity,
        params,
        temperature_results,
        humidity_results,
    );
}

// Tests
test "complete climate processing" {
    const testing = std.testing;

    // Create test data
    const positions = [_][2]f32{ .{ 50.0, 50.0 }, .{ 150.0, 150.0 }, .{ 100.0, 100.0 }, .{ 200.0, 200.0 } };
    const elevations = [_]f32{ 100.0, 1500.0, 500.0, 2000.0 };
    const base_temps = [_]i8{ 20, 15, 18, 10 };
    const base_rainfall = [_]f32{ 100.0, 200.0, 150.0, 300.0 };
    const base_humidity = [_]u8{ 60, 70, 65, 80 };
    const wind_directions = [_]f32{ 0.0, 0.0, 0.0, 0.0 };

    var temp_results = [_]i8{ 0, 0, 0, 0 };
    var rain_results = [_]f32{ 0.0, 0.0, 0.0, 0.0 };
    var hum_results = [_]u8{ 0, 0, 0, 0 };

    simpleClimateProcessing(
        positions[0..],
        elevations[0..],
        base_temps[0..],
        base_rainfall[0..],
        base_humidity[0..],
        wind_directions[0..],
        temp_results[0..],
        rain_results[0..],
        hum_results[0..],
    );

    // Results should be modified from base values
    try testing.expect(temp_results[0] != base_temps[0] or temp_results[1] != base_temps[1]);
    try testing.expect(rain_results[0] != base_rainfall[0] or rain_results[1] != base_rainfall[1]);
}

test "orographic only processing" {
    const testing = std.testing;

    const positions = [_][2]f32{ .{ 100.0, 100.0 }, .{ 200.0, 200.0 } };
    const elevations = [_]f32{ 0.0, 1500.0 };
    const base_rainfall = [_]f32{ 100.0, 100.0 };
    const wind_directions = [_]f32{ 0.0, 0.0 };
    const mountain_ranges: []orographic.MountainRange = &.{};

    const params = OrographicParams{
        .max_orographic_bonus = 200.0,
        .rain_shadow_factor = 0.6,
        .elevation_scale = 1.0,
        .wind_effect_scale = 1.0,
    };

    var results = [_]f32{ 0.0, 0.0 };

    processOrographicOnly(
        positions[0..],
        elevations[0..],
        base_rainfall[0..],
        wind_directions[0..],
        mountain_ranges,
        params,
        results[0..],
    );

    // Higher elevation should have enhanced rainfall
    try testing.expect(results[1] > results[0]);
}
