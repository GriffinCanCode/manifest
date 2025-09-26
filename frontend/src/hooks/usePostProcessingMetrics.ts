/**
 * Post-Processing Performance Monitoring Hook
 */

import { useFrame } from '@react-three/fiber';

import { useRenderStore } from '../stores/render-store';

/**
 * Performance monitoring hook for post-processing effects
 * Tracks render calls, triangles, and frame timing for optimization
 */
export const usePostProcessingMetrics = (): void => {
  const { updateMetrics } = useRenderStore();

  useFrame((_state, delta) => {
    // Monitor post-processing performance impact
    const renderer = _state.gl;
    const { info } = renderer;

    updateMetrics({
      drawCalls: info.render.calls,
      triangles: info.render.triangles,
      frameTime: delta * 1000,
    });
  });
};
