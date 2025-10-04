/**
 * Real-time Shader Diagnostics System
 * Runs in browser to identify shader system issues
 */

import * as THREE from 'three';

import {
  getShaderDefinition,
  SHADER_DEFINITIONS,
  type ShaderName,
} from '../shaders/definitions';
import { shaderManager } from '../shaders/manager';

interface DiagnosticResult {
  component: string;
  status: 'pass' | 'fail' | 'warning' | 'info';
  message: string;
  details?: unknown;
  timestamp: number;
}

class ShaderDiagnostics {
  private results: DiagnosticResult[] = [];
  private isRunning = false;

  /**
   * Run complete shader diagnostics
   */
  async runDiagnostics(): Promise<DiagnosticResult[]> {
    if (this.isRunning) {
      this.log('info', 'System', 'Diagnostics already running');
      return this.results;
    }

    this.isRunning = true;
    this.results = [];

    console.warn('🔍 SHADER DIAGNOSTICS: Starting comprehensive analysis...');

    try {
      this.checkWebGLSupport();
      this.checkShaderDefinitions();
      this.checkShaderManager();
      this.checkHexTerrainShader();
      await this.checkShaderProvider();
      this.checkThreeJSCompatibility();

      this.printResults();
    } catch (error) {
      this.log('fail', 'System', `Diagnostics failed: ${String(error)}`, {
        error,
      });
    } finally {
      this.isRunning = false;
    }

    return this.results;
  }

  private log(
    status: DiagnosticResult['status'],
    component: string,
    message: string,
    details?: unknown
  ) {
    const result: DiagnosticResult = {
      component,
      status,
      message,
      details,
      timestamp: Date.now(),
    };

    this.results.push(result);

    const emoji = {
      pass: '✅',
      fail: '❌',
      warning: '⚠️',
      info: 'ℹ️',
    }[status];

    console.warn(`${emoji} [${component}] ${message}`, details || '');
  }

  private checkWebGLSupport(): void {
    try {
      const canvas = document.createElement('canvas');
      const gl =
        canvas.getContext('webgl2') ??
        canvas.getContext('webgl') ??
        canvas.getContext('experimental-webgl');

      if (!gl) {
        this.log('fail', 'WebGL', 'WebGL not supported in this browser');
        return;
      }

      const version = (gl as WebGLRenderingContext).getParameter(
        (gl as WebGLRenderingContext).VERSION
      );
      const renderer = (gl as WebGLRenderingContext).getParameter(
        (gl as WebGLRenderingContext).RENDERER
      );
      const vendor = (gl as WebGLRenderingContext).getParameter(
        (gl as WebGLRenderingContext).VENDOR
      );

      this.log('pass', 'WebGL', `WebGL available: ${version}`, {
        version,
        renderer,
        vendor,
        webgl2: !!canvas.getContext('webgl2'),
      });

      // Check extensions
      const instancedArrays =
        (gl as WebGLRenderingContext).getExtension('ANGLE_instanced_arrays') ??
        (gl as WebGLRenderingContext).getExtension('WEBGL_instanced_arrays');
      if (!instancedArrays && !canvas.getContext('webgl2')) {
        this.log(
          'warning',
          'WebGL',
          'Instanced arrays extension not available - may cause issues'
        );
      } else {
        this.log('pass', 'WebGL', 'Instanced rendering supported');
      }

      // Test basic shader compilation
      const testVertexShader = `
        attribute vec3 position;
        uniform mat4 projectionMatrix;
        uniform mat4 modelViewMatrix;
        void main() {
          gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
        }
      `;

      const testFragmentShader = `
        precision mediump float;
        void main() {
          gl_FragColor = vec4(1.0, 0.0, 0.0, 1.0);
        }
      `;

      if (
        !this.compileTestShader(
          gl as WebGLRenderingContext,
          testVertexShader,
          testFragmentShader
        )
      ) {
        this.log('fail', 'WebGL', 'Basic shader compilation failed');
      } else {
        this.log('pass', 'WebGL', 'Basic shader compilation works');
      }
    } catch (error) {
      this.log('fail', 'WebGL', `WebGL check failed: ${String(error)}`);
    }
  }

