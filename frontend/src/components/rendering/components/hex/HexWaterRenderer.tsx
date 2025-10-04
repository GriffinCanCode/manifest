/**
 * HexWaterRenderer
 * Specialized renderer for animated ocean tiles using custom water shaders
 * Provides realistic water animation with waves, foam, and transparency
 */

import { useFrame } from '@react-three/fiber';
import React, { useCallback, useEffect, useMemo, useRef } from 'react';
import {
  CylinderGeometry,
  DynamicDrawUsage,
  Matrix4,
  MeshBasicMaterial,
  Vector3,
  type InstancedMesh,
} from 'three';

import { useRenderStore } from '../../../../stores/render-store';
import {
  HexUtils,
  TerrainType,
  type GameTile,
} from '../../../../utils/game-types';
import { useRendering } from '../../hooks/rendering-hooks';

// Import shaders

interface HexWaterRendererProps {
  readonly tiles: readonly GameTile[];
  readonly maxInstances?: number;
  readonly waveHeight?: number;
  readonly waveSpeed?: number;
  readonly transparency?: number;
}

/**
 * High-performance animated water renderer for ocean hex tiles
 * Uses custom shaders for realistic water effects
 */
export const HexWaterRenderer: React.FC<HexWaterRendererProps> = ({
  tiles,
  maxInstances = 5000,
  waveHeight: _waveHeight = 0.2,
  waveSpeed: _waveSpeed = 1.0,
  transparency = 0.8,
}) => {
  // ALL HOOKS FIRST - Required by React Rules of Hooks
  const { quality, debug } = useRenderStore();
  const renderingContext = useRendering();
  const meshRef = useRef<InstancedMesh>(null);
  const waterTilesRef = useRef<GameTile[]>([]);

  // Filter water tiles
  const waterTiles = useMemo(
    () => tiles.filter(tile => tile.terrain === TerrainType.Ocean),
    [tiles]
  );

  // Get animated water shader material
  const animatedWaterShader = useMemo(() => {
    if (!renderingContext?.materialService) {
      return null;
    }
    try {
      return renderingContext.materialService.getMaterial({
        terrainType: TerrainType.Ocean,
        useShader: true,
        wireframe: debug.showWireframe,
      });
    } catch {
      return null;
    }
  }, [renderingContext?.materialService, debug.showWireframe]);

  // Water shader material
  const waterMaterial = useMemo(() => {
    if (!animatedWaterShader) {
      // Fallback to basic material if shader not ready - use MeshBasicMaterial to avoid GeometricContext issues
      return new MeshBasicMaterial({
        color: 0x4a90e2,
        transparent: true,
        opacity: transparency,
        wireframe: debug.showWireframe,
      });
    }

    // Use the shader material directly - uniforms are managed by RenderingProvider
    // Clone is handled by material service internally
    return animatedWaterShader;
  }, [animatedWaterShader, transparency, debug.showWireframe]);

  // Create hex geometry based on quality
  const hexGeometry = useMemo(() => {
    const detail =
      quality.level === 'low' ? 6 : quality.level === 'medium' ? 8 : 12;
    return new CylinderGeometry(1, 1, 0.1, detail);
  }, [quality.level]);

  // Update instance matrices
  const updateInstances = useCallback(() => {
    const mesh = meshRef.current;
    if (!mesh || !waterTiles.length) return;

    const matrix = new Matrix4();
    const visibleTiles = waterTiles.slice(0, maxInstances);

    // Ensure we have enough instances
    if (mesh.count < visibleTiles.length) {
      mesh.instanceMatrix.setUsage(DynamicDrawUsage);
      mesh.count = Math.min(visibleTiles.length, maxInstances);
    }

    for (let i = 0; i < visibleTiles.length; i++) {
      const tile = visibleTiles[i];
      const [x, z] = HexUtils.hexToPixel(tile.hex);
      const y = tile.elevation * 0.5;

      // Create transformation matrix
      matrix.makeTranslation(x, y, z);
      matrix.scale(new Vector3(1, Math.max(0.1, Math.abs(tile.elevation)), 1));

      mesh.setMatrixAt(i, matrix);
    }

    mesh.instanceMatrix.needsUpdate = true;
    waterTilesRef.current = visibleTiles;
  }, [waterTiles, maxInstances]);

  // Update instances when tiles change
  useEffect(() => {
    updateInstances();
  }, [updateInstances]);

  // Don't render if no rendering context, water tiles, or material not ready
  if (!renderingContext || waterTiles.length === 0 || !waterMaterial) {
    return null;
  }

  return (
    <instancedMesh
      ref={meshRef}
      args={[
        hexGeometry,
        waterMaterial,
        Math.min(waterTiles.length, maxInstances),
      ]}
    />
  );
};

/**
 * Simplified water renderer for low-end devices
 * Uses basic animated materials without custom shaders
 */
export const SimpleHexWaterRenderer: React.FC<{
  tiles: readonly GameTile[];
  maxInstances?: number;
}> = ({ tiles, maxInstances = 2000 }) => {
  const meshRef = useRef<InstancedMesh>(null);

  const waterTiles = useMemo(
    () => tiles.filter(tile => tile.terrain === TerrainType.Ocean),
    [tiles]
  );

  const hexGeometry = useMemo(() => {
    return new CylinderGeometry(1, 1, 0.1, 6);
  }, []);

  const simpleMaterial = useMemo(() => {
    return new MeshBasicMaterial({
      color: 0x4a90e2,
      transparent: true,
      opacity: 0.8,
    });
  }, []);

  useFrame(({ clock }) => {
    const mesh = meshRef.current;
    if (!mesh) return;

    // Simple bobbing animation
    const time = clock.elapsedTime;
    mesh.rotation.y = Math.sin(time * 0.5) * 0.02;
    mesh.position.y = Math.sin(time * 2) * 0.05;
  });

  const updateInstances = useCallback(() => {
    const mesh = meshRef.current;
    if (!mesh || !waterTiles.length) return;

    const matrix = new Matrix4();
    const visibleTiles = waterTiles.slice(0, maxInstances);

    mesh.count = Math.min(visibleTiles.length, maxInstances);

    for (let i = 0; i < visibleTiles.length; i++) {
      const tile = visibleTiles[i];
      const [x, z] = HexUtils.hexToPixel(tile.hex);
      const y = tile.elevation * 0.5;

      matrix.makeTranslation(x, y, z);
      mesh.setMatrixAt(i, matrix);
    }

    mesh.instanceMatrix.needsUpdate = true;
  }, [waterTiles, maxInstances]);

  useEffect(() => {
    updateInstances();
  }, [updateInstances]);

  if (waterTiles.length === 0) {
    return null;
  }

  return (
    <instancedMesh
      ref={meshRef}
      args={[
        hexGeometry,
        simpleMaterial,
        Math.min(waterTiles.length, maxInstances),
      ]}
    />
  );
};

/**
 * Adaptive water renderer that chooses implementation based on device capabilities
 */
export const AdaptiveHexWaterRenderer: React.FC<
  HexWaterRendererProps
> = props => {
  const { capabilities, quality } = useRenderStore();

  // Use simple renderer for low-end devices
  if (quality.level === 'low' || !capabilities?.supportsWebGL2) {
    return <SimpleHexWaterRenderer {...props} />;
  }

  // Use full featured water renderer for capable devices
  return <HexWaterRenderer {...props} />;
};
