#ifdef GL_ES
precision highp float;
#endif

varying vec2 vUv;
varying vec3 vRayDirection;

uniform mat4 u_projectionMatrixInverse;
uniform mat4 u_viewMatrixInverse;

void main() {
  vUv = uv;
  
  // Calculate world-space ray direction
  vec4 clipSpace = vec4(uv * 2.0 - 1.0, 1.0, 1.0);
  vec4 viewSpace = u_projectionMatrixInverse * clipSpace;
  viewSpace /= viewSpace.w;
  
  vRayDirection = (u_viewMatrixInverse * vec4(viewSpace.xyz, 0.0)).xyz;
  
  gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
}
