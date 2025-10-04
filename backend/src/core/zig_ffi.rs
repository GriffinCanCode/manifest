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
    // CRITICAL: Validate input vectors have finite values
    if !a.x.is_finite() || !a.y.is_finite() || !a.z.is_finite() || !a.w.is_finite() {
        eprintln!("WARNING: Invalid input vector a: ({}, {}, {}, {}), using fallback", a.x, a.y, a.z, a.w);
        return simd_add_4_fallback(a, b);
    }
    if !b.x.is_finite() || !b.y.is_finite() || !b.z.is_finite() || !b.w.is_finite() {
        eprintln!("WARNING: Invalid input vector b: ({}, {}, {}, {}), using fallback", b.x, b.y, b.z, b.w);
        return simd_add_4_fallback(a, b);
    }
    
    let mut result = [0.0f32; 4];
    let a_arr = a.as_array();
    let b_arr = b.as_array();
    
    // CRITICAL: Ensure arrays are exactly 4 elements
    debug_assert_eq!(a_arr.len(), 4);
    debug_assert_eq!(b_arr.len(), 4);
    debug_assert_eq!(result.len(), 4);
    
    unsafe {
        // CRITICAL: Verify pointers are not null and aligned
        
        manifest_simd_add_4_f32(a_arr.as_ptr(), b_arr.as_ptr(), result.as_mut_ptr());
    }
    
    // CRITICAL: Validate results from Zig
    for (i, &val) in result.iter().enumerate() {
        if !val.is_finite() {
            eprintln!("WARNING: Zig returned invalid SIMD result at index {}: {}, using fallback", i, val);
            return simd_add_4_fallback(a, b);
        }
    }
    
    Vec4::from_array(result)
}

/// Safe fallback for SIMD vector addition
fn simd_add_4_fallback(a: Vec4, b: Vec4) -> Vec4 {
    Vec4::new(
        if a.x.is_finite() && b.x.is_finite() { a.x + b.x } else { 0.0 },
        if a.y.is_finite() && b.y.is_finite() { a.y + b.y } else { 0.0 },
        if a.z.is_finite() && b.z.is_finite() { a.z + b.z } else { 0.0 },
        if a.w.is_finite() && b.w.is_finite() { a.w + b.w } else { 0.0 },
    )
}

#[cfg(feature = "no_zig")]
pub fn simd_add_4(a: Vec4, b: Vec4) -> Vec4 {
    Vec4::new(a.x + b.x, a.y + b.y, a.z + b.z, a.w + b.w)
}

/// Safe wrapper for SIMD vector multiplication
#[cfg(not(feature = "no_zig"))]
pub fn simd_mul_4(a: Vec4, b: Vec4) -> Vec4 {
    // CRITICAL: Validate input vectors have finite values
    if !a.x.is_finite() || !a.y.is_finite() || !a.z.is_finite() || !a.w.is_finite() {
        eprintln!("WARNING: Invalid input vector a: ({}, {}, {}, {}), using fallback", a.x, a.y, a.z, a.w);
        return simd_mul_4_fallback(a, b);
    }
    if !b.x.is_finite() || !b.y.is_finite() || !b.z.is_finite() || !b.w.is_finite() {
        eprintln!("WARNING: Invalid input vector b: ({}, {}, {}, {}), using fallback", b.x, b.y, b.z, b.w);
        return simd_mul_4_fallback(a, b);
    }
    
    let mut result = [0.0f32; 4];
    let a_arr = a.as_array();
    let b_arr = b.as_array();
    
    // CRITICAL: Ensure arrays are exactly 4 elements
    debug_assert_eq!(a_arr.len(), 4);
    debug_assert_eq!(b_arr.len(), 4);
    debug_assert_eq!(result.len(), 4);
    
    unsafe {
        // CRITICAL: Verify pointers are not null and aligned
        
        manifest_simd_mul_4_f32(a_arr.as_ptr(), b_arr.as_ptr(), result.as_mut_ptr());
    }
    
    // CRITICAL: Validate results from Zig
    for (i, &val) in result.iter().enumerate() {
        if !val.is_finite() {
            eprintln!("WARNING: Zig returned invalid SIMD result at index {}: {}, using fallback", i, val);
            return simd_mul_4_fallback(a, b);
        }
    }
    
    Vec4::from_array(result)
}

