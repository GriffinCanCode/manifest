/**
 * Game Canvas with WebGL2/WebGPU Initialization
 * Integrates device detection, performance monitoring, and optimized rendering
 */

import { Environment, Html } from '@react-three/drei';
import { useFrame, type ThreeEvent } from '@react-three/fiber';
import { button, useControls } from 'leva';
import React, { Suspense, useCallback, useMemo, useRef, useState } from 'react';
import type * as THREE from 'three';
import { Vector3 } from 'three';

import { LoggingUtils } from '../../config/logging';
import { useLogger, usePerformanceLogger } from '../../hooks/use-logger';
import { useTileStreaming } from '../../hooks/use-tile-streaming';
import { usePostProcessingMetrics } from '../../hooks/usePostProcessingMetrics';
import {
  usePerformanceMonitoring,
  useRenderStore,
  type RenderDebug,
  type RenderQuality,
} from '../../stores/render-store';
import type {
  DeviceCapabilities,
  RenderingSettings,
} from '../../utils/capabilities';
import {
  HexUtils,
  TerrainType,
  type GameTile,
  type GameUnit,
} from '../../utils/game-types';
import { CameraController } from '../controls';
import { HexInstanceRenderer } from '../rendering/components/hex/HexInstanceRenderer';
import { MultiStepRenderer } from '../rendering/components/pipeline/MultiStepRenderer';
import RenderInitializer from '../rendering/components/pipeline/RenderInitializer';

/**
 * Now using sophisticated HexInstanceRenderer with BVH acceleration and advanced materials
 */

/**
 * Game scene with performance optimizations and adaptive quality
 */
