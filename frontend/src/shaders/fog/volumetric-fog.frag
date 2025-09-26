#ifdef GL_ES
precision highp float;
#endif

uniform sampler2D tColor;
uniform sampler2D tDepth;
uniform vec2 u_resolution;
uniform float u_time;
uniform vec3 u_cameraPosition;
uniform float u_cameraNear;
uniform float u_cameraFar;

// Fog properties
uniform float u_fogDensity;
uniform vec3 u_fogColor;
uniform float u_scatteringCoeff;
uniform float u_absorptionCoeff;
uniform float u_fogNear;
uniform float u_fogFar;
uniform int u_steps;

// Lighting
uniform vec3 u_lightDirection;
uniform float u_lightIntensity;

// Wind and noise
uniform int u_useNoise;
uniform float u_windSpeed;
uniform vec2 u_windDirection;

varying vec2 vUv;
varying vec3 vRayDirection;

// Convert depth buffer to linear depth
float linearizeDepth(float depth) {
  float z = depth * 2.0 - 1.0;
  return (2.0 * u_cameraNear * u_cameraFar) / 
         (u_cameraFar + u_cameraNear - z * (u_cameraFar - u_cameraNear));
}

// 3D Simplex noise for fog perturbation
vec3 mod289(vec3 x) { return x - floor(x * (1.0 / 289.0)) * 289.0; }
vec4 mod289(vec4 x) { return x - floor(x * (1.0 / 289.0)) * 289.0; }
vec4 permute(vec4 x) { return mod289(((x*34.0)+1.0)*x); }
vec4 taylorInvSqrt(vec4 r) { return 1.79284291400159 - 0.85373472095314 * r; }

float snoise(vec3 v) { 
  const vec2 C = vec2(1.0/6.0, 1.0/3.0);
  const vec4 D = vec4(0.0, 0.5, 1.0, 2.0);
  
  vec3 i  = floor(v + dot(v, C.yyy));
  vec3 x0 = v - i + dot(i, C.xxx);
  
  vec3 g = step(x0.yzx, x0.xyz);
  vec3 l = 1.0 - g;
  vec3 i1 = min(g.xyz, l.zxy);
  vec3 i2 = max(g.xyz, l.zxy);
  
  vec3 x1 = x0 - i1 + C.xxx;
  vec3 x2 = x0 - i2 + C.yyy;
  vec3 x3 = x0 - D.yyy;
  
  i = mod289(i);
  vec4 p = permute(permute(permute(
           i.z + vec4(0.0, i1.z, i2.z, 1.0))
         + i.y + vec4(0.0, i1.y, i2.y, 1.0))
         + i.x + vec4(0.0, i1.x, i2.x, 1.0));
  
  float n_ = 0.142857142857; // 1.0/7.0
  vec3 ns = n_ * D.wyz - D.xzx;
  
  vec4 j = p - 49.0 * floor(p * ns.z * ns.z);
  
  vec4 x_ = floor(j * ns.z);
  vec4 y_ = floor(j - 7.0 * x_);
  
  vec4 x = x_ *ns.x + ns.yyyy;
  vec4 y = y_ *ns.x + ns.yyyy;
  vec4 h = 1.0 - abs(x) - abs(y);
  
  vec4 b0 = vec4(x.xy, y.xy);
  vec4 b1 = vec4(x.zw, y.zw);
  
  vec4 s0 = floor(b0)*2.0 + 1.0;
  vec4 s1 = floor(b1)*2.0 + 1.0;
  vec4 sh = -step(h, vec4(0.0));
  
  vec4 a0 = b0.xzyw + s0.xzyw*sh.xxyy;
  vec4 a1 = b1.xzyw + s1.xzyw*sh.zzww;
  
  vec3 p0 = vec3(a0.xy, h.x);
  vec3 p1 = vec3(a0.zw, h.y);
  vec3 p2 = vec3(a1.xy, h.z);
  vec3 p3 = vec3(a1.zw, h.w);
  
  vec4 norm = taylorInvSqrt(vec4(dot(p0,p0), dot(p1,p1), dot(p2,p2), dot(p3,p3)));
  p0 *= norm.x;
  p1 *= norm.y;
  p2 *= norm.z;
  p3 *= norm.w;
  
  vec4 m = max(0.6 - vec4(dot(x0,x0), dot(x1,x1), dot(x2,x2), dot(x3,x3)), 0.0);
  m = m * m;
  return 42.0 * dot(m*m, vec4(dot(p0,x0), dot(p1,x1), dot(p2,x2), dot(p3,x3)));
}

