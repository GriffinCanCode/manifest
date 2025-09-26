/**
 * Multi-Step Renderer - Main orchestrator for the complete rendering system
 * Integrates RenderPipeline with existing rendering components
 */

import React from 'react';

import { useRenderStore } from '../../../../stores/render-store';
import { GeometryPass, PostProcessPass } from '../../core/RenderPass';
import { passRegistry } from '../../passes';
import { DebugPass } from '../../passes/DebugPass';
import { SelectionPass } from '../../passes/SelectionPass';
import PostProcessingComposer from '../effects/PostProcessingComposer';

import { RenderPipeline } from './RenderPipeline';

interface MultiStepRendererProps {
  children: React.ReactNode;
  enableSelection?: boolean;
  enableDebug?: boolean;
  enableTAA?: boolean;
}

/**
 * Complete multi-step rendering system with extensible pass management
 */
export const MultiStepRenderer: React.FC<MultiStepRendererProps> = ({
  children,
  enableSelection = true,
  enableDebug = import.meta.env?.MODE === 'development',
  enableTAA = true,
}) => {
  const { debug, capabilities, devMode } = useRenderStore();

  // Create custom passes based on configuration
  const customPasses = React.useMemo(() => {
    const passes = [];

    // Add selection pass for object picking
    if (enableSelection) {
      passes.push(new SelectionPass());
    }

    // Add debug pass for development visualization
    if (enableDebug && devMode) {
      passes.push(
        new DebugPass({
          showWireframes: debug.showWireframe,
          showBoundingBoxes: debug.showBounds,
        })
      );
    }

    return passes;
  }, [enableSelection, enableDebug, debug, devMode]);

  // SMART FIX: Only add essential passes, not all registry passes
  const essentialPasses = React.useMemo(() => {
    if (!capabilities) return [];

    const passes = [];

    // Always add geometry pass (essential for rendering)
    const geometryReg = passRegistry.get('geometry');
    passes.push(geometryReg ? geometryReg.factory() : new GeometryPass());

    // Only add postprocess pass (for our smart integration)
    const postprocessReg = passRegistry.get('postprocess');
    passes.push(
      postprocessReg ? postprocessReg.factory() : new PostProcessPass()
    );

    return passes;
  }, [capabilities]);

  const allPasses = [...essentialPasses, ...customPasses];

  return (
    <RenderPipeline
      passes={allPasses}
      enableMultiSampling={capabilities?.supportsFloatTextures}
    >
      <PostProcessingComposer
        enabled // Re-enable post-processing
        enableTAA={enableTAA && capabilities?.supportsHDR}
        enableSelectiveBloom={capabilities?.gpuTier === 'high'}
      >
        {children}
      </PostProcessingComposer>
    </RenderPipeline>
  );
};

export default MultiStepRenderer;
