/**
 * Material Service
 * Centralized tile material management following existing patterns
 */

import type { ShaderMaterial } from 'three';
import * as THREE from 'three';

import { getShaderDefinition } from '../shaders/definitions';
import { shaderManager } from '../shaders/manager';
import { useRenderStore } from '../stores/render-store';
import type { TerrainType } from '../utils/game-types';

export interface MaterialConfig {
  terrainType: TerrainType;
  texture?: THREE.Texture;
  useShader?: boolean;
  wireframe?: boolean;
}

export interface MaterialStats {
  cached: number;
  compiled: number;
  textured: number;
  fallback: number;
}

interface MaterialEntry {
  material: THREE.Material;
  terrainType: TerrainType;
  lastUsed: number;
  isShader: boolean;
}

/**
 * Centralized material management service
 * Consolidates shader and texture-based material creation
 */
export class MaterialService {
  private readonly cache = new Map<string, MaterialEntry>();
  private readonly textureCache = new Map<TerrainType, THREE.Texture>();

  private stats: MaterialStats = {
    cached: 0,
    compiled: 0,
    textured: 0,
    fallback: 0,
  };

  /**
   * Get or create material for terrain type
   */
  getMaterial(config: MaterialConfig): THREE.Material {
    const cacheKey = this.getCacheKey(config);
    const existing = this.cache.get(cacheKey);

    if (existing && this.isValidMaterial(existing)) {
      existing.lastUsed = Date.now();
      return existing.material;
    }

    const material = this.createMaterial(config);
    this.cacheMaterial(cacheKey, material, config);

    return material;
  }

  /**
   * Create optimized material based on configuration
   */
  private createMaterial(config: MaterialConfig): THREE.Material {
    const {
      terrainType,
      texture,
      useShader = true,
      wireframe = false,
    } = config;

    try {
      if (useShader && this.canUseShader()) {
        return this.createShaderMaterial(terrainType, texture, wireframe);
      }

      return this.createBasicMaterial(terrainType, texture, wireframe);
    } catch (error) {
      console.warn(`🎨 MaterialService: Fallback for ${terrainType}:`, error);
      this.stats.fallback++;
      return this.createFallbackMaterial(terrainType, wireframe);
    }
  }

  /**
   * Create shader-based material using hex-terrain shader
   */
  private createShaderMaterial(
    terrainType: TerrainType,
    texture?: THREE.Texture,
    wireframe = false
  ): ShaderMaterial {
    const { capabilities, quality, settings } = useRenderStore.getState();
    const hexTerrainDef = getShaderDefinition('hex-terrain');

    const material = shaderManager.compile('hex-terrain', hexTerrainDef, {
      defines: {
        TERRAIN_TYPE: this.getTerrainDefine(terrainType),
        USE_TEXTURE: texture ? 1 : 0,
        QUALITY_LEVEL: this.getQualityLevel(quality.level),
        USE_SHADOWS: settings?.shadows && capabilities?.supportsShadows ? 1 : 0,
        USE_HDR: capabilities?.supportsHDR ? 1 : 0,
      },
      transparent: false,
      depthTest: true,
      depthWrite: true,
    });

    // Update texture uniforms
    if (texture && material.uniforms) {
      const { uniforms } = material;
      if (uniforms.u_hasAlbedoTexture) uniforms.u_hasAlbedoTexture.value = true;
      if (uniforms.u_albedoTexture) uniforms.u_albedoTexture.value = texture;
      if (uniforms.u_textureScale) uniforms.u_textureScale.value = 1.0;
    }

    material.wireframe = wireframe;
    this.stats.compiled++;

    return material;
  }

  /**
   * Create Lambert material with texture fallback
   */
  private createBasicMaterial(
    terrainType: TerrainType,
    texture?: THREE.Texture,
    wireframe = false
  ): THREE.MeshLambertMaterial {
    // Create Lambert material but ensure proper lighting chunks are included
    const material = new THREE.MeshLambertMaterial({
      map: texture ?? null,
      color: this.getTerrainColor(terrainType),
      wireframe,
      transparent: false,
      flatShading: false,
    });

    // Override onBeforeCompile to ensure proper shader chunks are included
    material.onBeforeCompile = shader => {
      // Three.js should automatically include required chunks for Lambert materials
      // If GeometricContext errors persist, we could inject the struct definition here
      if (!shader.fragmentShader.includes('struct GeometricContext')) {
        shader.fragmentShader = shader.fragmentShader.replace(
          'precision highp float;',
          `precision highp float;
          
// GeometricContext struct for lighting calculations
struct GeometricContext {
  vec3 position;
  vec3 normal;
  vec3 viewDir;
};`
        );
      }
    };

    if (texture) this.stats.textured++;

    return material;
  }

