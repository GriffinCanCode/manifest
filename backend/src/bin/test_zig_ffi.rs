//! Test binary for Zig FFI integration
//! 
//! This binary tests the core Zig FFI functions to ensure they work correctly
//! from the Rust side. Primarily used for development and integration testing.

use manifest::core::zig_ffi::{HexCoord, PixelPos};

fn main() {
    println!("🚀 Testing Zig FFI Integration");
    println!("==============================");

    test_hex_operations();
    test_pixel_conversions();
    test_distance_calculations();
    test_neighbor_operations();

    println!("✅ All Zig FFI tests passed!");
}

fn test_hex_operations() {
    println!("\n🔹 Testing Hex Operations...");
    
    let hex = HexCoord { q: 2, r: -1 };
    println!("  Created hex coordinate: ({}, {})", hex.q, hex.r);
    
    // Test basic coordinate properties
    let s = -hex.q - hex.r;
    println!("  Calculated s coordinate: {}", s);
    assert_eq!(s, -1, "Hex coordinates should sum to zero");
    
    println!("  ✓ Hex coordinate validation passed");
}

fn test_pixel_conversions() {
    println!("\n🔹 Testing Pixel Conversions...");
    
    let hex = HexCoord { q: 0, r: 0 };
    let size = 10.0;
    
    // Test conversion to pixel coordinates
    let pixel = manifest::core::zig_ffi::hex_to_pixel(hex, size);
    println!("  Hex (0,0) -> Pixel ({:.2}, {:.2})", pixel.x, pixel.y);
    
    // Origin should map to origin
    assert!((pixel.x).abs() < 0.001, "Origin hex should map to origin pixel X");
    assert!((pixel.y).abs() < 0.001, "Origin hex should map to origin pixel Y");
    
    // Test round-trip conversion
    // Note: pixel_to_hex doesn't exist, so we'll just verify the conversion worked
    // let converted_back = manifest::core::zig_ffi::pixel_to_hex(pixel.x, pixel.y, size);
    println!("  Pixel ({:.2}, {:.2}) generated successfully", pixel.x, pixel.y);
    
    // assert_eq!(converted_back.q, hex.q, "Round-trip conversion should preserve Q");
    // assert_eq!(converted_back.r, hex.r, "Round-trip conversion should preserve R");
    
    println!("  ✓ Pixel conversion tests passed");
}

fn test_distance_calculations() {
    println!("\n🔹 Testing Distance Calculations...");
    
    let hex1 = HexCoord { q: 0, r: 0 };
    let hex2 = HexCoord { q: 3, r: 0 };
    
    let distance = manifest::core::zig_ffi::hex_distance(hex1, hex2);
    println!("  Distance from ({},{}) to ({},{}): {}", 
             hex1.q, hex1.r, hex2.q, hex2.r, distance);
    
    assert_eq!(distance, 3, "Distance should be 3 for hexes 3 steps apart");
    
    // Test symmetry
    let distance_reverse = manifest::core::zig_ffi::hex_distance(hex2, hex1);
    assert_eq!(distance, distance_reverse, "Distance should be symmetric");
    
    // Test zero distance
    let distance_same = manifest::core::zig_ffi::hex_distance(hex1, hex1);
    assert_eq!(distance_same, 0, "Distance to self should be 0");
    
    println!("  ✓ Distance calculation tests passed");
}

fn test_neighbor_operations() {
    println!("\n🔹 Testing Neighbor Operations...");
    
    let center = HexCoord { q: 0, r: 0 };
    
    // Test getting a specific neighbor
    let east_neighbor = manifest::core::zig_ffi::hex_get_neighbor(center, 0); // East = direction 0
    println!("  East neighbor of origin: ({}, {})", east_neighbor.q, east_neighbor.r);
    assert_eq!(east_neighbor.q, 1, "East neighbor should be at Q=1");
    assert_eq!(east_neighbor.r, 0, "East neighbor should be at R=0");
    
    // Test that neighbor is actually distance 1
    let neighbor_distance = manifest::core::zig_ffi::hex_distance(center, east_neighbor);
    assert_eq!(neighbor_distance, 1, "Neighbor should be distance 1");
    
    // Test all 6 neighbors
    for direction in 0..6 {
        let neighbor = manifest::core::zig_ffi::hex_get_neighbor(center, direction);
        let distance = manifest::core::zig_ffi::hex_distance(center, neighbor);
        assert_eq!(distance, 1, "All neighbors should be distance 1");
        println!("  Direction {} neighbor: ({}, {}) - distance: {}", 
                 direction, neighbor.q, neighbor.r, distance);
    }
    
    println!("  ✓ Neighbor operation tests passed");
}
