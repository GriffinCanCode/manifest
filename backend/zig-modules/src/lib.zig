//! Zig SIMD optimizations for Manifest Game Engine
//!
//! Provides deterministic high-performance math operations for cross-platform
//! reproducible game simulations with SIMD acceleration.

const std = @import("std");

// Export main modules
pub const precise = @import("math/precise.zig");
pub const hex = @import("math/hex.zig");
pub const math = @import("math/math.zig");
pub const simd = @import("simd/simd.zig");

// C exports for Rust FFI
export fn manifest_det_add_f32(a: f32, b: f32) f32 {
    return precise.detAdd(a, b);
}

export fn manifest_det_mul_f32(a: f32, b: f32) f32 {
    return precise.detMul(a, b);
}

export fn manifest_det_div_f32(a: f32, b: f32) f32 {
    return precise.detDiv(a, b);
}

export fn manifest_det_sqrt_f32(a: f32) f32 {
    return precise.detSqrt(a);
}

// SIMD vector operations
export fn manifest_simd_add_4_f32(a: *const f32, b: *const f32, result: *f32) void {
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
export fn manifest_hex_distance(q1: i32, r1: i32, q2: i32, r2: i32) u32 {
    return hex.distance(q1, r1, q2, r2);
}

export fn manifest_hex_to_pixel(q: i32, r: i32, size: f32, x: *f32, y: *f32) void {
    const pos = hex.toPixel(q, r, size);
    x.* = pos.x;
    y.* = pos.y;
}

export fn manifest_hex_from_pixel(x: f32, y: f32, size: f32, q: *i32, r: *i32) void {
    const coord = hex.fromPixel(x, y, size);
    q.* = coord.q;
    r.* = coord.r;
}

export fn manifest_hex_get_neighbors(q: i32, r: i32, neighbors: *[6]hex.HexCoord) void {
    const coord = hex.HexCoord.init(q, r);
    const result = hex.getNeighbors(coord);
    neighbors.* = result;
}

export fn manifest_hex_get_neighbor(q: i32, r: i32, direction: u8, out_q: *i32, out_r: *i32) void {
    const coord = hex.HexCoord.init(q, r);
    const neighbor = hex.getNeighbor(coord, @intCast(direction));
    out_q.* = neighbor.q;
    out_r.* = neighbor.r;
}

export fn manifest_hex_batch_to_pixel(coords: [*]const hex.HexCoord, size: f32, pixels: [*]hex.PixelPos, count: usize) void {
    const coord_slice = coords[0..count];
    const pixel_slice = pixels[0..count];
    hex.batchToPixel(coord_slice, size, pixel_slice);
}

export fn manifest_hex_round_to_hex(q_f: f32, r_f: f32, q: *i32, r: *i32) void {
    const coord = hex.roundToHex(q_f, r_f);
    q.* = coord.q;
    r.* = coord.r;
}
