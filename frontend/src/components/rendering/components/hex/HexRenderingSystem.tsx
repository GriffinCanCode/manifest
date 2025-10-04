/**
 * Hex Rendering System
 * Coordinates all hex rendering components following component patterns
 */

import React, { useMemo } from 'react';

import { useRenderStore } from '../../../../stores/render-store';
import { TerrainType, type GameTile } from '../../../../utils/game-types';
import { throttledLog } from '../../../../utils/throttled-logger';

import { HexInstanceRenderer } from './HexInstanceRenderer';
import { AdaptiveHexWaterRenderer } from './HexWaterRenderer';
// Import when needed: import { HexBorderRenderer } from './HexBorderRenderer';
// Import when needed: import { HexTextRenderer } from './HexTextRenderer';

interface HexRenderingSystemProps {
  readonly tiles: readonly GameTile[];
  readonly onTileClick?: (tile: GameTile) => void;
  readonly selectedTileId?: number;
  readonly highlightedTiles?: ReadonlySet<number>;
  readonly isLoading?: boolean;
  readonly cameraPosition?: THREE.Vector3;
  readonly maxInstances?: number;
}

interface RenderingLayers {
  terrain: readonly GameTile[];
  water: readonly GameTile[];
  selected: readonly GameTile[];
  highlighted: readonly GameTile[];
}

/**
 * Coordinated hex rendering system
 * Manages all hex renderers with shared state and performance optimization
 */
export const HexRenderingSystem: React.FC<HexRenderingSystemProps> = ({
  tiles,
  onTileClick,
  selectedTileId,
  highlightedTiles = new Set(),
  isLoading = false,
  cameraPosition,
  maxInstances = 10000,
}) => {
  const { quality, debug } = useRenderStore();

  // Organize tiles by rendering layers
  const layers = useMemo((): RenderingLayers => {
    const terrain: GameTile[] = [];
    const water: GameTile[] = [];
    const selected: GameTile[] = [];
    const highlighted: GameTile[] = [];

    tiles.forEach(tile => {
      // Categorize by terrain
      if (tile.terrain === TerrainType.Ocean) {
        water.push(tile);
      } else {
        terrain.push(tile);
      }

      // Selection state
      if (selectedTileId && tile.id === selectedTileId) {
        selected.push(tile);
      }
      if (highlightedTiles.has(tile.id)) {
        highlighted.push(tile);
      }
    });

    return { terrain, water, selected, highlighted };
  }, [tiles, selectedTileId, highlightedTiles]);

  // Performance monitoring
  React.useEffect(() => {
    throttledLog(
      'hex-system-layers',
      'log',
      `🎮 HexRenderingSystem: ${layers.terrain.length} terrain, ${layers.water.length} water`,
      [],
      30000
    );
  }, [layers.terrain.length, layers.water.length]);

  // Don't render if no tiles
  if (tiles.length === 0) {
    if (!isLoading) {
      throttledLog(
        'hex-system-no-tiles',
        'warn',
        '📭 HexRenderingSystem: No tiles to render',
        [],
        10000
      );
    }
    return null;
  }

  return (
    <group name='hex-rendering-system'>
      {/* Main terrain rendering */}
      <HexInstanceRenderer
        tiles={layers.terrain}
        onTileClick={onTileClick}
        selectedTileId={selectedTileId}
        highlightedTiles={highlightedTiles}
        maxInstances={maxInstances}
        enableSpatialQueries
        enableStreaming
        isLoading={isLoading}
        cameraPosition={cameraPosition}
      />

      {/* Water rendering with advanced shaders */}
      <AdaptiveHexWaterRenderer
        tiles={layers.water}
        maxInstances={Math.floor(maxInstances * 0.3)} // 30% allocation for water
        waveHeight={quality.level === 'low' ? 0.1 : 0.2}
        waveSpeed={quality.level === 'low' ? 0.5 : 1.0}
        transparency={0.8}
      />

      {/* Border rendering - DISABLED: due to WebGL context loss issues */}
      {/* 
      {quality.level !== 'low' && capabilities?.supportsWebGL2 && (
        <HexBorderRenderer
          tiles={tiles}
          selectedTileId={selectedTileId}
          highlightedTiles={highlightedTiles}
          showTerrainBorders={debug.showBounds}
          showSelectionBorder={!!selectedTileId}
          borderWidth={0.02}
          maxRenderDistance={50}
        />
      )}
      */}

      {/* Text rendering - DISABLED: for performance */}
      {/* 
      {quality.level === 'ultra' && debug.showStats && (
        <HexTextRenderer
          tiles={tiles}
          showCoordinates={debug.showStats}
          showResources={false}
          maxTextDistance={15}
          fontSize={0.3}
        />
      )}
      */}

      {/* Performance indicator */}
      {debug.showStats && (
        <mesh position={[0, 5, 0]}>
          <sphereGeometry args={[0.2, 8, 6]} />
          <meshBasicMaterial
            color={
              isLoading ? '#fbbf24' : tiles.length > 0 ? '#22c55e' : '#ef4444'
            }
          />
        </mesh>
      )}
    </group>
  );
};
