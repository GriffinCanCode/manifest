/**
 * Sampling Utilities for Postprocessing
 * Various sampling patterns and kernels for effects like blur, SSAO, etc.
 */

#ifndef SAMPLING_GLSL
#define SAMPLING_GLSL

#include ../modules/common.glsl

// Gaussian blur weights for different kernel sizes
vec3 gaussian3x3Weights = vec3(0.25, 0.5, 0.25);
vec4 gaussian4x4Weights = vec4(0.0625, 0.25, 0.375, 0.25);

// Generate Gaussian weights for arbitrary kernel size
float gaussianWeight(float x, float sigma) {
  return exp(-(x * x) / (2.0 * sigma * sigma));
}

// Box blur sampling
vec3 boxBlur(sampler2D texture, vec2 uv, vec2 texelSize, int radius) {
  vec3 color = vec3(0.0);
  float samples = 0.0;
  
  for(int x = -radius; x <= radius; x++) {
    for(int y = -radius; y <= radius; y++) {
      vec2 offset = vec2(float(x), float(y)) * texelSize;
      color += texture2D(texture, uv + offset).rgb;
      samples += 1.0;
    }
  }
  
  return color / samples;
}

// Gaussian blur (separable)
vec3 gaussianBlurH(sampler2D texture, vec2 uv, vec2 texelSize, float sigma, int radius) {
  vec3 color = vec3(0.0);
  float weightSum = 0.0;
  
  for(int i = -radius; i <= radius; i++) {
    float weight = gaussianWeight(float(i), sigma);
    vec2 offset = vec2(float(i), 0.0) * texelSize;
    color += texture2D(texture, uv + offset).rgb * weight;
    weightSum += weight;
  }
  
  return color / weightSum;
}

vec3 gaussianBlurV(sampler2D texture, vec2 uv, vec2 texelSize, float sigma, int radius) {
  vec3 color = vec3(0.0);
  float weightSum = 0.0;
  
  for(int i = -radius; i <= radius; i++) {
    float weight = gaussianWeight(float(i), sigma);
    vec2 offset = vec2(0.0, float(i)) * texelSize;
    color += texture2D(texture, uv + offset).rgb * weight;
    weightSum += weight;
  }
  
  return color / weightSum;
}

// Dual Kawase blur (efficient bloom blur)
vec3 kawaseBlurDown(sampler2D texture, vec2 uv, vec2 texelSize, float offset) {
  vec3 color = vec3(0.0);
  
  // Center sample
  color += texture2D(texture, uv).rgb * 4.0;
  
  // Corner samples
  color += texture2D(texture, uv + vec2(-offset, -offset) * texelSize).rgb;
  color += texture2D(texture, uv + vec2(offset, -offset) * texelSize).rgb;
  color += texture2D(texture, uv + vec2(-offset, offset) * texelSize).rgb;
  color += texture2D(texture, uv + vec2(offset, offset) * texelSize).rgb;
  
  return color / 8.0;
}

vec3 kawaseBlurUp(sampler2D texture, vec2 uv, vec2 texelSize, float offset) {
  vec3 color = vec3(0.0);
  
  // Edge samples
  color += texture2D(texture, uv + vec2(-offset, 0.0) * texelSize).rgb;
  color += texture2D(texture, uv + vec2(offset, 0.0) * texelSize).rgb;
  color += texture2D(texture, uv + vec2(0.0, -offset) * texelSize).rgb;
  color += texture2D(texture, uv + vec2(0.0, offset) * texelSize).rgb;
  
  // Corner samples
  color += texture2D(texture, uv + vec2(-offset, -offset) * texelSize).rgb;
  color += texture2D(texture, uv + vec2(offset, -offset) * texelSize).rgb;
  color += texture2D(texture, uv + vec2(-offset, offset) * texelSize).rgb;
  color += texture2D(texture, uv + vec2(offset, offset) * texelSize).rgb;
  
  return color / 8.0;
}

// Poisson disk samples for SSAO and other screen-space effects
vec2 poissonDisk16[16] = vec2[](
  vec2(-0.94201624, -0.39906216),
  vec2(0.94558609, -0.76890725),
  vec2(-0.094184101, -0.92938870),
  vec2(0.34495938, 0.29387760),
  vec2(-0.91588581, 0.45771432),
  vec2(-0.81544232, -0.87912464),
  vec2(-0.38277543, 0.27676845),
  vec2(0.97484398, 0.75648379),
  vec2(0.44323325, -0.97511554),
  vec2(0.53742981, -0.47373420),
  vec2(-0.26496911, -0.41893023),
  vec2(0.79197514, 0.19090188),
  vec2(-0.24188840, 0.99706507),
  vec2(-0.81409955, 0.91437590),
  vec2(0.19984126, 0.78641367),
  vec2(0.14383161, -0.14100790)
);

