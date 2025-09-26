// C bridge for Zig SIMD functions
// Provides C-compatible interface for Rust FFI integration

#include <stdint.h>

// Forward declarations for Zig exports
extern float det_f32_add(float a, float b);
extern float det_f32_mul(float a, float b);
extern float det_f32_div(float a, float b);
extern float det_f32_sqrt(float a);

extern void simd_f32_add_4(const float a[4], const float b[4], float result[4]);
extern void simd_f32_mul_4(const float a[4], const float b[4], float result[4]);
extern float simd_f32_dot_4(const float a[4], const float b[4]);

// Hex functions are now directly exported from Zig, no need for C bridge

// C wrapper functions for easier Rust integration
float manifest_det_add_f32(float a, float b) {
    return det_f32_add(a, b);
}

float manifest_det_mul_f32(float a, float b) {
    return det_f32_mul(a, b);
}

float manifest_det_div_f32(float a, float b) {
    return det_f32_div(a, b);
}

float manifest_det_sqrt_f32(float a) {
    return det_f32_sqrt(a);
}

void manifest_simd_add_4_f32(const float a[4], const float b[4], float result[4]) {
    float temp_a[4] = {a[0], a[1], a[2], a[3]};
    float temp_b[4] = {b[0], b[1], b[2], b[3]};
    float temp_result[4];
    
    simd_f32_add_4(temp_a, temp_b, temp_result);
    
    result[0] = temp_result[0];
    result[1] = temp_result[1];
    result[2] = temp_result[2];
    result[3] = temp_result[3];
}

void manifest_simd_mul_4_f32(const float a[4], const float b[4], float result[4]) {
    float temp_a[4] = {a[0], a[1], a[2], a[3]};
    float temp_b[4] = {b[0], b[1], b[2], b[3]};
    float temp_result[4];
    
    simd_f32_mul_4(temp_a, temp_b, temp_result);
    
    result[0] = temp_result[0];
    result[1] = temp_result[1];
    result[2] = temp_result[2];
    result[3] = temp_result[3];
}

float manifest_simd_dot_4_f32(const float a[4], const float b[4]) {
    return simd_f32_dot_4(a, b);
}

// Hex functions (manifest_hex_distance, manifest_hex_to_pixel) are exported directly from Zig
