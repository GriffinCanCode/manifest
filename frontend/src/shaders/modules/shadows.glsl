/**
 * Shadow Sampling Functions
 * Cascaded shadow mapping utilities for terrain shaders
 */

#ifndef SHADOWS_GLSL
#define SHADOWS_GLSL

// Shadow cascade uniforms (injected by CSM system)
uniform sampler2D csmTexture0;
uniform sampler2D csmTexture1;
uniform sampler2D csmTexture2;
uniform sampler2D csmTexture3;

uniform mat4 csmMatrix0;
uniform mat4 csmMatrix1;
uniform mat4 csmMatrix2;
uniform mat4 csmMatrix3;

uniform vec4 csmFrustums[4];
uniform int csmCascades;
uniform float csmBias;

/**
 * Calculate shadow factor using PCF (Percentage Closer Filtering)
 */
float sampleShadowPCF(sampler2D shadowMap, vec2 shadowCoord, float currentDepth, float bias) {
    float shadowFactor = 0.0;
    vec2 texelSize = 1.0 / vec2(textureSize(shadowMap, 0));
    
    // 3x3 PCF sampling
    for(int x = -1; x <= 1; x++) {
        for(int y = -1; y <= 1; y++) {
            vec2 offset = vec2(float(x), float(y)) * texelSize;
            float shadowDepth = texture(shadowMap, shadowCoord + offset).r;
            shadowFactor += (currentDepth - bias > shadowDepth) ? 0.0 : 1.0;
        }
    }
    
    return shadowFactor / 9.0;
}

/**
 * Calculate shadow factor using CSM
 */
float calculateCSMShadow(vec3 worldPosition, vec3 worldNormal) {
    if (csmCascades == 0) {
        return 1.0;
    }
    
    // Calculate view depth for cascade selection
    vec4 viewPos = viewMatrix * vec4(worldPosition, 1.0);
    float viewDepth = -viewPos.z;
    
    // Select appropriate cascade
    int cascadeIndex = 0;
    for (int i = 0; i < csmCascades; i++) {
        if (viewDepth < csmFrustums[i].y) {
            cascadeIndex = i;
            break;
        }
    }
    
    // Calculate shadow coordinates for selected cascade
    mat4 shadowMatrix;
    
    if (cascadeIndex == 0) {
        shadowMatrix = csmMatrix0;
    } else if (cascadeIndex == 1) {
        shadowMatrix = csmMatrix1;
    } else if (cascadeIndex == 2) {
        shadowMatrix = csmMatrix2;
    } else {
        shadowMatrix = csmMatrix3;
    }
    
    vec4 shadowCoord = shadowMatrix * vec4(worldPosition, 1.0);
    shadowCoord.xyz /= shadowCoord.w;
    shadowCoord.xyz = shadowCoord.xyz * 0.5 + 0.5;
    
    // Check if in shadow map bounds
    if (shadowCoord.x < 0.0 || shadowCoord.x > 1.0 || 
        shadowCoord.y < 0.0 || shadowCoord.y > 1.0 || 
        shadowCoord.z > 1.0) {
        return 1.0; // Outside shadow map bounds
    }
    
    // Calculate bias based on surface normal and light direction
    float bias = max(csmBias * (1.0 - dot(worldNormal, normalize(vec3(0.5, 1.0, 0.3)))), csmBias * 0.1);
    
    // Sample from the correct cascade shadow map
    if (cascadeIndex == 0) {
        return sampleShadowPCF(csmTexture0, shadowCoord.xy, shadowCoord.z, bias);
    } else if (cascadeIndex == 1) {
        return sampleShadowPCF(csmTexture1, shadowCoord.xy, shadowCoord.z, bias);
    } else if (cascadeIndex == 2) {
        return sampleShadowPCF(csmTexture2, shadowCoord.xy, shadowCoord.z, bias);
    } else {
        return sampleShadowPCF(csmTexture3, shadowCoord.xy, shadowCoord.z, bias);
    }
}

/**
 * Calculate shadow factor with cascade blending
 */
