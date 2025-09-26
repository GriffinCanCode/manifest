/**
 * Animated Water Vertex Shader
 * Provides wave displacement and vertex data for water animation
 */

#include common.glsl

// Uniforms
uniform float u_time;
uniform float u_waveHeight;
uniform float u_waveSpeed;
uniform vec3 u_cameraPosition;

// Instance attributes
attribute vec3 a_instancePosition;
attribute vec3 a_instanceScale;
attribute vec4 a_instanceColor;
attribute float a_instanceHeight;
attribute float a_instanceBiome;

// Output to fragment shader
varying vec2 v_uv;
varying vec3 v_normal;
varying vec3 v_worldPosition;
varying vec3 v_viewPosition;
varying vec4 v_color;
varying float v_height;
varying float v_lodLevel;

// Water wave calculation (matches fragment shader)
float waterWaves(vec2 position, float time) {
  float waves = 0.0;
  float amplitude = 1.0;
  float frequency = 1.0;
  vec2 pos = position;
  
  // Primary wave
  waves += sin(pos.x * frequency * 2.0 + time * u_waveSpeed) * amplitude * 0.5;
  waves += sin(pos.y * frequency * 1.5 + time * u_waveSpeed * 0.8) * amplitude * 0.3;
  
  // Secondary wave
  amplitude *= 0.5;
  frequency *= 2.0;
  pos = position * 1.2;
  waves += sin(pos.x * frequency + pos.y * frequency * 0.5 + time * u_waveSpeed * 1.2) * amplitude;
  
  // Tertiary wave
  amplitude *= 0.5;
  frequency *= 2.0;
  pos = position * 2.1;
  waves += sin(pos.x * frequency * 0.8 + pos.y * frequency * 1.1 + time * u_waveSpeed * 0.6) * amplitude;
  
  return waves * u_waveHeight;
}

// Calculate normal from wave function
vec3 calculateWaveNormal(vec2 position, float time) {
  float eps = 0.01;
  
  float h0 = waterWaves(position, time);
  float hx = waterWaves(position + vec2(eps, 0.0), time);
  float hy = waterWaves(position + vec2(0.0, eps), time);
  
  vec3 tangentX = vec3(eps, hx - h0, 0.0);
  vec3 tangentY = vec3(0.0, hy - h0, eps);
  
  return normalize(cross(tangentX, tangentY));
}

// LOD calculation based on distance
float calculateLOD(vec3 worldPos, vec3 cameraPos) {
  float distance = length(worldPos - cameraPos);
  float lodLevel = clamp(distance / 50.0, 0.0, 1.0);
  return lodLevel;
}

void main() {
  // UV coordinates
  v_uv = uv;
  
  // Instance transformation
  vec3 instancePos = position * a_instanceScale + a_instancePosition;
  vec2 worldPosXZ = instancePos.xz;
  
  // Calculate wave displacement
  float waveDisplacement = waterWaves(worldPosXZ, u_time);
  
  // Apply wave to vertex position
  instancePos.y += waveDisplacement;
  
  // Calculate world position
  vec4 worldPosition = modelMatrix * vec4(instancePos, 1.0);
  v_worldPosition = worldPosition.xyz;
  
  // Calculate view position
  vec4 viewPosition = viewMatrix * worldPosition;
  v_viewPosition = viewPosition.xyz;
  
  // Calculate LOD level
  v_lodLevel = calculateLOD(v_worldPosition, u_cameraPosition);
  
  // LOD-based vertex displacement reduction
  float lodMultiplier = 1.0 - v_lodLevel * 0.5;
  instancePos.y *= lodMultiplier;
  
  // Recalculate positions with LOD
  worldPosition = modelMatrix * vec4(instancePos, 1.0);
  v_worldPosition = worldPosition.xyz;
  viewPosition = viewMatrix * worldPosition;
  v_viewPosition = viewPosition.xyz;
  
  // Calculate wave normal
  vec3 waveNormal = calculateWaveNormal(worldPosXZ, u_time);
  
  // Transform normal to world space
  vec3 worldNormal = normalize(mat3(modelMatrix) * waveNormal);
  v_normal = worldNormal;
  
  // Pass through instance data
  v_color = a_instanceColor;
  v_height = a_instanceHeight;
  
  // Final position
  gl_Position = projectionMatrix * viewPosition;
}
