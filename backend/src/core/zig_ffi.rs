//! FFI bindings to Zig SIMD optimizations
//!
//! Provides safe Rust wrappers around Zig's high-performance deterministic math
//! and SIMD operations for cross-platform reproducible calculations.

use std::os::raw::{c_float, c_int};
use serde::{Serialize, Deserialize};

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
    fn manifest_hex_from_pixel(x: c_float, y: c_float, size: c_float, q: *mut c_int, r: *mut c_int);
    fn manifest_hex_get_neighbors(q: c_int, r: c_int, neighbors: *mut [HexCoordC; 6]);
    fn manifest_hex_get_neighbor(q: c_int, r: c_int, direction: u8, out_q: *mut c_int, out_r: *mut c_int);
    fn manifest_hex_batch_to_pixel(coords: *const HexCoordC, size: c_float, pixels: *mut PixelPosC, count: usize);
    fn manifest_hex_round_to_hex(q_f: c_float, r_f: c_float, q: *mut c_int, r: *mut c_int);
    fn manifest_hex_batch_distances(coords1: *const HexCoordC, coords2: *const HexCoordC, distances: *mut u32, count: usize);
    fn manifest_hex_get_ring(center_q: c_int, center_r: c_int, radius: u32, result: *mut HexCoordC, max_count: *mut usize);
    fn manifest_hex_line_draw(start_q: c_int, start_r: c_int, end_q: c_int, end_r: c_int, result: *mut HexCoordC, max_count: *mut usize);
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

/// C-compatible hex coordinate structure
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct HexCoordC {
    q: c_int,
    r: c_int,
}

/// C-compatible pixel position structure  
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
struct PixelPosC {
    x: c_float,
    y: c_float,
}

