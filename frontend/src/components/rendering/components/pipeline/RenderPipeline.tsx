/**
 * Multi-step rendering pipeline orchestrator
 * Manages render passes, targets, and execution order
 */

import { useFrame, useThree } from '@react-three/fiber';
import React, { useCallback, useEffect, useRef, useState } from 'react';
import {
  ClampToEdgeWrapping,
  FloatType,
  LinearFilter,
  RGBAFormat,
  UnsignedByteType,
  WebGLRenderTarget,
} from 'three';

import { useRenderStore } from '../../../../stores/render-store';
import { type RenderPass, PostProcessPass } from '../../core/RenderPass';
import { usePostProcessingContext } from '../effects/PostProcessingComposer';

interface RenderTargetConfig {
  name: string;
  width: number;
  height: number;
  format?: number;
  type?: typeof FloatType | typeof UnsignedByteType;
  generateMipmaps?: boolean;
  samples?: number;
}

interface RenderPipelineProps {
  children: React.ReactNode;
  passes?: RenderPass[];
  enableMultiSampling?: boolean;
}

/**
 * Core rendering pipeline that orchestrates multiple rendering passes
 */
export const RenderPipeline: React.FC<RenderPipelineProps> = ({
  children,
  passes: customPasses = [],
  enableMultiSampling = true,
}) => {
  const { gl, scene, camera, size } = useThree();
  const { capabilities, quality, postprocessing } = useRenderStore();
  const postProcessingContext = usePostProcessingContext();

  const renderTargetsRef = useRef<Map<string, WebGLRenderTarget>>(new Map());
  const passesRef = useRef<RenderPass[]>([]);
  const [isInitialized, setIsInitialized] = useState(false);

  /**
   * Create render target with quality-based settings
   */
  const createRenderTarget = useCallback(
    (config: RenderTargetConfig): WebGLRenderTarget => {
      const samples =
        enableMultiSampling && capabilities?.supportsWebGL2
          ? (config.samples ??
            (quality.level === 'low' ? 2 : quality.level === 'medium' ? 4 : 8))
          : 0;

      const target = new WebGLRenderTarget(config.width, config.height, {
        format: config.format ?? RGBAFormat,
        type:
          config.type ??
          (capabilities?.supportsFloatTextures ? FloatType : UnsignedByteType),
        generateMipmaps: config.generateMipmaps ?? false,
        minFilter: LinearFilter,
        magFilter: LinearFilter,
        wrapS: ClampToEdgeWrapping,
        wrapT: ClampToEdgeWrapping,
        samples,
      });

      target.texture.name = config.name;
      return target;
    },
    [capabilities, enableMultiSampling, quality.level]
  );

  /**
   * Initialize render targets based on viewport size
   */
  const initializeRenderTargets = useCallback(() => {
    const targets = renderTargetsRef.current;

    // Clear existing targets
    targets.forEach(target => target.dispose());
    targets.clear();

    const width = size.width * (quality.renderScale ?? 1.0);
    const height = size.height * (quality.renderScale ?? 1.0);

    // Main color buffer
    targets.set(
      'color',
      createRenderTarget({
        name: 'color',
        width,
        height,
        format: RGBAFormat,
        type: capabilities?.supportsHDR ? FloatType : UnsignedByteType,
      })
    );

    // Depth buffer
    targets.set(
      'depth',
      createRenderTarget({
        name: 'depth',
        width,
        height,
        format: RGBAFormat,
        type: UnsignedByteType,
      })
    );

    // Post-processing buffer
    if (postprocessing.enabled) {
      targets.set(
        'postprocess',
        createRenderTarget({
          name: 'postprocess',
          width,
          height,
          format: RGBAFormat,
          type: capabilities?.supportsHDR ? FloatType : UnsignedByteType,
        })
      );
    }
  }, [
    capabilities,
    quality.renderScale,
    postprocessing.enabled,
    size.width,
    size.height,
    createRenderTarget,
  ]);

  /**
   * Initialize rendering passes
   */
  const initializePasses = useCallback(() => {
    // Use only the passes provided by MultiStepRenderer - no duplicates!
    const passes = [...customPasses];

    // Sort by priority
    passes.sort((a, b) => a.priority - b.priority);

    // Initialize each pass
    passes.forEach(pass => {
      pass.initialize?.(gl);
    });

    passesRef.current = passes;
  }, [customPasses, gl]);

  /**
   * Handle viewport resize
   */
  const handleResize = useCallback(() => {
    initializeRenderTargets();

    passesRef.current.forEach(pass => {
      pass.resize?.(size.width, size.height);
    });
  }, [initializeRenderTargets, size.width, size.height]);

  // Initialize pipeline
  useEffect(() => {
    if (!capabilities || !gl) return;

    initializeRenderTargets();
    initializePasses();
    setIsInitialized(true);

    // Capture ref values for cleanup
    const renderTargets = renderTargetsRef.current;
    const passes = passesRef.current;

    return () => {
      // Cleanup using captured values
      renderTargets.forEach(target => target.dispose());
      passes.forEach(pass => pass.dispose?.());
    };
  }, [capabilities, gl, initializeRenderTargets, initializePasses]);

  // Handle resize
  useEffect(() => {
    if (isInitialized) {
      handleResize();
    }
  }, [
    size.width,
    size.height,
    quality.renderScale,
    isInitialized,
    handleResize,
  ]);

  /**
   * Execute rendering pipeline
   */
  useFrame(() => {
    if (!isInitialized || !camera || !scene) return;

    const passes = passesRef.current.filter(pass => pass.enabled);
    const targets = renderTargetsRef.current;

    // Debug logging for development
    if (import.meta.env.MODE === 'development') {
      if (passes.length === 0) {
        console.warn(
          'RenderPipeline: No enabled passes found - falling back to direct render'
        );
        // Fallback: render scene directly to screen
        gl.setRenderTarget(null);
        gl.clear(true, true, false);
        gl.render(scene, camera);
        return;
      }
      // Reduced logging: Only log every 60 frames (1 second at 60fps)
      const frameCount = ((window as any).__renderFrameCount as number) ?? 0;
      ((window as any).__renderFrameCount as number) = frameCount + 1;

      if (frameCount % 60 === 0) {
        console.warn(
          `RenderPipeline: Executing ${passes.length} passes: ${passes.map(p => `${p.name}(${p.priority})`).join(', ')}`
        );
      }
    }

    let readBuffer = targets.get('color');
    let writeBuffer = targets.get('postprocess');

    // Execute passes in priority order
    passes.forEach((pass, index) => {
      const isLastPass = index === passes.length - 1;

      // Determine buffers for this pass
      const currentWriteBuffer = isLastPass ? null : writeBuffer; // Render to screen if last pass
      const currentReadBuffer = index === 0 ? undefined : readBuffer;

      // Force the last pass to render to screen
      if (isLastPass) {
        pass.renderToScreen = true;
      }

      // SMART INTEGRATION: Connect PostProcessPass with PostProcessingComposer
      if (
        pass.name === 'postprocess' &&
        postProcessingContext &&
        pass instanceof PostProcessPass
      ) {
        // Connect the PostProcessingComposer's renderFromBuffer to PostProcessPass
        if (postProcessingContext.renderFromBuffer) {
          pass.setPostProcessingCallback(
            postProcessingContext.renderFromBuffer
          );

          const currentFrameCount =
            ((window as any).__renderFrameCount as number) ?? 0;
          if (currentFrameCount % 180 === 0) {
            console.warn(
              'RenderPipeline: ✅ Connected PostProcessPass with PostProcessingComposer'
            );
          }
        }
      } else if (pass.name === 'postprocess') {
        const currentFrameCount =
          ((window as any).__renderFrameCount as number) ?? 0;
        if (currentFrameCount % 180 === 0) {
          console.warn(
            'RenderPipeline: PostProcessingComposer context not available, using fallback'
          );
        }
      }

      // SMART FALLBACK: If this is a GeometryPass and no postprocessing, render to screen
      if (
        pass.name === 'geometry' &&
        !passes.some(p => p.name === 'postprocess')
      ) {
        // If no post-processing pass exists, let geometry render to screen
        pass.renderToScreen = true;
      }

      try {
        pass.render(
          gl,
          scene,
          camera,
          currentWriteBuffer ?? undefined,
          currentReadBuffer
        );
      } catch (error) {
        console.error(`RenderPipeline: Error in ${pass.name} pass:`, error);
      }

      // Swap buffers for next pass
      if (!isLastPass && readBuffer && writeBuffer) {
        [readBuffer, writeBuffer] = [writeBuffer, readBuffer];
      }
    });
  }, 1); // High priority to run before other effects

  // Provide render targets to child components via context
  return (
    <renderPipelineContext.Provider
      value={{
        renderTargets: renderTargetsRef.current,
        passes: passesRef.current,
        isInitialized,
      }}
    >
      {children}
    </renderPipelineContext.Provider>
  );
};

/**
 * Context for accessing render pipeline from child components
 */
interface RenderPipelineContextValue {
  renderTargets: Map<string, WebGLRenderTarget>;
  passes: RenderPass[];
  isInitialized: boolean;
}

const renderPipelineContext =
  React.createContext<RenderPipelineContextValue | null>(null);

export const useRenderPipeline = (): RenderPipelineContextValue => {
  const context = React.useContext(renderPipelineContext);
  if (!context) {
    throw new Error('useRenderPipeline must be used within RenderPipeline');
  }
  return context;
};

export default RenderPipeline;
