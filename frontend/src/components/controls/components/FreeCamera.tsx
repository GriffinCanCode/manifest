/**
 * Free Camera Component
 * FlyControls for unrestricted 3D navigation
 */

import { FlyControls } from '@react-three/drei';
import { useFrame, useThree } from '@react-three/fiber';
import React, { useCallback, useEffect, useMemo, useRef } from 'react';
import * as THREE from 'three';

import { useCameraStore } from '../../../stores/camera-store';
import { useRenderStore } from '../../../stores/render-store';

interface FreeCameraProps {
  movementSpeed?: number;
  rollSpeed?: number;
  dragToLook?: boolean;
  autoForward?: boolean;
  onUpdate?: (camera: THREE.Camera) => void;
}

export const FreeCamera: React.FC<FreeCameraProps> = ({
  movementSpeed = 10,
  rollSpeed = 0.3,
  dragToLook = true,
  autoForward = false,
  onUpdate,
}) => {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const flyRef = useRef<any>(null); // FlyControls from drei has complex typing, using any for ref
  const { camera } = useThree();
  const { updateCamera } = useRenderStore();
  const { constraints } = useCameraStore();

  // Memoize constraints to prevent unnecessary re-renders
  const freeCameraConstraints = useMemo(
    () => constraints.free ?? {},
    [constraints.free]
  );

  // Update camera state
  const handleUpdate = useCallback(() => {
    if (camera) {
      const target = new THREE.Vector3();
      camera.getWorldDirection(target);
      target.add(camera.position);

      updateCamera({
        position: [camera.position.x, camera.position.y, camera.position.z],
        target: [target.x, target.y, target.z],
        isDirty: true,
      });

      onUpdate?.(camera);
    }
  }, [camera, updateCamera, onUpdate]);

  // Apply constraints from store
  useEffect(() => {
    if (flyRef.current && freeCameraConstraints) {
      // Type assertion needed as @react-three/drei FlyControls doesn't expose properties in TS
      // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
      const controls: {
        movementSpeed?: number;
        rollSpeed?: number;
      } = flyRef.current;

      if (
        typeof freeCameraConstraints.movementSpeed === 'number' &&
        'movementSpeed' in controls
      ) {
        controls.movementSpeed = freeCameraConstraints.movementSpeed;
      }
      if (
        typeof freeCameraConstraints.rollSpeed === 'number' &&
        'rollSpeed' in controls
      ) {
        controls.rollSpeed = freeCameraConstraints.rollSpeed;
      }
    }
  }, [freeCameraConstraints]);

  // Frame update
  useFrame((_, delta) => {
    if (flyRef.current) {
      // Type assertion needed as @react-three/drei FlyControls doesn't expose update method in TS
      // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
      const controls: { update: (delta: number) => void } = flyRef.current;
      if ('update' in controls && typeof controls.update === 'function') {
        controls.update(delta);
      }
      handleUpdate();
    }
  });

  return (
    <FlyControls
      ref={flyRef}
      args={[camera]}
      movementSpeed={
        (typeof freeCameraConstraints.movementSpeed === 'number'
          ? freeCameraConstraints.movementSpeed
          : null) ?? movementSpeed
      }
      rollSpeed={
        (typeof freeCameraConstraints.rollSpeed === 'number'
          ? freeCameraConstraints.rollSpeed
          : null) ?? rollSpeed
      }
      dragToLook={dragToLook}
      autoForward={autoForward}
    />
  );
};

export default FreeCamera;
