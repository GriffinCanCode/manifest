/**
 * HexInstanceRenderer
 * Efficient hex tile rendering using instanced meshes with BVH acceleration
 * Now includes per-instance data streaming for optimal performance
 * Integrates with existing game types and render store
 */

import { useFrame, useThree } from '@react-three/fiber';
import { useCallback, useEffect, useMemo, useRef } from 'react';
import {
  CylinderGeometry,
  Frustum,
  Matrix4,
  MeshStandardMaterial,
  Vector3,
} from 'three';

import { useRenderStore } from '../../../../stores/render-store';
import type {
  CullingBounds,
  InstancedBVHManagerOptions,
  InstancedRenderingEventHandler,
  SpatialUpdateEvent,
} from '../../../../types/instanced-rendering';
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
import { InstancedBVHManager } from '../../../../utils/instanced-bvh-manager';
import { useShader } from '../../hooks/shader-hooks';

export interface HexInstanceRendererProps {
  readonly tiles: readonly GameTile[];
  readonly onTileClick?: (tile: GameTile) => void;
  readonly selectedTileId?: number;
  readonly highlightedTiles?: ReadonlySet<number>;
  readonly maxInstances?: number;
  readonly enableSpatialQueries?: boolean;
  readonly enableStreaming?: boolean;
  readonly streamingConfig?: Partial<InstanceDataStreamerOptions>;
}

interface HexRenderData {
  readonly tile: GameTile;
  readonly instanceId: number;
  readonly terrainColor: string;
  readonly isVisible: boolean;
}

/**
 * Optimized hex tile renderer using instanced meshes and BVH
 * Now with integrated per-instance data streaming
 */
