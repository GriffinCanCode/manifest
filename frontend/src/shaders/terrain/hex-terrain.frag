/**
 * Hex Terrain Fragment Shader
 * Renders hex tiles with biome-based coloring, resources, and effects
 */

#ifdef GL_ES
precision highp float;
#endif

// Varyings from vertex shader
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

// Lighting uniforms
uniform vec3 u_lightDirection;
uniform vec3 u_lightColor;
uniform float u_lightIntensity;
uniform vec3 u_ambientColor;
uniform float u_ambientIntensity;

// Material uniforms
uniform float u_roughness;
uniform float u_metallic;
uniform float u_specularIntensity;

// HDR uniforms
uniform float u_exposure;
uniform float u_emissiveIntensity;

// Rendering uniforms
uniform vec3 u_cameraPosition;
uniform float u_time;
uniform vec2 u_resolution;
uniform bool u_wireframe;
uniform float u_wireframeWidth;

// Fog uniforms
uniform vec3 u_fogColor;
uniform float u_fogNear;
uniform float u_fogFar;
uniform float u_fogDensity;

// Debug uniforms
uniform bool u_showLOD;
uniform bool u_showBiomes;
uniform bool u_showResources;
uniform bool u_showHeight;

// Import utility modules
#include ../modules/common.glsl
#include ../modules/hex.glsl
#include ../modules/noise.glsl

#ifdef USE_SHADOWS
#include ../modules/shadows.glsl
#endif

// Biome color palette
vec3 getBiomeColor(float biomeID, float height, float moisture) {
  vec3 colors[10];
  colors[0] = vec3(0.2, 0.6, 0.8);   // Deep ocean
  colors[1] = vec3(0.3, 0.7, 0.9);   // Shallow water
  colors[2] = vec3(0.8, 0.7, 0.4);   // Beach/sand
  colors[3] = vec3(0.2, 0.7, 0.2);   // Grassland
  colors[4] = vec3(0.1, 0.4, 0.1);   // Forest
  colors[5] = vec3(0.6, 0.5, 0.3);   // Plains
  colors[6] = vec3(0.4, 0.3, 0.2);   // Hills
  colors[7] = vec3(0.6, 0.6, 0.7);   // Mountains
  colors[8] = vec3(0.9, 0.9, 1.0);   // Snow/ice
  colors[9] = vec3(0.8, 0.6, 0.2);   // Desert
  
  int index = int(floor(biomeID * 9.0));
  index = clamp(index, 0, 9);
  
  vec3 baseColor = colors[index];
  
  // Modulate with height and moisture
  float heightMod = smoothstep(0.0, 1.0, height);
  float moistureMod = moisture;
  
  // Darken with height (except snow)
  if (index != 8) {
    baseColor *= mix(0.7, 1.2, 1.0 - heightMod * 0.5);
  }
  
  // Green tint with moisture (for applicable biomes)
  if (index >= 3 && index <= 4) {
    baseColor = mix(baseColor, vec3(0.1, 0.6, 0.1), moistureMod * 0.3);
  }
  
  return baseColor;
}

// Resource indicators
vec3 getResourceColor(float resourceMask) {
  vec3 resourceColors[8];
  resourceColors[0] = vec3(0.8, 0.6, 0.2); // Gold
  resourceColors[1] = vec3(0.5, 0.5, 0.6); // Iron
  resourceColors[2] = vec3(0.2, 0.2, 0.8); // Coal
  resourceColors[3] = vec3(0.1, 0.8, 0.1); // Oil
  resourceColors[4] = vec3(0.8, 0.2, 0.2); // Rare minerals
  resourceColors[5] = vec3(0.6, 0.3, 0.8); // Crystals
  resourceColors[6] = vec3(0.9, 0.9, 0.9); // Stone
  resourceColors[7] = vec3(0.4, 0.8, 0.8); // Water
  
  vec3 result = vec3(0.0);
  float mask = resourceMask;
  
  for (int i = 0; i < 8; i++) {
    float bit = floor(mod(mask, 2.0));
    result += resourceColors[i] * bit * 0.3;
    mask = floor(mask / 2.0);
  }
  
  return result;
}