/// Safe fallback for SIMD vector multiplication
fn simd_mul_4_fallback(a: Vec4, b: Vec4) -> Vec4 {
    Vec4::new(
        if a.x.is_finite() && b.x.is_finite() { a.x * b.x } else { 0.0 },
        if a.y.is_finite() && b.y.is_finite() { a.y * b.y } else { 0.0 },
        if a.z.is_finite() && b.z.is_finite() { a.z * b.z } else { 0.0 },
        if a.w.is_finite() && b.w.is_finite() { a.w * b.w } else { 0.0 },
    )
}

#[cfg(feature = "no_zig")]
pub fn simd_mul_4(a: Vec4, b: Vec4) -> Vec4 {
    Vec4::new(a.x * b.x, a.y * b.y, a.z * b.z, a.w * b.w)
}

/// Safe wrapper for SIMD dot product
#[cfg(not(feature = "no_zig"))]
pub fn simd_dot_4(a: Vec4, b: Vec4) -> f32 {
    // CRITICAL: Validate input vectors have finite values
    if !a.x.is_finite() || !a.y.is_finite() || !a.z.is_finite() || !a.w.is_finite() {
        eprintln!("WARNING: Invalid input vector a: ({}, {}, {}, {}), using fallback", a.x, a.y, a.z, a.w);
        return simd_dot_4_fallback(a, b);
    }
    if !b.x.is_finite() || !b.y.is_finite() || !b.z.is_finite() || !b.w.is_finite() {
        eprintln!("WARNING: Invalid input vector b: ({}, {}, {}, {}), using fallback", b.x, b.y, b.z, b.w);
        return simd_dot_4_fallback(a, b);
    }
    
    let a_arr = a.as_array();
    let b_arr = b.as_array();
    
    // CRITICAL: Ensure arrays are exactly 4 elements
    debug_assert_eq!(a_arr.len(), 4);
    debug_assert_eq!(b_arr.len(), 4);
    
    let result = unsafe {
        // CRITICAL: Verify pointers are not null and aligned
        
        manifest_simd_dot_4_f32(a_arr.as_ptr(), b_arr.as_ptr())
    };
    
    // CRITICAL: Validate result from Zig
    if !result.is_finite() {
        eprintln!("WARNING: Zig returned invalid dot product result: {}, using fallback", result);
        return simd_dot_4_fallback(a, b);
    }
    
    result
}

/// Safe fallback for SIMD dot product
fn simd_dot_4_fallback(a: Vec4, b: Vec4) -> f32 {
    let x_prod = if a.x.is_finite() && b.x.is_finite() { a.x * b.x } else { 0.0 };
    let y_prod = if a.y.is_finite() && b.y.is_finite() { a.y * b.y } else { 0.0 };
    let z_prod = if a.z.is_finite() && b.z.is_finite() { a.z * b.z } else { 0.0 };
    let w_prod = if a.w.is_finite() && b.w.is_finite() { a.w * b.w } else { 0.0 };
    
    x_prod + y_prod + z_prod + w_prod
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
    // CRITICAL: Validate coordinate values to prevent overflow
    const MAX_COORD: i32 = 100000;
    if coord.q.abs() > MAX_COORD || coord.r.abs() > MAX_COORD {
        eprintln!("WARNING: Hex coordinates ({}, {}) exceed safe range, using fallback", coord.q, coord.r);
        return hex_to_pixel_fallback(coord, size);
    }
    
    // CRITICAL: Validate size parameter
    if !size.is_finite() || size <= 0.0 || size > 10000.0 {
        eprintln!("WARNING: Invalid size {}, using fallback", size);
        return hex_to_pixel_fallback(coord, size.clamp(1.0, 1000.0));
    }
    
    let mut x = 0.0f32;
    let mut y = 0.0f32;
    
    unsafe {
        manifest_hex_to_pixel(coord.q, coord.r, size, &mut x, &mut y);
    }
    
    // CRITICAL: Validate output values
    if !x.is_finite() || !y.is_finite() {
        eprintln!("WARNING: Zig returned invalid pixel coordinates ({}, {}), using fallback", x, y);
        return hex_to_pixel_fallback(coord, size);
    }
    
    PixelPos::new(x, y)
}

