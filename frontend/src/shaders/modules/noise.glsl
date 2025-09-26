/**
 * Noise functions for terrain generation
 * Optimized versions of noise used in Rust backend (Zig SIMD)
 */

#include common.glsl

// 2D Simplex noise
float simplex2D(vec2 v) {
  const vec4 C = vec4(0.211324865405187,  // (3.0-sqrt(3.0))/6.0
                      0.366025403784439,  // 0.5*(sqrt(3.0)-1.0)
                      -0.577350269189626, // -1.0 + 2.0 * C.x
                      0.024390243902439); // 1.0 / 41.0

  vec2 i = floor(v + dot(v, C.yy));
  vec2 x0 = v - i + dot(i, C.xx);

  vec2 i1;
  i1 = (x0.x > x0.y) ? vec2(1.0, 0.0) : vec2(0.0, 1.0);

  vec4 x12 = x0.xyxy + C.xxzz;
  x12.xy -= i1;

  i = mod289(i);
  vec3 p = permute(permute(i.y + vec3(0.0, i1.y, 1.0)) + i.x + vec3(0.0, i1.x, 1.0));

  vec3 m = max(0.5 - vec3(dot(x0, x0), dot(x12.xy, x12.xy), dot(x12.zw, x12.zw)), 0.0);
  m = m * m;
  m = m * m;

  vec3 x = 2.0 * fract(p * C.www) - 1.0;
  vec3 h = abs(x) - 0.5;
  vec3 ox = floor(x + 0.5);
  vec3 a0 = x - ox;

  m *= 1.79284291400159 - 0.85373472095314 * (a0 * a0 + h * h);

  vec3 g;
  g.x = a0.x * x0.x + h.x * x0.y;
  g.yz = a0.yz * x12.xz + h.yz * x12.yw;
  return 130.0 * dot(m, g);
}

// 3D Simplex noise
float simplex3D(vec3 v) {
  const vec2 C = vec2(1.0/6.0, 1.0/3.0);
  const vec4 D = vec4(0.0, 0.5, 1.0, 2.0);

  vec3 i = floor(v + dot(v, C.yyy));
  vec3 x0 = v - i + dot(i, C.xxx);

  vec3 g = step(x0.yzx, x0.xyz);
  vec3 l = 1.0 - g;
  vec3 i1 = min(g.xyz, l.zxy);
  vec3 i2 = max(g.xyz, l.zxy);

  vec3 x1 = x0 - i1 + C.xxx;
  vec3 x2 = x0 - i2 + C.yyy;
  vec3 x3 = x0 - D.yyy;

  i = mod289(i);
  vec4 p = permute(permute(permute(
          i.z + vec4(0.0, i1.z, i2.z, 1.0))
          + i.y + vec4(0.0, i1.y, i2.y, 1.0))
          + i.x + vec4(0.0, i1.x, i2.x, 1.0));

  float n_ = 0.142857142857;
  vec3 ns = n_ * D.wyz - D.xzx;

  vec4 j = p - 49.0 * floor(p * ns.z * ns.z);

  vec4 x_ = floor(j * ns.z);
  vec4 y_ = floor(j - 7.0 * x_);

  vec4 x = x_ * ns.x + ns.yyyy;
  vec4 y = y_ * ns.x + ns.yyyy;
  vec4 h = 1.0 - abs(x) - abs(y);

  vec4 b0 = vec4(x.xy, y.xy);
  vec4 b1 = vec4(x.zw, y.zw);

  vec4 s0 = floor(b0) * 2.0 + 1.0;
  vec4 s1 = floor(b1) * 2.0 + 1.0;
  vec4 sh = -step(h, vec4(0.0));

  vec4 a0 = b0.xzyw + s0.xzyw * sh.xxyy;
  vec4 a1 = b1.xzyw + s1.xzyw * sh.zzww;

  vec3 p0 = vec3(a0.xy, h.x);
  vec3 p1 = vec3(a0.zw, h.y);
  vec3 p2 = vec3(a1.xy, h.z);
  vec3 p3 = vec3(a1.zw, h.w);

  vec4 norm = taylorInvSqrt(vec4(dot(p0, p0), dot(p1, p1), dot(p2, p2), dot(p3, p3)));
  p0 *= norm.x;
  p1 *= norm.y;
  p2 *= norm.z;
  p3 *= norm.w;

  vec4 m = max(0.6 - vec4(dot(x0, x0), dot(x1, x1), dot(x2, x2), dot(x3, x3)), 0.0);
  m = m * m;
  return 42.0 * dot(m * m, vec4(dot(p0, x0), dot(p1, x1), dot(p2, x2), dot(p3, x3)));
}

// Helper functions
vec3 mod289(vec3 x) {
  return x - floor(x * (1.0 / 289.0)) * 289.0;
}

vec4 mod289(vec4 x) {
  return x - floor(x * (1.0 / 289.0)) * 289.0;
}

