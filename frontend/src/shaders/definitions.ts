/**
 * Shader Definitions
 * Central registry of all shader programs with their uniforms
 */

import { Color, Matrix4, Vector3 } from 'three';

import type { ShaderDefinition, TerrainShaderUniforms } from '../types/shaders';

// Import compiled shaders

// Import water shaders
// Import fog shaders
import volumetricFogFragmentShader from './fog/volumetric-fog.frag';
import volumetricFogVertexShader from './fog/volumetric-fog.vert';
// Import postprocessing shaders
import bloomFragmentShader from './postprocessing/bloom.frag';
import colorCorrectionFragmentShader from './postprocessing/color-correction.frag';
import depthOfFieldFragmentShader from './postprocessing/depth-of-field.frag';
import fxaaFragmentShader from './postprocessing/fxaa.frag';
import hdrToneMappingFragmentShader from './postprocessing/hdr-tonemapping.frag';
import motionBlurFragmentShader from './postprocessing/motion-blur.frag';
import postprocessingVertexShader from './postprocessing/postprocessing.vert';
import ssaoFragmentShader from './postprocessing/ssao.frag';
import ssrFragmentShader from './postprocessing/ssr.frag';
import taaFragmentShader from './postprocessing/taa.frag';
import hexTerrainFragmentShader from './terrain/hex-terrain.frag';
import hexTerrainVertexShader from './terrain/hex-terrain.vert';
import animatedWaterFragmentShader from './water/animated-water.frag';
import animatedWaterVertexShader from './water/animated-water.vert';

/**
 * Hex Terrain Shader Definition
 * Main shader for rendering the procedural hex-based world
 */
export const HEX_TERRAIN_SHADER: ShaderDefinition = {
  name: 'hex-terrain',
  vertexShader: hexTerrainVertexShader,
  fragmentShader: hexTerrainFragmentShader,
  uniforms: {
    // Time and animation
    u_time: { value: 0 },
    u_deltaTime: { value: 0 },

    // Note: Camera matrices and position are automatically provided by Three.js
    // Don't override built-in uniforms: modelMatrix, viewMatrix, projectionMatrix, cameraPosition

    // Hex terrain properties
    u_hexSize: { value: 1.0 },
    u_hexSpacing: { value: 1.1 }, // ALIGNED with backend spacing
    u_heightScale: { value: 10.0 },
    u_lodDistance: { value: 100.0 },
    u_qualityLevel: { value: 3 },

    // Lighting
    u_lightDirection: { value: new Vector3(1, -1, 1).normalize() },
    u_lightColor: { value: new Color(0xffffff) },
    u_lightIntensity: { value: 1.0 },
    u_ambientColor: { value: new Color(0x404040) },
    u_ambientIntensity: { value: 0.2 },

    // Material properties
    u_roughness: { value: 0.8 },
    u_metallic: { value: 0.0 },
    u_specularIntensity: { value: 0.3 },

    // HDR properties
    u_exposure: { value: 1.0 },
    u_emissiveIntensity: { value: 1.0 },

    // Rendering controls
    u_resolution: { value: new Vector3(1920, 1080) },
    u_wireframe: { value: false },
    u_wireframeWidth: { value: 0.02 },

    // Fog
    u_fogColor: { value: new Color(0x87ceeb) },
    u_fogNear: { value: 50.0 },
    u_fogFar: { value: 200.0 },
    u_fogDensity: { value: 0.01 },

    // Debug modes
    u_showLOD: { value: false },
    u_showBiomes: { value: false },
    u_showResources: { value: false },
    u_showHeight: { value: false },

    // Procedural texture uniforms
    u_hasAlbedoTexture: { value: false },
    u_hasNormalTexture: { value: false },
    u_hasRoughnessTexture: { value: false },
    u_hasMetallicTexture: { value: false },
    u_albedoTexture: { value: null },
    u_normalTexture: { value: null },
    u_roughnessTexture: { value: null },
    u_metallicTexture: { value: null },
    u_textureScale: { value: 1.0 },
  } as TerrainShaderUniforms,
  defines: {
    // Note: USE_INSTANCING is handled automatically by Three.js
    USE_LOD: 1,
    HEX_TILES: 1,
    TERRAIN_DISPLACEMENT: 1,
  },
};