/// Hex coordinate structure
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

    /// Convert to C-compatible structure
    fn to_c(self) -> HexCoordC {
        HexCoordC { q: self.q, r: self.r }
    }

    /// Convert from C-compatible structure
    fn from_c(c: HexCoordC) -> Self {
        Self { q: c.q, r: c.r }
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

    /// Convert to C-compatible structure
    fn to_c(self) -> PixelPosC {
        PixelPosC { x: self.x, y: self.y }
    }

    /// Convert from C-compatible structure
    fn from_c(c: PixelPosC) -> Self {
        Self { x: c.x, y: c.y }
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

/// Safe wrapper for pixel to hex conversion
#[cfg(not(feature = "no_zig"))]
pub fn hex_from_pixel(pos: PixelPos, size: f32) -> HexCoord {
    let mut q = 0i32;
    let mut r = 0i32;
    
    unsafe {
        manifest_hex_from_pixel(pos.x, pos.y, size, &mut q, &mut r);
    }
    
    HexCoord::new(q, r)
}

#[cfg(feature = "no_zig")]
pub fn hex_from_pixel(pos: PixelPos, size: f32) -> HexCoord {
    // Fallback pixel-to-hex conversion
    let q_f = (2.0 / 3.0) * (pos.x / size);
    let r_f = (pos.y / (size * 3.0f32.sqrt())) - (0.5 * q_f);
    
    // Simple rounding for fallback
    let q = q_f.round() as i32;
    let r = r_f.round() as i32;
    
    HexCoord::new(q, r)
}

/// Get all 6 neighbors of a hex coordinate
#[cfg(not(feature = "no_zig"))]
pub fn hex_get_neighbors(coord: HexCoord) -> [HexCoord; 6] {
    let mut neighbors: [HexCoordC; 6] = unsafe { std::mem::zeroed() };
    
    unsafe {
        manifest_hex_get_neighbors(coord.q, coord.r, &mut neighbors);
    }
    
    [
        HexCoord::from_c(neighbors[0]),
        HexCoord::from_c(neighbors[1]),
        HexCoord::from_c(neighbors[2]),
        HexCoord::from_c(neighbors[3]),
        HexCoord::from_c(neighbors[4]),
        HexCoord::from_c(neighbors[5]),
    ]
}

#[cfg(feature = "no_zig")]
pub fn hex_get_neighbors(coord: HexCoord) -> [HexCoord; 6] {
    // Fallback neighbor calculation
    let directions = [(1, 0), (1, -1), (0, -1), (-1, 0), (-1, 1), (0, 1)];
    [
        HexCoord::new(coord.q + directions[0].0, coord.r + directions[0].1),
        HexCoord::new(coord.q + directions[1].0, coord.r + directions[1].1),
        HexCoord::new(coord.q + directions[2].0, coord.r + directions[2].1),
        HexCoord::new(coord.q + directions[3].0, coord.r + directions[3].1),
        HexCoord::new(coord.q + directions[4].0, coord.r + directions[4].1),
        HexCoord::new(coord.q + directions[5].0, coord.r + directions[5].1),
    ]
}

/// Get neighbor in specific direction (0-5)
#[cfg(not(feature = "no_zig"))]
pub fn hex_get_neighbor(coord: HexCoord, direction: u8) -> HexCoord {
    let mut q = 0i32;
    let mut r = 0i32;
    
    unsafe {
        manifest_hex_get_neighbor(coord.q, coord.r, direction, &mut q, &mut r);
    }
    
    HexCoord::new(q, r)
}

#[cfg(feature = "no_zig")]
pub fn hex_get_neighbor(coord: HexCoord, direction: u8) -> HexCoord {
    let neighbors = hex_get_neighbors(coord);
    neighbors[direction as usize % 6]
}

/// SIMD batch hex-to-pixel conversion
#[cfg(not(feature = "no_zig"))]
pub fn hex_batch_to_pixel(coords: &[HexCoord], size: f32) -> Vec<PixelPos> {
    let mut result = vec![PixelPosC { x: 0.0, y: 0.0 }; coords.len()];
    let c_coords: Vec<HexCoordC> = coords.iter().map(|c| c.to_c()).collect();
    
    unsafe {
        manifest_hex_batch_to_pixel(c_coords.as_ptr(), size, result.as_mut_ptr(), coords.len());
    }
    
    result.into_iter().map(PixelPos::from_c).collect()
}

#[cfg(feature = "no_zig")]
pub fn hex_batch_to_pixel(coords: &[HexCoord], size: f32) -> Vec<PixelPos> {
    coords.iter().map(|&coord| hex_to_pixel(coord, size)).collect()
}

/// Round fractional hex coordinates
#[cfg(not(feature = "no_zig"))]
pub fn hex_round(q_f: f32, r_f: f32) -> HexCoord {
    let mut q = 0i32;
    let mut r = 0i32;
    
    unsafe {
        manifest_hex_round_to_hex(q_f, r_f, &mut q, &mut r);
    }
    
    HexCoord::new(q, r)
}

#[cfg(feature = "no_zig")]
pub fn hex_round(q_f: f32, r_f: f32) -> HexCoord {
    let s_f = -q_f - r_f;
    
    let mut q = q_f.round() as i32;
    let mut r = r_f.round() as i32;
    let s = s_f.round() as i32;
    
    let q_diff = (q as f32 - q_f).abs();
    let r_diff = (r as f32 - r_f).abs();
    let s_diff = (s as f32 - s_f).abs();
    
    if q_diff > r_diff && q_diff > s_diff {
        q = -r - s;
    } else if r_diff > s_diff {
        r = -q - s;
    }
    
    HexCoord::new(q, r)
}

/// SIMD batch distance calculation
#[cfg(not(feature = "no_zig"))]
pub fn hex_batch_distances(coords1: &[HexCoord], coords2: &[HexCoord]) -> Vec<u32> {
    assert_eq!(coords1.len(), coords2.len());
    
    let mut distances = vec![0u32; coords1.len()];
    let c_coords1: Vec<HexCoordC> = coords1.iter().map(|c| c.to_c()).collect();
    let c_coords2: Vec<HexCoordC> = coords2.iter().map(|c| c.to_c()).collect();
    
    unsafe {
        manifest_hex_batch_distances(
            c_coords1.as_ptr(), 
            c_coords2.as_ptr(), 
            distances.as_mut_ptr(), 
            coords1.len()
        );
    }
    
    distances
}

#[cfg(feature = "no_zig")]
pub fn hex_batch_distances(coords1: &[HexCoord], coords2: &[HexCoord]) -> Vec<u32> {
    coords1.iter().zip(coords2.iter())
        .map(|(&a, &b)| hex_distance(a, b))
        .collect()
}

/// Get hex ring at specific radius
#[cfg(not(feature = "no_zig"))]
pub fn hex_get_ring(center: HexCoord, radius: u32) -> Vec<HexCoord> {
    if radius == 0 {
        return vec![center];
    }
    
    let max_ring_size = 6 * radius as usize;
    let mut result = vec![HexCoordC { q: 0, r: 0 }; max_ring_size];
    let mut actual_count = max_ring_size;
    
    unsafe {
        manifest_hex_get_ring(center.q, center.r, radius, result.as_mut_ptr(), &mut actual_count);
    }
    
    result.truncate(actual_count);
    result.into_iter().map(HexCoord::from_c).collect()
}

#[cfg(feature = "no_zig")]
pub fn hex_get_ring(center: HexCoord, radius: u32) -> Vec<HexCoord> {
    if radius == 0 {
        return vec![center];
    }
    
    // Fallback ring generation
    let mut ring = Vec::new();
    let mut current = HexCoord::new(center.q + radius as i32, center.r);
    let directions = [(0, -1), (-1, 0), (-1, 1), (0, 1), (1, 0), (1, -1)];
    
    for &(dq, dr) in &directions {
        for _ in 0..radius {
            ring.push(current);
            current.q += dq;
            current.r += dr;
        }
    }
    
    ring
}

/// Draw line between two hex coordinates
#[cfg(not(feature = "no_zig"))]
pub fn hex_line_draw(start: HexCoord, end: HexCoord) -> Vec<HexCoord> {
    let distance = hex_distance(start, end);
    let max_line_size = distance as usize + 1;
    let mut result = vec![HexCoordC { q: 0, r: 0 }; max_line_size];
    let mut actual_count = max_line_size;
    
    unsafe {
        manifest_hex_line_draw(start.q, start.r, end.q, end.r, result.as_mut_ptr(), &mut actual_count);
    }
    
    result.truncate(actual_count);
    result.into_iter().map(HexCoord::from_c).collect()
}

#[cfg(feature = "no_zig")]
pub fn hex_line_draw(start: HexCoord, end: HexCoord) -> Vec<HexCoord> {
    let distance = hex_distance(start, end);
    if distance == 0 {
        return vec![start];
    }
    
    let mut line = Vec::new();
    for i in 0..=distance {
        let t = i as f32 / distance as f32;
        let q_f = start.q as f32 + t * (end.q - start.q) as f32;
        let r_f = start.r as f32 + t * (end.r - start.r) as f32;
        line.push(hex_round(q_f, r_f));
    }
    
    line
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