vec4 permute(vec4 x) {
  return mod289(((x * 34.0) + 1.0) * x);
}

vec4 taylorInvSqrt(vec4 r) {
  return 1.79284291400159 - 0.85373472095314 * r;
}

// Fractional Brownian Motion
float fbm2D(vec2 p, int octaves, float lacunarity, float gain) {
  float value = 0.0;
  float amplitude = 0.5;
  float frequency = 1.0;
  
  for (int i = 0; i < octaves; i++) {
    value += amplitude * simplex2D(p * frequency);
    frequency *= lacunarity;
    amplitude *= gain;
  }
  
  return value;
}

float fbm3D(vec3 p, int octaves, float lacunarity, float gain) {
  float value = 0.0;
  float amplitude = 0.5;
  float frequency = 1.0;
  
  for (int i = 0; i < octaves; i++) {
    value += amplitude * simplex3D(p * frequency);
    frequency *= lacunarity;
    amplitude *= gain;
  }
  
  return value;
}

// Domain warping
vec2 domainWarp2D(vec2 p, float strength) {
  vec2 q = vec2(simplex2D(p + vec2(0.0, 0.0)),
                simplex2D(p + vec2(5.2, 1.3)));
  
  vec2 r = vec2(simplex2D(p + 4.0 * q + vec2(1.7, 9.2)),
                simplex2D(p + 4.0 * q + vec2(8.3, 2.8)));
  
  return p + strength * r;
}

vec3 domainWarp3D(vec3 p, float strength) {
  vec3 q = vec3(simplex3D(p + vec3(0.0, 0.0, 0.0)),
                simplex3D(p + vec3(5.2, 1.3, 3.1)),
                simplex3D(p + vec3(2.8, 7.4, 1.9)));
  
  vec3 r = vec3(simplex3D(p + 4.0 * q + vec3(1.7, 9.2, 4.3)),
                simplex3D(p + 4.0 * q + vec3(8.3, 2.8, 6.7)),
                simplex3D(p + 4.0 * q + vec3(3.5, 5.9, 8.1)));
  
  return p + strength * r;
}

// Ridged noise
float ridgedNoise2D(vec2 p) {
  return 1.0 - abs(simplex2D(p));
}

float ridgedNoise3D(vec3 p) {
  return 1.0 - abs(simplex3D(p));
}

// Billowy noise
float billowyNoise2D(vec2 p) {
  return abs(simplex2D(p));
}

float billowyNoise3D(vec3 p) {
  return abs(simplex3D(p));
}

// Voronoi noise (simplified)
float voronoi(vec2 p) {
  vec2 g = floor(p);
  vec2 f = fract(p);
  
  float minDist = 1.0;
  
  for (int i = -1; i <= 1; i++) {
    for (int j = -1; j <= 1; j++) {
      vec2 neighbor = vec2(float(i), float(j));
      vec2 point = hash22(g + neighbor);
      point = 0.5 + 0.5 * sin(6.2831 * point);
      
      vec2 diff = neighbor + point - f;
      float dist = length(diff);
      minDist = min(minDist, dist);
    }
  }
  
  return minDist;
}

// Terrain-specific noise combinations
float terrainHeight(vec2 pos, float scale, float time) {
  vec2 p = pos * scale;
  
  // Base terrain with multiple octaves
  float height = fbm2D(p, 4, 2.0, 0.5);
  
  // Add ridged mountains
  float ridges = ridgedNoise2D(p * 0.3) * 0.8;
  height += ridges * ridges; // Square for sharper peaks
  
  // Add fine detail
  height += fbm2D(p * 4.0, 3, 2.0, 0.25) * 0.1;
  
  // Animate with time (subtle)
  height += simplex2D(p + time * 0.01) * 0.02;
  
  return height;
}

float terrainMoisture(vec2 pos, float scale) {
  vec2 p = domainWarp2D(pos * scale, 0.1);
  return fbm2D(p, 3, 2.0, 0.6) * 0.5 + 0.5;
}

float terrainTemperature(vec2 pos, float scale, float latitude) {
  vec2 p = pos * scale;
  float temp = fbm2D(p * 0.5, 2, 2.0, 0.5);
  
  // Add latitude influence
  temp *= 1.0 - abs(latitude);
  
  return temp * 0.5 + 0.5;
}

#pragma glslify: export(simplex2D)
#pragma glslify: export(simplex3D)
#pragma glslify: export(fbm2D)
#pragma glslify: export(fbm3D)
#pragma glslify: export(domainWarp2D)
#pragma glslify: export(domainWarp3D)
#pragma glslify: export(ridgedNoise2D)
#pragma glslify: export(ridgedNoise3D)
#pragma glslify: export(billowyNoise2D)
#pragma glslify: export(billowyNoise3D)
#pragma glslify: export(voronoi)
#pragma glslify: export(terrainHeight)
#pragma glslify: export(terrainMoisture)
#pragma glslify: export(terrainTemperature)
