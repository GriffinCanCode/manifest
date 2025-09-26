/**
 * Tile Data Service
 * Handles communication between backend tile system and frontend rendering
 * Provides efficient tile data fetching, caching, and updates via Tauri IPC
 */

import { invoke } from '@tauri-apps/api/core';
import { Vector3 } from 'three';

import type {
  StreamingMetrics,
  TileInstanceData,
} from '../types/instanced-rendering';

import type { GameTile, HexCoord } from './game-types';

export interface TileQuery {
  readonly center: HexCoord;
  readonly radius: number;
  readonly lodLevel?: number;
  readonly includeResources?: boolean;
  readonly includeBiomes?: boolean;
}

export interface TileStreamingRequest {
  readonly cameraPosition: readonly [number, number, number];
  readonly viewRadius: number;
  readonly maxTiles: number;
  readonly lodLevels: readonly number[];
  readonly generation: number; // For change detection
}

export interface TileStreamingResponse {
  readonly tiles: readonly GameTile[];
  readonly instanceData: readonly TileInstanceData[];
  readonly generation: number;
  readonly hasMore: boolean;
  readonly nextOffset?: number;
}

export interface TileUpdateBatch {
  readonly updatedTiles: readonly number[]; // Tile IDs
  readonly removedTiles: readonly number[]; // Tile IDs
  readonly timestamp: number;
}

/**
 * Service for managing tile data between backend and frontend
 * Handles caching, batching, and efficient data transfer
 */
export class TileDataService {
  private readonly tileCache = new Map<number, GameTile>();
  private readonly instanceCache = new Map<number, TileInstanceData>();
  private readonly dirtyTiles = new Set<number>();

  private lastStreamingGeneration = 0;
  private lastUpdateTime = 0;
  private isStreaming = false;

  private metrics: StreamingMetrics = {
    instancesStreamed: 0,
    instancesUpdated: 0,
    streamingTimeMs: 0,
    gpuMemoryMB: 0,
    cacheHits: 0,
    cacheMisses: 0,
  };

  /**
   * Stream tiles based on camera position and requirements
   */
  public async streamTiles(
    request: TileStreamingRequest
  ): Promise<TileStreamingResponse> {
    if (this.isStreaming) {
      console.warn('Tile streaming already in progress, skipping request');
      return this.getCachedResponse(request);
    }

    this.isStreaming = true;
    const startTime = performance.now();

    try {
      // Call backend via Tauri IPC
      const response = await invoke<TileStreamingResponse>('stream_tiles', {
        request,
      });

      // Validate response structure
      if (
        !response ||
        typeof response !== 'object' ||
        !Array.isArray(response.tiles) ||
        !Array.isArray(response.instanceData)
      ) {
        console.error('Invalid response structure from backend');
        return this.getEmptyResponse();
      }

      // Update local caches
      this.updateCaches(response);

      // Update metrics
      const endTime = performance.now();
      this.metrics.streamingTimeMs = endTime - startTime;
      this.metrics.instancesStreamed += response.tiles.length;
      this.lastStreamingGeneration = response.generation;
      this.lastUpdateTime = Date.now();

      return response;
    } catch (error) {
      console.error('Failed to stream tiles from backend:', error);
      return this.getEmptyResponse();
    } finally {
      this.isStreaming = false;
    }
  }

  /**
   * Get tiles within a specific hex radius (synchronous, cached)
   */
  public getTilesInRadius(center: HexCoord, radius: number): GameTile[] {
    const result: GameTile[] = [];

    for (const [_tileId, tile] of this.tileCache) {
      const distance = this.hexDistance(center, tile.hex);
      if (distance <= radius) {
        result.push(tile);
        this.metrics.cacheHits++;
      }
    }

    return result;
  }

  /**
   * Get specific tile by ID (with caching)
   */
  public async getTile(tileId: number): Promise<GameTile | null> {
    // Check cache first
    if (this.tileCache.has(tileId)) {
      this.metrics.cacheHits++;
      return this.tileCache.get(tileId) ?? null;
    }

    try {
      // Fetch from backend
      const tile = await invoke<GameTile | null>('get_tile', { tileId });

      // Validate tile structure
      if (tile && typeof tile === 'object' && 'id' in tile && 'hex' in tile) {
        this.tileCache.set(tileId, tile);
        this.metrics.cacheMisses++;
        return tile;
      }

      return tile;
    } catch (error) {
      console.error(`Failed to fetch tile ${tileId}:`, error);
      return null;
    }
  }

  /**
   * Get instance data for tiles (optimized for rendering)
   */
  public getInstanceData(tileIds: readonly number[]): TileInstanceData[] {
    const result: TileInstanceData[] = [];

    for (const tileId of tileIds) {
      const instanceData = this.instanceCache.get(tileId);
      if (instanceData) {
        result.push(instanceData);
        this.metrics.cacheHits++;
      } else {
        this.metrics.cacheMisses++;
        // Create placeholder data if not cached
        result.push(this.createPlaceholderInstanceData(tileId));
      }
    }

    return result;
  }

