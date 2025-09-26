/**
 * Shader System Entry Point
 * Exports all shader-related functionality for the application
 */

// Core shader management
import { shaderManager, ShaderManager } from './manager';
export { shaderManager, ShaderManager };

// Shader definitions and utilities
export {
  ANIMATED_WATER_SHADER,
  BLOOM_SHADER,
  COLOR_CORRECTION_SHADER,
  DEBUG_GRID_SHADER,
  DEPTH_OF_FIELD_SHADER,
  FXAA_SHADER,
  getShaderDefinition,
  HDR_TONEMAPPING_SHADER,
  HEX_TERRAIN_SHADER,
  MOTION_BLUR_SHADER,
  SHADER_DEFINITIONS,
  SSAO_SHADER,
  SSR_SHADER,
  TAA_SHADER,
  UI_OVERLAY_SHADER,
  updateShaderUniforms,
  VOLUMETRIC_FOG_SHADER,
  type ShaderName,
} from './definitions';

// React integration
export { ShaderProvider } from '../components/rendering/components/providers';
export {
  ShaderDebugInfo,
  useShader,
  useShaders,
} from '../components/rendering/hooks';

// Type definitions
export type {
  ShaderDefinition,
  ShaderUniforms,
  TerrainShaderUniforms,
} from '../types/shaders';

// Module exports for direct shader imports
export { default as volumetricFogFragment } from './fog/volumetric-fog.frag';
export { default as volumetricFogVertex } from './fog/volumetric-fog.vert';
export { default as hexTerrainFragment } from './terrain/hex-terrain.frag';
export { default as hexTerrainVertex } from './terrain/hex-terrain.vert';
export { default as animatedWaterFragment } from './water/animated-water.frag';
export { default as animatedWaterVertex } from './water/animated-water.vert';

// Postprocessing shader exports
export { default as bloomFragment } from './postprocessing/bloom.frag';
export { default as colorCorrectionFragment } from './postprocessing/color-correction.frag';
export { default as depthOfFieldFragment } from './postprocessing/depth-of-field.frag';
export { default as fxaaFragment } from './postprocessing/fxaa.frag';
export { default as hdrToneMappingFragment } from './postprocessing/hdr-tonemapping.frag';
export { default as motionBlurFragment } from './postprocessing/motion-blur.frag';
export { default as postprocessingVertex } from './postprocessing/postprocessing.vert';
export { default as ssaoFragment } from './postprocessing/ssao.frag';
export { default as ssrFragment } from './postprocessing/ssr.frag';
export { default as taaFragment } from './postprocessing/taa.frag';

/**
 * Initialize the shader system
 * Call this during application startup
 */
export const initializeShaderSystem = (): void => {
  if (process.env.NODE_ENV === 'development') {
    console.warn('🎨 Initializing GLSL Shader System...');
    console.warn(
      '📁 Modules: common, hex, noise, shadows, screen-space, color, sampling'
    );
    console.warn(
      '🖼️  Core Shaders: hex-terrain, animated-water, volumetric-fog, debug-grid, ui-overlay'
    );
    console.warn(
      '✨ PostFX: HDR tonemapping, bloom, FXAA, SSAO, DoF, color correction, SSR, TAA, motion blur'
    );
    console.warn(
      '🔧 Features: hot-reload, LOD, instancing, PBR lighting, advanced postprocessing'
    );
  }
};

/**
 * Development utilities
 */
export const shaderDev = {
  manager: shaderManager,
  stats: (): ReturnType<typeof shaderManager.getStats> =>
    shaderManager.getStats(),
  dispose: (): void => shaderManager.dispose(),
};

// Attach to window in development
if (process.env.NODE_ENV === 'development') {
  (window as unknown as { __shaderDev: typeof shaderDev }).__shaderDev =
    shaderDev;
}
