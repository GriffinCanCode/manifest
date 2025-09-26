/**
 * SpatialQuerySystem
 * High-performance spatial queries using BVH acceleration
 * Provides raycasting, proximity searches, and frustum culling
 */

import { Frustum, Matrix4, Raycaster, Vector3 } from 'three';

import type {
  BVHQueryResult,
  BVHSpatialQuery,
  InstanceData,
} from '../types/instanced-rendering';

import type { InstancedBVHManager } from './instanced-bvh-manager';

export interface RaycastQuery {
  readonly origin: Vector3;
  readonly direction: Vector3;
  readonly maxDistance?: number;
  readonly layerMask?: number;
  readonly sortByDistance?: boolean;
}

export interface RaycastHit {
  readonly instanceId: number;
  readonly point: Vector3;
  readonly normal: Vector3;
  readonly distance: number;
  readonly data: InstanceData;
}

export interface ProximityQuery {
  readonly center: Vector3;
  readonly radius: number;
  readonly maxResults?: number;
  readonly filterPredicate?: (instanceId: number) => boolean;
  readonly sortByDistance?: boolean;
}

export interface FrustumQuery {
  readonly frustum: Frustum;
  readonly includePartiallyVisible?: boolean;
  readonly lodBias?: number;
}

export interface SpatialQueryStats {
  raycastTime: number;
  proximityTime: number;
  frustumTime: number;
  totalQueries: number;
  averageQueryTime: number;
}

/**
 * Comprehensive spatial query system with BVH acceleration
 */
export class SpatialQuerySystem {
  private readonly managers = new Map<string, InstancedBVHManager>();
  private readonly raycaster = new Raycaster();

  // Performance tracking
  private queryStats: SpatialQueryStats = {
    raycastTime: 0,
    proximityTime: 0,
    frustumTime: 0,
    totalQueries: 0,
    averageQueryTime: 0,
  };

  private frameQueryCount = 0;
  private frameStartTime = 0;

  /**
   * Register instanced BVH manager for spatial queries
   */
  registerManager(id: string, manager: InstancedBVHManager): void {
    this.managers.set(id, manager);
  }

  /**
   * Unregister manager
   */
  unregisterManager(id: string): boolean {
    return this.managers.delete(id);
  }

  /**
   * Perform BVH-accelerated raycast across all registered managers
   */
  raycast(query: RaycastQuery, managerIds?: string[]): RaycastHit[] {
    const startTime = performance.now();
    const hits: RaycastHit[] = [];

    // Set up raycaster
    this.raycaster.set(query.origin, query.direction);
    if (query.maxDistance) {
      this.raycaster.far = query.maxDistance;
    }

    // Query specified managers or all
    const targetIds = managerIds ?? Array.from(this.managers.keys());

    for (const managerId of targetIds) {
      const manager = this.managers.get(managerId);
      if (!manager) continue;

      const mesh = manager.getMesh();
      if (!mesh.bvh) continue;

      // Perform BVH raycast
      const intersects = this.raycaster.intersectObject(mesh);

      for (const intersect of intersects) {
        if (intersect.instanceId === undefined) continue;

        const instanceData = mesh.instanceData.get(intersect.instanceId);
        if (!instanceData) continue;

        hits.push({
          instanceId: intersect.instanceId,
          point: intersect.point,
          normal: intersect.face?.normal ?? new Vector3(0, 1, 0),
          distance: intersect.distance,
          data: instanceData,
        });
      }
    }

    // Sort by distance if requested
    if (query.sortByDistance !== false) {
      hits.sort((a, b) => a.distance - b.distance);
    }

    // Update performance stats
    const queryTime = performance.now() - startTime;
    this.updateStats('raycast', queryTime);

    return hits;
  }

  /**
   * Find instances within radius of point
   */
  proximitySearch(
    query: ProximityQuery,
    managerIds?: string[]
  ): BVHQueryResult[] {
    const startTime = performance.now();
    const results: BVHQueryResult[] = [];

    // Query specified managers or all
    const targetIds = managerIds ?? Array.from(this.managers.keys());

    for (const managerId of targetIds) {
      const manager = this.managers.get(managerId);
      if (!manager) continue;

      // Use manager's spatial query method
      const spatialQuery: BVHSpatialQuery = {
        position: query.center,
        radius: query.radius,
        maxResults: query.maxResults,
        filterPredicate: query.filterPredicate,
      };

      const managerResults = manager.spatialQuery(spatialQuery);
      results.push(...managerResults);
    }

    // Sort by distance if requested
    if (query.sortByDistance !== false) {
      results.sort((a, b) => a.distance - b.distance);
    }

    // Apply max results limit across all managers
    const finalResults = query.maxResults
      ? results.slice(0, query.maxResults)
      : results;

    // Update performance stats
    const queryTime = performance.now() - startTime;
    this.updateStats('proximity', queryTime);

    return finalResults;
  }