/**
 * Animated Water Shader Definition
 */
export const ANIMATED_WATER_SHADER: ShaderDefinition = {
  name: 'animated-water',
  vertexShader: animatedWaterVertexShader,
  fragmentShader: animatedWaterFragmentShader,
  uniforms: {
    u_time: { value: 0 },
    u_waterColor: { value: new Color(0x4a90e2) },
    u_foamColor: { value: new Color(0xffffff) },
    u_deepWaterColor: { value: new Color(0x1a4480) },
    u_waveHeight: { value: 0.5 },
    u_waveSpeed: { value: 1.0 },
    u_foamThreshold: { value: 0.7 },
    u_transparency: { value: 0.8 },
    // Note: cameraPosition is automatically provided by Three.js
    u_lightDirection: { value: new Vector3(1, -1, 1).normalize() },
    u_ambientColor: { value: new Color(0x404040) },
    u_ambientIntensity: { value: 0.2 },
    u_specularIntensity: { value: 0.8 },
    u_roughness: { value: 0.1 },
  },
  defines: {
    USE_WATER_ANIMATION: 1,
    USE_FOAM: 1,
  },
};

/**
 * Volumetric Fog Shader Definition
 */
export const VOLUMETRIC_FOG_SHADER: ShaderDefinition = {
  name: 'volumetric-fog',
  vertexShader: volumetricFogVertexShader,
  fragmentShader: volumetricFogFragmentShader,
  uniforms: {
    tColor: { value: null },
    tDepth: { value: null },
    u_resolution: { value: new Vector3(1920, 1080, 0) },
    u_time: { value: 0 },
    u_cameraPosition: { value: new Vector3() },
    u_cameraNear: { value: 0.1 },
    u_cameraFar: { value: 1000.0 },
    u_fogDensity: { value: 0.01 },
    u_fogColor: { value: new Color(0x87ceeb) },
    u_scatteringCoeff: { value: 0.1 },
    u_absorptionCoeff: { value: 0.05 },
    u_fogNear: { value: 50.0 },
    u_fogFar: { value: 200.0 },
    u_steps: { value: 64 },
    u_lightDirection: { value: new Vector3(1, -1, 1).normalize() },
    u_lightIntensity: { value: 1.0 },
    u_useNoise: { value: 1 },
    u_windSpeed: { value: 0.5 },
    u_windDirection: { value: new Vector3(1, 0).normalize() },
    u_projectionMatrixInverse: { value: new Matrix4() },
    u_viewMatrixInverse: { value: new Matrix4() },
  },
  defines: {
    USE_VOLUMETRIC_FOG: 1,
    USE_NOISE: 1,
  },
};

/**
 * Debug Grid Shader Definition
 */
export const DEBUG_GRID_SHADER: ShaderDefinition = {
  name: 'debug-grid',
  vertexShader: `
    attribute vec3 position;
    uniform mat4 modelViewMatrix;
    uniform mat4 projectionMatrix;
    
    void main() {
      gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
    }
  `,
  fragmentShader: `
    uniform vec3 u_gridColor;
    uniform float u_opacity;
    
    void main() {
      gl_FragColor = vec4(u_gridColor, u_opacity);
    }
  `,
  uniforms: {
    u_gridColor: { value: new Color(0xffffff) },
    u_opacity: { value: 0.3 },
  },
};

/**
 * UI Overlay Shader Definition
 */