vec2 poissonDisk64[64] = vec2[](
  vec2(-0.5119625f, -0.4827938f),
  vec2(-0.2171264f, -0.4768726f),
  vec2(-0.7552931f, -0.2426507f),
  vec2(-0.7136765f, -0.4496614f),
  vec2(-0.5938849f, -0.6895654f),
  vec2(-0.3148003f, -0.7047654f),
  vec2(-0.42215f, -0.2024607f),
  vec2(-0.9466816f, -0.2014508f),
  vec2(-0.8409063f, -0.03465778f),
  vec2(-0.6517572f, -0.07476326f),
  vec2(-0.1041822f, -0.02521214f),
  vec2(-0.3042712f, -0.02195431f),
  vec2(-0.5082307f, 0.1079806f),
  vec2(-0.08429877f, -0.2316298f),
  vec2(-0.9879128f, 0.119041f),
  vec2(-0.3859636f, 0.3363545f),
  vec2(-0.1925334f, 0.1787288f),
  vec2(0.003256182f, 0.138135f),
  vec2(-0.8706837f, 0.3010679f),
  vec2(-0.6982038f, 0.1904326f),
  vec2(0.1975043f, 0.2221317f),
  vec2(0.1507788f, 0.4204168f),
  vec2(0.3514056f, 0.09865579f),
  vec2(0.1558783f, -0.08460935f),
  vec2(-0.0684978f, 0.4461993f),
  vec2(0.3780522f, 0.3478679f),
  vec2(0.3956799f, -0.1469177f),
  vec2(0.5838975f, 0.1054943f),
  vec2(0.6155105f, 0.3245716f),
  vec2(0.3928624f, -0.4417621f),
  vec2(0.1749884f, -0.4202175f),
  vec2(0.6813727f, -0.2424808f),
  vec2(-0.6707711f, 0.4912741f),
  vec2(0.0005130528f, -0.8058334f),
  vec2(0.02703013f, -0.6010728f),
  vec2(-0.1658188f, -0.9695674f),
  vec2(0.4060591f, -0.7100726f),
  vec2(0.7713396f, -0.4713659f),
  vec2(0.573212f, -0.51544f),
  vec2(-0.3448896f, -0.9046497f),
  vec2(0.1268544f, -0.9874692f),
  vec2(0.7418533f, -0.6667366f),
  vec2(0.3492522f, 0.5924662f),
  vec2(0.5679897f, 0.5343465f),
  vec2(0.7986544f, 0.5929391f),
  vec2(-0.1997065f, 0.7724096f),
  vec2(0.8707036f, 0.3755975f),
  vec2(0.8302491f, -0.1312977f),
  vec2(0.97247f, 0.02434084f),
  vec2(0.06393322f, 0.6088467f),
  vec2(-0.1725579f, 0.5258549f),
  vec2(-0.4010898f, 0.6547014f),
  vec2(-0.2141643f, 0.9097462f),
  vec2(0.3682216f, 0.92429f),
  vec2(-0.4353976f, 0.8713849f),
  vec2(-0.6403824f, 0.6064505f),
  vec2(0.0294119f, 0.8816488f),
  vec2(-0.04186097f, -0.2725772f),
  vec2(-0.7699972f, 0.6667717f),
  vec2(0.8428077f, 0.07767493f),
  vec2(0.6218669f, -0.7852815f),
  vec2(0.8446086f, -0.4628626f),
  vec2(-0.7572713f, 0.84737f),
  vec2(0.2923725f, 0.8640115f)
);

// Halton sequence for temporal sampling
vec2 haltonSequence(int index, int base1, int base2) {
  float result1 = 0.0;
  float result2 = 0.0;
  float f1 = 1.0;
  float f2 = 1.0;
  int i1 = index;
  int i2 = index;
  
  // Base 1
  for(int j = 0; j < 10; j++) {
    if(i1 <= 0) break;
    f1 /= float(base1);
    // Manual modulus for GLSL ES 1.00 compatibility: a % b = a - (a / b) * b
    int mod_result1 = i1 - (i1 / base1) * base1;
    result1 += f1 * float(mod_result1);
    i1 /= base1;
  }
  
  // Base 2
  for(int j = 0; j < 10; j++) {
    if(i2 <= 0) break;
    f2 /= float(base2);
    // Manual modulus for GLSL ES 1.00 compatibility: a % b = a - (a / b) * b
    int mod_result2 = i2 - (i2 / base2) * base2;
    result2 += f2 * float(mod_result2);
    i2 /= base2;
  }
  
  return vec2(result1, result2);
}

