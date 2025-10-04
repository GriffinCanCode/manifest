/**
 * Rendering Provider Component
 * Consolidates material, texture, and uniform management following provider patterns
 */

import React, { useCallback, useEffect, useRef, useState } from 'react';

import { materialService } from '../../../services/materials';
import { textureService } from '../../../services/texture-factory-service';
import { uniformService } from '../../../services/uniforms';
import type { TerrainType } from '../../../utils/game-types';
import {
  RenderingContext,
  type MaterialConfig,
  type RenderingContextType,
  type RenderingStats,
} from '../contexts/rendering-contexts';

// useRendering hook is exported from rendering-hooks.tsx

interface RenderingProviderProps {
  children: React.ReactNode;
  autoGenerate?: boolean;
  updateInterval?: number;
}

export const RenderingProvider: React.FC<RenderingProviderProps> = ({
  children,
  autoGenerate = true,
  updateInterval = 5000,
}) => {
  const [isReady, setIsReady] = useState(false);
  const [isGenerating, setIsGenerating] = useState(false);
  const [stats, setStats] = useState<RenderingStats>({
    materials: {
      cached: 0,
      compiled: 0,
      textured: 0,
      fallback: 0,
      cacheSize: 0,
    },
    textures: {
      texturesLoaded: 0,
      materialsCreated: 0,
      cacheSize: 0,
    },
    uniforms: {
      registered: 0,
      updated: 0,
      skipped: 0,
      errors: 0,
      activeCount: 0,
    },
  });

  const initializationRef = useRef<Promise<void> | null>(null);

  const updateStats = useCallback((): void => {
    setStats({
      materials: materialService.getStats(),
      textures: textureService.getStats(),
      uniforms: uniformService.getStats(),
    });
  }, []);

  const initializeServices = useCallback((): void => {
    try {
      console.warn('🎨 Initializing rendering services...');
      setIsGenerating(true);

      if (autoGenerate) {
        textureService.initialize();
      }

      setIsReady(true);
      updateStats();

      console.warn('✅ Rendering services initialized successfully');
    } catch (error) {
      console.error('❌ Failed to initialize rendering services:', error);
      // Continue anyway - we can still use fallback rendering
      setIsReady(true);
    } finally {
      setIsGenerating(false);
    }
  }, [autoGenerate, updateStats]);

  // Initialize services
  useEffect(() => {
    if (initializationRef.current) {
      return;
    }

    initializeServices();
  }, [initializeServices]);

  const getMaterial = useCallback(
    (config: MaterialConfig) => {
      const material = materialService.getMaterial(config);

      // Register material for uniform updates if it's a shader material
      if (material.type === 'ShaderMaterial') {
        const materialId = `${config.terrainType}_${Date.now()}`;
        uniformService.register(
          materialId,
          material as THREE.ShaderMaterial,
          'normal'
        );
      }

      updateStats();
      return material;
    },
    [updateStats]
  );

  const getTerrainMaterial = useCallback(
    (terrainType: TerrainType, texture?: THREE.Texture, wireframe = false) => {
      return getMaterial({
        terrainType,
        texture,
        useShader: true,
        wireframe,
      });
    },
    [getMaterial]
  );

  const generateTextures = useCallback(async (): Promise<void> => {
    try {
      setIsGenerating(true);
      await textureService.generateTextures({
        resolution: 512,
        generate_normals: true,
        generate_materials: true,
        generate_atlases: true,
      });
      updateStats();
    } catch (error) {
      console.error('Failed to generate textures:', error);
      throw error;
    } finally {
      setIsGenerating(false);
    }
  }, [updateStats]);

  const clearCache = useCallback((): void => {
    materialService.clearCache();
    textureService.clearCache();
    uniformService.clear();
    updateStats();
  }, [updateStats]);

  // Periodic cleanup and stats update
  useEffect(() => {
    if (!isReady) return;

    const interval = setInterval(() => {
      materialService.cleanup();
      uniformService.cleanup();
      updateStats();
    }, updateInterval);

    return () => clearInterval(interval);
  }, [isReady, updateInterval, updateStats]);

  const contextValue: RenderingContextType = {
    materialService,
    textureService,
    uniformService,
    isReady,
    isGenerating,
    stats,
    getMaterial,
    getTerrainMaterial,
    generateTextures,
    clearCache,
  };

  return (
    <RenderingContext.Provider value={contextValue}>
      {children}
    </RenderingContext.Provider>
  );
};

export default RenderingProvider;
