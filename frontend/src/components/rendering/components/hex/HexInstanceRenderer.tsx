/**
 * HexInstanceRenderer
 * Efficient hex tile rendering using instanced meshes with unified material system
 * Now uses centralized MaterialService for all material management
 */

import React, { useCallback, useEffect, useMemo, useRef } from 'react';
import * as THREE from 'three';

import { useRenderStore } from '../../../../stores/render-store';
import {
  HexUtils,
  TerrainType,
  type GameTile,
} from '../../../../utils/game-types';
import {
  DEFAULT_STREAMING_CONFIG,
  InstanceDataStreamer,
  type InstanceDataStreamerOptions,
} from '../../../../utils/instance-data-streamer';
import { LODLevel, calculateLODLevel } from '../../../../utils/lod';
import { throttledLog } from '../../../../utils/throttled-logger';
import { useHexRendererDebug } from '../../../../utils/tile-debug-hook';
import { useRendering } from '../../hooks/rendering-hooks';

export interface HexInstanceRendererProps {
  readonly tiles: readonly GameTile[];
  readonly onTileClick?: (tile: GameTile) => void;
  readonly selectedTileId?: number;
  readonly highlightedTiles?: ReadonlySet<number>;
  readonly maxInstances?: number;
  readonly enableSpatialQueries?: boolean;
  readonly enableStreaming?: boolean;
  readonly streamingConfig?: Partial<InstanceDataStreamerOptions>;
  readonly isLoading?: boolean;
  readonly cameraPosition?: THREE.Vector3;
}

/**
 * Optimized hex tile renderer using unified material system
 */
