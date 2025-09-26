/**
 * Cinematic Camera Component
 * Smooth camera movements with focus tracking and keyframe animation
 */

import { useSpring } from '@react-spring/three';
import { PivotControls } from '@react-three/drei';
import { useFrame, useThree } from '@react-three/fiber';
import React, { useCallback, useEffect, useRef } from 'react';
import * as THREE from 'three';

import { useCameraStore } from '../../../stores/camera-store';
import { useRenderStore } from '../../../stores/render-store';

interface CinematicCameraProps {
  enableFocusTracking?: boolean;
  smoothness?: number;
  lookAhead?: number;
  onUpdate?: (camera: THREE.Camera) => void;
}

interface CinematicCameraRef {
  animateAlongPath: (path: THREE.CatmullRomCurve3, speed: number) => void;
  cinematicShake: (intensity: number, duration: number) => Promise<void>;
}

export const CinematicCamera = React.forwardRef<
  CinematicCameraRef,
  CinematicCameraProps
>(
  (
    { enableFocusTracking = true, smoothness = 0.1, lookAhead = 2, onUpdate },
    ref
  ) => {
    const pivotRef = useRef<THREE.Group | null>(null);
    const pathRef = useRef<THREE.CatmullRomCurve3 | null>(null);
    const progressRef = useRef(0);

    const { camera } = useThree();
    const { updateCamera } = useRenderStore();
    const {
      focusTarget,
      transitionTo,
      isTransitioning,
      completeTransition,
      constraints,
    } = useCameraStore();

    const cinematicConstraints = (constraints.cinematic as {
      smoothness?: number;
      lookAhead?: number;
    }) || {
      smoothness,
      lookAhead,
    };

    // Spring animation for smooth movements
    const [{ position, target }, springApi] = useSpring(() => ({
      position: [camera.position.x, camera.position.y, camera.position.z],
      target: [0, 0, 0],
      config: { tension: 120, friction: 30, mass: 1 },
    }));

    // Focus tracking
    const updateFocusTracking = useCallback(() => {
      if (!enableFocusTracking || !focusTarget) return;

      const currentTarget = new THREE.Vector3(
        ...(target.get() as [number, number, number])
      );
      const distance = currentTarget.distanceTo(focusTarget);

      if (distance > 0.1) {
        void springApi.start({
          target: [focusTarget.x, focusTarget.y, focusTarget.z],
        });
      }
    }, [enableFocusTracking, focusTarget, target, springApi]);

    // Handle smooth transitions
    useEffect(() => {
      if (transitionTo && isTransitioning) {
        void springApi.start({
          position: transitionTo.position,
          target: transitionTo.target,
          config: {
            duration: transitionTo.duration,
          },
          onRest: () => {
            completeTransition();
          },
        });
      }
    }, [transitionTo, isTransitioning, springApi, completeTransition]);

    // Camera path animation
    const animateAlongPath = useCallback(
      (path: THREE.CatmullRomCurve3, speed: number) => {
        pathRef.current = path;
        progressRef.current = 0;

        const animate = () => {
          if (!pathRef.current) return;

          progressRef.current += speed;

          if (progressRef.current >= 1) {
            progressRef.current = 1;
            pathRef.current = null;
            return;
          }

          const point = pathRef.current.getPoint(progressRef.current);

          // Look ahead along the path
          const lookAheadPoint = pathRef.current.getPoint(
            Math.min(
              1,
              progressRef.current +
                (cinematicConstraints.lookAhead ?? lookAhead) * 0.01
            )
          );

          void springApi.start({
            position: [point.x, point.y, point.z],
            target: [lookAheadPoint.x, lookAheadPoint.y, lookAheadPoint.z],
          });

          requestAnimationFrame(animate);
        };

        animate();
      },
      [springApi, cinematicConstraints.lookAhead, lookAhead]
    );

    // Keyframe animation methods
    const createKeyframes = useCallback(
      (
        keyframes: Array<{
          position: [number, number, number];
          target: [number, number, number];
          duration: number;
        }>
      ) => {
        let _currentTime = 0;

        const playKeyframes = async () => {
          for (const keyframe of keyframes) {
            await new Promise<void>(resolve => {
              void springApi.start({
                position: keyframe.position,
                target: keyframe.target,
                config: { duration: keyframe.duration },
                onRest: () => resolve(),
              });
            });
            _currentTime += keyframe.duration;
          }
        };

        return playKeyframes();
      },
      [springApi]
    );

    // Smooth camera shake for cinematic effect
    const cinematicShake = useCallback(
      (intensity: number, duration: number) => {
        const originalPos = position.get() as [number, number, number];
        const shakeframes = [];

        const steps = Math.floor(duration / 50); // 50ms steps
        for (let i = 0; i < steps; i++) {
          const shake = [
            originalPos[0] + (Math.random() - 0.5) * intensity,
            originalPos[1] + (Math.random() - 0.5) * intensity,
            originalPos[2] + (Math.random() - 0.5) * intensity,
          ] as [number, number, number];

          shakeframes.push({
            position: shake,
            target: target.get() as [number, number, number],
            duration: 50,
          });
        }

        // Return to original position
        shakeframes.push({
          position: originalPos,
          target: target.get() as [number, number, number],
          duration: 200,
        });

        return createKeyframes(shakeframes);
      },
      [position, target, createKeyframes]
    );

    // Expose methods through ref
    React.useImperativeHandle(
      ref,
      () => ({
        animateAlongPath,
        cinematicShake,
      }),
      [animateAlongPath, cinematicShake]
    );

    // Update camera state
    const handleUpdate = useCallback(() => {
      const pos = position.get() as [number, number, number];
      const tgt = target.get() as [number, number, number];

      updateCamera({
        position: pos,
        target: tgt,
        isDirty: true,
      });

      onUpdate?.(camera);
    }, [position, target, updateCamera, onUpdate, camera]);

    // Note: useFrame moved to return section to apply camera transforms

    // Update the actual camera's position and target in frame loop
    useFrame(() => {
      updateFocusTracking();
      handleUpdate();

      // Apply position and lookAt to the active camera
      const pos = position.get() as [number, number, number];
      const tgt = target.get() as [number, number, number];

      camera.position.set(pos[0], pos[1], pos[2]);
      camera.lookAt(tgt[0], tgt[1], tgt[2]);
      camera.updateMatrixWorld();
    });

    return (
      <>
        {/* Pivot controls for manual adjustment */}
        {enableFocusTracking && focusTarget && (
          <group position={[focusTarget.x, focusTarget.y, focusTarget.z]}>
            <PivotControls
              ref={pivotRef}
              scale={2}
              lineWidth={2}
              fixed
              depthTest={false}
            />
          </group>
        )}
      </>
    );
  }
);

CinematicCamera.displayName = 'CinematicCamera';

export type { CinematicCameraRef };
export default CinematicCamera;