  /**
   * Update tile data (for real-time changes)
   */
  public async updateTiles(
    tileIds: readonly number[]
  ): Promise<TileUpdateBatch> {
    try {
      const batch = await invoke<TileUpdateBatch>('get_tile_updates', {
        tileIds: Array.from(tileIds),
        lastUpdateTime: this.lastUpdateTime,
      });

      // Validate batch structure
      if (
        !batch ||
        typeof batch !== 'object' ||
        !Array.isArray(batch.updatedTiles) ||
        !Array.isArray(batch.removedTiles)
      ) {
        console.error('Invalid batch structure from backend');
        return {
          updatedTiles: [],
          removedTiles: [],
          timestamp: Date.now(),
        };
      }

      // Mark tiles as dirty for next streaming operation
      for (const tileId of batch.updatedTiles) {
        if (typeof tileId === 'number') {
          this.dirtyTiles.add(tileId);
          // Invalidate cache
          this.tileCache.delete(tileId);
          this.instanceCache.delete(tileId);
        }
      }

      // Remove deleted tiles from cache
      for (const tileId of batch.removedTiles) {
        if (typeof tileId === 'number') {
          this.tileCache.delete(tileId);
          this.instanceCache.delete(tileId);
          this.dirtyTiles.delete(tileId);
        }
      }

      this.metrics.instancesUpdated += batch.updatedTiles.length;
      return batch;
    } catch (error) {
      console.error('Failed to update tiles:', error);
      return {
        updatedTiles: [],
        removedTiles: [],
        timestamp: Date.now(),
      };
    }
  }

  /**
   * Clear caches and reset state
   */
  public clearCaches(): void {
    this.tileCache.clear();
    this.instanceCache.clear();
    this.dirtyTiles.clear();
    this.lastStreamingGeneration = 0;
    this.resetMetrics();
  }

  /**
   * Get streaming metrics for debugging
   */
  public getMetrics(): StreamingMetrics {
    // Calculate approximate cache memory usage
    const tileCacheSize = this.tileCache.size * 256; // Rough estimate per tile
    const instanceCacheSize = this.instanceCache.size * 128; // Rough estimate per instance
    this.metrics.gpuMemoryMB =
      (tileCacheSize + instanceCacheSize) / (1024 * 1024);

    return { ...this.metrics };
  }

  /**
   * Check if service is currently streaming
   */
  public isCurrentlyStreaming(): boolean {
    return this.isStreaming;
  }

  /**
   * Get dirty tiles that need updates
   */
  public getDirtyTiles(): Set<number> {
    return new Set(this.dirtyTiles);
  }

  // Private helper methods

  private updateCaches(response: TileStreamingResponse): void {
    // Update tile cache
    for (const tile of response.tiles) {
      this.tileCache.set(tile.id, tile);
    }

    // Update instance data cache
    for (const instanceData of response.instanceData) {
      this.instanceCache.set(instanceData.tileId, instanceData);
      this.dirtyTiles.delete(instanceData.tileId); // No longer dirty
    }
  }

  private getCachedResponse(
    request: TileStreamingRequest
  ): TileStreamingResponse {
    const cachedTiles: GameTile[] = [];
    const cachedInstanceData: TileInstanceData[] = [];

    // Return cached data that matches the request
    const centerPos = new Vector3(...request.cameraPosition);

    for (const [tileId, tile] of this.tileCache) {
      const tilePos = new Vector3(tile.worldX, tile.elevation, tile.worldZ);
      const distance = centerPos.distanceTo(tilePos);

      if (
        distance <= request.viewRadius &&
        cachedTiles.length < request.maxTiles
      ) {
        cachedTiles.push(tile);

        const instanceData = this.instanceCache.get(tileId);
        if (instanceData) {
          cachedInstanceData.push(instanceData);
        }
      }
    }

    return {
      tiles: cachedTiles,
      instanceData: cachedInstanceData,
      generation: this.lastStreamingGeneration,
      hasMore: false,
    };
  }

  private getEmptyResponse(): TileStreamingResponse {
    return {
      tiles: [],
      instanceData: [],
      generation: this.lastStreamingGeneration,
      hasMore: false,
    };
  }

  private createPlaceholderInstanceData(tileId: number): TileInstanceData {
    return {
      tileId,
      position: [0, 0, 0],
      color: [0.5, 0.5, 0.5],
      height: 0,
      biome: 0,
      resourceMask: 0,
      lodLevel: 0,
      flags: 0,
      lastUpdated: Date.now(),
    };
  }

  private hexDistance(a: HexCoord, b: HexCoord): number {
    // Hex distance calculation using cube coordinates
    const aq = a.q;
    const ar = a.r;
    const as = -aq - ar;

    const bq = b.q;
    const br = b.r;
    const bs = -bq - br;

    return Math.max(Math.abs(aq - bq), Math.abs(ar - br), Math.abs(as - bs));
  }

  private resetMetrics(): void {
    this.metrics = {
      instancesStreamed: 0,
      instancesUpdated: 0,
      streamingTimeMs: 0,
      gpuMemoryMB: 0,
      cacheHits: 0,
      cacheMisses: 0,
    };
  }
}

/**
 * Global tile data service instance
 */
export const tileDataService = new TileDataService();
