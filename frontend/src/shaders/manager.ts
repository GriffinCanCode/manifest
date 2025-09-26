/**
 * Shader Manager
 * Handles compilation, caching, and management of GLSL shaders
 * Integrates with existing render store and capabilities system
 */

import { useRenderStore } from '@stores/render-store';
import { ShaderChunk, ShaderMaterial } from 'three';

import type { ShaderDefinition, ShaderUniforms } from '../types/shaders';

interface ShaderEntry {
  material: ShaderMaterial;
  definition: ShaderDefinition;
  lastModified: number;
  dependents: Set<string>;
}

interface ShaderCompileOptions {
  defines?: Record<string, string | number>;
  vertexShader?: string;
  fragmentShader?: string;
  transparent?: boolean;
  depthTest?: boolean;
  depthWrite?: boolean;
}

export class ShaderManager {
  private cache = new Map<string, ShaderEntry>();
  private hot = new Map<string, () => void>();

  /**
   * Compile and cache a shader with dependencies
   */
  compile(
    name: string,
    definition: ShaderDefinition,
    options: ShaderCompileOptions = {}
  ): ShaderMaterial {
    const cacheKey = this.getCacheKey(name, options);
    const existing = this.cache.get(cacheKey);

    if (existing && !this.needsRecompile(existing)) {
      return existing.material;
    }

    const material = this.createMaterial(definition, options);
    const entry: ShaderEntry = {
      material,
      definition,
      lastModified: Date.now(),
      dependents: new Set(),
    };

    this.cache.set(cacheKey, entry);
    this.setupHotReload(name, cacheKey);

    return material;
  }

  /**
   * Create optimized shader material based on device capabilities
   */
  private createMaterial(
    definition: ShaderDefinition,
    options: ShaderCompileOptions
  ): ShaderMaterial {
    const { capabilities } = useRenderStore.getState();
    const { settings } = useRenderStore.getState();

    // Enhanced shader defines based on capabilities
    const defines: Record<string, string | number> = {
      // Custom defines first (can be overridden by built-in defines)
      ...definition.defines,
      ...options.defines,

      // Device capabilities (these override custom defines if there are conflicts)
      USE_WEBGL2: capabilities?.supportsWebGL2 ? 1 : 0,
      // Note: USE_INSTANCING should be handled by Three.js, not manually defined
      // USE_INSTANCING: capabilities?.supportsInstancing ? 1 : 0,

      // Quality settings
      QUALITY_LEVEL: this.getQualityLevel(),
      USE_SHADOWS: settings?.shadows ? 1 : 0,
      USE_FOG: !useRenderStore.getState().debug.disableFog ? 1 : 0,
      USE_HDR: 1, // Enable HDR processing

      // Precision settings
      PRECISION: this.getPrecisionLevel(),
    };

    // Process shader source with includes
    const vertexShader = this.processIncludes(
      options.vertexShader ?? definition.vertexShader
    );
    const fragmentShader = this.processIncludes(
      options.fragmentShader ?? definition.fragmentShader
    );

    const material = new ShaderMaterial({
      name: definition.name,
      vertexShader,
      fragmentShader,
      uniforms: { ...definition.uniforms },
      defines,
      transparent: options.transparent ?? false,
      depthTest: options.depthTest ?? true,
      depthWrite: options.depthWrite ?? true,
    });

    // Enhanced error handling
    material.onBeforeCompile = (shader, _renderer) => {
      this.validateShader(shader, definition.name);
    };

    return material;
  }

  /**
   * Process #include directives (fallback for vite-plugin-glsl)
   */
  private processIncludes(source: string): string {
    // This is a fallback - vite-plugin-glsl should handle most includes
    return source.replace(/#include\s+<([^>]+)>/g, (_match, includeName) => {
      if (ShaderChunk[includeName as keyof typeof ShaderChunk]) {
        return ShaderChunk[includeName as keyof typeof ShaderChunk];
      }
      console.warn(`Shader include not found: ${includeName}`);
      return '';
    });
  }

  /**
   * Get quality level based on render store
   */
  private getQualityLevel(): number {
    const { quality } = useRenderStore.getState();
    const qualityMap = { low: 1, medium: 2, high: 3, ultra: 4 };
    return qualityMap[quality.level] || 3;
  }

  /**
   * Get precision level based on settings
   */
  private getPrecisionLevel(): string {
    const { settings } = useRenderStore.getState();
    return settings?.precision ?? 'mediump';
  }

  /**
   * Update shader uniforms efficiently
   */
  updateUniforms(name: string, uniforms: Partial<ShaderUniforms>): void {
    const entry = this.cache.get(name);
    if (!entry) {
      console.warn(`Shader not found: ${name}`);
      return;
    }

    // Batch uniform updates
    Object.entries(uniforms).forEach(([key, uniform]) => {
      if (entry.material.uniforms[key] && uniform?.value !== undefined) {
        // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
        entry.material.uniforms[key].value = uniform.value;
      }
    });

    entry.material.uniformsNeedUpdate = true;
  }

  /**
   * Hot reload support for development
   */
  private setupHotReload(name: string, cacheKey: string): void {
    if (import.meta.env?.MODE !== 'development') return;

    const reload = () => {
      const entry = this.cache.get(cacheKey);
      if (entry) {
        entry.lastModified = Date.now();
        // Trigger recompilation on next access
        entry.material.needsUpdate = true;

        // Notify dependents
        entry.dependents.forEach(dependent => {
          this.hot.get(dependent)?.();
        });
      }
    };

    this.hot.set(name, reload);
  }

  /**
   * Check if shader needs recompilation
   */
  private needsRecompile(entry: ShaderEntry): boolean {
    // In development, always check for updates
    if (import.meta.env?.MODE === 'development') {
      return entry.material.needsUpdate;
    }
    return false;
  }

  /**
   * Generate cache key for shader variants
   */
  private getCacheKey(name: string, options: ShaderCompileOptions): string {
    const optionsHash = JSON.stringify(options);
    return `${name}_${btoa(optionsHash).slice(0, 8)}`;
  }

  /**
   * Validate shader compilation
   */
  private validateShader(
    _shader: { vertexShader: string; fragmentShader: string },
    _name: string
  ): void {
    // Enhanced shader validation will be added here
    // This is where we can add debugging and performance analysis
  }

  /**
   * Clean up resources
   */
  dispose(): void {
    this.cache.forEach(entry => {
      entry.material.dispose();
    });
    this.cache.clear();
    this.hot.clear();
  }

  /**
   * Get shader statistics for debugging
   */
  getStats() {
    return {
      cacheSize: this.cache.size,
      hotReloadCount: this.hot.size,
      materials: Array.from(this.cache.keys()),
    };
  }
}

// Singleton instance
export const shaderManager = new ShaderManager();

// Development helpers
if (import.meta.env?.MODE === 'development') {
  (window as Window & { __shaderManager?: ShaderManager }).__shaderManager =
    shaderManager;
}
