//! SIMD vector operations for high-performance math
//!
//! Provides vectorized operations using Zig's @Vector builtin for optimal performance
//! while maintaining deterministic behavior across platforms.

const std = @import("std");
const precise = @import("../math/precise.zig");

/// 4-element f32 vector type
pub const Vec4 = @Vector(4, f32);

/// SIMD addition of 4-element vectors
pub fn addVec4(a: [4]f32, b: [4]f32) [4]f32 {
    const va: Vec4 = a;
    const vb: Vec4 = b;
    const result: Vec4 = va + vb;
    return result;
}

/// SIMD multiplication of 4-element vectors
pub fn mulVec4(a: [4]f32, b: [4]f32) [4]f32 {
    const va: Vec4 = a;
    const vb: Vec4 = b;
    const result: Vec4 = va * vb;
    return result;
}

/// SIMD subtraction of 4-element vectors
pub fn subVec4(a: [4]f32, b: [4]f32) [4]f32 {
    const va: Vec4 = a;
    const vb: Vec4 = b;
    const result: Vec4 = va - vb;
    return result;
}

/// SIMD division of 4-element vectors
pub fn divVec4(a: [4]f32, b: [4]f32) [4]f32 {
    const va: Vec4 = a;
    const vb: Vec4 = b;
    const result: Vec4 = va / vb;
    return result;
}

/// SIMD dot product of 4-element vectors
pub fn dotVec4(a: [4]f32, b: [4]f32) f32 {
    const va: Vec4 = a;
    const vb: Vec4 = b;
    const mul_result: Vec4 = va * vb;

    // Horizontal add using deterministic precise operations
    return precise.detAdd(precise.detAdd(mul_result[0], mul_result[1]), precise.detAdd(mul_result[2], mul_result[3]));
}

/// SIMD cross product (using first 3 elements)
pub fn crossVec3(a: [4]f32, b: [4]f32) [4]f32 {
    return .{
        precise.detSub(precise.detMul(a[1], b[2]), precise.detMul(a[2], b[1])),
        precise.detSub(precise.detMul(a[2], b[0]), precise.detMul(a[0], b[2])),
        precise.detSub(precise.detMul(a[0], b[1]), precise.detMul(a[1], b[0])),
        0.0, // W component set to 0
    };
}

/// SIMD vector length squared (avoids sqrt for performance)
pub fn lengthSquaredVec4(a: [4]f32) f32 {
    return dotVec4(a, a);
}

/// SIMD vector length
pub fn lengthVec4(a: [4]f32) f32 {
    return precise.detSqrt(lengthSquaredVec4(a));
}

/// SIMD vector normalization
pub fn normalizeVec4(a: [4]f32) [4]f32 {
    const len = lengthVec4(a);
    if (len == 0.0) return .{ 0.0, 0.0, 0.0, 0.0 };

    const inv_len = precise.detDiv(1.0, len);
    return .{
        precise.detMul(a[0], inv_len),
        precise.detMul(a[1], inv_len),
        precise.detMul(a[2], inv_len),
        precise.detMul(a[3], inv_len),
    };
}

/// SIMD vector scaling by scalar
pub fn scaleVec4(a: [4]f32, scalar: f32) [4]f32 {
    const vs: Vec4 = @splat(scalar);
    const va: Vec4 = a;
    const result: Vec4 = va * vs;
    return result;
}

/// SIMD linear interpolation
pub fn lerpVec4(a: [4]f32, b: [4]f32, t: f32) [4]f32 {
    const clamped_t = precise.detClamp(t, 0.0, 1.0);
    const one_minus_t = precise.detSub(1.0, clamped_t);

    return .{
        precise.detAdd(precise.detMul(a[0], one_minus_t), precise.detMul(b[0], clamped_t)),
        precise.detAdd(precise.detMul(a[1], one_minus_t), precise.detMul(b[1], clamped_t)),
        precise.detAdd(precise.detMul(a[2], one_minus_t), precise.detMul(b[2], clamped_t)),
        precise.detAdd(precise.detMul(a[3], one_minus_t), precise.detMul(b[3], clamped_t)),
    };
}

/// SIMD min/max operations for bounds checking
pub fn minVec4(a: [4]f32, b: [4]f32) [4]f32 {
    return .{
        precise.detMin(a[0], b[0]),
        precise.detMin(a[1], b[1]),
        precise.detMin(a[2], b[2]),
        precise.detMin(a[3], b[3]),
    };
}

pub fn maxVec4(a: [4]f32, b: [4]f32) [4]f32 {
    return .{
        precise.detMax(a[0], b[0]),
        precise.detMax(a[1], b[1]),
        precise.detMax(a[2], b[2]),
        precise.detMax(a[3], b[3]),
    };
}

/// Batch process arrays of vectors (for large-scale operations)
pub fn batchAddVec4(a: []const [4]f32, b: []const [4]f32, result: []([4]f32)) void {
    std.debug.assert(a.len == b.len and b.len == result.len);

    for (a, b, result) |va, vb, *vr| {
        vr.* = addVec4(va, vb);
    }
}

pub fn batchMulVec4(a: []const [4]f32, b: []const [4]f32, result: []([4]f32)) void {
    std.debug.assert(a.len == b.len and b.len == result.len);

    for (a, b, result) |va, vb, *vr| {
        vr.* = mulVec4(va, vb);
    }
}

// Tests
test "SIMD vector operations" {
    const testing = std.testing;

    const a = [4]f32{ 1.0, 2.0, 3.0, 4.0 };
    const b = [4]f32{ 5.0, 6.0, 7.0, 8.0 };

    // Test addition
    const sum = addVec4(a, b);
    try testing.expect(sum[0] == 6.0);
    try testing.expect(sum[1] == 8.0);
    try testing.expect(sum[2] == 10.0);
    try testing.expect(sum[3] == 12.0);

    // Test dot product
    const dot = dotVec4(a, b);
    try testing.expect(dot == 70.0); // 1*5 + 2*6 + 3*7 + 4*8 = 5 + 12 + 21 + 32 = 70

    // Test length
    const len_sq = lengthSquaredVec4(a);
    try testing.expect(len_sq == 30.0); // 1 + 4 + 9 + 16 = 30
}
