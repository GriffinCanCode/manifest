/**
 * Debug rendering pass for development visualization
 * Provides wireframes, bounding boxes, and debug overlays
 */

import {
  Box3,
  BoxHelper,
  BufferGeometry,
  Color,
  Float32BufferAttribute,
  Line,
  LineBasicMaterial,
  Mesh,
  WireframeGeometry,
  type Camera,
  type Object3D,
  type Scene,
  type WebGLRenderer,
} from 'three';

import { RenderPass, type RenderPassConfig } from '../core/RenderPass';

interface DebugOptions {
  showWireframes: boolean;
  showBoundingBoxes: boolean;
  showNormals: boolean;
  showGrid: boolean;
  wireframeColor: string;
  boundingBoxColor: string;
}

/**
 * Debug visualization pass for development
 */
export class DebugPass extends RenderPass {
  private readonly options: DebugOptions;
  private readonly wireframeMaterial: LineBasicMaterial;
  private readonly boundingBoxMaterial: LineBasicMaterial;
  private readonly debugHelpers: Set<Object3D> = new Set();

  constructor(
    options: Partial<DebugOptions> = {},
    config: Partial<RenderPassConfig> = {}
  ) {
    super({
      name: 'debug',
      priority: 200,
      enabled: process.env.NODE_ENV === 'development',
      renderToScreen: true,
      clearColor: false,
      clearDepth: false,
      ...config,
    });

    this.options = {
      showWireframes: false,
      showBoundingBoxes: false,
      showNormals: false,
      showGrid: true,
      wireframeColor: '#00ff00',
      boundingBoxColor: '#ff0000',
      ...options,
    };

    this.wireframeMaterial = new LineBasicMaterial({
      color: new Color(this.options.wireframeColor),
      transparent: true,
      opacity: 0.5,
    });

    this.boundingBoxMaterial = new LineBasicMaterial({
      color: new Color(this.options.boundingBoxColor),
      transparent: true,
      opacity: 0.8,
    });
  }

  render(renderer: WebGLRenderer, scene: Scene, camera: Camera): void {
    if (!this.enabled) return;

    // Update debug helpers
    this.updateDebugHelpers(scene);

    // Render debug overlay
    this.setRenderTarget(renderer);
    renderer.render(scene, camera);

    // Clean up temporary helpers
    this.cleanupDebugHelpers(scene);
  }

  dispose(): void {
    this.wireframeMaterial.dispose();
    this.boundingBoxMaterial.dispose();
    this.debugHelpers.clear();
  }

  /**
   * Update debug visualization options
   */
  updateOptions(newOptions: Partial<DebugOptions>): void {
    Object.assign(this.options, newOptions);

    // Update material colors
    this.wireframeMaterial.color.set(this.options.wireframeColor);
    this.boundingBoxMaterial.color.set(this.options.boundingBoxColor);
  }

  private updateDebugHelpers(scene: Scene): void {
    if (this.options.showBoundingBoxes) {
      this.addBoundingBoxHelpers(scene);
    }

    if (this.options.showWireframes) {
      this.addWireframeHelpers(scene);
    }
  }

  private addBoundingBoxHelpers(scene: Scene): void {
    scene.traverse(object => {
      if (object instanceof Mesh && object.geometry) {
        const box = new Box3().setFromObject(object);

        if (!box.isEmpty()) {
          const helper = new BoxHelper(object, this.options.boundingBoxColor);
          helper.userData.isDebugHelper = true;
          scene.add(helper);
          this.debugHelpers.add(helper);
        }
      }
    });
  }

  private addWireframeHelpers(scene: Scene): void {
    scene.traverse(object => {
      if (object instanceof Mesh && object.geometry) {
        // Create wireframe geometry
        const wireframeGeometry = new WireframeGeometry(object.geometry);
        const wireframeMesh = new Line(
          wireframeGeometry,
          this.wireframeMaterial
        );

        // Apply same transform as original object
        wireframeMesh.matrix.copy(object.matrix);
        wireframeMesh.matrixAutoUpdate = false;
        wireframeMesh.userData.isDebugHelper = true;

        scene.add(wireframeMesh);
        this.debugHelpers.add(wireframeMesh);
      }
    });
  }

  private cleanupDebugHelpers(scene: Scene): void {
    this.debugHelpers.forEach(helper => {
      scene.remove(helper);
      if ('dispose' in helper && typeof helper.dispose === 'function') {
        (helper as { dispose: () => void }).dispose();
      }
    });
    this.debugHelpers.clear();
  }

  /**
   * Create debug grid for scene visualization
   */
  createDebugGrid(size = 100, divisions = 100): Line {
    const vertices = [];
    const half = size / 2;
    const step = size / divisions;

    // Create grid lines
    for (let i = 0; i <= divisions; i++) {
      const pos = -half + i * step;

      // Vertical lines
      vertices.push(-half, 0, pos, half, 0, pos);

      // Horizontal lines
      vertices.push(pos, 0, -half, pos, 0, half);
    }

    const geometry = new BufferGeometry();
    geometry.setAttribute('position', new Float32BufferAttribute(vertices, 3));

    const material = new LineBasicMaterial({
      color: 0x888888,
      transparent: true,
      opacity: 0.2,
    });

    const grid = new Line(geometry, material);
    grid.userData.isDebugHelper = true;

    return grid;
  }
}
