/**
 * Debug hook to monitor tile data flow in React components
 */

import { useEffect, useRef } from 'react';

import type { GameTile } from './game-types';
import { throttledLog } from './throttled-logger';

interface TileDebugInfo {
  tilesCount: number;
  lastUpdate: number;
  sampleTiles: GameTile[];
  terrainCounts: Record<string, number>;
  renderCount: number;
}

export function useTileDebug(
  tiles: readonly GameTile[],
  componentName: string
) {
  const debugRef = useRef<TileDebugInfo>({
    tilesCount: 0,
    lastUpdate: 0,
    sampleTiles: [],
    terrainCounts: {},
    renderCount: 0,
  });

  useEffect(() => {
    debugRef.current.renderCount++;
    debugRef.current.tilesCount = tiles.length;
    debugRef.current.lastUpdate = Date.now();
    debugRef.current.sampleTiles = tiles.slice(0, 5);

    // Count terrain types
    const terrainCounts: Record<string, number> = {};
    tiles.forEach(tile => {
      terrainCounts[tile.terrain] = (terrainCounts[tile.terrain] || 0) + 1;
    });
    debugRef.current.terrainCounts = terrainCounts;

    // Log significant changes (throttled with static keys)
    if (tiles.length > 0) {
      throttledLog(
        `tile-debug-${componentName}`,
        'log',
        `🔍 TILE DEBUG [${componentName}]`,
        [],
        5000
      );
      throttledLog(
        `tile-count-${componentName}`,
        'log',
        `📊 Tiles: ${tiles.length}`,
        [],
        5000
      );
      throttledLog(
        `render-count-${componentName}`,
        'log',
        `🔄 Render: #${debugRef.current.renderCount}`,
        [],
        5000
      );
      throttledLog(
        `terrain-types-${componentName}`,
        'log',
        `🌍 Terrain Types:`,
        [terrainCounts],
        5000
      );
      throttledLog(
        `sample-tiles-${componentName}`,
        'log',
        `📝 Sample Tiles:`,
        [debugRef.current.sampleTiles],
        5000
      );
    } else if (debugRef.current.renderCount > 1) {
      throttledLog(
        `empty-tiles-${componentName}`,
        'warn',
        `❌ ${componentName}: Tiles array is empty (render #${debugRef.current.renderCount})`,
        [],
        5000
      );
    }

    // Expose debug info globally
    if (typeof window !== 'undefined') {
      const win = window as any;
      if (!win.__tileDebug) win.__tileDebug = {};
      win.__tileDebug[componentName] = debugRef.current;
    }
  }, [tiles, componentName]);

  return debugRef.current;
}

// Hook to monitor HexInstanceRenderer specifically
export function useHexRendererDebug(
  tiles: readonly GameTile[],
  isLoading: boolean
) {
  const debugInfo = useTileDebug(tiles, 'HexInstanceRenderer');

  useEffect(() => {
    if (typeof window !== 'undefined') {
      const win = window as any;
      win.__hexRendererDebug = {
        ...debugInfo,
        isLoading,
        hasShaders: !!win.__shaderManager,
        hasTextures: !!win.__textureService,
      };
    }
  }, [debugInfo, isLoading]);

  return debugInfo;
}
