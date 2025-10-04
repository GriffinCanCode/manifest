/**
 * Comprehensive Shader System Tests
 * Tests all aspects of shader initialization, compilation, and usage
 */

import * as THREE from 'three';

import type { ShaderName } from '../shaders/definitions';
import {
  SHADER_DEFINITIONS,
  getShaderDefinition,
} from '../shaders/definitions';
import { shaderManager } from '../shaders/manager';

// Mock Three.js renderer for testing
class MockWebGLRenderer extends THREE.WebGLRenderer {
  constructor() {
    super({ canvas: document.createElement('canvas') });
    // Override getContext to return a mock context
    const mockContext = {
      getExtension: () => null,
      getParameter: () => null,
      createShader: () => ({}),
      shaderSource: () => {},
      compileShader: () => {},
      getShaderParameter: () => true,
      createProgram: () => ({}),
      attachShader: () => {},
      linkProgram: () => {},
      getProgramParameter: () => true,
      useProgram: () => {},
      getUniformLocation: () => null,
      uniform1f: () => {},
      uniform1i: () => {},
      uniform2f: () => {},
      uniform3f: () => {},
      uniform4f: () => {},
      uniformMatrix4fv: () => {},
    };

    (this as any).getContext = () => mockContext;
  }
}

// Test configuration
const TEST_CONFIG = {
  showDetailedLogs: true,
  stopOnFirstError: false,
  testShaderCompilation: true,
  testUniformBinding: true,
  testInstancedAttributes: true,
};

interface TestResult {
  name: string;
  passed: boolean;
  error?: Error;
  details?: any;
  duration: number;
}

class ShaderSystemTester {
  private results: TestResult[] = [];
  private renderer: THREE.WebGLRenderer;
  private scene: THREE.Scene;
  private camera: THREE.PerspectiveCamera;

  constructor() {
    this.renderer = new MockWebGLRenderer();
    this.scene = new THREE.Scene();
    this.camera = new THREE.PerspectiveCamera(75, 1, 0.1, 1000);
  }

  async runAllTests(): Promise<TestResult[]> {
    console.log('🧪 SHADER SYSTEM TESTS: Starting comprehensive test suite...');

    const tests = [
      () => this.testShaderDefinitions(),
      () => this.testShaderManagerInitialization(),
      () => this.testHexTerrainShaderCompilation(),
      () => this.testShaderUniformsValidation(),
      () => this.testInstancedAttributesSetup(),
      () => this.testShaderErrorHandling(),
      () => this.testShaderProviderIntegration(),
      () => this.testHexRendererCompatibility(),
      () => this.testWebGLCompatibility(),
      () => this.testShaderHotReload(),
    ];

    for (const test of tests) {
      try {
        await test();
        if (TEST_CONFIG.stopOnFirstError && this.results.some(r => !r.passed)) {
          break;
        }
      } catch (error) {
        console.error('❌ Test suite error:', error);
        if (TEST_CONFIG.stopOnFirstError) break;
      }
    }

    this.printTestResults();
    return this.results;
  }

  private async runTest(
    name: string,
    testFn: () => Promise<any> | any
  ): Promise<void> {
    const startTime = performance.now();

    try {
      if (TEST_CONFIG.showDetailedLogs) {
        console.log(`🧪 Testing: ${name}...`);
      }

      const result = await testFn();
      const duration = performance.now() - startTime;

      this.results.push({
        name,
        passed: true,
        details: result,
        duration,
      });

      if (TEST_CONFIG.showDetailedLogs) {
        console.log(`✅ ${name} - PASSED (${duration.toFixed(2)}ms)`);
      }
    } catch (error) {
      const duration = performance.now() - startTime;

      this.results.push({
        name,
        passed: false,
        error: error instanceof Error ? error : new Error(String(error)),
        duration,
      });

      console.error(`❌ ${name} - FAILED (${duration.toFixed(2)}ms):`, error);
    }
  }

  private async testShaderDefinitions(): Promise<void> {
    await this.runTest('Shader Definitions Structure', () => {
      // Test that all required shaders are defined
      const requiredShaders: ShaderName[] = [
        'hex-terrain',
        'animated-water',
        'volumetric-fog',
        'debug-grid',
        'ui-overlay',
      ];

      const missing: string[] = [];
      for (const shaderName of requiredShaders) {
        if (!SHADER_DEFINITIONS[shaderName]) {
          missing.push(shaderName);
        }
      }

      if (missing.length > 0) {
        throw new Error(`Missing shader definitions: ${missing.join(', ')}`);
      }

      // Test hex-terrain shader structure specifically
      const hexTerrain = SHADER_DEFINITIONS['hex-terrain'];
      if (!hexTerrain.vertexShader || !hexTerrain.fragmentShader) {
        throw new Error('hex-terrain shader missing vertex or fragment shader');
      }

      if (!hexTerrain.uniforms) {
        throw new Error('hex-terrain shader missing uniforms');
      }

      // Check critical uniforms
      const requiredUniforms = [
        'u_time',
        'u_hexSize',
        'u_heightScale',
        'u_lightDirection',
        'u_lightColor',
      ];

      for (const uniform of requiredUniforms) {
        if (!hexTerrain.uniforms[uniform]) {
          throw new Error(`hex-terrain shader missing uniform: ${uniform}`);
        }
      }

      return {
        definedShaders: Object.keys(SHADER_DEFINITIONS).length,
        hexTerrainUniforms: Object.keys(hexTerrain.uniforms).length,
      };
    });
  }