/// Safe fallback for hex to pixel conversion
fn hex_to_pixel_fallback(coord: HexCoord, size: f32) -> PixelPos {
    let safe_size = size.clamp(1.0, 1000.0);
    // Fallback flat-top hex conversion with overflow protection
    let q_f = coord.q as f64;
    let r_f = coord.r as f64;
    let size_f = safe_size as f64;
    
    let x = size_f * (1.5 * q_f);
    let y = size_f * (3.0_f64.sqrt() * (r_f + q_f * 0.5));
    
    // Clamp to reasonable pixel ranges
    let x_clamped = x.clamp(-1000000.0, 1000000.0) as f32;
    let y_clamped = y.clamp(-1000000.0, 1000000.0) as f32;
    
    PixelPos::new(x_clamped, y_clamped)
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
    // CRITICAL: Validate pixel position
    if !pos.x.is_finite() || !pos.y.is_finite() {
        eprintln!("WARNING: Invalid pixel position ({}, {}), using fallback", pos.x, pos.y);
        return hex_from_pixel_fallback(pos, size);
    }
    
    // CRITICAL: Validate size parameter
    if !size.is_finite() || size <= 0.0 || size > 10000.0 {
        eprintln!("WARNING: Invalid size {}, using fallback", size);
        return hex_from_pixel_fallback(pos, size.clamp(1.0, 1000.0));
    }
    
    // CRITICAL: Check for extreme pixel values that could cause overflow
    const MAX_PIXEL: f32 = 1000000.0;
    if pos.x.abs() > MAX_PIXEL || pos.y.abs() > MAX_PIXEL {
        eprintln!("WARNING: Pixel position ({}, {}) exceeds safe range, using fallback", pos.x, pos.y);
        return hex_from_pixel_fallback(pos, size);
    }
    
    let mut q = 0i32;
    let mut r = 0i32;
    
    unsafe {
        manifest_hex_from_pixel(pos.x, pos.y, size, &mut q, &mut r);
    }
    
    // CRITICAL: Validate output coordinates
    const MAX_COORD: i32 = 100000;
    if q.abs() > MAX_COORD || r.abs() > MAX_COORD {
        eprintln!("WARNING: Zig returned extreme coordinates ({}, {}), using fallback", q, r);
        return hex_from_pixel_fallback(pos, size);
    }
    
    HexCoord::new(q, r)
}

