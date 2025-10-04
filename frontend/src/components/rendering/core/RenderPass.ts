/**
 * Core render pass system for multi-step rendering pipeline
 * Provides extensible pass management with render target support
 */

import {
  type Camera,
  Color,
  Mesh,
  MeshBasicMaterial,
  OrthographicCamera,
  PlaneGeometry,
  type Scene,
  Scene as ThreeScene,
  type WebGLRenderer,
  type WebGLRenderTarget,
} from 'three';

// Extend Window interface for frame counting
declare global {
  interface Window {
    __renderFrameCount?: number;
  }
}

// Type for post-processing callback
type PostProcessingCallback = (
  renderer: WebGLRenderer,
  inputBuffer: WebGLRenderTarget,
  camera: Camera
) => void;

export interface RenderPassConfig {
  name: string;
  priority: number;
  enabled: boolean;
  renderToScreen: boolean;
  renderTarget?: WebGLRenderTarget;
  clearColor?: boolean;
  clearDepth?: boolean;
  clearStencil?: boolean;
}

export abstract class RenderPass {
  public readonly name: string;
  public readonly priority: number;
  public enabled: boolean;
  public renderToScreen: boolean;
  public renderTarget?: WebGLRenderTarget;

  protected clearColor: boolean;
  protected clearDepth: boolean;
  protected clearStencil: boolean;

  constructor(config: RenderPassConfig) {
    this.name = config.name;
    this.priority = config.priority;
    this.enabled = config.enabled;
    this.renderToScreen = config.renderToScreen;
    this.renderTarget = config.renderTarget;
    this.clearColor = config.clearColor ?? true;
    this.clearDepth = config.clearDepth ?? true;
    this.clearStencil = config.clearStencil ?? false;
  }

  /**
   * Execute this render pass
   */
  abstract render(
    renderer: WebGLRenderer,
    scene: Scene,
    camera: Camera,
    writeBuffer?: WebGLRenderTarget,
    readBuffer?: WebGLRenderTarget
  ): void;

  /**
   * Initialize pass resources
   */
  initialize?(renderer: WebGLRenderer): void;

  /**
   * Resize pass resources
   */
  resize?(width: number, height: number): void;

  /**
   * Cleanup pass resources
   */
  dispose?(): void;

  /**
   * Set render target and clear flags
   */
  protected setRenderTarget(
    renderer: WebGLRenderer,
    renderTarget?: WebGLRenderTarget | null
  ): void {
    const target = this.renderToScreen
      ? null
      : (renderTarget ?? this.renderTarget ?? null);
    renderer.setRenderTarget(target);

    if (this.clearColor || this.clearDepth || this.clearStencil) {
      renderer.clear(this.clearColor, this.clearDepth, this.clearStencil);
    }
  }
}

/**
 * Geometry pass for rendering scene objects
 */
export class GeometryPass extends RenderPass {
  constructor(config: Partial<RenderPassConfig> = {}) {
    super({
      name: 'geometry',
      priority: 0,
      enabled: true,
      renderToScreen: false,
      clearColor: true,
      clearDepth: true,
      ...config,
    });
  }

  render(
    renderer: WebGLRenderer,
    scene: Scene,
    camera: Camera,
    writeBuffer?: WebGLRenderTarget | null
  ): void {
    // Render to screen if writeBuffer is null/undefined OR renderToScreen is true
    const shouldRenderToScreen =
      writeBuffer === null || writeBuffer === undefined || this.renderToScreen;

    if (shouldRenderToScreen) {
      renderer.setRenderTarget(null);
      renderer.clear(true, true, false);
      renderer.render(scene, camera);
    } else {
      this.setRenderTarget(renderer, writeBuffer);
      renderer.render(scene, camera);
    }

    // Reduced debug logging
    if (import.meta.env.MODE === 'development') {
      const frameCount = window.__renderFrameCount ?? 0;
      if (frameCount % 60 === 0) {
        console.warn(
          `GeometryPass: Rendered scene to ${shouldRenderToScreen ? 'screen' : 'buffer'}`
        );
      }
    }
  }
}

/**
 * Shadow pass for rendering shadow maps
 */
