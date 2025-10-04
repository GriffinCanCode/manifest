/**
 * Rendering hooks - consolidated material, texture, and uniform management
 * Following existing hook patterns with strong typing and memoization
 */

import React, { useContext } from 'react';
import type * as THREE from 'three';

import { useRenderStore } from '../../../stores/render-store';
import type { TerrainType } from '../../../utils/game-types';
import {
  RenderingContext,
  type RenderingContextType,
} from '../contexts/rendering-contexts';

/**
 * Hook to access consolidated rendering context
 */
export const useRendering = (): RenderingContextType => {
  const context = useContext(RenderingContext);
  if (!context) {
    throw new Error('useRendering must be used within a RenderingProvider');
  }
  return context;
};

/**
 * Hook for getting terrain-specific material
 */
export const useTerrainMaterial = (
  terrainType: TerrainType,
  texture?: THREE.Texture,
  wireframe?: boolean
): THREE.Material => {
  const { getTerrainMaterial } = useRendering();
  return React.useMemo(
    () => getTerrainMaterial(terrainType, texture, wireframe),
    [getTerrainMaterial, terrainType, texture, wireframe]
  );
};

/**
 * Hook for bulk material creation with coordinated uniform updates
 */
export const useTileMaterials = (
  terrainTypes: TerrainType[],
  textures?: Map<TerrainType, THREE.Texture>,
  wireframe = false
): Map<TerrainType, THREE.Material> => {
  const { getTerrainMaterial } = useRendering();

  return React.useMemo(() => {
    const materials = new Map<TerrainType, THREE.Material>();

    terrainTypes.forEach(terrainType => {
      const texture = textures?.get(terrainType);
      const material = getTerrainMaterial(terrainType, texture, wireframe);
      materials.set(terrainType, material);
    });

    return materials;
  }, [terrainTypes, textures, wireframe, getTerrainMaterial]);
};

/**
 * Hook for unified material approach with single shader
 */
export const useUnifiedMaterial = (
  terrainTypes: TerrainType[],
  texture?: THREE.Texture
): THREE.Material => {
  const { getMaterial } = useRendering();

  return React.useMemo(() => {
    // Use the first terrain type as base for unified shader
    const baseTerrainType = terrainTypes[0] || 'grassland';

    return getMaterial({
      terrainType: baseTerrainType,
      texture,
      useShader: true,
      wireframe: false,
    });
  }, [terrainTypes, texture, getMaterial]);
};

/**
 * Development helper component for rendering statistics
 */
export const RenderingDebugInfo: React.FC = () => {
  const { stats, isReady } = useRendering();
  const { devMode } = useRenderStore();

  if (!devMode || !isReady) return null;

  return (
    <div
      style={{
        position: 'fixed',
        top: 10,
        right: 10,
        background: 'rgba(0,0,0,0.8)',
        color: 'white',
        padding: '12px',
        fontSize: '11px',
        fontFamily: 'monospace',
        borderRadius: '6px',
        zIndex: 1000,
        minWidth: '200px',
      }}
    >
      <div
        style={{ fontWeight: 'bold', marginBottom: '8px', color: '#4ade80' }}
      >
        🎨 Rendering System
      </div>

      <div style={{ marginBottom: '6px' }}>
        <div style={{ color: '#60a5fa' }}>📦 Materials</div>
        <div>
          Cached: {stats.materials.cached} | Size: {stats.materials.cacheSize}
        </div>
        <div>
          Compiled: {stats.materials.compiled} | Textured:{' '}
          {stats.materials.textured}
        </div>
        <div>Fallback: {stats.materials.fallback}</div>
      </div>

      <div style={{ marginBottom: '6px' }}>
        <div style={{ color: '#f59e0b' }}>🖼️ Textures</div>
        <div>Loaded: {stats.textures.texturesLoaded}</div>
        <div>Materials: {stats.textures.materialsCreated}</div>
      </div>

      <div>
        <div style={{ color: '#ec4899' }}>⚡ Uniforms</div>
        <div>
          Active: {stats.uniforms.activeCount}/{stats.uniforms.registered}
        </div>
        <div>
          Updated: {stats.uniforms.updated} | Errors: {stats.uniforms.errors}
        </div>
      </div>
    </div>
  );
};