export const HexInstanceRenderer: React.FC<HexInstanceRendererProps> = ({
  tiles,
  onTileClick,
  selectedTileId: _selectedTileId,
  highlightedTiles: _highlightedTiles = new Set(),
  maxInstances = 10000,
  enableSpatialQueries = true,
  enableStreaming = true,
  streamingConfig,
}) => {
  const { camera } = useThree();
  const { quality, debug, culling } = useRenderStore();
  const hexTerrainShader = useShader('hex-terrain');
  const managerRef = useRef<InstancedBVHManager | null>(null);
  const streamerRef = useRef<InstanceDataStreamer | null>(null);
  const renderDataRef = useRef<Map<number, HexRenderData>>(new Map());
  const lastTileCountRef = useRef(0);
  const lastCameraPositionRef = useRef<Vector3>(new Vector3());

  // Terrain color mapping with quality-based variations
  const terrainColors = useMemo(
    () => ({
      [TerrainType.Ocean]: '#1e40af',
      [TerrainType.Grassland]: '#22c55e',
      [TerrainType.Plains]: '#84cc16',
      [TerrainType.Desert]: '#eab308',
      [TerrainType.Tundra]: '#64748b',
      [TerrainType.Snow]: '#f1f5f9',
      [TerrainType.Forest]: '#166534',
      [TerrainType.Jungle]: '#14532d',
      [TerrainType.Hills]: '#a3a3a3',
      [TerrainType.Mountain]: '#525252',
    }),
    []
  );

  // Create geometry based on quality settings
  const hexGeometry = useMemo(() => {
    const detail =
      quality.level === 'low' ? 6 : quality.level === 'medium' ? 8 : 12;
    return new CylinderGeometry(1, 1, 1, detail);
  }, [quality.level]);

  // SMART TEMPORARY FIX: Use basic material while fixing shader compilation
  const hexMaterial = useMemo(() => {
    // Always return a working material - never null!
    // This ensures HexInstanceRenderer can always render something

    if (!hexTerrainShader) {
      // Fallback to basic material if shader not ready
      console.warn(
        'HexInstanceRenderer: Using fallback MeshStandardMaterial (shader not ready)'
      );
      return new MeshStandardMaterial({
        color: 0x4caf50,
        wireframe: debug.showWireframe,
        roughness: 0.8,
        metalness: 0.1,
      });
    }

    // Try to use custom shader, but catch any errors
    try {
      const material = hexTerrainShader.clone();
      if (material.uniforms.u_wireframe) {
        material.uniforms.u_wireframe.value = debug.showWireframe;
      }
      return material;
    } catch (error) {
      console.warn(
        'HexInstanceRenderer: Shader failed, using fallback material',
        error
      );
      return new MeshStandardMaterial({
        color: 0x4caf50,
        wireframe: debug.showWireframe,
        roughness: 0.8,
        metalness: 0.1,
      });
    }
  }, [hexTerrainShader, debug.showWireframe]);

  // Streaming configuration
  const finalStreamingConfig = useMemo(
    () => ({
      ...DEFAULT_STREAMING_CONFIG,
      ...streamingConfig,
      maxInstances,
    }),
    [streamingConfig, maxInstances]
  );

  // Initialize instance data streamer
  const initializeStreamer = useCallback(() => {
    if (!enableStreaming) return () => {};

    const streamer = new InstanceDataStreamer(finalStreamingConfig);
    streamerRef.current = streamer;

    return () => {
      streamer.dispose();
      streamerRef.current = null;
    };
  }, [enableStreaming, finalStreamingConfig]);

  // Initialize instanced BVH manager
  const initializeManager = useCallback(() => {
    if (managerRef.current) {
      managerRef.current.dispose();
    }

    // Don't initialize if material isn't ready
    if (!hexMaterial) {
      console.warn(
        'HexInstanceRenderer: Shader material not ready, skipping initialization'
      );
      return () => {};
    }

    const options: InstancedBVHManagerOptions = {
      config: {
        geometry: hexGeometry,
        material: hexMaterial,
        maxInstances,
        enableBVH: enableSpatialQueries,
        enableFrustumCulling: culling.frustumCulling,
        enableLOD: quality.level !== 'low',
        lodLevels: [0.5, 1.0, 2.0],
      },
      autoUpdate: true,
      debugMode: debug.showStats,
    };

    managerRef.current = new InstancedBVHManager(options);

    // Add event listener for spatial updates
    const eventHandler: InstancedRenderingEventHandler = (
      event: SpatialUpdateEvent
    ) => {
      if (debug.showStats) {
        console.warn('Spatial event:', event);
      }
    };

    managerRef.current.addEventListener(eventHandler);

    return () => {
      managerRef.current?.removeEventListener(eventHandler);
    };
  }, [
    hexGeometry,
    hexMaterial,
    maxInstances,
    enableSpatialQueries,
    culling.frustumCulling,
    quality.level,
    debug.showStats,
  ]);

  // Update instances when tiles change
  const updateInstances = useCallback(() => {
    const manager = managerRef.current;
    if (!manager) return;

    const currentTileCount = tiles.length;
    const renderData = renderDataRef.current;

    // Clear existing render data if tile count changed significantly
    if (Math.abs(currentTileCount - lastTileCountRef.current) > 100) {
      renderData.clear();
      lastTileCountRef.current = currentTileCount;
    }

    // Process each tile
    for (const tile of tiles) {
      const existingData = renderData.get(tile.id);
      const [x, z] = HexUtils.hexToPixel(tile.hex);
      const position = new Vector3(x, tile.elevation * 0.5, z);
      const terrainColor = terrainColors[tile.terrain] || '#64748b';

      if (!existingData) {
        // Add new instance
        const instanceId = manager.addInstance({
          position,
          scale: new Vector3(1, Math.max(0.1, tile.elevation), 1),
          userData: { tile },
        });

        // Store render data
        const newRenderData: HexRenderData = {
          tile,
          instanceId,
          terrainColor,
          isVisible: true,
        };
        renderData.set(tile.id, newRenderData);
      } else {
        // Update existing instance position if needed
        const currentPos = existingData.tile.hex;
        if (currentPos.q !== tile.hex.q || currentPos.r !== tile.hex.r) {
          manager.updateInstancePosition(existingData.instanceId, position);

          // Update render data
          renderData.set(tile.id, {
            ...existingData,
            tile,
          });
        }
      }
    }

    // Remove instances for tiles that no longer exist
    const currentTileIds = new Set(tiles.map(t => t.id));
    for (const [tileId, tileRenderData] of renderData) {
      if (!currentTileIds.has(tileId)) {
        manager.removeInstance(tileRenderData.instanceId);
        renderDataRef.current.delete(tileId);
      }
    }
  }, [tiles, terrainColors]);

  // Handle click events using raycasting
  const handleClick = useCallback(
    (_event: React.MouseEvent) => {
      if (onTileClick && managerRef.current) {
        // TODO: Implement proper raycasting with BVH
        // This would use the BVH-accelerated raycasting for performance
        // For now, we'll use a simplified approach
        // Implementation would depend on the specific raycasting setup
      }
    },
    [onTileClick]
  );

  // Initialize systems on mount
  useEffect(() => {
    const cleanupManager = initializeManager();
    const cleanupStreamer = initializeStreamer();

    return () => {
      cleanupManager();
      cleanupStreamer();
    };
  }, [initializeManager, initializeStreamer]);

  // Update instances when tiles change (legacy fallback)
  useEffect(() => {
    if (!enableStreaming) {
      updateInstances();
    }
  }, [updateInstances, enableStreaming]);

  // Main frame update loop with streaming integration
  useFrame(() => {
    const manager = managerRef.current;
    const streamer = streamerRef.current;

    if (!manager) return;

    const cameraPosition = camera.position;
    const cameraMoved =
      cameraPosition.distanceTo(lastCameraPositionRef.current) > 1.0;

    // Update streaming if enabled
    if (enableStreaming && streamer && (cameraMoved || tiles.length > 0)) {
      // Create culling bounds
      const cullingBounds: CullingBounds = {
        center: cameraPosition.clone(),
        radius: finalStreamingConfig.maxStreamingDistance,
        minLOD: 0,
        maxLOD: 3,
      };

      // Add frustum if culling enabled
      if (culling.frustumCulling) {
        const frustum = new Frustum();
        const projScreenMatrix = new Matrix4();
        projScreenMatrix.multiplyMatrices(
          camera.projectionMatrix,
          camera.matrixWorldInverse
        );
        frustum.setFromProjectionMatrix(projScreenMatrix);
        cullingBounds.frustum = frustum;
      }

      // Stream tiles based on camera position
      streamer.streamTiles(tiles, cameraPosition, cullingBounds);

      // Attach streaming buffers to geometry
      const mesh = manager.getMesh();
      if (mesh?.geometry) {
        streamer.attachToGeometry(mesh.geometry);
      }

      lastCameraPositionRef.current.copy(cameraPosition);
    }

    // Perform frustum culling (if not using streaming)
    if (!enableStreaming && culling.frustumCulling) {
      const frustum = new Frustum();
      const projScreenMatrix = new Matrix4();
      projScreenMatrix.multiplyMatrices(
        camera.projectionMatrix,
        camera.matrixWorldInverse
      );
      frustum.setFromProjectionMatrix(projScreenMatrix);

      manager.performFrustumCulling(frustum);
    }

    // Update manager
    manager.update(0.016); // 60fps delta
  });

  // Render the instanced mesh
  if (!managerRef.current || !hexMaterial) {
    console.warn('HexInstanceRenderer: Not ready', {
      hasManager: !!managerRef.current,
      hasMaterial: !!hexMaterial,
      tilesCount: tiles.length,
      hexTerrainShader: !!hexTerrainShader,
    });

    // FALLBACK: Render simple cubes for tiles instead of returning null
    if (tiles.length > 0) {
      return (
        <group>
          {tiles.slice(0, 100).map((tile, index) => {
            const [x, z] = HexUtils.hexToPixel(tile.hex);
            const y = tile.elevation * 0.5;
            return (
              <mesh key={tile.id || index} position={[x, y, z]}>
                <boxGeometry args={[0.8, Math.max(0.1, tile.elevation), 0.8]} />
                <meshStandardMaterial
                  color={terrainColors[tile.terrain] || '#64748b'}
                />
              </mesh>
            );
          })}
        </group>
      );
    }

    return null;
  }

  const mesh = managerRef.current.getMesh();

  return (
    <primitive
      object={mesh}
      onClick={handleClick}
      castShadow={quality.shadows}
      receiveShadow={quality.shadows}
    />
  );
};

/**
 * Higher-order component for hex instance rendering with performance monitoring
 */
export const OptimizedHexRenderer: React.FC<
  HexInstanceRendererProps
> = props => {
  const { debug } = useRenderStore();

  if (debug.showStats) {
    // Wrap with performance monitoring in debug mode
    return (
      <group name='hex-instances-debug'>
        <HexInstanceRenderer {...props} />
        {/* Could add debug overlays here */}
      </group>
    );
  }

  return <HexInstanceRenderer {...props} />;
};

export default OptimizedHexRenderer;
