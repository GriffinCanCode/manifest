/**
 * Cascaded Shadow Maps (CSM) Renderer
 * High-quality real-time shadow system for large-scale hex worlds
 */

import { useFrame, useThree } from '@react-three/fiber';
import React, { useEffect, useMemo, useRef } from 'react';
import {
  Box3,
  DirectionalLight,
  Frustum,
  Matrix4,
  Vector3,
  type Camera,
  type PerspectiveCamera,
  type WebGLRenderTarget,
} from 'three';
import CSM from 'three-csm';

import { useRenderStore } from '../../../../stores/render-store';

interface ShadowCascadeProps {
  lightDirection?: Vector3;
  shadowMapSize?: number;
  cascades?: number;
  maxFar?: number;
  shadowBias?: number;
  enabled?: boolean;
  children?: React.ReactNode;
}

interface CascadeData {
  camera: Camera;
  shadowMap: WebGLRenderTarget | null;
  frustum: Frustum;
  bounds: Box3;
  distance: number;
}

/**
 * Cascaded Shadow Maps implementation using three-csm
 * Provides high-quality shadows across large viewing distances
 */
export const ShadowCascadeRenderer: React.FC<ShadowCascadeProps> = ({
  lightDirection = new Vector3(-0.5, -1, -0.3).normalize(),
  shadowMapSize = 2048,
  cascades = 3,
  maxFar = 500,
  shadowBias = -0.0001,
  enabled = true,
  children,
}) => {
  const { scene, camera: mainCamera } = useThree();
  const { quality, capabilities, debug, isInitialized } = useRenderStore();

  const csmRef = useRef<CSM | null>(null);
  const lightRef = useRef<DirectionalLight | null>(null);
  const cascadeDataRef = useRef<CascadeData[]>([]);

  // Adaptive shadow quality based on performance
  const shadowQuality = useMemo(() => {
    const baseSize = shadowMapSize;
    switch (quality.level) {
      case 'low':
        return Math.max(512, baseSize * 0.25);
      case 'medium':
        return Math.max(1024, baseSize * 0.5);
      case 'high':
        return baseSize;
      case 'ultra':
        return Math.min(4096, baseSize * 1.5);
      default:
        return baseSize;
    }
  }, [quality.level, shadowMapSize]);

  // Cascade distances based on viewing distance
  const cascadeDistances = useMemo(() => {
    const far = maxFar;
    const near = (mainCamera as PerspectiveCamera).near || 0.1;

    switch (cascades) {
      case 2:
        return [near, far * 0.15, far];
      case 3:
        return [near, far * 0.05, far * 0.25, far];
      case 4:
        return [near, far * 0.02, far * 0.1, far * 0.4, far];
      default:
        return [near, far * 0.05, far * 0.25, far];
    }
  }, [cascades, maxFar, mainCamera]);

  // Create directional light for shadows
  useEffect(() => {
    if (!isInitialized || !capabilities?.supportsShadows || !enabled) return;

    // Create main directional light
    const light = new DirectionalLight(0xffffff, 1.0);
    light.position.copy(lightDirection).multiplyScalar(-100);
    light.castShadow = true;
    light.shadow.bias = shadowBias;
    light.shadow.normalBias = 0.02;

    scene.add(light);
    lightRef.current = light;

    return () => {
      if (lightRef.current) {
        scene.remove(lightRef.current);
        lightRef.current = null;
      }
    };
  }, [
    isInitialized,
    capabilities?.supportsShadows,
    enabled,
    lightDirection,
    shadowBias,
    scene,
  ]);

  // Initialize CSM
  useEffect(() => {
    if (
      !lightRef.current ||
      !isInitialized ||
      !capabilities?.supportsShadows ||
      !enabled
    ) {
      return;
    }

    try {
      const csm = new CSM({
        maxFar,
        cascades,
        shadowMapSize: shadowQuality,
        lightDirection: lightDirection.clone(),
        camera: mainCamera,
        parent: scene,
        shadowBias,
        lightMargin: 100,
      });

      // Configure cascade-specific properties
      csm.lights.forEach((light, index) => {
        light.shadow.bias = shadowBias * (index + 1);
        light.shadow.normalBias = 0.02 * (index + 1);
        light.shadow.camera.near = cascadeDistances[index];
        light.shadow.camera.far = cascadeDistances[index + 1];
      });

      csmRef.current = csm;

      // Store cascade data for debugging
      cascadeDataRef.current = csm.lights.map((light, index) => ({
        camera: light.shadow.camera,
        shadowMap: light.shadow.map,
        frustum: new Frustum(),
        bounds: new Box3(),
        distance: cascadeDistances[index + 1] - cascadeDistances[index],
      }));
    } catch (error) {
      console.warn('Failed to initialize CSM:', error);
    }

    return () => {
      if (csmRef.current) {
        csmRef.current.dispose();
        csmRef.current = null;
      }
      cascadeDataRef.current = [];
    };
  }, [
    isInitialized,
    capabilities?.supportsShadows,
    enabled,
    cascades,
    shadowQuality,
    maxFar,
    lightDirection,
    shadowBias,
    mainCamera,
    scene,
    cascadeDistances,
  ]);

  // Update CSM every frame
  useFrame(() => {
    if (!csmRef.current || !enabled) return;

    try {
      // Update CSM
      csmRef.current.update();

      // Update cascade debug data
      if (debug.showLOD) {
        csmRef.current.lights.forEach((light, index) => {
          if (cascadeDataRef.current[index]) {
            const cascade = cascadeDataRef.current[index];
            cascade.frustum.setFromProjectionMatrix(
              new Matrix4().multiplyMatrices(
                light.shadow.camera.projectionMatrix,
                light.shadow.camera.matrixWorldInverse
              )
            );
            cascade.bounds.setFromCenterAndSize(
              light.position,
              new Vector3(
                light.shadow.camera.right - light.shadow.camera.left,
                light.shadow.camera.top - light.shadow.camera.bottom,
                light.shadow.camera.far - light.shadow.camera.near
              )
            );
          }
        });
      }
    } catch (error) {
      console.warn('CSM update failed:', error);
    }
  });

  // Update shader uniforms when quality changes
  useEffect(() => {
    if (!csmRef.current) return;

    // Update quality-dependent settings
    csmRef.current.lights.forEach(light => {
      if (quality.level === 'low') {
        light.shadow.bias = shadowBias * 2;
        light.shadow.normalBias = 0.04;
      } else {
        light.shadow.bias = shadowBias;
        light.shadow.normalBias = 0.02;
      }
    });
  }, [quality.level, shadowBias]);

  // Return null if shadows not supported or disabled
  if (
    !isInitialized ||
    !capabilities?.supportsShadows ||
    !enabled ||
    !quality.shadows
  ) {
    return children ? (children as React.ReactElement) : null;
  }

  return (
    <group>
      {children}
      {debug.showLOD && csmRef.current && (
        <CascadeDebugHelper
          cascades={cascadeDataRef.current}
          csm={csmRef.current}
        />
      )}
    </group>
  );
};

