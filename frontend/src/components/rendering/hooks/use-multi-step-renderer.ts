/**
 * Hook for accessing multi-step renderer functionality
 */

import { useRenderStore } from '../../../stores/render-store';
import type { RenderPass } from '../core/RenderPass';
import { passRegistry } from '../passes';

export const useMultiStepRenderer = () => {
  const { capabilities, quality, debug, devMode } = useRenderStore();

  return {
    // Pass registration utilities
    registerPass: (
      name: string,
      passFactory: () => RenderPass,
      priority = 0
    ) => {
      passRegistry.register(name, {
        type: 'custom',
        factory: passFactory,
        priority,
      });
    },

    unregisterPass: (name: string) => {
      passRegistry.unregister(name);
    },

    // Quality utilities
    shouldEnableFeature: (feature: string) => {
      switch (feature) {
        case 'taa':
          return capabilities?.supportsHDR && quality.level !== 'low';
        case 'ssao':
          return capabilities?.supportsFloatTextures && quality.level !== 'low';
        case 'shadows':
          return capabilities?.supportsShadows;
        case 'bloom':
          return capabilities?.supportsHDR;
        default:
          return true;
      }
    },

    // Debug utilities
    isDebugEnabled: devMode,
    debugOptions: debug,
  };
};
