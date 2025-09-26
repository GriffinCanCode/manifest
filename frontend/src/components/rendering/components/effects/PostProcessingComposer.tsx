/**
 * Enhanced Post-Processing Composer integrated with custom shader system
 * Uses our custom shaders instead of @react-three/postprocessing
 */

import { useThree } from '@react-three/fiber';
import React, { useCallback, useEffect, useMemo, useRef } from 'react';
import {
  HalfFloatType,
  Mesh,
  MeshBasicMaterial,
  PlaneGeometry,
  RGBAFormat,
  Scene,
  Vector2,
  WebGLRenderTarget,
  type Camera,
  type WebGLRenderer,
} from 'three';

import { useRenderStore } from '../../../../stores/render-store';
import { useShader } from '../../hooks';

import { ShadowCascadeRenderer } from './ShadowCascadeRenderer';

// Context for coordinating with RenderPipeline
const PostProcessingContext = React.createContext<{
  renderFromBuffer: (
    renderer: WebGLRenderer,
    inputBuffer: WebGLRenderTarget,
    camera: Camera
  ) => void;
} | null>(null);

export const usePostProcessingContext = () =>
  React.useContext(PostProcessingContext);

interface PostProcessingComposerProps {
  children?: React.ReactNode;
  enabled?: boolean;
  enableTAA?: boolean;
  enableSelectiveBloom?: boolean;
}

/**
 * Custom post-processing composer using our shader system
 */
