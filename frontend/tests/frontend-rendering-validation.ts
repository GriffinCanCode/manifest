/**
 * Frontend Rendering Validation Tests
 * Tests to validate that the frontend rendering pipeline is working correctly
 */

import { Vector3 } from 'three';

import type { GameTile } from '../src/utils/game-types';
import { createMockGameWorld } from '../src/utils/game-types';

interface RenderValidationResult {
  isValid: boolean;
  errors: string[];
  warnings: string[];
  data?: any;
}

/**
 * Test suite for frontend rendering validation
 */
export class FrontendRenderingValidator {
  private mockTiles: GameTile[] = [];
  private canvas: HTMLCanvasElement | null = null;
  private gl: WebGLRenderingContext | null = null;

  constructor() {
    // Generate mock tiles for testing
    const mockWorld = createMockGameWorld();
    this.mockTiles = mockWorld.tiles;
    this.initializeWebGL();
  }

  /**
   * Initialize WebGL context for testing
   */
  private initializeWebGL(): void {
    try {
      this.canvas = document.createElement('canvas');
      this.canvas.width = 1024;
      this.canvas.height = 1024;

      this.gl =
        this.canvas.getContext('webgl2') || this.canvas.getContext('webgl');

      if (!this.gl) {
        console.warn('WebGL not available for rendering tests');
      }
    } catch (error) {
      console.warn('Failed to initialize WebGL for testing:', error);
    }
  }

  /**
   * Run all frontend rendering validation tests
   */
  async runAllTests(): Promise<RenderValidationResult> {
    const results: RenderValidationResult = {
      isValid: true,
      errors: [],
      warnings: [],
    };

    console.log('🎨 Starting Frontend Rendering Validation Tests...');

    const tests = [
      this.testWebGLSupport,
      this.testShaderCompilation,
      this.testGeometryGeneration,
      this.testInstancedRenderingSetup,
      this.testMaterialProperties,
      this.testBVHManager,
      this.testFrustumCulling,
      this.testTileVisibility,
      this.testRenderPerformance,
    ];

    for (const test of tests) {
      try {
        const testResult = await test.call(this);
        if (!testResult.isValid) {
          results.isValid = false;
        }
        results.errors.push(...testResult.errors);
        results.warnings.push(...testResult.warnings);
      } catch (error) {
        results.isValid = false;
        results.errors.push(
          `Test ${test.name} failed with error: ${error instanceof Error ? error.message : String(error)}`
        );
      }
    }

    return results;
  }

  /**
   * Test 1: WebGL Support
   */
  async testWebGLSupport(): Promise<RenderValidationResult> {
    console.log('🖥️ Testing WebGL support...');

    const errors: string[] = [];
    const warnings: string[] = [];

    if (!this.canvas) {
      errors.push('Canvas element could not be created');
    }

    if (!this.gl) {
      errors.push('WebGL context is not available');
      return { isValid: false, errors, warnings };
    }

    // Check WebGL version
    const version = this.gl.getParameter(this.gl.VERSION);
    const renderer = this.gl.getParameter(this.gl.RENDERER);
    const vendor = this.gl.getParameter(this.gl.VENDOR);

    if (!version.includes('WebGL')) {
      errors.push(`Invalid WebGL version: ${version}`);
    }

    // Check extensions needed for instanced rendering
    const instancedArrays =
      this.gl.getExtension('ANGLE_instanced_arrays') ||
      this.gl.getExtension('WEBGL_instanced_arrays');

    if (!instancedArrays) {
      warnings.push(
        'Instanced arrays extension not available - performance may be reduced'
      );
    }

    // Check max texture units
    const maxTextureUnits = this.gl.getParameter(
      this.gl.MAX_TEXTURE_IMAGE_UNITS
    );
    if (maxTextureUnits < 8) {
      warnings.push(`Limited texture units available: ${maxTextureUnits}`);
    }

    return {
      isValid: errors.length === 0,
      errors,
      warnings,
      data: {
        version,
        renderer,
        vendor,
        maxTextureUnits,
        instancedArraysSupported: !!instancedArrays,
      },
    };
  }

