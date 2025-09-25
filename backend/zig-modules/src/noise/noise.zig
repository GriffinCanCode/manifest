//! High-performance SIMD noise generation for Manifest game engine
//!
//! Provides vectorized noise calculations with deterministic cross-platform
//! results using Zig's SIMD capabilities and optimized math operations.

const std = @import("std");
const math = std.math;
const Vector = std.meta.Vector;

/// Noise parameters for C FFI
pub const NoiseParams = extern struct {
    frequency: f32,
    amplitude: f32,
    octaves: u32,
    lacunarity: f32,
    persistence: f32,
    seed: u32,
};

/// Coordinate pair for C FFI
pub const CoordPair = extern struct {
    x: f32,
    y: f32,
};

/// Noise result for C FFI
pub const NoiseResult = extern struct {
    height: f32,
    temperature: f32,
    moisture: f32,
};

/// SIMD vector size - use 4 for good cross-platform support
const SIMD_WIDTH = 4;
const FloatVec = Vector(SIMD_WIDTH, f32);

/// Permutation table for deterministic noise
const PERM_SIZE = 256;
var perm_table: [PERM_SIZE * 2]u8 = undefined;
var perm_table_initialized = false;

/// Initialize permutation table with seed
fn initPermTable(seed: u32) void {
    if (perm_table_initialized) return;

    var rng = std.rand.DefaultPrng.init(seed);
    const random = rng.random();

    // Fill initial permutation
    for (0..PERM_SIZE) |i| {
        perm_table[i] = @intCast(i);
    }

    // Shuffle using Fisher-Yates
    var i: usize = PERM_SIZE - 1;
    while (i > 0) : (i -= 1) {
        const j = random.intRangeLessThan(usize, 0, i + 1);
        const temp = perm_table[i];
        perm_table[i] = perm_table[j];
        perm_table[j] = temp;
    }

    // Duplicate for overflow handling
    for (0..PERM_SIZE) |idx| {
        perm_table[PERM_SIZE + idx] = perm_table[idx];
    }

    perm_table_initialized = true;
}

/// Fast floor function for SIMD
inline fn fastFloor(x: f32) i32 {
    const xi = @as(i32, @intFromFloat(x));
    return if (x < 0 and x != @as(f32, @floatFromInt(xi))) xi - 1 else xi;
}

/// SIMD fast floor for vectors
inline fn fastFloorVec(x: FloatVec) Vector(SIMD_WIDTH, i32) {
    var result: Vector(SIMD_WIDTH, i32) = undefined;
    for (0..SIMD_WIDTH) |i| {
        result[i] = fastFloor(x[i]);
    }
    return result;
}

/// Fade function for smooth interpolation
inline fn fade(t: f32) f32 {
    return t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
}

/// SIMD fade function
inline fn fadeVec(t: FloatVec) FloatVec {
    const t3 = t * t * t;
    const factor = t * @as(FloatVec, @splat(6.0)) - @as(FloatVec, @splat(15.0));
    return t3 * (t * factor + @as(FloatVec, @splat(10.0)));
}

/// Linear interpolation
inline fn lerp(a: f32, b: f32, t: f32) f32 {
    return a + t * (b - a);
}

/// SIMD linear interpolation
inline fn lerpVec(a: FloatVec, b: FloatVec, t: FloatVec) FloatVec {
    return a + t * (b - a);
}

/// Gradient function for Perlin noise
inline fn grad(hash: u8, x: f32, y: f32) f32 {
    const h = hash & 3;
    const u = if (h < 2) x else -x;
    const v = if (h & 1 == 0) y else -y;
    return u + v;
}