  private compileTestShader(
    gl: WebGLRenderingContext,
    vertexSrc: string,
    fragmentSrc: string
  ): boolean {
    try {
      // Compile vertex shader
      const vertexShader = gl.createShader(gl.VERTEX_SHADER);
      if (!vertexShader) return false;

      gl.shaderSource(vertexShader, vertexSrc);
      gl.compileShader(vertexShader);

      if (!gl.getShaderParameter(vertexShader, gl.COMPILE_STATUS)) {
        const error = gl.getShaderInfoLog(vertexShader);
        this.log(
          'fail',
          'WebGL',
          `Test vertex shader compilation failed: ${String(error)}`
        );
        return false;
      }

      // Compile fragment shader
      const fragmentShader = gl.createShader(gl.FRAGMENT_SHADER);
      if (!fragmentShader) return false;

      gl.shaderSource(fragmentShader, fragmentSrc);
      gl.compileShader(fragmentShader);

      if (!gl.getShaderParameter(fragmentShader, gl.COMPILE_STATUS)) {
        const error = gl.getShaderInfoLog(fragmentShader);
        this.log(
          'fail',
          'WebGL',
          `Test fragment shader compilation failed: ${String(error)}`
        );
        return false;
      }

      // Link program
      const program = gl.createProgram();
      if (!program) return false;

      gl.attachShader(program, vertexShader);
      gl.attachShader(program, fragmentShader);
      gl.linkProgram(program);

      if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
        const error = gl.getProgramInfoLog(program);
        this.log(
          'fail',
          'WebGL',
          `Test shader program linking failed: ${String(error)}`
        );
        return false;
      }

      return true;
    } catch (error) {
      this.log(
        'fail',
        'WebGL',
        `Test shader compilation error: ${String(error)}`
      );
      return false;
    }
  }

  private checkShaderDefinitions(): void {
    try {
      const shaderNames = Object.keys(SHADER_DEFINITIONS) as ShaderName[];

      if (shaderNames.length === 0) {
        this.log('fail', 'Definitions', 'No shader definitions found');
        return;
      }

      this.log(
        'pass',
        'Definitions',
        `Found ${shaderNames.length} shader definitions`
      );

      // Check hex-terrain specifically
      const hexTerrain = SHADER_DEFINITIONS['hex-terrain'];
      if (!hexTerrain) {
        this.log(
          'fail',
          'Definitions',
          'hex-terrain shader definition missing'
        );
        return;
      }

      // Check shader structure
      if (!hexTerrain.vertexShader) {
        this.log('fail', 'Definitions', 'hex-terrain missing vertex shader');
      } else if (hexTerrain.vertexShader.length < 100) {
        this.log(
          'warning',
          'Definitions',
          'hex-terrain vertex shader seems very short'
        );
      } else {
        this.log(
          'pass',
          'Definitions',
          `hex-terrain vertex shader: ${hexTerrain.vertexShader.length} chars`
        );
      }

      if (!hexTerrain.fragmentShader) {
        this.log('fail', 'Definitions', 'hex-terrain missing fragment shader');
      } else if (hexTerrain.fragmentShader.length < 100) {
        this.log(
          'warning',
          'Definitions',
          'hex-terrain fragment shader seems very short'
        );
      } else {
        this.log(
          'pass',
          'Definitions',
          `hex-terrain fragment shader: ${hexTerrain.fragmentShader.length} chars`
        );
      }

      // Check uniforms
      if (!hexTerrain.uniforms) {
        this.log('fail', 'Definitions', 'hex-terrain missing uniforms');
      } else {
        const uniformCount = Object.keys(hexTerrain.uniforms).length;
        this.log(
          'pass',
          'Definitions',
          `hex-terrain has ${uniformCount} uniforms`
        );
      }

      // Check critical uniforms
      const requiredUniforms = ['u_time', 'u_hexSize', 'u_heightScale'];
      for (const uniform of requiredUniforms) {
        if (!hexTerrain.uniforms?.[uniform]) {
          this.log(
            'fail',
            'Definitions',
            `hex-terrain missing required uniform: ${uniform}`
          );
        }
      }
    } catch (error) {
      this.log(
        'fail',
        'Definitions',
        `Shader definitions check failed: ${String(error)}`
      );
    }
  }

  private checkShaderManager(): void {
    try {
      if (!shaderManager) {
        this.log('fail', 'Manager', 'Shader manager not available');
        return;
      }

      this.log('pass', 'Manager', 'Shader manager available');

      // Test manager methods
      const methods = ['compile', 'get', 'has', 'clear', 'getStats'];
      for (const method of methods) {
        if (
          typeof (shaderManager as Record<string, unknown>)[method] !==
          'function'
        ) {
          this.log('fail', 'Manager', `Missing method: ${method}`);
        }
      }

      // Test manager stats
      try {
        const stats = shaderManager.getStats();
        this.log('info', 'Manager', `Manager stats: ${JSON.stringify(stats)}`);
      } catch (error) {
        this.log(
          'warning',
          'Manager',
          `Could not get manager stats: ${String(error)}`
        );
      }

      // Test compilation
      try {
        const hexTerrainDef = getShaderDefinition('hex-terrain');
        const material = shaderManager.compile('hex-terrain', hexTerrainDef);

        if (!material) {
          this.log(
            'fail',
            'Manager',
            'hex-terrain shader compilation returned null'
          );
        } else if (!(material instanceof THREE.ShaderMaterial)) {
          this.log(
            'fail',
            'Manager',
            'hex-terrain compilation did not return ShaderMaterial'
          );
        } else {
          this.log(
            'pass',
            'Manager',
            'hex-terrain shader compiled successfully'
          );

          // Check material properties
          if (!material.vertexShader) {
            this.log(
              'fail',
              'Manager',
              'Compiled material missing vertex shader'
            );
          }
          if (!material.fragmentShader) {
            this.log(
              'fail',
              'Manager',
              'Compiled material missing fragment shader'
            );
          }
          if (!material.uniforms) {
            this.log('fail', 'Manager', 'Compiled material missing uniforms');
          } else {
            this.log(
              'pass',
              'Manager',
              `Compiled material has ${Object.keys(material.uniforms).length} uniforms`
            );
          }
        }
      } catch (error) {
        this.log(
          'fail',
          'Manager',
          `Shader compilation failed: ${String(error)}`
        );
      }
    } catch (error) {
      this.log(
        'fail',
        'Manager',
        `Shader manager check failed: ${String(error)}`
      );
    }
  }

  private checkHexTerrainShader(): void {
    try {
      const hexTerrain = getShaderDefinition('hex-terrain');

      // Check shader source for common issues - use shader manager processed versions
      // This ensures we test the actual shaders that would be used in production
      const testMaterial = shaderManager.compile(
        'hex-terrain-test',
        hexTerrain,
        {
          defines: {
            QUALITY_LEVEL: 3,
            USE_SHADOWS: 0,
            USE_FOG: 1,
            USE_HDR: 1,
            USE_THREEJS_BUILTIN: 1,
          },
        }
      );

      const vertexSrc = testMaterial.vertexShader;
      const fragmentSrc = testMaterial.fragmentShader;

      // Check vertex shader requirements
      const vertexChecks = [
        { pattern: /gl_Position\s*=/, message: 'gl_Position assignment' },
        {
          pattern: /attribute\s+vec3\s+instancePosition/,
          message: 'instancePosition attribute',
        },
        {
          pattern: /attribute\s+vec3\s+instanceColor/,
          message: 'instanceColor attribute',
        },
        { pattern: /uniform\s+float\s+u_time/, message: 'u_time uniform' },
        { pattern: /void\s+main\s*\(\s*\)/, message: 'main function' },
      ];

      for (const check of vertexChecks) {
        if (!check.pattern.test(vertexSrc)) {
          this.log(
            'fail',
            'HexShader',
            `Vertex shader missing: ${check.message}`
          );
        } else {
          this.log('pass', 'HexShader', `Vertex shader has: ${check.message}`);
        }
      }

      // Check fragment shader requirements
      const fragmentChecks = [
        { pattern: /gl_FragColor\s*=/, message: 'gl_FragColor assignment' },
        { pattern: /varying\s+vec3\s+v_color/, message: 'v_color varying' },
        {
          pattern: /uniform\s+vec3\s+u_lightDirection/,
          message: 'u_lightDirection uniform',
        },
        { pattern: /void\s+main\s*\(\s*\)/, message: 'main function' },
      ];

      for (const check of fragmentChecks) {
        if (!check.pattern.test(fragmentSrc)) {
          this.log(
            'fail',
            'HexShader',
            `Fragment shader missing: ${check.message}`
          );
        } else {
          this.log(
            'pass',
            'HexShader',
            `Fragment shader has: ${check.message}`
          );
        }
      }

      // Test actual compilation with WebGL
      if (this.testShaderCompilation(vertexSrc, fragmentSrc)) {
        this.log(
          'pass',
          'HexShader',
          'Shader compiles successfully with WebGL'
        );
      } else {
        this.log('fail', 'HexShader', 'Shader fails to compile with WebGL');
      }
    } catch (error) {
      this.log(
        'fail',
        'HexShader',
        `Hex terrain shader check failed: ${String(error)}`
      );
    }
  }

  private testShaderCompilation(
    vertexSrc: string,
    fragmentSrc: string
  ): boolean {
    try {
      const canvas = document.createElement('canvas');
      const gl =
        canvas.getContext('webgl') ?? canvas.getContext('experimental-webgl');

      if (!gl) {
        this.log(
          'warning',
          'HexShader',
          'Cannot test compilation: WebGL not available'
        );
        return false;
      }

      return this.compileTestShader(
        gl as WebGLRenderingContext,
        vertexSrc,
        fragmentSrc
      );
    } catch (error) {
      this.log(
        'fail',
        'HexShader',
        `Shader compilation test error: ${String(error)}`
      );
      return false;
    }
  }

  private async checkShaderProvider(): Promise<void> {
    try {
      // Check if shader provider context is available
      // This is tricky to test outside of React, so we'll do basic checks

      this.log('info', 'Provider', 'Shader provider structure check');

      // Check if the render store is available (dependency of shader provider)
      try {
        const { useRenderStore } = await import('../stores/render-store');
        if (useRenderStore) {
          this.log('pass', 'Provider', 'Render store available');
        }
      } catch (error) {
        this.log(
          'fail',
          'Provider',
          `Render store not available: ${String(error)}`
        );
      }
    } catch (error) {
      this.log(
        'fail',
        'Provider',
        `Shader provider check failed: ${String(error)}`
      );
    }
  }

  private checkThreeJSCompatibility(): void {
    try {
      // Test Three.js shader material creation
      const testUniforms = {
        u_time: { value: 0 },
        u_test: { value: 1.0 },
      };

      const testVertexShader = `
        uniform float u_time;
        void main() {
          gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
        }
      `;

      const testFragmentShader = `
        precision mediump float;
        uniform float u_test;
        void main() {
          gl_FragColor = vec4(u_test, 0.0, 0.0, 1.0);
        }
      `;

      const material = new THREE.ShaderMaterial({
        vertexShader: testVertexShader,
        fragmentShader: testFragmentShader,
        uniforms: testUniforms,
      });

      if (!material) {
        this.log('fail', 'Three.js', 'Failed to create ShaderMaterial');
        return;
      }

      this.log('pass', 'Three.js', 'ShaderMaterial creation works');

      // Test instanced mesh creation
      const geometry = new THREE.CylinderGeometry(1, 1, 0.1, 6);
      const mesh = new THREE.InstancedMesh(geometry, material, 100);

      if (!mesh) {
        this.log('fail', 'Three.js', 'Failed to create InstancedMesh');
      } else {
        this.log('pass', 'Three.js', 'InstancedMesh creation works');
      }

      // Test instanced attributes
      try {
        geometry.setAttribute(
          'instancePosition',
          new THREE.InstancedBufferAttribute(new Float32Array(300), 3)
        );
        this.log('pass', 'Three.js', 'Instanced attributes work');
      } catch (error) {
        this.log(
          'fail',
          'Three.js',
          `Instanced attributes failed: ${String(error)}`
        );
      }
    } catch (error) {
      this.log(
        'fail',
        'Three.js',
        `Three.js compatibility check failed: ${String(error)}`
      );
    }
  }

  private printResults(): void {
    const stats = {
      total: this.results.length,
      pass: this.results.filter(r => r.status === 'pass').length,
      fail: this.results.filter(r => r.status === 'fail').length,
      warning: this.results.filter(r => r.status === 'warning').length,
      info: this.results.filter(r => r.status === 'info').length,
    };

    console.warn('\n🔍 SHADER DIAGNOSTICS COMPLETE');
    console.warn('================================');
    console.warn(`✅ Passed: ${stats.pass}`);
    console.warn(`❌ Failed: ${stats.fail}`);
    console.warn(`⚠️ Warnings: ${stats.warning}`);
    console.warn(`ℹ️ Info: ${stats.info}`);
    console.warn('');

    // Group results by component
    const byComponent: { [key: string]: DiagnosticResult[] } = {};
    for (const result of this.results) {
      if (!byComponent[result.component]) {
        byComponent[result.component] = [];
      }
      byComponent[result.component].push(result);
    }

    // Show failures first
    const failedComponents = Object.keys(byComponent).filter(component =>
      byComponent[component].some(r => r.status === 'fail')
    );

    if (failedComponents.length > 0) {
      console.error('🚨 CRITICAL ISSUES:');
      for (const component of failedComponents) {
        const failures = byComponent[component].filter(
          r => r.status === 'fail'
        );
        console.error(`   [${component}] ${failures.length} failures`);
        failures.forEach(failure => {
          console.error(`     • ${failure.message}`);
        });
      }
      console.warn('');
    }

    // Recommendations
    this.printRecommendations(stats, byComponent);
  }

  private printRecommendations(
    stats: Record<string, number>,
    byComponent: { [key: string]: DiagnosticResult[] }
  ): void {
    console.warn('💡 RECOMMENDATIONS:');

    if (stats.fail === 0) {
      console.warn('   🎉 Shader system appears to be working correctly!');
      console.warn("   ✨ If you're still seeing issues, check:");
      console.warn('     • Browser console for runtime errors');
      console.warn('     • Network tab for shader file loading');
      console.warn('     • React DevTools for component state');
      return;
    }

    // WebGL issues
    if (byComponent['WebGL']?.some(r => r.status === 'fail')) {
      console.warn('   🌐 WebGL Issues:');
      console.warn('     • Try a different browser or update graphics drivers');
      console.warn('     • Check if hardware acceleration is enabled');
      console.warn('     • Verify WebGL support at webglreport.com');
    }

    // Shader definition issues
    if (byComponent['Definitions']?.some(r => r.status === 'fail')) {
      console.warn('   📝 Shader Definition Issues:');
      console.warn('     • Check shader file imports and exports');
      console.warn('     • Verify GLSL syntax in shader files');
      console.warn('     • Ensure all required uniforms are defined');
    }

    // Manager issues
    if (byComponent['Manager']?.some(r => r.status === 'fail')) {
      console.warn('   ⚙️  Shader Manager Issues:');
      console.warn('     • Check shader manager initialization');
      console.warn('     • Verify compilation pipeline');
      console.warn('     • Check for circular dependencies');
    }

    // Three.js issues
    if (byComponent['Three.js']?.some(r => r.status === 'fail')) {
      console.warn('   🎮 Three.js Compatibility Issues:');
      console.warn('     • Update Three.js to latest version');
      console.warn('     • Check instanced rendering support');
      console.warn('     • Verify WebGL context configuration');
    }

    console.warn('\n🔧 Next Steps:');
    console.warn('   1. Fix critical issues (❌) first');
    console.warn('   2. Address warnings (⚠️) for better performance');
    console.warn('   3. Run diagnostics again to verify fixes');
    console.warn('   4. Test in multiple browsers if issues persist');
  }

  /**
   * Get results as JSON for external analysis
   */
  getResults(): DiagnosticResult[] {
    return [...this.results];
  }
}

// Create global instance
export const shaderDiagnostics = new ShaderDiagnostics();

// Convenience function for manual testing
export const runShaderDiagnostics = () => shaderDiagnostics.runDiagnostics();

// Auto-run in development with console command
if (import.meta.env.MODE === 'development' && typeof window !== 'undefined') {
  // Add to window for manual access
  (window as Record<string, unknown>).runShaderDiagnostics =
    runShaderDiagnostics;
  (window as Record<string, unknown>).shaderDiagnostics = shaderDiagnostics;

  console.warn('🔍 SHADER DIAGNOSTICS: Functions exposed to global scope');
  console.warn('   • runShaderDiagnostics() - Run full diagnostics');
  console.warn('   • shaderDiagnostics.getResults() - Get last results');

  // Auto-run diagnostics after a short delay
  setTimeout(() => {
    console.warn('🔍 SHADER DIAGNOSTICS: Auto-running in 3 seconds...');
    console.warn('   (Or type runShaderDiagnostics() to run manually)');
    void setTimeout(runShaderDiagnostics, 3000);
  }, 1000);
}

export default shaderDiagnostics;
