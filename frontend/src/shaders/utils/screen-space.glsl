/**
 * Screen Space Utilities
 * Common functions for postprocessing and screen-space effects
 */

#ifndef SCREEN_SPACE_GLSL
#define SCREEN_SPACE_GLSL

// Standard screen-space vertex shader for fullscreen quads
attribute vec3 position;
attribute vec2 uv;

varying vec2 vUv;

// Screen-space vertex transformation (for postprocessing passes)
void screenSpaceVertex() {
  vUv = uv;
  gl_Position = vec4(position, 1.0);
}

// Convert screen UV to normalized device coordinates
vec2 screenToNDC(vec2 screenUV) {
  return screenUV * 2.0 - 1.0;
}

// Convert normalized device coordinates to screen UV
vec2 ndcToScreen(vec2 ndc) {
  return ndc * 0.5 + 0.5;
}

// Calculate screen-space derivatives for edge detection
vec2 getScreenDerivatives(sampler2D depthTexture, vec2 uv, vec2 texelSize) {
  float depthC = texture2D(depthTexture, uv).r;
  float depthL = texture2D(depthTexture, uv - vec2(texelSize.x, 0.0)).r;
  float depthR = texture2D(depthTexture, uv + vec2(texelSize.x, 0.0)).r;
  float depthU = texture2D(depthTexture, uv - vec2(0.0, texelSize.y)).r;
  float depthD = texture2D(depthTexture, uv + vec2(0.0, texelSize.y)).r;
  
  float ddx = depthR - depthL;
  float ddy = depthD - depthU;
  
  return vec2(ddx, ddy);
}

// Linear depth conversion utilities
float linearizeDepth(float depth, float near, float far) {
  float z = depth * 2.0 - 1.0; // Convert to NDC
  return (2.0 * near * far) / (far + near - z * (far - near));
}

// View position reconstruction from depth
vec3 reconstructViewPos(vec2 uv, float depth, mat4 projMatrixInv) {
  vec4 ndc = vec4(uv * 2.0 - 1.0, depth * 2.0 - 1.0, 1.0);
  vec4 viewPos = projMatrixInv * ndc;
  return viewPos.xyz / viewPos.w;
}

// World position reconstruction from depth
vec3 reconstructWorldPos(vec2 uv, float depth, mat4 projMatrixInv, mat4 viewMatrixInv) {
  vec3 viewPos = reconstructViewPos(uv, depth, projMatrixInv);
  vec4 worldPos = viewMatrixInv * vec4(viewPos, 1.0);
  return worldPos.xyz;
}

// Screen-space normal reconstruction from depth
vec3 reconstructNormal(sampler2D depthTexture, vec2 uv, vec2 texelSize, mat4 projMatrixInv) {
  // Sample neighboring depths
  float depth = texture2D(depthTexture, uv).r;
  float depthR = texture2D(depthTexture, uv + vec2(texelSize.x, 0.0)).r;
  float depthU = texture2D(depthTexture, uv + vec2(0.0, texelSize.y)).r;
  
  // Reconstruct positions
  vec3 pos = reconstructViewPos(uv, depth, projMatrixInv);
  vec3 posR = reconstructViewPos(uv + vec2(texelSize.x, 0.0), depthR, projMatrixInv);
  vec3 posU = reconstructViewPos(uv + vec2(0.0, texelSize.y), depthU, projMatrixInv);
  
  // Calculate normal from cross product
  vec3 dx = posR - pos;
  vec3 dy = posU - pos;
  
  return normalize(cross(dx, dy));
}

// Bilateral filtering weight calculation
float bilateralWeight(float depth1, float depth2, float sigma) {
  float diff = abs(depth1 - depth2);
  return exp(-diff * diff / (2.0 * sigma * sigma));
}

// Screen-space ray direction calculation
vec3 getScreenRayDirection(vec2 uv, mat4 projMatrixInv, mat4 viewMatrixInv) {
  vec4 rayEnd = projMatrixInv * vec4(uv * 2.0 - 1.0, 1.0, 1.0);
  rayEnd.xyz /= rayEnd.w;
  
  vec4 worldRayEnd = viewMatrixInv * rayEnd;
  vec4 worldCameraPos = viewMatrixInv * vec4(0.0, 0.0, 0.0, 1.0);
  
  return normalize(worldRayEnd.xyz - worldCameraPos.xyz);
}

// Pack/unpack normal to RG texture
vec2 packNormal(vec3 normal) {
  return normal.xy * 0.5 + 0.5;
}

vec3 unpackNormal(vec2 packed) {
  vec2 xy = packed * 2.0 - 1.0;
  float z = sqrt(1.0 - dot(xy, xy));
  return vec3(xy, z);
}

// Pack/unpack depth and stencil
vec4 packDepthStencil(float depth, float stencil) {
  return vec4(depth, stencil, 0.0, 1.0);
}

vec2 unpackDepthStencil(vec4 packed) {
  return vec2(packed.x, packed.y);
}

// Screen-space temporal reprojection
vec2 reprojectScreen(vec3 worldPos, mat4 prevViewProjMatrix) {
  vec4 prevClipPos = prevViewProjMatrix * vec4(worldPos, 1.0);
  vec3 prevNDC = prevClipPos.xyz / prevClipPos.w;
  return prevNDC.xy * 0.5 + 0.5;
}

// Velocity calculation for temporal effects
vec2 calculateVelocity(vec3 worldPos, mat4 currentViewProjMatrix, mat4 prevViewProjMatrix) {
  vec4 currentClipPos = currentViewProjMatrix * vec4(worldPos, 1.0);
  vec4 prevClipPos = prevViewProjMatrix * vec4(worldPos, 1.0);
  
  vec2 currentScreen = currentClipPos.xy / currentClipPos.w;
  vec2 prevScreen = prevClipPos.xy / prevClipPos.w;
  
  return currentScreen - prevScreen;
}

#endif // SCREEN_SPACE_GLSL
