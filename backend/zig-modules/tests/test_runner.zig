//! Comprehensive test runner for all Zig modules in the Manifest Game Engine
//!
//! This test suite ensures all SIMD-optimized calculations work correctly
//! and maintain deterministic behavior across platforms.

const std = @import("std");
const testing = std.testing;
const TestAllocator = std.testing.allocator;

const climate = @import("climate/climate.zig");
const hydrology = @import("hydrology/mod.zig");
const hex = @import("math/hex.zig");
const precise = @import("math/precise.zig");
const noise = @import("noise/noise.zig");
const simd = @import("simd/simd.zig");
const geometry = @import("tectonics/geometry.zig");
const plates = @import("tectonics/plates.zig");
const stress = @import("tectonics/stress.zig");
const volcanic = @import("tectonics/volcanic.zig");

// Import all modules under test
// Test utilities
/// Test result tracking
const TestResult = struct {
    name: []const u8,
    passed: bool,
    error_message: ?[]const u8 = null,
};

var test_results = std.ArrayList(TestResult).init(TestAllocator);

/// Helper function to run a test and track results
fn runTest(comptime test_name: []const u8, test_fn: *const fn () anyerror!void) !void {
    test_fn() catch |err| {
        try test_results.append(TestResult{
            .name = test_name,
            .passed = false,
            .error_message = @errorName(err),
        });
        std.debug.print("❌ {s}: {}\n", .{ test_name, err });
        return;
    };

    try test_results.append(TestResult{
        .name = test_name,
        .passed = true,
    });
    std.debug.print("✅ {s}\n", .{test_name});
}

/// Print summary of all test results
fn printTestSummary() !void {
    var passed: usize = 0;
    var failed: usize = 0;

    std.debug.print("\n" ++ "=" * 60 ++ "\n");
    std.debug.print("MANIFEST ZIG MODULE TEST SUMMARY\n");
    std.debug.print("=" * 60 ++ "\n");

    for (test_results.items) |result| {
        if (result.passed) {
            passed += 1;
        } else {
            failed += 1;
            std.debug.print("FAILED: {s}", .{result.name});
            if (result.error_message) |msg| {
                std.debug.print(" - {s}", .{msg});
            }
            std.debug.print("\n");
        }
    }

    std.debug.print("\nTotal: {} | Passed: {} | Failed: {}\n", .{ passed + failed, passed, failed });

    if (failed == 0) {
        std.debug.print("🎉 ALL TESTS PASSED!\n");
    } else {
        std.debug.print("⚠️  {} tests failed\n", .{failed});
    }

    std.debug.print("=" * 60 ++ "\n\n");
}

pub fn main() !void {
    defer test_results.deinit();

    std.debug.print("🚀 Starting Manifest Zig Module Tests\n\n");

    // Math module tests
    std.debug.print("📐 Testing Math Modules...\n");
    try runTest("Hex Coordinate Conversion", testHexCoordinateConversion);
    try runTest("Hex Distance Calculations", testHexDistanceCalculations);
    try runTest("Hex SIMD Operations", testHexSimdOperations);
    try runTest("Hex Pixel Conversion", testHexPixelConversion);
    try runTest("Hex Neighbors and Rings", testHexNeighborsAndRings);
    try runTest("Hex Line Drawing", testHexLineDrawing);

    try runTest("Precise Math Operations", testPreciseMathOperations);
    try runTest("Precise NaN/Infinity Handling", testPreciseNanInfinityHandling);
    try runTest("Precise Approximation", testPreciseApproximation);

    try runTest("SIMD Vector Operations", testSimdVectorOperations);
    try runTest("SIMD Batch Operations", testSimdBatchOperations);
    try runTest("SIMD Advanced Operations", testSimdAdvancedOperations);

    // Climate module tests
    std.debug.print("\n🌡️  Testing Climate Modules...\n");
    try runTest("Climate Processing Pipeline", testClimateProcessingPipeline);
    try runTest("Orographic Effects", testOrographicEffects);
    try runTest("Continental Effects", testContinentalEffects);
    try runTest("Seasonal Variations", testSeasonalVariations);

    // Hydrology module tests
    std.debug.print("\n🌊 Testing Hydrology Modules...\n");
    try runTest("Hydraulic Calculations", testHydraulicCalculations);
    try runTest("Flow Analysis", testFlowAnalysis);
    try runTest("Groundwater Modeling", testGroundwaterModeling);
    try runTest("Spring Generation", testSpringGeneration);

    // Tectonics module tests
    std.debug.print("\n🌋 Testing Tectonics Modules...\n");
    try runTest("Plate Force Calculations", testPlateForceCalculations);
    try runTest("Geometry Operations", testGeometryOperations);
    try runTest("Stress Calculations", testStressCalculations);
    try runTest("Volcanic Hazards", testVolcanicHazards);

    // Integration tests with FFI exports
    std.debug.print("\n🔌 Testing FFI Integration...\n");
    try runTest("FFI Hex Operations", testFfiHexOperations);
    try runTest("FFI Climate Batch Processing", testFfiClimateBatchProcessing);
    try runTest("FFI Hydraulics Batch", testFfiHydraulicsBatch);

    try printTestSummary();
}

