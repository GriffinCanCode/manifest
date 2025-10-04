/**
 * Frame Updater Component for Rendering System
 * Must be used inside React Three Fiber Canvas context
 */

import { useFrame, useThree } from '@react-three/fiber';
import { Vector3 } from 'three';

import { uniformService } from '../../../services/uniforms';

import { useRendering } from './rendering-hooks';

interface RenderingFrameUpdaterProps {
  // This component renders nothing but handles frame updates
}

/**
 * Component that handles R3F frame updates for the rendering system
 * Must be rendered inside Canvas component
 */
export const RenderingFrameUpdater: React.FC<
  RenderingFrameUpdaterProps
> = () => {
  const { camera } = useThree();
  const { isReady } = useRendering();

  // Unified frame updates - SINGLE SOURCE OF TRUTH for uniform updates
  useFrame((_state, delta) => {
    if (!isReady) return;

    const cameraPosition = new Vector3().setFromMatrixPosition(
      camera.matrixWorld
    );

    // Update all uniforms from single location
    uniformService.updateFrame(delta, cameraPosition);
  });

  return null; // This component only handles frame updates
};

export default RenderingFrameUpdater;