/**
 * Debug helper component for visualizing shadow cascades
 */
interface CascadeDebugHelperProps {
  cascades: CascadeData[];
  csm: CSM;
}

const CascadeDebugHelper: React.FC<CascadeDebugHelperProps> = ({
  cascades,
  csm,
}) => {
  const cascadeColors = useMemo(
    () => [
      0xff0000, // Red for first cascade
      0x00ff00, // Green for second cascade
      0x0000ff, // Blue for third cascade
      0xffff00, // Yellow for fourth cascade
    ],
    []
  );

  return (
    <group>
      {csm.lights.map((light, index) => (
        <group key={index}>
          {/* Frustum helper */}
          <primitive object={light.shadow.camera} position={light.position} />

          {/* Cascade bounds visualization */}
          {cascades[index] && (
            <mesh position={light.position}>
              <boxGeometry
                args={[
                  light.shadow.camera.right - light.shadow.camera.left,
                  light.shadow.camera.top - light.shadow.camera.bottom,
                  light.shadow.camera.far - light.shadow.camera.near,
                ]}
              />
              <meshBasicMaterial
                color={cascadeColors[index] || 0xffffff}
                wireframe
                transparent
                opacity={0.3}
              />
            </mesh>
          )}
        </group>
      ))}
    </group>
  );
};

export default ShadowCascadeRenderer;