// ============================================================================
// MATH MODULE TESTS
// ============================================================================

fn testHexCoordinateConversion() !void {
    // Test axial ↔ cube coordinate conversion
    const hex_coord = hex.HexCoord.init(2, -1);
    const cube_coord = hex_coord.toCube();

    try testing.expect(cube_coord.x == 2);
    try testing.expect(cube_coord.y == -1);
    try testing.expect(cube_coord.z == -1);
    try testing.expect(cube_coord.x + cube_coord.y + cube_coord.z == 0);

    const back_to_hex = cube_coord.toAxial();
    try testing.expect(back_to_hex.q == hex_coord.q);
    try testing.expect(back_to_hex.r == hex_coord.r);

    // Test offset coordinate conversion
    const offset_coord = hex_coord.toOffset();
    const back_from_offset = offset_coord.toAxial();
    try testing.expect(back_from_offset.q == hex_coord.q);
    try testing.expect(back_from_offset.r == hex_coord.r);

    // Test cube rotation
    const rotated = cube_coord.rotate(1);
    try testing.expect(rotated.x == -cube_coord.z);
    try testing.expect(rotated.y == -cube_coord.x);
    try testing.expect(rotated.z == -cube_coord.y);
}

fn testHexDistanceCalculations() !void {
    // Test basic distance calculation
    try testing.expect(hex.distance(0, 0, 3, 0) == 3);
    try testing.expect(hex.distance(0, 0, 0, 3) == 3);
    try testing.expect(hex.distance(0, 0, 2, 2) == 4);
    try testing.expect(hex.distance(1, 1, 1, 1) == 0);

    // Test symmetry
    try testing.expect(hex.distance(0, 0, 3, 2) == hex.distance(3, 2, 0, 0));

    // Test batch distance calculations
    const coords1 = [_]hex.HexCoord{ hex.HexCoord.init(0, 0), hex.HexCoord.init(1, 1) };
    const coords2 = [_]hex.HexCoord{ hex.HexCoord.init(0, 0), hex.HexCoord.init(2, 2) };
    var distances: [2]u32 = undefined;

    hex.batchDistances(&coords1, &coords2, &distances);
    try testing.expect(distances[0] == 0); // (0,0) to (0,0)
    try testing.expect(distances[1] == 2); // (1,1) to (2,2)
}

fn testHexSimdOperations() !void {
    const coords = [_]hex.HexCoord{
        hex.HexCoord.init(0, 0),
        hex.HexCoord.init(1, 0),
        hex.HexCoord.init(2, 0),
        hex.HexCoord.init(3, 0),
    };

    var pixels: [4]hex.PixelPos = undefined;
    const size: f32 = 10.0;

    hex.batchToPixel(&coords, size, &pixels);

    // Verify first coordinate (0,0) maps to (0,0)
    try testing.expect(pixels[0].x == 0.0);
    try testing.expect(pixels[0].y == 0.0);

    // Verify second coordinate has correct spacing
    const expected_spacing = 15.0; // 3/2 * size for flat-top hex
    try testing.expect(@abs(pixels[1].x - expected_spacing) < 0.001);
}

fn testHexPixelConversion() !void {
    const size: f32 = 20.0;
    const original = hex.HexCoord.init(3, -2);

    // Convert to pixel and back
    const pixel = hex.toPixel(original.q, original.r, size);
    const converted_back = hex.fromPixel(pixel.x, pixel.y, size);

    try testing.expect(converted_back.q == original.q);
    try testing.expect(converted_back.r == original.r);

    // Test floating-point rounding
    const rounded = hex.roundToHex(1.7, -0.8);
    try testing.expect(rounded.q == 2);
    try testing.expect(rounded.r == -1);
}

fn testHexNeighborsAndRings() !void {
    const center = hex.HexCoord.init(0, 0);

    // Test neighbors
    const neighbors = hex.getNeighbors(center);
    try testing.expect(neighbors.len == 6);

    // All neighbors should be distance 1
    for (neighbors) |neighbor| {
        try testing.expect(hex.distance(center.q, center.r, neighbor.q, neighbor.r) == 1);
    }

    // Test specific neighbor
    const east_neighbor = hex.getNeighbor(center, 0); // East = direction 0
    try testing.expect(east_neighbor.q == 1 and east_neighbor.r == 0);

    // Test ring generation
    const ring = try hex.getHexRing(center, 2, TestAllocator);
    defer TestAllocator.free(ring);

    try testing.expect(ring.len == 12); // 6 * radius for radius > 0

    // All hexes in ring should be distance 2
    for (ring) |hex_in_ring| {
        try testing.expect(hex.distance(center.q, center.r, hex_in_ring.q, hex_in_ring.r) == 2);
    }
}