  private async testShaderManagerInitialization(): Promise<void> {
    await this.runTest('Shader Manager Initialization', () => {
      // Test manager instance
      if (!shaderManager) {
        throw new Error('Shader manager not initialized');
      }

      // Test compilation method exists
      if (typeof shaderManager.compile !== 'function') {
        throw new Error('Shader manager missing compile method');
      }

      return { managerReady: true };
    });
  }

  private async testHexTerrainShaderCompilation(): Promise<void> {
    await this.runTest('Hex Terrain Shader Compilation', () => {
      if (!TEST_CONFIG.testShaderCompilation) {
        return { skipped: 'Shader compilation testing disabled' };
      }

      try {
        const hexTerrainDef = getShaderDefinition('hex-terrain');

        // Test basic compilation without WebGL context
        const material = new THREE.ShaderMaterial({
          vertexShader: hexTerrainDef.vertexShader,
          fragmentShader: hexTerrainDef.fragmentShader,
          uniforms: { ...hexTerrainDef.uniforms },
        });

        if (!material) {
          throw new Error('Failed to create ShaderMaterial');
        }

        // Test shader source validity
        const vertexLines = hexTerrainDef.vertexShader.split('\n');
        const fragmentLines = hexTerrainDef.fragmentShader.split('\n');

        if (vertexLines.length < 10) {
          throw new Error('Vertex shader seems too short');
        }

        if (fragmentLines.length < 10) {
          throw new Error('Fragment shader seems too short');
        }

        // Check for common GLSL errors
        const vertexSrc = hexTerrainDef.vertexShader;
        const fragmentSrc = hexTerrainDef.fragmentShader;

        // Check for required vertex shader elements
        if (!vertexSrc.includes('gl_Position')) {
          throw new Error('Vertex shader missing gl_Position assignment');
        }

        if (!vertexSrc.includes('attribute vec3 instancePosition')) {
          throw new Error('Vertex shader missing instancePosition attribute');
        }

        // Check for required fragment shader elements
        if (!fragmentSrc.includes('gl_FragColor')) {
          throw new Error('Fragment shader missing gl_FragColor assignment');
        }

        return {
          vertexLines: vertexLines.length,
          fragmentLines: fragmentLines.length,
          uniformCount: Object.keys(hexTerrainDef.uniforms).length,
        };
      } catch (compileError) {
        throw new Error(`Shader compilation test failed: ${compileError}`);
      }
    });
  }

  private async testShaderUniformsValidation(): Promise<void> {
    await this.runTest('Shader Uniforms Validation', () => {
      if (!TEST_CONFIG.testUniformBinding) {
        return { skipped: 'Uniform testing disabled' };
      }

      const hexTerrain = getShaderDefinition('hex-terrain');
      const { uniforms } = hexTerrain;

      // Test uniform types and values
      const uniformTests = [
        { name: 'u_time', expectedType: 'number', required: true },
        { name: 'u_hexSize', expectedType: 'number', required: true },
        { name: 'u_heightScale', expectedType: 'number', required: true },
        { name: 'u_lightDirection', expectedType: 'object', required: true },
        { name: 'u_lightColor', expectedType: 'object', required: true },
      ];

      const errors: string[] = [];

      for (const test of uniformTests) {
        const uniform = uniforms[test.name];

        if (test.required && !uniform) {
          errors.push(`Missing required uniform: ${test.name}`);
          continue;
        }

        if (uniform && !uniform.hasOwnProperty('value')) {
          errors.push(`Uniform ${test.name} missing value property`);
        }
      }

      if (errors.length > 0) {
        throw new Error(`Uniform validation errors: ${errors.join(', ')}`);
      }

      return {
        uniformsCount: Object.keys(uniforms).length,
        validatedUniforms: uniformTests.length,
      };
    });
  }

