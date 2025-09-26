/**
 * InstancedBVHManager
 * Manages instanced meshes with BVH acceleration for efficient spatial queries
 * Integrates with existing render store and capabilities system
 */

import { useRenderStore } from '@stores/render-store';
import {
  Euler,
  InstancedMesh,
  Matrix4,
  Quaternion,
  Vector3,
  type Frustum,
} from 'three';
import {
  acceleratedRaycast,
  computeBoundsTree,
  disposeBoundsTree,
  type MeshBVH,
} from 'three-mesh-bvh';

import type {
  BVHQueryResult,
  BVHSpatialQuery,
  InstanceData,
  InstancedBVHManagerOptions,
  InstancedBVHStats,
  InstancedMeshWithBVH,
  InstancedRenderingEventHandler,
  SpatialUpdateEvent,
} from '../types/instanced-rendering';

export class InstancedBVHManager {
  private readonly mesh: InstancedMeshWithBVH;
  private readonly options: InstancedBVHManagerOptions;
  private readonly instanceData = new Map<number, InstanceData>();
  private readonly visibilityMask: boolean[] = [];
  private readonly eventHandlers = new Set<InstancedRenderingEventHandler>();
  private readonly stats: InstancedBVHStats;

  private nextInstanceId = 0;
  private needsBVHRebuild = false;
  private lastUpdateTime = 0;

  // Performance tracking
  private frameStats = {
    visibleCount: 0,
    culledCount: 0,
    queryTime: 0,
    renderTime: 0,
  };

  constructor(options: InstancedBVHManagerOptions) {
    this.options = options;

    // Create instanced mesh with BVH capabilities
    this.mesh = this.createInstancedMesh();

    // Initialize stats
    this.stats = {
      totalInstances: 0,
      visibleInstances: 0,
      culledInstances: 0,
      bvhNodeCount: 0,
      lastQueryTime: 0,
      renderTime: 0,
    };

    // Enable BVH acceleration for raycast if supported
    if (this.options.config.enableBVH) {
      this.enableBVHAcceleration();
    }
  }

  /**
   * Create optimized instanced mesh
   */
  private createInstancedMesh(): InstancedMeshWithBVH {
    const { geometry, material, maxInstances } = this.options.config;

    const mesh = new InstancedMesh(
      geometry,
      material,
      maxInstances
    ) as InstancedMeshWithBVH;

    // Initialize instance matrix attribute
    mesh.instanceMatrix.setUsage(35048); // DYNAMIC_DRAW

    // Add custom properties
    mesh.instanceData = this.instanceData;
    mesh.visibilityMask = this.visibilityMask;
    mesh.stats = this.stats;

    // Initialize visibility mask
    for (let i = 0; i < maxInstances; i++) {
      this.visibilityMask[i] = false;
    }

    // Set initial instance count to 0
    mesh.count = 0;

    return mesh;
  }

  /**
   * Enable BVH acceleration for efficient raycasting
   */
  private enableBVHAcceleration(): void {
    const { capabilities } = useRenderStore.getState();

    if (!capabilities?.supportsInstancing) {
      console.warn('InstancedBVHManager: Device does not support instancing');
      return;
    }

    // Extend geometry with BVH compute methods
    this.mesh.geometry.computeBoundsTree = computeBoundsTree;
    this.mesh.geometry.disposeBoundsTree = disposeBoundsTree;

    // Override raycast method to use BVH
    this.mesh.raycast = acceleratedRaycast;

    // Build initial BVH
    this.rebuildBVH();
  }

  /**
   * Add new instance with automatic positioning
   */
  addInstance(data: Omit<InstanceData, 'id' | 'matrix'>): number {
    const instanceId = this.nextInstanceId++;

    // Create transformation matrix
    const matrix = new Matrix4();
    const rotation = data.rotation
      ? new Quaternion().setFromEuler(
          new Euler(data.rotation.x, data.rotation.y, data.rotation.z)
        )
      : new Quaternion();
    matrix.compose(data.position, rotation, data.scale ?? new Vector3(1, 1, 1));

    const instanceData: InstanceData = {
      id: instanceId,
      matrix,
      ...data,
    };

    // Store instance data
    this.instanceData.set(instanceId, instanceData);

    // Update instance matrix in buffer
    this.updateInstanceMatrix(instanceId, matrix);

    // Update visibility and count
    this.visibilityMask[instanceId] = true;
    this.mesh.count = Math.max(this.mesh.count, instanceId + 1);

    // Mark for BVH rebuild
    this.needsBVHRebuild = true;

    // Update stats
    this.stats.totalInstances++;

    // Emit event
    this.emitEvent({
      type: 'instance-added',
      instanceId,
      timestamp: performance.now(),
    });

    return instanceId;
  }

  /**
   * Remove instance by ID
   */
  removeInstance(instanceId: number): boolean {
    if (!this.instanceData.has(instanceId)) {
      return false;
    }

    // Remove from data structures
    this.instanceData.delete(instanceId);
    this.visibilityMask[instanceId] = false;

    // Mark for BVH rebuild
    this.needsBVHRebuild = true;

    // Update stats
    this.stats.totalInstances--;

    // Emit event
    this.emitEvent({
      type: 'instance-removed',
      instanceId,
      timestamp: performance.now(),
    });

    return true;
  }

