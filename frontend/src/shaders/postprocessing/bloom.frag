/**
 * Bloom Effect Fragment Shader
 * Multi-pass bloom with threshold, blur, and compositing
 */

precision highp float;

uniform sampler2D tColor;
uniform sampler2D tBloom;
uniform vec2 u_resolution;
uniform float u_bloomThreshold;
uniform float u_bloomSoftKnee;
uniform float u_bloomIntensity;
uniform float u_bloomRadius;
uniform bool u_bloomEnabled;
uniform int u_passType; // 0: threshold, 1: blur_h, 2: blur_v, 3: composite

varying vec2 vUv;

#include ../utils/color.glsl
#include ../utils/sampling.glsl

void main() {
  vec2 texelSize = 1.0 / u_resolution;
  
  if (u_passType == 0) {
    // Threshold pass - extract bright regions
    vec3 color = texture2D(tColor, vUv).rgb;
    
    if (u_bloomEnabled) {
      vec3 bloomColor = bloomThreshold(color, u_bloomThreshold, u_bloomSoftKnee);
      gl_FragColor = vec4(bloomColor, 1.0);
    } else {
      gl_FragColor = vec4(0.0, 0.0, 0.0, 1.0);
    }
    
  } else if (u_passType == 1) {
    // Horizontal blur pass using Kawase blur
    if (u_bloomEnabled) {
      vec3 blurredColor = kawaseBlurDown(tColor, vUv, texelSize, u_bloomRadius);
      gl_FragColor = vec4(blurredColor, 1.0);
    } else {
      gl_FragColor = vec4(0.0, 0.0, 0.0, 1.0);
    }
    
  } else if (u_passType == 2) {
    // Vertical blur pass using Kawase blur
    if (u_bloomEnabled) {
      vec3 blurredColor = kawaseBlurUp(tColor, vUv, texelSize, u_bloomRadius);
      gl_FragColor = vec4(blurredColor, 1.0);
    } else {
      gl_FragColor = vec4(0.0, 0.0, 0.0, 1.0);
    }
    
  } else if (u_passType == 3) {
    // Composite pass - blend bloom with original
    vec3 originalColor = texture2D(tColor, vUv).rgb;
    
    if (u_bloomEnabled) {
      vec3 bloomColor = texture2D(tBloom, vUv).rgb;
      
      // Additive blending with intensity control
      vec3 finalColor = originalColor + bloomColor * u_bloomIntensity;
      gl_FragColor = vec4(finalColor, 1.0);
    } else {
      gl_FragColor = vec4(originalColor, 1.0);
    }
    
  } else {
    // Fallback - pass through original color
    vec3 originalColor = texture2D(tColor, vUv).rgb;
    gl_FragColor = vec4(originalColor, 1.0);
  }
}