// Simple PBR lighting
vec3 calculatePBRLighting(vec3 albedo, vec3 normal, vec3 viewDir, vec3 lightDir) {
  // Lambertian diffuse
  float NdotL = max(dot(normal, lightDir), 0.0);
  vec3 diffuse = albedo * NdotL;
  
  // Simplified specular (Blinn-Phong approximation)
  vec3 halfVector = normalize(lightDir + viewDir);
  float NdotH = max(dot(normal, halfVector), 0.0);
  float roughness2 = u_roughness * u_roughness;
  float spec = pow(NdotH, 2.0 / (roughness2 * roughness2));
  
  // Fresnel approximation
  float fresnel = pow(1.0 - max(dot(viewDir, normal), 0.0), 5.0);
  fresnel = mix(0.04, 1.0, fresnel);
  
  vec3 specular = vec3(spec * fresnel * u_specularIntensity);
  
  return diffuse + specular;
}

// Hex grid overlay
float getHexGrid(vec2 uv, float width) {
  vec2 hexCoord = v_instancePosition.xz;
  vec2 localUV = (v_worldPosition.xz - hexCoord) * 2.0;
  
  return hexEdge(localUV, width);
}

// Fog calculation
vec3 applyFog(vec3 color, float distance) {
  float fogFactor = exp(-distance * u_fogDensity);
  fogFactor = clamp(fogFactor, 0.0, 1.0);
  return mix(u_fogColor, color, fogFactor);
}

// Debug visualizations
vec3 getDebugColor() {
  if (u_showLOD) {
    return mix(vec3(0.0, 1.0, 0.0), vec3(1.0, 0.0, 0.0), v_lodLevel);
  }
  
  if (u_showBiomes) {
    return hsv2rgb(vec3(v_biome, 0.8, 0.9));
  }
  
  if (u_showResources) {
    return getResourceColor(v_resourceMask);
  }
  
  if (u_showHeight) {
    float heightColor = v_height;
    return vec3(heightColor, heightColor, heightColor);
  }
  
  return vec3(1.0); // Default
}

void main() {
  // Calculate lighting vectors
  vec3 normal = normalize(v_normal);
  vec3 viewDir = normalize(u_cameraPosition - v_worldPosition);
  vec3 lightDir = normalize(-u_lightDirection);
  
  // Base color from biome
  float moisture = terrainMoisture(v_instancePosition.xz, 0.1);
  vec3 baseColor = getBiomeColor(v_biome, v_height, moisture);
  
  // Add instance color modulation
  baseColor *= v_color;
  
  // Add resource highlights with HDR intensity
  vec3 resourceColor = getResourceColor(v_resourceMask);
  baseColor = mix(baseColor, baseColor + resourceColor * u_emissiveIntensity, 0.3);
  
  // Add subtle noise variation
  float noiseVariation = simplex2D(v_worldPosition.xz * 0.5 + u_time * 0.01);
  baseColor *= 0.9 + noiseVariation * 0.2;
  
  // Calculate shadow factor
  float shadowFactor = 1.0;
#ifdef USE_SHADOWS
  shadowFactor = calculateCSMShadowFromVaryings(v_shadowCoord, v_shadowDistance, normal);
#endif

  // Calculate lighting with shadows
  vec3 litColor = calculatePBRLighting(baseColor, normal, viewDir, lightDir);
  litColor *= shadowFactor; // Apply shadows to direct lighting
  
  // Add ambient lighting (not affected by shadows)
  vec3 ambient = u_ambientColor * u_ambientIntensity * baseColor;
  litColor += ambient;
  
  // Apply wireframe if enabled
  if (u_wireframe) {
    float grid = getHexGrid(v_uv, u_wireframeWidth);
    litColor = mix(litColor, vec3(1.0), grid * 0.5);
  }
  
  // Apply fog
  float viewDistance = length(v_viewPosition);
  litColor = applyFog(litColor, viewDistance);
  
  // Debug override
  if (u_showLOD || u_showBiomes || u_showResources || u_showHeight) {
    litColor = mix(litColor, getDebugColor(), 0.7);
  }
  
  // LOD-based alpha for distant tiles
  float alpha = 1.0;
  if (v_lodLevel > 0.9) {
    alpha = (1.0 - v_lodLevel) * 10.0;
    alpha = clamp(alpha, 0.1, 1.0);
  }
  
  // Apply HDR exposure
  litColor *= u_exposure;
  
  // Output linear HDR color (no gamma correction - handled by tone mapping)
  // The post-processing pipeline will handle tone mapping and gamma correction
  gl_FragColor = vec4(litColor, alpha);
}