/// SIMD Perlin noise calculation
fn perlinNoiseSIMD(x: FloatVec, y: FloatVec, params: *const NoiseParams) FloatVec {
    initPermTable(params.seed);

    const freq = @as(FloatVec, @splat(params.frequency));
    const fx = x * freq;
    const fy = y * freq;

    const xi = fastFloorVec(fx);
    const yi = fastFloorVec(fy);

    const xf = fx - @as(FloatVec, @floatFromInt(xi));
    const yf = fy - @as(FloatVec, @floatFromInt(yi));

    const u = fadeVec(xf);
    const v = fadeVec(yf);

    var result: FloatVec = undefined;

    for (0..SIMD_WIDTH) |i| {
        const x0 = @as(u8, @intCast(xi[i] & 255));
        const x1 = @as(u8, @intCast((xi[i] + 1) & 255));
        const y0 = @as(u8, @intCast(yi[i] & 255));
        const y1 = @as(u8, @intCast((yi[i] + 1) & 255));

        const p00 = perm_table[perm_table[x0] + y0];
        const p01 = perm_table[perm_table[x0] + y1];
        const p10 = perm_table[perm_table[x1] + y0];
        const p11 = perm_table[perm_table[x1] + y1];

        const g00 = grad(p00, xf[i], yf[i]);
        const g01 = grad(p01, xf[i], yf[i] - 1.0);
        const g10 = grad(p10, xf[i] - 1.0, yf[i]);
        const g11 = grad(p11, xf[i] - 1.0, yf[i] - 1.0);

        const nx0 = lerp(g00, g10, u[i]);
        const nx1 = lerp(g01, g11, u[i]);

        result[i] = lerp(nx0, nx1, v[i]) * params.amplitude;
    }

    return result;
}

/// Simplex noise implementation using SIMD
fn simplexNoiseSIMD(x: FloatVec, y: FloatVec, params: *const NoiseParams) FloatVec {
    initPermTable(params.seed);

    const freq = @as(FloatVec, @splat(params.frequency));
    const fx = x * freq;
    const fy = y * freq;

    // Simplex noise skewing factors
    const F2 = 0.366025403; // (sqrt(3) - 1) / 2
    const G2 = 0.211324865; // (3 - sqrt(3)) / 6

    const s = (fx + fy) * @as(FloatVec, @splat(F2));
    const i = fastFloorVec(fx + s);
    const j = fastFloorVec(fy + s);

    const t = @as(FloatVec, @floatFromInt(i + j)) * @as(FloatVec, @splat(G2));
    const X0 = @as(FloatVec, @floatFromInt(i)) - t;
    const Y0 = @as(FloatVec, @floatFromInt(j)) - t;

    const x0 = fx - X0;
    const y0 = fy - Y0;

    var result: FloatVec = undefined;

    for (0..SIMD_WIDTH) |idx| {
        // Determine which simplex we're in
        const i_offset = if (x0[idx] > y0[idx]) @as(i32, 1) else 0;
        const j_offset = if (x0[idx] > y0[idx]) @as(i32, 0) else 1;

        // Offsets for second (middle) and third (last) corners
        const x1 = x0[idx] - @as(f32, @floatFromInt(i_offset)) + G2;
        const y1 = y0[idx] - @as(f32, @floatFromInt(j_offset)) + G2;
        const x2 = x0[idx] - 1.0 + 2.0 * G2;
        const y2 = y0[idx] - 1.0 + 2.0 * G2;

        // Permutation indices
        const ii = @as(u8, @intCast(i[idx] & 255));
        const jj = @as(u8, @intCast(j[idx] & 255));

        // Calculate contributions from each corner
        var n0: f32 = 0.0;
        var n1: f32 = 0.0;
        var n2: f32 = 0.0;

        // First corner
        var t0 = 0.5 - x0[idx] * x0[idx] - y0[idx] * y0[idx];
        if (t0 >= 0) {
            t0 *= t0;
            const gi0 = perm_table[ii + perm_table[jj]];
            n0 = t0 * t0 * grad(gi0, x0[idx], y0[idx]);
        }

        // Second corner
        var t1 = 0.5 - x1 * x1 - y1 * y1;
        if (t1 >= 0) {
            t1 *= t1;
            const gi1 = perm_table[ii + @as(u8, @intCast(i_offset)) + perm_table[jj + @as(u8, @intCast(j_offset))]];
            n1 = t1 * t1 * grad(gi1, x1, y1);
        }

        // Third corner
        var t2 = 0.5 - x2 * x2 - y2 * y2;
        if (t2 >= 0) {
            t2 *= t2;
            const gi2 = perm_table[ii + 1 + perm_table[jj + 1]];
            n2 = t2 * t2 * grad(gi2, x2, y2);
        }

        // Add contributions and scale
        result[idx] = 70.0 * (n0 + n1 + n2) * params.amplitude;
    }

    return result;
}

/// Fractal Brownian Motion with SIMD
fn fbmNoiseSIMD(x: FloatVec, y: FloatVec, params: *const NoiseParams) FloatVec {
    var result = @as(FloatVec, @splat(0.0));
    var amplitude = @as(FloatVec, @splat(params.amplitude));
    var frequency = @as(FloatVec, @splat(params.frequency));

    for (0..params.octaves) |_| {
        const noise = simplexNoiseSIMD(x * frequency, y * frequency, params);
        result += noise * amplitude;

        amplitude *= @as(FloatVec, @splat(params.persistence));
        frequency *= @as(FloatVec, @splat(params.lacunarity));
    }

    return result;
}

