/**
 * Shader Provider Component
 * Integrates GLSL shader system with Three.js and React ecosystem
 */

import { useFrame, useThree } from '@react-three/fiber';
import React, { useEffect, useRef } from 'react';
import { Vector3, type Color, type ShaderMaterial } from 'three';

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
import { ShaderContext } from '../../hooks/shader-hooks';

interface ShaderContextType {
  getShader: (name: ShaderName) => ShaderMaterial | null;
  updateShaderUniforms: (
    name: ShaderName,
    uniforms: Partial<ShaderUniforms>
  ) => void;
  isReady: boolean;
}

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
  const isReadyRef = useRef<boolean>(false);

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
              USE_INSTANCING: capabilities.supportsInstancing ? 1 : 0,
              USE_HDR: capabilities.supportsHDR ? 1 : 0,
            },
          }
        );

        shadersRef.current.set('hex-terrain', hexTerrainMaterial);

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

        isReadyRef.current = true;
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
   */
  useFrame((_state, delta) => {
    if (!isReadyRef.current) return;

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
      updateShaderUniforms(
        hexTerrainMaterial.uniforms as TerrainShaderUniforms,
        timeRef.current,
        cameraPosition,
        qualityLevel
      );

      // Update performance-based settings
      hexTerrainMaterial.uniforms.u_wireframe.value =
        useRenderStore.getState().debug.showWireframe;

      // Update fog color based on time of day (future enhancement)
      if (process.env.NODE_ENV === 'development') {
        const dayFactor = Math.sin(timeRef.current * 0.1) * 0.5 + 0.5;
        const fogColor = hexTerrainMaterial.uniforms.u_fogColor.value as Color;
        if (fogColor && typeof fogColor.setHSL === 'function') {
          fogColor.setHSL(0.55, 0.3, 0.7 + dayFactor * 0.2);
        }
      }

      hexTerrainMaterial.uniformsNeedUpdate = true;
    }

    // Update other shaders as needed
    shadersRef.current.forEach((material, name) => {
      if (name !== 'hex-terrain' && material.uniforms?.u_time) {
        material.uniforms.u_time.value = timeRef.current;
        material.uniformsNeedUpdate = true;
      }
    });
  });

  /**
   * Handle quality changes
   */
  useEffect(() => {
    if (!isReadyRef.current) return;

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
  }, [quality.level]);

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
    isReady: isReadyRef.current,
  };

  return (
    <ShaderContext.Provider value={contextValue}>
      {children}
    </ShaderContext.Provider>
  );
};
