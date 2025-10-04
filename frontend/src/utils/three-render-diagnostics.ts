/**
 * Three.js Rendering Diagnostic System
 * Deep inspection of Three.js rendering pipeline for tile visibility issues
 */

import * as THREE from 'three';

interface ThreeRenderDiagnosticResult {
  name: string;
  status: 'pass' | 'fail' | 'warning';
  message: string;
  data?: any;
}

class ThreeRenderDiagnostics {
  private results: ThreeRenderDiagnosticResult[] = [];

  async runRenderingDiagnostics(): Promise<ThreeRenderDiagnosticResult[]> {
    this.results = [];

    console.group('🎨 THREE.JS RENDERING DIAGNOSTICS');
    console.log('Deep inspection of Three.js rendering pipeline...');

    // Test 1: Find and inspect the canvas and renderer
    this.inspectCanvasAndRenderer();

    // Test 2: Find and inspect the scene
    this.inspectScene();

    // Test 3: Find and inspect cameras
    this.inspectCameras();

    // Test 4: Find and inspect instanced meshes (our hex tiles)
    this.inspectInstancedMeshes();

    // Test 5: Check frustum culling and visibility
    this.inspectVisibilityAndCulling();

    // Test 6: Inspect shaders and materials
    this.inspectShadersAndMaterials();

    // Test 7: Check instance matrices and positioning
    this.inspectInstanceMatrices();

    // Test 8: Check lighting
    this.inspectLighting();

    this.printRenderSummary();
    console.groupEnd();

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

  private inspectCanvasAndRenderer() {
    try {
      const canvas = document.querySelector('canvas');
      if (!canvas) {
        this.addResult('Canvas', 'fail', 'No canvas found');
        return;
      }

      const rect = canvas.getBoundingClientRect();
      const canvasInfo = {
        dimensions: `${canvas.width}x${canvas.height}`,
        displaySize: `${rect.width}x${rect.height}`,
        devicePixelRatio: window.devicePixelRatio || 1,
        style: {
          visibility: canvas.style.visibility || 'visible',
          display: canvas.style.display || 'block',
          opacity: canvas.style.opacity || '1',
        },
      };

      this.addResult(
        'Canvas Properties',
        'pass',
        'Canvas found and accessible',
        canvasInfo
      );

      // Check if there's a WebGL context active
      const gl = canvas.getContext('webgl2') || canvas.getContext('webgl');
      if (gl) {
        const viewport = gl.getParameter(gl.VIEWPORT);
        this.addResult(
          'WebGL Viewport',
          'pass',
          `Viewport: [${viewport.join(', ')}]`
        );
      }
    } catch (error) {
      this.addResult(
        'Canvas Inspection',
        'fail',
        `Error: ${(error as Error).message}`
      );
    }
  }

  private inspectScene() {
    try {
      // Try to find Three.js scene through various methods
      const win = window as any;
      let scene = null;
      const sceneInfo: any = {};

      // Method 1: Check for exposed scene on window
      if (win.__scene) {
        scene = win.__scene;
        sceneInfo.source = 'window.__scene';
      }

      // Method 2: Try to find through React Fiber (more complex)
      // This would require more intricate inspection

      if (scene) {
        sceneInfo.children = scene.children.length;
        sceneInfo.matrixWorldNeedsUpdate = scene.matrixWorldNeedsUpdate;
        sceneInfo.visible = scene.visible;
        sceneInfo.childTypes = scene.children.map(
          (child: any) => child.type || child.constructor.name
        );

        this.addResult(
          'Scene',
          'pass',
          `Scene found with ${scene.children.length} children`,
          sceneInfo
        );
      } else {
        this.addResult(
          'Scene',
          'warning',
          'Scene not accessible from diagnostics'
        );
      }
    } catch (error) {
      this.addResult(
        'Scene Inspection',
        'fail',
        `Error: ${(error as Error).message}`
      );
    }
  }

  private inspectCameras() {
    try {
      const win = window as any;
      let camera = null;
      const cameraInfo: any = {};

      if (win.__camera) {
        camera = win.__camera;
        cameraInfo.source = 'window.__camera';
      }

      if (camera) {
        cameraInfo.position = `(${camera.position.x.toFixed(2)}, ${camera.position.y.toFixed(2)}, ${camera.position.z.toFixed(2)})`;
        cameraInfo.rotation = `(${camera.rotation.x.toFixed(2)}, ${camera.rotation.y.toFixed(2)}, ${camera.rotation.z.toFixed(2)})`;

        if (camera.isPerspectiveCamera) {
          cameraInfo.fov = camera.fov;
          cameraInfo.near = camera.near;
          cameraInfo.far = camera.far;
          cameraInfo.aspect = camera.aspect;
        }

        // Calculate what the camera is looking at
        const direction = new THREE.Vector3();
        camera.getWorldDirection(direction);
        cameraInfo.lookingAt = `(${direction.x.toFixed(2)}, ${direction.y.toFixed(2)}, ${direction.z.toFixed(2)})`;

        this.addResult(
          'Camera',
          'pass',
          'Camera found and accessible',
          cameraInfo
        );
      } else {
        this.addResult(
          'Camera',
          'warning',
          'Camera not accessible from diagnostics'
        );
      }
    } catch (error) {
      this.addResult(
        'Camera Inspection',
        'fail',
        `Error: ${(error as Error).message}`
      );
    }
  }

  private inspectInstancedMeshes() {
    try {
      const win = window as any;

      // Try to find instanced meshes through various methods
      if (win.__instancedMesh) {
        const mesh = win.__instancedMesh;
        this.inspectSingleInstancedMesh(mesh, 'window.__instancedMesh');
      }

      if (win.__hexRendererDebug?.meshRef) {
        const mesh = win.__hexRendererDebug.meshRef;
        this.inspectSingleInstancedMesh(mesh, 'HexRenderer debug');
      }

      // If no meshes found, report it
      if (!win.__instancedMesh && !win.__hexRendererDebug?.meshRef) {
        this.addResult(
          'Instanced Meshes',
          'warning',
          'No instanced meshes accessible from diagnostics'
        );
      }
    } catch (error) {
      this.addResult(
        'Instanced Mesh Inspection',
        'fail',
        `Error: ${(error as Error).message}`
      );
    }
  }

  private inspectSingleInstancedMesh(mesh: any, source: string) {
    if (!mesh) return;

    const meshInfo: any = {
      source,
      visible: mesh.visible,
      count: mesh.count,
      geometry: mesh.geometry?.type || 'unknown',
      material: mesh.material?.type || 'unknown',
      frustumCulled: mesh.frustumCulled,
      castShadow: mesh.castShadow,
      receiveShadow: mesh.receiveShadow,
    };

    // Check bounds
    if (mesh.geometry) {
      mesh.geometry.computeBoundingBox();
      if (mesh.geometry.boundingBox) {
        const box = mesh.geometry.boundingBox;
        meshInfo.boundingBox = {
          min: `(${box.min.x.toFixed(2)}, ${box.min.y.toFixed(2)}, ${box.min.z.toFixed(2)})`,
          max: `(${box.max.x.toFixed(2)}, ${box.max.y.toFixed(2)}, ${box.max.z.toFixed(2)})`,
        };
      }
    }

    // Check matrix
    if (mesh.matrix) {
      meshInfo.matrixDeterminant = mesh.matrix.determinant().toFixed(6);
      meshInfo.matrixNeedsUpdate = mesh.matrixNeedsUpdate;
    }

    // Check material properties
    if (mesh.material) {
      meshInfo.materialVisible = mesh.material.visible;
      meshInfo.materialOpacity = mesh.material.opacity;
      meshInfo.materialTransparent = mesh.material.transparent;
      if (mesh.material.uniforms) {
        meshInfo.hasUniforms = true;
        meshInfo.uniformCount = Object.keys(mesh.material.uniforms).length;
      }
    }

    const status = mesh.visible && mesh.count > 0 ? 'pass' : 'fail';
    this.addResult(
      `Instanced Mesh (${source})`,
      status,
      `${mesh.count} instances, visible: ${mesh.visible}`,
      meshInfo
    );
  }

  private inspectVisibilityAndCulling() {
    try {
      const win = window as any;
      const camera = win.__camera;
      const mesh = win.__instancedMesh || win.__hexRendererDebug?.meshRef;

      if (camera && mesh) {
        // Create a frustum from the camera
        const frustum = new THREE.Frustum();
        const matrix = new THREE.Matrix4().multiplyMatrices(
          camera.projectionMatrix,
          camera.matrixWorldInverse
        );
        frustum.setFromProjectionMatrix(matrix);

        // Check if mesh is in frustum
        mesh.geometry.computeBoundingBox();
        const { boundingBox } = mesh.geometry;
        let inFrustum = false;

        if (boundingBox) {
          // Create a bounding sphere for frustum testing
          const center = boundingBox.getCenter(new THREE.Vector3());
          const size = boundingBox.getSize(new THREE.Vector3());
          const radius = size.length() * 0.5;

          const sphere = new THREE.Sphere(center, radius);
          inFrustum = frustum.intersectsSphere(sphere);
        }

        this.addResult(
          'Frustum Culling',
          inFrustum ? 'pass' : 'warning',
          inFrustum
            ? 'Mesh visible in camera frustum'
            : 'Mesh outside camera frustum'
        );
      } else {
        this.addResult(
          'Frustum Culling',
          'warning',
          'Camera or mesh not accessible for frustum test'
        );
      }
    } catch (error) {
      this.addResult(
        'Visibility Inspection',
        'fail',
        `Error: ${(error as Error).message}`
      );
    }
  }

  private inspectShadersAndMaterials() {
    try {
      const win = window as any;
      const mesh = win.__instancedMesh || win.__hexRendererDebug?.meshRef;

      if (mesh?.material) {
        const { material } = mesh;
        const shaderInfo: any = {
          type: material.type,
          visible: material.visible,
          opacity: material.opacity,
          transparent: material.transparent,
          side: material.side,
        };

        if (material.type === 'ShaderMaterial') {
          shaderInfo.hasVertexShader = !!material.vertexShader;
          shaderInfo.hasFragmentShader = !!material.fragmentShader;
          shaderInfo.uniformsCount = material.uniforms
            ? Object.keys(material.uniforms).length
            : 0;

          // Check key uniforms
          if (material.uniforms) {
            const { uniforms } = material;
            shaderInfo.keyUniforms = {};

            // Check commonly problematic uniforms
            if (uniforms.u_time)
              shaderInfo.keyUniforms.u_time = uniforms.u_time.value;
            if (uniforms.u_lightDirection) {
              const ld = uniforms.u_lightDirection.value;
              shaderInfo.keyUniforms.u_lightDirection = `(${ld.x?.toFixed(2)}, ${ld.y?.toFixed(2)}, ${ld.z?.toFixed(2)})`;
            }
            if (uniforms.u_hasAlbedoTexture)
              shaderInfo.keyUniforms.u_hasAlbedoTexture =
                uniforms.u_hasAlbedoTexture.value;
          }
        }

        this.addResult(
          'Shader Material',
          'pass',
          'Material accessible and configured',
          shaderInfo
        );
      } else {
        this.addResult('Shader Material', 'warning', 'Material not accessible');
      }
    } catch (error) {
      this.addResult(
        'Shader Inspection',
        'fail',
        `Error: ${(error as Error).message}`
      );
    }
  }

  private inspectInstanceMatrices() {
    try {
      const win = window as any;
      const mesh = win.__instancedMesh || win.__hexRendererDebug?.meshRef;

      if (mesh?.instanceMatrix) {
        const matrixInfo: any = {
          count: mesh.count,
          needsUpdate: mesh.instanceMatrix.needsUpdate,
          arrayLength: mesh.instanceMatrix.array.length,
          expectedLength: mesh.count * 16, // 4x4 matrix = 16 floats
        };

        // Sample first few matrices
        const sampleMatrices = [];
        for (let i = 0; i < Math.min(3, mesh.count); i++) {
          const matrix = new THREE.Matrix4();
          mesh.getMatrixAt(i, matrix);
          const pos = new THREE.Vector3();
          const rot = new THREE.Euler();
          const scale = new THREE.Vector3();
          matrix.decompose(pos, new THREE.Quaternion(), scale);

          sampleMatrices.push({
            instance: i,
            position: `(${pos.x.toFixed(2)}, ${pos.y.toFixed(2)}, ${pos.z.toFixed(2)})`,
            scale: `(${scale.x.toFixed(2)}, ${scale.y.toFixed(2)}, ${scale.z.toFixed(2)})`,
          });
        }
        matrixInfo.sampleMatrices = sampleMatrices;

        this.addResult(
          'Instance Matrices',
          'pass',
          `${mesh.count} instance matrices configured`,
          matrixInfo
        );
      } else {
        this.addResult(
          'Instance Matrices',
          'warning',
          'Instance matrices not accessible'
        );
      }
    } catch (error) {
      this.addResult(
        'Matrix Inspection',
        'fail',
        `Error: ${(error as Error).message}`
      );
    }
  }

  private inspectLighting() {
    try {
      const win = window as any;
      const scene = win.__scene;

      if (scene) {
        const lights: any[] = [];
        scene.traverse((child: any) => {
          if (child.isLight) {
            lights.push({
              type: child.type,
              intensity: child.intensity,
              position: child.position
                ? `(${child.position.x.toFixed(2)}, ${child.position.y.toFixed(2)}, ${child.position.z.toFixed(2)})`
                : 'N/A',
              visible: child.visible,
            });
          }
        });

        this.addResult(
          'Scene Lighting',
          lights.length > 0 ? 'pass' : 'warning',
          `${lights.length} lights found`,
          { lights }
        );
      } else {
        this.addResult(
          'Scene Lighting',
          'warning',
          'Scene not accessible for lighting inspection'
        );
      }
    } catch (error) {
      this.addResult(
        'Lighting Inspection',
        'fail',
        `Error: ${(error as Error).message}`
      );
    }
  }

  private printRenderSummary() {
    const passed = this.results.filter(r => r.status === 'pass').length;
    const failed = this.results.filter(r => r.status === 'fail').length;
    const warnings = this.results.filter(r => r.status === 'warning').length;

    console.log('\n📊 RENDERING DIAGNOSTIC SUMMARY:');
    console.log(`✅ Passed: ${passed}`);
    console.log(`❌ Failed: ${failed}`);
    console.log(`⚠️ Warnings: ${warnings}`);

    if (failed > 0) {
      console.log('\n🚨 CRITICAL RENDERING ISSUES:');
      this.results
        .filter(r => r.status === 'fail')
        .forEach(r => {
          console.log(`❌ ${r.name}: ${r.message}`);
        });
    }

    // Specific rendering advice
    console.log('\n🎨 RENDERING TROUBLESHOOTING ADVICE:');
    console.log('1. Check if instanced mesh has correct count and is visible');
    console.log('2. Verify camera is positioned to see the rendered objects');
    console.log('3. Ensure instance matrices are properly set and updated');
    console.log('4. Check material opacity and transparency settings');
    console.log('5. Verify frustum culling is not hiding objects');
    console.log('6. Check for shader compilation errors or uniform issues');
  }

  getResults(): ThreeRenderDiagnosticResult[] {
    return this.results;
  }
}

// Create global instance
export const threeRenderDiagnostics = new ThreeRenderDiagnostics();

// Expose to window for manual testing
if (typeof window !== 'undefined') {
  (window as any).__threeRenderDiagnostics = threeRenderDiagnostics;
  (window as any).runThreeRenderDiagnostics = () =>
    threeRenderDiagnostics.runRenderingDiagnostics();
}
