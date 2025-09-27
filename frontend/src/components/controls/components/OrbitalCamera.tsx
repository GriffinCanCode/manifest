/**
 * Orbital Camera Component
 * Three.js OrbitControls with enhanced features and constraints
 */

import { OrbitControls } from '@react-three/drei';
import { useFrame, useThree } from '@react-three/fiber';
import React, { useCallback, useEffect, useMemo, useRef } from 'react';
import * as THREE from 'three';
import type { OrbitControls as OrbitControlsImpl } from 'three-stdlib';

import { useCameraStore } from '../../../stores/camera-store';
import { useRenderStore } from '../../../stores/render-store';
import type { CameraConstraints } from '../types';
import { CameraUtils } from '../utils/CameraUtils';

interface OrbitalCameraProps {
  constraints?: Partial<CameraConstraints>;
  enableDamping?: boolean;
  dampingFactor?: number;
  enableAutoRotate?: boolean;
  autoRotateSpeed?: number;
  onUpdate?: (camera: THREE.Camera) => void;
}

const DEFAULT_CONSTRAINTS: CameraConstraints = {
  minDistance: 8,
  maxDistance: 1000,
  minPolarAngle: Math.PI / 6,
  maxPolarAngle: Math.PI / 2.2,
  minAzimuthAngle: -Math.PI / 2,
  maxAzimuthAngle: Math.PI / 2,
  enablePan: true,
  enableZoom: true,
  enableRotate: true,
};

export const OrbitalCamera: React.FC<OrbitalCameraProps> = ({
  constraints = {},
  enableDamping = true,
  dampingFactor = 0.05,
  enableAutoRotate = false,
  autoRotateSpeed = 2,
  onUpdate,
}) => {
  const orbitRef = useRef<OrbitControlsImpl>(null);
  const { camera } = useThree();
  const { updateCamera } = useRenderStore();
  const { constraints: storeConstraints } = useCameraStore();

  const finalConstraints = useMemo(
    () => ({
      ...DEFAULT_CONSTRAINTS,
      ...storeConstraints.orbital,
      ...constraints,
    }),
    [storeConstraints.orbital, constraints]
  );

  // Update camera state in render store
  const handleChange = useCallback(() => {
    if (orbitRef.current && camera) {
      const controls = orbitRef.current;

      updateCamera({
        position: [camera.position.x, camera.position.y, camera.position.z],
        target: [controls.target.x, controls.target.y, controls.target.z],
        zoom: camera instanceof THREE.PerspectiveCamera ? camera.zoom : 1,
        isDirty: true,
      });

      onUpdate?.(camera);
    }
  }, [camera, updateCamera, onUpdate]);

  // Apply constraints when they change
  useEffect(() => {
    if (orbitRef.current) {
      const controls = orbitRef.current;

      controls.minDistance = finalConstraints.minDistance;
      controls.maxDistance = finalConstraints.maxDistance;
      controls.minPolarAngle = finalConstraints.minPolarAngle;
      controls.maxPolarAngle = finalConstraints.maxPolarAngle;

      if (finalConstraints.minAzimuthAngle !== undefined) {
        controls.minAzimuthAngle = finalConstraints.minAzimuthAngle;
      }
      if (finalConstraints.maxAzimuthAngle !== undefined) {
        controls.maxAzimuthAngle = finalConstraints.maxAzimuthAngle;
      }

      controls.enablePan = finalConstraints.enablePan;
      controls.enableZoom = finalConstraints.enableZoom;
      controls.enableRotate = finalConstraints.enableRotate;
    }
  }, [finalConstraints]);

  // Handle bounds constraints
  useEffect(() => {
    if (orbitRef.current && finalConstraints.bounds && camera) {
      const controls = orbitRef.current;

      // Create a custom constraint function
      const originalUpdate = controls.update.bind(controls);
      controls.update = () => {
        originalUpdate();

        // Apply bounds constraint
        if (finalConstraints.bounds) {
          const constrainedPos = CameraUtils.applyConstraints(
            camera.position,
            controls.target,
            finalConstraints as CameraConstraints
          );
          camera.position.copy(constrainedPos);
        }
      };
    }
  }, [finalConstraints, camera]);

  // Frame update for smooth controls
  useFrame(() => {
    if (orbitRef.current) {
      orbitRef.current.update();
    }
  });

  // Set initial camera position and target for hex world viewing
  useEffect(() => {
    if (camera && orbitRef.current) {
      // Position camera to look down at hex grid from an angle
      camera.position.set(0, 20, 20);
      orbitRef.current.target.set(0, 0, 0);
      orbitRef.current.update();
      handleChange();
    }
  }, [camera, handleChange]);

  return (
    <OrbitControls
      ref={orbitRef}
      args={[camera]}
      // Basic controls
      enablePan={finalConstraints.enablePan}
      enableZoom={finalConstraints.enableZoom}
      enableRotate={finalConstraints.enableRotate}
      // Distance constraints
      minDistance={finalConstraints.minDistance}
      maxDistance={finalConstraints.maxDistance}
      // Angle constraints
      minPolarAngle={finalConstraints.minPolarAngle}
      maxPolarAngle={finalConstraints.maxPolarAngle}
      minAzimuthAngle={finalConstraints.minAzimuthAngle}
      maxAzimuthAngle={finalConstraints.maxAzimuthAngle}
      // Damping
      enableDamping={enableDamping}
      dampingFactor={dampingFactor}
      // Auto rotation
      autoRotate={enableAutoRotate}
      autoRotateSpeed={autoRotateSpeed}
      // Sensitivity
      rotateSpeed={0.5}
      panSpeed={1}
      zoomSpeed={1}
      // Touch controls
      touches={{ ONE: THREE.TOUCH.ROTATE, TWO: THREE.TOUCH.DOLLY_PAN }}
      // Mouse controls
      mouseButtons={{
        LEFT: THREE.MOUSE.ROTATE,
        MIDDLE: THREE.MOUSE.DOLLY,
        RIGHT: THREE.MOUSE.PAN,
      }}
      // Events
      onChange={handleChange}
    />
  );
};

export default OrbitalCamera;
