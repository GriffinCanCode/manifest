//! Mathematical utilities and constants
//!
//! Common mathematical functions and constants used throughout the engine.

const std = @import("std");
const precise = @import("precise.zig");

/// Mathematical constants with high precision
pub const PI: f32 = std.math.pi;
pub const TAU: f32 = 2.0 * std.math.pi;
pub const E: f32 = std.math.e;
pub const SQRT2: f32 = std.math.sqrt2;
pub const SQRT3: f32 = 1.7320508075688772;
pub const INV_SQRT2: f32 = 1.0 / std.math.sqrt2;
pub const INV_SQRT3: f32 = 1.0 / SQRT3;

/// Degrees to radians conversion
pub fn degToRad(degrees: f32) f32 {
    return precise.detMul(degrees, PI / 180.0);
}

/// Radians to degrees conversion
pub fn radToDeg(radians: f32) f32 {
    return precise.detMul(radians, 180.0 / PI);
}

/// Fast modulo operation for positive numbers
pub fn fastMod(value: f32, divisor: f32) f32 {
    return precise.detSub(value, precise.detMul(divisor, @floor(precise.detDiv(value, divisor))));
}

/// Smooth step function (0 at x=0, 1 at x=1)
pub fn smoothStep(x: f32) f32 {
    const clamped = precise.detClamp(x, 0.0, 1.0);
    return precise.detMul(precise.detMul(clamped, clamped), precise.detSub(3.0, precise.detMul(2.0, clamped)));
}

/// Smoother step function (more gradual acceleration/deceleration)
pub fn smootherStep(x: f32) f32 {
    const clamped = precise.detClamp(x, 0.0, 1.0);
    const x2 = precise.detMul(clamped, clamped);
    const x3 = precise.detMul(x2, clamped);

    return precise.detAdd(precise.detMul(x3, precise.detMul(clamped, precise.detSub(precise.detMul(clamped, 70.0), 315.0))), precise.detAdd(precise.detMul(x3, 540.0), precise.detMul(x2, -420.0)));
}

/// Exponential ease-out function
pub fn easeOut(x: f32) f32 {
    return precise.detSub(1.0, std.math.pow(f32, precise.detSub(1.0, precise.detClamp(x, 0.0, 1.0)), 3.0));
}

/// Exponential ease-in function
pub fn easeIn(x: f32) f32 {
    const clamped = precise.detClamp(x, 0.0, 1.0);
    return std.math.pow(f32, clamped, 3.0);
}

/// Exponential ease-in-out function
pub fn easeInOut(x: f32) f32 {
    const clamped = precise.detClamp(x, 0.0, 1.0);
    if (clamped < 0.5) {
        return precise.detMul(4.0, std.math.pow(f32, clamped, 3.0));
    } else {
        const shifted = precise.detSub(precise.detMul(2.0, clamped), 2.0);
        return precise.detSub(1.0, precise.detMul(0.5, std.math.pow(f32, shifted, 3.0)));
    }
}

/// Fast inverse square root approximation (Quake algorithm)
pub fn fastInvSqrt(x: f32) f32 {
    if (x <= 0.0) return 0.0;

    const threehalfs = 1.5;
    const x2 = precise.detMul(x, 0.5);
    var y = x;

    // Convert to integer for bit manipulation
    var i = @as(i32, @bitCast(y));
    i = 0x5f3759df - (i >> 1); // Magic number
    y = @bitCast(i);

    // Newton-Raphson iteration
    y = precise.detMul(y, precise.detSub(threehalfs, precise.detMul(x2, precise.detMul(y, y))));

    return y;
}

/// Check if a value is a power of two
pub fn isPowerOfTwo(value: u32) bool {
    return (value != 0) and ((value & (value - 1)) == 0);
}

/// Next power of two greater than or equal to value
pub fn nextPowerOfTwo(value: u32) u32 {
    if (value == 0) return 1;

    var n = value - 1;
    n |= n >> 1;
    n |= n >> 2;
    n |= n >> 4;
    n |= n >> 8;
    n |= n >> 16;

    return n + 1;
}

/// Linear interpolation between two values
pub fn lerp(a: f32, b: f32, t: f32) f32 {
    return precise.detLerp(a, b, t);
}

/// Bilinear interpolation
pub fn bilerp(a: f32, b: f32, c: f32, d: f32, tx: f32, ty: f32) f32 {
    const x1 = lerp(a, b, tx);
    const x2 = lerp(c, d, tx);
    return lerp(x1, x2, ty);
}

/// Wrap angle to [0, 2π) range
pub fn wrapAngle(angle: f32) f32 {
    return fastMod(angle, TAU);
}

/// Wrap angle to [-π, π) range
pub fn wrapAngleSigned(angle: f32) f32 {
    const wrapped = wrapAngle(angle);
    return if (wrapped > PI) wrapped - TAU else wrapped;
}

/// Shortest angular distance between two angles
pub fn angleDifference(a: f32, b: f32) f32 {
    const diff = wrapAngleSigned(precise.detSub(b, a));
    return diff;
}

/// Angular lerp (handles wrap-around)
pub fn lerpAngle(a: f32, b: f32, t: f32) f32 {
    const diff = angleDifference(a, b);
    return wrapAngle(precise.detAdd(a, precise.detMul(diff, precise.detClamp(t, 0.0, 1.0))));
}

/// 2D vector magnitude
pub fn vec2Length(x: f32, y: f32) f32 {
    return precise.detSqrt(precise.detAdd(precise.detMul(x, x), precise.detMul(y, y)));
}

/// 2D vector normalization
pub fn vec2Normalize(x: f32, y: f32) struct { x: f32, y: f32 } {
    const len = vec2Length(x, y);
    if (len == 0.0) return .{ .x = 0.0, .y = 0.0 };

    const inv_len = precise.detDiv(1.0, len);
    return .{ .x = precise.detMul(x, inv_len), .y = precise.detMul(y, inv_len) };
}

/// 2D dot product
pub fn vec2Dot(x1: f32, y1: f32, x2: f32, y2: f32) f32 {
    return precise.detAdd(precise.detMul(x1, x2), precise.detMul(y1, y2));
}

/// 2D cross product (returns scalar)
pub fn vec2Cross(x1: f32, y1: f32, x2: f32, y2: f32) f32 {
    return precise.detSub(precise.detMul(x1, y2), precise.detMul(y1, x2));
}

// Tests
test "math utilities" {
    const testing = std.testing;

    // Test angle operations
    const rad = degToRad(180.0);
    try testing.expect(precise.detApproxEq(rad, PI, 0.0001));

    const deg = radToDeg(PI);
    try testing.expect(precise.detApproxEq(deg, 180.0, 0.0001));

    // Test power of two
    try testing.expect(isPowerOfTwo(8));
    try testing.expect(!isPowerOfTwo(7));
    try testing.expect(nextPowerOfTwo(7) == 8);

    // Test vector operations
    const len = vec2Length(3.0, 4.0);
    try testing.expect(precise.detApproxEq(len, 5.0, 0.0001));

    const norm = vec2Normalize(3.0, 4.0);
    try testing.expect(precise.detApproxEq(norm.x, 0.6, 0.0001));
    try testing.expect(precise.detApproxEq(norm.y, 0.8, 0.0001));
}
