/**
 * Motion Blur Fragment Shader
 * Per-pixel motion blur using velocity buffers
 */

precision highp float;

uniform sampler2D tColor;
uniform sampler2D tVelocity;
uniform sampler2D tDepth;
uniform vec2 u_resolution;
uniform float u_motionBlurIntensity;
uniform int u_motionBlurSamples;
uniform float u_motionBlurMaxRadius;
uniform bool u_motionBlurEnabled;

varying vec2 vUv;

#include ../utils/screen-space.glsl
#include ../modules/common.glsl

// Soft depth comparison for better object separation
float depthWeight(float centerDepth, float sampleDepth, float threshold) {
  float diff = abs(centerDepth - sampleDepth);
  return exp(-diff / threshold);
}

void main() {
  if (!u_motionBlurEnabled || u_motionBlurIntensity <= 0.0) {
    gl_FragColor = vec4(texture2D(tColor, vUv).rgb, 1.0);
    return;
  }
  
  vec2 texelSize = 1.0 / u_resolution;
  vec3 centerColor = texture2D(tColor, vUv).rgb;
  vec2 velocity = texture2D(tVelocity, vUv).xy;
  float centerDepth = texture2D(tDepth, vUv).r;
  
  // Scale velocity by intensity and limit maximum radius
  velocity *= u_motionBlurIntensity;
  float velocityLength = length(velocity);
  
  if (velocityLength < 0.001) {
    // No motion, return original color
    gl_FragColor = vec4(centerColor, 1.0);
    return;
  }
  
  // Clamp velocity to maximum radius
  if (velocityLength > u_motionBlurMaxRadius) {
    velocity = normalize(velocity) * u_motionBlurMaxRadius;
  }
  
  vec3 blurredColor = centerColor;
  float totalWeight = 1.0;
  
  // Sample along motion vector
  int samples = min(u_motionBlurSamples, 32); // Clamp for performance
  
  for (int i = 1; i < 32; i++) {
    if (i >= samples) break;
    
    float t = float(i) / float(samples - 1);
    
    // Sample in both directions along motion vector
    vec2 sampleUV1 = vUv + velocity * t;
    vec2 sampleUV2 = vUv - velocity * t;
    
    // Sample forward direction
    if (sampleUV1.x >= 0.0 && sampleUV1.x <= 1.0 && 
        sampleUV1.y >= 0.0 && sampleUV1.y <= 1.0) {
      
      vec3 sampleColor1 = texture2D(tColor, sampleUV1).rgb;
      float sampleDepth1 = texture2D(tDepth, sampleUV1).r;
      
      // Weight by depth similarity to avoid bleeding
      float weight1 = depthWeight(centerDepth, sampleDepth1, 0.1);
      
      // Weight by distance (closer samples have more influence)
      weight1 *= (1.0 - t);
      
      blurredColor += sampleColor1 * weight1;
      totalWeight += weight1;
    }
    
    // Sample backward direction
    if (sampleUV2.x >= 0.0 && sampleUV2.x <= 1.0 && 
        sampleUV2.y >= 0.0 && sampleUV2.y <= 1.0) {
      
      vec3 sampleColor2 = texture2D(tColor, sampleUV2).rgb;
      float sampleDepth2 = texture2D(tDepth, sampleUV2).r;
      
      // Weight by depth similarity
      float weight2 = depthWeight(centerDepth, sampleDepth2, 0.1);
      
      // Weight by distance
      weight2 *= (1.0 - t);
      
      blurredColor += sampleColor2 * weight2;
      totalWeight += weight2;
    }
  }
  
  if (totalWeight > 0.0) {
    blurredColor /= totalWeight;
  }
  
  gl_FragColor = vec4(blurredColor, 1.0);
}