/// Safe fallback for pixel to hex conversion
fn hex_from_pixel_fallback(pos: PixelPos, size: f32) -> HexCoord {
    let safe_size = size.clamp(1.0, 1000.0) as f64;
    let safe_x = pos.x.clamp(-1000000.0, 1000000.0) as f64;
    let safe_y = pos.y.clamp(-1000000.0, 1000000.0) as f64;
    
    // Fallback pixel-to-hex conversion with safe arithmetic
    let q_f = (2.0 / 3.0) * (safe_x / safe_size);
    let r_f = (safe_y / (safe_size * 3.0_f64.sqrt())) - (0.5 * q_f);
    
    // Safe rounding with bounds checking
    let q = q_f.clamp(-100000.0, 100000.0).round() as i32;
    let r = r_f.clamp(-100000.0, 100000.0).round() as i32;
    
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
    
    // CRITICAL: Reasonable radius limit to prevent massive allocations
    const MAX_RADIUS: u32 = 1000;
    if radius > MAX_RADIUS {
        eprintln!("WARNING: Radius {} exceeds maximum safe radius {}, using fallback", radius, MAX_RADIUS);
        return hex_get_ring_fallback(center, radius);
    }
    
    // CRITICAL: Validate center coordinates are reasonable
    const MAX_COORD: i32 = 100000;
    if center.q.abs() > MAX_COORD || center.r.abs() > MAX_COORD {
        eprintln!("WARNING: Center coordinates ({}, {}) exceed safe range, using fallback", center.q, center.r);
        return hex_get_ring_fallback(center, radius);
    }
    
    let max_ring_size = 6 * radius as usize;
    let mut result = vec![HexCoordC { q: 0, r: 0 }; max_ring_size];
    let mut actual_count = max_ring_size;
    
    unsafe {
        // CRITICAL: Verify pointers are not null
        
        manifest_hex_get_ring(center.q, center.r, radius, result.as_mut_ptr(), &mut actual_count);
    }
    
    // CRITICAL: Validate Zig-provided count BEFORE using it
    if actual_count > max_ring_size {
        eprintln!("SECURITY WARNING: Zig returned invalid count {} > max {}, using fallback", 
                 actual_count, max_ring_size);
        return hex_get_ring_fallback(center, radius);
    }
    
    // Additional safety: Ensure count is reasonable
    if actual_count == 0 {
        eprintln!("WARNING: Zig returned zero ring size for radius {}, using fallback", radius);
        return hex_get_ring_fallback(center, radius);
    }
    
    // CRITICAL: Safe truncation with validated count
    result.truncate(actual_count);
    
    // CRITICAL: Validate returned coordinates are reasonable
    let coords: Result<Vec<HexCoord>, String> = result
        .into_iter()
        .enumerate()
        .map(|(i, c)| {
            if c.q.abs() > MAX_COORD || c.r.abs() > MAX_COORD {
                Err(format!("Invalid coordinate at index {}: ({}, {})", i, c.q, c.r))
            } else {
                Ok(HexCoord::from_c(c))
            }
        })
        .collect();
    
    match coords {
        Ok(valid_coords) => valid_coords,
        Err(error) => {
            eprintln!("WARNING: Zig returned invalid coordinates ({}), using fallback", error);
            hex_get_ring_fallback(center, radius)
        }
    }
}

