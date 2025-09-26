/**
 * Screen Space Reflections (SSR) Fragment Shader
 * Real-time reflections using screen space ray marching
 */

precision highp float;

uniform sampler2D tColor;
uniform sampler2D tDepth;
uniform sampler2D tNormal;
uniform vec2 u_resolution;
uniform mat4 u_viewMatrix;
uniform mat4 u_projectionMatrix;
uniform mat4 u_projectionMatrixInverse;
uniform vec3 u_cameraPosition;
uniform float u_cameraNear;
uniform float u_cameraFar;

// SSR parameters
uniform float u_ssrIntensity;
uniform float u_ssrMaxDistance;
uniform int u_ssrSteps;
uniform int u_ssrBinarySteps;
uniform float u_ssrThickness;
uniform float u_ssrJitter;
uniform float u_ssrFresnel;
uniform bool u_ssrEnabled;
uniform float u_time;

varying vec2 vUv;

#include ../utils/screen-space.glsl
#include ../utils/color.glsl
#include ../modules/common.glsl

// Ray marching for screen space reflections
vec4 screenSpaceRayMarch(vec3 rayStart, vec3 rayDir, float maxDistance, int steps, int binarySteps) {
  vec3 rayPos = rayStart;
  vec3 rayStep = rayDir * (maxDistance / float(steps));
  
  vec2 hitUV = vec2(-1.0);
  float depth = 0.0;
  bool hit = false;
  
  // Linear ray marching
  for (int i = 0; i < 128; i++) {
    if (i >= steps) break;
    
    rayPos += rayStep;
    
    // Project to screen space
    vec4 projectedPos = u_projectionMatrix * vec4(rayPos, 1.0);
    vec2 screenUV = (projectedPos.xy / projectedPos.w) * 0.5 + 0.5;
    
    // Check if ray is outside screen bounds
    if (screenUV.x < 0.0 || screenUV.x > 1.0 || 
        screenUV.y < 0.0 || screenUV.y > 1.0) {
      break;
    }
    
    // Sample depth at current position
    float sampledDepth = texture2D(tDepth, screenUV).r;
    vec3 sampledWorldPos = reconstructViewPos(screenUV, sampledDepth, u_projectionMatrixInverse);
    
    // Check for intersection
    if (rayPos.z > sampledWorldPos.z && 
        rayPos.z < sampledWorldPos.z + u_ssrThickness) {
      hitUV = screenUV;
      depth = sampledDepth;
      hit = true;
      break;
    }
  }
  
  // Binary search refinement
  if (hit && binarySteps > 0) {
    vec3 refinedStart = rayPos - rayStep;
    vec3 refinedEnd = rayPos;
    
    for (int i = 0; i < 16; i++) {
      if (i >= binarySteps) break;
      
      vec3 midPoint = (refinedStart + refinedEnd) * 0.5;
      vec4 projectedMid = u_projectionMatrix * vec4(midPoint, 1.0);
      vec2 midUV = (projectedMid.xy / projectedMid.w) * 0.5 + 0.5;
      
      if (midUV.x < 0.0 || midUV.x > 1.0 || 
          midUV.y < 0.0 || midUV.y > 1.0) {
        break;
      }
      
      float midDepth = texture2D(tDepth, midUV).r;
      vec3 midWorldPos = reconstructViewPos(midUV, midDepth, u_projectionMatrixInverse);
      
      if (midPoint.z > midWorldPos.z) {
        refinedEnd = midPoint;
        hitUV = midUV;
      } else {
        refinedStart = midPoint;
      }
    }
  }
  
  if (hit) {
    return vec4(hitUV, depth, 1.0);
  } else {
    return vec4(0.0, 0.0, 0.0, 0.0);
  }
}

// Calculate reflection fade based on screen edge proximity
float calculateEdgeFade(vec2 uv, float fadeWidth) {
  vec2 edge = smoothstep(0.0, fadeWidth, uv) * (1.0 - smoothstep(1.0 - fadeWidth, 1.0, uv));
  return edge.x * edge.y;
}

// Calculate distance fade
float calculateDistanceFade(float distance, float maxDistance) {
  return 1.0 - smoothstep(maxDistance * 0.7, maxDistance, distance);
}

void main() {
  if (!u_ssrEnabled || u_ssrIntensity <= 0.0) {
    gl_FragColor = vec4(texture2D(tColor, vUv).rgb, 1.0);
    return;
  }
  
  vec3 originalColor = texture2D(tColor, vUv).rgb;
  float depth = texture2D(tDepth, vUv).r;
  
  // Skip skybox
  if (depth >= 0.9999) {
    gl_FragColor = vec4(originalColor, 1.0);
    return;
  }
  
  // Get world space position and normal
  vec3 viewPos = reconstructViewPos(vUv, depth, u_projectionMatrixInverse);
  vec3 normal = normalize(texture2D(tNormal, vUv).rgb * 2.0 - 1.0);
  
  // If no normal texture, reconstruct from depth (less accurate)
  if (length(normal) < 0.1) {
    normal = reconstructNormal(tDepth, vUv, 1.0 / u_resolution, u_projectionMatrixInverse);
  }
  
  // Calculate view direction
  vec3 viewDir = normalize(viewPos);
  
  // Calculate reflection direction
  vec3 reflectionDir = reflect(viewDir, normal);
  
  // Add jitter to reduce aliasing
  if (u_ssrJitter > 0.0) {
    vec2 jitter = (hash22(vUv + u_time) - 0.5) * u_ssrJitter;
    reflectionDir.xy += jitter;
    reflectionDir = normalize(reflectionDir);
  }
  
  // Perform ray marching
  vec4 rayMarchResult = screenSpaceRayMarch(
    viewPos, 
    reflectionDir, 
    u_ssrMaxDistance, 
    u_ssrSteps, 
    u_ssrBinarySteps
  );
  
  vec3 finalColor = originalColor;
  
  if (rayMarchResult.w > 0.0) {
    vec2 reflectionUV = rayMarchResult.xy;
    vec3 reflectionColor = texture2D(tColor, reflectionUV).rgb;
    
    // Calculate Fresnel effect
    float fresnel = calculateFresnel(-viewDir, normal);
    if (u_ssrFresnel > 0.0) {
      fresnel = mix(1.0, fresnel, u_ssrFresnel);
    }
    
    // Calculate fade factors
    float edgeFade = calculateEdgeFade(reflectionUV, 0.1);
    float distanceFade = calculateDistanceFade(length(reflectionDir), u_ssrMaxDistance);
    
    // Combine fade factors
    float reflectionStrength = fresnel * edgeFade * distanceFade * u_ssrIntensity;
    
    // Blend reflection with original color
    finalColor = mix(originalColor, reflectionColor, reflectionStrength);
  }
  
  gl_FragColor = vec4(finalColor, 1.0);
}
