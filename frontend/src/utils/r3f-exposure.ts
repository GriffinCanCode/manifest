/**
 * React Three Fiber Exposure System
 * Properly exposes R3F scene, camera, and renderer for diagnostics
 */

import { useFrame, useThree } from '@react-three/fiber';
import { useEffect } from 'react';

/**
 * Hook to expose R3F objects to window for diagnostics
 */
export function useExposeR3F() {
  const { scene, camera, gl: renderer } = useThree();

  useEffect(() => {
    if (typeof window !== 'undefined') {
      const win = window as any;
      win.__scene = scene;
      win.__camera = camera;
      win.__renderer = renderer;
      win.__r3f = { scene, camera, renderer };

      console.log('📷 R3F EXPOSED: Scene, camera, and renderer available');
      console.log('   • window.__scene -', scene.type);
      console.log('   • window.__camera -', camera.type);
      console.log('   • window.__renderer -', renderer.info.render);
    }
  }, [scene, camera, renderer]);

  // Also expose on every frame for live updates
  useFrame(() => {
    if (typeof window !== 'undefined') {
      const win = window as any;
      if (!win.__cameraPosition) {
        win.__cameraPosition = camera.position;
        win.__cameraRotation = camera.rotation;
      }
    }
  });

  return { scene, camera, renderer };
}

/**
 * Component that just exposes R3F objects - add to your scene
 */
export function R3FExposer() {
  useExposeR3F();
  return null;
}
