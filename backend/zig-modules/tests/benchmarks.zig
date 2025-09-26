//! Performance benchmarks for Zig modules
//!
//! Measures execution time and throughput of critical game engine operations
//! to ensure optimal performance in production gameplay.

const std = @import("std");
const testing = std.testing;
/// Timing utilities
const Timer = std.time.Timer;

const climate = @import("../src/climate/climate.zig");
const hex = @import("../src/math/hex.zig");
const precise = @import("../src/math/precise.zig");
const simd = @import("../src/simd/simd.zig");

// Import modules for benchmarking
/// Benchmark result tracking
const BenchmarkResult = struct {
    name: []const u8,
    iterations: u64,
    total_time_ns: u64,
    avg_time_ns: u64,
    operations_per_second: f64,
};

var benchmark_results = std.ArrayList(BenchmarkResult).init(std.testing.allocator);

/// Run a benchmark function multiple times and measure performance
fn runBenchmark(
    comptime benchmark_name: []const u8,
    benchmark_fn: *const fn (iteration: u64) void,
    iterations: u64,
) !void {
    std.debug.print("🏃 Running {s} benchmark ({} iterations)...\n", .{ benchmark_name, iterations });

    var timer = try Timer.start();
    const start_time = timer.read();

    for (0..iterations) |i| {
        benchmark_fn(i);
    }

    const end_time = timer.read();
    const total_time = end_time - start_time;
    const avg_time = total_time / iterations;
    const ops_per_second = @as(f64, @floatFromInt(iterations)) / (@as(f64, @floatFromInt(total_time)) / 1_000_000_000.0);

    try benchmark_results.append(BenchmarkResult{
        .name = benchmark_name,
        .iterations = iterations,
        .total_time_ns = total_time,
        .avg_time_ns = avg_time,
        .operations_per_second = ops_per_second,
    });

    std.debug.print("✅ {s}: {d:.2} ops/sec | {d:.2}μs avg\n", .{
        benchmark_name,
        ops_per_second,
        @as(f64, @floatFromInt(avg_time)) / 1000.0,
    });
}

/// Print benchmark summary
fn printBenchmarkSummary() !void {
    std.debug.print("\n" ++ "=" * 80 ++ "\n");
    std.debug.print("MANIFEST ZIG PERFORMANCE BENCHMARKS\n");
    std.debug.print("=" * 80 ++ "\n");

    std.debug.print("{s:<30} {s:<12} {s:<15} {s:<15}\n", .{ "Benchmark", "Iterations", "Ops/Second", "Avg Time (μs)" });
    std.debug.print("-" * 80 ++ "\n");

    for (benchmark_results.items) |result| {
        const avg_time_us = @as(f64, @floatFromInt(result.avg_time_ns)) / 1000.0;
        std.debug.print("{s:<30} {d:<12} {d:<15.0} {d:<15.2}\n", .{
            result.name,
            result.iterations,
            result.operations_per_second,
            avg_time_us,
        });
    }

    std.debug.print("=" * 80 ++ "\n\n");
}

pub fn main() !void {
    defer benchmark_results.deinit();

    std.debug.print("🚀 Starting Manifest Zig Performance Benchmarks\n\n");

    // Hex operations benchmarks
    try runBenchmark("Hex Distance Calculation", benchmarkHexDistance, 100_000);
    try runBenchmark("Hex to Pixel Conversion", benchmarkHexToPixel, 100_000);
    try runBenchmark("Hex Batch Operations", benchmarkHexBatch, 10_000);

    // SIMD operations benchmarks
    try runBenchmark("SIMD Vector Addition", benchmarkSimdAdd, 100_000);
    try runBenchmark("SIMD Vector Dot Product", benchmarkSimdDot, 100_000);
    try runBenchmark("SIMD Batch Processing", benchmarkSimdBatch, 10_000);

    // Precise math benchmarks
    try runBenchmark("Precise Math Operations", benchmarkPreciseMath, 100_000);

    // Climate system benchmarks
    try runBenchmark("Climate Processing", benchmarkClimate, 1_000);

    try printBenchmarkSummary();
}

// ============================================================================
// INDIVIDUAL BENCHMARKS
// ============================================================================

fn benchmarkHexDistance(iteration: u64) void {
    _ = iteration;
    // Calculate distance between various hex coordinates
    const coords = [_]struct { q1: i32, r1: i32, q2: i32, r2: i32 }{
        .{ .q1 = 0, .r1 = 0, .q2 = 10, .r2 = 5 },
        .{ .q1 = -5, .r1 = 3, .q2 = 8, .r2 = -2 },
        .{ .q1 = 15, .r1 = -10, .q2 = -3, .r2 = 7 },
        .{ .q1 = 0, .r1 = 0, .q2 = 0, .r2 = 0 },
    };

    for (coords) |coord_pair| {
        _ = hex.distance(coord_pair.q1, coord_pair.r1, coord_pair.q2, coord_pair.r2);
    }
}

fn benchmarkHexToPixel(iteration: u64) void {
    _ = iteration;
    const coords = [_]struct { q: i32, r: i32 }{
        .{ .q = 0, .r = 0 },
        .{ .q = 5, .r = -3 },
        .{ .q = -2, .r = 7 },
        .{ .q = 10, .r = 10 },
    };

    for (coords) |coord| {
        _ = hex.toPixel(coord.q, coord.r, 20.0);
    }
}

