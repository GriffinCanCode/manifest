/**
 * Shader Manager
 * Handles compilation, caching, and management of GLSL shaders
 * Integrates with existing render store and capabilities system
 */

import { useRenderStore } from '@stores/render-store';
import { ShaderChunk, ShaderMaterial } from 'three';

import type { ShaderDefinition, ShaderUniforms } from '../types/shaders';

// Import custom shader modules
import commonModule from './modules/common.glsl';
import hexModule from './modules/hex.glsl';
import noiseModule from './modules/noise.glsl';
import shadowsModule from './modules/shadows.glsl';

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

  // Custom shader modules mapping
  private customModules = new Map<string, string>([
    ['common', commonModule],
    ['hex', hexModule],
    ['noise', noiseModule],
    ['shadows', shadowsModule],
  ]);

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

      // Three.js context indicator - prevents manual attribute/uniform declarations
      USE_THREEJS_BUILTIN: 1,

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

      // GLSL ES version compatibility
      GL_ES: capabilities?.supportsWebGL2 ? 0 : 1, // Enable compatibility mode for WebGL 1.0
    };

    // Process shader source with includes
    let vertexShader = this.processIncludes(
      options.vertexShader ?? definition.vertexShader,
      new Set()
    );
    let fragmentShader = this.processIncludes(
      options.fragmentShader ?? definition.fragmentShader,
      new Set()
    );

    // Debug logging for shader processing
    if (import.meta.env?.MODE === 'development') {
      console.log(`🔧 Processing shader: ${definition.name}`);
      console.log(`  Vertex shader lines: ${vertexShader.split('\n').length}`);
      console.log(
        `  Fragment shader lines: ${fragmentShader.split('\n').length}`
      );
    }

    // Add GLSL version declaration if not present for better compatibility
    if (!vertexShader.includes('#version')) {
      if (capabilities?.supportsWebGL2) {
        vertexShader = `#version 300 es\n${vertexShader}`;
      } else {
        vertexShader = `#version 100\n${vertexShader}`;
      }
    }
    if (!fragmentShader.includes('#version')) {
      if (capabilities?.supportsWebGL2) {
        fragmentShader = `#version 300 es\n${fragmentShader}`;
      } else {
        fragmentShader = `#version 100\n${fragmentShader}`;
      }
    }

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
   * Resolve a single include by name
   */
  private resolveInclude(
    includeName: string,
    processedIncludes: Set<string>
  ): string {
    // Skip if already processed (prevent duplicates)
    if (processedIncludes.has(includeName)) {
      if (import.meta.env?.MODE === 'development') {
        console.log(`🔄 Skipping already processed include: ${includeName}`);
      }
      return '';
    }

    // Check custom modules first
    if (this.customModules.has(includeName)) {
      const moduleContent = this.customModules.get(includeName);
      if (moduleContent) {
        // Mark this module as processed
        processedIncludes.add(includeName);
        if (import.meta.env?.MODE === 'development') {
          console.log(`📦 Including custom module: ${includeName}`);
        }
        // Recursively process includes in the module itself
        return this.processIncludes(moduleContent, processedIncludes);
      }
    }

    // Then check Three.js built-in includes
    if (ShaderChunk[includeName as keyof typeof ShaderChunk]) {
      processedIncludes.add(includeName);
      if (import.meta.env?.MODE === 'development') {
        console.log(`📦 Including Three.js chunk: ${includeName}`);
      }
      return ShaderChunk[includeName as keyof typeof ShaderChunk];
    }

    console.warn(`❌ Shader include not found: ${includeName}`);
    return '';
  }

  /**
   * Process #include directives (fallback for vite-plugin-glsl)
   */
  private processIncludes(
    source: string,
    processedIncludes: Set<string> = new Set()
  ): string {
    // This is a fallback - vite-plugin-glsl should handle most includes
    let processedSource = source;

    // Remove glslify pragmas to avoid compilation warnings
    processedSource = processedSource.replace(/#pragma glslify:.*$/gm, '');

    // Remove standalone include guards from processed source since we handle includes manually
    processedSource = processedSource.replace(/#ifndef\s+\w+_GLSL\s*\n/g, '');
    processedSource = processedSource.replace(/#define\s+\w+_GLSL\s*\n/g, '');
    processedSource = processedSource.replace(
      /#endif\s*\/\/\s*\w+_GLSL\s*\n/g,
      ''
    );

    // Process both angle bracket and relative path includes
    // Handle angle bracket includes: #include <modulename>
    processedSource = processedSource.replace(
      /#include\s+<([^>]+)>/g,
      (_match, includeName: string) =>
        this.resolveInclude(includeName, processedIncludes)
    );

    // Handle relative path includes: #include ./modulename.glsl
    processedSource = processedSource.replace(
      /#include\s+\.\/([^.\s]+)\.glsl/g,
      (_match, moduleName: string) =>
        this.resolveInclude(moduleName, processedIncludes)
    );

    // For shader materials that need Three.js lighting, inject required structs
    // Look for GeometricContext usage patterns but avoid duplicates
    if (
      (processedSource.includes('GeometricContext geometry') ||
        processedSource.includes('GeometricContext ')) &&
      !processedSource.includes('struct GeometricContext')
    ) {
      // Inject struct definitions at the beginning after precision declaration
      const structDefinitions = `
// Three.js GeometricContext struct for lighting calculations
struct GeometricContext {
  vec3 position;
  vec3 normal;
  vec3 viewDir;
};

// Three.js LambertMaterial struct for compatibility  
struct LambertMaterial {
  vec3 diffuseColor;
  float specularStrength;
};
`;

      // Insert after precision declaration
      const precisionMatch = processedSource.match(/precision\s+\w+\s+float;/);
      if (precisionMatch) {
        const insertIndex =
          processedSource.indexOf(precisionMatch[0]) + precisionMatch[0].length;
        processedSource = `${processedSource.slice(0, insertIndex)}\n${structDefinitions}${processedSource.slice(insertIndex)}`;
      } else {
        // Fallback: insert at the very beginning
        processedSource = `${structDefinitions}\n${processedSource}`;
      }
    }

    return processedSource;
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
    // Force recompilation if shader was compiled before our fix
    const thirtySecondsAgo = Date.now() - 30 * 1000;
    if (entry.lastModified < thirtySecondsAgo) {
      return true;
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
   * Validate shader compilation and program linking
   */
  private validateShader(
    shader: { vertexShader: string; fragmentShader: string },
    name: string
  ): void {
    try {
      // Get WebGL context for validation
      const canvas = document.querySelector('canvas');
      if (!canvas) return;

      const gl = canvas.getContext('webgl2') ?? canvas.getContext('webgl');
      if (!gl) return;

      // Test compile the shaders for validation
      const vertexShader = this.compileShaderSource(
        gl,
        gl.VERTEX_SHADER,
        shader.vertexShader
      );
      const fragmentShader = this.compileShaderSource(
        gl,
        gl.FRAGMENT_SHADER,
        shader.fragmentShader
      );

      if (!vertexShader || !fragmentShader) {
        console.error(
          `❌ ${name}: Shader compilation failed during validation`
        );
        return;
      }

      // Test program linking and validation
      const program = gl.createProgram();
      if (!program) return;

      gl.attachShader(program, vertexShader);
      gl.attachShader(program, fragmentShader);
      gl.linkProgram(program);

      if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
        const error = gl.getProgramInfoLog(program);
        console.error(`❌ ${name}: Program linking failed - ${error}`);
        this.logShaderDebugInfo(shader, name);
        return;
      }

      // Validate the program
      gl.validateProgram(program);
      if (!gl.getProgramParameter(program, gl.VALIDATE_STATUS)) {
        const error = gl.getProgramInfoLog(program);
        console.warn(`⚠️ ${name}: Program validation warning - ${error}`);
        // Don't return here - validation warnings don't always mean the program won't work
      }

      // Check for attribute/uniform mismatches
      this.validateAttributes(gl, program, name);
      this.validateUniforms(gl, program, name);

      // Cleanup test resources
      gl.deleteProgram(program);
      gl.deleteShader(vertexShader);
      gl.deleteShader(fragmentShader);
    } catch (error) {
      console.error(`❌ ${name}: Shader validation error - ${String(error)}`);
    }
  }

  /**
   * Compile a shader source for validation
   */
  private compileShaderSource(
    gl: WebGLRenderingContext,
    type: number,
    source: string
  ): WebGLShader | null {
    const shader = gl.createShader(type);
    if (!shader) return null;

    gl.shaderSource(shader, source);
    gl.compileShader(shader);

    if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
      const error = gl.getShaderInfoLog(shader);
      const shaderType = type === gl.VERTEX_SHADER ? 'vertex' : 'fragment';
      console.error(
        `❌ ${shaderType} shader compilation failed: ${String(error)}`
      );
      gl.deleteShader(shader);
      return null;
    }

    return shader;
  }

  /**
   * Validate shader attributes
   */
  private validateAttributes(
    gl: WebGLRenderingContext,
    program: WebGLProgram,
    name: string
  ): void {
    const attributeCount = gl.getProgramParameter(
      program,
      gl.ACTIVE_ATTRIBUTES
    ) as number;
    const requiredAttributes = [
      'position',
      'instancePosition',
      'instanceColor',
      'instanceHeight',
    ];
    const foundAttributes: string[] = [];

    for (let i = 0; i < attributeCount; i++) {
      const attributeInfo = gl.getActiveAttrib(program, i);
      if (attributeInfo?.name) {
        foundAttributes.push(attributeInfo.name);
      }
    }

    // Check for hex terrain specific attributes
    if (name.includes('hex-terrain')) {
      const missingRequired = requiredAttributes.filter(
        attr => !foundAttributes.some(found => found.includes(attr))
      );

      if (missingRequired.length > 0) {
        console.warn(
          `⚠️ ${name}: Missing expected attributes: ${missingRequired.join(', ')}`
        );
      }
    }
  }

  /**
   * Validate shader uniforms
   */
  private validateUniforms(
    gl: WebGLRenderingContext,
    program: WebGLProgram,
    name: string
  ): void {
    const uniformCount = gl.getProgramParameter(
      program,
      gl.ACTIVE_UNIFORMS
    ) as number;
    const foundUniforms: string[] = [];

    for (let i = 0; i < uniformCount; i++) {
      const uniformInfo = gl.getActiveUniform(program, i);
      if (uniformInfo?.name) {
        foundUniforms.push(uniformInfo.name);
      }
    }

    // Check for critical uniforms
    const criticalUniforms = ['u_time', 'u_lightDirection', 'u_exposure'];
    const missingCritical = criticalUniforms.filter(
      uniform => !foundUniforms.some(found => found.includes(uniform))
    );

    if (missingCritical.length > 0) {
      console.warn(
        `⚠️ ${name}: Missing critical uniforms: ${missingCritical.join(', ')}`
      );
    }
  }

  /**
   * Log shader debug information
   */
  private logShaderDebugInfo(
    shader: { vertexShader: string; fragmentShader: string },
    name: string
  ): void {
    console.warn(`🔍 ${name} Shader Debug Info:`);
    console.warn(
      'Vertex Shader Lines:',
      shader.vertexShader.split('\n').length
    );
    console.warn(
      'Fragment Shader Lines:',
      shader.fragmentShader.split('\n').length
    );
    console.warn(
      'Vertex Shader Preview:',
      shader.vertexShader.split('\n').slice(0, 10).join('\n')
    );
    console.warn(
      'Fragment Shader Preview:',
      shader.fragmentShader.split('\n').slice(0, 10).join('\n')
    );
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
   * Get a cached shader material
   */
  get(
    name: string,
    options: ShaderCompileOptions = {}
  ): ShaderMaterial | undefined {
    const cacheKey = this.getCacheKey(name, options);
    return this.cache.get(cacheKey)?.material;
  }

  /**
   * Check if a shader exists in cache
   */
  has(name: string, options: ShaderCompileOptions = {}): boolean {
    const cacheKey = this.getCacheKey(name, options);
    return this.cache.has(cacheKey);
  }

  /**
   * Clear shader cache
   */
  clear(): void {
    this.cache.forEach(entry => {
      entry.material.dispose();
    });
    this.cache.clear();
    this.hot.clear();
  }

  /**
   * Force recompilation of all cached shaders
   */
  forceRecompileAll(): void {
    console.warn(
      '🔄 ShaderManager: Forcing recompilation of all cached shaders'
    );
    this.cache.forEach(entry => {
      entry.material.needsUpdate = true;
      entry.lastModified = 0; // Force recompilation
    });
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
  const windowWithShaderManager = window as Window & {
    __shaderManager?: ShaderManager;
    __forceShaderRecompile?: () => void;
  };
  windowWithShaderManager.__shaderManager = shaderManager;
  windowWithShaderManager.__forceShaderRecompile = () => {
    shaderManager.forceRecompileAll();
    console.warn(
      '✅ Shader recompilation forced. Refresh the page to see changes.'
    );
  };
}