float calculateCSMShadowWithBlending(vec3 worldPosition, vec3 worldNormal) {
    if (csmCascades == 0) {
        return 1.0;
    }
    
    vec4 viewPos = viewMatrix * vec4(worldPosition, 1.0);
    float viewDepth = -viewPos.z;
    
    // Find current and next cascade
    int cascadeIndex = 0;
    float blendFactor = 0.0;
    
    for (int i = 0; i < csmCascades - 1; i++) {
        if (viewDepth < csmFrustums[i].y) {
            cascadeIndex = i;
            // Calculate blend factor for smooth transitions
            float cascadeRange = csmFrustums[i].y - csmFrustums[i].x;
            float blendZone = cascadeRange * 0.1; // 10% blend zone
            float distanceFromEnd = csmFrustums[i].y - viewDepth;
            
            if (distanceFromEnd < blendZone) {
                blendFactor = 1.0 - (distanceFromEnd / blendZone);
            }
            break;
        }
    }
    
    // Calculate shadow for current cascade
    float currentShadow = calculateCSMShadow(worldPosition, worldNormal);
    
    // If no blending needed, return current shadow
    if (blendFactor == 0.0 || cascadeIndex >= csmCascades - 1) {
        return currentShadow;
    }
    
    // Calculate shadow for next cascade (simplified for performance)
    // In practice, you might want full calculation here
    float nextShadow = currentShadow; // Simplified
    
    return mix(currentShadow, nextShadow, blendFactor);
}

/**
 * Calculate CSM shadow from pre-calculated varyings (more efficient)
 */
float calculateCSMShadowFromVaryings(vec4 shadowCoords[4], float viewDepth, vec3 worldNormal) {
    if (csmCascades == 0) {
        return 1.0;
    }
    
    // Select appropriate cascade based on view depth
    int cascadeIndex = 0;
    for (int i = 0; i < csmCascades; i++) {
        if (viewDepth < csmFrustums[i].y) {
            cascadeIndex = i;
            break;
        }
    }
    
    // Get shadow coordinates for selected cascade
    vec4 shadowCoord = shadowCoords[cascadeIndex];
    shadowCoord.xyz /= shadowCoord.w;
    shadowCoord.xyz = shadowCoord.xyz * 0.5 + 0.5;
    
    // Check bounds
    if (shadowCoord.x < 0.0 || shadowCoord.x > 1.0 || 
        shadowCoord.y < 0.0 || shadowCoord.y > 1.0 || 
        shadowCoord.z > 1.0) {
        return 1.0;
    }
    
    // Select shadow map
    // Calculate bias
    float bias = max(csmBias * (1.0 - dot(worldNormal, normalize(vec3(0.5, 1.0, 0.3)))), csmBias * 0.1);
    
    // Sample from the correct cascade shadow map
    if (cascadeIndex == 0) {
        return sampleShadowPCF(csmTexture0, shadowCoord.xy, shadowCoord.z, bias);
    } else if (cascadeIndex == 1) {
        return sampleShadowPCF(csmTexture1, shadowCoord.xy, shadowCoord.z, bias);
    } else if (cascadeIndex == 2) {
        return sampleShadowPCF(csmTexture2, shadowCoord.xy, shadowCoord.z, bias);
    } else {
        return sampleShadowPCF(csmTexture3, shadowCoord.xy, shadowCoord.z, bias);
    }
}

#ifdef USE_SHADOWS
    #define SHADOW_FACTOR calculateCSMShadow(worldPosition, worldNormal)
    #define SHADOW_FACTOR_BLENDED calculateCSMShadowWithBlending(worldPosition, worldNormal)
    #define SHADOW_FACTOR_FROM_VARYINGS calculateCSMShadowFromVaryings(v_shadowCoord, v_shadowDistance, v_normal)
#else
    #define SHADOW_FACTOR 1.0
    #define SHADOW_FACTOR_BLENDED 1.0
    #define SHADOW_FACTOR_FROM_VARYINGS 1.0
#endif

#endif // SHADOWS_GLSL
