/**
 * Utility functions for cinematic camera operations
 */

import * as THREE from 'three';

export const cinematicUtils = {
  createSmoothPath: (points: THREE.Vector3[]) => {
    return new THREE.CatmullRomCurve3(points);
  },

  createCircularPath: (
    center: THREE.Vector3,
    radius: number,
    segments = 64
  ) => {
    const points: THREE.Vector3[] = [];
    for (let i = 0; i <= segments; i++) {
      const angle = (i / segments) * Math.PI * 2;
      points.push(
        new THREE.Vector3(
          center.x + Math.cos(angle) * radius,
          center.y,
          center.z + Math.sin(angle) * radius
        )
      );
    }
    return new THREE.CatmullRomCurve3(points);
  },
};

export default cinematicUtils;
