//! Seasonal Climate Variations
//!
//! SIMD-optimized seasonal climate calculations including temperature and rainfall cycles.
//! Handles hemispheric differences and climate zone variations.

const std = @import("std");

const precise = @import("../math/precise.zig");
const simd = @import("../simd/simd.zig");

/// Climate zone types for seasonal variation
pub const ClimateZone = enum(u8) {
    Equatorial = 0,
    Tropical = 1,
    Temperate = 2,
    Polar = 3,
    Desert = 4,
    Mediterranean = 5,
};

/// Seasonal parameters for different climate zones
pub const SeasonalParams = struct {
    temperature_variation: [6]f32, // Temperature variation by climate zone
    rainfall_variation: [6]f32, // Rainfall variation by climate zone
    season_offset: f32, // Phase offset for southern hemisphere
    season_intensity: f32, // Overall seasonal intensity multiplier

    pub fn default() SeasonalParams {
        return SeasonalParams{
            .temperature_variation = .{ 2.0, 5.0, 15.0, 25.0, 12.0, 8.0 }, // Equatorial to Mediterranean
            .rainfall_variation = .{ 50.0, 100.0, 75.0, 25.0, 20.0, 120.0 },
            .season_offset = 0.5, // Half-year offset for southern hemisphere
            .season_intensity = 1.0,
        };
    }
};

/// Current seasonal state
pub const SeasonalState = struct {
    current_season: f32, // 0.0-1.0 where 0.0 = spring equinox
    year_progress: f32, // Progress through the year (0.0-1.0)
    hemisphere_modifier: f32, // 1.0 for northern, -1.0 for southern
};

/// Apply seasonal temperature variation
pub fn applyTemperatureVariation(
    base_temp: i8,
    climate_zone: ClimateZone,
    latitude: f32,
    seasonal_state: SeasonalState,
    params: SeasonalParams,
) i8 {
    const zone_index = @intFromEnum(climate_zone);
    const variation = params.temperature_variation[zone_index];

    // Determine hemisphere (positive latitude = northern)
    const hemisphere_offset = if (latitude >= 0.0) 0.0 else params.season_offset;
    const season_phase = precise.detAdd(seasonal_state.current_season, hemisphere_offset);

    // Calculate seasonal cycle with intensity modifier
    const season_cycle = precise.detSin(precise.detMul(season_phase, precise.detMul(2.0, std.math.pi)));
    const scaled_cycle = precise.detMul(season_cycle, params.season_intensity);

    // Apply latitude scaling (stronger variations at higher latitudes)
    const latitude_factor = precise.detMin(precise.detDiv(precise.detAbs(latitude), 90.0), 1.0);
    const latitude_scaled_variation = precise.detMul(variation, latitude_factor);

    const temp_change = precise.detMul(scaled_cycle, latitude_scaled_variation);
    const new_temp = precise.detAdd(@as(f32, @floatFromInt(base_temp)), temp_change);

    return @as(i8, @intFromFloat(precise.detClamp(new_temp, -50.0, 50.0)));
}

/// Apply seasonal rainfall variation
pub fn applyRainfallVariation(
    base_rainfall: u16,
    climate_zone: ClimateZone,
    latitude: f32,
    seasonal_state: SeasonalState,
    params: SeasonalParams,
) u16 {
    const zone_index = @intFromEnum(climate_zone);
    const variation = params.rainfall_variation[zone_index];

    // Rainfall cycle offset by quarter season from temperature
    const hemisphere_offset = if (latitude >= 0.0) 0.0 else params.season_offset;
    const season_phase = precise.detAdd(precise.detAdd(seasonal_state.current_season, hemisphere_offset), 0.25 // Quarter phase offset
    );

    const rain_cycle = precise.detSin(precise.detMul(season_phase, precise.detMul(2.0, std.math.pi)));
    const rain_change = precise.detMul(rain_cycle, variation);

    const new_rainfall = precise.detAdd(@as(f32, @floatFromInt(base_rainfall)), rain_change);
    return @as(u16, @intFromFloat(precise.detClamp(new_rainfall, 0.0, 500.0)));
}