/// Domain warping with SIMD
fn domainWarpSIMD(x: FloatVec, y: FloatVec, params: *const NoiseParams) struct { x: FloatVec, y: FloatVec } {
    const warp_x = simplexNoiseSIMD(x, y, params);
    const warp_y = simplexNoiseSIMD(x + @as(FloatVec, @splat(100.0)), y + @as(FloatVec, @splat(100.0)), params);

    return .{
        .x = x + warp_x * @as(FloatVec, @splat(params.amplitude * 0.01)),
        .y = y + warp_y * @as(FloatVec, @splat(params.amplitude * 0.01)),
    };
}

/// Ridged noise with SIMD
fn ridgedNoiseSIMD(x: FloatVec, y: FloatVec, params: *const NoiseParams) FloatVec {
    var result = @as(FloatVec, @splat(0.0));
    var amplitude = @as(FloatVec, @splat(params.amplitude));
    var frequency = @as(FloatVec, @splat(params.frequency));

    for (0..params.octaves) |_| {
        var noise = simplexNoiseSIMD(x * frequency, y * frequency, params);
        noise = @abs(noise);
        noise = @as(FloatVec, @splat(1.0)) - noise;
        noise = noise * noise;

        result += noise * amplitude;

        amplitude *= @as(FloatVec, @splat(params.persistence));
        frequency *= @as(FloatVec, @splat(params.lacunarity));
    }

    return result;
}

/// Process coordinates in SIMD batches
fn processCoordBatch(coords: []const CoordPair, start: usize, params: *const NoiseParams, noise_func: fn (FloatVec, FloatVec, *const NoiseParams) FloatVec) [SIMD_WIDTH]f32 {
    var x_vec: FloatVec = undefined;
    var y_vec: FloatVec = undefined;

    for (0..SIMD_WIDTH) |i| {
        const idx = start + i;
        if (idx < coords.len) {
            x_vec[i] = coords[idx].x;
            y_vec[i] = coords[idx].y;
        } else {
            x_vec[i] = 0.0;
            y_vec[i] = 0.0;
        }
    }

    const result = noise_func(x_vec, y_vec, params);
    return @as([SIMD_WIDTH]f32, result);
}

//
// C Export Functions for Rust FFI
//

export fn manifest_noise_simplex_batch_simd(
    coords: [*]const CoordPair,
    params: *const NoiseParams,
    results: [*]f32,
    count: u32,
) void {
    const coord_slice = coords[0..count];
    const result_slice = results[0..count];

    var i: usize = 0;
    while (i < count) : (i += SIMD_WIDTH) {
        const batch_results = processCoordBatch(coord_slice, i, params, simplexNoiseSIMD);

        for (0..SIMD_WIDTH) |j| {
            const idx = i + j;
            if (idx < count) {
                result_slice[idx] = batch_results[j];
            }
        }
    }
}

export fn manifest_noise_perlin_batch_simd(
    coords: [*]const CoordPair,
    params: *const NoiseParams,
    results: [*]f32,
    count: u32,
) void {
    const coord_slice = coords[0..count];
    const result_slice = results[0..count];

    var i: usize = 0;
    while (i < count) : (i += SIMD_WIDTH) {
        const batch_results = processCoordBatch(coord_slice, i, params, perlinNoiseSIMD);

        for (0..SIMD_WIDTH) |j| {
            const idx = i + j;
            if (idx < count) {
                result_slice[idx] = batch_results[j];
            }
        }
    }
}

export fn manifest_noise_fbm_batch_simd(
    coords: [*]const CoordPair,
    params: *const NoiseParams,
    results: [*]f32,
    count: u32,
) void {
    const coord_slice = coords[0..count];
    const result_slice = results[0..count];

    var i: usize = 0;
    while (i < count) : (i += SIMD_WIDTH) {
        const batch_results = processCoordBatch(coord_slice, i, params, fbmNoiseSIMD);

        for (0..SIMD_WIDTH) |j| {
            const idx = i + j;
            if (idx < count) {
                result_slice[idx] = batch_results[j];
            }
        }
    }
}

