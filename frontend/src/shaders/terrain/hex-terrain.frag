/**
 * Hex Terrain Fragment Shader (Optimized)
 * Renders hex tiles with biome-based coloring, resources, and effects
 */

#ifdef GL_ES
precision highp float;
#endif

// GeometricContext struct will be automatically injected by the shader manager

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

// Essential uniforms only
uniform vec3 u_lightDirection;
uniform vec3 u_lightColor;
uniform float u_lightIntensity;
uniform vec3 u_ambientColor;
uniform float u_ambientIntensity;
uniform float u_time;
uniform float u_exposure;

// Material property uniforms
uniform float u_roughness;
uniform float u_metallic;

// Procedural texture uniforms
uniform bool u_hasAlbedoTexture;
uniform bool u_hasNormalTexture;
uniform bool u_hasRoughnessTexture;
uniform bool u_hasMetallicTexture;
uniform sampler2D u_albedoTexture;
uniform sampler2D u_normalTexture;
uniform sampler2D u_roughnessTexture;
uniform sampler2D u_metallicTexture;
uniform float u_textureScale;

// Debug uniforms
uniform bool u_showLOD;
uniform bool u_showBiomes;
uniform bool u_showHeight;

// Fast hash function instead of complex simplex noise
float hash(vec2 p) {
  return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

// Simple noise using hash function
float noise(vec2 p) {
  vec2 i = floor(p);
  vec2 f = fract(p);
  f = f * f * (3.0 - 2.0 * f);
  return mix(mix(hash(i), hash(i + vec2(1.0, 0.0)), f.x),
             mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), f.x), f.y);
}

// Fast moisture calculation
float terrainMoisture(vec2 pos) {
  return noise(pos * 0.1) * 0.5 + noise(pos * 0.05) * 0.3 + 0.2;
}

// Optimized biome color lookup using conditional branches instead of arrays
vec3 getBiomeColor(float biomeID, float height, float moisture) {
  int index = int(floor(biomeID * 9.0 + 0.5));
  vec3 baseColor;
  
  if (index == 0) baseColor = vec3(0.0, 0.4, 0.8);        // Ocean
  else if (index == 1) baseColor = vec3(0.2, 0.8, 0.3);   // Grassland
  else if (index == 2) baseColor = vec3(0.5, 0.9, 0.2);   // Plains
  else if (index == 3) baseColor = vec3(0.9, 0.8, 0.2);   // Desert
  else if (index == 4) baseColor = vec3(0.7, 0.7, 0.8);   // Tundra
  else if (index == 5) baseColor = vec3(0.9, 0.9, 1.0);   // Snow
  else if (index == 6) baseColor = vec3(0.1, 0.5, 0.1);   // Forest
  else if (index == 7) baseColor = vec3(0.2, 0.6, 0.2);   // Jungle
  else if (index == 8) baseColor = vec3(0.6, 0.4, 0.2);   // Hills
  else baseColor = vec3(0.4, 0.4, 0.5);                   // Mountain
  
  // Simple height and moisture modulation
  float heightMod = height;
  if (index != 5) baseColor *= 0.9 + heightMod * 0.2;
  if (index >= 1 && index <= 2) baseColor = mix(baseColor, vec3(0.1, 0.7, 0.1), moisture * 0.15);
  
  return baseColor;
}

// Simplified resource color
vec3 getResourceColor(float resourceMask) {
  if (resourceMask > 0.5) return vec3(0.8, 0.6, 0.2); // Gold highlight
  return vec3(0.0);
}

// Enhanced PBR lighting with normal, roughness, and metallic mapping
vec3 calculateLighting(vec3 albedo, vec3 worldNormal, vec3 lightDir, vec2 textureUV) {
  vec3 normal = normalize(worldNormal);
  
  // Apply normal mapping if available
  if (u_hasNormalTexture) {
    vec3 normalMap = texture2D(u_normalTexture, textureUV).xyz * 2.0 - 1.0;
    // Simple normal mapping approximation
    normal = normalize(normal + normalMap * 0.3);
  }
  
  // Sample roughness texture
  float roughness = u_roughness;
  if (u_hasRoughnessTexture) {
    roughness = texture2D(u_roughnessTexture, textureUV).r;
  }
  
  // Sample metallic texture
  float metallic = u_metallic;
  if (u_hasMetallicTexture) {
    metallic = texture2D(u_metallicTexture, textureUV).r;
  }
  
  // Setup geometric context for Three.js compatibility
  GeometricContext geometry;
  geometry.position = -v_viewPosition;
  geometry.normal = normal;
  geometry.viewDir = normalize(v_viewPosition);
  
  // Basic PBR calculations
  float NdotL = max(dot(normal, lightDir), 0.0);
  vec3 ambient = u_ambientColor * u_ambientIntensity;
  vec3 diffuse = u_lightColor * u_lightIntensity * NdotL;
  
  // Apply metallic workflow
  vec3 baseColor = albedo;
  vec3 diffuseColor = baseColor * (1.0 - metallic);
  vec3 specularColor = mix(vec3(0.04), baseColor, metallic);
  
  vec3 finalDiffuse = diffuseColor * (ambient + diffuse);
  
  // Simple specular reflection using geometry context
  vec3 reflectDir = reflect(-lightDir, geometry.normal);
  float spec = pow(max(dot(geometry.viewDir, reflectDir), 0.0), mix(4.0, 128.0, 1.0 - roughness));
  vec3 specular = specularColor * spec * u_lightIntensity;
  
  return finalDiffuse + specular;
}

// Debug visualization
vec3 getDebugColor() {
  if (u_showLOD) return mix(vec3(0.0, 1.0, 0.0), vec3(1.0, 0.0, 0.0), v_lodLevel);
  if (u_showBiomes) return vec3(v_biome, 0.8, 0.9);
  if (u_showHeight) return vec3(v_height);
  return vec3(1.0);
}

void main() {
  vec3 normal = normalize(v_normal);
  vec3 lightDir = normalize(-u_lightDirection);
  
  // Fast biome color calculation
  float moisture = terrainMoisture(v_instancePosition.xz);
  vec3 baseColor = getBiomeColor(v_biome, v_height, moisture);
  baseColor *= v_color;
  
  // Apply procedural texture if available
  if (u_hasAlbedoTexture) {
    // Use UV coordinates from vertex shader instead of world position for better texture mapping
    vec2 textureUV = v_uv * u_textureScale;
    vec3 textureColor = texture2D(u_albedoTexture, textureUV).rgb;
    
    // Blend texture with base biome color for variation
    baseColor = mix(baseColor, textureColor, 0.8);
  } else {
    // Add subtle noise variation with fast hash (fallback)
    float noiseVar = noise(v_worldPosition.xz * 0.5 + u_time * 0.01);
    baseColor *= 0.9 + noiseVar * 0.2;
  }
  
  // Simple resource highlights
  vec3 resourceColor = getResourceColor(v_resourceMask);
  baseColor += resourceColor * 0.2;
  
  // Calculate final color with enhanced lighting
  vec2 textureUV = v_worldPosition.xz * u_textureScale;
  vec3 finalColor = calculateLighting(baseColor, normal, lightDir, textureUV);
  
  // Debug override
  if (u_showLOD || u_showBiomes || u_showHeight) {
    finalColor = mix(finalColor, getDebugColor(), 0.7);
  }
  
  // Apply exposure
  finalColor *= u_exposure;
  
  gl_FragColor = vec4(finalColor, 1.0);
}
