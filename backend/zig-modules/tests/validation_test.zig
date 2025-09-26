//! Quick validation test to verify Zig modules are working correctly
//!
//! This is a minimal test that can be run to quickly validate the setup
//! and ensure core functionality is operational.

const std = @import("std");
const testing = std.testing;

const hex = @import("math/hex.zig");
const precise = @import("math/precise.zig");
const simd = @import("simd/simd.zig");

// Import core modules using relative path
pub fn main() !void {
    std.debug.print("🔧 Manifest Zig Modules - Quick Validation Test\n");
    std.debug.print("===============================================\n\n");

    // Test 1: Basic hex operations
    std.debug.print("1️⃣  Testing hex coordinate operations...\n");
    const hex_coord = hex.HexCoord.init(3, -1);
    const distance_result = hex.distance(0, 0, hex_coord.q, hex_coord.r);
    std.debug.print("   • Hex distance (0,0) to (3,-1): {}\n", .{distance_result});

    const pixel = hex.toPixel(hex_coord.q, hex_coord.r, 10.0);
    std.debug.print("   • Hex to pixel: ({d:.2}, {d:.2})\n", .{ pixel.x, pixel.y });

    if (distance_result == 3 and pixel.x > 0) {
        std.debug.print("   ✅ Hex operations working correctly\n\n");
    } else {
        std.debug.print("   ❌ Hex operations failed\n\n");
        return;
    }

    // Test 2: Precise math operations
    std.debug.print("2️⃣  Testing precise math operations...\n");
    const result_add = precise.detAdd(2.5, 3.5);
    const result_mul = precise.detMul(4.0, 2.5);
    const result_sqrt = precise.detSqrt(16.0);

    std.debug.print("   • 2.5 + 3.5 = {d:.1}\n", .{result_add});
    std.debug.print("   • 4.0 × 2.5 = {d:.1}\n", .{result_mul});
    std.debug.print("   • √16 = {d:.1}\n", .{result_sqrt});

    if (result_add == 6.0 and result_mul == 10.0 and result_sqrt == 4.0) {
        std.debug.print("   ✅ Precise math working correctly\n\n");
    } else {
        std.debug.print("   ❌ Precise math failed\n\n");
        return;
    }

    // Test 3: SIMD operations
    std.debug.print("3️⃣  Testing SIMD vector operations...\n");
    const vec_a = [4]f32{ 1.0, 2.0, 3.0, 4.0 };
    const vec_b = [4]f32{ 5.0, 6.0, 7.0, 8.0 };

    const vec_sum = simd.addVec4(vec_a, vec_b);
    const dot_product = simd.dotVec4(vec_a, vec_b);
    const vec_length = simd.lengthVec4(vec_a);

    std.debug.print("   • Vector sum: [{d:.0}, {d:.0}, {d:.0}, {d:.0}]\n", .{ vec_sum[0], vec_sum[1], vec_sum[2], vec_sum[3] });
    std.debug.print("   • Dot product: {d:.0}\n", .{dot_product});
    std.debug.print("   • Vector length: {d:.2}\n", .{vec_length});

    if (vec_sum[0] == 6.0 and dot_product == 70.0 and vec_length > 5.4 and vec_length < 5.5) {
        std.debug.print("   ✅ SIMD operations working correctly\n\n");
    } else {
        std.debug.print("   ❌ SIMD operations failed\n\n");
        return;
    }

    // Test 4: FFI export functions
    std.debug.print("4️⃣  Testing FFI export functions...\n");
    const lib = @import("lib.zig");

    const ffi_distance = lib.manifest_hex_distance(0, 0, 4, 3);
    std.debug.print("   • FFI hex distance: {}\n", .{ffi_distance});

    const ffi_add = lib.manifest_det_add_f32(1.5, 2.5);
    std.debug.print("   • FFI deterministic add: {d:.1}\n", .{ffi_add});

    // Test SIMD FFI
    const a_vals = [4]f32{ 2.0, 3.0, 4.0, 5.0 };
    const b_vals = [4]f32{ 1.0, 2.0, 3.0, 4.0 };
    var result_vals: [4]f32 = undefined;

    lib.manifest_simd_add_4_f32(&a_vals, &b_vals, &result_vals);
    std.debug.print("   • FFI SIMD add result: [{d:.0}, {d:.0}, {d:.0}, {d:.0}]\n", .{ result_vals[0], result_vals[1], result_vals[2], result_vals[3] });

    if (ffi_distance == 7 and ffi_add == 4.0 and result_vals[0] == 3.0) {
        std.debug.print("   ✅ FFI exports working correctly\n\n");
    } else {
        std.debug.print("   ❌ FFI exports failed\n\n");
        return;
    }

    // All tests passed
    std.debug.print("🎉 ALL VALIDATION TESTS PASSED!\n");
    std.debug.print("===============================\n\n");
    std.debug.print("✨ Your Zig modules are working correctly and ready for use!\n");
    std.debug.print("🚀 You can now run the full test suite with: ./run_tests.sh\n");
    std.debug.print("📊 For performance benchmarks, run: zig build-exe tests/benchmarks.zig && ./benchmarks\n\n");
}
