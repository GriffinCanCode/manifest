/**
 * Core render pass system for multi-step rendering pipeline
 * Provides extensible pass management with render target support
 */

import type { Camera, Scene, WebGLRenderer, WebGLRenderTarget } from 'three';

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
    writeBuffer?: WebGLRenderTarget
  ): void {
    this.setRenderTarget(renderer, writeBuffer);
    renderer.render(scene, camera);
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
 */
export class PostProcessPass extends RenderPass {
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

  render(
    renderer: WebGLRenderer,
    _scene: Scene,
    _camera: Camera,
    writeBuffer?: WebGLRenderTarget,
    _readBuffer?: WebGLRenderTarget
  ): void {
    // Post-processing handled by EffectComposer
    // This pass coordinates with existing PostProcessingComposer
    this.setRenderTarget(renderer, writeBuffer);
  }
}