export const HexInstanceRenderer: React.FC<HexInstanceRendererProps> = ({
  tiles,
  onTileClick,
  selectedTileId: _selectedTileId,
  highlightedTiles: _highlightedTiles = new Set(),
  maxInstances = 25000,
  enableSpatialQueries: _enableSpatialQueries = true,
  enableStreaming = true,
  streamingConfig,
  isLoading = false,
  cameraPosition,
}) => {
  // ALL HOOKS FIRST - Required by React Rules of Hooks
  const { debug, camera } = useRenderStore();
  const renderingContext = useRendering();
  useHexRendererDebug(tiles, isLoading);
  const streamerRef = useRef<InstanceDataStreamer | null>(null);
  const instancedMeshRef = useRef<THREE.InstancedMesh | null>(null);

  // Conditional validation after all hooks
  if (!renderingContext) {
    return null;
  }

  const {
    isReady: isRenderingReady,
    textureService,
    getTerrainMaterial,
  } = renderingContext;

  const texturesReady =
    isRenderingReady &&
    textureService &&
    typeof textureService.getStats === 'function' &&
    textureService.getStats().texturesLoaded > 0;

  // Get unique terrain types from tiles
  const terrainTypes = useMemo(() => {
    const types = new Set<TerrainType>();
    tiles.forEach(tile => types.add(tile.terrain));
    return Array.from(types);
  }, [tiles]);

  // Get textures for each terrain type
  const terrainTextures = useMemo(() => {
    if (
      !texturesReady ||
      !textureService ||
      typeof textureService.getTexture !== 'function'
    ) {
      return undefined;
    }

    const textures = new Map<TerrainType, THREE.Texture>();
    terrainTypes.forEach(terrain => {
      const biomeMap = {
        [TerrainType.Ocean]: 'ocean',
        [TerrainType.Grassland]: 'grassland',
        [TerrainType.Plains]: 'plains',
        [TerrainType.Desert]: 'desert',
        [TerrainType.Tundra]: 'tundra',
        [TerrainType.Snow]: 'snow',
        [TerrainType.Forest]: 'forest',
        [TerrainType.Jungle]: 'jungle',
        [TerrainType.Hills]: 'hills',
        [TerrainType.Mountain]: 'mountain',
      };

      try {
        const texture = textureService.getTexture(`biome_${biomeMap[terrain]}`);
        if (texture) {
          textures.set(terrain, texture);
        }
      } catch (error) {
        console.warn(`Failed to get texture for ${terrain}:`, error);
      }
    });

    return textures;
  }, [terrainTypes, texturesReady, textureService]);

  // Materials will be accessed through getTerrainMaterial as needed

  // Create hex geometry
  const hexGeometry = useMemo(() => {
    const geometry = new THREE.CylinderGeometry(
      1.2, // radiusTop
      1.2, // radiusBottom
      0.3, // height
      6, // radialSegments - hexagon
      1, // heightSegments
      false // openEnded
    );

    // Rotate to lay flat and orient correctly (pointy-top)
    geometry.rotateX(-Math.PI / 2);
    geometry.rotateY(Math.PI / 6);

    return geometry;
  }, []);

  // Initialize instanced attributes for shader
  const initializeInstancedMesh = useCallback(
    (mesh: THREE.InstancedMesh | null) => {
      if (!mesh) return;

      const { geometry } = mesh;

      // Set up instanced attributes required by shader
      geometry.setAttribute(
        'instancePosition',
        new THREE.InstancedBufferAttribute(
          new Float32Array(maxInstances * 3),
          3
        )
      );

      geometry.setAttribute(
        'instanceColor',
        new THREE.InstancedBufferAttribute(
          new Float32Array(maxInstances * 3),
          3
        )
      );

      geometry.setAttribute(
        'instanceHeight',
        new THREE.InstancedBufferAttribute(new Float32Array(maxInstances), 1)
      );

      geometry.setAttribute(
        'instanceBiome',
        new THREE.InstancedBufferAttribute(new Float32Array(maxInstances), 1)
      );

      geometry.setAttribute(
        'instanceTexCoords',
        new THREE.InstancedBufferAttribute(
          new Float32Array(maxInstances * 2),
          2
        )
      );

      geometry.setAttribute(
        'instanceResourceMask',
        new THREE.InstancedBufferAttribute(new Float32Array(maxInstances), 1)
      );

      mesh.visible = true;
      mesh.frustumCulled = false;
    },
    [maxInstances]
  );

  // Set mesh reference callback
  const setInstancedMeshRef = useCallback(
    (mesh: THREE.InstancedMesh | null) => {
      instancedMeshRef.current = mesh;
      initializeInstancedMesh(mesh);
    },
    [initializeInstancedMesh]
  );

  // Update instances with tile data
  const updateInstances = useCallback(() => {
    const instancedMesh = instancedMeshRef.current;
    if (!instancedMesh || tiles.length === 0) return;

    const targetCount = Math.min(tiles.length, maxInstances);
    if (instancedMesh.count !== targetCount) {
      instancedMesh.count = targetCount;
    }

    const matrix = new THREE.Matrix4();
    const color = new THREE.Color();
    const cameraHex = cameraPosition
      ? HexUtils.pixelToHex(cameraPosition.x, cameraPosition.z)
      : { q: 0, r: 0 };
    const cameraZoom = camera.zoom || 1.0;

    // Terrain color mapping
    const terrainColors = {
      [TerrainType.Ocean]: '#0066ff',
      [TerrainType.Grassland]: '#00ff44',
      [TerrainType.Plains]: '#88ff00',
      [TerrainType.Desert]: '#ffdd00',
      [TerrainType.Tundra]: '#cccccc',
      [TerrainType.Snow]: '#ffffff',
      [TerrainType.Forest]: '#006622',
      [TerrainType.Jungle]: '#00aa33',
      [TerrainType.Hills]: '#996633',
      [TerrainType.Mountain]: '#666666',
    };

    for (let i = 0; i < Math.min(tiles.length, maxInstances); i++) {
      const tile = tiles[i];
      const [worldX, worldZ] = HexUtils.hexToPixel(tile.hex);
      const y = tile.elevation * 0.5;

      const lodLevel = calculateLODLevel(cameraHex, tile.hex, cameraZoom);

      // LOD-based scaling
      let scale = 1.0;
      switch (lodLevel) {
        case LODLevel.HIGH:
          scale = 1.0;
          break;
        case LODLevel.MEDIUM:
          scale = 0.8;
          break;
        case LODLevel.LOW:
          scale = 0.6;
          break;
        case LODLevel.CULLED:
          scale = 0.0;
          break;
      }

      // Set matrix
      matrix.makeTranslation(worldX, y, worldZ);
      matrix.scale(new THREE.Vector3(scale, scale, scale));
      instancedMesh.setMatrixAt(i, matrix);

      // Update instanced attributes
      const { geometry } = instancedMesh;

      // Position
      if (geometry.attributes.instancePosition) {
        const posArray = geometry.attributes.instancePosition
          .array as Float32Array;
        posArray[i * 3] = worldX;
        posArray[i * 3 + 1] = y;
        posArray[i * 3 + 2] = worldZ;
      }

      // Color
      if (geometry.attributes.instanceColor) {
        const colorHex = terrainColors[tile.terrain] || '#64748b';
        color.set(colorHex);

        // LOD-based color adjustments
        if (lodLevel === LODLevel.MEDIUM) {
          color.lerp(new THREE.Color('#888888'), 0.2);
        } else if (lodLevel === LODLevel.LOW) {
          color.lerp(new THREE.Color('#666666'), 0.4);
        } else if (lodLevel === LODLevel.CULLED) {
          color.set('#000000');
        }

        const colorArray = geometry.attributes.instanceColor
          .array as Float32Array;
        colorArray[i * 3] = color.r;
        colorArray[i * 3 + 1] = color.g;
        colorArray[i * 3 + 2] = color.b;
      }

      // Height
      if (geometry.attributes.instanceHeight) {
        const heightArray = geometry.attributes.instanceHeight
          .array as Float32Array;
        heightArray[i] = tile.elevation;
      }

      // Biome
      if (geometry.attributes.instanceBiome) {
        const biomeArray = geometry.attributes.instanceBiome
          .array as Float32Array;
        const biomeMap: Record<string, number> = {
          ocean: 0,
          grassland: 1,
          plains: 2,
          desert: 3,
          tundra: 4,
          snow: 5,
          forest: 6,
          jungle: 7,
          hills: 8,
          mountain: 9,
        };
        const biomeValue = biomeMap[tile.terrain] ?? 0;
        biomeArray[i] = biomeValue / 9.0;
      }

      // Texture coords
      if (geometry.attributes.instanceTexCoords) {
        const texArray = geometry.attributes.instanceTexCoords
          .array as Float32Array;
        texArray[i * 2] = tile.hex.q / 100.0;
        texArray[i * 2 + 1] = tile.hex.r / 100.0;
      }

      // Resource mask
      if (geometry.attributes.instanceResourceMask) {
        const resourceArray = geometry.attributes.instanceResourceMask
          .array as Float32Array;
        const resourceValue =
          lodLevel === LODLevel.HIGH ? (tile.resourceMask ?? 0) : 0;
        resourceArray[i] = resourceValue;
      }
    }

    // Mark all attributes for update
    instancedMesh.instanceMatrix.needsUpdate = true;
    const { geometry } = instancedMesh;

    Object.keys(geometry.attributes).forEach(key => {
      if (key.startsWith('instance')) {
        geometry.attributes[key].needsUpdate = true;
      }
    });

    instancedMesh.visible = true;
    instancedMesh.frustumCulled = false;
    instancedMesh.matrixAutoUpdate = false;
  }, [tiles, maxInstances, cameraPosition, camera.zoom]);

  // Handle click events
  const handleClick = useCallback(
    (_event: THREE.Event) => {
      if (onTileClick) {
        // TODO: Implement proper raycasting
      }
    },
    [onTileClick]
  );

  // Streaming configuration
  const finalStreamingConfig = useMemo(
    () => ({
      ...DEFAULT_STREAMING_CONFIG,
      ...streamingConfig,
      maxInstances,
    }),
    [streamingConfig, maxInstances]
  );

  // Initialize streamer
  const initializeStreamer = useCallback(() => {
    if (!enableStreaming) return () => {};

    const streamer = new InstanceDataStreamer(finalStreamingConfig);
    streamerRef.current = streamer;

    return () => {
      streamer.dispose();
      streamerRef.current = null;
    };
  }, [enableStreaming, finalStreamingConfig]);

  // Initialize on mount
  useEffect(() => {
    const cleanupStreamer = initializeStreamer();
    return cleanupStreamer;
  }, [initializeStreamer]);

  // Update instances when tiles change
  useEffect(() => {
    if (tiles.length > 0) {
      updateInstances();
    }
  }, [tiles, updateInstances]);

  // Material uniforms are now handled by UniformService - no need for manual updates

  // Don't render if not ready or no tiles
  if (!isRenderingReady || !getTerrainMaterial) {
    throttledLog(
      'hex-renderer-materials-wait',
      'warn',
      '⏳ HexRenderer: Waiting for materials...',
      [],
      5000
    );
    return null;
  }

  if (tiles.length === 0) {
    if (!isLoading) {
      throttledLog(
        'hex-renderer-no-tiles',
        'warn',
        '📭 HexRenderer: No tiles to render',
        [],
        10000
      );
    }
    return null;
  }

  // Choose rendering approach based on terrain diversity
  const useUnifiedApproach = terrainTypes.length > 3 || !texturesReady;

  if (useUnifiedApproach) {
    // Unified approach: single shader material handles all terrain types
    // getTerrainMaterial is guaranteed to exist at this point due to early return check
    const unifiedMaterial = getTerrainMaterial(
      terrainTypes[0] || TerrainType.Grassland,
      undefined,
      debug.showWireframe
    );

    throttledLog(
      'hex-renderer-unified',
      'log',
      `🎨 HexRenderer: Using unified approach for ${tiles.length} tiles`,
      [],
      30000
    );

    return (
      <group name='hex-renderer-group'>
        <instancedMesh
          ref={setInstancedMeshRef}
          args={[
            hexGeometry,
            unifiedMaterial,
            Math.max(1, Math.min(tiles.length, maxInstances)),
          ]}
          onClick={handleClick}
          castShadow
          receiveShadow
        />
      </group>
    );
  }

  // Multi-material approach: separate mesh per terrain type
  throttledLog(
    'hex-renderer-multi',
    'log',
    `🎨 HexRenderer: Using multi-material approach for ${terrainTypes.length} terrain types`,
    [],
    30000
  );

  return (
    <group name='hex-renderer-group'>
      {terrainTypes.map(terrainType => {
        const terrainTiles = tiles.filter(tile => tile.terrain === terrainType);
        if (terrainTiles.length === 0) return null;

        const texture = terrainTextures?.get(terrainType);
        // getTerrainMaterial is guaranteed to exist at this point due to early return check
        const material = getTerrainMaterial(
          terrainType,
          texture,
          debug.showWireframe
        );

        return (
          <instancedMesh
            key={`terrain-${terrainType}`}
            ref={mesh => {
              if (mesh && terrainTiles.length > 0) {
                const matrix = new THREE.Matrix4();
                terrainTiles.forEach((tile, index) => {
                  const [x, z] = HexUtils.hexToPixel(tile.hex);
                  const y = (tile.elevation || 0) * 0.5;
                  matrix.makeTranslation(x, y, z);
                  mesh.setMatrixAt(index, matrix);
                });
                mesh.instanceMatrix.needsUpdate = true;
                mesh.count = terrainTiles.length;
                mesh.visible = true;
                mesh.frustumCulled = false;
              }
            }}
            args={[
              hexGeometry,
              material,
              Math.min(terrainTiles.length, maxInstances),
            ]}
            onClick={handleClick}
            castShadow
            receiveShadow
          />
        );
      })}
    </group>
  );
};

export default HexInstanceRenderer;