export class ShadowPass extends RenderPass {
  constructor(config: Partial<RenderPassConfig> = {}) {
    super({
      name: 'shadow',
      priority: -10,
      enabled: true,
      renderToScreen: false,
      clearColor: false,
      clearDepth: true,
      ...config,
    });
  }

  render(renderer: WebGLRenderer, _scene: Scene, _camera: Camera): void {
    // Shadow rendering handled by Three.js shadow system
    // This pass ensures shadows are rendered before geometry
    this.setRenderTarget(renderer, this.renderTarget);
  }
}

/**
 * Post-process pass for effects processing
 * Coordinates with PostProcessingComposer for intelligent integration
 */
export class PostProcessPass extends RenderPass {
  private postProcessingCallback: PostProcessingCallback | null = null;

  constructor(config: Partial<RenderPassConfig> = {}) {
    super({
      name: 'postprocess',
      priority: 100,
      enabled: true,
      renderToScreen: true,
      clearColor: false,
      clearDepth: false,
      ...config,
    });
  }

  /**
   * Set the PostProcessingComposer callback for coordination
   */
  setPostProcessingCallback(callback: PostProcessingCallback): void {
    this.postProcessingCallback = callback;
  }

  render(
    renderer: WebGLRenderer,
    scene: Scene,
    camera: Camera,
    writeBuffer?: WebGLRenderTarget,
    readBuffer?: WebGLRenderTarget
  ): void {
    // This pass coordinates with PostProcessingComposer for intelligent integration
    const shouldRenderToScreen =
      writeBuffer === null || writeBuffer === undefined || this.renderToScreen;

    if (shouldRenderToScreen) {
      // Final pass: Use PostProcessingComposer to apply effects and render to screen
      if (readBuffer && this.postProcessingCallback) {
        // Smart integration: Use PostProcessingComposer's render function with our input buffer
        this.postProcessingCallback(renderer, readBuffer, camera);
      } else if (readBuffer) {
        // Fallback: Simple copy to screen
        this.copyBufferToScreen(renderer, readBuffer);
      } else {
        // Last resort: direct scene render
        renderer.setRenderTarget(null);
        renderer.clear(true, true, false);
        renderer.render(scene, camera);
      }
    } else {
      // Intermediate pass: just copy buffer
      this.setRenderTarget(renderer, writeBuffer);
      if (readBuffer) {
        this.copyBufferToScreen(renderer, readBuffer);
      }
    }

    // Reduced debug logging
    if (import.meta.env.MODE === 'development') {
      const frameCount = window.__renderFrameCount ?? 0;
      if (frameCount % 60 === 0) {
        const method = this.postProcessingCallback
          ? 'PostProcessingComposer'
          : 'direct copy';
        console.warn(
          `PostProcessPass: ${shouldRenderToScreen ? 'Applied effects to screen' : 'Applied effects to buffer'} using ${method}`
        );
      }
    }
  }

  private copyBufferToScreen(
    renderer: WebGLRenderer,
    sourceBuffer: WebGLRenderTarget
  ): void {
    // Advanced buffer copy using Three.js render-to-texture functionality
    try {
      // Create a temporary scene with a fullscreen quad
      const copyScene = new ThreeScene();
      const copyCamera = new OrthographicCamera(-1, 1, 1, -1, 0, 1);
      const copyMaterial = new MeshBasicMaterial({
        map: sourceBuffer.texture,
        transparent: false,
        depthTest: false,
        depthWrite: false,
      });
      const copyMesh = new Mesh(new PlaneGeometry(2, 2), copyMaterial);

      copyScene.add(copyMesh);

      // Render the buffer to screen
      const currentClearColor = renderer.getClearColor(new Color());
      const currentClearAlpha = renderer.getClearAlpha();

      renderer.setRenderTarget(null);
      renderer.clear(false, false, false); // Don't clear, just composite
      renderer.render(copyScene, copyCamera);

      // Restore previous state
      renderer.setClearColor(currentClearColor, currentClearAlpha);

      // Cleanup
      copyMaterial.dispose();
      copyMesh.geometry.dispose();

      // Reduced logging
      const frameCount = window.__renderFrameCount ?? 0;
      if (frameCount % 60 === 0) {
        console.warn('PostProcessPass: Successfully copied buffer to screen');
      }
    } catch (error) {
      console.warn('PostProcessPass: Buffer copy failed', error);
    }
  }
}
