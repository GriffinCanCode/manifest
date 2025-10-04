/**
 * Tile Rendering Diagnostic System
 * Runs comprehensive tests on app startup to identify rendering issues
 */

interface DiagnosticResult {
  name: string;
  status: 'pass' | 'fail' | 'warning';
  message: string;
  data?: any;
}

class TileRenderDiagnostics {
  private results: DiagnosticResult[] = [];
  private isRunning = false;

  async runAllDiagnostics(): Promise<DiagnosticResult[]> {
    if (this.isRunning) return this.results;
    this.isRunning = true;
    this.results = [];

    console.group('🔍 TILE RENDER DIAGNOSTICS');
    console.log('Running comprehensive tile rendering diagnostics...');

    // Test 1: Check if we're in Tauri mode
    this.checkTauriMode();

    // Test 2: Check tile streaming hook
    await this.checkTileStreaming();

    // Test 3: Check Three.js context
    this.checkThreeJSContext();

    // Test 4: Check shader system
    this.checkShaderSystem();

    // Test 5: Check texture system
    this.checkTextureSystem();

    // Test 6: Check WebGL state
    this.checkWebGLState();

    // Test 7: Check console for errors
    this.checkConsoleErrors();

    // Summary
    this.printSummary();
    console.groupEnd();

    this.isRunning = false;
    return this.results;
  }

  private addResult(
    name: string,
    status: 'pass' | 'fail' | 'warning',
    message: string,
    data?: any
  ) {
    const result = { name, status, message, data };
    this.results.push(result);

    const emoji = status === 'pass' ? '✅' : status === 'fail' ? '❌' : '⚠️';
    console.log(`${emoji} ${name}: ${message}`);
    if (data) console.log('   Data:', data);
  }

  private checkTauriMode() {
    const hasTauri =
      typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
    const hasLegacy = typeof window !== 'undefined' && '__TAURI__' in window;

    if (hasTauri || hasLegacy) {
      this.addResult(
        'Tauri Detection',
        'pass',
        'Running in Tauri desktop mode',
        {
          hasTauriInternals: hasTauri,
          hasTauriLegacy: hasLegacy,
        }
      );
    } else {
      this.addResult(
        'Tauri Detection',
        'fail',
        'Running in browser mode - backend unavailable',
        {
          windowKeys:
            typeof window !== 'undefined'
              ? Object.keys(window).filter(k => k.includes('TAURI'))
              : [],
        }
      );
    }
  }

  private async checkTileStreaming() {
    try {
      // Check if useTileStreaming hook is accessible
      const gameCanvas = document.querySelector('.game-canvas');
      if (!gameCanvas) {
        this.addResult('Game Canvas', 'fail', 'Game canvas element not found');
        return;
      }

      this.addResult('Game Canvas', 'pass', 'Game canvas element found');

      // Check for React component state
      setTimeout(() => {
        this.checkReactComponentState();
      }, 1000);
    } catch (error) {
      this.addResult(
        'Tile Streaming',
        'fail',
        `Error checking tile streaming: ${(error as Error).message}`
      );
    }
  }

  private checkReactComponentState() {
    try {
      // Look for tile data in React DevTools or window debugging
      const debugInfo: any = {};

      // Check if there's a debug hook exposed
      if (typeof window !== 'undefined') {
        const win = window as any;
        if (win.__REACT_DEVTOOLS_GLOBAL_HOOK__) {
          debugInfo.reactDevTools = true;
        }
        if (win.__tileDebug) {
          debugInfo.tileDebugData = win.__tileDebug;
        }
      }

      this.addResult(
        'React State',
        'warning',
        'React component state check completed',
        debugInfo
      );
    } catch (error) {
      this.addResult(
        'React State',
        'fail',
        `Error checking React state: ${(error as Error).message}`
      );
    }
  }

  private checkThreeJSContext() {
    try {
      const canvas = document.querySelector('canvas');
      if (!canvas) {
        this.addResult('Three.js Canvas', 'fail', 'No canvas element found');
        return;
      }

      const gl = canvas.getContext('webgl2') || canvas.getContext('webgl');
      if (!gl) {
        this.addResult(
          'Three.js Context',
          'fail',
          'No WebGL context available'
        );
        return;
      }

      const debugInfo = {
        renderer: gl.getParameter(gl.RENDERER),
        vendor: gl.getParameter(gl.VENDOR),
        version: gl.getParameter(gl.VERSION),
        maxTextureSize: gl.getParameter(gl.MAX_TEXTURE_SIZE),
        maxVertexAttribs: gl.getParameter(gl.MAX_VERTEX_ATTRIBS),
      };

      this.addResult(
        'Three.js Context',
        'pass',
        'WebGL context available',
        debugInfo
      );

      // Check for existing meshes
      this.checkExistingMeshes(canvas);
    } catch (error) {
      this.addResult(
        'Three.js Context',
        'fail',
        `Error checking Three.js: ${(error as Error).message}`
      );
    }
  }

  private checkExistingMeshes(canvas: HTMLCanvasElement) {
    try {
      // Try to access Three.js objects if available
      const win = window as any;
      if (win.THREE) {
        this.addResult('Three.js Library', 'pass', 'Three.js library loaded');
      } else {
        this.addResult(
          'Three.js Library',
          'warning',
          'Three.js library not found on window'
        );
      }

      // Check canvas size
      const rect = canvas.getBoundingClientRect();
      this.addResult(
        'Canvas Size',
        rect.width > 0 && rect.height > 0 ? 'pass' : 'fail',
        `Canvas dimensions: ${rect.width}x${rect.height}`
      );
    } catch (error) {
      this.addResult(
        'Mesh Check',
        'fail',
        `Error checking meshes: ${(error as Error).message}`
      );
    }
  }