fn testHexLineDrawing() !void {
    const start = hex.HexCoord.init(0, 0);
    const end = hex.HexCoord.init(3, 0);

    const line = try hex.drawLine(start, end, TestAllocator);
    defer TestAllocator.free(line);

    try testing.expect(line.len == 4); // Distance + 1
    try testing.expect(line[0].q == 0 and line[0].r == 0); // Start
    try testing.expect(line[3].q == 3 and line[3].r == 0); // End

    // Verify line is connected (each step is distance 1 from previous)
    for (1..line.len) |i| {
        const dist = hex.distance(line[i - 1].q, line[i - 1].r, line[i].q, line[i].r);
        try testing.expect(dist <= 1);
    }
}

fn testPreciseMathOperations() !void {
    // Test basic operations
    try testing.expect(precise.detAdd(2.5, 3.5) == 6.0);
    try testing.expect(precise.detMul(4.0, 2.5) == 10.0);
    try testing.expect(precise.detDiv(10.0, 2.0) == 5.0);
    try testing.expect(precise.detSqrt(16.0) == 4.0);

    // Test deterministic trigonometry
    const angle = std.math.pi / 4.0;
    const sin_val = precise.detSin(angle);
    const cos_val = precise.detCos(angle);

    // sin(π/4) ≈ cos(π/4) ≈ √2/2 ≈ 0.707
    try testing.expect(@abs(sin_val - cos_val) < 0.001);
    try testing.expect(@abs(sin_val - 0.707) < 0.01);

    // Test min/max operations
    try testing.expect(precise.detMin(5.0, 3.0) == 3.0);
    try testing.expect(precise.detMax(5.0, 3.0) == 5.0);
    try testing.expect(precise.detClamp(7.0, 2.0, 5.0) == 5.0);

    // Test lerp
    const lerp_result = precise.detLerp(0.0, 10.0, 0.5);
    try testing.expect(lerp_result == 5.0);
}

fn testPreciseNanInfinityHandling() !void {
    const nan = std.math.nan(f32);
    const inf = std.math.inf(f32);

    // NaN propagation
    try testing.expect(std.math.isNan(precise.detAdd(nan, 5.0)));
    try testing.expect(std.math.isNan(precise.detMul(nan, 2.0)));

    // Infinity handling
    try testing.expect(precise.detAdd(inf, 5.0) == inf);
    try testing.expect(std.math.isNan(precise.detDiv(0.0, 0.0)));

    // Division by zero
    try testing.expect(std.math.isInf(precise.detDiv(5.0, 0.0)));
    try testing.expect(precise.detDiv(5.0, 0.0) > 0);
    try testing.expect(precise.detDiv(-5.0, 0.0) < 0);
}

fn testPreciseApproximation() !void {
    try testing.expect(precise.detApproxEq(1.0, 1.0001, 0.001));
    try testing.expect(!precise.detApproxEq(1.0, 1.1, 0.05));

    // Test with very small differences
    const a: f32 = 0.1 + 0.2;
    const b: f32 = 0.3;
    try testing.expect(precise.detApproxEq(a, b, 1e-6));
}

fn testSimdVectorOperations() !void {
    const a = [4]f32{ 1.0, 2.0, 3.0, 4.0 };
    const b = [4]f32{ 5.0, 6.0, 7.0, 8.0 };

    // Test basic operations
    const sum = simd.addVec4(a, b);
    const expected_sum = [4]f32{ 6.0, 8.0, 10.0, 12.0 };
    for (0..4) |i| {
        try testing.expect(sum[i] == expected_sum[i]);
    }

    const product = simd.mulVec4(a, b);
    const expected_product = [4]f32{ 5.0, 12.0, 21.0, 32.0 };
    for (0..4) |i| {
        try testing.expect(product[i] == expected_product[i]);
    }

    // Test dot product
    const dot = simd.dotVec4(a, b);
    try testing.expect(dot == 70.0); // 1*5 + 2*6 + 3*7 + 4*8 = 70

    // Test cross product (3D)
    const cross = simd.crossVec3(a, b);
    const expected_cross = [4]f32{ -4.0, 8.0, -4.0, 0.0 };
    for (0..4) |i| {
        try testing.expect(@abs(cross[i] - expected_cross[i]) < 0.001);
    }

    // Test length operations
    const len_sq = simd.lengthSquaredVec4(a);
    try testing.expect(len_sq == 30.0); // 1² + 2² + 3² + 4² = 30

    const len = simd.lengthVec4(a);
    try testing.expect(@abs(len - @sqrt(30.0)) < 0.001);
}