const GameScene: React.FC = () => {
  const [selectedTile, setSelectedTile] = useState<GameTile | null>(null);
  const [selectedUnit, setSelectedUnit] = useState<GameUnit | null>(null);
  const [_highlightedTiles, setHighlightedTiles] = useState<Set<number>>(
    new Set()
  );

  const { capabilities, quality, debug } = useRenderStore();
  const { checkPerformance } = usePerformanceMonitoring();

  // Initialize logging for game scene
  const gameLogger = useLogger('game', 'GameScene');
  const renderLogger = useLogger('render', 'GameScene');
  const performanceLogger = usePerformanceLogger('performance', 'GameScene');

  // Camera position ref for tile streaming
  const cameraPositionRef = useRef(new Vector3(15, 15, 15));
  const lastCameraLogRef = useRef(0);

  // Real tile streaming from backend (replaces mock data)
  const {
    tiles,
    isLoading: tilesLoading,
    error: tileError,
    metrics,
    refreshTiles,
  } = useTileStreaming({
    cameraPosition: cameraPositionRef.current,
    maxDistance: 50,
    quality:
      quality.level === 'low'
        ? 'low'
        : quality.level === 'ultra'
          ? 'high'
          : 'medium',
    autoStream: true,
  });

  // Log tile streaming events
  React.useEffect(() => {
    if (tileError) {
      gameLogger.error('Tile streaming error occurred', new Error(tileError), {
        cameraPosition: {
          x: cameraPositionRef.current.x,
          y: cameraPositionRef.current.y,
          z: cameraPositionRef.current.z,
        },
        qualityLevel: quality.level,
      });
    }
  }, [tileError, gameLogger, quality.level]);

  React.useEffect(() => {
    gameLogger.info('Tile streaming metrics updated', {
      tilesLoaded: metrics.tilesLoaded,
      streamingTimeMs: metrics.streamingTimeMs,
      isLoading: tilesLoading,
      qualityLevel: quality.level,
    });
  }, [metrics, tilesLoading, gameLogger, quality.level]);

  // Create game world structure with real tiles
  const gameWorld = useMemo(
    () => ({
      tiles,
      units: [] as GameUnit[], // TODO: Add real unit streaming next
    }),
    [tiles]
  );

  // Performance monitoring and camera tracking
  useFrame(state => {
    // Update camera position for tile streaming
    const previousPosition = cameraPositionRef.current.clone();
    cameraPositionRef.current.copy(state.camera.position);

    // CRITICAL DEBUG: Log camera state every few seconds
    const now = Date.now();
    if (now - lastCameraLogRef.current > 3000) {
      lastCameraLogRef.current = now;
      renderLogger.info('🎥 CAMERA DEBUG:', {
        position: {
          x: Math.round(cameraPositionRef.current.x * 100) / 100,
          y: Math.round(cameraPositionRef.current.y * 100) / 100,
          z: Math.round(cameraPositionRef.current.z * 100) / 100,
        },
        fov: 'fov' in state.camera ? state.camera.fov : 'N/A',
        lookingAt: state.camera.getWorldDirection(new Vector3()),
        tilesCount: gameWorld.tiles.length,
        renderingCubes: gameWorld.tiles.length > 0 ? 'YES' : 'NO',
        cameraType: state.camera.type,
        projectionMatrix: state.camera.projectionMatrix.elements.slice(0, 4),
      });
    }

    // Log significant camera movements
    const distanceMoved = previousPosition.distanceTo(
      cameraPositionRef.current
    );
    if (distanceMoved > 5) {
      // Only log significant movements
      renderLogger.debug('Camera position updated', {
        position: {
          x: Math.round(cameraPositionRef.current.x * 100) / 100,
          y: Math.round(cameraPositionRef.current.y * 100) / 100,
          z: Math.round(cameraPositionRef.current.z * 100) / 100,
        },
        distanceMoved: Math.round(distanceMoved * 100) / 100,
      });
    }

    if (LoggingUtils.isDevelopment()) {
      const deltaMs = state.clock.getDelta() * 1000;
      checkPerformance(state.clock.elapsedTime, deltaMs);

      // Log performance warnings
      if (deltaMs > 50) {
        // Frame took longer than 50ms
        performanceLogger.warn('Slow frame detected', {
          frameTime: Math.round(deltaMs * 100) / 100,
          fps: Math.round(1000 / deltaMs),
          qualityLevel: quality.level,
          tilesCount: tiles.length,
        });
      }
    }
  });

  // Dev controls for testing (only in development)
  if (LoggingUtils.isDevelopment()) {
    // eslint-disable-next-line react-hooks/rules-of-hooks
    useControls(
      'Render Settings',
      {
        showWireframe: debug.showWireframe,
        showBounds: debug.showBounds,
        showStats: debug.showStats,
        lodBias: { value: quality.lodBias, min: 0.1, max: 3.0, step: 0.1 },
        renderScale: {
          value: quality.renderScale,
          min: 0.5,
          max: 2.0,
          step: 0.1,
        },
        particleQuality: {
          value: quality.particleQuality,
          min: 0.1,
          max: 1.0,
          step: 0.1,
        },
      },
      { collapsed: !debug.showStats }
    );

    // eslint-disable-next-line react-hooks/rules-of-hooks
    useControls(
      'Tile Streaming',
      {
        tilesLoaded: { value: metrics.tilesLoaded, disabled: true },
        isLoading: { value: tilesLoading, disabled: true },
        streamingTime: {
          value: `${metrics.streamingTimeMs.toFixed(2)}ms`,
          disabled: true,
        },
        refreshTiles: button(() => void refreshTiles()),
        error: { value: tileError ?? 'None', disabled: true },
      },
      { collapsed: !debug.showStats }
    );
  }

  const handleTileClick = useCallback(
    (tile: GameTile) => {
      setSelectedTile(tile);
      setSelectedUnit(null);
      setHighlightedTiles(new Set());

      gameLogger.info('Tile selected by user', {
        tileId: tile.id,
        position: { q: tile.hex.q, r: tile.hex.r },
        terrain: tile.terrain,
        elevation: tile.elevation,
        hasResources: !!tile.resources?.length,
      });
    },
    [gameLogger]
  );

  const handleUnitClick = useCallback(
    (unit: GameUnit) => {
      const timer = performanceLogger.startTimer('unit-selection');

      setSelectedUnit(unit);
      setSelectedTile(null);

      // Highlight tiles in movement range
      const movementRange = 3;
      const inRange = gameWorld.tiles.filter(tile => {
        const distance =
          Math.abs(tile.hex.q - unit.position.q) +
          Math.abs(tile.hex.r - unit.position.r) +
          Math.abs(
            -tile.hex.q - tile.hex.r + unit.position.q + unit.position.r
          );
        return distance <= movementRange;
      });

      const highlightedTileIds = new Set(inRange.map(t => t.id));
      setHighlightedTiles(highlightedTileIds);

      timer.end('Unit selection and range calculation completed', {
        unitId: unit.id,
        unitType: unit.type,
        movementRange,
        tilesInRange: inRange.length,
      });

      gameLogger.info('Unit selected by user', {
        unitId: unit.id,
        unitType: unit.type,
        playerId: unit.playerId,
        position: { q: unit.position.q, r: unit.position.r },
        health: unit.health,
        tilesInMovementRange: inRange.length,
      });
    },
    [gameWorld.tiles, gameLogger, performanceLogger]
  );

  // Monitor post-processing performance
  usePostProcessingMetrics();

  // Show error overlay if tile streaming fails
  if (tileError) {
    const handleRetry = () => {
      gameLogger.info('User initiated tile streaming retry', {
        error: tileError,
        cameraPosition: {
          x: cameraPositionRef.current.x,
          y: cameraPositionRef.current.y,
          z: cameraPositionRef.current.z,
        },
      });
      void refreshTiles();
    };

    return (
      <group>
        <Html center>
          <div
            style={{
              color: 'red',
              background: 'rgba(0,0,0,0.8)',
              padding: '1rem',
              borderRadius: '8px',
              textAlign: 'center' as const,
            }}
          >
            <h3>🌍 Backend Connection Error</h3>
            <p>{tileError}</p>
            <button
              onClick={handleRetry}
              style={{
                padding: '0.5rem 1rem',
                marginTop: '0.5rem',
                cursor: 'pointer',
                background: '#ff4444',
                color: 'white',
                border: 'none',
                borderRadius: '4px',
              }}
            >
              Retry Tile Streaming
            </button>
          </div>
        </Html>
      </group>
    );
  }

  // STEP 1: Re-enable MultiStepRenderer with minimal passes
  return (
    <MultiStepRenderer
      enableSelection={false}
      enableDebug={false}
      enableTAA={false}
    >
      {/* SIMPLE LIGHTING - no complex shader dependencies */}
      <ambientLight intensity={0.6} />
      <directionalLight position={[10, 10, 5]} intensity={1} />

      {/* SOPHISTICATED HEX RENDERING: Advanced instanced rendering with BVH acceleration */}
      {/* DEBUG: Monitor tile data flow */}
      {(() => {
        console.warn(`🔍 TILE FLOW DEBUG:`, {
          'gameWorld.tiles.length': gameWorld.tiles.length,
          'raw tiles.length': tiles.length,
          tilesLoading,
          'tiles sample': gameWorld.tiles.slice(0, 2),
        });
        return null;
      })()}

      <HexInstanceRenderer
        tiles={gameWorld.tiles}
        onTileClick={handleTileClick}
        selectedTileId={selectedTile?.id}
        maxInstances={5000}
        enableSpatialQueries
        enableStreaming
      />

      {/* EMERGENCY FALLBACK: Simple cube rendering if sophisticated renderer fails */}
      {gameWorld.tiles.length > 0 && (
        <group>
          {/* Green indicator: tiles are loaded */}
          <mesh position={[0, 8, 0]}>
            <sphereGeometry args={[0.5, 8, 6]} />
            <meshBasicMaterial color='#00ff00' />
          </mesh>

          {/* Simple fallback tiles - positioned slightly higher to be visible */}
          {gameWorld.tiles.slice(0, 30).map((tile, index) => {
            const [x, z] = HexUtils.hexToPixel(tile.hex);
            const y = tile.elevation * 0.5;
            return (
              <mesh
                key={`fallback-${tile.id || index}`}
                position={[x, y + 2, z]}
              >
                <boxGeometry
                  args={[
                    0.6,
                    Math.max(0.1, Math.abs(tile.elevation * 0.5)),
                    0.6,
                  ]}
                />
                <meshBasicMaterial
                  color={
                    tile.terrain === TerrainType.Ocean
                      ? '#1e40af'
                      : tile.terrain === TerrainType.Grassland
                        ? '#22c55e'
                        : '#84cc16'
                  }
                  transparent
                  opacity={0.8}
                />
              </mesh>
            );
          })}
        </group>
      )}

      {/* Render units with instancing optimization */}
      <UnitsRenderer
        units={gameWorld.units}
        onUnitClick={handleUnitClick}
        selectedUnitId={selectedUnit?.id}
        quality={quality}
        debug={debug}
      />

      {/* Game UI overlays */}
      <GameUIOverlays selectedTile={selectedTile} selectedUnit={selectedUnit} />

      {/* Enhanced camera controls */}
      <CameraController
        mode='orbital'
        enableShake={false}
        enableFocus
        smoothTransitions
      />

      {/* Clean scene - no debug geometry - showing actual game map */}

      {/* Adaptive environment */}
      <AdaptiveEnvironment capabilities={capabilities} quality={quality} />

      {/* Loading indicator for tile streaming */}
      {tilesLoading && (
        <Html center>
          <div
            style={{
              color: 'white',
              background: 'rgba(0,0,0,0.7)',
              padding: '1rem',
              borderRadius: '8px',
              display: 'flex',
              alignItems: 'center',
              gap: '0.5rem',
            }}
          >
            <div
              style={{
                width: '20px',
                height: '20px',
                border: '2px solid transparent',
                borderTop: '2px solid white',
                borderRadius: '50%',
                animation: 'spin 1s linear infinite',
              }}
            />
            🌍 Streaming tiles from backend...
          </div>
        </Html>
      )}

      {/* Conditional fog - disabled for now */}
      {/* {quality.level !== 'low' && !debug.disableFog && (
        <fog attach='fog' args={['#87CEEB', 20, 80]} />
      )} */}
    </MultiStepRenderer>
  );
};

