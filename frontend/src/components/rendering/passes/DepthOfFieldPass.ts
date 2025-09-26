/**
 * Depth of Field rendering pass with bokeh effects
 * Provides cinematic focus control with high-quality bokeh
 */

import {
  type Camera,
  DoubleSide,
  Mesh,
  type PerspectiveCamera,
  PlaneGeometry,
  type Scene,
  ShaderMaterial,
  Uniform,
  Vector2,
  type Vector3,
  type WebGLRenderer,
  WebGLRenderTarget,
} from 'three';

import { RenderPass, type RenderPassConfig } from '../core/RenderPass';

interface DoFOptions {
  focusDistance: number;
  focusRange: number;
  bokehSize: number;
  maxBlur: number;
  enabled: boolean;
  autoFocus: boolean;
  quality: 'low' | 'medium' | 'high';
}

/**
 * Advanced depth of field pass with bokeh rendering
 * Uses two-pass technique for optimal performance
 */
export class DepthOfFieldPass extends RenderPass {
  private options: DoFOptions;
  private screenQuad!: Mesh;
  private dofMaterial!: ShaderMaterial;
  private bokehMaterial!: ShaderMaterial;
  private blurTarget!: WebGLRenderTarget;
  private cocTarget!: WebGLRenderTarget;

  constructor(
    options: Partial<DoFOptions> = {},
    config: Partial<RenderPassConfig> = {}
  ) {
    super({
      name: 'depth-of-field',
      priority: 90, // After geometry, before final post-processing
      enabled: true,
      renderToScreen: false,
      clearColor: false,
      clearDepth: false,
      ...config,
    });

    this.options = {
      focusDistance: 10.0,
      focusRange: 5.0,
      bokehSize: 2.0,
      maxBlur: 10.0,
      enabled: true,
      autoFocus: false,
      quality: 'medium',
      ...options,
    };

    this.setupMaterials();
    this.setupGeometry();
  }

  private setupMaterials(): void {
    // Circle of confusion calculation material
    this.dofMaterial = new ShaderMaterial({
      uniforms: {
        tColor: new Uniform(null),
        tDepth: new Uniform(null),
        u_resolution: new Uniform(new Vector2()),
        u_cameraNear: new Uniform(0.1),
        u_cameraFar: new Uniform(1000.0),
        u_focusDistance: new Uniform(this.options.focusDistance),
        u_focusRange: new Uniform(this.options.focusRange),
        u_bokehSize: new Uniform(this.options.bokehSize),
        u_maxBlur: new Uniform(this.options.maxBlur),
        u_quality: new Uniform(this.getQualityLevel()),
      },
      vertexShader: /* glsl */ `
        varying vec2 vUv;
        
        void main() {
          vUv = uv;
          gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
        }
      `,
      fragmentShader: /* glsl */ `
        uniform sampler2D tColor;
        uniform sampler2D tDepth;
        uniform vec2 u_resolution;
        uniform float u_cameraNear;
        uniform float u_cameraFar;
        uniform float u_focusDistance;
        uniform float u_focusRange;
        uniform float u_bokehSize;
        uniform float u_maxBlur;
        uniform int u_quality;
        
        varying vec2 vUv;
        
        // Convert depth buffer to linear depth
        float linearizeDepth(float depth) {
          float z = depth * 2.0 - 1.0;
          return (2.0 * u_cameraNear * u_cameraFar) / 
                 (u_cameraFar + u_cameraNear - z * (u_cameraFar - u_cameraNear));
        }
        
        // Calculate circle of confusion
        float calculateCoC(float depth) {
          float focusRange = max(u_focusRange, 0.1);
          float distance = abs(depth - u_focusDistance);
          return clamp(distance / focusRange, 0.0, 1.0) * u_maxBlur;
        }
        
        // High-quality bokeh sampling pattern
        const int SAMPLE_COUNT_HIGH = 64;
        const int SAMPLE_COUNT_MEDIUM = 32;
        const int SAMPLE_COUNT_LOW = 16;
        
        vec3 sampleBokeh(vec2 uv, float coc) {
          if (coc < 0.5) return texture2D(tColor, uv).rgb;
          
          int sampleCount = u_quality == 2 ? SAMPLE_COUNT_HIGH :
                           u_quality == 1 ? SAMPLE_COUNT_MEDIUM : SAMPLE_COUNT_LOW;
          
          vec3 color = vec3(0.0);
          float totalWeight = 0.0;
          
          float radius = coc * u_bokehSize / u_resolution.x;
          
          for (int i = 0; i < SAMPLE_COUNT_HIGH; i++) {
            if (i >= sampleCount) break;
            
            // Generate spiral sampling pattern for natural bokeh
            float angle = float(i) * 2.3998277; // Golden angle
            float radiusScale = sqrt(float(i)) / sqrt(float(sampleCount));
            
            vec2 offset = vec2(cos(angle), sin(angle)) * radius * radiusScale;
            vec2 sampleUv = uv + offset;
            
            // Skip out-of-bounds samples
            if (sampleUv.x < 0.0 || sampleUv.x > 1.0 || 
                sampleUv.y < 0.0 || sampleUv.y > 1.0) continue;
            
            vec3 sampleColor = texture2D(tColor, sampleUv).rgb;
            float sampleDepth = linearizeDepth(texture2D(tDepth, sampleUv).r);
            float sampleCoC = calculateCoC(sampleDepth);
            
            // Weight samples based on CoC for proper bokeh layering
            float weight = 1.0;
            if (sampleCoC < coc * 0.8) {
              // Foreground samples get higher weight
              weight = 2.0;
            }
            
            color += sampleColor * weight;
            totalWeight += weight;
          }
          
          return totalWeight > 0.0 ? color / totalWeight : texture2D(tColor, uv).rgb;
        }
        
        void main() {
          float depth = linearizeDepth(texture2D(tDepth, vUv).r);
          float coc = calculateCoC(depth);
          
          vec3 focusedColor = sampleBokeh(vUv, coc);
          
          gl_FragColor = vec4(focusedColor, 1.0);
        }
      `,
      side: DoubleSide,
      transparent: false,
    });

    // Bokeh highlight enhancement material
    this.bokehMaterial = new ShaderMaterial({
      uniforms: {
        tColor: new Uniform(null),
        u_resolution: new Uniform(new Vector2()),
        u_bokehIntensity: new Uniform(1.0),
        u_highlightThreshold: new Uniform(0.8),
      },
      vertexShader: /* glsl */ `
        varying vec2 vUv;
        
        void main() {
          vUv = uv;
          gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
        }
      `,
      fragmentShader: /* glsl */ `
        uniform sampler2D tColor;
        uniform vec2 u_resolution;
        uniform float u_bokehIntensity;
        uniform float u_highlightThreshold;
        
        varying vec2 vUv;
        
        void main() {
          vec3 color = texture2D(tColor, vUv).rgb;
          
          // Enhance bright highlights for better bokeh visibility
          float luminance = dot(color, vec3(0.299, 0.587, 0.114));
          if (luminance > u_highlightThreshold) {
            float enhancement = (luminance - u_highlightThreshold) * u_bokehIntensity;
            color *= 1.0 + enhancement;
          }
          
          gl_FragColor = vec4(color, 1.0);
        }
      `,
      side: DoubleSide,
      transparent: false,
    });
  }