fn testSimdBatchOperations() !void {
    const a_batch = [_][4]f32{ .{ 1, 2, 3, 4 }, .{ 5, 6, 7, 8 } };
    const b_batch = [_][4]f32{ .{ 2, 3, 4, 5 }, .{ 6, 7, 8, 9 } };
    var result_batch: [2][4]f32 = undefined;

    simd.batchAddVec4(&a_batch, &b_batch, &result_batch);

    try testing.expect(result_batch[0][0] == 3.0); // 1 + 2
    try testing.expect(result_batch[0][1] == 5.0); // 2 + 3
    try testing.expect(result_batch[1][0] == 11.0); // 5 + 6
}

fn testSimdAdvancedOperations() !void {
    const a = [4]f32{ 2.0, 4.0, 6.0, 8.0 };

    // Test scaling
    const scaled = simd.scaleVec4(a, 0.5);
    const expected_scaled = [4]f32{ 1.0, 2.0, 3.0, 4.0 };
    for (0..4) |i| {
        try testing.expect(scaled[i] == expected_scaled[i]);
    }

    // Test normalization
    const normalized = simd.normalizeVec4(a);
    const normalized_length = simd.lengthVec4(normalized);
    try testing.expect(@abs(normalized_length - 1.0) < 0.001);

    // Test interpolation
    const b = [4]f32{ 10.0, 20.0, 30.0, 40.0 };
    const lerp_result = simd.lerpVec4(a, b, 0.5);
    const expected_lerp = [4]f32{ 6.0, 12.0, 18.0, 24.0 };
    for (0..4) |i| {
        try testing.expect(@abs(lerp_result[i] - expected_lerp[i]) < 0.001);
    }

    // Test min/max operations
    const min_result = simd.minVec4(a, b);
    for (0..4) |i| {
        try testing.expect(min_result[i] == a[i]); // a < b for all components
    }

    const max_result = simd.maxVec4(a, b);
    for (0..4) |i| {
        try testing.expect(max_result[i] == b[i]); // b > a for all components
    }
}

// ============================================================================
// CLIMATE MODULE TESTS
// ============================================================================