export const UI_OVERLAY_SHADER: ShaderDefinition = {
  name: 'ui-overlay',
  vertexShader: `
    attribute vec3 position;
    attribute vec2 uv;
    varying vec2 v_uv;
    uniform mat4 modelViewMatrix;
    uniform mat4 projectionMatrix;
    
    void main() {
      v_uv = uv;
      gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
    }
  `,
  fragmentShader: `
    varying vec2 v_uv;
    uniform sampler2D u_texture;
    uniform float u_opacity;
    uniform vec3 u_tint;
    
    void main() {
      vec4 texColor = texture2D(u_texture, v_uv);
      gl_FragColor = vec4(texColor.rgb * u_tint, texColor.a * u_opacity);
    }
  `,
  uniforms: {
    u_opacity: { value: 1.0 },
    u_tint: { value: new Color(0xffffff) },
  },
};

/**
 * HDR Tone Mapping Shader Definition
 */
export const HDR_TONEMAPPING_SHADER: ShaderDefinition = {
  name: 'hdr-tonemapping',
  vertexShader: postprocessingVertexShader,
  fragmentShader: hdrToneMappingFragmentShader,
  uniforms: {
    tColor: { value: null },
    u_resolution: { value: new Vector3(1920, 1080, 0) },
    u_exposure: { value: 1.0 },
    u_whitePoint: { value: 11.2 },
    u_toneMappingType: { value: 3 }, // ACES by default
    u_adaptationRate: { value: 0.1 },
    u_minLuminance: { value: 0.01 },
    u_maxLuminance: { value: 100.0 },
    u_autoExposure: { value: false },
  },
  defines: {
    USE_HDR_TONEMAPPING: 1,
  },
};

/**
 * Bloom Effect Shader Definition
 */
export const BLOOM_SHADER: ShaderDefinition = {
  name: 'bloom',
  vertexShader: postprocessingVertexShader,
  fragmentShader: bloomFragmentShader,
  uniforms: {
    tColor: { value: null },
    tBloom: { value: null },
    u_resolution: { value: new Vector3(1920, 1080, 0) },
    u_bloomThreshold: { value: 1.0 },
    u_bloomSoftKnee: { value: 0.5 },
    u_bloomIntensity: { value: 0.8 },
    u_bloomRadius: { value: 1.0 },
    u_bloomEnabled: { value: true },
    u_passType: { value: 0 }, // 0: threshold, 1: blur_h, 2: blur_v, 3: composite
  },
  defines: {
    USE_BLOOM: 1,
  },
};

/**
 * FXAA Anti-aliasing Shader Definition
 */
export const FXAA_SHADER: ShaderDefinition = {
  name: 'fxaa',
  vertexShader: postprocessingVertexShader,
  fragmentShader: fxaaFragmentShader,
  uniforms: {
    tColor: { value: null },
    u_resolution: { value: new Vector3(1920, 1080, 0) },
    u_fxaaQualitySubpix: { value: 0.75 },
    u_fxaaQualityEdgeThreshold: { value: 0.166 },
    u_fxaaQualityEdgeThresholdMin: { value: 0.0625 },
    u_fxaaEnabled: { value: true },
  },
  defines: {
    USE_FXAA: 1,
  },
};

/**
 * SSAO (Screen Space Ambient Occlusion) Shader Definition
 */
export const SSAO_SHADER: ShaderDefinition = {
  name: 'ssao',
  vertexShader: postprocessingVertexShader,
  fragmentShader: ssaoFragmentShader,
  uniforms: {
    tColor: { value: null },
    tDepth: { value: null },
    tNormal: { value: null },
    u_resolution: { value: new Vector3(1920, 1080, 0) },
    u_projectionMatrix: { value: new Matrix4() },
    u_projectionMatrixInverse: { value: new Matrix4() },
    u_cameraNear: { value: 0.1 },
    u_cameraFar: { value: 1000.0 },
    u_ssaoRadius: { value: 0.5 },
    u_ssaoIntensity: { value: 1.0 },
    u_ssaoBias: { value: 0.025 },
    u_ssaoFalloff: { value: 1.0 },
    u_ssaoSamples: { value: 16 },
    u_ssaoEnabled: { value: true },
    u_time: { value: 0 },
  },
  defines: {
    USE_SSAO: 1,
  },
};