  private setupGeometry(): void {
    const geometry = new PlaneGeometry(2, 2);
    this.screenQuad = new Mesh(geometry, this.dofMaterial);
    this.screenQuad.frustumCulled = false;
  }

  private getQualityLevel(): number {
    switch (this.options.quality) {
      case 'low':
        return 0;
      case 'medium':
        return 1;
      case 'high':
        return 2;
      default:
        return 1;
    }
  }

  initialize(renderer: WebGLRenderer): void {
    const size = renderer.getSize(new Vector2());

    // Create render targets for multi-pass rendering
    this.blurTarget = new WebGLRenderTarget(size.x, size.y, {
      stencilBuffer: false,
      depthBuffer: false,
    });

    this.cocTarget = new WebGLRenderTarget(size.x, size.y, {
      stencilBuffer: false,
      depthBuffer: false,
    });

    (this.dofMaterial.uniforms.u_resolution.value as Vector2).set(
      size.x,
      size.y
    );
    (this.bokehMaterial.uniforms.u_resolution.value as Vector2).set(
      size.x,
      size.y
    );
  }

  resize(width: number, height: number): void {
    if (this.blurTarget) {
      this.blurTarget.setSize(width, height);
    }
    if (this.cocTarget) {
      this.cocTarget.setSize(width, height);
    }

    (this.dofMaterial.uniforms.u_resolution.value as Vector2).set(
      width,
      height
    );
    (this.bokehMaterial.uniforms.u_resolution.value as Vector2).set(
      width,
      height
    );
  }

  render(
    renderer: WebGLRenderer,
    _scene: Scene,
    camera: Camera,
    writeBuffer?: WebGLRenderTarget,
    readBuffer?: WebGLRenderTarget
  ): void {
    if (!this.enabled || !readBuffer || !this.options.enabled) return;

    // Update camera-dependent uniforms
    const perspectiveCamera = camera as PerspectiveCamera;
    this.dofMaterial.uniforms.u_cameraNear.value = perspectiveCamera.near;
    this.dofMaterial.uniforms.u_cameraFar.value = perspectiveCamera.far;
    this.dofMaterial.uniforms.u_focusDistance.value =
      this.options.focusDistance;
    this.dofMaterial.uniforms.u_focusRange.value = this.options.focusRange;
    this.dofMaterial.uniforms.u_bokehSize.value = this.options.bokehSize;
    this.dofMaterial.uniforms.u_maxBlur.value = this.options.maxBlur;
    this.dofMaterial.uniforms.u_quality.value = this.getQualityLevel();

    // Set input textures
    this.dofMaterial.uniforms.tColor.value = readBuffer.texture;
    this.dofMaterial.uniforms.tDepth.value = readBuffer.depthTexture;

    // Render DoF effect
    this.setRenderTarget(renderer, writeBuffer);
    renderer.render(this.screenQuad, camera);
  }

  dispose(): void {
    this.dofMaterial.dispose();
    this.bokehMaterial.dispose();
    this.screenQuad.geometry.dispose();

    if (this.blurTarget) {
      this.blurTarget.dispose();
    }
    if (this.cocTarget) {
      this.cocTarget.dispose();
    }
  }

  /**
   * Update DoF settings
   */
  updateSettings(newOptions: Partial<DoFOptions>): void {
    Object.assign(this.options, newOptions);
  }

  /**
   * Set focus distance for manual control
   */
  setFocusDistance(distance: number): void {
    this.options.focusDistance = Math.max(0.1, distance);
  }

  /**
   * Auto-focus on a world position
   */
  autoFocusOnPosition(worldPosition: Vector3, camera: Camera): void {
    if (!this.options.autoFocus) return;

    // Calculate distance from camera to focus point
    const focusDistance = camera.position.distanceTo(worldPosition);
    this.setFocusDistance(focusDistance);
  }
}
