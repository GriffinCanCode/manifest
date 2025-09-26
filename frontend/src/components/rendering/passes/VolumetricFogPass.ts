/**
 * Volumetric fog rendering pass with ray-marched effects
 * Provides atmospheric depth and dynamic lighting interaction
 */

import {
  Mesh,
  PlaneGeometry,
  Scene as ThreeScene,
  Vector2,
  Vector3,
  WebGLRenderTarget,
  type Camera,
  type PerspectiveCamera,
  type ShaderMaterial,
  type WebGLRenderer,
} from 'three';

import { getShaderDefinition } from '../../../shaders/definitions';
import { shaderManager } from '../../../shaders/manager';
import { RenderPass, type RenderPassConfig } from '../core/RenderPass';

interface VolumetricFogOptions {
  density: number;
  color: Vector3;
  scatteringCoefficient: number;
  absorptionCoefficient: number;
  lightIntensity: number;
  lightDirection: Vector3;
  fogNear: number;
  fogFar: number;
  steps: number;
  enabled: boolean;
  quality: 'low' | 'medium' | 'high' | 'ultra';
  useNoise: boolean;
  windSpeed: number;
  windDirection: Vector2;
}

/**
 * Advanced volumetric fog pass with ray marching
 * Supports dynamic lighting, noise perturbation, and atmospheric scattering
 */
export class VolumetricFogPass extends RenderPass {
  private options: VolumetricFogOptions;
  private screenQuad!: Mesh;
  private screenScene!: ThreeScene;
  private fogMaterial!: ShaderMaterial;
  private fogTarget!: WebGLRenderTarget;
  private frameCount: number = 0;

  constructor(
    options: Partial<VolumetricFogOptions> = {},
    config: Partial<RenderPassConfig> = {}
  ) {
    super({
      name: 'volumetric-fog',
      priority: 85, // After geometry, before post-processing
      enabled: true,
      renderToScreen: false,
      clearColor: false,
      clearDepth: false,
      ...config,
    });

    this.options = {
      density: 0.02,
      color: new Vector3(0.8, 0.9, 1.0),
      scatteringCoefficient: 0.1,
      absorptionCoefficient: 0.05,
      lightIntensity: 1.0,
      lightDirection: new Vector3(0.5, -0.5, 0.5).normalize(),
      fogNear: 1.0,
      fogFar: 100.0,
      steps: 32,
      enabled: true,
      quality: 'medium',
      useNoise: true,
      windSpeed: 0.5,
      windDirection: new Vector2(1.0, 0.3).normalize(),
      ...options,
    };

    this.setupMaterial();
    this.setupGeometry();
  }

  private setupMaterial(): void {
    const fogShaderDef = getShaderDefinition('volumetric-fog');

    this.fogMaterial = shaderManager.compile('volumetric-fog', fogShaderDef, {
      defines: {
        USE_VOLUMETRIC_FOG: 1,
        USE_NOISE: this.options.useNoise ? 1 : 0,
      },
      transparent: false,
    });

    // Update uniforms with custom values
    const { uniforms } = this.fogMaterial;
    if (uniforms) {
      // Fog properties
      if (uniforms.u_fogDensity)
        uniforms.u_fogDensity.value = this.options.density;
      if (uniforms.u_fogColor) uniforms.u_fogColor.value = this.options.color;
      if (uniforms.u_scatteringCoeff)
        uniforms.u_scatteringCoeff.value = this.options.scatteringCoefficient;
      if (uniforms.u_absorptionCoeff)
        uniforms.u_absorptionCoeff.value = this.options.absorptionCoefficient;
      if (uniforms.u_fogNear) uniforms.u_fogNear.value = this.options.fogNear;
      if (uniforms.u_fogFar) uniforms.u_fogFar.value = this.options.fogFar;
      if (uniforms.u_steps) uniforms.u_steps.value = this.getStepCount();

      // Lighting
      if (uniforms.u_lightDirection)
        uniforms.u_lightDirection.value = this.options.lightDirection;
      if (uniforms.u_lightIntensity)
        uniforms.u_lightIntensity.value = this.options.lightIntensity;

      // Wind and noise
      if (uniforms.u_windSpeed)
        uniforms.u_windSpeed.value = this.options.windSpeed;
      if (uniforms.u_windDirection)
        uniforms.u_windDirection.value = this.options.windDirection;
    }
  }

  private setupGeometry(): void {
    const geometry = new PlaneGeometry(2, 2);
    this.screenQuad = new Mesh(geometry, this.fogMaterial);
    this.screenQuad.frustumCulled = false;

    // Create a scene for the screen quad
    this.screenScene = new ThreeScene();
    this.screenScene.add(this.screenQuad);
  }

  private getStepCount(): number {
    switch (this.options.quality) {
      case 'low':
        return 16;
      case 'medium':
        return 32;
      case 'high':
        return 64;
      case 'ultra':
        return 128;
      default:
        return 32;
    }
  }

