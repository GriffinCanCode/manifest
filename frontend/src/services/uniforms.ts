/**
 * Uniform Management Service
 * Centralized uniform updates following existing service patterns
 */

import { type ShaderMaterial, Vector3 } from 'three';

import { useRenderStore } from '../stores/render-store';

export interface UniformEntry {
  material: ShaderMaterial;
  lastUpdated: number;
  priority: 'low' | 'normal' | 'high';
}

interface UniformStats {
  registered: number;
  updated: number;
  skipped: number;
  errors: number;
}

/**
 * Centralized uniform management service
 * Single source of truth for all shader uniform updates
 */
export class UniformService {
  private readonly materials = new Map<string, UniformEntry>();
  private currentTime = 0;
  private frameCount = 0;

  private stats: UniformStats = {
    registered: 0,
    updated: 0,
    skipped: 0,
    errors: 0,
  };

  /**
   * Register material for uniform updates
   */
  register(
    id: string,
    material: ShaderMaterial,
    priority: UniformEntry['priority'] = 'normal'
  ): void {
    if (this.materials.has(id)) {
      return; // Already registered
    }

    this.materials.set(id, {
      material,
      lastUpdated: 0,
      priority,
    });

    this.stats.registered++;
  }

  /**
   * Unregister material from updates
   */
  unregister(id: string): void {
    const entry = this.materials.get(id);
    if (entry) {
      this.materials.delete(id);
      this.stats.registered--;
    }
  }

  /**
   * Update all registered materials with current frame data
   */
  updateFrame(deltaTime: number, cameraPosition: Vector3): void {
    this.currentTime += deltaTime;
    this.frameCount++;

    // Update materials by priority
    const entries = Array.from(this.materials.entries());
    const priorityOrder = ['high', 'normal', 'low'] as const;

    for (const priority of priorityOrder) {
      const priorityEntries = entries.filter(
        ([, entry]) => entry.priority === priority
      );

      for (const [id, entry] of priorityEntries) {
        try {
          this.updateMaterialUniforms(entry, cameraPosition);
          entry.lastUpdated = this.currentTime;
          this.stats.updated++;
        } catch (error) {
          console.warn(`🚨 UniformService: Failed to update ${id}:`, error);
          this.stats.errors++;
        }
      }
    }

    // Performance-based updates for quality/debug changes
    if (this.frameCount % 60 === 0) {
      this.updatePerformanceUniforms();
    }
  }

  /**
   * Update individual material uniforms
   */
  private updateMaterialUniforms(
    entry: UniformEntry,
    cameraPosition: Vector3
  ): void {
    const { material } = entry;
    const { uniforms } = material;

    if (!uniforms) return;

    // Core time uniform
    if (uniforms.u_time) {
      uniforms.u_time.value = this.currentTime;
    }

    // Camera position
    if (uniforms.u_cameraPosition?.value instanceof Vector3) {
      uniforms.u_cameraPosition.value.copy(cameraPosition);
    }

    // Mark for update
    material.uniformsNeedUpdate = true;
  }

  /**
   * Update performance-dependent uniforms (less frequent)
   */
  private updatePerformanceUniforms(): void {
    const { quality, debug } = useRenderStore.getState();

    this.materials.forEach(entry => {
      const { uniforms } = entry.material;
      if (!uniforms) return;

      // Quality level
      if (uniforms.u_qualityLevel) {
        uniforms.u_qualityLevel.value = this.getQualityLevel(quality.level);
      }

      // Wireframe mode
      if (uniforms.u_wireframe) {
        uniforms.u_wireframe.value = debug.showWireframe;
      }

      // LOD bias
      if (uniforms.u_lodBias) {
        uniforms.u_lodBias.value = quality.lodBias;
      }

      entry.material.uniformsNeedUpdate = true;
    });
  }

  /**
   * Get quality level number
   */
  private getQualityLevel(level: string): number {
    const levels = { low: 1, medium: 2, high: 3, ultra: 4 };
    return levels[level as keyof typeof levels] ?? 2;
  }

  /**
   * Cleanup expired materials
   */
  cleanup(): void {
    const maxAge = 5 * 60 * 1000; // 5 minutes
    const now = Date.now();
    const expired = Array.from(this.materials.entries()).filter(
      ([, entry]) => now - entry.lastUpdated > maxAge
    );

    expired.forEach(([id]) => {
      this.materials.delete(id);
      this.stats.registered--;
    });

    if (expired.length > 0) {
      console.warn(
        `🧹 UniformService: Cleaned ${expired.length} expired materials`
      );
    }
  }

  /**
   * Get service statistics
   */
  getStats(): UniformStats & { activeCount: number } {
    return {
      ...this.stats,
      activeCount: this.materials.size,
    };
  }

  /**
   * Clear all registered materials
   */
  clear(): void {
    this.materials.clear();
    this.stats = {
      registered: 0,
      updated: 0,
      skipped: 0,
      errors: 0,
    };
  }
}

// Singleton instance following existing patterns
export const uniformService = new UniformService();