/**
 * Adaptive lighting system that adjusts based on device capabilities
 * Currently disabled in favor of simple lighting for debugging
 */
// DISABLED: Using simple lighting instead
/*
const AdaptiveLighting: React.FC<{
  capabilities: DeviceCapabilities | null;
  quality: RenderQuality;
}> = ({ capabilities, quality }) => {
  const lightRef = useRef<THREE.DirectionalLight>(null);
  const shadowsEnabled = capabilities?.supportsShadows && quality.shadows;

  // Configure shadow properties after mount
  useFrame(() => {
    if (lightRef.current && shadowsEnabled) {
      const light = lightRef.current;
      light.shadow.camera.left = -50;
      light.shadow.camera.right = 50;
      light.shadow.camera.top = 50;
      light.shadow.camera.bottom = -50;

      const mapSize =
        quality.level === 'low'
          ? 512
          : quality.level === 'medium'
            ? 1024
            : 2048;
      light.shadow.mapSize.width = mapSize;
      light.shadow.mapSize.height = mapSize;
    }
  });

  return (
    <>
      <ambientLight intensity={quality.level === 'low' ? 0.5 : 0.3} />
      <directionalLight
        ref={lightRef}
        position={[10, 20, 10]}
        intensity={quality.level === 'low' ? 0.8 : 1.0}
        castShadow={shadowsEnabled}
      />
      {quality.level !== 'low' && (
        <pointLight position={[0, 10, 0]} intensity={0.3} />
      )}
    </>
  );
};
*/