/**
 * Depth of Field Shader Definition
 */
export const DEPTH_OF_FIELD_SHADER: ShaderDefinition = {
  name: 'depth-of-field',
  vertexShader: postprocessingVertexShader,
  fragmentShader: depthOfFieldFragmentShader,
  uniforms: {
    tColor: { value: null },
    tDepth: { value: null },
    u_resolution: { value: new Vector3(1920, 1080, 0) },
    u_focusDistance: { value: 10.0 },
    u_focusRange: { value: 5.0 },
    u_bokehRadius: { value: 4.0 },
    u_aperture: { value: 0.025 },
    u_cameraNear: { value: 0.1 },
    u_cameraFar: { value: 1000.0 },
    u_dofEnabled: { value: true },
    u_bokehSamples: { value: 32 },
  },
  defines: {
    USE_DEPTH_OF_FIELD: 1,
  },
};

/**
 * Color Correction and Grading Shader Definition
 */
export const COLOR_CORRECTION_SHADER: ShaderDefinition = {
  name: 'color-correction',
  vertexShader: postprocessingVertexShader,
  fragmentShader: colorCorrectionFragmentShader,
  uniforms: {
    tColor: { value: null },
    u_resolution: { value: new Vector3(1920, 1080, 0) },
    u_brightness: { value: 0.0 },
    u_contrast: { value: 1.0 },
    u_saturation: { value: 1.0 },
    u_vibrance: { value: 0.0 },
    u_hueShift: { value: 0.0 },
    u_temperature: { value: 6500.0 },
    u_tint: { value: 0.0 },
    u_lift: { value: new Color(0x000000) },
    u_gamma: { value: new Color(0x808080) },
    u_gain: { value: new Color(0xffffff) },
    u_shadows: { value: new Color(0x000000) },
    u_midtones: { value: new Color(0x808080) },
    u_highlights: { value: new Color(0xffffff) },
    u_channelMix: { value: new Matrix4() }, // 3x3 stored in 4x4
    u_vignetteIntensity: { value: 0.0 },
    u_vignetteSmoothness: { value: 0.5 },
    u_vignetteRoundness: { value: 1.0 },
    u_grainAmount: { value: 0.0 },
    u_grainSize: { value: 2.0 },
    u_time: { value: 0 },
    u_colorCorrectionEnabled: { value: true },
    u_vignetteEnabled: { value: false },
    u_grainEnabled: { value: false },
  },
  defines: {
    USE_COLOR_CORRECTION: 1,
  },
};

/**
 * Screen Space Reflections Shader Definition
 */
export const SSR_SHADER: ShaderDefinition = {
  name: 'ssr',
  vertexShader: postprocessingVertexShader,
  fragmentShader: ssrFragmentShader,
  uniforms: {
    tColor: { value: null },
    tDepth: { value: null },
    tNormal: { value: null },
    u_resolution: { value: new Vector3(1920, 1080, 0) },
    u_viewMatrix: { value: new Matrix4() },
    u_projectionMatrix: { value: new Matrix4() },
    u_projectionMatrixInverse: { value: new Matrix4() },
    u_cameraPosition: { value: new Vector3() },
    u_cameraNear: { value: 0.1 },
    u_cameraFar: { value: 1000.0 },
    u_ssrIntensity: { value: 0.5 },
    u_ssrMaxDistance: { value: 100.0 },
    u_ssrSteps: { value: 32 },
    u_ssrBinarySteps: { value: 4 },
    u_ssrThickness: { value: 0.5 },
    u_ssrJitter: { value: 0.1 },
    u_ssrFresnel: { value: 1.0 },
    u_ssrEnabled: { value: true },
    u_time: { value: 0 },
  },
  defines: {
    USE_SSR: 1,
  },
};

