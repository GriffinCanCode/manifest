/**
 * Comprehensive testing utilities for render passes
 * Provides validation, performance monitoring, and debugging tools
 */

import {
  type Camera,
  PerspectiveCamera,
  Scene,
  WebGLRenderer,
  WebGLRenderTarget,
} from 'three';

import type { RenderPass } from '../core/RenderPass';

export interface PassTestResult {
  passName: string;
  success: boolean;
  errors: string[];
  warnings: string[];
  metrics: {
    initTime: number;
    renderTime: number;
    memoryUsage: number;
  };
}

interface PassTestConfig {
  width: number;
  height: number;
  iterations: number;
  enableProfiling: boolean;
  validateOutput: boolean;
}

/**
 * Comprehensive render pass testing utility
 */
export class PassTester {
  private renderer!: WebGLRenderer;
  private scene!: Scene;
  private camera!: Camera;
  private renderTarget!: WebGLRenderTarget;

  constructor() {
    this.setupTestEnvironment();
  }

  private setupTestEnvironment(): void {
    // Create minimal WebGL context for testing
    const canvas = document.createElement('canvas');
    canvas.width = 512;
    canvas.height = 512;

    this.renderer = new WebGLRenderer({
      canvas,
      antialias: false,
      alpha: false,
      depth: true,
      stencil: false,
      premultipliedAlpha: false,
      preserveDrawingBuffer: true,
    });

    this.scene = new Scene();
    this.camera = new PerspectiveCamera(75, 1, 0.1, 1000);
    this.camera.position.set(0, 0, 5);

    this.renderTarget = new WebGLRenderTarget(512, 512, {
      stencilBuffer: false,
      depthBuffer: true,
    });
  }

  /**
   * Test a single render pass
   */
  testPass(
    pass: RenderPass,
    config: Partial<PassTestConfig> = {}
  ): PassTestResult {
    const finalConfig: PassTestConfig = {
      width: 512,
      height: 512,
      iterations: 10,
      enableProfiling: true,
      validateOutput: true,
      ...config,
    };

    const result: PassTestResult = {
      passName: pass.name,
      success: true,
      errors: [],
      warnings: [],
      metrics: {
        initTime: 0,
        renderTime: 0,
        memoryUsage: 0,
      },
    };

    try {
      // Test initialization
      const initStart = performance.now();

      try {
        if (pass.initialize) {
          pass.initialize(this.renderer);
        }
        result.metrics.initTime = performance.now() - initStart;
      } catch (error) {
        result.errors.push(`Initialization failed: ${String(error)}`);
        result.success = false;
      }

      // Test resize handling
      try {
        if (pass.resize) {
          pass.resize(finalConfig.width, finalConfig.height);
        }
      } catch (error) {
        result.errors.push(`Resize failed: ${String(error)}`);
        result.success = false;
      }

      // Test rendering
      if (result.success) {
        const renderTimes: number[] = [];

        for (let i = 0; i < finalConfig.iterations; i++) {
          const renderStart = performance.now();

          try {
            pass.render(
              this.renderer,
              this.scene,
              this.camera,
              this.renderTarget,
              this.renderTarget
            );

            renderTimes.push(performance.now() - renderStart);
          } catch (error) {
            result.errors.push(
              `Render iteration ${i} failed: ${String(error)}`
            );
            result.success = false;
            break;
          }
        }

        // Calculate average render time
        if (renderTimes.length > 0) {
          result.metrics.renderTime =
            renderTimes.reduce((a, b) => a + b, 0) / renderTimes.length;
        }
      }

      // Memory usage estimation
      result.metrics.memoryUsage = this.estimateMemoryUsage();

      // Validate pass configuration
      this.validatePassConfiguration(pass, result);

      // Output validation
      if (finalConfig.validateOutput && result.success) {
        this.validateRenderOutput(pass, result);
      }
    } catch (error) {
      result.errors.push(`Unexpected error: ${String(error)}`);
      result.success = false;
    }

    return result;
  }

