/**
 * Instance Data Streamer
 * Efficiently streams per-instance tile data from backend to GPU
 * Handles LOD, culling, and incremental updates for optimal performance
 */

import { InstancedBufferAttribute, Vector3, type BufferGeometry } from 'three';

import type {
  CullingBounds,
  LODLevel,
  StreamingMetrics,
  TileInstanceData,
} from '../types/instanced-rendering';

import type { GameTile } from './game-types';

export interface InstanceDataStreamerOptions {
  readonly maxInstances: number;
  readonly maxStreamingDistance: number;
  readonly lodLevels: readonly LODLevel[];
  readonly cullingEnabled: boolean;
  readonly incrementalUpdates: boolean;
  readonly batchSize: number;
}

/**
 * Streams and manages per-instance data for efficient GPU rendering
 * Optimizes bandwidth and memory usage through smart culling and LOD
 */
export class InstanceDataStreamer {
  private readonly options: InstanceDataStreamerOptions;
  private readonly instanceData = new Map<number, TileInstanceData>();
  private readonly dirtyInstances = new Set<number>();
  private readonly visibleInstances = new Set<number>();
  private readonly streamingQueue: number[] = [];

  // Instance buffers for GPU upload
  private positionBuffer?: InstancedBufferAttribute;
  private colorBuffer?: InstancedBufferAttribute;
  private heightBuffer?: InstancedBufferAttribute;
  private biomeBuffer?: InstancedBufferAttribute;
  private resourceBuffer?: InstancedBufferAttribute;
  private metaBuffer?: InstancedBufferAttribute;

  // Streaming state
  private lastCameraPosition = new Vector3();
  private frameCount = 0;
  private metrics: StreamingMetrics = {
    instancesStreamed: 0,
    instancesUpdated: 0,
    streamingTimeMs: 0,
    gpuMemoryMB: 0,
    cacheHits: 0,
    cacheMisses: 0,
  };

  constructor(options: InstanceDataStreamerOptions) {
    this.options = options;
    this.initializeBuffers();
  }

  /**
   * Initialize GPU instance buffers
   */
  private initializeBuffers(): void {
    const { maxInstances } = this.options;

    // Position: vec3 (x, y, z)
    this.positionBuffer = new InstancedBufferAttribute(
      new Float32Array(maxInstances * 3),
      3
    );

    // Color: vec3 (r, g, b)
    this.colorBuffer = new InstancedBufferAttribute(
      new Float32Array(maxInstances * 3),
      3
    );

    // Height: float (elevation)
    this.heightBuffer = new InstancedBufferAttribute(
      new Float32Array(maxInstances),
      1
    );

    // Biome: float (biome ID)
    this.biomeBuffer = new InstancedBufferAttribute(
      new Float32Array(maxInstances),
      1
    );

    // Resource mask: float (packed resource flags)
    this.resourceBuffer = new InstancedBufferAttribute(
      new Float32Array(maxInstances),
      1
    );

    // Meta data: vec2 (LOD level, flags)
    this.metaBuffer = new InstancedBufferAttribute(
      new Float32Array(maxInstances * 2),
      2
    );

    // Mark all buffers as dynamic for efficient updates
    this.positionBuffer.setUsage(35048); // THREE.DynamicDrawUsage
    this.colorBuffer.setUsage(35048);
    this.heightBuffer.setUsage(35048);
    this.biomeBuffer.setUsage(35048);
    this.resourceBuffer.setUsage(35048);
    this.metaBuffer.setUsage(35048);
  }

  /**
   * Stream tiles based on camera position and culling bounds
   */
  public streamTiles(
    tiles: readonly GameTile[],
    cameraPosition: Vector3,
    cullingBounds: CullingBounds
  ): void {
    const startTime = performance.now();
    this.frameCount++;

    // Check if we need to stream (camera moved or tiles changed)
    const shouldStream = this.shouldStreamThisFrame(cameraPosition, tiles);
    if (!shouldStream) {
      return;
    }

    // Update visible instances based on culling
    this.updateVisibleInstances(tiles, cameraPosition, cullingBounds);

    // Process streaming queue in batches
    this.processStreamingQueue();

    // Update GPU buffers if needed
    this.updateInstanceBuffers();

    // Update metrics
    const endTime = performance.now();
    this.metrics.streamingTimeMs = endTime - startTime;
    this.lastCameraPosition.copy(cameraPosition);
  }