/**
 * Optimized units renderer
 */
const UnitsRenderer: React.FC<{
  units: GameUnit[];
  onUnitClick: (unit: GameUnit) => void;
  selectedUnitId?: number;
  quality: RenderQuality;
  debug: RenderDebug;
}> = ({ units, onUnitClick, selectedUnitId, quality, debug }) => {
  // In production, this would use instanced rendering for better performance
  return (
    <>
      {units.map(unit => (
        <GameUnitComponent
          key={unit.id}
          unit={{ ...unit, isSelected: selectedUnitId === unit.id }}
          onUnitClick={onUnitClick}
          quality={quality}
          debug={debug}
        />
      ))}
    </>
  );
};

/**
 * Enhanced unit component with quality optimizations
 */
const GameUnitComponent: React.FC<{
  unit: GameUnit & { isSelected: boolean };
  onUnitClick: (unit: GameUnit) => void;
  quality: RenderQuality;
  debug: RenderDebug;
}> = ({ unit, onUnitClick, quality, debug }) => {
  const meshRef = useRef<THREE.Mesh>(null);
  const [hovered, setHovered] = useState(false);
  const [x, z] = HexUtils.hexToPixel(unit.position);
  const y = 0.2;

  const playerColors = ['#ef4444', '#3b82f6', '#10b981', '#f59e0b', '#8b5cf6'];
  const unitColor = playerColors[unit.playerId % playerColors.length];

  // Animate unit based on quality level
  useFrame(({ clock }) => {
    if (meshRef.current && quality.level !== 'low') {
      meshRef.current.position.y =
        y + Math.sin(clock.getElapsedTime() * 2) * 0.05;
    }
  });

  const handleClick = useCallback(
    (event: ThreeEvent<MouseEvent>) => {
      event.stopPropagation();
      onUnitClick(unit);
    },
    [unit, onUnitClick]
  );

  const unitDetail =
    quality.level === 'low' ? 4 : quality.level === 'medium' ? 8 : 16;

  return (
    <group position={[x, y, z]}>
      {/* Unit body */}
      <mesh
        ref={meshRef}
        onClick={handleClick}
        onPointerOver={() => setHovered(true)}
        onPointerOut={() => setHovered(false)}
        castShadow={quality.shadows}
      >
        <boxGeometry args={[0.6, 0.3, 0.6]} />
        <meshLambertMaterial
          color={hovered ? '#ffffff' : unitColor}
          wireframe={debug.showWireframe}
        />
      </mesh>

      {/* Health bar (only for medium+ quality) */}
      {quality.level !== 'low' && (
        <group position={[0, 0.4, 0]}>
          <mesh>
            <planeGeometry args={[0.8, 0.1]} />
            <meshBasicMaterial color='#ff0000' />
          </mesh>
          <mesh position={[-(0.8 - (0.8 * unit.health) / 100) / 2, 0, 0.001]}>
            <planeGeometry args={[0.8 * (unit.health / 100), 0.1]} />
            <meshBasicMaterial color='#00ff00' />
          </mesh>
        </group>
      )}

      {/* Selection indicator */}
      {unit.isSelected && (
        <mesh position={[0, -0.05, 0]}>
          <ringGeometry args={[0.7, 0.8, unitDetail]} />
          <meshBasicMaterial color='#fbbf24' transparent opacity={0.8} />
        </mesh>
      )}
    </group>
  );
};

