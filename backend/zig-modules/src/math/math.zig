//! Mathematical utilities and constants for the Manifest game engine
//!
//! Provides commonly used mathematical functions, constants, and utilities
//! that are used across the engine's mathematical systems.

const std = @import("std");
/// Mathematical constants
pub const PI = std.math.pi;
pub const E = std.math.e;

const precise = @import("precise.zig");

pub const TAU = 2.0 * PI;
/// Common mathematical functions with deterministic behavior
pub const det = struct {
    pub const add = precise.detAdd;
    pub const sub = precise.detSub;
    pub const mul = precise.detMul;
    pub const div = precise.detDiv;
    pub const sqrt = precise.detSqrt;
    pub const sin = precise.detSin;
    pub const cos = precise.detCos;
    pub const atan2 = precise.detAtan2;
    pub const min = precise.detMin;
    pub const max = precise.detMax;
    pub const clamp = precise.detClamp;
    pub const lerp = precise.detLerp;
    pub const approxEq = precise.detApproxEq;
};

/// Angle conversion utilities
pub fn degreesToRadians(degrees: f32) f32 {
    return precise.detMul(degrees, PI / 180.0);
}

pub fn radiansToDegrees(radians: f32) f32 {
    return precise.detMul(radians, 180.0 / PI);
}

/// Fast inverse square root (deterministic version)
pub fn fastInvSqrt(x: f32) f32 {
    if (x <= 0.0) return 0.0;
    return precise.detDiv(1.0, precise.detSqrt(x));
}

/// Smoothstep interpolation function
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) f32 {
    const t = precise.detClamp(precise.detDiv(precise.detSub(x, edge0), precise.detSub(edge1, edge0)), 0.0, 1.0);
    return precise.detMul(precise.detMul(t, t), precise.detSub(3.0, precise.detMul(2.0, t)));
}

/// Check if a number is power of 2
pub fn isPowerOfTwo(x: u32) bool {
    return x != 0 and (x & (x - 1)) == 0;
}

/// Next power of 2
pub fn nextPowerOfTwo(x: u32) u32 {
    var n = x;
    n -= 1;
    n |= n >> 1;
    n |= n >> 2;
    n |= n >> 4;
    n |= n >> 8;
    n |= n >> 16;
    n += 1;
    return n;
}

/// Wrap angle to [-π, π] range
pub fn wrapAngle(angle: f32) f32 {
    var wrapped = precise.detSub(angle, precise.detMul(TAU, @floor(precise.detDiv(precise.detAdd(angle, PI), TAU))));
    if (wrapped > PI) wrapped = precise.detSub(wrapped, TAU);
    if (wrapped < -PI) wrapped = precise.detAdd(wrapped, TAU);
    return wrapped;
}

/// 2D vector operations
pub const Vec2 = struct {
    x: f32,
    y: f32,

    pub fn init(x: f32, y: f32) Vec2 {
        return Vec2{ .x = x, .y = y };
    }

    pub fn add(self: Vec2, other: Vec2) Vec2 {
        return Vec2{
            .x = precise.detAdd(self.x, other.x),
            .y = precise.detAdd(self.y, other.y),
        };
    }

    pub fn sub(self: Vec2, other: Vec2) Vec2 {
        return Vec2{
            .x = precise.detSub(self.x, other.x),
            .y = precise.detSub(self.y, other.y),
        };
    }

    pub fn scale(self: Vec2, scalar: f32) Vec2 {
        return Vec2{
            .x = precise.detMul(self.x, scalar),
            .y = precise.detMul(self.y, scalar),
        };
    }

    pub fn dot(self: Vec2, other: Vec2) f32 {
        return precise.detAdd(precise.detMul(self.x, other.x), precise.detMul(self.y, other.y));
    }

    pub fn lengthSquared(self: Vec2) f32 {
        return self.dot(self);
    }

    pub fn length(self: Vec2) f32 {
        return precise.detSqrt(self.lengthSquared());
    }

    pub fn normalize(self: Vec2) Vec2 {
        const len = self.length();
        if (len == 0.0) return Vec2{ .x = 0.0, .y = 0.0 };
        const inv_len = precise.detDiv(1.0, len);
        return self.scale(inv_len);
    }
};

// Tests
test "math utilities" {
    const testing = std.testing;

    // Test angle conversion
    const rad = degreesToRadians(90.0);
    try testing.expect(precise.detApproxEq(rad, PI / 2.0, 0.001));

    const deg = radiansToDegrees(PI);
    try testing.expect(precise.detApproxEq(deg, 180.0, 0.001));

    // Test power of 2
    try testing.expect(isPowerOfTwo(8));
    try testing.expect(!isPowerOfTwo(7));

    try testing.expect(nextPowerOfTwo(7) == 8);
    try testing.expect(nextPowerOfTwo(8) == 8);

    // Test Vec2 operations
    const v1 = Vec2.init(1.0, 2.0);
    const v2 = Vec2.init(3.0, 4.0);

    const sum = v1.add(v2);
    try testing.expect(sum.x == 4.0 and sum.y == 6.0);

    const dot_product = v1.dot(v2);
    try testing.expect(dot_product == 11.0); // 1*3 + 2*4 = 11
}
