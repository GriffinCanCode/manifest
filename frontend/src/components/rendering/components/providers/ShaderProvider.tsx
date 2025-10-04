/**
 * Shader Provider Component
 * Integrates GLSL shader system with Three.js and React ecosystem
 */

import { useFrame, useThree } from '@react-three/fiber';
import React, { useEffect, useRef, useState } from 'react';
import { Vector3, type Color, type ShaderMaterial } from 'three';

// Browser-compatible environment check with proper typing
interface ViteImportMeta {
  env?: {
    MODE?: string;
    [key: string]: unknown;
  };
}

import {
  getShaderDefinition,
  updateShaderUniforms,
  type ShaderName,
} from '../../../../shaders/definitions';
import { shaderManager } from '../../../../shaders/manager';
import { useRenderStore } from '../../../../stores/render-store';
import type {
  ShaderUniforms,
  TerrainShaderUniforms,
} from '../../../../types/shaders';
import {
  ShaderContext,
  type ShaderContextType,
} from '../../hooks/shader-hooks';

interface ShaderProviderProps {
  children: React.ReactNode;
}

/**
 * Provides shader management context to child components
 */
export const ShaderProvider: React.FC<ShaderProviderProps> = ({ children }) => {
  const { camera } = useThree();
  const { quality, settings, capabilities, isInitialized } = useRenderStore();

  const shadersRef = useRef<Map<ShaderName, ShaderMaterial>>(new Map());
  const timeRef = useRef<number>(0);
  const [isReady, setIsReady] = useState<boolean>(false);

  /**
   * Initialize all core shaders
   */
  // Extract fog state for dependency tracking
  const disableFog = useRenderStore(state => state.debug.disableFog);

  useEffect(() => {
    if (!isInitialized || !capabilities) return;

    const initShaders = () => {
      try {
        // Initialize hex terrain shader
        const hexTerrainDef = getShaderDefinition('hex-terrain');
        const hexTerrainMaterial = shaderManager.compile(
          'hex-terrain',
          hexTerrainDef,
          {
            defines: {
              QUALITY_LEVEL:
                quality.level === 'low'
                  ? 1
                  : quality.level === 'medium'
                    ? 2
                    : quality.level === 'high'
                      ? 3
                      : 4,
              USE_SHADOWS:
                settings?.shadows && capabilities.supportsShadows ? 1 : 0,
              USE_FOG: !disableFog ? 1 : 0,
              // Note: USE_INSTANCING is automatically handled by Three.js when using instanced geometry
              USE_HDR: capabilities.supportsHDR ? 1 : 0,
            },
          }
        );

        shadersRef.current.set('hex-terrain', hexTerrainMaterial);

        // Initialize animated water shader
        const animatedWaterDef = getShaderDefinition('animated-water');
        const animatedWaterMaterial = shaderManager.compile(
          'animated-water',
          animatedWaterDef,
          {
            defines: {
              QUALITY_LEVEL:
                quality.level === 'low'
                  ? 1
                  : quality.level === 'medium'
                    ? 2
                    : quality.level === 'high'
                      ? 3
                      : 4,
              USE_WATER_ANIMATION: 1,
              USE_FOAM: capabilities.supportsFloatTextures ? 1 : 0,
            },
            transparent: true,
            depthWrite: false,
          }
        );
        shadersRef.current.set('animated-water', animatedWaterMaterial);

        // Initialize debug grid shader
        const debugGridDef = getShaderDefinition('debug-grid');
        const debugGridMaterial = shaderManager.compile(
          'debug-grid',
          debugGridDef
        );
        shadersRef.current.set('debug-grid', debugGridMaterial);

        // Initialize UI overlay shader
        const uiOverlayDef = getShaderDefinition('ui-overlay');
        const uiOverlayMaterial = shaderManager.compile(
          'ui-overlay',
          uiOverlayDef
        );
        shadersRef.current.set('ui-overlay', uiOverlayMaterial);

        // Initialize postprocessing shaders
        const postprocessingShaders = [
          'volumetric-fog',
          'hdr-tonemapping',
          'bloom',
          'fxaa',
          'ssao',
          'depth-of-field',
          'color-correction',
          'ssr',
          'taa',
          'motion-blur',
        ];

        for (const shaderName of postprocessingShaders) {
          try {
            const shaderDef = getShaderDefinition(shaderName as ShaderName);
            const shaderMaterial = shaderManager.compile(
              shaderName,
              shaderDef,
              {
                defines: {
                  QUALITY_LEVEL:
                    quality.level === 'low'
                      ? 1
                      : quality.level === 'medium'
                        ? 2
                        : quality.level === 'high'
                          ? 3
                          : 4,
                  USE_HDR: capabilities.supportsHDR ? 1 : 0,
                },
              }
            );
            shadersRef.current.set(shaderName as ShaderName, shaderMaterial);
          } catch (error) {
            console.warn(`⚠️ Failed to compile ${shaderName} shader:`, error);
          }
        }

        setIsReady(true);
      } catch (error) {
        console.error('❌ Failed to initialize shader system:', error);
      }
    };

    initShaders();
  }, [
    isInitialized,
    capabilities,
    quality.level,
    settings?.shadows,
    disableFog,
  ]);

  /**
   * Update shader uniforms every frame
   * Note: RenderingProvider handles global uniform updates, this handles shader-specific updates
   */
  useFrame((_state, delta) => {
    if (!isReady) return;

    timeRef.current += delta;
    const cameraPosition = new Vector3().setFromMatrixPosition(
      camera.matrixWorld
    );
    const qualityLevel =
      quality.level === 'low'
        ? 1
        : quality.level === 'medium'
          ? 2
          : quality.level === 'high'
            ? 3
            : 4;

    // Update hex terrain shader uniforms
    const hexTerrainMaterial = shadersRef.current.get('hex-terrain');
    if (hexTerrainMaterial?.uniforms) {
      try {
        updateShaderUniforms(
          hexTerrainMaterial.uniforms as TerrainShaderUniforms,
          timeRef.current,
          cameraPosition,
          qualityLevel
        );

        // Update performance-based settings (only if uniform exists)
        if (hexTerrainMaterial.uniforms.u_wireframe) {
          hexTerrainMaterial.uniforms.u_wireframe.value =
            useRenderStore.getState().debug.showWireframe;
        }

        // Update fog color based on time of day (future enhancement)
        if ((import.meta as ViteImportMeta)?.env?.MODE === 'development') {
          const dayFactor = Math.sin(timeRef.current * 0.1) * 0.5 + 0.5;
          const fogColor = hexTerrainMaterial.uniforms.u_fogColor
            ?.value as Color;
          if (fogColor && typeof fogColor.setHSL === 'function') {
            fogColor.setHSL(0.55, 0.3, 0.7 + dayFactor * 0.2);
          }
        }

        hexTerrainMaterial.uniformsNeedUpdate = true;
      } catch (error) {
        console.error(
          '🚨 Failed to update hex terrain shader uniforms:',
          error
        );
      }
    }

    // Update animated water shader uniforms
    const animatedWaterMaterial = shadersRef.current.get('animated-water');
    if (animatedWaterMaterial?.uniforms) {
      if (animatedWaterMaterial.uniforms.u_time) {
        animatedWaterMaterial.uniforms.u_time.value = timeRef.current;
      }
      // Note: Camera position is automatically provided by Three.js for water shader
      animatedWaterMaterial.uniformsNeedUpdate = true;
    }

    // Update postprocessing and other shaders
    const postprocessingShaders = [
      'volumetric-fog',
      'hdr-tonemapping',
      'bloom',
      'fxaa',
      'ssao',
      'depth-of-field',
      'color-correction',
      'ssr',
      'taa',
      'motion-blur',
    ];

    shadersRef.current.forEach((material, name) => {
      if (name !== 'hex-terrain' && name !== 'animated-water') {
        let needsUpdate = false;

        // Update time for all shaders that have it
        if (material.uniforms?.u_time) {
          material.uniforms.u_time.value = timeRef.current;
          needsUpdate = true;
        }

        // Note: Camera position is automatically provided by Three.js for shaders that need it
        // No need to manually update cameraPosition uniform

        // Update resolution for postprocessing shaders
        if (
          postprocessingShaders.includes(name) &&
          material.uniforms?.u_resolution?.value
        ) {
          const { width, height } = useRenderStore.getState().viewport;
          const resolutionUniform = material.uniforms.u_resolution;
          if (
            resolutionUniform?.value &&
            typeof resolutionUniform.value === 'object' &&
            'set' in resolutionUniform.value
          ) {
            (
              resolutionUniform.value as { set: (x: number, y: number) => void }
            ).set(width, height);
            needsUpdate = true;
          }
        }

        if (needsUpdate) {
          material.uniformsNeedUpdate = true;
        }
      }
    });
  });

  /**
   * Handle quality changes
   */
  useEffect(() => {
    if (!isReady) return;

    const hexTerrainMaterial = shadersRef.current.get('hex-terrain');
    if (hexTerrainMaterial) {
      // Update quality-based defines
      const qualityLevel =
        quality.level === 'low'
          ? 1
          : quality.level === 'medium'
            ? 2
            : quality.level === 'high'
              ? 3
              : 4;
      hexTerrainMaterial.defines.QUALITY_LEVEL = qualityLevel;
      hexTerrainMaterial.needsUpdate = true;
    }
  }, [quality.level, isReady]);

  /**
   * Context API methods
   */
  const getShader = (name: ShaderName): ShaderMaterial | null => {
    return shadersRef.current.get(name) ?? null;
  };

  const updateShaderUniformsContext = (
    name: ShaderName,
    uniforms: Partial<ShaderUniforms>
  ): void => {
    shaderManager.updateUniforms(name, uniforms);
  };

  const contextValue: ShaderContextType = {
    getShader,
    updateShaderUniforms: updateShaderUniformsContext,
    isReady,
  };

  return (
    <ShaderContext.Provider value={contextValue}>
      {children}
    </ShaderContext.Provider>
  );
};
