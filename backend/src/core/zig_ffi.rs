//! FFI bindings to Zig SIMD optimizations
//!
//! Provides safe Rust wrappers around Zig's high-performance deterministic math
//! and SIMD operations for cross-platform reproducible calculations.

use std::os::raw::{c_float, c_int};

// External C function declarations from Zig
extern "C" {
    fn manifest_det_add_f32(a: c_float, b: c_float) -> c_float;
    fn manifest_det_mul_f32(a: c_float, b: c_float) -> c_float;
    fn manifest_det_div_f32(a: c_float, b: c_float) -> c_float;
    fn manifest_det_sqrt_f32(a: c_float) -> c_float;
    
    fn manifest_simd_add_4_f32(a: *const c_float, b: *const c_float, result: *mut c_float);
    fn manifest_simd_mul_4_f32(a: *const c_float, b: *const c_float, result: *mut c_float);
    fn manifest_simd_dot_4_f32(a: *const c_float, b: *const c_float) -> c_float;
    
    fn manifest_hex_distance(q1: c_int, r1: c_int, q2: c_int, r2: c_int) -> u32;
    fn manifest_hex_to_pixel(q: c_int, r: c_int, size: c_float, x: *mut c_float, y: *mut c_float);
}

/// Safe wrapper for deterministic addition
#[cfg(not(feature = "no_zig"))]
pub fn det_add_f32(a: f32, b: f32) -> f32 {
    unsafe { manifest_det_add_f32(a, b) }
}

/// Fallback implementation when Zig is not available
#[cfg(feature = "no_zig")]
pub fn det_add_f32(a: f32, b: f32) -> f32 {
    a + b // Basic fallback
}

/// Safe wrapper for deterministic multiplication
#[cfg(not(feature = "no_zig"))]
pub fn det_mul_f32(a: f32, b: f32) -> f32 {
    unsafe { manifest_det_mul_f32(a, b) }
}

#[cfg(feature = "no_zig")]
pub fn det_mul_f32(a: f32, b: f32) -> f32 {
    a * b
}

/// Safe wrapper for deterministic division
#[cfg(not(feature = "no_zig"))]
pub fn det_div_f32(a: f32, b: f32) -> f32 {
    unsafe { manifest_det_div_f32(a, b) }
}

#[cfg(feature = "no_zig")]
pub fn det_div_f32(a: f32, b: f32) -> f32 {
    a / b
}

/// Safe wrapper for deterministic square root
#[cfg(not(feature = "no_zig"))]
pub fn det_sqrt_f32(a: f32) -> f32 {
    unsafe { manifest_det_sqrt_f32(a) }
}

#[cfg(feature = "no_zig")]
pub fn det_sqrt_f32(a: f32) -> f32 {
    a.sqrt()
}

/// SIMD 4-element vector type
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Vec4 {
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    pub fn zero() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0)
    }

    pub fn one() -> Self {
        Self::new(1.0, 1.0, 1.0, 1.0)
    }

    pub fn as_array(&self) -> [f32; 4] {
        [self.x, self.y, self.z, self.w]
    }

    pub fn from_array(arr: [f32; 4]) -> Self {
        Self::new(arr[0], arr[1], arr[2], arr[3])
    }
}

/// Safe wrapper for SIMD vector addition
#[cfg(not(feature = "no_zig"))]
pub fn simd_add_4(a: Vec4, b: Vec4) -> Vec4 {
    let mut result = [0.0f32; 4];
    let a_arr = a.as_array();
    let b_arr = b.as_array();
    
    unsafe {
        manifest_simd_add_4_f32(a_arr.as_ptr(), b_arr.as_ptr(), result.as_mut_ptr());
    }
    
    Vec4::from_array(result)
}

#[cfg(feature = "no_zig")]
pub fn simd_add_4(a: Vec4, b: Vec4) -> Vec4 {
    Vec4::new(a.x + b.x, a.y + b.y, a.z + b.z, a.w + b.w)
}

/// Safe wrapper for SIMD vector multiplication
#[cfg(not(feature = "no_zig"))]
pub fn simd_mul_4(a: Vec4, b: Vec4) -> Vec4 {
    let mut result = [0.0f32; 4];
    let a_arr = a.as_array();
    let b_arr = b.as_array();
    
    unsafe {
        manifest_simd_mul_4_f32(a_arr.as_ptr(), b_arr.as_ptr(), result.as_mut_ptr());
    }
    
    Vec4::from_array(result)
}

#[cfg(feature = "no_zig")]
pub fn simd_mul_4(a: Vec4, b: Vec4) -> Vec4 {
    Vec4::new(a.x * b.x, a.y * b.y, a.z * b.z, a.w * b.w)
}

