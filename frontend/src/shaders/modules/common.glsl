/**
 * Common GLSL utilities and constants
 * Shared across all shaders for consistency
 */

#ifndef COMMON_GLSL
#define COMMON_GLSL

// Mathematical constants
#define PI 3.14159265359
#define TAU 6.28318530718
#define HALF_PI 1.57079632679

// Precision helpers based on device capabilities
#ifdef GL_FRAGMENT_PRECISION_HIGH
  precision highp float;
#else
  precision mediump float;
#endif

// Quality levels (set by ShaderManager)
#ifndef QUALITY_LEVEL
#define QUALITY_LEVEL 3
#endif

// Common conversion functions
vec3 rgb2hsv(vec3 c) {
  vec4 K = vec4(0.0, -1.0 / 3.0, 2.0 / 3.0, -1.0);
  vec4 p = mix(vec4(c.bg, K.wz), vec4(c.gb, K.xy), step(c.b, c.g));
  vec4 q = mix(vec4(p.xyw, c.r), vec4(c.r, p.yzx), step(p.x, c.r));

  float d = q.x - min(q.w, q.y);
  float e = 1.0e-10;
  return vec3(abs(q.z + (q.w - q.y) / (6.0 * d + e)), d / (q.x + e), q.x);
}

vec3 hsv2rgb(vec3 c) {
  vec4 K = vec4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
  vec3 p = abs(fract(c.xxx + K.xyz) * 6.0 - K.www);
  return c.z * mix(K.xxx, clamp(p - K.xxx, 0.0, 1.0), c.y);
}

// Smooth interpolation functions
float smootherstep(float edge0, float edge1, float x) {
  x = clamp((x - edge0) / (edge1 - edge0), 0.0, 1.0);
  return x * x * x * (x * (x * 6.0 - 15.0) + 10.0);
}

// Easing functions
float easeInOutCubic(float t) {
  return t < 0.5 ? 4.0 * t * t * t : 1.0 - pow(-2.0 * t + 2.0, 3.0) / 2.0;
}

float easeOutQuint(float t) {
  return 1.0 - pow(1.0 - t, 5.0);
}

// Random functions
float random(vec2 st) {
  return fract(sin(dot(st.xy, vec2(12.9898, 78.233))) * 43758.5453123);
}

float random(vec3 st) {
  return fract(sin(dot(st.xyz, vec3(12.9898, 78.233, 37.719))) * 43758.5453123);
}

// Hash function for deterministic randomness
vec2 hash22(vec2 p) {
  p = vec2(dot(p, vec2(127.1, 311.7)), dot(p, vec2(269.5, 183.3)));
  return -1.0 + 2.0 * fract(sin(p) * 43758.5453123);
}

vec3 hash33(vec3 p) {
  p = vec3(dot(p, vec3(127.1, 311.7, 74.7)),
           dot(p, vec3(269.5, 183.3, 246.1)),
           dot(p, vec3(113.5, 271.9, 124.6)));
  return -1.0 + 2.0 * fract(sin(p) * 43758.5453123);
}

// Optimized rotate functions
vec2 rotate2D(vec2 v, float a) {
  float s = sin(a);
  float c = cos(a);
  return vec2(v.x * c - v.y * s, v.x * s + v.y * c);
}

mat2 rotate2DMat(float a) {
  float s = sin(a);
  float c = cos(a);
  return mat2(c, -s, s, c);
}

// Distance functions for terrain (hexDistance moved to hex.glsl to avoid conflicts)

// UV coordinate helpers
vec2 tileUV(vec2 uv, float tiles) {
  return fract(uv * tiles);
}

vec2 mirrorUV(vec2 uv) {
  return abs(fract(uv * 0.5) * 2.0 - 1.0);
}

// Depth encoding/decoding
float encodeDepth(float depth) {
  return depth;
}

float decodeDepth(float encoded) {
  return encoded;
}

// Color blending modes
vec3 overlay(vec3 base, vec3 blend) {
  return mix(
    2.0 * base * blend,
    1.0 - 2.0 * (1.0 - base) * (1.0 - blend),
    step(0.5, base)
  );
}

vec3 softLight(vec3 base, vec3 blend) {
  return mix(
    2.0 * base * blend + base * base * (1.0 - 2.0 * blend),
    sqrt(base) * (2.0 * blend - 1.0) + 2.0 * base * (1.0 - blend),
    step(0.5, blend)
  );
}

#pragma glslify: export(rgb2hsv)
#pragma glslify: export(hsv2rgb)
#pragma glslify: export(smootherstep)
#pragma glslify: export(easeInOutCubic)
#pragma glslify: export(easeOutQuint)
#pragma glslify: export(random)
#pragma glslify: export(hash22)
#pragma glslify: export(hash33)
#pragma glslify: export(rotate2D)
#pragma glslify: export(rotate2DMat)
#pragma glslify: export(tileUV)
#pragma glslify: export(mirrorUV)
#pragma glslify: export(overlay)
#pragma glslify: export(softLight)

#endif // COMMON_GLSL
