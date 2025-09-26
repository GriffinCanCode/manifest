/**
 * Color Utilities for Postprocessing
 * HDR tone mapping, color grading, and conversion functions
 */

#ifndef COLOR_GLSL
#define COLOR_GLSL

#include ../modules/common.glsl

// Luminance calculation constants (Rec. 709)
const vec3 LUMA_WEIGHTS = vec3(0.2126, 0.7152, 0.0722);
const vec3 LUMA_WEIGHTS_BT601 = vec3(0.299, 0.587, 0.114);

// HDR tone mapping operators
vec3 reinhardToneMapping(vec3 color, float exposure) {
  vec3 exposedColor = color * exposure;
  return exposedColor / (exposedColor + vec3(1.0));
}

vec3 reinhardExtendedToneMapping(vec3 color, float exposure, float whitePoint) {
  vec3 exposedColor = color * exposure;
  vec3 numerator = exposedColor * (1.0 + (exposedColor / (whitePoint * whitePoint)));
  return numerator / (1.0 + exposedColor);
}

vec3 acesToneMapping(vec3 color, float exposure) {
  color *= exposure;
  
  const float A = 2.51;
  const float B = 0.03;
  const float C = 2.43;
  const float D = 0.59;
  const float E = 0.14;
  
  return clamp((color * (A * color + B)) / (color * (C * color + D) + E), 0.0, 1.0);
}

vec3 uncharted2ToneMapping(vec3 color, float exposure) {
  const float A = 0.15;
  const float B = 0.50;
  const float C = 0.10;
  const float D = 0.20;
  const float E = 0.02;
  const float F = 0.30;
  const float W = 11.2;
  
  vec3 exposureColor = color * exposure;
  vec3 curr = ((exposureColor * (A * exposureColor + C * B) + D * E) / 
               (exposureColor * (A * exposureColor + B) + D * F)) - E / F;
  
  vec3 whiteScale = 1.0 / (((W * (A * W + C * B) + D * E) / 
                           (W * (A * W + B) + D * F)) - E / F);
  
  return curr * whiteScale;
}

// Luminance calculations
float getLuminance(vec3 color) {
  return dot(color, LUMA_WEIGHTS);
}

float getRelativeLuminance(vec3 color) {
  return dot(color, LUMA_WEIGHTS_BT601);
}

// Linear to sRGB gamma correction
vec3 linearTosRGB(vec3 linearColor) {
  vec3 lower = linearColor * 12.92;
  vec3 higher = 1.055 * pow(linearColor, vec3(1.0 / 2.4)) - 0.055;
  
  return mix(lower, higher, step(vec3(0.0031308), linearColor));
}

// sRGB to linear conversion
vec3 sRGBToLinear(vec3 srgbColor) {
  vec3 lower = srgbColor / 12.92;
  vec3 higher = pow((srgbColor + 0.055) / 1.055, vec3(2.4));
  
  return mix(lower, higher, step(vec3(0.04045), srgbColor));
}

// Color temperature adjustment (simplified)
vec3 adjustTemperature(vec3 color, float temperature) {
  // Temperature in Kelvin (1000-40000)
  temperature = clamp(temperature, 1000.0, 40000.0) / 100.0;
  
  vec3 colorTemp = vec3(1.0);
  
  // Red component
  if (temperature <= 66.0) {
    colorTemp.r = 1.0;
  } else {
    colorTemp.r = clamp(1.29293618606 * pow(temperature - 60.0, -0.1332047592), 0.0, 1.0);
  }
  
  // Green component
  if (temperature <= 66.0) {
    colorTemp.g = clamp(0.39008157444 * log(temperature) - 0.63184144378, 0.0, 1.0);
  } else {
    colorTemp.g = clamp(1.29293618606 * pow(temperature - 60.0, -0.0755148492), 0.0, 1.0);
  }
  
  // Blue component
  if (temperature >= 66.0) {
    colorTemp.b = 1.0;
  } else if (temperature <= 19.0) {
    colorTemp.b = 0.0;
  } else {
    colorTemp.b = clamp(0.54320678911 * log(temperature - 10.0) - 1.19625408914, 0.0, 1.0);
  }
  
  return color * colorTemp;
}

// Saturation adjustment
vec3 adjustSaturation(vec3 color, float saturation) {
  float luminance = getLuminance(color);
  return mix(vec3(luminance), color, saturation);
}