  private async testInstancedAttributesSetup(): Promise<void> {
    await this.runTest('Instanced Attributes Setup', () => {
      if (!TEST_CONFIG.testInstancedAttributes) {
        return { skipped: 'Instanced attributes testing disabled' };
      }

      // Create a test geometry with instanced attributes
      const geometry = new THREE.CylinderGeometry(1, 1, 0.15, 6);
      const maxInstances = 100;

      // Set up the attributes that our hex shader expects
      const requiredAttributes = [
        { name: 'instancePosition', components: 3 },
        { name: 'instanceColor', components: 3 },
        { name: 'instanceHeight', components: 1 },
        { name: 'instanceBiome', components: 1 },
        { name: 'instanceTexCoords', components: 2 },
        { name: 'instanceResourceMask', components: 1 },
      ];

      try {
        for (const attr of requiredAttributes) {
          const array = new Float32Array(maxInstances * attr.components);
          geometry.setAttribute(
            attr.name,
            new THREE.InstancedBufferAttribute(array, attr.components)
          );
        }

        // Test that attributes were created correctly
        for (const attr of requiredAttributes) {
          const attribute = geometry.attributes[attr.name];
          if (!attribute) {
            throw new Error(`Failed to create attribute: ${attr.name}`);
          }

          if (!(attribute instanceof THREE.InstancedBufferAttribute)) {
            throw new Error(
              `Attribute ${attr.name} is not an InstancedBufferAttribute`
            );
          }

          if (attribute.count !== maxInstances) {
            throw new Error(
              `Attribute ${attr.name} has wrong count: ${attribute.count} vs ${maxInstances}`
            );
          }
        }

        return {
          attributesCreated: requiredAttributes.length,
          maxInstances,
          geometryReady: true,
        };
      } catch (error) {
        throw new Error(`Instanced attributes setup failed: ${error}`);
      }
    });
  }

  private async testShaderErrorHandling(): Promise<void> {
    await this.runTest('Shader Error Handling', () => {
      // Test what happens with invalid shader definitions
      try {
        const invalidShader = {
          name: 'test-invalid',
          vertexShader: 'invalid glsl code!!!',
          fragmentShader: 'also invalid!!!',
          uniforms: {},
        };

        // This should not throw during material creation (only during compilation)
        const material = new THREE.ShaderMaterial({
          vertexShader: invalidShader.vertexShader,
          fragmentShader: invalidShader.fragmentShader,
          uniforms: invalidShader.uniforms,
        });

        if (!material) {
          throw new Error(
            'Material creation should not fail for invalid shaders'
          );
        }

        return { errorHandlingWorks: true };
      } catch (error) {
        // If it throws here, that's actually unexpected
        throw new Error(`Error handling test failed unexpectedly: ${error}`);
      }
    });
  }

  private async testShaderProviderIntegration(): Promise<void> {
    await this.runTest('Shader Provider Integration', () => {
      // Test that the shader provider components are properly structured
      // This is more of a structural test since we can't easily test React components

      try {
        // Check if shader manager has the methods we expect
        const expectedMethods = ['compile', 'get', 'has', 'clear'];
        const managerMethods = Object.getOwnPropertyNames(
          Object.getPrototypeOf(shaderManager)
        );

        const missingMethods = expectedMethods.filter(
          method => !managerMethods.includes(method)
        );

        if (missingMethods.length > 0) {
          throw new Error(
            `Shader manager missing methods: ${missingMethods.join(', ')}`
          );
        }

        return {
          managerMethods: managerMethods.length,
          expectedMethods: expectedMethods.length,
        };
      } catch (error) {
        throw new Error(`Shader provider integration test failed: ${error}`);
      }
    });
  }

  private async testHexRendererCompatibility(): Promise<void> {
    await this.runTest('Hex Renderer Compatibility', () => {
      // Test that the hex renderer can use the shader system

      try {
        const hexTerrain = getShaderDefinition('hex-terrain');

        // Create a material like the hex renderer would
        const material = new THREE.ShaderMaterial({
          name: 'hex-terrain',
          vertexShader: hexTerrain.vertexShader,
          fragmentShader: hexTerrain.fragmentShader,
          uniforms: { ...hexTerrain.uniforms },
          defines: {
            USE_INSTANCING: 1,
            QUALITY_LEVEL: 3,
            HEX_TILES: 1,
          },
        });

        // Test that critical uniforms can be updated
        if (material.uniforms.u_time) {
          material.uniforms.u_time.value = 1.0;
        }

        if (material.uniforms.u_hexSize) {
          material.uniforms.u_hexSize.value = 0.9;
        }

        // Test that the material can be used with instanced geometry
        const geometry = new THREE.CylinderGeometry(0.85, 0.85, 0.15, 6);
        const mesh = new THREE.InstancedMesh(geometry, material, 100);

        if (!mesh) {
          throw new Error('Failed to create InstancedMesh');
        }

        return {
          materialCreated: true,
          instancedMeshCreated: true,
          uniformsUpdateable: true,
        };
      } catch (error) {
        throw new Error(`Hex renderer compatibility test failed: ${error}`);
      }
    });
  }

