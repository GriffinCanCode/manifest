/**
 * HexWaterRenderer
 * Specialized renderer for animated ocean tiles using custom water shaders
 * Provides realistic water animation with waves, foam, and transparency
 */

import { useFrame } from '@react-three/fiber';
import { useCallback, useEffect, useMemo, useRef } from 'react';
import {
  CylinderGeometry,
  DynamicDrawUsage,
  Matrix4,
  MeshLambertMaterial,
  Vector3,
  type InstancedMesh,
  type ShaderMaterial,
} from 'three';

import { useRenderStore } from '../../../../stores/render-store';
import {
  HexUtils,
  TerrainType,
  type GameTile,
} from '../../../../utils/game-types';
import { useShader } from '../../hooks/shader-hooks';

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
  waveHeight = 0.2,
  waveSpeed = 1.0,
  transparency = 0.8,
}) => {
  const { quality, debug } = useRenderStore();
  const animatedWaterShader = useShader('animated-water');
  const meshRef = useRef<InstancedMesh>(null);
  const waterTilesRef = useRef<GameTile[]>([]);

  // Filter water tiles
  const waterTiles = useMemo(
    () => tiles.filter(tile => tile.terrain === TerrainType.Ocean),
    [tiles]
  );

  // Water shader material
  const waterMaterial = useMemo(() => {
    if (!animatedWaterShader) {
      // Fallback to basic material if shader not ready
      return new MeshLambertMaterial({
        color: 0x4a90e2,
        transparent: true,
        opacity: transparency,
        wireframe: debug.showWireframe,
      });
    }

    // Clone the shader material and update uniforms
    const material = animatedWaterShader.clone();

    // Update custom uniforms (safely check if they exist)
    const { uniforms } = material;
    if (uniforms && typeof uniforms === 'object') {
      if ('u_waveHeight' in uniforms && uniforms.u_waveHeight) {
        uniforms.u_waveHeight.value = waveHeight;
      }
      if ('u_waveSpeed' in uniforms && uniforms.u_waveSpeed) {
        uniforms.u_waveSpeed.value = waveSpeed;
      }
      if ('u_transparency' in uniforms && uniforms.u_transparency) {
        uniforms.u_transparency.value = transparency;
      }
    }

    // Update wireframe mode
    material.wireframe = debug.showWireframe;

    return material;
  }, [
    animatedWaterShader,
    waveHeight,
    waveSpeed,
    transparency,
    debug.showWireframe,
  ]);

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

  // Update uniforms on each frame
  useFrame(({ clock, camera: frameCamera }) => {
    const mesh = meshRef.current;
    if (!mesh || !animatedWaterShader) return;

    const material = mesh.material as ShaderMaterial;
    if (!material?.uniforms) return;

    // Update time for animation
    if (material.uniforms.u_time) {
      material.uniforms.u_time.value = clock.elapsedTime;
    }

    // Update camera position
    if (material.uniforms.u_cameraPosition?.value instanceof Vector3) {
      material.uniforms.u_cameraPosition.value.copy(frameCamera.position);
    }

    // Quality-based animation adjustments
    const qualityMultiplier = quality.level === 'low' ? 0.5 : 1.0;
    if (material.uniforms.u_waveHeight) {
      material.uniforms.u_waveHeight.value = waveHeight * qualityMultiplier;
    }
    if (material.uniforms.u_waveSpeed) {
      material.uniforms.u_waveSpeed.value = waveSpeed * qualityMultiplier;
    }

    // LOD adjustments
    const cameraDistance = frameCamera.position.length();
    const lodFactor = Math.min(1.0, 50.0 / cameraDistance);
    if (material.uniforms.u_waveHeight) {
      material.uniforms.u_waveHeight.value *= lodFactor;
    }
  });

  // Update instances when tiles change
  useEffect(() => {
    updateInstances();
  }, [updateInstances]);

  // Don't render if no water tiles or material not ready
  if (waterTiles.length === 0 || !waterMaterial) {
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
    return new MeshLambertMaterial({
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