/// SIMD batch seasonal temperature calculation
pub fn batchSeasonalTemperature(
    base_temperatures: []const i8,
    climate_zones: []const ClimateZone,
    latitudes: []const f32,
    seasonal_state: SeasonalState,
    params: SeasonalParams,
    results: []i8,
) void {
    std.debug.assert(base_temperatures.len == climate_zones.len);
    std.debug.assert(base_temperatures.len == latitudes.len);
    std.debug.assert(base_temperatures.len == results.len);

    const len = base_temperatures.len;
    const simd_len = len / 4;

    // Pre-calculate seasonal cycle components
    const two_pi = precise.detMul(2.0, std.math.pi);
    const season_scaled = precise.detMul(seasonal_state.current_season, params.season_intensity);

    var i: usize = 0;

    // Process 4 elements at a time with SIMD
    while (i < simd_len * 4) : (i += 4) {
        const base_temps = [4]f32{ @as(f32, @floatFromInt(base_temperatures[i])), @as(f32, @floatFromInt(base_temperatures[i + 1])), @as(f32, @floatFromInt(base_temperatures[i + 2])), @as(f32, @floatFromInt(base_temperatures[i + 3])) };

        const lats = [4]f32{ latitudes[i], latitudes[i + 1], latitudes[i + 2], latitudes[i + 3] };

        // Calculate hemisphere offsets
        var hemisphere_offsets: [4]f32 = undefined;
        var variations: [4]f32 = undefined;

        for (0..4) |j| {
            const idx = i + j;
            hemisphere_offsets[j] = if (lats[j] >= 0.0) 0.0 else params.season_offset;

            const zone_index = @intFromEnum(climate_zones[idx]);
            variations[j] = params.temperature_variation[zone_index];
        }

        // Calculate seasonal phases
        var seasonal_phases: [4]f32 = undefined;
        for (0..4) |j| {
            seasonal_phases[j] = precise.detAdd(season_scaled, hemisphere_offsets[j]);
        }

        // Calculate seasonal cycles
        var season_cycles: [4]f32 = undefined;
        for (0..4) |j| {
            season_cycles[j] = precise.detSin(precise.detMul(seasonal_phases[j], two_pi));
        }

        // Calculate latitude factors (SIMD)
        const ninety = [4]f32{ 90.0, 90.0, 90.0, 90.0 };
        const one = [4]f32{ 1.0, 1.0, 1.0, 1.0 };
        const abs_lats = [4]f32{ precise.detAbs(lats[0]), precise.detAbs(lats[1]), precise.detAbs(lats[2]), precise.detAbs(lats[3]) };
        const latitude_factors = simd.minVec4(simd.divVec4(abs_lats, ninety), one);

        // Apply all effects
        for (0..4) |j| {
            const idx = i + j;
            const latitude_scaled_variation = precise.detMul(variations[j], latitude_factors[j]);
            const temp_change = precise.detMul(season_cycles[j], latitude_scaled_variation);
            const new_temp = precise.detAdd(base_temps[j], temp_change);

            results[idx] = @as(i8, @intFromFloat(precise.detClamp(new_temp, -50.0, 50.0)));
        }
    }

    // Handle remaining elements
    while (i < len) : (i += 1) {
        results[i] = applyTemperatureVariation(base_temperatures[i], climate_zones[i], latitudes[i], seasonal_state, params);
    }
}

/// SIMD batch seasonal rainfall calculation
pub fn batchSeasonalRainfall(
    base_rainfall: []const u16,
    climate_zones: []const ClimateZone,
    latitudes: []const f32,
    seasonal_state: SeasonalState,
    params: SeasonalParams,
    results: []u16,
) void {
    std.debug.assert(base_rainfall.len == climate_zones.len);
    std.debug.assert(base_rainfall.len == latitudes.len);
    std.debug.assert(base_rainfall.len == results.len);

    const len = base_rainfall.len;
    const simd_len = len / 4;

    // Pre-calculate seasonal cycle components
    const two_pi = precise.detMul(2.0, std.math.pi);
    const quarter_offset = 0.25;

    var i: usize = 0;

    // Process 4 elements at a time
    while (i < simd_len * 4) : (i += 4) {
        const base_rain = [4]f32{ @as(f32, @floatFromInt(base_rainfall[i])), @as(f32, @floatFromInt(base_rainfall[i + 1])), @as(f32, @floatFromInt(base_rainfall[i + 2])), @as(f32, @floatFromInt(base_rainfall[i + 3])) };

        const lats = [4]f32{ latitudes[i], latitudes[i + 1], latitudes[i + 2], latitudes[i + 3] };

        // Calculate hemisphere offsets and variations
        var hemisphere_offsets: [4]f32 = undefined;
        var variations: [4]f32 = undefined;

        for (0..4) |j| {
            const idx = i + j;
            hemisphere_offsets[j] = if (lats[j] >= 0.0) 0.0 else params.season_offset;

            const zone_index = @intFromEnum(climate_zones[idx]);
            variations[j] = params.rainfall_variation[zone_index];
        }

        // Calculate seasonal phases with quarter offset
        var seasonal_phases: [4]f32 = undefined;
        for (0..4) |j| {
            seasonal_phases[j] = precise.detAdd(precise.detAdd(seasonal_state.current_season, hemisphere_offsets[j]), quarter_offset);
        }

        // Calculate rainfall cycles and apply changes
        for (0..4) |j| {
            const idx = i + j;
            const rain_cycle = precise.detSin(precise.detMul(seasonal_phases[j], two_pi));
            const rain_change = precise.detMul(rain_cycle, variations[j]);
            const new_rainfall = precise.detAdd(base_rain[j], rain_change);

            results[idx] = @as(u16, @intFromFloat(precise.detClamp(new_rainfall, 0.0, 500.0)));
        }
    }

    // Handle remaining elements
    while (i < len) : (i += 1) {
        results[i] = applyRainfallVariation(base_rainfall[i], climate_zones[i], latitudes[i], seasonal_state, params);
    }
}