  private checkShaderSystem() {
    try {
      const win = window as any;

      if (win.__shaderManager) {
        const stats = win.__shaderManager.getStats();
        this.addResult(
          'Shader System',
          'pass',
          'Shader manager available',
          stats
        );
      } else {
        this.addResult(
          'Shader System',
          'warning',
          'Shader manager not found on window'
        );
      }

      // Check for shader diagnostics
      if (win.runShaderDiagnostics) {
        this.addResult(
          'Shader Diagnostics',
          'pass',
          'Shader diagnostics available'
        );
      } else {
        this.addResult(
          'Shader Diagnostics',
          'warning',
          'Shader diagnostics not available'
        );
      }
    } catch (error) {
      this.addResult(
        'Shader System',
        'fail',
        `Error checking shaders: ${(error as Error).message}`
      );
    }
  }

  private checkTextureSystem() {
    try {
      const win = window as any;

      // Check for texture debug info
      let textureInfo = 'No texture debug info available';

      if (win.__textureService) {
        textureInfo = 'Texture service available';
        this.addResult('Texture System', 'pass', textureInfo);
      } else {
        this.addResult('Texture System', 'warning', textureInfo);
      }

      // Check for procedural textures in DOM
      const textureElements = document.querySelectorAll(
        '[class*="texture"], [id*="texture"]'
      );
      this.addResult(
        'Texture Elements',
        textureElements.length > 0 ? 'pass' : 'warning',
        `Found ${textureElements.length} texture-related DOM elements`
      );
    } catch (error) {
      this.addResult(
        'Texture System',
        'fail',
        `Error checking textures: ${(error as Error).message}`
      );
    }
  }

  private checkWebGLState() {
    try {
      const canvas = document.querySelector('canvas');
      if (!canvas) return;

      const gl = canvas.getContext('webgl2') || canvas.getContext('webgl');
      if (!gl) return;

      // Check for WebGL errors
      const glError = gl.getError();
      if (glError !== gl.NO_ERROR) {
        this.addResult(
          'WebGL State',
          'fail',
          `WebGL error detected: ${glError}`
        );
      } else {
        this.addResult('WebGL State', 'pass', 'No WebGL errors detected');
      }

      // Check current program
      const program = gl.getParameter(gl.CURRENT_PROGRAM);
      this.addResult(
        'Active Shader',
        program ? 'pass' : 'warning',
        program ? 'Shader program active' : 'No active shader program'
      );
    } catch (error) {
      this.addResult(
        'WebGL State',
        'fail',
        `Error checking WebGL: ${(error as Error).message}`
      );
    }
  }

  private checkConsoleErrors() {
    try {
      // Override console.error temporarily to catch errors
      const originalError = console.error;
      let errorCount = 0;
      const recentErrors: string[] = [];

      console.error = (...args: any[]) => {
        errorCount++;
        recentErrors.push(args.join(' '));
        if (recentErrors.length > 5) recentErrors.shift();
        originalError.apply(console, args);
      };

      // Restore after a short delay
      setTimeout(() => {
        console.error = originalError;
      }, 5000);

      this.addResult(
        'Console Errors',
        errorCount === 0 ? 'pass' : 'warning',
        `${errorCount} console errors detected`,
        { recentErrors }
      );
    } catch (error) {
      this.addResult(
        'Console Check',
        'fail',
        `Error checking console: ${(error as Error).message}`
      );
    }
  }

  private printSummary() {
    const passed = this.results.filter(r => r.status === 'pass').length;
    const failed = this.results.filter(r => r.status === 'fail').length;
    const warnings = this.results.filter(r => r.status === 'warning').length;

    console.log('\n📊 DIAGNOSTIC SUMMARY:');
    console.log(`✅ Passed: ${passed}`);
    console.log(`❌ Failed: ${failed}`);
    console.log(`⚠️ Warnings: ${warnings}`);

    if (failed > 0) {
      console.log('\n🚨 CRITICAL ISSUES:');
      this.results
        .filter(r => r.status === 'fail')
        .forEach(r => {
          console.log(`❌ ${r.name}: ${r.message}`);
        });
    }

    // Specific tile rendering advice
    if (failed > 0 || warnings > 2) {
      console.log('\n💡 TILE RENDERING TROUBLESHOOTING:');
      console.log('1. Check if tiles array is populated in React component');
      console.log('2. Verify HexInstanceRenderer is receiving tiles prop');
      console.log('3. Check for shader compilation errors');
      console.log('4. Verify WebGL context is not lost');
      console.log('5. Check for React re-rendering issues');
    }
  }

  getResults(): DiagnosticResult[] {
    return this.results;
  }
}

// Create global instance
export const tileRenderDiagnostics = new TileRenderDiagnostics();

// Expose to window for manual testing
if (typeof window !== 'undefined') {
  (window as any).__tileRenderDiagnostics = tileRenderDiagnostics;
  (window as any).runTileRenderDiagnostics = () =>
    tileRenderDiagnostics.runAllDiagnostics();
}