export const PostProcessingComposer: React.FC<PostProcessingComposerProps> = ({
  children,
  enabled = true,
  enableTAA = true,
  enableSelectiveBloom: _enableSelectiveBloom = false,
}) => {
  const { postprocessing, quality, capabilities, isInitialized, shadows } =
    useRenderStore();
  const { gl, size } = useThree();

  // Get all postprocessing shaders
  const ssaoShader = useShader('ssao');
  const bloomShader = useShader('bloom');
  const fxaaShader = useShader('fxaa');
  const hdrToneMappingShader = useShader('hdr-tonemapping');
  const colorCorrectionShader = useShader('color-correction');
  const taaShader = useShader('taa');
  const motionBlurShader = useShader('motion-blur');

  // Render targets
  const renderTargets = useRef<{
    read: WebGLRenderTarget;
    write: WebGLRenderTarget;
  }>();
  const quadGeometry = useRef<PlaneGeometry>();
  const quadMesh = useRef<Mesh>();
  const postprocessingScene = useRef<Scene>();

  // Initialize render targets and geometry
  useEffect(() => {
    if (!gl || !enabled) return;

    // Create render targets
    const createRenderTarget = () => {
      return new WebGLRenderTarget(size.width, size.height, {
        format: RGBAFormat,
        type: capabilities?.supportsHDR ? HalfFloatType : undefined,
        generateMipmaps: false,
        stencilBuffer: false,
      });
    };

    renderTargets.current = {
      read: createRenderTarget(),
      write: createRenderTarget(),
    };

    // Create quad geometry and mesh
    quadGeometry.current = new PlaneGeometry(2, 2);
    quadMesh.current = new Mesh(quadGeometry.current);
    postprocessingScene.current = new Scene();
    postprocessingScene.current.add(quadMesh.current);

    return () => {
      renderTargets.current?.read.dispose();
      renderTargets.current?.write.dispose();
      quadGeometry.current?.dispose();
    };
  }, [gl, size.width, size.height, capabilities?.supportsHDR, enabled]);

  // Adaptive quality settings
  const adaptiveSettings = useMemo(() => {
    const gpuTier = capabilities?.gpuTier ?? 'medium';

    return {
      samples: gpuTier === 'high' ? 8 : gpuTier === 'medium' ? 4 : 2,
      effectIntensities: {
        bloom: quality.level === 'low' ? 0.3 : 0.5,
        aoRadius:
          quality.level === 'low'
            ? 0.5
            : quality.level === 'medium'
              ? 0.75
              : 1.0,
        aoIntensity: quality.level === 'low' ? 0.8 : 1.2,
        taa: quality.level === 'low' ? 0.5 : 0.8,
      },
    };
  }, [quality.level, capabilities?.gpuTier]);

  // Effect enable/disable logic
  const effectsEnabled = useMemo(
    () => ({
      ssao:
        postprocessing.ssao &&
        capabilities?.supportsFloatTextures &&
        ssaoShader,
      bloom: postprocessing.bloom && capabilities?.supportsHDR && bloomShader,
      toneMapping: capabilities?.supportsHDR && hdrToneMappingShader,
      fxaa:
        postprocessing.fxaa && quality.antialias && !enableTAA && fxaaShader,
      taa: enableTAA && quality.level !== 'low' && taaShader,
      colorCorrection: colorCorrectionShader && quality.level !== 'low',
      motionBlur: motionBlurShader && quality.level === 'ultra',
    }),
    [
      postprocessing,
      quality,
      capabilities,
      enableTAA,
      ssaoShader,
      bloomShader,
      hdrToneMappingShader,
      fxaaShader,
      taaShader,
      colorCorrectionShader,
      motionBlurShader,
    ]
  );

  // Custom postprocessing render
  const renderPostProcessing = useCallback(
    (renderer: WebGLRenderer, scene: Scene, camera: Camera) => {
      if (
        !renderTargets.current ||
        !quadMesh.current ||
        !postprocessingScene.current
      ) {
        return;
      }

      const { read, write } = renderTargets.current;
      let currentRead = read;
      let currentWrite = write;

      // Helper function to swap render targets
      const swapTargets = () => {
        [currentRead, currentWrite] = [currentWrite, currentRead];
      };

      // NOTE: Scene rendering is now handled by RenderPipeline GeometryPass
      // Input should come from the pipeline's render buffer
      // For now, render scene as fallback if no input buffer provided
      renderer.setRenderTarget(currentWrite);
      renderer.clear(true, true, false);
      renderer.render(scene, camera);

      // Apply postprocessing passes
      renderer.autoClear = false;

      // SSAO Pass
      if (effectsEnabled.ssao && ssaoShader) {
        swapTargets();
        const ssaoMaterial = ssaoShader.clone();
        quadMesh.current.material = ssaoMaterial;
        if (ssaoMaterial.uniforms) {
          ssaoMaterial.uniforms.tDiffuse = {
            value: currentRead.texture,
          };
          ssaoMaterial.uniforms.u_resolution = {
            value: new Vector2(size.width, size.height),
          };
        }
        renderer.setRenderTarget(currentWrite);
        renderer.render(postprocessingScene.current, camera);
      }

      // Bloom Pass
      if (effectsEnabled.bloom && bloomShader) {
        swapTargets();
        const bloomMaterial = bloomShader.clone();
        quadMesh.current.material = bloomMaterial;
        if (bloomMaterial.uniforms) {
          bloomMaterial.uniforms.tDiffuse = {
            value: currentRead.texture,
          };
          bloomMaterial.uniforms.u_intensity = {
            value: adaptiveSettings.effectIntensities.bloom,
          };
        }
        renderer.setRenderTarget(currentWrite);
        renderer.render(postprocessingScene.current, camera);
      }

      // HDR Tone Mapping Pass
      if (effectsEnabled.toneMapping && hdrToneMappingShader) {
        swapTargets();
        const toneMappingMaterial = hdrToneMappingShader.clone();
        quadMesh.current.material = toneMappingMaterial;
        if (toneMappingMaterial.uniforms) {
          toneMappingMaterial.uniforms.tDiffuse = {
            value: currentRead.texture,
          };
        }
        renderer.setRenderTarget(currentWrite);
        renderer.render(postprocessingScene.current, camera);
      }

      // Color Correction Pass
      if (effectsEnabled.colorCorrection && colorCorrectionShader) {
        swapTargets();
        const colorCorrectionMaterial = colorCorrectionShader.clone();
        quadMesh.current.material = colorCorrectionMaterial;
        if (colorCorrectionMaterial.uniforms) {
          colorCorrectionMaterial.uniforms.tDiffuse = {
            value: currentRead.texture,
          };
        }
        renderer.setRenderTarget(currentWrite);
        renderer.render(postprocessingScene.current, camera);
      }

      // TAA Pass
      if (effectsEnabled.taa && taaShader) {
        swapTargets();
        const taaMaterial = taaShader.clone();
        quadMesh.current.material = taaMaterial;
        if (taaMaterial.uniforms) {
          taaMaterial.uniforms.tDiffuse = {
            value: currentRead.texture,
          };
          taaMaterial.uniforms.u_alpha = {
            value: adaptiveSettings.effectIntensities.taa,
          };
        }
        renderer.setRenderTarget(currentWrite);
        renderer.render(postprocessingScene.current, camera);
      }

      // Motion Blur Pass
      if (effectsEnabled.motionBlur && motionBlurShader) {
        swapTargets();
        const motionBlurMaterial = motionBlurShader.clone();
        quadMesh.current.material = motionBlurMaterial;
        if (motionBlurMaterial.uniforms) {
          motionBlurMaterial.uniforms.tDiffuse = {
            value: currentRead.texture,
          };
        }
        renderer.setRenderTarget(currentWrite);
        renderer.render(postprocessingScene.current, camera);
      }

      // FXAA Pass (final)
      if (effectsEnabled.fxaa && fxaaShader) {
        swapTargets();
        const fxaaMaterial = fxaaShader.clone();
        quadMesh.current.material = fxaaMaterial;
        if (fxaaMaterial.uniforms) {
          fxaaMaterial.uniforms.tDiffuse = {
            value: currentRead.texture,
          };
          fxaaMaterial.uniforms.u_resolution = {
            value: new Vector2(size.width, size.height),
          };
        }
      } else {
        // Copy to screen if no FXAA
        swapTargets();
      }

      // Final render to screen
      renderer.setRenderTarget(null);
      renderer.render(postprocessingScene.current, camera);

      renderer.autoClear = true;
    },
    [
      effectsEnabled,
      adaptiveSettings,
      ssaoShader,
      bloomShader,
      hdrToneMappingShader,
      colorCorrectionShader,
      taaShader,
      motionBlurShader,
      fxaaShader,
      size.width,
      size.height,
    ]
  );

  /**
   * Render post-processing effects from an input buffer (called by RenderPipeline)
   */
  const renderFromBuffer = useCallback(
    (
      renderer: WebGLRenderer,
      inputBuffer: WebGLRenderTarget,
      camera: Camera
    ) => {
      if (
        !renderTargets.current ||
        !quadMesh.current ||
        !postprocessingScene.current
      ) {
        console.warn('PostProcessingComposer: Not ready, skipping render');
        return;
      }

      const currentRead = inputBuffer; // Start with input from RenderPipeline

      // Apply postprocessing passes using the input buffer
      renderer.autoClear = false;

      // STEP 3: Enable FXAA anti-aliasing effect (reduced logging)
      const frameCount = ((window as any).__renderFrameCount as number) ?? 0;
      if (frameCount % 180 === 0) {
        console.warn('PostProcessingComposer: effectsEnabled:', {
          fxaa: effectsEnabled.fxaa,
          fxaaShader: !!fxaaShader,
          bloom: effectsEnabled.bloom,
          ssao: effectsEnabled.ssao,
          toneMapping: effectsEnabled.toneMapping,
        });
      }

      // Enable FXAA with smart fallback
      if (effectsEnabled.fxaa && fxaaShader) {
        try {
          const fxaaMaterial = fxaaShader.clone();
          if (fxaaMaterial.uniforms) {
            fxaaMaterial.uniforms.tDiffuse = { value: currentRead.texture };
            fxaaMaterial.uniforms.u_resolution = {
              value: new Vector2(size.width, size.height),
            };
          }
          quadMesh.current.material = fxaaMaterial;
          if (frameCount % 180 === 0) {
            console.warn('PostProcessingComposer: ✅ FXAA enabled');
          }
        } catch (error) {
          console.warn(
            'PostProcessingComposer: FXAA failed, using copy fallback:',
            error
          );
          quadMesh.current.material = new MeshBasicMaterial({
            map: currentRead.texture,
          });
        }
      } else {
        // Simple copy material as fallback
        const copyMaterial = new MeshBasicMaterial({
          map: currentRead.texture,
        });
        quadMesh.current.material = copyMaterial;
        if (frameCount % 180 === 0) {
          console.warn('PostProcessingComposer: Using copy material fallback');
        }
      }

      // Final render to screen
      renderer.setRenderTarget(null);
      renderer.clear(false, false, false);
      renderer.render(postprocessingScene.current, camera);

      renderer.autoClear = true;

      if (frameCount % 180 === 0) {
        console.warn(
          'PostProcessingComposer: ✅ Successfully rendered from input buffer to screen'
        );
      }
    },
    [effectsEnabled, fxaaShader, size.width, size.height]
  );

  // Remove competing useFrame hook - let RenderPipeline control the render loop

  if (!isInitialized || !enabled || !postprocessing.enabled) {
    return (
      <PostProcessingContext.Provider value={{ renderFromBuffer }}>
        <ShadowCascadeRenderer
          enabled={shadows.enabled && capabilities?.supportsShadows}
          cascades={shadows.cascades}
          shadowMapSize={shadows.mapSize}
          maxFar={shadows.maxDistance}
          shadowBias={shadows.bias}
        >
          {children}
        </ShadowCascadeRenderer>
      </PostProcessingContext.Provider>
    );
  }

  return (
    <PostProcessingContext.Provider value={{ renderFromBuffer }}>
      <ShadowCascadeRenderer
        enabled={shadows.enabled && capabilities?.supportsShadows}
        cascades={shadows.cascades}
        shadowMapSize={shadows.mapSize}
        maxFar={shadows.maxDistance}
        shadowBias={shadows.bias}
      >
        {/* Children will be rendered through our custom postprocessing pipeline */}
        {children}

        {/* Expose render functions for parent pipeline to use */}
        <primitive
          object={{
            renderPostProcessing,
            renderFromBuffer,
            isCustomPostProcessing: true,
          }}
        />
      </ShadowCascadeRenderer>
    </PostProcessingContext.Provider>
  );
};

export default PostProcessingComposer;
