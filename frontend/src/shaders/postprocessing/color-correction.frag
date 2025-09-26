/**
 * Color Correction and Grading Fragment Shader
 * Comprehensive color adjustment tools for final image enhancement
 */

precision highp float;

uniform sampler2D tColor;
uniform vec2 u_resolution;

// Basic adjustments
uniform float u_brightness;
uniform float u_contrast;
uniform float u_saturation;
uniform float u_vibrance;
uniform float u_hueShift;
uniform float u_temperature;
uniform float u_tint;

// Lift/Gamma/Gain color grading
uniform vec3 u_lift;
uniform vec3 u_gamma;
uniform vec3 u_gain;

// Shadow/Midtone/Highlight color balance
uniform vec3 u_shadows;
uniform vec3 u_midtones;
uniform vec3 u_highlights;

// Channel mixer
uniform mat3 u_channelMix;

// Vignette
uniform float u_vignetteIntensity;
uniform float u_vignetteSmoothness;
uniform float u_vignetteRoundness;

// Film grain
uniform float u_grainAmount;
uniform float u_grainSize;
uniform float u_time;

// Enable flags
uniform bool u_colorCorrectionEnabled;
uniform bool u_vignetteEnabled;
uniform bool u_grainEnabled;

varying vec2 vUv;

#include ../utils/color.glsl
#include ../modules/common.glsl

// Film grain generation
float filmGrain(vec2 uv, float time, float amount, float size) {
  vec2 grainUV = uv * size;
  grainUV += time * 0.5;
  
  float grain = random(grainUV);
  grain = grain * 2.0 - 1.0;
  
  return grain * amount;
}

// Vignette calculation
float calculateVignette(vec2 uv, float intensity, float smoothness, float roundness) {
  vec2 center = uv - 0.5;
  
  // Apply roundness
  center.x *= mix(1.0, u_resolution.x / u_resolution.y, roundness);
  
  float distance = length(center);
  float vignette = smoothstep(0.0, smoothness, 1.0 - distance * intensity);
  
  return vignette;
}

void main() {
  vec3 color = texture2D(tColor, vUv).rgb;
  
  if (!u_colorCorrectionEnabled) {
    gl_FragColor = vec4(color, 1.0);
    return;
  }
  
  // Basic adjustments
  color = adjustBrightness(color, u_brightness);
  color = adjustContrast(color, u_contrast);
  color = adjustSaturation(color, u_saturation);
  color = adjustVibrance(color, u_vibrance);
  
  // Hue shift
  color = adjustHue(color, u_hueShift);
  
  // Temperature and tint
  color = adjustTemperature(color, u_temperature);
  // Simple tint implementation
  color.rb = mix(color.rb, color.rb * vec2(1.0 + u_tint * 0.1, 1.0 - u_tint * 0.1), abs(u_tint));
  
  // Lift/Gamma/Gain color grading
  color = colorGrade(color, u_lift, u_gamma, u_gain);
  
  // Shadow/Midtone/Highlight color balance
  color = colorBalance(color, u_shadows, u_midtones, u_highlights);
  
  // Channel mixer
  if (length(u_channelMix[0]) > 0.1 || length(u_channelMix[1]) > 0.1 || length(u_channelMix[2]) > 0.1) {
    color = channelMix(color, u_channelMix);
  }
  
  // Vignette
  if (u_vignetteEnabled && u_vignetteIntensity > 0.0) {
    float vignette = calculateVignette(vUv, u_vignetteIntensity, u_vignetteSmoothness, u_vignetteRoundness);
    color *= vignette;
  }
  
  // Film grain
  if (u_grainEnabled && u_grainAmount > 0.0) {
    float grain = filmGrain(vUv, u_time, u_grainAmount, u_grainSize);
    color = mix(color, color + grain, u_grainAmount);
  }
  
  // Ensure colors stay in valid range
  color = clamp(color, 0.0, 1.0);
  
  gl_FragColor = vec4(color, 1.0);
}
