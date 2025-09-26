//! SIMD Operations Module
//!
//! High-performance SIMD vector operations for game engine mathematics.
//! Provides deterministic, cross-platform compatible vectorized calculations
//! using Zig's built-in SIMD support for optimal performance.
//!
//! ## Features
//! - 4-element f32 vector operations (add, mul, sub, div)
//! - Dot product and magnitude calculations
//! - Deterministic behavior across platforms
//! - Cross product and angle calculations
//! - Batch processing utilities for large datasets
//! - Integration with precise math operations
//!
//! ## Usage
//! ```zig
//! const simd = @import("simd/mod.zig");
//!
//! // Vector operations
//! const a = [4]f32{ 1.0, 2.0, 3.0, 4.0 };
//! const b = [4]f32{ 5.0, 6.0, 7.0, 8.0 };
//! const result = simd.addVec4(a, b);
//! const dot_product = simd.dotVec4(a, b);
//!
//! // Batch processing
//! simd.batchAddVec4(a_array, b_array, results, count);
//! ```

const simd_main = @import("simd.zig");
pub const Vec4 = simd_main.Vec4;
pub const addVec4 = simd_main.addVec4;
pub const mulVec4 = simd_main.mulVec4;
pub const subVec4 = simd_main.subVec4;
pub const divVec4 = simd_main.divVec4;
pub const dotVec4 = simd_main.dotVec4;
pub const magnitudeVec4 = simd_main.magnitudeVec4;
pub const normalizeVec4 = simd_main.normalizeVec4;
pub const crossProductVec3 = simd_main.crossProductVec3;
pub const angleBetweenVec3 = simd_main.angleBetweenVec3;
pub const batchAddVec4 = simd_main.batchAddVec4;
pub const batchMulVec4 = simd_main.batchMulVec4;
pub const batchDotVec4 = simd_main.batchDotVec4;

// Re-export types and functions from the main SIMD module