fn benchmarkHexBatch(iteration: u64) void {
    _ = iteration;
    const coords1 = [_]hex.HexCoord{
        hex.HexCoord.init(0, 0),
        hex.HexCoord.init(1, 1),
        hex.HexCoord.init(2, 2),
        hex.HexCoord.init(3, 3),
        hex.HexCoord.init(4, 4),
        hex.HexCoord.init(5, 5),
        hex.HexCoord.init(6, 6),
        hex.HexCoord.init(7, 7),
    };

    const coords2 = [_]hex.HexCoord{
        hex.HexCoord.init(8, 8),
        hex.HexCoord.init(7, 9),
        hex.HexCoord.init(6, 10),
        hex.HexCoord.init(5, 11),
        hex.HexCoord.init(4, 12),
        hex.HexCoord.init(3, 13),
        hex.HexCoord.init(2, 14),
        hex.HexCoord.init(1, 15),
    };

    var distances: [8]u32 = undefined;
    hex.batchDistances(&coords1, &coords2, &distances);

    // Also benchmark batch pixel conversion
    var pixels: [8]hex.PixelPos = undefined;
    hex.batchToPixel(&coords1, 15.0, &pixels);
}

fn benchmarkSimdAdd(iteration: u64) void {
    _ = iteration;
    const vectors = [_]struct { a: [4]f32, b: [4]f32 }{
        .{ .a = .{ 1.0, 2.0, 3.0, 4.0 }, .b = .{ 5.0, 6.0, 7.0, 8.0 } },
        .{ .a = .{ 2.5, 3.5, 4.5, 5.5 }, .b = .{ 1.5, 2.5, 3.5, 4.5 } },
        .{ .a = .{ -1.0, -2.0, 3.0, 4.0 }, .b = .{ 1.0, 2.0, -3.0, -4.0 } },
        .{ .a = .{ 0.1, 0.2, 0.3, 0.4 }, .b = .{ 0.9, 0.8, 0.7, 0.6 } },
    };

    for (vectors) |vec_pair| {
        _ = simd.addVec4(vec_pair.a, vec_pair.b);
    }
}

fn benchmarkSimdDot(iteration: u64) void {
    _ = iteration;
    const vectors = [_]struct { a: [4]f32, b: [4]f32 }{
        .{ .a = .{ 1.0, 2.0, 3.0, 4.0 }, .b = .{ 5.0, 6.0, 7.0, 8.0 } },
        .{ .a = .{ 2.5, 3.5, 4.5, 5.5 }, .b = .{ 1.5, 2.5, 3.5, 4.5 } },
        .{ .a = .{ -1.0, -2.0, 3.0, 4.0 }, .b = .{ 1.0, 2.0, -3.0, -4.0 } },
        .{ .a = .{ 0.1, 0.2, 0.3, 0.4 }, .b = .{ 0.9, 0.8, 0.7, 0.6 } },
    };

    for (vectors) |vec_pair| {
        _ = simd.dotVec4(vec_pair.a, vec_pair.b);
    }
}

fn benchmarkSimdBatch(iteration: u64) void {
    _ = iteration;
    const a_batch = [_][4]f32{
        .{ 1, 2, 3, 4 },
        .{ 5, 6, 7, 8 },
        .{ 9, 10, 11, 12 },
        .{ 13, 14, 15, 16 },
    };

    const b_batch = [_][4]f32{
        .{ 2, 3, 4, 5 },
        .{ 6, 7, 8, 9 },
        .{ 10, 11, 12, 13 },
        .{ 14, 15, 16, 17 },
    };

    var result_batch: [4][4]f32 = undefined;

    simd.batchAddVec4(&a_batch, &b_batch, &result_batch);
    simd.batchMulVec4(&a_batch, &b_batch, &result_batch);
}

fn benchmarkPreciseMath(iteration: u64) void {
    _ = iteration;
    const test_values = [_]struct { a: f32, b: f32 }{
        .{ .a = 1.5, .b = 2.7 },
        .{ .a = -3.2, .b = 4.8 },
        .{ .a = 0.001, .b = 1000.0 },
        .{ .a = std.math.pi, .b = std.math.e },
    };

    for (test_values) |vals| {
        _ = precise.detAdd(vals.a, vals.b);
        _ = precise.detMul(vals.a, vals.b);
        _ = precise.detDiv(vals.a, vals.b);
        _ = precise.detSqrt(@abs(vals.a));
        _ = precise.detSin(vals.a);
        _ = precise.detCos(vals.a);
    }
}

fn benchmarkClimate(iteration: u64) void {
    _ = iteration;
    const positions = [_][2]f32{
        .{ 50.0, 50.0 },
        .{ 150.0, 150.0 },
        .{ 100.0, 200.0 },
        .{ 200.0, 100.0 },
        .{ 75.0, 175.0 },
        .{ 175.0, 75.0 },
        .{ 125.0, 125.0 },
        .{ 25.0, 225.0 },
    };

    const elevations = [_]f32{ 100.0, 1500.0, 800.0, 2000.0, 500.0, 1200.0, 300.0, 1800.0 };
    const base_temps = [_]i8{ 20, 15, 18, 10, 22, 12, 25, 8 };
    const base_rainfall = [_]f32{ 100.0, 200.0, 150.0, 300.0, 120.0, 180.0, 90.0, 250.0 };
    const base_humidity = [_]u8{ 60, 70, 65, 80, 55, 75, 50, 85 };
    const wind_directions = [_]f32{ 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0 };

    var temp_results = [_]i8{ 0, 0, 0, 0, 0, 0, 0, 0 };
    var rain_results = [_]f32{ 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0 };
    var hum_results = [_]u8{ 0, 0, 0, 0, 0, 0, 0, 0 };

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
}