  /**
   * Check if we should stream this frame
   */
  private shouldStreamThisFrame(
    cameraPosition: Vector3,
    tiles: readonly GameTile[]
  ): boolean {
    // Stream every frame in debug mode
    if (this.frameCount % 60 === 0) return true; // Every second at 60fps

    // Stream if camera moved significantly
    const cameraMoved =
      cameraPosition.distanceTo(this.lastCameraPosition) > 5.0;
    if (cameraMoved) return true;

    // Stream if we have dirty instances
    if (this.dirtyInstances.size > 0) return true;

    // Stream if tiles were added/removed
    const currentTileCount = tiles.length;
    const previousTileCount = this.instanceData.size;
    if (currentTileCount !== previousTileCount) return true;

    return false;
  }

  /**
   * Update which instances are visible based on culling
   */
  private updateVisibleInstances(
    tiles: readonly GameTile[],
    cameraPosition: Vector3,
    cullingBounds: CullingBounds
  ): void {
    this.visibleInstances.clear();
    this.streamingQueue.length = 0;

    for (const tile of tiles) {
      const tilePos = new Vector3(
        tile.worldX,
        tile.elevation * 0.5,
        tile.worldZ
      );

      // Distance culling
      const distance = cameraPosition.distanceTo(tilePos);
      if (distance > this.options.maxStreamingDistance) {
        continue;
      }

      // Frustum culling (if enabled)
      if (this.options.cullingEnabled && cullingBounds.frustum) {
        if (!cullingBounds.frustum.containsPoint(tilePos)) {
          continue;
        }
      }

      // Mark as visible
      this.visibleInstances.add(tile.id);

      // Add to streaming queue if not already streamed or dirty
      if (!this.instanceData.has(tile.id) || this.dirtyInstances.has(tile.id)) {
        this.streamingQueue.push(tile.id);
      }
    }
  }

  /**
   * Process streaming queue in batches for performance
   */
  private processStreamingQueue(): void {
    const batchSize = Math.min(
      this.options.batchSize,
      this.streamingQueue.length
    );

    for (let i = 0; i < batchSize; i++) {
      const tileId = this.streamingQueue.shift();
      if (tileId === undefined) break;

      this.streamTileData(tileId);
    }
  }

  /**
   * Stream data for a specific tile
   */
  private streamTileData(tileId: number): void {
    // This would typically fetch from backend via IPC
    // For now, we'll simulate with placeholder data
    const instanceData: TileInstanceData = {
      tileId,
      position: [0, 0, 0], // Will be set from actual tile data
      color: [1, 1, 1],
      height: 0,
      biome: 0,
      resourceMask: 0,
      lodLevel: this.calculateLOD(tileId),
      flags: 0,
      lastUpdated: Date.now(),
    };

    this.instanceData.set(tileId, instanceData);
    this.dirtyInstances.add(tileId);
    this.metrics.instancesStreamed++;
  }

  /**
   * Calculate LOD level based on distance and tile importance
   */
  private calculateLOD(_tileId: number): number {
    // Placeholder LOD calculation
    // Would be based on distance, tile importance, etc.
    return 0;
  }