export fn manifest_noise_domain_warp_batch_simd(
    coords: [*]const CoordPair,
    warp_params: *const NoiseParams,
    warped_coords: [*]CoordPair,
    count: u32,
) void {
    const coord_slice = coords[0..count];
    const warped_slice = warped_coords[0..count];

    var i: usize = 0;
    while (i < count) : (i += SIMD_WIDTH) {
        var x_vec: FloatVec = undefined;
        var y_vec: FloatVec = undefined;

        for (0..SIMD_WIDTH) |j| {
            const idx = i + j;
            if (idx < count) {
                x_vec[j] = coord_slice[idx].x;
                y_vec[j] = coord_slice[idx].y;
            } else {
                x_vec[j] = 0.0;
                y_vec[j] = 0.0;
            }
        }

        const warped = domainWarpSIMD(x_vec, y_vec, warp_params);

        for (0..SIMD_WIDTH) |j| {
            const idx = i + j;
            if (idx < count) {
                warped_slice[idx].x = warped.x[j];
                warped_slice[idx].y = warped.y[j];
            }
        }
    }
}

export fn manifest_noise_ridged_batch_simd(
    coords: [*]const CoordPair,
    params: *const NoiseParams,
    results: [*]f32,
    count: u32,
) void {
    const coord_slice = coords[0..count];
    const result_slice = results[0..count];

    var i: usize = 0;
    while (i < count) : (i += SIMD_WIDTH) {
        const batch_results = processCoordBatch(coord_slice, i, params, ridgedNoiseSIMD);

        for (0..SIMD_WIDTH) |j| {
            const idx = i + j;
            if (idx < count) {
                result_slice[idx] = batch_results[j];
            }
        }
    }
}

export fn manifest_noise_mix_batch_simd(
    noise1: [*]const f32,
    noise2: [*]const f32,
    weights: [*]const f32,
    results: [*]f32,
    count: u32,
    operation: i32,
) void {
    const n1_slice = noise1[0..count];
    const n2_slice = noise2[0..count];
    const w_slice = weights[0..count];
    const result_slice = results[0..count];

    var i: usize = 0;
    while (i < count) : (i += SIMD_WIDTH) {
        var n1_vec: FloatVec = undefined;
        var n2_vec: FloatVec = undefined;
        var w_vec: FloatVec = undefined;

        for (0..SIMD_WIDTH) |j| {
            const idx = i + j;
            if (idx < count) {
                n1_vec[j] = n1_slice[idx];
                n2_vec[j] = n2_slice[idx];
                w_vec[j] = w_slice[idx];
            } else {
                n1_vec[j] = 0.0;
                n2_vec[j] = 0.0;
                w_vec[j] = 0.0;
            }
        }

        const mixed = switch (operation) {
            0 => n1_vec + n2_vec * w_vec, // Add
            1 => n1_vec * (n2_vec * w_vec + @as(FloatVec, @splat(1.0))), // Multiply
            2 => @max(n1_vec, n2_vec), // Max
            3 => @min(n1_vec, n2_vec), // Min
            4 => n1_vec * (@as(FloatVec, @splat(1.0)) - w_vec) + n2_vec * w_vec, // Blend
            else => n1_vec,
        };

        for (0..SIMD_WIDTH) |j| {
            const idx = i + j;
            if (idx < count) {
                result_slice[idx] = mixed[j];
            }
        }
    }
}

export fn manifest_noise_multilayer_batch_simd(
    coords: [*]const CoordPair,
    height_params: *const NoiseParams,
    temp_params: *const NoiseParams,
    moisture_params: *const NoiseParams,
    results: [*]NoiseResult,
    count: u32,
) void {
    const coord_slice = coords[0..count];
    const result_slice = results[0..count];

    var i: usize = 0;
    while (i < count) : (i += SIMD_WIDTH) {
        const height_batch = processCoordBatch(coord_slice, i, height_params, simplexNoiseSIMD);
        const temp_batch = processCoordBatch(coord_slice, i, temp_params, fbmNoiseSIMD);
        const moisture_batch = processCoordBatch(coord_slice, i, moisture_params, ridgedNoiseSIMD);

        for (0..SIMD_WIDTH) |j| {
            const idx = i + j;
            if (idx < count) {
                result_slice[idx].height = height_batch[j];
                result_slice[idx].temperature = temp_batch[j] * 0.5 + 0.5; // Normalize to 0-1
                result_slice[idx].moisture = moisture_batch[j] * 0.5 + 0.5; // Normalize to 0-1
            }
        }
    }
}
