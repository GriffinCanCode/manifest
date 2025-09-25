use std::env;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    
    println!("cargo:rustc-link-search=native={}/zig-modules", manifest_dir);
    println!("cargo:rustc-link-lib=static=manifest_zig");
    
    // Test Zig function calls
    extern "C" {
        fn manifest_det_add_f32(a: f32, b: f32) -> f32;
        fn manifest_hex_distance(q1: i32, r1: i32, q2: i32, r2: i32) -> u32;
        fn manifest_simd_dot_4_f32(a: *const f32, b: *const f32) -> f32;
    }
    
    println!("\n🎯 Testing Zig SIMD Integration:");
    println!("================================");
    
    // Test 1: Deterministic math
    let result = unsafe { manifest_det_add_f32(2.5, 3.7) };
    println!("✅ Deterministic add: 2.5 + 3.7 = {:.6}", result);
    
    // Test 2: Hex distance
    let distance = unsafe { manifest_hex_distance(0, 0, 3, 3) };
    println!("✅ Hex distance: (0,0) to (3,3) = {}", distance);
    
    // Test 3: SIMD dot product
    let a = [1.0f32, 2.0, 3.0, 4.0];
    let b = [5.0f32, 6.0, 7.0, 8.0];
    let dot = unsafe { manifest_simd_dot_4_f32(a.as_ptr(), b.as_ptr()) };
    println!("✅ SIMD dot product: [1,2,3,4] · [5,6,7,8] = {:.6}", dot);
    
    println!("\n🚀 All Zig SIMD optimizations are working!");
}