fn testClimateProcessingPipeline() !void {
    // Create test data for climate processing
    const positions = [_][2]f32{ .{ 50.0, 50.0 }, .{ 150.0, 150.0 }, .{ 100.0, 200.0 } };
    const elevations = [_]f32{ 100.0, 1500.0, 800.0 };
    const base_temps = [_]i8{ 20, 15, 18 };
    const base_rainfall = [_]f32{ 100.0, 200.0, 150.0 };
    const base_humidity = [_]u8{ 60, 70, 65 };
    const wind_directions = [_]f32{ 0.0, 0.0, 0.0 };

    var temp_results = [_]i8{ 0, 0, 0 };
    var rain_results = [_]f32{ 0.0, 0.0, 0.0 };
    var hum_results = [_]u8{ 0, 0, 0 };

    climate.simpleClimateProcessing(
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

    // Results should be different from base values (processing occurred)
    var temp_changed = false;
    var rain_changed = false;
    for (0..3) |i| {
        if (temp_results[i] != base_temps[i]) temp_changed = true;
        if (rain_results[i] != base_rainfall[i]) rain_changed = true;
    }

    // At least one result should be modified by climate processing
    try testing.expect(temp_changed or rain_changed);
}

fn testOrographicEffects() !void {
    const positions = [_][2]f32{ .{ 100.0, 100.0 }, .{ 200.0, 200.0 } };
    const elevations = [_]f32{ 0.0, 2000.0 }; // Sea level vs high elevation
    const base_rainfall = [_]f32{ 100.0, 100.0 };
    const wind_directions = [_]f32{ 0.0, 0.0 };
    const mountain_ranges: []climate.orographic.MountainRange = &.{};

    const params = climate.OrographicParams{
        .max_orographic_bonus = 200.0,
        .rain_shadow_factor = 0.6,
        .elevation_scale = 1.0,
        .wind_effect_scale = 1.0,
    };

    var results = [_]f32{ 0.0, 0.0 };

    climate.processOrographicOnly(
        positions[0..],
        elevations[0..],
        base_rainfall[0..],
        wind_directions[0..],
        mountain_ranges,
        params,
        results[0..],
    );

    // Higher elevation should generally have enhanced rainfall due to orographic lifting
    try testing.expect(results[1] >= results[0]);
}

fn testContinentalEffects() !void {
    // Test continental effects on temperature and humidity
    const positions = [_][2]f32{
        .{ 50.0, 50.0 }, // Near edge (maritime influence)
        .{ 128.0, 128.0 }, // Center (continental influence)
    };
    const base_temps = [_]i8{ 20, 20 };
    const base_humidity = [_]u8{ 60, 60 };

    const params = climate.ContinentalParams{
        .temperature_amplification = 1.5,
        .humidity_reduction = 0.8,
        .world_width = 256.0,
        .world_height = 256.0,
    };

    var temp_results = [_]i8{ 0, 0 };
    var humidity_results = [_]u8{ 0, 0 };

    climate.processContinentalOnly(
        positions[0..],
        base_temps[0..],
        base_humidity[0..],
        params,
        temp_results[0..],
        humidity_results[0..],
    );

    // Continental effects should modify at least some values
    const temp_modified = temp_results[0] != base_temps[0] or temp_results[1] != base_temps[1];
    const humidity_modified = humidity_results[0] != base_humidity[0] or humidity_results[1] != base_humidity[1];

    try testing.expect(temp_modified or humidity_modified);
}

fn testSeasonalVariations() !void {
    // Test that seasonal variations produce different results for different seasons
    const base_temps = [_]i8{ 15, 10, 20 };
    const climate_zones = [_]climate.ClimateZone{ climate.ClimateZone.Temperate, climate.ClimateZone.Arctic, climate.ClimateZone.Tropical };
    const latitudes = [_]f32{ 45.0, 75.0, 10.0 };

    const winter_state = climate.SeasonalState{
        .current_season = 0.0, // Winter
        .year_progress = 0.0,
        .hemisphere_modifier = 1.0,
    };

    const summer_state = climate.SeasonalState{
        .current_season = 0.5, // Summer
        .year_progress = 0.5,
        .hemisphere_modifier = 1.0,
    };

    const params = climate.SeasonalParams.default();

    var winter_results = [_]i8{ 0, 0, 0 };
    var summer_results = [_]i8{ 0, 0, 0 };

    climate.seasonal.batchSeasonalTemperature(
        base_temps[0..],
        climate_zones[0..],
        latitudes[0..],
        winter_state,
        params,
        winter_results[0..],
    );

    climate.seasonal.batchSeasonalTemperature(
        base_temps[0..],
        climate_zones[0..],
        latitudes[0..],
        summer_state,
        params,
        summer_results[0..],
    );

    // Summer and winter results should be different (seasonal variation applied)
    var seasonal_difference = false;
    for (0..3) |i| {
        if (winter_results[i] != summer_results[i]) {
            seasonal_difference = true;
            break;
        }
    }

    try testing.expect(seasonal_difference);
}

// ============================================================================
// HYDROLOGY MODULE TESTS
// ============================================================================

fn testHydraulicCalculations() !void {
    // Test Manning's equation calculations
    const area = 10.0; // m²
    const wetted_perimeter = 12.0; // m
    const slope = 0.001; // m/m
    const manning_n = 0.03; // Clean natural channel

    var velocity_result: f64 = undefined;
    var discharge_result: f64 = undefined;
    var hydraulic_radius_result: f64 = undefined;

    const results = hydrology.hydraulics.calculateManning(area, wetted_perimeter, slope, manning_n);
    velocity_result = results.velocity;
    discharge_result = results.discharge;
    hydraulic_radius_result = results.hydraulic_radius;

    // Verify hydraulic radius calculation
    try testing.expect(@abs(hydraulic_radius_result - (area / wetted_perimeter)) < 0.001);

    // Verify discharge = area × velocity
    try testing.expect(@abs(discharge_result - (area * velocity_result)) < 0.001);

    // Verify reasonable values
    try testing.expect(velocity_result > 0.0 and velocity_result < 10.0); // Reasonable flow velocity
    try testing.expect(discharge_result > 0.0); // Positive discharge

    // Test critical depth calculation
    const discharge = 15.0; // m³/s
    const width = 8.0; // m
    const gravity = 9.81; // m/s²

    const critical_depth = hydrology.hydraulics.calculateCriticalDepth(discharge, width, gravity);
    try testing.expect(critical_depth > 0.0 and critical_depth < 5.0); // Reasonable depth

    // Test Froude number
    const froude = hydrology.hydraulics.calculateFroude(velocity_result, critical_depth);
    try testing.expect(froude >= 0.0); // Froude number should be non-negative
}

fn testFlowAnalysis() !void {
    // Create a simple elevation grid for flow analysis
    const width = 5;
    const height = 5;
    const cell_size = 10.0;

    // Create a simple valley - higher on edges, lower in center
    var elevation_data = [_]f64{
        100, 90, 80, 90, 100,
        90,  70, 60, 70, 90,
        80,  60, 50, 60, 80, // Lowest point in center
        90,  70, 60, 70, 90,
        100, 90, 80, 90, 100,
    };

    var flow_grid = hydrology.flow.FlowGrid.init(width, height, cell_size, &elevation_data, TestAllocator) catch unreachable;
    defer flow_grid.deinit(TestAllocator);

    // Calculate flow directions
    flow_grid.calculateFlowDirections();

    // Calculate flow accumulation
    flow_grid.calculateFlowAccumulation(TestAllocator) catch unreachable;

    // The center cell (2,2) should have the highest flow accumulation
    const center_index = flow_grid.getIndex(2, 2);
    const center_accumulation = flow_grid.flow_accumulation[center_index];

    var max_accumulation: f64 = 0.0;
    var max_index: usize = 0;
    for (0..flow_grid.flow_accumulation.len) |i| {
        if (flow_grid.flow_accumulation[i] > max_accumulation) {
            max_accumulation = flow_grid.flow_accumulation[i];
            max_index = i;
        }
    }

    // The maximum accumulation should be at or near the center
    try testing.expect(max_index == center_index or center_accumulation >= max_accumulation * 0.8);
}

fn testGroundwaterModeling() !void {
    // Test basic groundwater calculations
    const hydraulic_conductivity = 1e-5; // m/s
    const head_gradient_x = 0.001; // m/m
    const head_gradient_y = 0.0005; // m/m

    var velocity_x: f64 = undefined;
    var velocity_y: f64 = undefined;
    var magnitude: f64 = undefined;

    const cell_data = hydrology.aquifers.AquiferCell.init(100.0, hydraulic_conductivity, 10.0, 0.3, .unconfined);
    const flow_vector = cell_data.calculateDarcyVelocity(head_gradient_x, head_gradient_y);

    velocity_x = flow_vector.velocity_x;
    velocity_y = flow_vector.velocity_y;
    magnitude = flow_vector.magnitude;

    // Darcy's law: v = -K * gradient
    const expected_vx = -hydraulic_conductivity * head_gradient_x;
    const expected_vy = -hydraulic_conductivity * head_gradient_y;

    try testing.expect(@abs(velocity_x - expected_vx) < 1e-10);
    try testing.expect(@abs(velocity_y - expected_vy) < 1e-10);

    const expected_magnitude = @sqrt(expected_vx * expected_vx + expected_vy * expected_vy);
    try testing.expect(@abs(magnitude - expected_magnitude) < 1e-10);

    // Test seepage velocity calculation
    const porosity = 0.3;
    var seepage_x: f64 = undefined;
    var seepage_y: f64 = undefined;

    const seepage_cell = hydrology.aquifers.AquiferCell.init(100.0, 1e-5, 10.0, porosity, .unconfined);
    const darcy_flow = hydrology.aquifers.FlowVector.init(velocity_x, velocity_y);
    const seepage_flow = seepage_cell.calculateSeepageVelocity(darcy_flow);

    seepage_x = seepage_flow.velocity_x;
    seepage_y = seepage_flow.velocity_y;

    // Seepage velocity = Darcy velocity / effective porosity (porosity * 0.8)
    const effective_porosity = porosity * 0.8;
    try testing.expect(@abs(seepage_x - (velocity_x / effective_porosity)) < 1e-10);
    try testing.expect(@abs(seepage_y - (velocity_y / effective_porosity)) < 1e-10);
}

fn testSpringGeneration() !void {
    // Test spring discharge calculation
    const head_difference = 50.0; // meters
    const aquifer_type = hydrology.aquifers.AquiferType.unconfined;

    const discharge = hydrology.aquifers.calculateSpringDischarge(head_difference, aquifer_type);
    try testing.expect(discharge > 0.0); // Springs should have positive discharge

    // Test seasonal discharge variation
    const base_discharge = 0.1; // m³/s
    const seasonal_variation = 0.3; // 30% variation
    const day_of_year = 180; // Mid-summer

    const spring_data = hydrology.aquifers.Spring{
        .x = 100.0,
        .y = 100.0,
        .elevation = 500.0,
        .discharge = base_discharge,
        .temperature = 15.0,
        .spring_type = .gravity,
        .aquifer_connection = .unconfined,
        .seasonal_variation = seasonal_variation,
    };

    const seasonal_discharge = spring_data.getSeasonalDischarge(day_of_year);

    // Seasonal discharge should be within reasonable bounds
    const min_expected = base_discharge * (1.0 - seasonal_variation);
    const max_expected = base_discharge * (1.0 + seasonal_variation);

    try testing.expect(seasonal_discharge >= min_expected and seasonal_discharge <= max_expected);
}

// ============================================================================
// TECTONICS MODULE TESTS
// ============================================================================

fn testPlateForceCalculations() !void {
    const plate = plates.TectonicPlate{
        .id = 1,
        .center = plates.Vec2.init(100.0, 200.0),
        .velocity = plates.Vec2.init(0.05, 0.02), // cm/year
        .age_million_years = 50.0,
        .density = 2700.0, // kg/m³
        .area = 1e12, // m²
    };

    const movement_speed = 2.0; // cm/year

    // Test ridge push calculation
    var result_x: f64 = undefined;
    var result_y: f64 = undefined;

    const ridge_force = plates.calculateRidgePush(&plate, movement_speed);
    result_x = ridge_force.x;
    result_y = ridge_force.y;

    // Force should be non-zero and reasonable magnitude
    const force_magnitude = @sqrt(result_x * result_x + result_y * result_y);
    try testing.expect(force_magnitude > 1e10 and force_magnitude < 1e15); // Reasonable tectonic forces

    // Test basal drag calculation
    const basal_force = plates.calculateBasalDrag(&plate);
    try testing.expect(basal_force.x != 0.0 or basal_force.y != 0.0); // Should produce resistance

    // Test mantle convection
    const convection_force = plates.calculateMantelConvection(&plate, movement_speed);
    try testing.expect(@abs(convection_force.x) < 1e14 and @abs(convection_force.y) < 1e14); // Bounded forces
}

fn testGeometryOperations() !void {
    // Test point-to-segment distance
    const point = geometry.Point2D.init(2.0, 2.0);
    const segment_start = geometry.Point2D.init(0.0, 0.0);
    const segment_end = geometry.Point2D.init(4.0, 0.0);
    const segment = geometry.LineSegment.init(segment_start, segment_end);

    const distance = geometry.pointToSegmentDistance(point, segment);
    try testing.expect(@abs(distance - 2.0) < 0.001); // Point (2,2) to line y=0 should be distance 2

    // Test polygon area calculation
    const vertices = [_]geometry.Point2D{
        geometry.Point2D.init(0.0, 0.0),
        geometry.Point2D.init(4.0, 0.0),
        geometry.Point2D.init(4.0, 3.0),
        geometry.Point2D.init(0.0, 3.0),
    };

    const polygon = geometry.Polygon.init(&vertices);
    const area = polygon.area();
    try testing.expect(@abs(area - 12.0) < 0.001); // 4×3 rectangle = 12 area units

    // Test point-in-polygon
    const test_point = geometry.Point2D.init(2.0, 1.5);
    try testing.expect(polygon.containsPoint(test_point)); // Inside rectangle

    const outside_point = geometry.Point2D.init(5.0, 1.5);
    try testing.expect(!polygon.containsPoint(outside_point)); // Outside rectangle
}

fn testStressCalculations() !void {
    // Test stress tensor calculations
    const stress_xx = 100e6; // 100 MPa
    const stress_yy = 50e6; // 50 MPa
    const stress_xy = 30e6; // 30 MPa shear

    const tensor = stress.StressTensor.init(stress_xx, stress_yy, stress_xy);

    // Test von Mises stress
    const von_mises = tensor.vonMisesStress();
    try testing.expect(von_mises > 0.0 and von_mises < 200e6); // Reasonable stress range

    // Test maximum principal stress
    const max_principal = tensor.maxPrincipalStress();
    try testing.expect(max_principal >= @max(stress_xx, stress_yy)); // Should be >= larger normal stress

    // Test principal stress angle
    const angle = tensor.principalStressAngle();
    try testing.expect(angle >= -std.math.pi / 2.0 and angle <= std.math.pi / 2.0); // Valid angle range
}

fn testVolcanicHazards() !void {
    const volcano = volcanic.Volcano{
        .x = 1000.0,
        .y = 2000.0,
        .elevation = 3000.0,
        .vei_scale = 4, // VEI 4 eruption
        .hazard_radius = 50000.0, // 50 km
        .magma_chamber_depth = 5000.0,
        .last_eruption_years_ago = 200.0,
        .eruption_probability = 0.7,
    };

    // Test pyroclastic flow hazard
    const target_x = 1200.0; // 200m from volcano
    const target_y = 2000.0;
    const wind_direction = 0.0; // East
    const wind_speed = 10.0; // m/s

    const pyroclastic_hazard = volcanic.calculatePyroclasticFlowHazard(&volcano, target_x, target_y, wind_direction, wind_speed);

    try testing.expect(pyroclastic_hazard >= 0.0 and pyroclastic_hazard <= 1.0); // Normalized hazard
    try testing.expect(pyroclastic_hazard > 0.8); // Close target should have high hazard

    // Test ash fall hazard
    const column_height = 15000.0; // 15 km high eruption column
    const ash_hazard = volcanic.calculateAshFallHazard(&volcano, target_x, target_y, wind_direction, wind_speed, column_height);

    try testing.expect(ash_hazard >= 0.0 and ash_hazard <= 1.0); // Normalized hazard

    // Test distant target has lower hazard
    const distant_x = 10000.0; // 10 km away
    const distant_ash = volcanic.calculateAshFallHazard(&volcano, distant_x, target_y, wind_direction, wind_speed, column_height);

    try testing.expect(distant_ash < ash_hazard); // Farther away should be lower hazard
}

// ============================================================================
// FFI INTEGRATION TESTS
// ============================================================================

fn testFfiHexOperations() !void {
    // Test FFI hex distance calculation
    const distance_result = @import("../src/lib.zig").manifest_hex_distance(0, 0, 3, 4);
    try testing.expect(distance_result == 7); // Manhattan distance in hex coordinates

    // Test FFI hex to pixel conversion
    var x: f32 = undefined;
    var y: f32 = undefined;

    @import("../src/lib.zig").manifest_hex_to_pixel(2, -1, 10.0, &x, &y);

    // Convert back and verify
    var q: i32 = undefined;
    var r: i32 = undefined;

    @import("../src/lib.zig").manifest_hex_from_pixel(x, y, 10.0, &q, &r);
    try testing.expect(q == 2 and r == -1); // Round trip should preserve coordinates

    // Test FFI neighbor operations
    var neighbors: [6]hex.HexCoord = undefined;
    @import("../src/lib.zig").manifest_hex_get_neighbors(0, 0, &neighbors);

    // Verify all neighbors are distance 1
    for (neighbors) |neighbor| {
        const dist = hex.distance(0, 0, neighbor.q, neighbor.r);
        try testing.expect(dist == 1);
    }
}

fn testFfiClimateBatchProcessing() !void {
    const count = 4;
    const positions_x = [_]f32{ 50.0, 150.0, 100.0, 200.0 };
    const positions_y = [_]f32{ 50.0, 150.0, 200.0, 100.0 };
    const elevations = [_]f32{ 100.0, 1500.0, 800.0, 2000.0 };
    const base_temperatures = [_]i8{ 20, 15, 18, 10 };
    const base_rainfall = [_]f32{ 100.0, 200.0, 150.0, 300.0 };
    const base_humidity = [_]u8{ 60, 70, 65, 80 };
    const wind_directions = [_]f32{ 0.0, 0.0, 0.0, 0.0 };

    var temperature_results = [_]i8{ 0, 0, 0, 0 };
    var rainfall_results = [_]f32{ 0.0, 0.0, 0.0, 0.0 };
    var humidity_results = [_]u8{ 0, 0, 0, 0 };

    @import("../src/lib.zig").manifest_climate_process_all(
        &positions_x,
        &positions_y,
        &elevations,
        &base_temperatures,
        &base_rainfall,
        &base_humidity,
        &wind_directions,
        count,
        &temperature_results,
        &rainfall_results,
        &humidity_results,
    );

    // Verify processing occurred (results different from base)
    var results_modified = false;
    for (0..count) |i| {
        if (temperature_results[i] != base_temperatures[i] or
            rainfall_results[i] != base_rainfall[i] or
            humidity_results[i] != base_humidity[i])
        {
            results_modified = true;
            break;
        }
    }

    try testing.expect(results_modified);
}

fn testFfiHydraulicsBatch() !void {
    const count = 3;
    const areas = [_]f64{ 10.0, 15.0, 8.0 };
    const wetted_perimeters = [_]f64{ 12.0, 18.0, 10.0 };
    const slopes = [_]f64{ 0.001, 0.002, 0.0005 };
    const manning_ns = [_]f64{ 0.03, 0.035, 0.025 };

    var velocities = [_]f64{ 0.0, 0.0, 0.0 };
    var discharges = [_]f64{ 0.0, 0.0, 0.0 };
    var hydraulic_radii = [_]f64{ 0.0, 0.0, 0.0 };

    @import("../src/lib.zig").manifest_batch_manning_calculations(
        &areas,
        &wetted_perimeters,
        &slopes,
        &manning_ns,
        &velocities,
        &discharges,
        &hydraulic_radii,
        count,
    );

    // Verify calculations produced valid results
    for (0..count) |i| {
        try testing.expect(velocities[i] > 0.0); // Positive velocity
        try testing.expect(discharges[i] > 0.0); // Positive discharge
        try testing.expect(hydraulic_radii[i] > 0.0); // Positive hydraulic radius

        // Verify Q = A × V relationship
        const expected_discharge = areas[i] * velocities[i];
        try testing.expect(@abs(discharges[i] - expected_discharge) < 0.001);
    }
}
