/**
 * Device capability detection for optimal WebGL2/WebGPU initialization
 * Provides intelligent fallback and feature detection for rendering pipeline
 */

export interface DeviceCapabilities {
  readonly supportsWebGPU: boolean;
  readonly supportsWebGL2: boolean;
  readonly supportsWebGL: boolean;
  readonly preferredBackend: 'webgpu' | 'webgl2' | 'webgl' | 'none';
  readonly gpuTier: 'high' | 'medium' | 'low';
  readonly maxTextureSize: number;
  readonly maxAnisotropy: number;
  readonly supportsInstancing: boolean;
  readonly supportsFloatTextures: boolean;
  readonly supportsHDR: boolean;
  readonly supportsShadows: boolean;
  readonly devicePixelRatio: number;
  readonly memoryInfo?: {
    totalJSHeapSize?: number;
    usedJSHeapSize?: number;
    jsHeapSizeLimit?: number;
  };
}

export interface RenderingSettings {
  backend: 'webgpu' | 'webgl2' | 'webgl';
  pixelRatio: number;
  antialias: boolean;
  shadows: boolean;
  powerPreference: 'high-performance' | 'low-power' | 'default';
  precision: 'highp' | 'mediump' | 'lowp';
  logarithmicDepthBuffer: boolean;
  alpha: boolean;
  premultipliedAlpha: boolean;
  preserveDrawingBuffer: boolean;
}

/**
 * Comprehensive device capability detection
 * Uses feature detection and benchmarking to determine optimal settings
 */
class CapabilitiesDetector {
  private capabilities: DeviceCapabilities | null = null;

  async detect(): Promise<DeviceCapabilities> {
    if (this.capabilities) return this.capabilities;

    const canvas = document.createElement('canvas');
    canvas.width = 1;
    canvas.height = 1;

    // Test WebGPU support
    const supportsWebGPU = await this.testWebGPU();

    // Test WebGL2 support
    const webgl2Context = canvas.getContext('webgl2', {
      antialias: false,
      failIfMajorPerformanceCaveat: false,
    });
    const supportsWebGL2 = webgl2Context !== null;

    // Test WebGL support
    const webglContext =
      canvas.getContext('webgl', {
        antialias: false,
        failIfMajorPerformanceCaveat: false,
      }) ?? (canvas.getContext('experimental-webgl') as WebGLRenderingContext);
    const supportsWebGL = webglContext !== null;

    // Determine preferred backend
    const preferredBackend = this.determinePreferredBackend(
      supportsWebGPU,
      supportsWebGL2,
      supportsWebGL
    );

    // Get active context for feature testing
    const gl = webgl2Context ?? webglContext;

    const capabilities: DeviceCapabilities = {
      supportsWebGPU,
      supportsWebGL2,
      supportsWebGL,
      preferredBackend,
      gpuTier: this.detectGPUTier(gl),
      maxTextureSize: this.getMaxTextureSize(gl),
      maxAnisotropy: this.getMaxAnisotropy(gl),
      supportsInstancing: this.testInstancing(gl),
      supportsFloatTextures: this.testFloatTextures(gl),
      supportsHDR: this.testHDRSupport(gl),
      supportsShadows: this.testShadowSupport(gl),
      devicePixelRatio: Math.min(window.devicePixelRatio ?? 1, 2),
      memoryInfo: this.getMemoryInfo(),
    };

    canvas.remove();
    this.capabilities = capabilities;
    return capabilities;
  }

  private async testWebGPU(): Promise<boolean> {
    // Check if WebGPU is supported
    if (!('gpu' in navigator)) return false;

    try {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any, @typescript-eslint/no-unsafe-assignment
      const { gpu } = navigator as any;
      // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment, @typescript-eslint/no-unsafe-call, @typescript-eslint/no-unsafe-member-access
      const adapter = await gpu.requestAdapter({
        powerPreference: 'high-performance',
      });
      return adapter !== null;
    } catch {
      return false;
    }
  }

  private determinePreferredBackend(
    webgpu: boolean,
    webgl2: boolean,
    webgl: boolean
  ): DeviceCapabilities['preferredBackend'] {
    // WebGPU is preferred when available (future-ready)
    if (webgpu) return 'webgpu';
    // WebGL2 is preferred over WebGL1
    if (webgl2) return 'webgl2';
    // WebGL1 fallback
    if (webgl) return 'webgl';
    return 'none';
  }

  private detectGPUTier(
    gl: WebGL2RenderingContext | WebGLRenderingContext | null
  ): DeviceCapabilities['gpuTier'] {
    if (!gl) return 'low';

    const debugInfo = gl.getExtension('WEBGL_debug_renderer_info');
    if (!debugInfo) return 'medium';

    const renderer = String(
      gl.getParameter(debugInfo.UNMASKED_RENDERER_WEBGL)
    ).toLowerCase();

    // High-end GPUs
    if (
      renderer.includes('nvidia') &&
      (renderer.includes('rtx') ||
        renderer.includes('gtx 16') ||
        renderer.includes('gtx 20') ||
        renderer.includes('gtx 30') ||
        renderer.includes('gtx 40'))
    )
      return 'high';

    if (
      renderer.includes('radeon') &&
      (renderer.includes('rx 6') ||
        renderer.includes('rx 7') ||
        renderer.includes('rx 5700') ||
        renderer.includes('rx 6800') ||
        renderer.includes('rx 6900'))
    )
      return 'high';

    if (
      renderer.includes('apple') &&
      (renderer.includes('m1') ||
        renderer.includes('m2') ||
        renderer.includes('m3'))
    )
      return 'high';

    // Medium GPUs
    if (renderer.includes('gtx 10') || renderer.includes('gtx 9'))
      return 'medium';
    if (renderer.includes('rx 5') || renderer.includes('rx 4')) return 'medium';
    if (renderer.includes('intel iris')) return 'medium';

    // Low-end fallback
    return 'low';
  }

