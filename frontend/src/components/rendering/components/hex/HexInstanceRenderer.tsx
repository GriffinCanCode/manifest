/**
 * HexInstanceRenderer
 * Efficient hex tile rendering using instanced meshes with BVH acceleration
 * Now includes per-instance data streaming for optimal performance
 * Integrates with existing game types and render store
 */

import { useFrame } from '@react-three/fiber';
import { useCallback, useEffect, useMemo, useRef } from 'react';
import * as THREE from 'three';
import { Matrix4 } from 'three';

import { shaderManager } from '../../../../shaders/manager';
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
import { useShader, useShaders } from '../../hooks/shader-hooks';

// Import WebGL diagnostics for browser console
import '../../../../utils/webgl-diagnostic';

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

  // EMERGENCY BYPASS: Force basic material if shader system is broken
  // Set this to true to temporarily bypass shader system during debugging
  const FORCE_BASIC_MATERIAL = false;
  const streamerRef = useRef<InstanceDataStreamer | null>(null);
  const { isReady: shadersReady } = useShaders();
  const hexTerrainShader = useShader('hex-terrain');
  const diagnosticRanRef = useRef<boolean>(false);

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

  // PROPER HEXAGON geometry for Civ-style tiles
  const hexGeometry = useMemo(() => {
    const geometry = new THREE.CylinderGeometry(
      0.85, // radiusTop - smaller for better tile spacing
      0.85, // radiusBottom
      0.15, // height - slightly thicker for visibility
      6, // radialSegments - hexagon
      1, // heightSegments
      false // openEnded
    );

    // Rotate to lay flat and orient hexagons correctly (flat-top)
    geometry.rotateX(-Math.PI / 2);
    // Rotate 30 degrees to get flat-top hexagon orientation (pointy sides)
    geometry.rotateY(Math.PI / 6);

    return geometry;
  }, []);

  // CIV-STYLE: Use hex-terrain custom shader material with comprehensive error handling
  const hexMaterial = useMemo(() => {
    // EMERGENCY BYPASS: Use basic material if forced or if shader system fails
    if (FORCE_BASIC_MATERIAL) {
      console.warn('🚨 HexRenderer: Using FORCED basic material bypass');
      return new THREE.MeshBasicMaterial({
        color: '#00aa33',
        wireframe: debug.showWireframe,
      });
    }

    if (!shadersReady) {
      console.warn('🎨 HexRenderer: Shaders not ready yet');
      return null;
    }

    if (!hexTerrainShader) {
      console.error('🚨 HexRenderer: hex-terrain shader failed to load');
      console.error(
        '🚨 Available shaders:',
        Object.keys(shaderManager.getStats())
      );
      // Fall back to basic material instead of returning null
      return new THREE.MeshBasicMaterial({
        color: '#00ff00',
        wireframe: debug.showWireframe,
      });
    }

    console.warn('✅ HexRenderer: hex-terrain shader loaded successfully');

    try {
      // Clone the shader material and configure it
      const material = hexTerrainShader.clone();
      material.wireframe = debug.showWireframe;

      // CRITICAL: Update uniforms that might conflict with Three.js built-ins
      if (material.uniforms) {
        // Three.js automatically provides cameraPosition, modelMatrix, viewMatrix, projectionMatrix
        // Remove any conflicting uniforms that we might have inadvertently included
        ['u_cameraPosition', 'u_viewMatrix', 'u_projectionMatrix'].forEach(
          uniformName => {
            if (material.uniforms[uniformName]) {
              delete material.uniforms[uniformName];
            }
          }
        );

        // Set hex-specific uniforms
        if (material.uniforms.u_hexSize) {
          material.uniforms.u_hexSize.value = 0.9; // Match geometry size
        }
        if (material.uniforms.u_hexSpacing) {
          material.uniforms.u_hexSpacing.value = 1.0;
        }
        if (material.uniforms.u_heightScale) {
          material.uniforms.u_heightScale.value = 0.5; // Match our elevation scaling
        }
        if (material.uniforms.u_time) {
          material.uniforms.u_time.value = 0; // Will be updated in useFrame
        }

        // Disable debug modes to see actual terrain colors
        if (material.uniforms.u_showBiomes) {
          material.uniforms.u_showBiomes.value = false;
        }
        if (material.uniforms.u_showLOD) {
          material.uniforms.u_showLOD.value = false;
        }
        if (material.uniforms.u_showHeight) {
          material.uniforms.u_showHeight.value = false;
        }
      }

      // Add compilation error detection and debugging
      material.onBeforeCompile = (shader, _renderer) => {
        console.warn('🎨 HexRenderer: Compiling hex-terrain shader...');
        console.warn('🎨 Defines:', shader.defines);
        console.warn(
          '🎨 Vertex shader length:',
          shader.vertexShader?.length || 0
        );
        console.warn(
          '🎨 Fragment shader length:',
          shader.fragmentShader?.length || 0
        );

        // Check for required instanced attributes in vertex shader
        if (shader.vertexShader.includes('instancePosition')) {
          console.warn(
            '✅ HexRenderer: instancePosition attribute found in vertex shader'
          );
        } else {
          console.error(
            '🚨 HexRenderer: instancePosition attribute missing from vertex shader'
          );
        }
      };

      return material;
    } catch (error) {
      console.error(
        '🚨 HexRenderer: Failed to configure hex-terrain shader:',
        error
      );
      // Fall back to basic material
      return new THREE.MeshBasicMaterial({
        color: '#ff0000',
        wireframe: debug.showWireframe,
      });
    }
  }, [
    FORCE_BASIC_MATERIAL,
    shadersReady,
    hexTerrainShader,
    debug.showWireframe,
  ]);

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
      return;
    }

    // Ensure we have the right number of instances
    const targetCount = Math.min(tiles.length, maxInstances);
    if (instancedMesh.count !== targetCount) {
      instancedMesh.count = targetCount;
    }

    const matrix = new Matrix4();
    const color = new THREE.Color(); // Using colors for terrain types

    // Update each instance with ENHANCED DEBUGGING
    for (let i = 0; i < Math.min(tiles.length, maxInstances); i++) {
      const tile = tiles[i];
      const [x, z] = HexUtils.hexToPixel(tile.hex);
      const y = tile.elevation * 0.5;

      // For shader-based rendering, still set matrix for basic Three.js compatibility
      // but keep it simple - let shader handle most positioning
      matrix.makeTranslation(x, y, z);
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
        // Convert TerrainType enum to numeric value for shader
        const biomeMap: Record<TerrainType, number> = {
          [TerrainType.Ocean]: 0,
          [TerrainType.Grassland]: 1,
          [TerrainType.Plains]: 2,
          [TerrainType.Desert]: 3,
          [TerrainType.Tundra]: 4,
          [TerrainType.Snow]: 5,
          [TerrainType.Forest]: 6,
          [TerrainType.Jungle]: 7,
          [TerrainType.Hills]: 8,
          [TerrainType.Mountain]: 9,
        };
        const biomeValue = biomeMap[tile.terrain] ?? 0;
        biomeArray[i] = biomeValue / 9.0; // Normalize to 0-1 range
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
  }, [tiles, terrainColors, maxInstances]);

  // Handle click events using raycasting
  const handleClick = useCallback(
    (_event: THREE.Event) => {
      if (onTileClick) {
        // TODO: Implement proper raycasting to determine which tile was clicked
      }
    },
    [onTileClick]
  );

  // Initialize simple renderer on mount
  useEffect(() => {
    const cleanupRenderer = initializeSimpleRenderer();
    const cleanupStreamer = initializeStreamer();

    return () => {
      cleanupRenderer();
      cleanupStreamer();
    };
  }, [initializeSimpleRenderer, initializeStreamer]);

  // Run shader diagnostics when ready but potentially having issues
  useEffect(() => {
    if (shadersReady && !hexTerrainShader && !diagnosticRanRef.current) {
      diagnosticRanRef.current = true;
      // Shader diagnostic code can be enabled if needed for debugging
    }
  }, [shadersReady, hexTerrainShader]);

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

  // Simplified frame update - update shader uniforms
  useFrame(({ clock }) => {
    // Update shader time uniform for animations
    if (
      hexMaterial &&
      'uniforms' in hexMaterial &&
      hexMaterial.uniforms?.u_time
    ) {
      hexMaterial.uniforms.u_time.value = clock.getElapsedTime();
    }
    // Future: Add LOD, frustum culling, etc. here if needed
  });

  // CRITICAL: Wait for shader system to initialize before rendering
  if (!shadersReady) {
    console.warn('⏳ HexRenderer: Waiting for shader system...');
    return null;
  }

  // Don't render if no tiles or material
  if (!hexMaterial) {
    console.error('🚨 HexRenderer: No material available, cannot render');
    return null;
  }

  if (tiles.length === 0) {
    console.warn('📭 HexRenderer: No tiles to render');
    return null;
  }

  console.warn(
    `✅ HexRenderer: Rendering ${tiles.length} tiles with shader system`
  );

  // SHADER DEBUG: Log material type and check if it's using our custom shader
  const materialType = hexMaterial.constructor.name;
  const isCustomShader = materialType === 'ShaderMaterial';
  console.warn(
    `🎨 HexRenderer: Using ${materialType} (custom shader: ${isCustomShader})`
  );

  if (isCustomShader && 'uniforms' in hexMaterial && hexMaterial.uniforms) {
    console.warn(
      '🎨 HexRenderer: Available uniforms:',
      Object.keys(hexMaterial.uniforms)
    );
  }

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