  /**
   * Test 2: Shader Compilation
   */
  async testShaderCompilation(): Promise<RenderValidationResult> {
    console.log('🎯 Testing shader compilation...');

    if (!this.gl) {
      return {
        isValid: false,
        errors: ['WebGL not available for shader testing'],
        warnings: [],
      };
    }

    const errors: string[] = [];
    const warnings: string[] = [];

    // Simple vertex shader for testing
    const vertexShaderSource = `
      attribute vec3 position;
      uniform mat4 projectionMatrix;
      uniform mat4 modelViewMatrix;
      
      void main() {
        gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
      }
    `;

    // Simple fragment shader for testing
    const fragmentShaderSource = `
      precision mediump float;
      uniform vec3 color;
      
      void main() {
        gl_FragColor = vec4(color, 1.0);
      }
    `;

    try {
      // Compile vertex shader
      const vertexShader = this.gl.createShader(this.gl.VERTEX_SHADER);
      if (!vertexShader) {
        errors.push('Failed to create vertex shader');
        return { isValid: false, errors, warnings };
      }

      this.gl.shaderSource(vertexShader, vertexShaderSource);
      this.gl.compileShader(vertexShader);

      if (!this.gl.getShaderParameter(vertexShader, this.gl.COMPILE_STATUS)) {
        const error = this.gl.getShaderInfoLog(vertexShader);
        errors.push(`Vertex shader compilation failed: ${error}`);
      }

      // Compile fragment shader
      const fragmentShader = this.gl.createShader(this.gl.FRAGMENT_SHADER);
      if (!fragmentShader) {
        errors.push('Failed to create fragment shader');
        return { isValid: false, errors, warnings };
      }

      this.gl.shaderSource(fragmentShader, fragmentShaderSource);
      this.gl.compileShader(fragmentShader);

      if (!this.gl.getShaderParameter(fragmentShader, this.gl.COMPILE_STATUS)) {
        const error = this.gl.getShaderInfoLog(fragmentShader);
        errors.push(`Fragment shader compilation failed: ${error}`);
      }

      // Link program
      const program = this.gl.createProgram();
      if (!program) {
        errors.push('Failed to create shader program');
        return { isValid: false, errors, warnings };
      }

      this.gl.attachShader(program, vertexShader);
      this.gl.attachShader(program, fragmentShader);
      this.gl.linkProgram(program);

      if (!this.gl.getProgramParameter(program, this.gl.LINK_STATUS)) {
        const error = this.gl.getProgramInfoLog(program);
        errors.push(`Shader program linking failed: ${error}`);
      }

      // Clean up
      this.gl.deleteShader(vertexShader);
      this.gl.deleteShader(fragmentShader);
      this.gl.deleteProgram(program);
    } catch (error) {
      errors.push(
        `Shader compilation test failed: ${error instanceof Error ? error.message : String(error)}`
      );
    }

    return {
      isValid: errors.length === 0,
      errors,
      warnings,
    };
  }

  /**
   * Test 3: Geometry Generation
   */
  async testGeometryGeneration(): Promise<RenderValidationResult> {
    console.log('📐 Testing geometry generation...');

    const errors: string[] = [];
    const warnings: string[] = [];

    try {
      // Test hex geometry generation using Three.js CylinderGeometry
      const { CylinderGeometry } = await import('three');

      const geometry = new CylinderGeometry(1, 1, 1, 6); // Hex cylinder

      if (!geometry.attributes.position) {
        errors.push('Geometry missing position attribute');
      }

      if (!geometry.attributes.normal) {
        errors.push('Geometry missing normal attribute');
      }

      if (!geometry.attributes.uv) {
        warnings.push('Geometry missing UV attribute - textures may not work');
      }

      const vertexCount = geometry.attributes.position.count;
      if (vertexCount === 0) {
        errors.push('Geometry has no vertices');
      } else if (vertexCount < 12) {
        warnings.push(`Very low vertex count: ${vertexCount}`);
      }

      geometry.dispose();

      return {
        isValid: errors.length === 0,
        errors,
        warnings,
        data: {
          vertexCount,
          hasNormals: !!geometry.attributes.normal,
          hasUVs: !!geometry.attributes.uv,
        },
      };
    } catch (error) {
      return {
        isValid: false,
        errors: [
          `Geometry generation failed: ${error instanceof Error ? error.message : String(error)}`,
        ],
        warnings: [],
      };
    }
  }

