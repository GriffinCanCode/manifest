/**
 * Depth of Field Fragment Shader
 * Bokeh-based depth of field with focus distance and aperture controls
 */

precision highp float;

uniform sampler2D tColor;
uniform sampler2D tDepth;
uniform vec2 u_resolution;
uniform float u_focusDistance;
uniform float u_focusRange;
uniform float u_bokehRadius;
uniform float u_aperture;
uniform float u_cameraNear;
uniform float u_cameraFar;
uniform bool u_dofEnabled;
uniform int u_bokehSamples;

varying vec2 vUv;

#include ../utils/screen-space.glsl
#include ../utils/sampling.glsl
#include ../modules/common.glsl

// Bokeh sampling pattern (hexagonal for more natural look)
vec2 hexBokehPattern(int index, int samples) {
  float angle = float(index) * TAU / float(samples);
  float radius = sqrt(float(index) / float(samples));
  return vec2(cos(angle), sin(angle)) * radius;
}

// Calculate circle of confusion based on depth
float calculateCoC(float depth, float focusDistance, float focusRange, float aperture) {
  float linearDepth = linearizeDepth(depth, u_cameraNear, u_cameraFar);
  float focusDepth = focusDistance;
  
  // Calculate circle of confusion using thin lens equation
  float coc = abs(linearDepth - focusDepth) / focusRange;
  coc *= aperture;
  
  return clamp(coc, 0.0, 1.0);
}

// Bokeh shape function (can be customized for different shapes)
float bokehWeight(vec2 offset, float radius) {
  float distance = length(offset);
  
  // Smooth circular falloff
  return 1.0 - smoothstep(0.0, radius, distance);
}

void main() {
  if (!u_dofEnabled) {
    gl_FragColor = vec4(texture2D(tColor, vUv).rgb, 1.0);
    return;
  }
  
  vec2 texelSize = 1.0 / u_resolution;
  float centerDepth = texture2D(tDepth, vUv).r;
  vec3 centerColor = texture2D(tColor, vUv).rgb;
  
  // Skip depth of field for skybox
  if (centerDepth >= 0.9999) {
    gl_FragColor = vec4(centerColor, 1.0);
    return;
  }
  
  // Calculate circle of confusion for center pixel
  float centerCoC = calculateCoC(centerDepth, u_focusDistance, u_focusRange, u_aperture);
  
  // If in focus, return original color
  if (centerCoC < 0.01) {
    gl_FragColor = vec4(centerColor, 1.0);
    return;
  }
  
  vec3 blurredColor = vec3(0.0);
  float totalWeight = 0.0;
  float maxRadius = centerCoC * u_bokehRadius;
  
  // Sample in bokeh pattern
  for (int i = 0; i < 64; i++) {
    if (i >= u_bokehSamples) break;
    
    vec2 offset = hexBokehPattern(i, u_bokehSamples) * maxRadius * texelSize;
    vec2 sampleUV = vUv + offset;
    
    // Skip samples outside screen
    if (sampleUV.x < 0.0 || sampleUV.x > 1.0 || 
        sampleUV.y < 0.0 || sampleUV.y > 1.0) {
      continue;
    }
    
    float sampleDepth = texture2D(tDepth, sampleUV).r;
    vec3 sampleColor = texture2D(tColor, sampleUV).rgb;
    
    // Calculate CoC for sample
    float sampleCoC = calculateCoC(sampleDepth, u_focusDistance, u_focusRange, u_aperture);
    
    // Bokeh weight based on distance and CoC
    float weight = bokehWeight(offset / texelSize, maxRadius);
    
    // Reduce contribution of focused areas to blurred regions
    if (sampleCoC < centerCoC * 0.5) {
      weight *= 0.1;
    }
    
    // Weight by sample's own blur amount
    weight *= (sampleCoC + 0.1);
    
    blurredColor += sampleColor * weight;
    totalWeight += weight;
  }
  
  if (totalWeight > 0.0) {
    blurredColor /= totalWeight;
  } else {
    blurredColor = centerColor;
  }
  
  // Blend between original and blurred based on CoC
  vec3 finalColor = mix(centerColor, blurredColor, smoothstep(0.0, 1.0, centerCoC));
  
  gl_FragColor = vec4(finalColor, 1.0);
}