  initialize(renderer: WebGLRenderer): void {
    const size = renderer.getSize(new Vector2());

    this.fogTarget = new WebGLRenderTarget(size.x, size.y, {
      stencilBuffer: false,
      depthBuffer: false,
    });

    // Set resolution with safety check
    if (this.fogMaterial.uniforms.u_resolution) {
      (this.fogMaterial.uniforms.u_resolution.value as Vector2).set(
        size.x,
        size.y
      );
    }
  }

  resize(width: number, height: number): void {
    if (this.fogTarget) {
      this.fogTarget.setSize(width, height);
    }

    // Set resolution with safety check
    if (this.fogMaterial.uniforms.u_resolution) {
      (this.fogMaterial.uniforms.u_resolution.value as Vector2).set(
        width,
        height
      );
    }
  }

  render(
    renderer: WebGLRenderer,
    _scene: ThreeScene,
    camera: Camera,
    writeBuffer?: WebGLRenderTarget,
    readBuffer?: WebGLRenderTarget
  ): void {
    if (!this.enabled || !readBuffer || !this.options.enabled) return;

    this.frameCount++;

    // Update time-dependent uniforms with safety checks
    const { uniforms } = this.fogMaterial;

    if (uniforms.u_time) {
      uniforms.u_time.value = this.frameCount * 0.016;
    }

    if (uniforms.u_cameraPosition) {
      const { position } = camera;
      (uniforms.u_cameraPosition.value as Vector3).copy(position);
    }

    if (uniforms.u_projectionMatrixInverse) {
      const { projectionMatrixInverse } = camera;
      (
        uniforms.u_projectionMatrixInverse
          .value as typeof projectionMatrixInverse
      ).copy(projectionMatrixInverse);
    }

    if (uniforms.u_viewMatrixInverse) {
      const { matrixWorld } = camera;
      (uniforms.u_viewMatrixInverse.value as typeof matrixWorld).copy(
        matrixWorld
      );
    }

    // Handle camera properties safely
    const perspectiveCamera = camera as PerspectiveCamera;
    if (uniforms.u_cameraNear) {
      uniforms.u_cameraNear.value = perspectiveCamera.near ?? 0.1;
    }
    if (uniforms.u_cameraFar) {
      uniforms.u_cameraFar.value = perspectiveCamera.far ?? 1000.0;
    }

    // Update fog parameters with safety checks
    if (uniforms.u_fogDensity) {
      uniforms.u_fogDensity.value = this.options.density;
    }
    if (uniforms.u_fogColor) {
      (uniforms.u_fogColor.value as Vector3).copy(this.options.color);
    }
    if (uniforms.u_scatteringCoeff) {
      uniforms.u_scatteringCoeff.value = this.options.scatteringCoefficient;
    }
    if (uniforms.u_absorptionCoeff) {
      uniforms.u_absorptionCoeff.value = this.options.absorptionCoefficient;
    }
    if (uniforms.u_fogNear) {
      uniforms.u_fogNear.value = this.options.fogNear;
    }
    if (uniforms.u_fogFar) {
      uniforms.u_fogFar.value = this.options.fogFar;
    }
    if (uniforms.u_steps) {
      uniforms.u_steps.value = this.getStepCount();
    }

    // Update lighting with safety checks
    if (uniforms.u_lightDirection) {
      (uniforms.u_lightDirection.value as Vector3).copy(
        this.options.lightDirection
      );
    }
    if (uniforms.u_lightIntensity) {
      uniforms.u_lightIntensity.value = this.options.lightIntensity;
    }

    // Update wind and noise with safety checks
    if (uniforms.u_useNoise) {
      uniforms.u_useNoise.value = this.options.useNoise ? 1 : 0;
    }
    if (uniforms.u_windSpeed) {
      uniforms.u_windSpeed.value = this.options.windSpeed;
    }
    if (uniforms.u_windDirection) {
      (uniforms.u_windDirection.value as Vector2).copy(
        this.options.windDirection
      );
    }

    // Set input textures with safety checks
    if (uniforms.tColor) {
      uniforms.tColor.value = readBuffer.texture;
    }
    if (uniforms.tDepth) {
      uniforms.tDepth.value = readBuffer.depthTexture;
    }

    // Render volumetric fog
    this.setRenderTarget(renderer, writeBuffer);
    renderer.render(this.screenScene, camera);
  }

  dispose(): void {
    this.fogMaterial.dispose();
    this.screenQuad.geometry.dispose();

    if (this.fogTarget) {
      this.fogTarget.dispose();
    }
  }

  /**
   * Update fog settings
   */
  updateSettings(newOptions: Partial<VolumetricFogOptions>): void {
    Object.assign(this.options, newOptions);
  }

  /**
   * Set fog density
   */
  setDensity(density: number): void {
    this.options.density = Math.max(0, density);
  }

  /**
   * Set fog color
   */
  setColor(r: number, g: number, b: number): void {
    this.options.color.set(r, g, b);
  }

  /**
   * Update wind parameters for dynamic fog movement
   */
  setWind(speed: number, direction: Vector2): void {
    this.options.windSpeed = speed;
    this.options.windDirection.copy(direction).normalize();
  }
}
