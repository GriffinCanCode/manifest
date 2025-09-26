/**
 * Camera Effects Hook
 * Manages camera shake, focus tracking, and smooth transitions
 */

import { useFrame } from '@react-three/fiber';
import { useCallback, useRef } from 'react';
import * as THREE from 'three';

import { useCameraStore } from '../../../stores/camera-store';
import { useRenderStore } from '../../../stores/render-store';

interface CameraEffectsOptions {
  enableShake?: boolean;
  enableFocus?: boolean;
  enableTransitions?: boolean;
}

export const useCameraEffects = (options: CameraEffectsOptions = {}) => {
  const {
    enableShake = true,
    enableFocus = true,
    enableTransitions = true,
  } = options;

  const { updateCamera } = useRenderStore();
  const {
    shake,
    setShake,
    focusTarget,
    transitionTo,
    completeTransition,
    isTransitioning,
  } = useCameraStore();

  const shakeOffset = useRef(new THREE.Vector3());
  const originalPosition = useRef(new THREE.Vector3());

  // Camera shake effect
  const updateShake = useCallback(
    (camera: THREE.Camera, delta: number) => {
      if (!enableShake || shake.intensity <= 0) {
        if (shakeOffset.current.length() > 0) {
          // Reset position if shake stopped
          camera.position.sub(shakeOffset.current);
          shakeOffset.current.set(0, 0, 0);
        }
        return;
      }

      // Store original position on first shake
      if (shakeOffset.current.length() === 0) {
        originalPosition.current.copy(camera.position);
      } else {
        // Remove previous shake offset
        camera.position.sub(shakeOffset.current);
      }

      // Generate new shake offset
      const { intensity } = shake;
      shakeOffset.current.set(
        (Math.random() - 0.5) * intensity,
        (Math.random() - 0.5) * intensity,
        (Math.random() - 0.5) * intensity
      );

      // Apply new shake offset
      camera.position.add(shakeOffset.current);

      // Decay shake over time
      const newIntensity = shake.intensity * Math.pow(shake.decay, delta * 60);
      if (newIntensity < 0.001) {
        setShake({ intensity: 0, duration: 0, decay: shake.decay });
      } else {
        setShake({ ...shake, intensity: newIntensity });
      }
    },
    [enableShake, shake, setShake]
  );

  // Focus tracking effect
  const updateFocus = useCallback(
    (camera: THREE.Camera) => {
      if (!enableFocus || !focusTarget) return;

      const direction = new THREE.Vector3()
        .subVectors(camera.position, focusTarget)
        .normalize();

      camera.lookAt(focusTarget);

      updateCamera({
        target: [focusTarget.x, focusTarget.y, focusTarget.z],
        isDirty: true,
      });
    },
    [enableFocus, focusTarget, updateCamera]
  );

  // Smooth transitions
  const updateTransition = useCallback(
    (camera: THREE.Camera, delta: number) => {
      if (!enableTransitions || !transitionTo || !isTransitioning) return;

      // This would be handled by react-spring in the actual implementation
      // Here we just complete the transition
      completeTransition();
    },
    [enableTransitions, transitionTo, isTransitioning, completeTransition]
  );

  // Main update loop
  useFrame((state, delta) => {
    const { camera } = state;

    updateShake(camera, delta);
    updateFocus(camera);
    updateTransition(camera, delta);
  });

  // Trigger shake effect
  const triggerShake = useCallback(
    (intensity = 1, duration = 1000, decay = 0.95) => {
      if (enableShake) {
        setShake({ intensity, duration, decay });
      }
    },
    [enableShake, setShake]
  );

  // Stop shake effect
  const stopShake = useCallback(() => {
    setShake({ intensity: 0, duration: 0, decay: 1 });
  }, [setShake]);

  return {
    triggerShake,
    stopShake,
    shake,
    isShaking: shake.intensity > 0,
    focusTarget,
    isTransitioning,
  };
};
