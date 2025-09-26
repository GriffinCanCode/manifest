/**
 * Selection pass for object picking and highlighting
 * Renders selection buffers for efficient mouse picking
 */

import type { Camera, Material, Scene, WebGLRenderer } from 'three';
import {
  Color,
  Mesh,
  MeshBasicMaterial,
  RGBAFormat,
  UnsignedByteType,
  Vector2,
  WebGLRenderTarget,
} from 'three';

import type { RenderPassConfig } from '../core/RenderPass';
import { RenderPass } from '../core/RenderPass';

interface SelectionBuffer {
  target: WebGLRenderTarget;
  materials: Map<Mesh, Material>;
}

/**
 * Selection pass for object picking via color-coded rendering
 */
export class SelectionPass extends RenderPass {
  private selectionBuffer?: SelectionBuffer;
  private readonly selectionMaterial = new MeshBasicMaterial();
  private nextObjectId = 1;
  private readonly objectIds = new Map<Mesh, number>();
  private readonly idToObject = new Map<number, Mesh>();

  constructor(config: Partial<RenderPassConfig> = {}) {
    super({
      name: 'selection',
      priority: 50,
      enabled: true,
      renderToScreen: false,
      clearColor: true,
      clearDepth: true,
      ...config,
    });
  }

  initialize(renderer: WebGLRenderer): void {
    this.createSelectionBuffer(renderer);
  }

  resize(width: number, height: number): void {
    if (this.selectionBuffer) {
      this.selectionBuffer.target.setSize(width, height);
    }
  }

  render(
    renderer: WebGLRenderer,
    scene: Scene,
    camera: Camera,
    _writeBuffer?: WebGLRenderTarget
  ): void {
    if (!this.selectionBuffer) return;

    // Store original materials
    this.storeOriginalMaterials(scene);

    // Apply selection materials
    this.applySelectionMaterials(scene);

    // Render to selection buffer
    this.setRenderTarget(renderer, this.selectionBuffer.target);
    renderer.render(scene, camera);

    // Restore original materials
    this.restoreOriginalMaterials(scene);
  }

  dispose(): void {
    this.selectionBuffer?.target.dispose();
    this.selectionMaterial.dispose();
    this.objectIds.clear();
    this.idToObject.clear();
  }

  /**
   * Pick object at screen coordinates
   */
  pick(renderer: WebGLRenderer, x: number, y: number): Mesh | null {
    if (!this.selectionBuffer) return null;

    // Read pixel from selection buffer
    const pixelBuffer = new Uint8Array(4);
    renderer.readRenderTargetPixels(
      this.selectionBuffer.target,
      x,
      y,
      1,
      1,
      pixelBuffer
    );

    // Convert RGB to object ID
    const id = (pixelBuffer[0] << 16) | (pixelBuffer[1] << 8) | pixelBuffer[2];
    return this.idToObject.get(id) ?? null;
  }

  private createSelectionBuffer(renderer: WebGLRenderer): void {
    const size = renderer.getSize(new Vector2());

    this.selectionBuffer = {
      target: new WebGLRenderTarget(size.x, size.y, {
        format: RGBAFormat,
        type: UnsignedByteType,
        generateMipmaps: false,
      }),
      materials: new Map(),
    };
  }

  private storeOriginalMaterials(scene: Scene): void {
    if (!this.selectionBuffer) return;

    this.selectionBuffer.materials.clear();

    scene.traverse(object => {
      if (object instanceof Mesh && object.material && this.selectionBuffer) {
        this.selectionBuffer.materials.set(object, object.material);
      }
    });
  }

  private applySelectionMaterials(scene: Scene): void {
    scene.traverse(object => {
      if (object instanceof Mesh) {
        // Assign unique ID if not already assigned
        if (!this.objectIds.has(object)) {
          const id = this.nextObjectId++;
          this.objectIds.set(object, id);
          this.idToObject.set(id, object);
        }

        // Convert ID to color
        const id = this.objectIds.get(object);
        if (!id) return;

        const color = new Color();
        color.setHex(id);

        // Apply selection material
        this.selectionMaterial.color = color;
        object.material = this.selectionMaterial;
      }
    });
  }

  private restoreOriginalMaterials(scene: Scene): void {
    if (!this.selectionBuffer) return;

    scene.traverse(object => {
      if (object instanceof Mesh) {
        const originalMaterial = this.selectionBuffer!.materials.get(object);
        if (originalMaterial) {
          object.material = originalMaterial;
        }
      }
    });
  }
}