/**
 * Temporal Anti-Aliasing Shader Definition
 */
export const TAA_SHADER: ShaderDefinition = {
  name: 'taa',
  vertexShader: postprocessingVertexShader,
  fragmentShader: taaFragmentShader,
  uniforms: {
    tColor: { value: null },
    tHistory: { value: null },
    tDepth: { value: null },
    tVelocity: { value: null },
    u_resolution: { value: new Vector3(1920, 1080, 0) },
    u_viewProjectionMatrix: { value: new Matrix4() },
    u_prevViewProjectionMatrix: { value: new Matrix4() },
    u_taaBlendFactor: { value: 0.05 },
    u_taaClampFactor: { value: 1.0 },
    u_taaEnabled: { value: true },
    u_frameCount: { value: 0 },
    u_time: { value: 0 },
  },
  defines: {
    USE_TAA: 1,
  },
};

/**
 * Motion Blur Shader Definition
 */
export const MOTION_BLUR_SHADER: ShaderDefinition = {
  name: 'motion-blur',
  vertexShader: postprocessingVertexShader,
  fragmentShader: motionBlurFragmentShader,
  uniforms: {
    tColor: { value: null },
    tVelocity: { value: null },
    tDepth: { value: null },
    u_resolution: { value: new Vector3(1920, 1080, 0) },
    u_motionBlurIntensity: { value: 0.5 },
    u_motionBlurSamples: { value: 8 },
    u_motionBlurMaxRadius: { value: 32.0 },
    u_motionBlurEnabled: { value: true },
  },
  defines: {
    USE_MOTION_BLUR: 1,
  },
};

/**
 * All available shader definitions
 */
export const SHADER_DEFINITIONS = {
  'hex-terrain': HEX_TERRAIN_SHADER,
  'animated-water': ANIMATED_WATER_SHADER,
  'volumetric-fog': VOLUMETRIC_FOG_SHADER,
  'debug-grid': DEBUG_GRID_SHADER,
  'ui-overlay': UI_OVERLAY_SHADER,

  // Postprocessing shaders
  'hdr-tonemapping': HDR_TONEMAPPING_SHADER,
  bloom: BLOOM_SHADER,
  fxaa: FXAA_SHADER,
  ssao: SSAO_SHADER,
  'depth-of-field': DEPTH_OF_FIELD_SHADER,
  'color-correction': COLOR_CORRECTION_SHADER,
  ssr: SSR_SHADER,
  taa: TAA_SHADER,
  'motion-blur': MOTION_BLUR_SHADER,
} as const;

export type ShaderName = keyof typeof SHADER_DEFINITIONS;

/**
 * Get shader definition by name
 */
export const getShaderDefinition = (name: ShaderName): ShaderDefinition => {
  return SHADER_DEFINITIONS[name];
};

/**
 * Update shader uniforms based on render state
 */
export const updateShaderUniforms = (
  uniforms: TerrainShaderUniforms,
  time: number,
  _cameraPosition: Vector3,
  qualityLevel: number
): void => {
  uniforms.u_time.value = time;
  // Note: cameraPosition is automatically provided by Three.js, no need to update manually
  uniforms.u_qualityLevel.value = qualityLevel;

  // Adjust LOD distance based on quality
  const lodDistances = [50, 75, 100, 150]; // low, medium, high, ultra
  uniforms.u_lodDistance.value = lodDistances[qualityLevel - 1] || 100;

  // Adjust fog based on quality
  if (qualityLevel < 3) {
    uniforms.u_fogDensity.value = 0.02; // More fog for lower quality
  } else {
    uniforms.u_fogDensity.value = 0.01; // Less fog for higher quality
  }
};