  /**
   * Test multiple passes for compatibility
   */
  testPassSequence(
    passes: RenderPass[],
    config: Partial<PassTestConfig> = {}
  ): {
    results: PassTestResult[];
    sequenceValid: boolean;
    overallMetrics: {
      totalTime: number;
      totalMemory: number;
    };
  } {
    const results: PassTestResult[] = [];
    let sequenceValid = true;
    const overallStart = performance.now();

    // Initialize all passes
    for (const pass of passes) {
      try {
        if (pass.initialize) {
          pass.initialize(this.renderer);
        }
        if (pass.resize) {
          pass.resize(config.width ?? 512, config.height ?? 512);
        }
      } catch (_error) {
        sequenceValid = false;
      }
    }

    // Test each pass individually
    for (const pass of passes) {
      const result = this.testPass(pass, config);
      results.push(result);

      if (!result.success) {
        sequenceValid = false;
      }
    }

    // Test passes as a sequence
    if (sequenceValid) {
      try {
        let readBuffer = this.renderTarget;
        let writeBuffer = new WebGLRenderTarget(512, 512);

        for (let i = 0; i < passes.length; i++) {
          const pass = passes[i];
          const isLastPass = i === passes.length - 1;

          pass.render(
            this.renderer,
            this.scene,
            this.camera,
            isLastPass ? undefined : writeBuffer,
            readBuffer
          );

          // Swap buffers
          [readBuffer, writeBuffer] = [writeBuffer, readBuffer];
        }

        writeBuffer.dispose();
      } catch (error) {
        sequenceValid = false;
        const errorMessage = `Sequence test failed: ${String(error)}`;
        results.forEach(r => {
          r.warnings.push(errorMessage);
        });
      }
    }

    const overallTime = performance.now() - overallStart;
    const totalMemory = results.reduce(
      (sum, r) => sum + r.metrics.memoryUsage,
      0
    );

    return {
      results,
      sequenceValid,
      overallMetrics: {
        totalTime: overallTime,
        totalMemory,
      },
    };
  }

  /**
   * Validate pass configuration
   */
  private validatePassConfiguration(
    pass: RenderPass,
    result: PassTestResult
  ): void {
    // Check required properties
    if (!pass.name || pass.name.trim().length === 0) {
      result.warnings.push('Pass name is empty or invalid');
    }

    if (pass.priority === undefined || pass.priority === null) {
      result.warnings.push('Pass priority not set');
    }

    // Check for common issues
    if (pass.enabled === undefined) {
      result.warnings.push('Pass enabled state not explicitly set');
    }

    if (pass.renderToScreen === undefined) {
      result.warnings.push('renderToScreen property not set');
    }

    // Validate methods
    const requiredMethods = ['render'] as const;
    for (const method of requiredMethods) {
      if (
        typeof (pass as unknown as Record<string, unknown>)[method] !==
        'function'
      ) {
        result.errors.push(`Required method '${method}' not implemented`);
        result.success = false;
      }
    }

    const optionalMethods = ['initialize', 'resize', 'dispose'] as const;
    for (const method of optionalMethods) {
      const methodValue = (pass as unknown as Record<string, unknown>)[method];
      if (methodValue !== undefined && typeof methodValue !== 'function') {
        result.warnings.push(`Method '${method}' exists but is not a function`);
      }
    }
  }

  /**
   * Basic render output validation
   */
  private validateRenderOutput(pass: RenderPass, result: PassTestResult): void {
    try {
      // Render to a test target
      const testTarget = new WebGLRenderTarget(64, 64);

      pass.render(
        this.renderer,
        this.scene,
        this.camera,
        testTarget,
        testTarget
      );

      // Read pixel data to validate output
      const pixels = new Uint8Array(64 * 64 * 4);
      this.renderer.readRenderTargetPixels(testTarget, 0, 0, 64, 64, pixels);

      // Check for obvious issues
      let allZero = true;
      let allMax = true;

      for (let i = 0; i < pixels.length; i += 4) {
        const r = pixels[i];
        const g = pixels[i + 1];
        const b = pixels[i + 2];
        const a = pixels[i + 3];

        if (r !== 0 || g !== 0 || b !== 0 || a !== 0) {
          allZero = false;
        }

        if (r !== 255 || g !== 255 || b !== 255 || a !== 255) {
          allMax = false;
        }
      }

      if (allZero) {
        result.warnings.push(
          'Render output is completely black - may indicate rendering issue'
        );
      }

      if (allMax) {
        result.warnings.push(
          'Render output is completely white - may indicate overflow'
        );
      }

      testTarget.dispose();
    } catch (error) {
      result.warnings.push(`Output validation failed: ${String(error)}`);
    }
  }

