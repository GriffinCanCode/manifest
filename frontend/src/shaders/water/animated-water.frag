/**
 * Animated Water Shader for Ocean Hex Tiles
 * GPU-optimized water animation with waves, foam, and transparency
 */

#ifdef GL_ES
precision highp float;
#endif

#include ../modules/common.glsl
#include ../modules/noise.glsl

// Water animation uniforms
uniform float u_time;
uniform vec3 u_waterColor;
uniform vec3 u_foamColor;
uniform vec3 u_deepWaterColor;
uniform float u_waveHeight;
uniform float u_waveSpeed;
uniform float u_foamThreshold;
uniform float u_transparency;
// Note: cameraPosition is automatically provided by Three.js
uniform vec3 u_lightDirection;

// Lighting uniforms
uniform vec3 u_ambientColor;
uniform float u_ambientIntensity;
uniform float u_specularIntensity;
uniform float u_roughness;

// Instance data from vertex shader
varying vec2 v_uv;
varying vec3 v_normal;
varying vec3 v_worldPosition;
varying vec3 v_viewPosition;
varying vec4 v_color;
varying float v_height;
varying float v_lodLevel;

// Water wave calculation using multiple octaves
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
  
  // Tertiary wave for detail
  amplitude *= 0.5;
  frequency *= 2.0;
  pos = position * 2.1;
  waves += sin(pos.x * frequency * 0.8 + pos.y * frequency * 1.1 + time * u_waveSpeed * 0.6) * amplitude;
  
  return waves * u_waveHeight;
}

// Generate foam pattern
float generateFoam(vec2 position, float time, float waveValue) {
  // Foam appears at wave peaks and shorelines
  float foam = 0.0;
  
  // Wave-based foam
  float waveThreshold = u_foamThreshold;
  if (waveValue > waveThreshold) {
    foam += (waveValue - waveThreshold) / (1.0 - waveThreshold);
  }
  
  // Animated foam texture using noise
  vec2 foamUV = position * 3.0 + time * 0.2;
  float foamNoise = simplex2D(foamUV);
  foamNoise += simplex2D(foamUV * 2.0 + time * 0.3) * 0.5;
  foamNoise += simplex2D(foamUV * 4.0 - time * 0.1) * 0.25;
  
  foam *= max(0.0, foamNoise * 0.5 + 0.5);
  
  // Edge-based foam for hex boundaries
  float edgeDistance = min(
    min(v_uv.x, 1.0 - v_uv.x),
    min(v_uv.y, 1.0 - v_uv.y)
  );
  float edgeFoam = smoothstep(0.0, 0.2, 1.0 - edgeDistance);
  foam += edgeFoam * 0.3;
  
  return clamp(foam, 0.0, 1.0);
}

// Calculate water surface normal using wave derivatives
vec3 calculateWaterNormal(vec2 position, float time) {
  float eps = 0.01;
  
  float h0 = waterWaves(position, time);
  float hx = waterWaves(position + vec2(eps, 0.0), time);
  float hy = waterWaves(position + vec2(0.0, eps), time);
  
  vec3 tangentX = vec3(eps, hx - h0, 0.0);
  vec3 tangentY = vec3(0.0, hy - h0, eps);
  
  vec3 normal = normalize(cross(tangentX, tangentY));
  return normal;
}

// Fresnel reflection calculation
float calculateFresnel(vec3 viewDir, vec3 normal) {
  float cosTheta = max(dot(viewDir, normal), 0.0);
  float f0 = 0.02; // Water IOR approximation
  return f0 + (1.0 - f0) * pow(1.0 - cosTheta, 5.0);
}

// PBR water lighting
vec3 calculateWaterLighting(vec3 baseColor, vec3 normal, vec3 viewDir, vec3 lightDir) {
  // Diffuse component
  float NdotL = max(dot(normal, lightDir), 0.0);
  vec3 diffuse = baseColor * NdotL;
  
  // Specular reflection
  vec3 reflectDir = reflect(-lightDir, normal);
  float specular = pow(max(dot(viewDir, reflectDir), 0.0), 1.0 / (u_roughness * u_roughness));
  
  // Fresnel-modulated specular
  float fresnel = calculateFresnel(viewDir, normal);
  vec3 specularColor = vec3(fresnel * specular * u_specularIntensity);
  
  // Ambient lighting
  vec3 ambient = u_ambientColor * u_ambientIntensity * baseColor;
  
  return diffuse + specularColor + ambient;
}

// Depth-based color mixing
vec3 getWaterDepthColor(float depth, vec3 shallowColor, vec3 deepColor) {
  float normalizedDepth = clamp(depth / 5.0, 0.0, 1.0);
  return mix(shallowColor, deepColor, normalizedDepth);
}

// Caustic light patterns
float calculateCaustics(vec2 position, float time) {
  vec2 causticsUV = position * 0.5;
  
  float caustics = 0.0;
  caustics += sin(causticsUV.x * 8.0 + time * 2.0) * sin(causticsUV.y * 8.0 + time * 1.5);
  caustics += sin((causticsUV.x + causticsUV.y) * 6.0 + time * 1.8);
  caustics += sin((causticsUV.x - causticsUV.y) * 4.0 + time * 2.2);
  
  caustics = caustics * 0.1 + 0.9;
  return max(0.0, caustics);
}

void main() {
  vec2 worldPos = v_worldPosition.xz;
  float time = u_time;
  
  // Calculate wave displacement
  float waveValue = waterWaves(worldPos, time);
  
  // Calculate animated surface normal
  vec3 waterNormal = calculateWaterNormal(worldPos, time);
  
  // Mix with base normal for stability
  vec3 finalNormal = normalize(mix(v_normal, waterNormal, 0.7));
  
  // Calculate view direction
  vec3 viewDir = normalize(cameraPosition - v_worldPosition);
  vec3 lightDir = normalize(-u_lightDirection);
  
  // Base water color based on depth
  float waterDepth = max(0.1, -v_height + waveValue);
  vec3 baseWaterColor = getWaterDepthColor(waterDepth, u_waterColor, u_deepWaterColor);
  
  // Generate foam
  float foamAmount = generateFoam(worldPos, time, waveValue);
  
  // Mix water and foam colors
  vec3 surfaceColor = mix(baseWaterColor, u_foamColor, foamAmount);
  
  // Apply lighting
  vec3 litColor = calculateWaterLighting(surfaceColor, finalNormal, viewDir, lightDir);
  
  // Add caustic effects for shallow water
  if (waterDepth < 3.0) {
    float caustics = calculateCaustics(worldPos, time);
    litColor *= caustics;
  }
  
  // Add subtle animation highlights
  float highlight = sin(time * 1.5 + worldPos.x * 0.1 + worldPos.y * 0.08) * 0.05 + 0.95;
  litColor *= highlight;
  
  // Calculate transparency based on depth and foam
  float alpha = mix(u_transparency, 1.0, foamAmount);
  alpha = mix(alpha, 0.9, clamp(waterDepth / 2.0, 0.0, 1.0));
  
  // LOD-based alpha adjustment
  if (v_lodLevel > 0.8) {
    alpha *= (1.0 - v_lodLevel) * 5.0;
    alpha = clamp(alpha, 0.1, 1.0);
  }
  
  // Output linear HDR color (no gamma correction - handled by tone mapping)
  // The post-processing pipeline will handle tone mapping and gamma correction
  gl_FragColor = vec4(litColor, alpha);
}
