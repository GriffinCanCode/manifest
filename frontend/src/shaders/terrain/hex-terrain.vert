/**
 * Hex Terrain Vertex Shader
 * Handles instanced hex tile rendering with height displacement
 */

// Standard attributes
attribute vec3 position;
attribute vec3 normal;
attribute vec2 uv;

// Instanced attributes (per-hex data)
attribute vec3 instancePosition;
attribute vec3 instanceColor;
attribute float instanceHeight;
attribute float instanceBiome;
attribute vec2 instanceTexCoords;
attribute float instanceResourceMask;

// Standard uniforms
uniform mat4 modelMatrix;
uniform mat4 viewMatrix;
uniform mat4 projectionMatrix;
uniform mat3 normalMatrix;
uniform vec3 cameraPosition;

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

// Import utility modules
#include ../modules/noise.glsl
#include ../modules/hex.glsl
#include ../modules/common.glsl

#ifdef USE_SHADOWS
#include ../modules/shadows.glsl
// Shadow matrices (injected by CSM system)
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