  /**
   * Test 4: Instanced Rendering Setup
   */
  async testInstancedRenderingSetup(): Promise<RenderValidationResult> {
    console.log('⚡ Testing instanced rendering setup...');

    const errors: string[] = [];
    const warnings: string[] = [];

    try {
      const { InstancedMesh, CylinderGeometry, MeshBasicMaterial, Matrix4 } =
        await import('three');

      const geometry = new CylinderGeometry(1, 1, 1, 6);
      const material = new MeshBasicMaterial({ color: 0x00ff00 });
      const instanceCount = 1000;

      const instancedMesh = new InstancedMesh(
        geometry,
        material,
        instanceCount
      );

      if (instancedMesh.count !== instanceCount) {
        errors.push(
          `Instance count mismatch: expected ${instanceCount}, got ${instancedMesh.count}`
        );
      }

      // Test setting instance matrix
      const matrix = new Matrix4();
      matrix.setPosition(1, 2, 3);
      instancedMesh.setMatrixAt(0, matrix);

      // Test getting instance matrix
      const retrievedMatrix = new Matrix4();
      instancedMesh.getMatrixAt(0, retrievedMatrix);

      const position = new Vector3();
      const { Quaternion } = await import('three');
      const quaternion = new Quaternion();
      const scale = new Vector3();
      retrievedMatrix.decompose(position, quaternion, scale);

      if (
        Math.abs(position.x - 1) > 0.001 ||
        Math.abs(position.y - 2) > 0.001 ||
        Math.abs(position.z - 3) > 0.001
      ) {
        errors.push('Instance matrix position not preserved correctly');
      }

      // Clean up
      geometry.dispose();
      material.dispose();

      return {
        isValid: errors.length === 0,
        errors,
        warnings,
        data: {
          instanceCount,
          matrixSetGetWorking: errors.length === 0,
        },
      };
    } catch (error) {
      return {
        isValid: false,
        errors: [
          `Instanced rendering setup failed: ${error instanceof Error ? error.message : String(error)}`,
        ],
        warnings: [],
      };
    }
  }

  /**
   * Test 5: Material Properties
   */
  async testMaterialProperties(): Promise<RenderValidationResult> {
    console.log('🎨 Testing material properties...');

    const errors: string[] = [];
    const warnings: string[] = [];

    try {
      const { MeshStandardMaterial, Color } = await import('three');

      const material = new MeshStandardMaterial({
        color: 0x00ff00,
        metalness: 0.1,
        roughness: 0.8,
      });

      // Test color setting
      const testColor = new Color(0xff0000);
      material.color = testColor;

      if (material.color.getHex() !== 0xff0000) {
        errors.push('Material color not set correctly');
      }

      // Test transparency
      material.transparent = true;
      material.opacity = 0.5;

      if (!material.transparent || material.opacity !== 0.5) {
        errors.push('Material transparency not working');
      }

      // Test material properties (uniforms are created during rendering)
      if (typeof material.opacity !== 'number') {
        warnings.push('Material opacity property not initialized correctly');
      }

      material.dispose();

      return {
        isValid: errors.length === 0,
        errors,
        warnings,
        data: {
          colorSupported: material.color instanceof Color,
          transparencySupported: material.transparent,
        },
      };
    } catch (error) {
      return {
        isValid: false,
        errors: [
          `Material properties test failed: ${error instanceof Error ? error.message : String(error)}`,
        ],
        warnings: [],
      };
    }
  }