// Blue noise sampling (simplified pattern)
vec2 blueNoise(vec2 uv, float time) {
  return fract(sin(dot(uv + time, vec2(12.9898, 78.233))) * vec2(43758.5453, 28001.8384));
}

// Mitchell-Netravali filtering
float mitchellNetravali(float x, float B, float C) {
  float ax = abs(x);
  
  if(ax < 1.0) {
    return ((12.0 - 9.0 * B - 6.0 * C) * ax * ax * ax + 
            (-18.0 + 12.0 * B + 6.0 * C) * ax * ax + 
            (6.0 - 2.0 * B)) / 6.0;
  } else if(ax < 2.0) {
    return ((-B - 6.0 * C) * ax * ax * ax + 
            (6.0 * B + 30.0 * C) * ax * ax + 
            (-12.0 * B - 48.0 * C) * ax + 
            (8.0 * B + 24.0 * C)) / 6.0;
  } else {
    return 0.0;
  }
}

// Bicubic texture sampling
vec3 bicubicSample(sampler2D texture, vec2 uv, vec2 texelSize) {
  vec2 center = uv - texelSize * 0.5;
  vec2 f = fract(center / texelSize);
  center = floor(center / texelSize) * texelSize;
  
  vec3 result = vec3(0.0);
  
  for(int x = -1; x <= 2; x++) {
    for(int y = -1; y <= 2; y++) {
      vec2 offset = vec2(float(x), float(y)) * texelSize;
      float wx = mitchellNetravali(f.x - float(x), 1.0/3.0, 1.0/3.0);
      float wy = mitchellNetravali(f.y - float(y), 1.0/3.0, 1.0/3.0);
      result += texture2D(texture, center + offset).rgb * wx * wy;
    }
  }
  
  return result;
}

// FXAA edge detection
float fxaaLuma(vec3 color) {
  return color.y * (0.587/0.299) + color.x; // Simplified luma
}

vec3 fxaaSample(sampler2D texture, vec2 uv, vec2 texelSize) {
  vec3 rgbNW = texture2D(texture, uv + vec2(-1.0, -1.0) * texelSize).rgb;
  vec3 rgbNE = texture2D(texture, uv + vec2(1.0, -1.0) * texelSize).rgb;
  vec3 rgbSW = texture2D(texture, uv + vec2(-1.0, 1.0) * texelSize).rgb;
  vec3 rgbSE = texture2D(texture, uv + vec2(1.0, 1.0) * texelSize).rgb;
  vec3 rgbM = texture2D(texture, uv).rgb;
  
  float lumaNW = fxaaLuma(rgbNW);
  float lumaNE = fxaaLuma(rgbNE);
  float lumaSW = fxaaLuma(rgbSW);
  float lumaSE = fxaaLuma(rgbSE);
  float lumaM = fxaaLuma(rgbM);
  
  float lumaMin = min(lumaM, min(min(lumaNW, lumaNE), min(lumaSW, lumaSE)));
  float lumaMax = max(lumaM, max(max(lumaNW, lumaNE), max(lumaSW, lumaSE)));
  
  vec2 dir;
  dir.x = -((lumaNW + lumaNE) - (lumaSW + lumaSE));
  dir.y = ((lumaNW + lumaSW) - (lumaNE + lumaSE));
  
  float dirReduce = max((lumaNW + lumaNE + lumaSW + lumaSE) * (0.25 * 0.125), 0.0078125);
  float rcpDirMin = 1.0 / (min(abs(dir.x), abs(dir.y)) + dirReduce);
  
  dir = min(vec2(8.0), max(vec2(-8.0), dir * rcpDirMin)) * texelSize;
  
  vec3 rgbA = 0.5 * (
    texture2D(texture, uv + dir * (1.0/3.0 - 0.5)).rgb +
    texture2D(texture, uv + dir * (2.0/3.0 - 0.5)).rgb);
  
  vec3 rgbB = rgbA * 0.5 + 0.25 * (
    texture2D(texture, uv + dir * (0.0/3.0 - 0.5)).rgb +
    texture2D(texture, uv + dir * (3.0/3.0 - 0.5)).rgb);
  
  float lumaB = fxaaLuma(rgbB);
  
  if((lumaB < lumaMin) || (lumaB > lumaMax)) {
    return rgbA;
  } else {
    return rgbB;
  }
}

#endif // SAMPLING_GLSL