  /**
   * Create emergency fallback material
   */
  private createFallbackMaterial(
    terrainType: TerrainType,
    wireframe = false
  ): THREE.MeshBasicMaterial {
    return new THREE.MeshBasicMaterial({
      color: this.getTerrainColor(terrainType),
      wireframe,
      transparent: false,
    });
  }

  /**
   * Cache material with metadata
   */
  private cacheMaterial(
    key: string,
    material: THREE.Material,
    config: MaterialConfig
  ): void {
    const entry: MaterialEntry = {
      material,
      terrainType: config.terrainType,
      lastUsed: Date.now(),
      isShader: material instanceof THREE.ShaderMaterial,
    };

    this.cache.set(key, entry);
    this.stats.cached++;
  }

  /**
   * Generate cache key for material configuration
   */
  private getCacheKey(config: MaterialConfig): string {
    const parts = [
      config.terrainType,
      config.useShader ? 'shader' : 'basic',
      config.texture ? 'textured' : 'color',
      config.wireframe ? 'wire' : 'solid',
    ];

    return parts.join(':');
  }

  /**
   * Validate cached material is still usable
   */
  private isValidMaterial(entry: MaterialEntry): boolean {
    const maxAge = 5 * 60 * 1000; // 5 minutes
    const age = Date.now() - entry.lastUsed;

    return age < maxAge && entry.material.userData?.isDisposed !== true;
  }

  /**
   * Check if shader compilation is available
   */
  private canUseShader(): boolean {
    const { capabilities } = useRenderStore.getState();
    return capabilities?.supportsWebGL2 ?? true;
  }

  /**
   * Get terrain-specific shader define
   */
  private getTerrainDefine(terrainType: TerrainType): number {
    const defines = {
      ocean: 0,
      grassland: 1,
      plains: 2,
      desert: 3,
      tundra: 4,
      snow: 5,
      forest: 6,
      jungle: 7,
      hills: 8,
      mountain: 9,
    };

    return defines[terrainType] ?? 0;
  }

  /**
   * Get quality level number
   */
  private getQualityLevel(level: string): number {
    const levels = { low: 1, medium: 2, high: 3, ultra: 4 };
    return levels[level as keyof typeof levels] ?? 2;
  }

  /**
   * Get fallback color for terrain type
   */
  private getTerrainColor(terrainType: TerrainType): THREE.Color {
    const colors = {
      ocean: new THREE.Color(0x1e40af),
      grassland: new THREE.Color(0x22c55e),
      plains: new THREE.Color(0x84cc16),
      desert: new THREE.Color(0xf59e0b),
      tundra: new THREE.Color(0x6b7280),
      snow: new THREE.Color(0xf8fafc),
      forest: new THREE.Color(0x16a34a),
      jungle: new THREE.Color(0x15803d),
      hills: new THREE.Color(0xa3a3a3),
      mountain: new THREE.Color(0x525252),
    };

    return colors[terrainType] ?? new THREE.Color(0x6b7280);
  }

  /**
   * Update cached materials with new uniforms
   */
  updateUniforms(time: number): void {
    this.cache.forEach(entry => {
      if (entry.isShader && entry.material instanceof THREE.ShaderMaterial) {
        const { uniforms } = entry.material;
        if (uniforms?.u_time) {
          uniforms.u_time.value = time;
          entry.material.uniformsNeedUpdate = true;
        }
      }
    });
  }

  /**
   * Clear expired materials from cache
   */
  cleanup(): void {
    const expired = Array.from(this.cache.entries()).filter(
      ([, entry]) => !this.isValidMaterial(entry)
    );

    expired.forEach(([key, entry]) => {
      entry.material.dispose();
      this.cache.delete(key);
    });

    if (expired.length > 0) {
      console.warn(
        `🧹 MaterialService: Cleaned ${expired.length} expired materials`
      );
    }
  }

  /**
   * Get service statistics
   */
  getStats(): MaterialStats & { cacheSize: number } {
    return {
      ...this.stats,
      cacheSize: this.cache.size,
    };
  }

  /**
   * Clear all cached materials
   */
  clearCache(): void {
    this.cache.forEach(entry => entry.material.dispose());
    this.cache.clear();
    this.textureCache.clear();

    this.stats = {
      cached: 0,
      compiled: 0,
      textured: 0,
      fallback: 0,
    };
  }
}

// Singleton instance following existing patterns
export const materialService = new MaterialService();