  /**
   * Test 6: BVH Manager Functionality (Mock)
   */
  async testBVHManager(): Promise<RenderValidationResult> {
    console.log('🌳 Testing BVH manager functionality...');

    const errors: string[] = [];
    const warnings: string[] = [];

    try {
      // Since we can't fully test the actual BVHManager without a full Three.js scene,
      // we'll test the basic concepts it depends on

      const { Box3, Vector3 } = await import('three');

      // Test bounding box calculations
      const positions = [
        new Vector3(0, 0, 0),
        new Vector3(1, 1, 1),
        new Vector3(-1, -1, -1),
        new Vector3(2, 2, 2),
      ];

      const boundingBox = new Box3();
      boundingBox.setFromPoints(positions);

      if (boundingBox.isEmpty()) {
        errors.push('Bounding box calculation failed');
      }

      const expectedMin = new Vector3(-1, -1, -1);
      const expectedMax = new Vector3(2, 2, 2);

      if (
        !boundingBox.min.equals(expectedMin) ||
        !boundingBox.max.equals(expectedMax)
      ) {
        errors.push('Bounding box dimensions incorrect');
      }

      // Test frustum intersection (basic concept)
      const { Frustum, PerspectiveCamera, Matrix4 } = await import('three');

      const camera = new PerspectiveCamera(75, 1, 0.1, 100);
      camera.position.set(0, 0, 5);
      camera.lookAt(0, 0, 0);
      camera.updateProjectionMatrix();

      const frustum = new Frustum();
      const cameraMatrix = new Matrix4();
      cameraMatrix.multiplyMatrices(
        camera.projectionMatrix,
        camera.matrixWorldInverse
      );
      frustum.setFromProjectionMatrix(cameraMatrix);

      // Test if origin is in frustum
      const originBox = new Box3(
        new Vector3(-0.5, -0.5, -0.5),
        new Vector3(0.5, 0.5, 0.5)
      );
      const isInFrustum = frustum.intersectsBox(originBox);

      if (!isInFrustum) {
        warnings.push('Frustum culling may be too aggressive');
      }

      return {
        isValid: errors.length === 0,
        errors,
        warnings,
        data: {
          boundingBoxWorking: !boundingBox.isEmpty(),
          frustumCullingWorking: isInFrustum,
        },
      };
    } catch (error) {
      return {
        isValid: false,
        errors: [
          `BVH manager test failed: ${error instanceof Error ? error.message : String(error)}`,
        ],
        warnings: [],
      };
    }
  }

  /**
   * Test 7: Frustum Culling
   */
  async testFrustumCulling(): Promise<RenderValidationResult> {
    console.log('🎭 Testing frustum culling...');

    const errors: string[] = [];
    const warnings: string[] = [];

    try {
      const { Frustum, PerspectiveCamera, Matrix4, Box3, Vector3 } =
        await import('three');

      const camera = new PerspectiveCamera(75, 1, 0.1, 100);
      camera.position.set(0, 0, 5);
      camera.updateProjectionMatrix();

      const frustum = new Frustum();
      const cameraMatrix = new Matrix4();
      cameraMatrix.multiplyMatrices(
        camera.projectionMatrix,
        camera.matrixWorldInverse
      );
      frustum.setFromProjectionMatrix(cameraMatrix);

      // Test objects that should be visible
      const visibleBox = new Box3(
        new Vector3(-1, -1, -1),
        new Vector3(1, 1, 1)
      );
      if (!frustum.intersectsBox(visibleBox)) {
        errors.push('Frustum culling rejecting objects that should be visible');
      }

      // Test objects that should be culled
      const culledBox = new Box3(
        new Vector3(100, 100, 100),
        new Vector3(101, 101, 101)
      );
      if (frustum.intersectsBox(culledBox)) {
        warnings.push(
          'Frustum culling not rejecting distant objects - performance may suffer'
        );
      }

      return {
        isValid: errors.length === 0,
        errors,
        warnings,
        data: {
          visibleObjectAccepted: frustum.intersectsBox(visibleBox),
          distantObjectRejected: !frustum.intersectsBox(culledBox),
        },
      };
    } catch (error) {
      return {
        isValid: false,
        errors: [
          `Frustum culling test failed: ${error instanceof Error ? error.message : String(error)}`,
        ],
        warnings: [],
      };
    }
  }

