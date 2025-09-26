//! Math Module - Comprehensive Mathematical Utilities
//!
//! Provides deterministic mathematical operations, hexagonal grid calculations,
//! and precision-focused utilities for reproducible game simulations.
//!
//! ## Features
//! - Deterministic floating-point operations for cross-platform consistency
//! - Comprehensive hexagonal grid mathematics with SIMD optimizations
//! - Common mathematical utilities and constants
//! - Precise numerical algorithms for game world calculations
//!
//! ## Usage
//! ```zig
//! const math = @import("math/mod.zig");
//!
//! // Deterministic math operations
//! const result = math.precise.detAdd(1.0, 2.0);
//!
//! // Hexagonal grid calculations
//! const coord = math.hex.HexCoord.init(10, 20);
//! const distance = math.hex.distance(0, 0, 10, 20);
//!
//! // General math utilities
//! const angle_rad = math.degreesToRadians(45.0);
//! ```

pub const hex = @import("hex.zig");
pub const HexCoord = hex.HexCoord;
pub const CubeCoord = hex.CubeCoord;
pub const OffsetCoord = hex.OffsetCoord;
pub const PixelPos = hex.PixelPos;
pub const distance = hex.distance;
pub const toPixel = hex.toPixel;
pub const fromPixel = hex.fromPixel;
pub const getNeighbors = hex.getNeighbors;
pub const getNeighbor = hex.getNeighbor;
pub const batchToPixel = hex.batchToPixel;
pub const roundToHex = hex.roundToHex;
pub const math = @import("math.zig");
pub const PI = math.PI;
pub const E = math.E;
pub const TAU = math.TAU;
pub const det = math.det;
pub const degreesToRadians = math.degreesToRadians;
pub const radiansToDegrees = math.radiansToDegrees;
pub const fastInvSqrt = math.fastInvSqrt;
pub const smoothstep = math.smoothstep;
pub const bias = math.bias;
pub const gain = math.gain;
pub const mapRange = math.mapRange;
pub const normalizeAngle = math.normalizeAngle;
pub const shortestAngleDistance = math.shortestAngleDistance;
pub const precise = @import("precise.zig");
pub const detAdd = precise.detAdd;
pub const detSub = precise.detSub;
pub const detMul = precise.detMul;
pub const detDiv = precise.detDiv;
pub const detSqrt = precise.detSqrt;
pub const detSin = precise.detSin;
pub const detCos = precise.detCos;
pub const detAtan2 = precise.detAtan2;
pub const detMin = precise.detMin;
pub const detMax = precise.detMax;
pub const detClamp = precise.detClamp;
pub const detLerp = precise.detLerp;
pub const detApproxEq = precise.detApproxEq;

// Re-export commonly used hex types and functions
// Re-export precise math operations
// Re-export common math utilities
