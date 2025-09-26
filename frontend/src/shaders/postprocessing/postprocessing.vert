/**
 * Postprocessing Vertex Shader
 * Shared vertex shader for all fullscreen postprocessing passes
 */

attribute vec3 position;
attribute vec2 uv;

varying vec2 vUv;
varying vec2 vUv0; // Center
varying vec2 vUv1; // Top-left
varying vec2 vUv2; // Top-right  
varying vec2 vUv3; // Bottom-left
varying vec2 vUv4; // Bottom-right

// For some effects that need neighboring UVs
uniform vec2 u_resolution;
uniform bool u_useOffsetUVs; // Enable offset UVs for edge detection, etc.

void main() {
  vUv = uv;
  vUv0 = uv;
  
  // Calculate offset UVs for edge detection and sampling
  if (u_useOffsetUVs) {
    vec2 texelSize = 1.0 / u_resolution;
    
    vUv1 = uv + vec2(-texelSize.x, -texelSize.y); // Top-left
    vUv2 = uv + vec2( texelSize.x, -texelSize.y); // Top-right
    vUv3 = uv + vec2(-texelSize.x,  texelSize.y); // Bottom-left
    vUv4 = uv + vec2( texelSize.x,  texelSize.y); // Bottom-right
  } else {
    vUv1 = vUv2 = vUv3 = vUv4 = uv;
  }
  
  // Standard fullscreen quad positioning
  gl_Position = vec4(position, 1.0);
}