  /**
   * Test 8: Tile Visibility Calculation
   */
  async testTileVisibility(): Promise<RenderValidationResult> {
    console.log('👁️ Testing tile visibility calculation...');

    const errors: string[] = [];
    const warnings: string[] = [];

    if (this.mockTiles.length === 0) {
      return {
        isValid: false,
        errors: ['No mock tiles available for visibility testing'],
        warnings: [],
      };
    }

    try {
      const { Vector3, PerspectiveCamera } = await import('three');
      const { HexUtils } = await import('../src/utils/game-types');

      const camera = new PerspectiveCamera(75, 1, 0.1, 100);
      camera.position.set(0, 10, 10);
      camera.lookAt(0, 0, 0);

      let visibleTileCount = 0;
      let totalDistance = 0;

      for (const tile of this.mockTiles.slice(0, 50)) {
        // Test subset
        const [x, z] = HexUtils.hexToPixel(tile.hex);
        const tilePosition = new Vector3(x, tile.elevation * 0.5, z);

        const distance = camera.position.distanceTo(tilePosition);
        totalDistance += distance;

        // Simple visibility check (distance-based)
        const maxVisibleDistance = 50;
        if (distance <= maxVisibleDistance) {
          visibleTileCount++;
        }
      }

      const visibilityRatio =
        visibleTileCount / Math.min(50, this.mockTiles.length);
      const averageDistance =
        totalDistance / Math.min(50, this.mockTiles.length);

      if (visibilityRatio === 0) {
        errors.push(
          'No tiles calculated as visible - visibility calculation may be broken'
        );
      } else if (visibilityRatio === 1) {
        warnings.push(
          'All tiles calculated as visible - culling may not be working'
        );
      }

      return {
        isValid: errors.length === 0,
        errors,
        warnings,
        data: {
          visibleTileCount,
          totalTilesTested: Math.min(50, this.mockTiles.length),
          visibilityRatio,
          averageDistance,
        },
      };
    } catch (error) {
      return {
        isValid: false,
        errors: [
          `Tile visibility test failed: ${error instanceof Error ? error.message : String(error)}`,
        ],
        warnings: [],
      };
    }
  }

  /**
   * Test 9: Render Performance Analysis
   */
  async testRenderPerformance(): Promise<RenderValidationResult> {
    console.log('⚡ Testing render performance...');

    const errors: string[] = [];
    const warnings: string[] = [];

    try {
      const {
        InstancedMesh,
        CylinderGeometry,
        MeshBasicMaterial,
        Matrix4,
        Scene,
        PerspectiveCamera,
        WebGLRenderer,
      } = await import('three');

      // Create a simple render test
      const scene = new Scene();
      const camera = new PerspectiveCamera(75, 1, 0.1, 100);

      // Only create renderer if WebGL is available
      let renderer: any = null;
      if (this.gl && this.canvas) {
        renderer = new WebGLRenderer({ canvas: this.canvas, context: this.gl });
        renderer.setSize(256, 256); // Small size for testing
      } else {
        warnings.push(
          'WebGL renderer not available - performance test limited'
        );
      }

      const geometry = new CylinderGeometry(1, 1, 1, 6);
      const material = new MeshBasicMaterial({ color: 0x00ff00 });
      const instanceCount = 100; // Small count for testing

      const instancedMesh = new InstancedMesh(
        geometry,
        material,
        instanceCount
      );

      // Set up instances
      const matrix = new Matrix4();
      const setupStartTime = performance.now();

      for (let i = 0; i < instanceCount; i++) {
        matrix.setPosition((i % 10) * 2 - 10, 0, Math.floor(i / 10) * 2 - 10);
        instancedMesh.setMatrixAt(i, matrix);
      }
      instancedMesh.instanceMatrix.needsUpdate = true;

      const setupEndTime = performance.now();
      const setupTime = setupEndTime - setupStartTime;

      scene.add(instancedMesh);
      camera.position.set(0, 10, 20);
      camera.lookAt(0, 0, 0);

      // Render test
      let renderTime = 0;
      if (renderer) {
        const renderStartTime = performance.now();
        renderer.render(scene, camera);
        const renderEndTime = performance.now();
        renderTime = renderEndTime - renderStartTime;
      }

      // Performance thresholds
      if (setupTime > 100) {
        warnings.push(
          `Slow instance setup: ${setupTime.toFixed(2)}ms for ${instanceCount} instances`
        );
      }

      if (renderTime > 50 && renderer) {
        warnings.push(
          `Slow render time: ${renderTime.toFixed(2)}ms for ${instanceCount} instances`
        );
      }

      // Clean up
      geometry.dispose();
      material.dispose();
      if (renderer) {
        renderer.dispose();
      }

      return {
        isValid: errors.length === 0,
        errors,
        warnings,
        data: {
          instanceCount,
          setupTimeMs: setupTime,
          renderTimeMs: renderTime,
          rendererAvailable: !!renderer,
        },
      };
    } catch (error) {
      return {
        isValid: false,
        errors: [
          `Render performance test failed: ${error instanceof Error ? error.message : String(error)}`,
        ],
        warnings: [],
      };
    }
  }
}

/**
 * Utility function to run frontend rendering validation tests
 */
export async function validateFrontendRendering(): Promise<RenderValidationResult> {
  const validator = new FrontendRenderingValidator();
  return await validator.runAllTests();
}
