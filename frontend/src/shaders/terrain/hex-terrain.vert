/**
 * Hex Terrain Vertex Shader
 * Handles instanced hex tile rendering with height displacement
 */

#ifdef GL_ES
precision highp float;
#endif

// Note: Standard attributes (position, normal, uv) and uniforms (modelMatrix, viewMatrix, etc.)
// are automatically provided by Three.js - no need to declare them explicitly

// Instanced attributes (per-hex data)
attribute vec3 instancePosition;
attribute vec3 instanceColor;
attribute float instanceHeight;
attribute float instanceBiome;
attribute vec2 instanceTexCoords;
attribute float instanceResourceMask;

// Custom uniforms
uniform float u_time;
uniform float u_hexSize;
uniform float u_hexSpacing;
uniform float u_heightScale;
uniform float u_lodDistance;
uniform int u_qualityLevel;

// Varyings to fragment shader
varying vec3 v_worldPosition;
varying vec3 v_viewPosition;
varying vec3 v_normal;
varying vec2 v_uv;
varying vec3 v_color;
varying float v_height;
varying float v_biome;
varying float v_resourceMask;
varying float v_lodLevel;
varying vec3 v_instancePosition;

// Shadow-specific varyings
#ifdef USE_SHADOWS
varying vec4 v_shadowCoord[4]; // Support up to 4 cascades
varying float v_shadowDistance;
#endif

// Import utility modules - Add functions directly since includes aren't working
// #include ../modules/noise.glsl
// #include ../modules/hex.glsl
// #include ../modules/common.glsl

// Essential noise functions for vertex shader
vec2 mod289(vec2 x) {
  return x - floor(x * (1.0 / 289.0)) * 289.0;
}

vec3 mod289(vec3 x) {
  return x - floor(x * (1.0 / 289.0)) * 289.0;
}

vec3 permute(vec3 x) {
  return mod289(((x * 34.0) + 1.0) * x);
}

float simplex2D(vec2 v) {
  const vec4 C = vec4(0.211324865405187, 0.366025403784439, -0.577350269189626, 0.024390243902439);
  vec2 i = floor(v + dot(v, C.yy));
  vec2 x0 = v - i + dot(i, C.xx);
  vec2 i1 = (x0.x > x0.y) ? vec2(1.0, 0.0) : vec2(0.0, 1.0);
  vec4 x12 = x0.xyxy + C.xxzz;
  x12.xy -= i1;
  i = mod289(i.xy);
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

float terrainHeight(vec2 pos, float scale, float time) {
  vec2 p = pos * scale;
  // Simple fbm approximation using simplex noise  
  float height = simplex2D(p) * 0.5 + 0.5;
  height += simplex2D(p * 2.0) * 0.25;
  height += simplex2D(p * 4.0) * 0.125;
  // Add time-based animation
  height += simplex2D(p + time * 0.01) * 0.02;
  return height;
}

#ifdef USE_SHADOWS
// CSM matrices - these would normally come from shadows module
uniform mat4 csmMatrix0;
uniform mat4 csmMatrix1;
uniform mat4 csmMatrix2;
uniform mat4 csmMatrix3;
#endif

// Calculate LOD based on distance to camera
float calculateLOD(vec3 worldPos, vec3 cameraPos, float maxDistance) {
  float distance = length(worldPos - cameraPos);
  float lod = clamp(distance / maxDistance, 0.0, 1.0);
  return lod;
}

// Generate vertex displacement for terrain detail
vec3 displaceTerrain(vec3 pos, vec2 hexCoord, float height, float lod) {
  vec3 displaced = pos;
  
  // Base height displacement
  displaced.y += height * u_heightScale;
  
  // Add noise-based micro displacement at close range
  if (lod < 0.5 && u_qualityLevel > 2) {
    float microNoise = simplex2D(hexCoord * 8.0 + u_time * 0.1) * 0.05;
    displaced.y += microNoise * (1.0 - lod * 2.0);
  }
  
  // Add subtle animation for water/dynamic tiles
  if (instanceBiome > 0.8) { // Water biomes
    float wave = sin(u_time + dot(hexCoord, vec2(1.0, 0.5))) * 0.02;
    displaced.y += wave * (1.0 - lod);
  }
  
  return displaced;
}

// Generate normals for terrain
vec3 calculateTerrainNormal(vec2 hexCoord, float height, float lod) {
  if (lod > 0.7) {
    // Use standard normal for distant tiles
    return normal;
  }
  
  // Calculate height gradient for nearby tiles
  float offset = 0.1;
  float heightL = terrainHeight(hexCoord - vec2(offset, 0.0), 1.0, u_time);
  float heightR = terrainHeight(hexCoord + vec2(offset, 0.0), 1.0, u_time);
  float heightD = terrainHeight(hexCoord - vec2(0.0, offset), 1.0, u_time);
  float heightU = terrainHeight(hexCoord + vec2(0.0, offset), 1.0, u_time);
  
  vec3 tangent = normalize(vec3(2.0 * offset, (heightR - heightL) * u_heightScale, 0.0));
  vec3 bitangent = normalize(vec3(0.0, (heightU - heightD) * u_heightScale, 2.0 * offset));
  
  return normalize(cross(tangent, bitangent));
}

void main() {
  // Calculate hex coordinates
  vec2 hexCoord = instancePosition.xz / u_hexSpacing;
  
  // Transform position to instance space
  vec3 transformed = position * u_hexSize;
  transformed += instancePosition;
  
  // Calculate LOD
  vec3 worldPos = (modelMatrix * vec4(transformed, 1.0)).xyz;
  float lod = calculateLOD(worldPos, cameraPosition, u_lodDistance);
  
  // Apply terrain displacement
  transformed = displaceTerrain(transformed, hexCoord, instanceHeight, lod);
  
  // Calculate final world position
  vec4 worldPosition = modelMatrix * vec4(transformed, 1.0);
  vec4 viewPosition = viewMatrix * worldPosition;
  
  // Calculate normals
  vec3 terrainNormal = calculateTerrainNormal(hexCoord, instanceHeight, lod);
  vec3 worldNormal = normalMatrix * terrainNormal;
  
  // Pass data to fragment shader
  v_worldPosition = worldPosition.xyz;
  v_viewPosition = viewPosition.xyz;
  v_normal = normalize(worldNormal);
  v_uv = uv;
  v_color = instanceColor;
  v_height = instanceHeight;
  v_biome = instanceBiome;
  v_resourceMask = instanceResourceMask;
  v_lodLevel = lod;
  v_instancePosition = instancePosition;

#ifdef USE_SHADOWS
  // Calculate shadow coordinates for all cascades
  v_shadowCoord[0] = csmMatrix0 * worldPosition;
  v_shadowCoord[1] = csmMatrix1 * worldPosition;
  v_shadowCoord[2] = csmMatrix2 * worldPosition;
  v_shadowCoord[3] = csmMatrix3 * worldPosition;
  
  // Store view distance for cascade selection
  v_shadowDistance = -viewPosition.z;
#endif
  
  // Final position
  gl_Position = projectionMatrix * viewPosition;
  
  // LOD-based point size for distant rendering
  if (lod > 0.8) {
    gl_PointSize = 2.0 * (1.0 - lod) + 1.0;
  }
}
