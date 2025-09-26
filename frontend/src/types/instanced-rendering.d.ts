/**
 * TypeScript interfaces for instanced rendering with BVH acceleration
 * Provides strong typing for all instanced rendering components
 */

import type {
  BufferGeometry,
  Frustum,
  InstancedMesh,
  Material,
  Matrix4,
  Vector3,
} from 'three';
import type { MeshBVH } from 'three-mesh-bvh';

export interface InstanceData {
  readonly id: number;
  readonly position: Vector3;
  readonly rotation?: Vector3;
  readonly scale?: Vector3;
  readonly matrix: Matrix4;
  readonly userData?: Record<string, unknown>;
}

export interface InstancedRenderConfig {
  readonly geometry: BufferGeometry;
  readonly material: Material;
  readonly maxInstances: number;
  readonly enableBVH: boolean;
  readonly enableFrustumCulling: boolean;
  readonly enableLOD: boolean;
  readonly lodLevels?: readonly number[];
}

export interface BVHSpatialQuery {
  readonly position: Vector3;
  readonly radius?: number;
  readonly maxResults?: number;
  readonly filterPredicate?: (instanceId: number) => boolean;
}

export interface BVHQueryResult {
  readonly instanceId: number;
  readonly distance: number;
  readonly data: InstanceData;
}

export interface InstancedBVHStats {
  totalInstances: number;
  visibleInstances: number;
  culledInstances: number;
  bvhNodeCount: number;
  lastQueryTime: number;
  renderTime: number;
}

export interface InstancedBVHManagerOptions {
  readonly config: InstancedRenderConfig;
  readonly autoUpdate: boolean;
  readonly spatialHashSize?: number;
  readonly debugMode?: boolean;
}

export interface InstancedMeshWithBVH extends InstancedMesh {
  bvh?: MeshBVH;
  instanceData: Map<number, InstanceData>;
  visibilityMask: boolean[];
  stats: InstancedBVHStats;
}

export interface SpatialUpdateEvent {
  readonly type:
    | 'instance-added'
    | 'instance-removed'
    | 'instance-moved'
    | 'bvh-rebuilt';
  readonly instanceId?: number;
  readonly timestamp: number;
}

export type InstancedRenderingEventHandler = (
  event: SpatialUpdateEvent
) => void;

/**
 * Per-instance tile data for GPU streaming
 */
export interface TileInstanceData {
  readonly tileId: number;
  readonly position: readonly [number, number, number];
  readonly color: readonly [number, number, number];
  readonly height: number;
  readonly biome: number;
  readonly resourceMask: number;
  readonly lodLevel: number;
  readonly flags: number;
  readonly lastUpdated: number;
}

/**
 * Streaming configuration for tile data
 */
export interface StreamingConfig {
  readonly maxInstances: number;
  readonly streamingRadius: number;
  readonly updateFrequencyHz: number;
  readonly batchSize: number;
  readonly enableCompression: boolean;
}

/**
 * Streaming performance metrics
 */
export interface StreamingMetrics {
  instancesStreamed: number;
  instancesUpdated: number;
  streamingTimeMs: number;
  gpuMemoryMB: number;
  cacheHits: number;
  cacheMisses: number;
}

/**
 * Culling bounds for instance streaming
 */
export interface CullingBounds {
  frustum?: Frustum;
  readonly center: Vector3;
  readonly radius: number;
  readonly minLOD: number;
  readonly maxLOD: number;
}

/**
 * LOD level configuration
 */
export interface LODLevel {
  readonly distance: number;
  readonly quality: number;
}