// Multi-octave noise for fog density variation
float fbm(vec3 p) {
  float value = 0.0;
  float amplitude = 0.5;
  float frequency = 1.0;
  
  for (int i = 0; i < 4; i++) {
    value += amplitude * snoise(p * frequency);
    amplitude *= 0.5;
    frequency *= 2.0;
  }
  
  return value;
}

// Henyey-Greenstein phase function for atmospheric scattering
float henyeyGreenstein(float cosTheta, float g) {
  float g2 = g * g;
  return (1.0 - g2) / pow(1.0 + g2 - 2.0 * g * cosTheta, 1.5);
}

// Calculate fog density at a world position
float calculateFogDensity(vec3 worldPos) {
  float baseDensity = u_fogDensity;
  
  // Height-based density falloff
  float heightFactor = exp(-max(0.0, worldPos.y) * 0.1);
  baseDensity *= heightFactor;
  
  // Noise perturbation for realistic fog variation
  if (u_useNoise == 1) {
    vec3 windOffset = vec3(
      u_windDirection.x * u_windSpeed * u_time,
      0.0,
      u_windDirection.y * u_windSpeed * u_time
    );
    
    float noiseValue = fbm((worldPos + windOffset) * 0.05);
    baseDensity *= (1.0 + noiseValue * 0.3);
  }
  
  return max(0.0, baseDensity);
}

// Ray-marched volumetric fog
vec3 calculateVolumetricFog(vec3 rayStart, vec3 rayDirection, float rayLength) {
  float stepSize = rayLength / float(u_steps);
  vec3 currentPos = rayStart;
  vec3 stepVector = rayDirection * stepSize;
  
  vec3 accumulatedLight = vec3(0.0);
  float transmittance = 1.0;
  
  // March along the ray
  for (int i = 0; i < 128; i++) {
    if (i >= u_steps) break;
    
    currentPos += stepVector;
    
    // Calculate fog density at current position
    float density = calculateFogDensity(currentPos);
    if (density <= 0.0) continue;
    
    // Calculate lighting contribution
    float lightDistance = length(currentPos - u_cameraPosition);
    float lightAttenuation = 1.0 / (1.0 + lightDistance * lightDistance * 0.01);
    
    // Phase function for in-scattering
    float cosTheta = dot(-u_lightDirection, rayDirection);
    float phase = henyeyGreenstein(cosTheta, 0.3);
    
    // Calculate in-scattered light
    vec3 inScattering = u_fogColor * density * u_scatteringCoeff * 
                       u_lightIntensity * lightAttenuation * phase * stepSize;
    
    // Apply transmittance
    accumulatedLight += inScattering * transmittance;
    
    // Update transmittance (Beer's law)
    float extinction = density * (u_scatteringCoeff + u_absorptionCoeff) * stepSize;
    transmittance *= exp(-extinction);
    
    // Early termination if transmittance is very low
    if (transmittance < 0.01) break;
  }
  
  return accumulatedLight;
}

void main() {
  vec4 originalColor = texture2D(tColor, vUv);
  float depth = texture2D(tDepth, vUv).r;
  float linearDepth = linearizeDepth(depth);
  
  // Skip fog calculation if depth is at far plane (skybox)
  if (depth >= 0.9999) {
    gl_FragColor = originalColor;
    return;
  }
  
  // Calculate ray parameters
  vec3 rayDirection = normalize(vRayDirection);
  float rayLength = min(linearDepth, u_fogFar);
  
  // Skip fog if too close
  if (rayLength < u_fogNear) {
    gl_FragColor = originalColor;
    return;
  }
  
  // Calculate fog contribution
  vec3 rayStart = u_cameraPosition;
  vec3 fogContribution = calculateVolumetricFog(rayStart, rayDirection, rayLength);
  
  // Distance-based fog mixing
  float fogFactor = smoothstep(u_fogNear, u_fogFar, rayLength);
  fogFactor *= smoothstep(0.0, 1.0, length(fogContribution));
  
  // Blend with original color
  vec3 finalColor = mix(originalColor.rgb, originalColor.rgb + fogContribution, fogFactor);
  
  gl_FragColor = vec4(finalColor, originalColor.a);
}