  /**
   * Estimate memory usage
   */
  private estimateMemoryUsage(): number {
    // Basic estimation - in a real implementation, this would be more sophisticated
    const { info } = this.renderer;
    return (info.memory?.geometries || 0) + (info.memory?.textures || 0);
  }

  /**
   * Generate a comprehensive test report
   */
  generateReport(results: PassTestResult[]): string {
    let report = '📊 Render Pass Test Report\n';
    report += `${'═'.repeat(50)}\n\n`;

    const successful = results.filter(r => r.success);
    const failed = results.filter(r => !r.success);

    report += `✅ Successful: ${successful.length}\n`;
    report += `❌ Failed: ${failed.length}\n`;
    report += `📈 Total passes tested: ${results.length}\n\n`;

    // Performance summary
    const avgRenderTime =
      results.reduce((sum, r) => sum + r.metrics.renderTime, 0) /
      results.length;
    const totalMemory = results.reduce(
      (sum, r) => sum + r.metrics.memoryUsage,
      0
    );

    report += '⚡ Performance Summary\n';
    report += `${'─'.repeat(30)}\n`;
    report += `Average render time: ${avgRenderTime.toFixed(2)}ms\n`;
    report += `Total memory usage: ${totalMemory}\n\n`;

    // Detailed results
    report += '📝 Detailed Results\n';
    report += `${'─'.repeat(30)}\n`;

    for (const result of results) {
      report += `\n🎯 ${result.passName}\n`;
      report += `   Status: ${result.success ? '✅ PASS' : '❌ FAIL'}\n`;
      report += `   Render time: ${result.metrics.renderTime.toFixed(2)}ms\n`;
      report += `   Init time: ${result.metrics.initTime.toFixed(2)}ms\n`;

      if (result.errors.length > 0) {
        report += `   ❌ Errors:\n`;
        for (const error of result.errors) {
          report += `      • ${error}\n`;
        }
      }

      if (result.warnings.length > 0) {
        report += `   ⚠️  Warnings:\n`;
        for (const warning of result.warnings) {
          report += `      • ${warning}\n`;
        }
      }
    }

    return report;
  }

  /**
   * Cleanup test resources
   */
  dispose(): void {
    if (this.renderTarget) {
      this.renderTarget.dispose();
    }

    if (this.renderer) {
      this.renderer.dispose();
    }
  }
}

/**
 * Quick test utility for development
 */
export const quickTestPass = (pass: RenderPass): void => {
  if (process.env.NODE_ENV !== 'development') {
    return;
  }

  const tester = new PassTester();
  const result = tester.testPass(pass, { iterations: 1 });

  console.warn(`🧪 Quick Test: ${pass.name}`);

  if (result.success) {
    console.warn('✅ Test passed');
  } else {
    console.error('❌ Test failed');
    result.errors.forEach(error => console.error(`  • ${error}`));
  }

  if (result.warnings.length > 0) {
    console.warn('⚠️ Warnings:');
    result.warnings.forEach(warning => console.warn(`  • ${warning}`));
  }

  console.warn(`⚡ Render time: ${result.metrics.renderTime.toFixed(2)}ms`);

  tester.dispose();
};

/**
 * Test all registered passes in the registry
 */
export const testAllRegisteredPasses = async (): Promise<void> => {
  if (process.env.NODE_ENV !== 'development') {
    console.warn('Pass testing is only available in development mode');
    return;
  }

  try {
    const { passRegistry } = await import('../passes');
    const passes = passRegistry.createOrderedPasses();

    const tester = new PassTester();
    const sequenceResult = tester.testPassSequence(passes);

    console.warn('🧪 Multi-Step Rendering System Test');
    console.warn(tester.generateReport(sequenceResult.results));

    if (sequenceResult.sequenceValid) {
      console.warn('✅ Pass sequence is valid');
    } else {
      console.error('❌ Pass sequence has issues');
    }

    tester.dispose();
  } catch (error) {
    console.error('Failed to test passes:', String(error));
  }
};
