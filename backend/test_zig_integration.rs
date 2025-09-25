//! Simple test to verify Zig SIMD optimizations are working

use std::time::Instant;

// Mock the zig_ffi module functions for testing
mod mock_zig_ffi {
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

        pub fn as_array(&self) -> [f32; 4] {
            [self.x, self.y, self.z, self.w]
        }

        pub fn from_array(arr: [f32; 4]) -> Self {
            Self::new(arr[0], arr[1], arr[2], arr[3])
        }
    }

    impl PartialEq for Vec4 {
        fn eq(&self, other: &Self) -> bool {
            (self.x - other.x).abs() < 0.001 &&
            (self.y - other.y).abs() < 0.001 &&
            (self.z - other.z).abs() < 0.001 &&
            (self.w - other.w).abs() < 0.001
        }
    }

    // Zig FFI functions (these will call the actual Zig implementations if available)
    extern "C" {
        fn manifest_det_add_f32(a: f32, b: f32) -> f32;
        fn manifest_simd_add_4_f32(a: *const f32, b: *const f32, result: *mut f32);
        fn manifest_simd_dot_4_f32(a: *const f32, b: *const f32) -> f32;
        fn manifest_hex_distance(q1: i32, r1: i32, q2: i32, r2: i32) -> u32;
    }

    pub fn det_add_f32(a: f32, b: f32) -> f32 {
        unsafe { manifest_det_add_f32(a, b) }
    }

    pub fn simd_add_4(a: Vec4, b: Vec4) -> Vec4 {
        let mut result = [0.0f32; 4];
        let a_arr = a.as_array();
        let b_arr = b.as_array();
        
        unsafe {
            manifest_simd_add_4_f32(a_arr.as_ptr(), b_arr.as_ptr(), result.as_mut_ptr());
        }
        
        Vec4::from_array(result)
    }

    pub fn simd_dot_4(a: Vec4, b: Vec4) -> f32 {
        let a_arr = a.as_array();
        let b_arr = b.as_array();
        
        unsafe {
            manifest_simd_dot_4_f32(a_arr.as_ptr(), b_arr.as_ptr())
        }
    }

    pub fn hex_distance(q1: i32, r1: i32, q2: i32, r2: i32) -> u32 {
        unsafe {
            manifest_hex_distance(q1, r1, q2, r2)
        }
    }

    // Fallback implementations for comparison
    pub fn fallback_det_add_f32(a: f32, b: f32) -> f32 {
        a + b
    }

    pub fn fallback_simd_add_4(a: Vec4, b: Vec4) -> Vec4 {
        Vec4::new(a.x + b.x, a.y + b.y, a.z + b.z, a.w + b.w)
    }

    pub fn fallback_simd_dot_4(a: Vec4, b: Vec4) -> f32 {
        a.x * b.x + a.y * b.y + a.z * b.z + a.w * b.w
    }

    pub fn fallback_hex_distance(q1: i32, r1: i32, q2: i32, r2: i32) -> u32 {
        // Fallback Manhattan distance calculation
        let dx = (q1 - q2).abs();
        let dy = (r1 - r2).abs();
        let dz = ((q1 + r1) - (q2 + r2)).abs();
        (dx.max(dy).max(dz)) as u32
    }
}

fn test_zig_functions() {
    use mock_zig_ffi::*;

    println!("Testing Zig SIMD optimizations...");

    // Test deterministic addition
    println!("\n1. Testing deterministic addition:");
    let a = 2.5f32;
    let b = 3.7f32;
    
    let zig_result = det_add_f32(a, b);
    let fallback_result = fallback_det_add_f32(a, b);
    
    println!("  Zig result: {}", zig_result);
    println!("  Fallback result: {}", fallback_result);
    println!("  Match: {}", (zig_result - fallback_result).abs() < 0.001);

    // Test SIMD vector addition
    println!("\n2. Testing SIMD vector addition:");
    let vec_a = Vec4::new(1.0, 2.0, 3.0, 4.0);
    let vec_b = Vec4::new(5.0, 6.0, 7.0, 8.0);
    
    let zig_sum = simd_add_4(vec_a, vec_b);
    let fallback_sum = fallback_simd_add_4(vec_a, vec_b);
    
    println!("  Zig result: ({}, {}, {}, {})", zig_sum.x, zig_sum.y, zig_sum.z, zig_sum.w);
    println!("  Fallback result: ({}, {}, {}, {})", fallback_sum.x, fallback_sum.y, fallback_sum.z, fallback_sum.w);
    println!("  Match: {}", zig_sum == fallback_sum);

    // Test SIMD dot product
    println!("\n3. Testing SIMD dot product:");
    let zig_dot = simd_dot_4(vec_a, vec_b);
    let fallback_dot = fallback_simd_dot_4(vec_a, vec_b);
    
    println!("  Zig result: {}", zig_dot);
    println!("  Fallback result: {}", fallback_dot);
    println!("  Match: {}", (zig_dot - fallback_dot).abs() < 0.001);

    // Test hex distance
    println!("\n4. Testing hex distance:");
    let zig_dist = hex_distance(0, 0, 3, 3);
    let fallback_dist = fallback_hex_distance(0, 0, 3, 3);
    
    println!("  Zig result: {}", zig_dist);
    println!("  Fallback result: {}", fallback_dist);
    println!("  Match: {}", zig_dist == fallback_dist);
}

fn benchmark_zig_functions() {
    use mock_zig_ffi::*;

    println!("\n=== Performance Benchmarks ===");
    const ITERATIONS: usize = 1_000_000;

    // Benchmark SIMD dot product
    let vec_a = Vec4::new(1.0, 2.0, 3.0, 4.0);
    let vec_b = Vec4::new(5.0, 6.0, 7.0, 8.0);

    // Zig SIMD version
    let start = Instant::now();
    let mut result = 0.0f32;
    for _ in 0..ITERATIONS {
        result += simd_dot_4(vec_a, vec_b);
    }
    let zig_time = start.elapsed();

    // Fallback version
    let start = Instant::now();
    let mut fallback_result = 0.0f32;
    for _ in 0..ITERATIONS {
        fallback_result += fallback_simd_dot_4(vec_a, vec_b);
    }
    let fallback_time = start.elapsed();

    println!("\nSIMD Dot Product ({} iterations):", ITERATIONS);
    println!("  Zig SIMD time: {:?}", zig_time);
    println!("  Fallback time: {:?}", fallback_time);
    println!("  Speedup: {:.2}x", fallback_time.as_secs_f64() / zig_time.as_secs_f64());
    println!("  Results match: {}", (result - fallback_result).abs() < 0.1);
}

fn main() {
    println!("Zig SIMD Integration Test");
    println!("========================");
    
    // Test basic functionality
    test_zig_functions();
    
    // Run performance benchmarks
    benchmark_zig_functions();
    
    println!("\n✅ All tests completed!");
}
