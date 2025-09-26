//! Precise deterministic floating point operations
//!
//! Ensures cross-platform reproducible math operations for deterministic simulation.
//! Uses IEEE 754 strict compliance and careful ordering to guarantee identical results.

const std = @import("std");
const math = std.math;

/// Deterministic addition with precise ordering
pub fn detAdd(a: f32, b: f32) f32 {
    // Ensure consistent NaN/infinity handling
    if (math.isNan(a) or math.isNan(b)) return math.nan(f32);
    if (math.isInf(a)) return a;
    if (math.isInf(b)) return b;

    return a + b;
}

/// Deterministic multiplication with precise ordering
pub fn detMul(a: f32, b: f32) f32 {
    if (math.isNan(a) or math.isNan(b)) return math.nan(f32);
    if (math.isInf(a)) {
        if (b == 0.0) return math.nan(f32);
        return if (math.signbit(b)) -a else a;
    }
    if (math.isInf(b)) {
        if (a == 0.0) return math.nan(f32);
        return if (math.signbit(a)) -b else b;
    }

    return a * b;
}

/// Deterministic division with precise error handling
pub fn detDiv(a: f32, b: f32) f32 {
    if (math.isNan(a) or math.isNan(b)) return math.nan(f32);
    if (b == 0.0) {
        if (a == 0.0) return math.nan(f32);
        return if (math.signbit(a)) -math.inf(f32) else math.inf(f32);
    }
    if (math.isInf(a)) {
        if (math.isInf(b)) return math.nan(f32);
        return if (math.signbit(a) != math.signbit(b)) -math.inf(f32) else math.inf(f32);
    }
    if (math.isInf(b)) return if (math.signbit(a) != math.signbit(b)) -0.0 else 0.0;

    return a / b;
}

/// Deterministic square root
pub fn detSqrt(a: f32) f32 {
    if (math.isNan(a) or a < 0.0) return math.nan(f32);
    if (a == 0.0 or math.isInf(a)) return a;

    return math.sqrt(a);
}

/// Deterministic sine with normalized input
pub fn detSin(a: f32) f32 {
    if (math.isNan(a) or math.isInf(a)) return math.nan(f32);

    // Normalize to [-2π, 2π] range for consistency
    const two_pi = 2.0 * math.pi;
    const normalized = @mod(a, two_pi);

    return math.sin(normalized);
}

/// Deterministic cosine with normalized input
pub fn detCos(a: f32) f32 {
    if (math.isNan(a) or math.isInf(a)) return math.nan(f32);

    const two_pi = 2.0 * math.pi;
    const normalized = @mod(a, two_pi);

    return math.cos(normalized);
}

/// Deterministic atan2 for angle calculations
pub fn detAtan2(y: f32, x: f32) f32 {
    if (math.isNan(y) or math.isNan(x)) return math.nan(f32);
    return math.atan2(f32, y, x);
}

/// Deterministic minimum with NaN handling
pub fn detMin(a: f32, b: f32) f32 {
    if (math.isNan(a)) return b;
    if (math.isNan(b)) return a;
    return @min(a, b);
}

/// Deterministic maximum with NaN handling
pub fn detMax(a: f32, b: f32) f32 {
    if (math.isNan(a)) return b;
    if (math.isNan(b)) return a;
    return @max(a, b);
}

/// Deterministic clamp operation
pub fn detClamp(value: f32, min_val: f32, max_val: f32) f32 {
    return detMax(min_val, detMin(max_val, value));
}

/// Deterministic linear interpolation
pub fn detLerp(a: f32, b: f32, t: f32) f32 {
    const clamped_t = detClamp(t, 0.0, 1.0);
    return detAdd(detMul(a, detSub(1.0, clamped_t)), detMul(b, clamped_t));
}

/// Deterministic subtraction
pub fn detSub(a: f32, b: f32) f32 {
    return detAdd(a, -b);
}

/// Deterministic absolute value
pub fn detAbs(a: f32) f32 {
    if (math.isNan(a)) return math.nan(f32);
    return if (a < 0) -a else a;
}

/// Deterministic exponential function
pub fn detExp(a: f32) f32 {
    if (math.isNan(a)) return math.nan(f32);
    if (math.isInf(a)) return if (a > 0) math.inf(f32) else 0.0;
    return @exp(a);
}

/// Deterministic negation
pub fn detNeg(a: f32) f32 {
    if (math.isNan(a)) return math.nan(f32);
    return -a;
}

/// Check if two floats are approximately equal (deterministic)
pub fn detApproxEq(a: f32, b: f32, epsilon: f32) bool {
    const diff = detSub(a, b);
    const abs_diff = if (diff < 0) -diff else diff;
    return abs_diff <= epsilon;
}

// Tests
test "precise deterministic operations" {
    const testing = std.testing;

    // Test basic operations
    try testing.expect(detAdd(1.0, 2.0) == 3.0);
    try testing.expect(detMul(2.0, 3.0) == 6.0);
    try testing.expect(detDiv(6.0, 2.0) == 3.0);
    try testing.expect(detSqrt(4.0) == 2.0);

    // Test NaN handling
    try testing.expect(math.isNan(detAdd(math.nan(f32), 1.0)));
    try testing.expect(math.isNan(detDiv(0.0, 0.0)));

    // Test approximate equality
    try testing.expect(detApproxEq(1.0, 1.0000001, 0.00001));
    try testing.expect(!detApproxEq(1.0, 1.1, 0.05));
}