  private getMaxTextureSize(
    gl: WebGL2RenderingContext | WebGLRenderingContext | null
  ): number {
    if (!gl) return 512;
    return gl.getParameter(gl.MAX_TEXTURE_SIZE) as number;
  }

  private getMaxAnisotropy(
    gl: WebGL2RenderingContext | WebGLRenderingContext | null
  ): number {
    if (!gl) return 1;

    const ext = gl.getExtension('EXT_texture_filter_anisotropic');
    if (!ext) return 1;

    return gl.getParameter(ext.MAX_TEXTURE_MAX_ANISOTROPY_EXT) as number;
  }

  private testInstancing(
    gl: WebGL2RenderingContext | WebGLRenderingContext | null
  ): boolean {
    if (!gl) return false;

    // WebGL2 has built-in instancing
    if ('drawArraysInstanced' in gl) return true;

    // WebGL1 extension check
    return gl.getExtension('ANGLE_instanced_arrays') !== null;
  }

  private testFloatTextures(
    gl: WebGL2RenderingContext | WebGLRenderingContext | null
  ): boolean {
    if (!gl) return false;

    // WebGL2 has built-in float texture support
    if ('texImage2D' in gl && gl instanceof WebGL2RenderingContext) return true;

    // WebGL1 extension check
    const floatExt = gl.getExtension('OES_texture_float');
    return floatExt !== null;
  }

  private testHDRSupport(
    gl: WebGL2RenderingContext | WebGLRenderingContext | null
  ): boolean {
    if (!gl) return false;

    // Test for half-float support
    const halfFloatExt =
      gl.getExtension('OES_texture_half_float') ??
      gl.getExtension('EXT_color_buffer_half_float');
    return halfFloatExt !== null;
  }

  private testShadowSupport(
    gl: WebGL2RenderingContext | WebGLRenderingContext | null
  ): boolean {
    if (!gl) return false;

    // Test for depth texture support
    // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
    const depthExt =
      gl.getExtension('WEBGL_depth_texture') ??
      gl.getExtension('OES_depth_texture');
    return depthExt !== null || 'DEPTH_COMPONENT24' in gl;
  }

  private getMemoryInfo() {
    // Check for performance.memory (non-standard but widely supported)
    if (!('memory' in performance)) return undefined;

    // eslint-disable-next-line @typescript-eslint/no-explicit-any, @typescript-eslint/no-unsafe-assignment, @typescript-eslint/no-unsafe-member-access
    const perfMemory = (performance as any).memory;
    if (!perfMemory) return undefined;

    return {
      // eslint-disable-next-line @typescript-eslint/no-unsafe-member-access
      totalJSHeapSize: perfMemory.totalJSHeapSize as number,
      // eslint-disable-next-line @typescript-eslint/no-unsafe-member-access
      usedJSHeapSize: perfMemory.usedJSHeapSize as number,
      // eslint-disable-next-line @typescript-eslint/no-unsafe-member-access
      jsHeapSizeLimit: perfMemory.jsHeapSizeLimit as number,
    };
  }

  /**
   * Generate optimal rendering settings based on capabilities
   */
  generateOptimalSettings(capabilities: DeviceCapabilities): RenderingSettings {
    const isHighEnd = capabilities.gpuTier === 'high';
    const isMediumEnd = capabilities.gpuTier === 'medium';

    return {
      backend:
        capabilities.preferredBackend === 'none'
          ? 'webgl'
          : capabilities.preferredBackend,
      pixelRatio: capabilities.devicePixelRatio,
      antialias: isHighEnd || isMediumEnd,
      shadows: capabilities.supportsShadows && isHighEnd,
      powerPreference: isHighEnd ? 'high-performance' : 'default',
      precision: isHighEnd ? 'highp' : isMediumEnd ? 'mediump' : 'lowp',
      logarithmicDepthBuffer: false, // Enable only if depth fighting issues
      alpha: false, // Opaque canvas for better performance
      premultipliedAlpha: true,
      preserveDrawingBuffer: false, // Better performance
    };
  }
}

// Singleton instance
export const capabilitiesDetector = new CapabilitiesDetector();

/**
 * Utility function for easy capability detection
 */
export const detectCapabilities = async (): Promise<DeviceCapabilities> => {
  return await capabilitiesDetector.detect();
};

/**
 * Generate optimal settings for the detected device
 */
export const getOptimalRenderingSettings =
  async (): Promise<RenderingSettings> => {
    const capabilities = await detectCapabilities();
    return capabilitiesDetector.generateOptimalSettings(capabilities);
  };
