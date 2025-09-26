/**
 * Custom hook for managing real tile data streaming from backend
 * Replaces mock data with actual game world data
 */

import { useCallback, useEffect, useRef, useState } from 'react';
import type { Vector3 } from 'three';

import type { GameTile, HexCoord } from '../utils/game-types';
import {
  TileDataService,
  type TileStreamingRequest,
  type TileStreamingResponse,
} from '../utils/tile-data-service';

interface UseTileStreamingProps {
  /** Camera position for streaming based on view */
  cameraPosition?: Vector3;
  /** Maximum streaming distance */
  maxDistance?: number;
  /** Streaming quality level */
  quality?: 'low' | 'medium' | 'high';
  /** Enable automatic streaming based on camera movement */
  autoStream?: boolean;
}

interface TileStreamingState {
  /** Current tile data */
  tiles: GameTile[];
  /** Streaming status */
  isLoading: boolean;
  /** Any error that occurred */
  error: string | null;
  /** Streaming metrics */
  metrics: {
    tilesLoaded: number;
    lastUpdateTime: number;
    streamingTimeMs: number;
  };
}

/**
 * Hook for managing tile data streaming from the backend
 * Replaces createMockGameWorld with real data
 */
export const useTileStreaming = ({
  cameraPosition,
  maxDistance = 50,
  quality = 'high',
  autoStream = true,
}: UseTileStreamingProps = {}): TileStreamingState & {
  /** Manually trigger tile streaming */
  streamTiles: (center?: HexCoord) => Promise<void>;
  /** Refresh current viewport tiles */
  refreshTiles: () => Promise<void>;
} => {
  // Tile data service instance
  const tileServiceRef = useRef<TileDataService>();
  const lastCameraPositionRef = useRef<Vector3>();

  // State management
  const [state, setState] = useState<TileStreamingState>({
    tiles: [],
    isLoading: false,
    error: null,
    metrics: {
      tilesLoaded: 0,
      lastUpdateTime: 0,
      streamingTimeMs: 0,
    },
  });

  // Initialize tile service
  useEffect(() => {
    tileServiceRef.current ??= new TileDataService();
  }, []);

  /**
   * Convert world position to hex coordinate
   */
  const worldToHex = useCallback((worldPos: Vector3): HexCoord => {
    // Convert 3D world position to hex coordinates
    // This assumes your hex-to-pixel conversion follows a specific pattern
    const { x } = worldPos;
    const { z } = worldPos;

    // Convert pixel coordinates back to hex (reverse of HexUtils.hexToPixel)
    const q = Math.round((Math.sqrt(3) * x - z) / 3);
    const r = Math.round((2 * z) / 3);

    return { q, r };
  }, []);

  /**
   * Stream tiles from backend
   */
  const streamTiles = useCallback(
    async (center?: HexCoord) => {
      if (!tileServiceRef.current) return;

      setState(prev => ({ ...prev, isLoading: true, error: null }));

      try {
        const startTime = performance.now();

        // Determine center position
        let centerHex: HexCoord;
        if (center) {
          centerHex = center;
        } else if (cameraPosition) {
          centerHex = worldToHex(cameraPosition);
        } else {
          centerHex = { q: 0, r: 0 }; // Default to origin
        }

        // Create streaming request
        const request: TileStreamingRequest = {
          cameraPosition: cameraPosition
            ? [cameraPosition.x, cameraPosition.y, cameraPosition.z]
            : [centerHex.q * 3, 0, centerHex.r * 3],
          viewRadius: maxDistance,
          maxTiles:
            quality === 'low' ? 1000 : quality === 'medium' ? 5000 : 20000,
          lodLevels:
            quality === 'low' ? [0] : quality === 'medium' ? [0, 1] : [0, 1, 2],
          generation: 0, // TODO: Track actual generation for change detection
        };

        // Stream tiles from backend
        const response: TileStreamingResponse =
          await tileServiceRef.current.streamTiles(request);

        const endTime = performance.now();
        const streamingTime = endTime - startTime;

        // Validate response
        if (!response.tiles || !Array.isArray(response.tiles)) {
          throw new Error('Invalid tile data received from backend');
        }

        console.warn(
          `🌍 Streamed ${response.tiles.length} tiles in ${streamingTime.toFixed(2)}ms`
        );

        // Update state with real tile data
        setState(prev => ({
          ...prev,
          tiles: [...response.tiles], // Convert readonly array to mutable array
          isLoading: false,
          error: null,
          metrics: {
            tilesLoaded: response.tiles.length,
            lastUpdateTime: Date.now(),
            streamingTimeMs: streamingTime,
          },
        }));
      } catch (error) {
        console.error('Failed to stream tiles:', error);

        setState(prev => ({
          ...prev,
          isLoading: false,
          error:
            error instanceof Error ? error.message : 'Unknown streaming error',
        }));
      }
    },
    [cameraPosition, maxDistance, quality, worldToHex]
  );

  /**
   * Refresh current viewport tiles
   */
  const refreshTiles = useCallback(async () => {
    await streamTiles();
  }, [streamTiles]);

  /**
   * Auto-stream based on camera movement
   */
  useEffect(() => {
    if (!autoStream || !cameraPosition) return;

    const lastPos = lastCameraPositionRef.current;

    // Check if camera moved significantly
    const cameraMoved = !lastPos || cameraPosition.distanceTo(lastPos) > 10;

    if (cameraMoved) {
      // Update last position
      lastCameraPositionRef.current = cameraPosition.clone();

      // Stream tiles for new position
      void streamTiles();
    }
  }, [cameraPosition, autoStream, streamTiles]);

  /**
   * Initial tile load
   */
  useEffect(() => {
    // Load initial tiles on mount
    void streamTiles();
  }, [streamTiles]);

  return {
    ...state,
    streamTiles,
    refreshTiles,
  };
};

export type { TileStreamingState };