/**
 * Game UI overlays with performance optimizations
 */
const GameUIOverlays: React.FC<{
  selectedTile: GameTile | null;
  selectedUnit: GameUnit | null;
}> = ({ selectedTile, selectedUnit }) => (
  <>
    {selectedTile && (
      <Html position={[5, 5, 5]} transform={false}>
        <div className='game-info-panel'>
          <h3>Tile Info</h3>
          <p>
            Position: ({selectedTile.hex.q}, {selectedTile.hex.r})
          </p>
          <p>Terrain: {selectedTile.terrain}</p>
          <p>Elevation: {selectedTile.elevation.toFixed(1)}</p>
          {selectedTile.resources && (
            <p>Resources: {selectedTile.resources.join(', ')}</p>
          )}
        </div>
      </Html>
    )}

    {selectedUnit && (
      <Html position={[5, 3, 5]} transform={false}>
        <div className='game-info-panel'>
          <h3>Unit Info</h3>
          <p>Type: {selectedUnit.type}</p>
          <p>Player: {selectedUnit.playerId + 1}</p>
          <p>Health: {selectedUnit.health}%</p>
          <p>
            Position: ({selectedUnit.position.q}, {selectedUnit.position.r})
          </p>
        </div>
      </Html>
    )}
  </>
);

/**
 * Adaptive environment based on device capabilities
 */
const AdaptiveEnvironment: React.FC<{
  capabilities: DeviceCapabilities | null;
  quality: RenderQuality;
}> = ({ capabilities, quality }) => {
  const envPreset =
    quality.level === 'low'
      ? 'dawn'
      : quality.level === 'medium'
        ? 'sunset'
        : capabilities?.supportsHDR
          ? 'studio'
          : 'dawn';

  return <Environment preset={envPreset} />;
};

/**
 * Main Game Canvas component
 */
const GameCanvas: React.FC = () => {
  const renderLogger = useLogger('render', 'GameCanvas');

  const handleInitialized = useCallback(
    (capabilities: DeviceCapabilities, settings: RenderingSettings) => {
      renderLogger.info('Render system initialized successfully', {
        capabilities: {
          supportsHDR: capabilities.supportsHDR,
          supportsShadows: capabilities.supportsShadows,
          maxTextureSize: capabilities.maxTextureSize,
          preferredBackend: capabilities.preferredBackend,
        },
        settings: {
          backend: settings.backend,
          pixelRatio: settings.pixelRatio,
          antialias: settings.antialias,
        },
      });
    },
    [renderLogger]
  );

  const handleInitError = useCallback(
    (error: Error) => {
      renderLogger.error('Render initialization failed', error, {
        errorName: error.name,
        errorMessage: error.message,
        userAgent: navigator.userAgent,
      });
    },
    [renderLogger]
  );

  return (
    <div className='game-canvas'>
      <RenderInitializer
        enableDevTools={LoggingUtils.isDevelopment()}
        onInitialized={handleInitialized}
        onError={handleInitError}
      >
        <Suspense fallback={null}>
          <GameScene />
        </Suspense>
      </RenderInitializer>

      <GameControlsUI />
    </div>
  );
};

/**
 * Game controls UI overlay
 */
const GameControlsUI: React.FC = () => (
  <div className='game-controls'>
    <div className='control-panel'>
      <h3>Game Controls</h3>
      <p>✨ WebGL2/WebGPU optimized rendering</p>
      <p>🎮 Click tiles and units to select them</p>
      <p>🖱️ Mouse: Rotate camera</p>
      <p>🔍 Scroll: Zoom in/out</p>
      <p>🤚 Right-click + drag: Pan</p>
    </div>
  </div>
);

export default GameCanvas;