// Contrast adjustment
vec3 adjustContrast(vec3 color, float contrast) {
  return (color - 0.5) * contrast + 0.5;
}

// Brightness adjustment
vec3 adjustBrightness(vec3 color, float brightness) {
  return color + brightness;
}

// Vibrance adjustment (saturation that preserves skin tones)
vec3 adjustVibrance(vec3 color, float vibrance) {
  float luminance = getLuminance(color);
  float sat = length(color - vec3(luminance));
  float mask = 1.0 - pow(sat, vibrance);
  
  return mix(vec3(luminance), color, 1.0 + vibrance * mask);
}

// Color grading with lift/gamma/gain
vec3 colorGrade(vec3 color, vec3 lift, vec3 gamma, vec3 gain) {
  // Apply lift (shadows)
  color = color + lift;
  
  // Apply gamma (midtones) - avoid division by zero
  vec3 safeGamma = max(gamma, vec3(0.001));
  color = pow(max(color, vec3(0.0)), 1.0 / safeGamma);
  
  // Apply gain (highlights)
  color = color * gain;
  
  return color;
}

// ACES color space conversion (simplified)
vec3 linearToACES(vec3 linearColor) {
  mat3 linearToACESMatrix = mat3(
    0.6131, 0.0701, 0.0206,
    0.3395, 0.9164, 0.1096,
    0.0474, 0.0135, 0.8698
  );
  
  return linearToACESMatrix * linearColor;
}

vec3 acesToLinear(vec3 acesColor) {
  mat3 acesToLinearMatrix = mat3(
    1.7049, -0.1297, -0.0240,
    -0.6217, 1.1409, -0.1289,
    -0.0832, -0.0112, 1.1529
  );
  
  return acesToLinearMatrix * acesColor;
}

// Hue shift
vec3 adjustHue(vec3 color, float hueShift) {
  vec3 hsvColor = rgb2hsv(color);
  hsvColor.x = fract(hsvColor.x + hueShift / 360.0);
  return hsv2rgb(hsvColor);
}

// Color balance (shadows/midtones/highlights)
vec3 colorBalance(vec3 color, vec3 shadows, vec3 midtones, vec3 highlights) {
  float luminance = getLuminance(color);
  
  // Create masks for each tonal range
  float shadowMask = 1.0 - smoothstep(0.0, 0.5, luminance);
  float highlightMask = smoothstep(0.5, 1.0, luminance);
  float midtoneMask = 1.0 - shadowMask - highlightMask;
  
  // Apply color balance
  vec3 result = color;
  result += shadows * shadowMask;
  result += midtones * midtoneMask;
  result += highlights * highlightMask;
  
  return result;
}

// Channel mixer for creative color effects
vec3 channelMix(vec3 color, mat3 mixMatrix) {
  return mixMatrix * color;
}

// Auto exposure calculation
float calculateAutoExposure(sampler2D colorTexture, vec2 texelSize, float minLuminance, float maxLuminance) {
  float avgLuminance = 0.0;
  int samples = 0;
  
  // Sample luminance across the image (simplified grid sampling)
  for(int x = 0; x < 8; x++) {
    for(int y = 0; y < 8; y++) {
      vec2 uv = vec2(float(x) / 7.0, float(y) / 7.0);
      vec3 color = texture2D(colorTexture, uv).rgb;
      float luminance = getLuminance(color);
      
      if(luminance > 0.001) { // Avoid zero luminance
        avgLuminance += log(luminance);
        samples++;
      }
    }
  }
  
  if(samples > 0) {
    avgLuminance = exp(avgLuminance / float(samples));
    avgLuminance = clamp(avgLuminance, minLuminance, maxLuminance);
    
    // Key value for average scene luminance mapping to middle gray
    float key = 0.18;
    return key / avgLuminance;
  }
  
  return 1.0;
}

// Bloom threshold with soft knee
vec3 bloomThreshold(vec3 color, float threshold, float softKnee) {
  float brightness = getLuminance(color);
  float softness = clamp(brightness - threshold + softKnee, 0.0, 2.0 * softKnee);
  softness = (softness * softness) / (4.0 * softKnee + 0.00001);
  float multiplier = max(brightness - threshold, softness) / max(brightness, 0.00001);
  return color * multiplier;
}

#endif // COLOR_GLSL