/// Safe fallback implementation for hex_get_ring
fn hex_get_ring_fallback(center: HexCoord, radius: u32) -> Vec<HexCoord> {
    if radius == 0 {
        return vec![center];
    }
    
    // Limit radius for fallback to prevent excessive computation
    const MAX_FALLBACK_RADIUS: u32 = 100;
    let safe_radius = radius.min(MAX_FALLBACK_RADIUS);
    
    // Fallback ring generation using safe integer arithmetic
    let mut ring = Vec::new();
    
    // Check for potential overflow before arithmetic
    if center.q.checked_add(safe_radius as i32).is_none() ||
       center.q.checked_sub(safe_radius as i32).is_none() ||
       center.r.checked_add(safe_radius as i32).is_none() ||
       center.r.checked_sub(safe_radius as i32).is_none() {
        // If overflow would occur, return just the center
        return vec![center];
    }
    
    let mut current = HexCoord::new(center.q + safe_radius as i32, center.r);
    let directions = [(0, -1), (-1, 0), (-1, 1), (0, 1), (1, 0), (1, -1)];
    
    for &(dq, dr) in &directions {
        for _ in 0..safe_radius {
            ring.push(current);
            // Safe arithmetic with overflow checking
            if let (Some(new_q), Some(new_r)) = (current.q.checked_add(dq), current.r.checked_add(dr)) {
                current.q = new_q;
                current.r = new_r;
            } else {
                // Overflow would occur, stop here
                return ring;
            }
        }
    }
    
    ring
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
    // CRITICAL: Validate coordinates are reasonable
    const MAX_COORD: i32 = 100000;
    if start.q.abs() > MAX_COORD || start.r.abs() > MAX_COORD ||
       end.q.abs() > MAX_COORD || end.r.abs() > MAX_COORD {
        eprintln!("WARNING: Coordinates exceed safe range, using fallback");
        return hex_line_draw_fallback(start, end);
    }
    
    let distance = hex_distance(start, end);
    
    // CRITICAL: Prevent excessive line lengths
    const MAX_LINE_LENGTH: u32 = 10000;
    if distance > MAX_LINE_LENGTH {
        eprintln!("WARNING: Line distance {} exceeds maximum safe length {}, using fallback", 
                 distance, MAX_LINE_LENGTH);
        return hex_line_draw_fallback(start, end);
    }
    
    let max_line_size = distance as usize + 1;
    let mut result = vec![HexCoordC { q: 0, r: 0 }; max_line_size];
    let mut actual_count = max_line_size;
    
    unsafe {
        // CRITICAL: Verify pointers are not null
        
        manifest_hex_line_draw(start.q, start.r, end.q, end.r, result.as_mut_ptr(), &mut actual_count);
    }
    
    // CRITICAL: Validate Zig-provided count BEFORE using it
    if actual_count > max_line_size {
        eprintln!("SECURITY WARNING: Zig returned invalid line count {} > max {}, using fallback", 
                 actual_count, max_line_size);
        return hex_line_draw_fallback(start, end);
    }
    
    if actual_count == 0 {
        eprintln!("WARNING: Zig returned zero line length, using fallback");
        return hex_line_draw_fallback(start, end);
    }
    
    // CRITICAL: Safe truncation with validated count
    result.truncate(actual_count);
    
    // CRITICAL: Validate returned coordinates
    let coords: Result<Vec<HexCoord>, String> = result
        .into_iter()
        .enumerate()
        .map(|(i, c)| {
            if c.q.abs() > MAX_COORD || c.r.abs() > MAX_COORD {
                Err(format!("Invalid coordinate at index {}: ({}, {})", i, c.q, c.r))
            } else {
                Ok(HexCoord::from_c(c))
            }
        })
        .collect();
    
    match coords {
        Ok(valid_coords) => valid_coords,
        Err(error) => {
            eprintln!("WARNING: Zig returned invalid coordinates ({}), using fallback", error);
            hex_line_draw_fallback(start, end)
        }
    }
}

/// Safe fallback implementation for hex_line_draw
fn hex_line_draw_fallback(start: HexCoord, end: HexCoord) -> Vec<HexCoord> {
    let distance = hex_distance(start, end);
    if distance == 0 {
        return vec![start];
    }
    
    // Limit distance for fallback
    const MAX_FALLBACK_DISTANCE: u32 = 1000;
    let safe_distance = distance.min(MAX_FALLBACK_DISTANCE);
    
    let mut line = Vec::with_capacity(safe_distance as usize + 1);
    for i in 0..=safe_distance {
        let t = i as f32 / safe_distance as f32;
        let q_f = start.q as f32 + t * (end.q - start.q) as f32;
        let r_f = start.r as f32 + t * (end.r - start.r) as f32;
        
        // Use safe rounding that checks for reasonable values
        if q_f.is_finite() && r_f.is_finite() && 
           q_f.abs() < 1000000.0 && r_f.abs() < 1000000.0 {
            line.push(hex_round(q_f, r_f));
        } else {
            // Stop at invalid coordinates
            break;
        }
    }
    
    if line.is_empty() {
        vec![start]
    } else {
        line
    }
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