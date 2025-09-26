/**
 * HexInstanceRenderer
 * Efficient hex tile rendering using instanced meshes with BVH acceleration
 * Now includes per-instance data streaming for optimal performance
 * Integrates with existing game types and render store
 */

import { useFrame } from '@react-three/fiber';
import { useCallback, useEffect, useMemo, useRef } from 'react';
import * as THREE from 'three';
import { Matrix4, Vector3 } from 'three';

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
  enableSpatialQueries: _enableSpatialQueries = true,
  enableStreaming = true,
  streamingConfig,
}) => {
  const { debug } = useRenderStore();
  const streamerRef = useRef<InstanceDataStreamer | null>(null);
  const hexTerrainShader = useShader('hex-terrain');

  // HEX-TERRAIN SHADER: Initialize instanced attributes for shader
  const initializeInstancedMesh = useCallback(
    (mesh: THREE.InstancedMesh | null) => {
      if (!mesh) return;

      // Set up instanced attributes required by hex-terrain shader
      const { geometry } = mesh;

      // instancePosition (vec3) - will be set per tile
      geometry.setAttribute(
        'instancePosition',
        new THREE.InstancedBufferAttribute(
          new Float32Array(maxInstances * 3),
          3
        )
      );

      // instanceColor (vec3) - terrain colors
      geometry.setAttribute(
        'instanceColor',
        new THREE.InstancedBufferAttribute(
          new Float32Array(maxInstances * 3),
          3
        )
      );

      // instanceHeight (float) - elevation
      geometry.setAttribute(
        'instanceHeight',
        new THREE.InstancedBufferAttribute(new Float32Array(maxInstances), 1)
      );

      // instanceBiome (float) - terrain type as number
      geometry.setAttribute(
        'instanceBiome',
        new THREE.InstancedBufferAttribute(new Float32Array(maxInstances), 1)
      );

      // instanceTexCoords (vec2) - for texture mapping
      geometry.setAttribute(
        'instanceTexCoords',
        new THREE.InstancedBufferAttribute(
          new Float32Array(maxInstances * 2),
          2
        )
      );

      // instanceResourceMask (float) - resource information
      geometry.setAttribute(
        'instanceResourceMask',
        new THREE.InstancedBufferAttribute(new Float32Array(maxInstances), 1)
      );

      // Force initial visibility
      mesh.visible = true;
      mesh.frustumCulled = false;

      console.warn('🎨 InstancedMesh HEX-TERRAIN SHADER initialization:', {
        count: mesh.count,
        maxInstances,
        visible: mesh.visible,
        material: (mesh.material as THREE.Material).type,
        attributes: Object.keys(geometry.attributes),
        hasInstancedPosition: !!geometry.attributes.instancePosition,
        hasInstancedColor: !!geometry.attributes.instanceColor,
      });
    },
    [maxInstances]
  );

  // BRIGHT terrain color mapping for maximum visibility
  const terrainColors = useMemo(
    () => ({
      [TerrainType.Ocean]: '#0066ff', // Bright blue
      [TerrainType.Grassland]: '#00ff44', // Bright green
      [TerrainType.Plains]: '#88ff00', // Bright lime
      [TerrainType.Desert]: '#ffdd00', // Bright yellow
      [TerrainType.Tundra]: '#cccccc', // Bright gray
      [TerrainType.Snow]: '#ffffff', // Pure white
      [TerrainType.Forest]: '#006622', // Bright dark green
      [TerrainType.Jungle]: '#00aa33', // Bright jungle green
      [TerrainType.Hills]: '#996633', // Brown
      [TerrainType.Mountain]: '#666666', // Medium gray
    }),
    []
  );

  // PROPER HEXAGON geometry for Civ-style tiles (keeping working material)
  const hexGeometry = useMemo(() => {
    console.warn('🔧 Creating CIV-STYLE hexagon geometry...');
    const geometry = new THREE.CylinderGeometry(
      0.9, // radiusTop - slightly smaller than 1.0 for tile spacing
      0.9, // radiusBottom
      0.1, // height - flat tiles
      6, // radialSegments - hexagon
      1, // heightSegments
      false // openEnded
    );

    // Rotate to lay flat (hexagons are created vertically by default)
    geometry.rotateX(-Math.PI / 2);

    console.warn('✅ Hexagon geometry created:', {
      vertices: geometry.attributes.position.count,
      triangles: geometry.index ? geometry.index.count / 3 : 'no index',
      radiusTop: 0.9,
      height: 0.1,
      segments: 6,
    });
    return geometry;
  }, []);

  // CIV-STYLE: Use hex-terrain custom shader material
  const hexMaterial = useMemo(() => {
    console.warn('🎨 Setting up hex-terrain shader material...');

    if (!hexTerrainShader) {
      // Fallback to basic material while shader loads
      console.warn('⏳ Hex-terrain shader not ready, using fallback...');
      return new THREE.MeshBasicMaterial({
        color: 0x00aa00, // Solid green fallback
        wireframe: debug.showWireframe,
        transparent: false,
      });
    }

    // Clone the shader material and configure it
    const material = hexTerrainShader.clone();
    material.wireframe = debug.showWireframe;

    console.warn('✅ Hex-terrain shader material created:', {
      type: material.type,
      wireframe: material.wireframe,
      uniforms: Object.keys(material.uniforms || {}),
    });
    return material;
  }, [hexTerrainShader, debug.showWireframe]);

  // Simple wireframe border material (disabled for now)
  // const _wireframeMaterial = useMemo(() => {
  //   return new THREE.MeshBasicMaterial({
  //     color: 0x000000, // Black borders
  //     wireframe: true,
  //     transparent: true,
  //     opacity: 0.6,
  //   });
  // }, []);

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

  // SIMPLIFIED WORKING RENDERER: Use basic instanced mesh instead of complex BVH manager
  const instancedMeshRef = useRef<THREE.InstancedMesh | null>(null);

  // Callback ref to properly initialize instanced mesh
  const setInstancedMeshRef = useCallback(
    (mesh: THREE.InstancedMesh | null) => {
      instancedMeshRef.current = mesh;
      initializeInstancedMesh(mesh);
    },
    [initializeInstancedMesh]
  );

  const initializeSimpleRenderer = useCallback(() => {
    // Simple working initialization that will definitely work
    console.warn('🏗️ HexInstanceRenderer: Initializing simple renderer...');
    return () => {
      // Cleanup if needed
      if (instancedMeshRef.current) {
        instancedMeshRef.current.dispose();
      }
    };
  }, []);

  // SIMPLIFIED UPDATE: Update instanced mesh directly
  const updateInstances = useCallback(() => {
    const instancedMesh = instancedMeshRef.current;
    if (!instancedMesh || tiles.length === 0) {
      console.warn('❌ updateInstances FAILED:', {
        hasInstancedMesh: !!instancedMesh,
        tilesLength: tiles.length,
        instancedMeshRef: instancedMeshRef.current,
      });
      return;
    }

    console.warn('🔄 Updating CIV-STYLE hex renderer:', {
      tilesLength: tiles.length,
      hasInstancedMesh: !!instancedMesh,
      instancedMeshVisible: instancedMesh.visible,
      instancedMeshCount: instancedMesh.count,
      hasInstanceColor: !!instancedMesh.instanceColor,
      meshPosition: `${instancedMesh.position.x}, ${instancedMesh.position.y}, ${instancedMesh.position.z}`,
      meshScale: `${instancedMesh.scale.x}, ${instancedMesh.scale.y}, ${instancedMesh.scale.z}`,
      materialType: (instancedMesh.material as THREE.MeshBasicMaterial).type,
      geometryType: instancedMesh.geometry.type,
    });

    // Ensure we have the right number of instances
    const targetCount = Math.min(tiles.length, maxInstances);
    if (instancedMesh.count !== targetCount) {
      instancedMesh.count = targetCount;
      console.warn('🔢 Set instancedMesh.count to:', targetCount);
    }

    const matrix = new Matrix4();
    const color = new THREE.Color(); // Using colors for terrain types

    // Update each instance with ENHANCED DEBUGGING
    for (let i = 0; i < Math.min(tiles.length, maxInstances); i++) {
      const tile = tiles[i];
      const [x, z] = HexUtils.hexToPixel(tile.hex);
      const y = tile.elevation * 0.5;

      // Civ-style hex scaling - reasonable size
      matrix.makeTranslation(x, y, z);
      const scaleSize = 0.95; // Slightly smaller than 1.0 for proper tile spacing
      const height = Math.max(0.1, 0.1 + Math.abs(tile.elevation) * 0.2); // Subtle elevation
      matrix.scale(new Vector3(scaleSize, height, scaleSize));
      instancedMesh.setMatrixAt(i, matrix);

      // Set instanced attributes for hex-terrain shader
      const { geometry } = instancedMesh;

      // instancePosition (vec3)
      if (geometry.attributes.instancePosition) {
        const posArray = geometry.attributes.instancePosition
          .array as Float32Array;
        posArray[i * 3] = x;
        posArray[i * 3 + 1] = y;
        posArray[i * 3 + 2] = z;
      }

      // instanceColor (vec3) - terrain colors
      if (geometry.attributes.instanceColor) {
        const colorHex = terrainColors[tile.terrain] || '#64748b';
        color.set(colorHex);
        const colorArray = geometry.attributes.instanceColor
          .array as Float32Array;
        colorArray[i * 3] = color.r;
        colorArray[i * 3 + 1] = color.g;
        colorArray[i * 3 + 2] = color.b;
      }

      // instanceHeight (float) - elevation
      if (geometry.attributes.instanceHeight) {
        const heightArray = geometry.attributes.instanceHeight
          .array as Float32Array;
        heightArray[i] = tile.elevation;
      }

      // instanceBiome (float) - terrain type as number
      if (geometry.attributes.instanceBiome) {
        const biomeArray = geometry.attributes.instanceBiome
          .array as Float32Array;
        biomeArray[i] = Number(tile.terrain) / 10.0; // Normalize terrain type
      }

      // instanceTexCoords (vec2) - hex coordinates
      if (geometry.attributes.instanceTexCoords) {
        const texArray = geometry.attributes.instanceTexCoords
          .array as Float32Array;
        texArray[i * 2] = tile.hex.q / 100.0; // Normalize hex coordinates
        texArray[i * 2 + 1] = tile.hex.r / 100.0;
      }

      // instanceResourceMask (float) - placeholder
      if (geometry.attributes.instanceResourceMask) {
        const resourceArray = geometry.attributes.instanceResourceMask
          .array as Float32Array;
        resourceArray[i] = 0.0; // No resources for now
      }

      // DEBUG: Log first few instances
      if (i < 3) {
        console.warn(`🎯 Instance ${i} setup:`, {
          tileId: tile.id,
          hex: tile.hex,
          pixelPos: [x, z],
          worldPos: [x, y, z],
          scaleSize,
          height,
          terrain: tile.terrain,
          colorHex: terrainColors[tile.terrain] || '#64748b',
        });
      }
    }

    // Mark all instanced attributes for update
    const { geometry } = instancedMesh;
    instancedMesh.instanceMatrix.needsUpdate = true;

    // Update all instanced attributes for shader
    if (geometry.attributes.instancePosition) {
      geometry.attributes.instancePosition.needsUpdate = true;
    }
    if (geometry.attributes.instanceColor) {
      geometry.attributes.instanceColor.needsUpdate = true;
    }
    if (geometry.attributes.instanceHeight) {
      geometry.attributes.instanceHeight.needsUpdate = true;
    }
    if (geometry.attributes.instanceBiome) {
      geometry.attributes.instanceBiome.needsUpdate = true;
    }
    if (geometry.attributes.instanceTexCoords) {
      geometry.attributes.instanceTexCoords.needsUpdate = true;
    }
    if (geometry.attributes.instanceResourceMask) {
      geometry.attributes.instanceResourceMask.needsUpdate = true;
    }

    // CRITICAL: Force mesh to be visible
    instancedMesh.visible = true;
    instancedMesh.frustumCulled = false; // Disable frustum culling for debugging

    console.warn('🎨 InstancedMesh CIV-STYLE visible:', {
      visible: instancedMesh.visible,
      frustumCulled: instancedMesh.frustumCulled,
      count: instancedMesh.count,
      hasInstanceColor: !!instancedMesh.instanceColor,
      materialType: (instancedMesh.material as THREE.MeshBasicMaterial).type,
      boundingBox: instancedMesh.geometry.boundingBox,
    });

    console.warn(
      `✅ Updated ${Math.min(tiles.length, maxInstances)} hex instances with positions:`,
      {
        samplePositions: tiles.slice(0, 3).map(t => {
          const [x, z] = HexUtils.hexToPixel(t.hex);
          return `${t.terrain} at (${x.toFixed(1)}, ${t.elevation * 0.5}, ${z.toFixed(1)})`;
        }),
        sampleColors: tiles
          .slice(0, 3)
          .map(t => terrainColors[t.terrain] || '#64748b'),
      }
    );
  }, [tiles, terrainColors, maxInstances]);

  // Handle click events using raycasting
  const handleClick = useCallback(
    (_event: THREE.Event) => {
      if (onTileClick) {
        // TODO: Implement proper raycasting to determine which tile was clicked
        // For now, just a placeholder
        console.warn('Tile clicked - raycasting not implemented yet');
      }
    },
    [onTileClick]
  );

  // Initialize simple renderer on mount
  useEffect(() => {
    console.warn('🚀 HexInstanceRenderer: Initializing simple renderer...');

    const cleanupRenderer = initializeSimpleRenderer();
    const cleanupStreamer = initializeStreamer();

    return () => {
      cleanupRenderer();
      cleanupStreamer();
    };
  }, [initializeSimpleRenderer, initializeStreamer]);

  // Wire frame mesh reference
  // Wireframe mesh disabled for now
  // const _wireframeMeshRef = useRef<THREE.InstancedMesh | null>(null);
  // const _setWireframeMeshRef = useCallback(
  //   (mesh: THREE.InstancedMesh | null) => {
  //     _wireframeMeshRef.current = mesh;
  //     if (mesh) {
  //       mesh.visible = true;
  //       mesh.frustumCulled = false;
  //     }
  //   },
  //   []
  // );

  // Simplified single effect for tile updates
  useEffect(() => {
    console.warn('🔄 TILES CHANGED - HexInstanceRenderer received new tiles:', {
      tilesLength: tiles.length,
      timestamp: Date.now(),
      firstTile: tiles[0]
        ? {
            id: tiles[0].id,
            terrain: tiles[0].terrain,
            hex: tiles[0].hex,
          }
        : null,
    });

    if (tiles.length > 0) {
      // Update main instances
      updateInstances();

      // Wireframe instances disabled for now
      // const wireframeMesh = _wireframeMeshRef.current;
      // if (wireframeMesh) {
      //   const matrix = new Matrix4();
      //   const numInstances = Math.min(tiles.length, maxInstances);
      //   wireframeMesh.count = numInstances;
      //   for (let i = 0; i < numInstances; i++) {
      //     const tile = tiles[i];
      //     const [x, z] = HexUtils.hexToPixel(tile.hex);
      //     const y = tile.elevation * 0.5 + 0.01; // Slightly above main tiles
      //     matrix.makeTranslation(x, y, z);
      //     const scaleSize = 0.8;
      //     const height = Math.max(0.2, Math.abs(tile.elevation));
      //     matrix.scale(new Vector3(scaleSize, height, scaleSize));
      //     wireframeMesh.setMatrixAt(i, matrix);
      //   }
      //   wireframeMesh.instanceMatrix.needsUpdate = true;
      //   console.warn('✅ Updated wireframe borders for', numInstances, 'tiles');
      // }
    }
  }, [tiles, updateInstances, maxInstances]);

  // Simplified frame update - just update instances if needed
  useFrame(() => {
    // For now, simple renderer doesn't need per-frame updates
    // Future: Add LOD, frustum culling, etc. here if needed
  });

  // SIMPLIFIED RENDER: Use simple instanced mesh
  console.warn('🎨 HexInstanceRenderer CIV-STYLE render:', {
    hasMaterial: !!hexMaterial,
    tilesCount: tiles.length,
    hasGeometry: !!hexGeometry,
    materialType: hexMaterial?.type,
    geometryType: hexGeometry?.type,
    tilesType: typeof tiles,
    tilesArray: Array.isArray(tiles),
    firstTileId: tiles[0]?.id,
    componentTimestamp: Date.now(),
  });

  // Don't render if no tiles or material
  if (!hexMaterial) {
    console.warn('❌ HexInstanceRenderer: No material, skipping render');
    return null;
  }

  if (tiles.length === 0) {
    console.warn(
      '❌ HexInstanceRenderer: No tiles (length=0), skipping render. Waiting for tiles...',
      {
        tilesLength: tiles.length,
        tilesType: typeof tiles,
        isArray: Array.isArray(tiles),
      }
    );
    return null;
  }

  console.warn(
    '✅ HexInstanceRenderer: Ready to render with',
    tiles.length,
    'tiles!'
  );

  console.warn('🔥 CIV-STYLE HEX RENDER ATTEMPT:', {
    hexGeometry: !!hexGeometry,
    hexMaterial: !!hexMaterial,
    tilesLength: tiles.length,
    maxInstances,
    instanceCount: Math.min(tiles.length, maxInstances),
    geometryType: hexGeometry?.type,
    materialType: hexMaterial?.type,
    hasVertexColors: hexMaterial?.vertexColors,
  });

  console.warn('🚀 RENDERING HEX-TERRAIN SHADER:', {
    geometry: hexGeometry?.type,
    material: hexMaterial?.type,
    shaderName:
      hexMaterial?.type === 'ShaderMaterial' ? 'hex-terrain' : 'fallback',
    instances: Math.min(tiles.length, maxInstances),
    hasGeometry: !!hexGeometry,
    hasMaterial: !!hexMaterial,
    hasShader: !!hexTerrainShader,
  });

  return (
    <group name='hex-renderer-group'>
      {/* Main hex tiles with solid colors */}
      <instancedMesh
        ref={setInstancedMeshRef}
        args={[hexGeometry, hexMaterial, Math.min(tiles.length, maxInstances)]}
        onClick={handleClick}
        castShadow={false}
        receiveShadow={false}
      />

      {/* Simple wireframe borders overlay - DISABLED until colors are working */}
      {/* <instancedMesh
        ref={setWireframeMeshRef}
        args={[
          hexGeometry,
          wireframeMaterial,
          Math.min(tiles.length, maxInstances),
        ]}
      /> */}
    </group>
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
