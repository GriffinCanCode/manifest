/**
 * Temporal Anti-Aliasing (TAA) Fragment Shader
 * Accumulates samples over time for high-quality anti-aliasing
 */

precision highp float;

uniform sampler2D tColor;        // Current frame
uniform sampler2D tHistory;      // Previous frame
uniform sampler2D tDepth;        // Current depth
uniform sampler2D tVelocity;     // Motion vectors
uniform vec2 u_resolution;
uniform mat4 u_viewProjectionMatrix;
uniform mat4 u_prevViewProjectionMatrix;
uniform float u_taaBlendFactor;  // 0.05-0.1 typical
uniform float u_taaClampFactor;  // Clamp strength for ghosting reduction
uniform bool u_taaEnabled;
uniform int u_frameCount;
uniform float u_time;

varying vec2 vUv;

#include ../utils/screen-space.glsl
#include ../utils/color.glsl
#include ../modules/common.glsl

// Calculate motion vector from depth and matrices
vec2 calculateMotionVector(vec2 uv, float depth) {
  // Reconstruct world position
  vec3 worldPos = reconstructWorldPos(uv, depth, 
    inverse(u_viewProjectionMatrix), inverse(mat4(1.0))); // Simplified
  
  // Project to current and previous screen space
  vec4 currentClip = u_viewProjectionMatrix * vec4(worldPos, 1.0);
  vec4 prevClip = u_prevViewProjectionMatrix * vec4(worldPos, 1.0);
  
  vec2 currentScreen = currentClip.xy / currentClip.w;
  vec2 prevScreen = prevClip.xy / prevClip.w;
  
  return (currentScreen - prevScreen) * 0.5;
}

// Color space conversion for better blending
vec3 rgb2ycocg(vec3 rgb) {
  float Y = dot(rgb, vec3(0.25, 0.5, 0.25));
  float Co = dot(rgb, vec3(0.5, 0.0, -0.5));
  float Cg = dot(rgb, vec3(-0.25, 0.5, -0.25));
  return vec3(Y, Co, Cg);
}

vec3 ycocg2rgb(vec3 ycocg) {
  float Y = ycocg.x;
  float Co = ycocg.y;
  float Cg = ycocg.z;
  return vec3(Y + Co - Cg, Y + Cg, Y - Co - Cg);
}

// Catmull-Rom filtering for better temporal sampling
vec3 sampleCatmullRom(sampler2D tex, vec2 uv, vec2 texelSize) {
  vec2 center = uv - texelSize * 0.5;
  vec2 f = fract(center / texelSize);
  center = floor(center / texelSize) * texelSize + texelSize * 0.5;
  
  vec3 samples[16];
  for(int i = 0; i < 4; i++) {
    for(int j = 0; j < 4; j++) {
      vec2 offset = vec2(float(i-1), float(j-1)) * texelSize;
      samples[i*4 + j] = texture2D(tex, center + offset).rgb;
    }
  }
  
  // Catmull-Rom weights
  vec4 wx = vec4(-0.5, 1.5, -1.5, 0.5) * f.x * f.x * f.x +
            vec4(1.0, -2.5, 2.0, -0.5) * f.x * f.x +
            vec4(-0.5, 0.0, 0.5, 0.0) * f.x +
            vec4(0.0, 1.0, 0.0, 0.0);
            
  vec4 wy = vec4(-0.5, 1.5, -1.5, 0.5) * f.y * f.y * f.y +
            vec4(1.0, -2.5, 2.0, -0.5) * f.y * f.y +
            vec4(-0.5, 0.0, 0.5, 0.0) * f.y +
            vec4(0.0, 1.0, 0.0, 0.0);
  
  vec3 result = vec3(0.0);
  for(int i = 0; i < 4; i++) {
    for(int j = 0; j < 4; j++) {
      result += samples[i*4 + j] * wx[i] * wy[j];
    }
  }
  
  return result;
}

// Variance clipping to reduce ghosting
vec3 clipToAABB(vec3 color, vec3 minimum, vec3 maximum, vec3 average) {
  vec3 center = 0.5 * (maximum + minimum);
  vec3 extent = 0.5 * (maximum - minimum);
  
  // Move color relative to center
  vec3 offset = color - center;
  
  // Clip to AABB
  vec3 ts = abs(offset) / max(extent, vec3(0.0001));
  float t = max(max(ts.x, ts.y), ts.z);
  
  if (t > 1.0) {
    return center + offset / t;
  }
  
  return color;
}

void main() {
  if (!u_taaEnabled) {
    gl_FragColor = vec4(texture2D(tColor, vUv).rgb, 1.0);
    return;
  }
  
  vec2 texelSize = 1.0 / u_resolution;
  vec3 currentColor = texture2D(tColor, vUv).rgb;
  float depth = texture2D(tDepth, vUv).r;
  
  // Get motion vector
  vec2 velocity = texture2D(tVelocity, vUv).xy;
  if (length(velocity) < 0.001) {
    // Fallback to calculated motion vector
    velocity = calculateMotionVector(vUv, depth);
  }
  
  // Calculate previous frame UV
  vec2 prevUV = vUv - velocity;
  
  // Sample history with Catmull-Rom filtering
  vec3 historyColor = sampleCatmullRom(tHistory, prevUV, texelSize);
  
  // Discard history if outside screen bounds
  if (prevUV.x < 0.0 || prevUV.x > 1.0 || prevUV.y < 0.0 || prevUV.y > 1.0) {
    gl_FragColor = vec4(currentColor, 1.0);
    return;
  }
  
  // Convert to YCoCg for better temporal blending
  vec3 currentYCoCg = rgb2ycocg(currentColor);
  vec3 historyYCoCg = rgb2ycocg(historyColor);
  
  // Sample neighborhood for variance clipping
  vec3 minColor = vec3(1.0);
  vec3 maxColor = vec3(0.0);
  vec3 avgColor = vec3(0.0);
  
  for(int x = -1; x <= 1; x++) {
    for(int y = -1; y <= 1; y++) {
      vec2 offset = vec2(float(x), float(y)) * texelSize;
      vec3 neighborColor = rgb2ycocg(texture2D(tColor, vUv + offset).rgb);
      
      minColor = min(minColor, neighborColor);
      maxColor = max(maxColor, neighborColor);
      avgColor += neighborColor;
    }
  }
  avgColor /= 9.0;
  
  // Clip history to neighborhood AABB to reduce ghosting
  historyYCoCg = clipToAABB(historyYCoCg, minColor, maxColor, avgColor);
  
  // Adaptive blend factor based on motion and edge detection
  float blendFactor = u_taaBlendFactor;
  
  // Increase blend factor for high motion areas
  float motionMagnitude = length(velocity);
  blendFactor = mix(blendFactor, 0.5, smoothstep(0.01, 0.1, motionMagnitude));
  
  // Increase blend factor for edges (depth discontinuities)
  vec2 depthGrad = getScreenDerivatives(tDepth, vUv, texelSize);
  float edgeStrength = length(depthGrad);
  blendFactor = mix(blendFactor, 0.3, smoothstep(0.01, 0.1, edgeStrength));
  
  // Temporal accumulation
  vec3 blendedYCoCg = mix(historyYCoCg, currentYCoCg, blendFactor);
  
  // Convert back to RGB
  vec3 finalColor = ycocg2rgb(blendedYCoCg);
  
  // Ensure valid range
  finalColor = clamp(finalColor, 0.0, 1.0);
  
  gl_FragColor = vec4(finalColor, 1.0);
}
