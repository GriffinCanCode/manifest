/**
 * Multi-Step Renderer - Main orchestrator for the complete rendering system
 * Integrates RenderPipeline with existing rendering components
 */

import React from 'react';

import { useRenderStore } from '../../../../stores/render-store';
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
  enableDebug = process.env.NODE_ENV === 'development',
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

  // Registry-based pass creation for extensibility
  const registryPasses = React.useMemo(() => {
    if (!capabilities) return [];

    return passRegistry.createOrderedPasses().filter(pass => {
      // Filter passes based on capabilities
      if (pass.name === 'shadow' && !capabilities.supportsShadows) {
        return false;
      }
      return true;
    });
  }, [capabilities]);

  const allPasses = [...registryPasses, ...customPasses];

  return (
    <RenderPipeline
      passes={allPasses}
      enableMultiSampling={capabilities?.supportsFloatTextures}
    >
      <PostProcessingComposer
        enabled
        enableTAA={enableTAA && capabilities?.supportsHDR}
        enableSelectiveBloom={capabilities?.gpuTier === 'high'}
      >
        {children}
      </PostProcessingComposer>
    </RenderPipeline>
  );
};

export default MultiStepRenderer;
