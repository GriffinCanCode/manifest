/**
 * Volumetric fog rendering pass with ray-marched effects
 * Provides atmospheric depth and dynamic lighting interaction
 */

import {
  type Camera,
  DoubleSide,
  Mesh,
  type PerspectiveCamera,
  PlaneGeometry,
  ShaderMaterial,
  Scene as ThreeScene,
  Uniform,
  Vector2,
  Vector3,
  type WebGLRenderer,
  WebGLRenderTarget,
} from 'three';

import volumetricFogFragmentShader from '../../../shaders/fog/volumetric-fog.frag';
import volumetricFogVertexShader from '../../../shaders/fog/volumetric-fog.vert';
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
    this.fogMaterial = new ShaderMaterial({
      uniforms: {
        tColor: new Uniform(null),
        tDepth: new Uniform(null),
        u_resolution: new Uniform(new Vector2()),
        u_time: new Uniform(0),
        u_cameraPosition: new Uniform(new Vector3()),
        u_cameraMatrix: new Uniform(null),
        u_projectionMatrixInverse: new Uniform(null),
        u_viewMatrixInverse: new Uniform(null),
        u_cameraNear: new Uniform(0.1),
        u_cameraFar: new Uniform(1000.0),

        // Fog properties
        u_fogDensity: new Uniform(this.options.density),
        u_fogColor: new Uniform(this.options.color),
        u_scatteringCoeff: new Uniform(this.options.scatteringCoefficient),
        u_absorptionCoeff: new Uniform(this.options.absorptionCoefficient),
        u_fogNear: new Uniform(this.options.fogNear),
        u_fogFar: new Uniform(this.options.fogFar),
        u_steps: new Uniform(this.getStepCount()),

        // Lighting
        u_lightDirection: new Uniform(this.options.lightDirection),
        u_lightIntensity: new Uniform(this.options.lightIntensity),

        // Wind and noise
        u_useNoise: new Uniform(this.options.useNoise ? 1 : 0),
        u_windSpeed: new Uniform(this.options.windSpeed),
        u_windDirection: new Uniform(this.options.windDirection),
      },
      vertexShader: volumetricFogVertexShader,
      fragmentShader: volumetricFogFragmentShader,
      side: DoubleSide,
      transparent: false,
    });
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

    (this.fogMaterial.uniforms.u_resolution.value as Vector2).set(
      size.x,
      size.y
    );
  }

  resize(width: number, height: number): void {
    if (this.fogTarget) {
      this.fogTarget.setSize(width, height);
    }

    (this.fogMaterial.uniforms.u_resolution.value as Vector2).set(
      width,
      height
    );
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

    // Update time-dependent uniforms
    this.fogMaterial.uniforms.u_time.value = this.frameCount * 0.016;
    (this.fogMaterial.uniforms.u_cameraPosition.value as Vector3).copy(
      camera.position
    );
    (
      this.fogMaterial.uniforms.u_cameraMatrix.value as typeof camera.matrix
    ).copy(camera.matrix);
    (
      this.fogMaterial.uniforms.u_projectionMatrixInverse
        .value as typeof camera.projectionMatrixInverse
    ).copy(camera.projectionMatrixInverse);
    (
      this.fogMaterial.uniforms.u_viewMatrixInverse
        .value as typeof camera.matrixWorld
    ).copy(camera.matrixWorld);
    // Handle camera properties safely
    const perspectiveCamera = camera as PerspectiveCamera;
    this.fogMaterial.uniforms.u_cameraNear.value =
      perspectiveCamera.near ?? 0.1;
    this.fogMaterial.uniforms.u_cameraFar.value =
      perspectiveCamera.far ?? 1000.0;

    // Update fog parameters
    this.fogMaterial.uniforms.u_fogDensity.value = this.options.density;
    (this.fogMaterial.uniforms.u_fogColor.value as Vector3).copy(
      this.options.color
    );
    this.fogMaterial.uniforms.u_scatteringCoeff.value =
      this.options.scatteringCoefficient;
    this.fogMaterial.uniforms.u_absorptionCoeff.value =
      this.options.absorptionCoefficient;
    this.fogMaterial.uniforms.u_fogNear.value = this.options.fogNear;
    this.fogMaterial.uniforms.u_fogFar.value = this.options.fogFar;
    this.fogMaterial.uniforms.u_steps.value = this.getStepCount();

    // Update lighting
    (this.fogMaterial.uniforms.u_lightDirection.value as Vector3).copy(
      this.options.lightDirection
    );
    this.fogMaterial.uniforms.u_lightIntensity.value =
      this.options.lightIntensity;

    // Update wind and noise
    this.fogMaterial.uniforms.u_useNoise.value = this.options.useNoise ? 1 : 0;
    this.fogMaterial.uniforms.u_windSpeed.value = this.options.windSpeed;
    (this.fogMaterial.uniforms.u_windDirection.value as Vector2).copy(
      this.options.windDirection
    );

    // Set input textures
    this.fogMaterial.uniforms.tColor.value = readBuffer.texture;
    this.fogMaterial.uniforms.tDepth.value = readBuffer.depthTexture;

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
