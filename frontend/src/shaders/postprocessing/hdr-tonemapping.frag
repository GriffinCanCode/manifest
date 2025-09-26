/**
 * HDR Tone Mapping Fragment Shader
 * Converts HDR colors to LDR with various tone mapping operators
 */

precision highp float;

uniform sampler2D tColor;
uniform vec2 u_resolution;
uniform float u_exposure;
uniform float u_whitePoint;
uniform int u_toneMappingType;
uniform float u_adaptationRate;
uniform float u_minLuminance;
uniform float u_maxLuminance;
uniform bool u_autoExposure;

varying vec2 vUv;

#include ../utils/color.glsl
#include ../utils/screen-space.glsl

void main() {
  vec3 hdrColor = texture2D(tColor, vUv).rgb;
  vec2 texelSize = 1.0 / u_resolution;
  
  float exposure = u_exposure;
  
  // Auto exposure calculation
  if (u_autoExposure) {
    exposure = calculateAutoExposure(tColor, texelSize, u_minLuminance, u_maxLuminance);
    // Smooth adaptation over time
    exposure = mix(u_exposure, exposure, u_adaptationRate);
  }
  
  vec3 toneMappedColor;
  
  // Apply tone mapping operator
  if (u_toneMappingType == 0) {
    // Linear (no tone mapping)
    toneMappedColor = hdrColor * exposure;
  } else if (u_toneMappingType == 1) {
    // Reinhard
    toneMappedColor = reinhardToneMapping(hdrColor, exposure);
  } else if (u_toneMappingType == 2) {
    // Reinhard Extended
    toneMappedColor = reinhardExtendedToneMapping(hdrColor, exposure, u_whitePoint);
  } else if (u_toneMappingType == 3) {
    // ACES
    toneMappedColor = acesToneMapping(hdrColor, exposure);
  } else if (u_toneMappingType == 4) {
    // Uncharted 2
    toneMappedColor = uncharted2ToneMapping(hdrColor, exposure);
  } else {
    // Default to ACES
    toneMappedColor = acesToneMapping(hdrColor, exposure);
  }
  
  // Apply gamma correction
  toneMappedColor = linearTosRGB(toneMappedColor);
  
  // Clamp to prevent any issues
  toneMappedColor = clamp(toneMappedColor, 0.0, 1.0);
  
  gl_FragColor = vec4(toneMappedColor, 1.0);
}