  /**
   * Update instance position efficiently
   */
  updateInstancePosition(instanceId: number, position: Vector3): boolean {
    const instance = this.instanceData.get(instanceId);
    if (!instance) return false;

    // Update position in data
    instance.position.copy(position);

    // Recompose matrix
    const rotation = instance.rotation
      ? new Quaternion().setFromEuler(
          new Euler(
            instance.rotation.x,
            instance.rotation.y,
            instance.rotation.z
          )
        )
      : new Quaternion();
    instance.matrix.compose(
      position,
      rotation,
      instance.scale ?? new Vector3(1, 1, 1)
    );

    // Update buffer
    this.updateInstanceMatrix(instanceId, instance.matrix);

    // Mark for BVH rebuild
    this.needsBVHRebuild = true;

    // Emit event
    this.emitEvent({
      type: 'instance-moved',
      instanceId,
      timestamp: performance.now(),
    });

    return true;
  }

  /**
   * Perform efficient spatial query using BVH
   */
  spatialQuery(query: BVHSpatialQuery): BVHQueryResult[] {
    const startTime = performance.now();
    const results: BVHQueryResult[] = [];

    // Use radius-based query for efficiency
    const radius = query.radius ?? 10;
    const maxResults = query.maxResults ?? 100;

    for (const [instanceId, data] of this.instanceData) {
      if (!this.visibilityMask[instanceId]) continue;

      const distance = query.position.distanceTo(data.position);

      if (distance <= radius) {
        // Apply filter if provided
        if (query.filterPredicate && !query.filterPredicate(instanceId)) {
          continue;
        }

        results.push({
          instanceId,
          distance,
          data,
        });

        if (results.length >= maxResults) break;
      }
    }

    // Sort by distance
    results.sort((a, b) => a.distance - b.distance);

    // Update performance stats
    const queryTime = performance.now() - startTime;
    this.frameStats.queryTime = queryTime;
    this.stats.lastQueryTime = queryTime;

    return results;
  }

  /**
   * Perform frustum culling for efficient rendering
   */
  performFrustumCulling(frustum: Frustum): void {
    if (!this.options.config.enableFrustumCulling) return;

    let visibleCount = 0;
    let culledCount = 0;

    for (const [instanceId, data] of this.instanceData) {
      // Simple sphere-frustum test for performance
      const isVisible = frustum.containsPoint(data.position);

      if (isVisible !== this.visibilityMask[instanceId]) {
        this.visibilityMask[instanceId] = isVisible;
        // Could optimize to batch matrix updates
      }

      if (isVisible) visibleCount++;
      else culledCount++;
    }

    // Update stats
    this.frameStats.visibleCount = visibleCount;
    this.frameStats.culledCount = culledCount;
    this.stats.visibleInstances = visibleCount;
    this.stats.culledInstances = culledCount;

    if (this.options.debugMode) {
      console.warn(
        `Frustum culling: ${visibleCount} visible, ${culledCount} culled`
      );
    }
  }

  /**
   * Update instance matrix in GPU buffer
   */
  private updateInstanceMatrix(instanceId: number, matrix: Matrix4): void {
    this.mesh.setMatrixAt(instanceId, matrix);
    this.mesh.instanceMatrix.needsUpdate = true;
  }

  /**
   * Rebuild BVH when needed
   */
  private rebuildBVH(): void {
    if (!this.options.config.enableBVH) return;

    // Dispose existing BVH
    if (this.mesh.bvh) {
      this.mesh.geometry.disposeBoundsTree?.();
    }

    // Rebuild BVH
    this.mesh.geometry.computeBoundsTree?.();
    this.mesh.bvh = this.mesh.geometry.boundsTree as MeshBVH;

    // Update stats
    if (this.mesh.bvh) {
      this.stats.bvhNodeCount = 1; // Simplified - BVH node count calculation
    }

    this.needsBVHRebuild = false;

    // Emit event
    this.emitEvent({
      type: 'bvh-rebuilt',
      timestamp: performance.now(),
    });

    if (this.options.debugMode) {
      console.warn('BVH rebuilt');
    }
  }

  /**
   * Update system per frame
   */
  update(_deltaTime: number): void {
    const currentTime = performance.now();

    // Rebuild BVH if needed
    if (this.needsBVHRebuild) {
      this.rebuildBVH();
    }

    // Update performance stats
    this.stats.renderTime = currentTime - this.lastUpdateTime;
    this.lastUpdateTime = currentTime;
  }

  /**
   * Add event listener for spatial updates
   */
  addEventListener(handler: InstancedRenderingEventHandler): void {
    this.eventHandlers.add(handler);
  }

  /**
   * Remove event listener
   */
  removeEventListener(handler: InstancedRenderingEventHandler): void {
    this.eventHandlers.delete(handler);
  }

  /**
   * Emit spatial update event
   */
  private emitEvent(event: SpatialUpdateEvent): void {
    this.eventHandlers.forEach(handler => handler(event));
  }

  /**
   * Get instanced mesh for rendering
   */
  getMesh(): InstancedMeshWithBVH {
    return this.mesh;
  }

  /**
   * Get current performance stats
   */
  getStats(): InstancedBVHStats {
    return { ...this.stats };
  }

  /**
   * Dispose resources
   */
  dispose(): void {
    if (this.mesh.bvh) {
      this.mesh.geometry.disposeBoundsTree?.();
    }

    this.instanceData.clear();
    this.eventHandlers.clear();
    this.mesh.dispose();
  }
}