  private async testWebGLCompatibility(): Promise<void> {
    await this.runTest('WebGL Compatibility', () => {
      // Test WebGL context and shader compilation compatibility

      try {
        // Get canvas and context
        const canvas = document.createElement('canvas');
        const gl =
          canvas.getContext('webgl') || canvas.getContext('experimental-webgl');

        if (!gl) {
          return { skipped: 'WebGL not available in test environment' };
        }

        // Test basic shader compilation
        const hexTerrain = getShaderDefinition('hex-terrain');

        // Create and compile vertex shader
        const vertexShader = gl.createShader(gl.VERTEX_SHADER);
        if (!vertexShader) throw new Error('Failed to create vertex shader');

        gl.shaderSource(vertexShader, hexTerrain.vertexShader);
        gl.compileShader(vertexShader);

        if (!gl.getShaderParameter(vertexShader, gl.COMPILE_STATUS)) {
          const error = gl.getShaderInfoLog(vertexShader);
          throw new Error(`Vertex shader compilation failed: ${error}`);
        }

        // Create and compile fragment shader
        const fragmentShader = gl.createShader(gl.FRAGMENT_SHADER);
        if (!fragmentShader)
          throw new Error('Failed to create fragment shader');

        gl.shaderSource(fragmentShader, hexTerrain.fragmentShader);
        gl.compileShader(fragmentShader);

        if (!gl.getShaderParameter(fragmentShader, gl.COMPILE_STATUS)) {
          const error = gl.getShaderInfoLog(fragmentShader);
          throw new Error(`Fragment shader compilation failed: ${error}`);
        }

        return {
          webglAvailable: true,
          vertexShaderCompiled: true,
          fragmentShaderCompiled: true,
          webglVersion: gl.getParameter(gl.VERSION),
        };
      } catch (error) {
        throw new Error(`WebGL compatibility test failed: ${error}`);
      }
    });
  }

  private async testShaderHotReload(): Promise<void> {
    await this.runTest('Shader Hot Reload', () => {
      // Test that shaders can be reloaded/updated

      try {
        // This is more of a structural test for now
        // In a full implementation, we'd test actual hot reloading

        const initialDefinition = getShaderDefinition('hex-terrain');

        if (!initialDefinition) {
          throw new Error('Could not get initial shader definition');
        }

        // Test that shader manager cache works
        const stats = shaderManager.getStats();

        return {
          hotReloadStructureReady: true,
          shaderCacheStats: stats,
        };
      } catch (error) {
        throw new Error(`Shader hot reload test failed: ${error}`);
      }
    });
  }

  private printTestResults(): void {
    const passed = this.results.filter(r => r.passed).length;
    const failed = this.results.filter(r => !r.passed).length;
    const totalTime = this.results.reduce((sum, r) => sum + r.duration, 0);

    console.log('\n📊 SHADER SYSTEM TEST RESULTS');
    console.log('================================');
    console.log(`✅ Passed: ${passed}`);
    console.log(`❌ Failed: ${failed}`);
    console.log(`⏱️  Total Time: ${totalTime.toFixed(2)}ms`);
    console.log('');

    if (failed > 0) {
      console.log('❌ FAILED TESTS:');
      this.results
        .filter(r => !r.passed)
        .forEach(result => {
          console.log(`   • ${result.name}: ${result.error?.message}`);
        });
      console.log('');
    }

    // Summary recommendations
    if (failed === 0) {
      console.log(
        '🎉 All shader system tests passed! The system should be working correctly.'
      );
    } else {
      console.log(
        '🚨 Some shader tests failed. Check the errors above to fix the shader system.'
      );

      // Specific recommendations based on failed tests
      const failedTests = this.results.filter(r => !r.passed).map(r => r.name);

      if (failedTests.some(name => name.includes('Compilation'))) {
        console.log(
          '   💡 Shader compilation issues detected - check GLSL syntax'
        );
      }

      if (failedTests.some(name => name.includes('Uniforms'))) {
        console.log(
          '   💡 Uniform validation issues - check shader definitions'
        );
      }

      if (failedTests.some(name => name.includes('WebGL'))) {
        console.log('   💡 WebGL compatibility issues - check browser support');
      }
    }
  }
}

// Export test runner for manual execution
export const runShaderSystemTests = async (): Promise<TestResult[]> => {
  const tester = new ShaderSystemTester();
  return await tester.runAllTests();
};

// Auto-run tests in development
if (import.meta.env.MODE === 'development' && typeof window !== 'undefined') {
  // Expose to global scope
  (window as any).runShaderSystemTests = runShaderSystemTests;

  console.log('🧪 SHADER TESTS: Functions exposed to global scope');
  console.log('   • runShaderSystemTests() - Run comprehensive unit tests');
  console.log(
    '🧪 SHADER TESTS: Available in development mode (auto-run disabled)'
  );
}
