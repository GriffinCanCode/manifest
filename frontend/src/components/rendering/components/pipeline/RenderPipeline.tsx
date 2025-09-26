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
import {
  GeometryPass,
  PostProcessPass,
  ShadowPass,
  type RenderPass,
} from '../../core/RenderPass';

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
    const passes: RenderPass[] = [
      // Shadow pass (if shadows enabled)
      ...(capabilities?.supportsShadows ? [new ShadowPass()] : []),

      // Main geometry pass
      new GeometryPass({
        renderTarget: renderTargetsRef.current.get('color'),
      }),

      // Post-processing pass (if enabled)
      ...(postprocessing.enabled ? [new PostProcessPass()] : []),

      // Custom passes
      ...customPasses,
    ];

    // Sort by priority
    passes.sort((a, b) => a.priority - b.priority);

    // Initialize each pass
    passes.forEach(pass => {
      pass.initialize?.(gl);
    });

    passesRef.current = passes;
  }, [capabilities?.supportsShadows, postprocessing.enabled, customPasses, gl]);

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
  }, [
    capabilities,
    gl,
    postprocessing.enabled,
    initializeRenderTargets,
    initializePasses,
  ]);

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

    let readBuffer = targets.get('color');
    let writeBuffer = targets.get('postprocess');

    // Execute passes in priority order
    passes.forEach((pass, index) => {
      const isLastPass = index === passes.length - 1;

      // Determine buffers for this pass
      const currentWriteBuffer = isLastPass ? undefined : writeBuffer;
      const currentReadBuffer = index === 0 ? undefined : readBuffer;

      pass.render(gl, scene, camera, currentWriteBuffer, currentReadBuffer);

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