  /**
   * Update GPU instance buffers with dirty data
   */
  private updateInstanceBuffers(): void {
    if (this.dirtyInstances.size === 0) return;

    let bufferIndex = 0;
    const visibleInstancesArray = Array.from(this.visibleInstances);

    for (const tileId of visibleInstancesArray) {
      const data = this.instanceData.get(tileId);
      if (!data) continue;

      if (bufferIndex >= this.options.maxInstances) {
        // Exceeded max instances limit
        break;
      }

      // Update position buffer
      if (this.positionBuffer) {
        this.positionBuffer.setXYZ(
          bufferIndex,
          data.position[0],
          data.position[1],
          data.position[2]
        );
      }

      // Update color buffer
      if (this.colorBuffer) {
        this.colorBuffer.setXYZ(
          bufferIndex,
          data.color[0],
          data.color[1],
          data.color[2]
        );
      }

      // Update height buffer
      if (this.heightBuffer) {
        this.heightBuffer.setX(bufferIndex, data.height);
      }

      // Update biome buffer
      if (this.biomeBuffer) {
        this.biomeBuffer.setX(bufferIndex, data.biome);
      }

      // Update resource buffer
      if (this.resourceBuffer) {
        this.resourceBuffer.setX(bufferIndex, data.resourceMask);
      }

      // Update meta buffer (LOD, flags)
      if (this.metaBuffer) {
        this.metaBuffer.setXY(bufferIndex, data.lodLevel, data.flags);
      }

      bufferIndex++;
    }

    // Mark buffers as needing update
    if (this.positionBuffer) this.positionBuffer.needsUpdate = true;
    if (this.colorBuffer) this.colorBuffer.needsUpdate = true;
    if (this.heightBuffer) this.heightBuffer.needsUpdate = true;
    if (this.biomeBuffer) this.biomeBuffer.needsUpdate = true;
    if (this.resourceBuffer) this.resourceBuffer.needsUpdate = true;
    if (this.metaBuffer) this.metaBuffer.needsUpdate = true;

    // Update metrics
    this.metrics.instancesUpdated = bufferIndex;
    this.metrics.gpuMemoryMB = this.calculateGPUMemoryUsage();

    // Clear dirty instances
    this.dirtyInstances.clear();
  }

  /**
   * Calculate approximate GPU memory usage
   */
  private calculateGPUMemoryUsage(): number {
    const visibleCount = this.visibleInstances.size;
    const bytesPerInstance = (3 + 3 + 1 + 1 + 1 + 2) * 4; // 11 floats * 4 bytes
    return (visibleCount * bytesPerInstance) / (1024 * 1024); // Convert to MB
  }

  /**
   * Attach buffers to geometry
   */
  public attachToGeometry(geometry: BufferGeometry): void {
    if (this.positionBuffer) {
      geometry.setAttribute('instancePosition', this.positionBuffer);
    }
    if (this.colorBuffer) {
      geometry.setAttribute('instanceColor', this.colorBuffer);
    }
    if (this.heightBuffer) {
      geometry.setAttribute('instanceHeight', this.heightBuffer);
    }
    if (this.biomeBuffer) {
      geometry.setAttribute('instanceBiome', this.biomeBuffer);
    }
    if (this.resourceBuffer) {
      geometry.setAttribute('instanceResourceMask', this.resourceBuffer);
    }
    if (this.metaBuffer) {
      geometry.setAttribute('instanceMeta', this.metaBuffer);
    }
  }

  /**
   * Get streaming metrics for debugging
   */
  public getMetrics(): StreamingMetrics {
    return { ...this.metrics };
  }

  /**
   * Cleanup resources
   */
  public dispose(): void {
    this.instanceData.clear();
    this.dirtyInstances.clear();
    this.visibleInstances.clear();
    this.streamingQueue.length = 0;

    // Clear buffer references (InstancedBufferAttribute doesn't have dispose method)
    this.positionBuffer = undefined;
    this.colorBuffer = undefined;
    this.heightBuffer = undefined;
    this.biomeBuffer = undefined;
    this.resourceBuffer = undefined;
    this.metaBuffer = undefined;
  }
}

/**
 * Default configuration for instance data streaming
 */
export const DEFAULT_STREAMING_CONFIG: InstanceDataStreamerOptions = {
  maxInstances: 50000,
  maxStreamingDistance: 1000,
  lodLevels: [
    { distance: 100, quality: 1.0 },
    { distance: 500, quality: 0.5 },
    { distance: 1000, quality: 0.25 },
  ],
  cullingEnabled: true,
  incrementalUpdates: true,
  batchSize: 100,
};