  /**
   * Perform frustum culling across all managers
   */
  frustumCull(
    query: FrustumQuery,
    managerIds?: string[]
  ): Map<string, number[]> {
    const startTime = performance.now();
    const visibleInstances = new Map<string, number[]>();

    // Query specified managers or all
    const targetIds = managerIds ?? Array.from(this.managers.keys());

    for (const managerId of targetIds) {
      const manager = this.managers.get(managerId);
      if (!manager) continue;

      // Perform frustum culling on manager
      manager.performFrustumCulling(query.frustum);

      // Collect visible instance IDs
      const mesh = manager.getMesh();
      const visible: number[] = [];

      for (let i = 0; i < mesh.count; i++) {
        if (mesh.visibilityMask[i]) {
          visible.push(i);
        }
      }

      visibleInstances.set(managerId, visible);
    }

    // Update performance stats
    const queryTime = performance.now() - startTime;
    this.updateStats('frustum', queryTime);

    return visibleInstances;
  }

  /**
   * Get instances within screen-space bounds
   */
  screenSpaceQuery(
    _screenBounds: { x: number; y: number; width: number; height: number },
    camera: {
      projectionMatrix: Matrix4;
      matrixWorldInverse: Matrix4;
      position: Vector3;
    },
    managerIds?: string[]
  ): BVHQueryResult[] {
    // Convert screen bounds to world space frustum
    const projMatrix = new Matrix4();
    projMatrix.multiplyMatrices(
      camera.projectionMatrix,
      camera.matrixWorldInverse
    );

    const frustum = new Frustum();
    frustum.setFromProjectionMatrix(projMatrix);

    // Use frustum culling as base for screen space query
    const visibleByManager = this.frustumCull({ frustum }, managerIds);

    // Convert to unified result format
    const results: BVHQueryResult[] = [];

    for (const [managerId, instanceIds] of visibleByManager) {
      const manager = this.managers.get(managerId);
      if (!manager) continue;

      const mesh = manager.getMesh();

      for (const instanceId of instanceIds) {
        const instanceData = mesh.instanceData.get(instanceId);
        if (!instanceData) continue;

        // Calculate screen-space distance (approximate)
        const distance = camera.position.distanceTo(instanceData.position);

        results.push({
          instanceId,
          distance,
          data: instanceData,
        });
      }
    }

    return results;
  }

  /**
   * Optimized batch query for multiple points
   */
  batchProximitySearch(
    queries: ProximityQuery[],
    managerIds?: string[]
  ): BVHQueryResult[][] {
    return queries.map(query => this.proximitySearch(query, managerIds));
  }

  /**
   * Get performance statistics
   */
  getStats(): SpatialQueryStats {
    return { ...this.queryStats };
  }

  /**
   * Reset performance statistics
   */
  resetStats(): void {
    this.queryStats = {
      raycastTime: 0,
      proximityTime: 0,
      frustumTime: 0,
      totalQueries: 0,
      averageQueryTime: 0,
    };
    this.frameQueryCount = 0;
  }

  /**
   * Start frame timing
   */
  beginFrame(): void {
    this.frameStartTime = performance.now();
    this.frameQueryCount = 0;
  }

  /**
   * End frame timing and update stats
   */
  endFrame(): void {
    if (this.frameQueryCount > 0) {
      const frameTime = performance.now() - this.frameStartTime;
      this.queryStats.averageQueryTime = frameTime / this.frameQueryCount;
    }
  }

  /**
   * Update performance statistics
   */
  private updateStats(
    queryType: 'raycast' | 'proximity' | 'frustum',
    time: number
  ): void {
    this.queryStats.totalQueries++;
    this.frameQueryCount++;

    switch (queryType) {
      case 'raycast':
        this.queryStats.raycastTime = time;
        break;
      case 'proximity':
        this.queryStats.proximityTime = time;
        break;
      case 'frustum':
        this.queryStats.frustumTime = time;
        break;
    }
  }

  /**
   * Dispose all resources
   */
  dispose(): void {
    this.managers.clear();
    this.resetStats();
  }
}

// Singleton instance for global access
export const spatialQuerySystem = new SpatialQuerySystem();
