/**
 * Screen Space Ambient Occlusion (SSAO) Fragment Shader
 * Calculates ambient occlusion based on depth buffer
 */

precision highp float;

uniform sampler2D tColor;
uniform sampler2D tDepth;
uniform sampler2D tNormal;
uniform vec2 u_resolution;
uniform mat4 u_projectionMatrix;
uniform mat4 u_projectionMatrixInverse;
uniform float u_cameraNear;
uniform float u_cameraFar;
uniform float u_ssaoRadius;
uniform float u_ssaoIntensity;
uniform float u_ssaoBias;
uniform float u_ssaoFalloff;
uniform int u_ssaoSamples;
uniform bool u_ssaoEnabled;
uniform float u_time;

varying vec2 vUv;

#include ../utils/screen-space.glsl
#include ../utils/sampling.glsl
#include ../modules/common.glsl

// Generate hemisphere samples for SSAO
vec3 getHemisphereSample(int index, vec3 normal) {
  vec2 poissonSample;
  
  if (index < 16) {
    poissonSample = poissonDisk16[index];
  } else if (index < 64) {
    poissonSample = poissonDisk64[index];
  } else {
    // Fallback to random pattern
    poissonSample = hash22(vec2(float(index), u_time));
  }
  
  // Convert to hemisphere sample
  vec3 sample = vec3(poissonSample, sqrt(1.0 - dot(poissonSample, poissonSample)));
  
  // Orient sample towards normal
  vec3 tangent = normalize(cross(normal, vec3(0.0, 1.0, 0.0)));
  if (dot(tangent, tangent) < 0.1) {
    tangent = normalize(cross(normal, vec3(1.0, 0.0, 0.0)));
  }
  vec3 bitangent = cross(normal, tangent);
  
  mat3 TBN = mat3(tangent, bitangent, normal);
  return TBN * sample;
}

void main() {
  if (!u_ssaoEnabled) {
    gl_FragColor = vec4(1.0, 1.0, 1.0, 1.0);
    return;
  }
  
  vec2 texelSize = 1.0 / u_resolution;
  float depth = texture2D(tDepth, vUv).r;
  
  // Skip skybox/far plane
  if (depth >= 0.9999) {
    gl_FragColor = vec4(1.0, 1.0, 1.0, 1.0);
    return;
  }
  
  // Get view space position and normal
  vec3 viewPos = reconstructViewPos(vUv, depth, u_projectionMatrixInverse);
  vec3 normal = normalize(texture2D(tNormal, vUv).rgb * 2.0 - 1.0);
  
  // If no normal texture, reconstruct from depth
  if (length(normal) < 0.1) {
    normal = reconstructNormal(tDepth, vUv, texelSize, u_projectionMatrixInverse);
  }
  
  float occlusion = 0.0;
  float validSamples = 0.0;
  
  // Sample hemisphere around fragment
  for (int i = 0; i < 64; i++) {
    if (i >= u_ssaoSamples) break;
    
    // Get hemisphere sample
    vec3 sampleDir = getHemisphereSample(i, normal);
    vec3 samplePos = viewPos + sampleDir * u_ssaoRadius;
    
    // Project sample to screen space
    vec4 sampleClipPos = u_projectionMatrix * vec4(samplePos, 1.0);
    vec3 sampleNDC = sampleClipPos.xyz / sampleClipPos.w;
    vec2 sampleUV = sampleNDC.xy * 0.5 + 0.5;
    
    // Skip samples outside screen
    if (sampleUV.x < 0.0 || sampleUV.x > 1.0 || 
        sampleUV.y < 0.0 || sampleUV.y > 1.0) {
      continue;
    }
    
    // Sample depth at projected position
    float sampleDepth = texture2D(tDepth, sampleUV).r;
    vec3 sampleViewPos = reconstructViewPos(sampleUV, sampleDepth, u_projectionMatrixInverse);
    
    // Calculate occlusion
    float rangeCheck = smoothstep(0.0, 1.0, u_ssaoRadius / abs(viewPos.z - sampleViewPos.z));
    
    // Check if sample is in front of surface (with bias)
    if (sampleViewPos.z > samplePos.z + u_ssaoBias) {
      occlusion += rangeCheck;
    }
    
    validSamples += 1.0;
  }
  
  if (validSamples > 0.0) {
    occlusion /= validSamples;
  }
  
  // Apply falloff and intensity
  occlusion = pow(occlusion, u_ssaoFalloff);
  occlusion = 1.0 - occlusion * u_ssaoIntensity;
  occlusion = clamp(occlusion, 0.0, 1.0);
  
  gl_FragColor = vec4(vec3(occlusion), 1.0);
}