/// Safe wrapper for SIMD dot product
#[cfg(not(feature = "no_zig"))]
pub fn simd_dot_4(a: Vec4, b: Vec4) -> f32 {
    let a_arr = a.as_array();
    let b_arr = b.as_array();
    
    unsafe {
        manifest_simd_dot_4_f32(a_arr.as_ptr(), b_arr.as_ptr())
    }
}

#[cfg(feature = "no_zig")]
pub fn simd_dot_4(a: Vec4, b: Vec4) -> f32 {
    a.x * b.x + a.y * b.y + a.z * b.z + a.w * b.w
}

/// Hex coordinate structure
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HexCoord {
    pub q: i32,
    pub r: i32,
}

impl HexCoord {
    pub fn new(q: i32, r: i32) -> Self {
        Self { q, r }
    }

    pub fn zero() -> Self {
        Self::new(0, 0)
    }
}

/// Pixel position structure
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PixelPos {
    pub x: f32,
    pub y: f32,
}

impl PixelPos {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn zero() -> Self {
        Self::new(0.0, 0.0)
    }
}

/// Safe wrapper for hex distance calculation
#[cfg(not(feature = "no_zig"))]
pub fn hex_distance(a: HexCoord, b: HexCoord) -> u32 {
    unsafe {
        manifest_hex_distance(a.q, a.r, b.q, b.r)
    }
}

#[cfg(feature = "no_zig")]
pub fn hex_distance(a: HexCoord, b: HexCoord) -> u32 {
    // Fallback Manhattan distance calculation
    let dx = (a.q - b.q).abs();
    let dy = (a.r - b.r).abs();
    let dz = (a.q + a.r - b.q - b.r).abs();
    (dx.max(dy).max(dz)) as u32
}

/// Safe wrapper for hex to pixel conversion
#[cfg(not(feature = "no_zig"))]
pub fn hex_to_pixel(coord: HexCoord, size: f32) -> PixelPos {
    let mut x = 0.0f32;
    let mut y = 0.0f32;
    
    unsafe {
        manifest_hex_to_pixel(coord.q, coord.r, size, &mut x, &mut y);
    }
    
    PixelPos::new(x, y)
}

#[cfg(feature = "no_zig")]
pub fn hex_to_pixel(coord: HexCoord, size: f32) -> PixelPos {
    // Fallback flat-top hex conversion
    let x = size * (3.0 / 2.0 * coord.q as f32);
    let y = size * (3.0f32.sqrt() * (coord.r as f32 + coord.q as f32 / 2.0));
    PixelPos::new(x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic_math() {
        let a = 2.5f32;
        let b = 3.7f32;
        
        let sum = det_add_f32(a, b);
        let product = det_mul_f32(a, b);
        let quotient = det_div_f32(a, b);
        let sqrt_val = det_sqrt_f32(4.0);
        
        // Results should be deterministic
        assert!((sum - 6.2f32).abs() < 0.001);
        assert!((product - 9.25f32).abs() < 0.001);
        assert!((quotient - (2.5f32 / 3.7f32)).abs() < 0.001);
        assert!((sqrt_val - 2.0f32).abs() < 0.001);
    }

    #[test]
    fn test_simd_operations() {
        let a = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let b = Vec4::new(5.0, 6.0, 7.0, 8.0);
        
        let sum = simd_add_4(a, b);
        let product = simd_mul_4(a, b);
        let dot = simd_dot_4(a, b);
        
        assert_eq!(sum, Vec4::new(6.0, 8.0, 10.0, 12.0));
        assert_eq!(product, Vec4::new(5.0, 12.0, 21.0, 32.0));
        assert!((dot - 70.0f32).abs() < 0.001); // 1*5 + 2*6 + 3*7 + 4*8 = 70
    }

    #[test]
    fn test_hex_operations() {
        let a = HexCoord::new(0, 0);
        let b = HexCoord::new(3, 3);
        
        let distance = hex_distance(a, b);
        assert_eq!(distance, 6);
        
        let pixel = hex_to_pixel(HexCoord::zero(), 10.0);
        assert!((pixel.x).abs() < 0.001);
        assert!((pixel.y).abs() < 0.001);
    }

    #[test]
    fn test_vec4_operations() {
        let v1 = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let v2 = Vec4::from_array([1.0, 2.0, 3.0, 4.0]);
        
        assert_eq!(v1, v2);
        assert_eq!(v1.as_array(), [1.0, 2.0, 3.0, 4.0]);
        
        let zero = Vec4::zero();
        assert_eq!(zero, Vec4::new(0.0, 0.0, 0.0, 0.0));
        
        let one = Vec4::one();
        assert_eq!(one, Vec4::new(1.0, 1.0, 1.0, 1.0));
    }
}