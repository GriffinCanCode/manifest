//! Noise Generation Module
//!
//! High-performance SIMD-optimized noise generation for procedural world generation.
//! Provides deterministic, cross-platform compatible noise functions with
//! vectorized calculations for optimal performance.
//!
//! ## Features
//! - SIMD-optimized Perlin and Simplex noise
//! - Deterministic results across platforms
//! - Fractal noise with octaves, lacunarity, and persistence
//! - Batch processing for large datasets
//! - C FFI compatible structures
//!
//! ## Usage
//! ```zig
//! const noise = @import("noise/mod.zig");
//!
//! // Generate single noise value
//! const value = noise.perlin2d(10.5, 20.3, 0.01, 12345);
//!
//! // Batch processing
//! const params = noise.NoiseParams{
//!     .frequency = 0.01,
//!     .amplitude = 1.0,
//!     .octaves = 4,
//!     .lacunarity = 2.0,
//!     .persistence = 0.5,
//!     .seed = 12345,
//! };
//! noise.batchFractalNoise2D(coords, results, params);
//! ```

const noise_main = @import("noise.zig");
pub const NoiseParams = noise_main.NoiseParams;
pub const CoordPair = noise_main.CoordPair;
pub const NoiseResult = noise_main.NoiseResult;
pub const perlin2d = noise_main.perlin2d;
pub const perlin3d = noise_main.perlin3d;
pub const fractalNoise2d = noise_main.fractalNoise2d;
pub const fractalNoise3d = noise_main.fractalNoise3d;
pub const batchFractalNoise2D = noise_main.batchFractalNoise2D;
pub const batchFractalNoise3D = noise_main.batchFractalNoise3D;

// Re-export types and functions from the main noise module