/// Calculate monsoon effects for tropical regions
pub fn calculateMonsoonEffect(
    latitude: f32,
    _: f32, // longitude - not used in current implementation
    seasonal_state: SeasonalState,
    monsoon_strength: f32,
) f32 {
    // Monsoons are strongest in tropical latitudes (10-30 degrees)
    const abs_lat = precise.detAbs(latitude);
    const monsoon_latitude_factor = if (abs_lat >= 10.0 and abs_lat <= 30.0)
        precise.detSub(1.0, precise.detDiv(precise.detAbs(precise.detSub(abs_lat, 20.0)), 10.0))
    else
        0.0;

    // Monsoon cycle (offset from regular seasons)
    const monsoon_phase = precise.detAdd(seasonal_state.current_season, 0.3);
    const monsoon_cycle = precise.detSin(precise.detMul(monsoon_phase, precise.detMul(2.0, std.math.pi)));

    // Combine factors
    return precise.detMul(precise.detMul(monsoon_latitude_factor, monsoon_cycle), monsoon_strength);
}

/// Update seasonal state based on current time
pub fn updateSeasonalState(
    current_turn: u64,
    turns_per_year: u64,
) SeasonalState {
    const year_progress = precise.detDiv(@as(f32, @floatFromInt(current_turn % turns_per_year)), @as(f32, @floatFromInt(turns_per_year)));

    // Convert to seasonal phase (0.0 = spring equinox)
    const current_season = year_progress;

    return SeasonalState{
        .current_season = current_season,
        .year_progress = year_progress,
        .hemisphere_modifier = 1.0, // Updated per calculation based on latitude
    };
}

// Tests
test "seasonal temperature variation" {
    const testing = std.testing;

    const params = SeasonalParams.default();
    const summer_state = SeasonalState{
        .current_season = 0.25, // Summer
        .year_progress = 0.25,
        .hemisphere_modifier = 1.0,
    };
    const winter_state = SeasonalState{
        .current_season = 0.75, // Winter
        .year_progress = 0.75,
        .hemisphere_modifier = 1.0,
    };

    // Test temperate zone (should have strong seasonal variation)
    const base_temp: i8 = 20;
    const temperate_latitude: f32 = 45.0; // Northern temperate

    const summer_temp = applyTemperatureVariation(base_temp, ClimateZone.Temperate, temperate_latitude, summer_state, params);
    const winter_temp = applyTemperatureVariation(base_temp, ClimateZone.Temperate, temperate_latitude, winter_state, params);

    // Summer should be hotter than winter in northern hemisphere
    try testing.expect(summer_temp > winter_temp);

    // Test equatorial zone (should have minimal variation)
    const equatorial_summer = applyTemperatureVariation(base_temp, ClimateZone.Equatorial, 0.0, summer_state, params);
    const equatorial_winter = applyTemperatureVariation(base_temp, ClimateZone.Equatorial, 0.0, winter_state, params);

    // Equatorial variation should be smaller than temperate
    const temperate_variation = @abs(summer_temp - winter_temp);
    const equatorial_variation = @abs(equatorial_summer - equatorial_winter);
    try testing.expect(equatorial_variation < temperate_variation);
}

test "hemisphere seasonal differences" {
    const testing = std.testing;

    const params = SeasonalParams.default();
    const seasonal_state = SeasonalState{
        .current_season = 0.25, // Northern summer
        .year_progress = 0.25,
        .hemisphere_modifier = 1.0,
    };

    const base_temp: i8 = 20;

    // Northern hemisphere (positive latitude)
    const northern_temp = applyTemperatureVariation(base_temp, ClimateZone.Temperate, 45.0, seasonal_state, params);

    // Southern hemisphere (negative latitude)
    const southern_temp = applyTemperatureVariation(base_temp, ClimateZone.Temperate, -45.0, seasonal_state, params);

    // During northern summer, southern hemisphere should be cooler
    try testing.expect(northern_temp != southern_temp);
}

test "monsoon effect calculation" {
    const testing = std.testing;

    const seasonal_state = SeasonalState{
        .current_season = 0.4, // Monsoon season
        .year_progress = 0.4,
        .hemisphere_modifier = 1.0,
    };

    // Test tropical latitude (strong monsoon)
    const tropical_monsoon = calculateMonsoonEffect(20.0, 80.0, seasonal_state, 100.0);
    try testing.expect(tropical_monsoon != 0.0);

    // Test polar latitude (no monsoon)
    const polar_monsoon = calculateMonsoonEffect(70.0, 80.0, seasonal_state, 100.0);
    try testing.expect(polar_monsoon == 0.0);
}

test "seasonal state update" {
    const testing = std.testing;

    // Test beginning of year
    const start_state = updateSeasonalState(0, 365);
    try testing.expect(start_state.current_season == 0.0);
    try testing.expect(start_state.year_progress == 0.0);

    // Test middle of year
    const mid_state = updateSeasonalState(182, 365);
    try testing.expect(mid_state.year_progress > 0.4 and mid_state.year_progress < 0.6);
}
