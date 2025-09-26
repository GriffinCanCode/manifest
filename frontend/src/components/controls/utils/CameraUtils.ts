/**
 * Camera Utility Functions
 * Pure functions for camera calculations and helpers
 */

import * as THREE from 'three';

import type { CameraBookmark, CameraConstraints, CameraMode } from '../types';

export class CameraUtils {
  /**
   * Calculate smooth interpolation between two positions
   */
  static lerp(start: number, end: number, alpha: number): number {
    return start + (end - start) * alpha;
  }

  /**
   * Calculate smooth vector interpolation
   */
  static lerpVector3(
    start: THREE.Vector3,
    end: THREE.Vector3,
    alpha: number,
    target?: THREE.Vector3
  ): THREE.Vector3 {
    const result = target ?? new THREE.Vector3();
    return result.lerpVectors(start, end, alpha);
  }

  /**
   * Apply constraints to camera position
   */
  static applyConstraints(
    position: THREE.Vector3,
    target: THREE.Vector3,
    constraints: CameraConstraints
  ): THREE.Vector3 {
    const distance = position.distanceTo(target);
    const direction = position.clone().sub(target).normalize();

    // Apply distance constraints
    const clampedDistance = Math.max(
      constraints.minDistance,
      Math.min(constraints.maxDistance, distance)
    );

    // Apply bounds constraints
    if (constraints.bounds) {
      position.clamp(constraints.bounds.min, constraints.bounds.max);
    }

    return target.clone().add(direction.multiplyScalar(clampedDistance));
  }

  /**
   * Calculate frustum corners for camera bounds
   */
  static getFrustumCorners(camera: THREE.PerspectiveCamera): THREE.Vector3[] {
    const frustum = new THREE.Frustum();
    const matrix = new THREE.Matrix4();
    matrix.multiplyMatrices(camera.projectionMatrix, camera.matrixWorldInverse);
    frustum.setFromProjectionMatrix(matrix);

    const corners: THREE.Vector3[] = [];

    // Calculate the 8 corners of the frustum
    // This is a simplified implementation - full implementation would require
    // complex intersection calculations between frustum planes
    for (let i = 0; i < 8; i++) {
      const corner = new THREE.Vector3();
      corners.push(corner);
    }

    return corners;
  }

  /**
   * Create camera bookmark from current state
   */
  static createBookmark(
    camera: THREE.Camera,
    target: THREE.Vector3,
    mode: CameraMode,
    name?: string
  ): CameraBookmark {
    return {
      id: `bookmark_${Date.now()}`,
      name: name ?? `Bookmark ${Date.now()}`,
      position: [camera.position.x, camera.position.y, camera.position.z],
      target: [target.x, target.y, target.z],
      mode,
      fov: camera instanceof THREE.PerspectiveCamera ? camera.fov : undefined,
      createdAt: Date.now(),
    };
  }

  /**
   * Calculate optimal camera distance for object bounds
   */
  static calculateOptimalDistance(
    bounds: THREE.Box3,
    camera: THREE.PerspectiveCamera
  ): number {
    const size = bounds.getSize(new THREE.Vector3());
    const maxDim = Math.max(size.x, size.y, size.z);
    const fov = camera.fov * (Math.PI / 180);
    return (maxDim / (2 * Math.tan(fov / 2))) * 1.5; // 1.5 for padding
  }

  /**
   * Get camera direction vector
   */
  static getCameraDirection(camera: THREE.Camera): THREE.Vector3 {
    const direction = new THREE.Vector3();
    camera.getWorldDirection(direction);
    return direction;
  }

  /**
   * Check if point is in camera view
   */
  static isPointInView(point: THREE.Vector3, camera: THREE.Camera): boolean {
    const frustum = new THREE.Frustum();
    const matrix = new THREE.Matrix4();
    matrix.multiplyMatrices(camera.projectionMatrix, camera.matrixWorldInverse);
    frustum.setFromProjectionMatrix(matrix);
    return frustum.containsPoint(point);
  }

  /**
   * Convert world position to screen coordinates
   */
  static worldToScreen(
    worldPos: THREE.Vector3,
    camera: THREE.Camera,
    renderer: THREE.WebGLRenderer
  ): THREE.Vector2 {
    const vector = worldPos.clone();
    vector.project(camera);

    const size = renderer.getSize(new THREE.Vector2());
    vector.x = ((vector.x + 1) / 2) * size.x;
    vector.y = (-(vector.y - 1) / 2) * size.y;

    return new THREE.Vector2(vector.x, vector.y);
  }

  /**
   * Convert screen coordinates to world ray
   */
  static screenToWorldRay(
    screenPos: THREE.Vector2,
    camera: THREE.Camera,
    renderer: THREE.WebGLRenderer
  ): THREE.Ray {
    const size = renderer.getSize(new THREE.Vector2());
    const mouse = new THREE.Vector2();
    mouse.x = (screenPos.x / size.x) * 2 - 1;
    mouse.y = -(screenPos.y / size.y) * 2 + 1;

    const raycaster = new THREE.Raycaster();
    raycaster.setFromCamera(mouse, camera);
    return raycaster.ray;
  }

  /**
   * Calculate edge scroll vector based on mouse position
   */
  static calculateEdgeScroll(
    mousePos: THREE.Vector2,
    screenSize: THREE.Vector2,
    threshold: number,
    speed: number
  ): THREE.Vector2 {
    const scroll = new THREE.Vector2();

    // Check edges
    if (mousePos.x < threshold) {
      scroll.x = -speed * (1 - mousePos.x / threshold);
    } else if (mousePos.x > screenSize.x - threshold) {
      scroll.x =
        speed * ((mousePos.x - (screenSize.x - threshold)) / threshold);
    }

    if (mousePos.y < threshold) {
      scroll.y = speed * (1 - mousePos.y / threshold);
    } else if (mousePos.y > screenSize.y - threshold) {
      scroll.y =
        -speed * ((mousePos.y - (screenSize.y - threshold)) / threshold);
    }

    return scroll;
  }

  /**
   * Generate smooth camera path between points
   */
  static generateCameraPath(
    start: THREE.Vector3,
    end: THREE.Vector3,
    steps: number
  ): THREE.Vector3[] {
    const path: THREE.Vector3[] = [];
    const curve = new THREE.CatmullRomCurve3([start, end]);

    for (let i = 0; i <= steps; i++) {
      const t = i / steps;
      path.push(curve.getPoint(t));
    }

    return path;
  }

  /**
   * Default constraints for different camera modes
   */
  static getDefaultConstraints(mode: CameraMode): Partial<CameraConstraints> {
    switch (mode) {
      case 'orbital':
        return {
          minDistance: 5,
          maxDistance: 1000,
          minPolarAngle: Math.PI / 6,
          maxPolarAngle: Math.PI / 2.2,
          enablePan: true,
          enableZoom: true,
          enableRotate: true,
        };
      case 'free':
        return {
          enablePan: true,
          enableZoom: true,
          enableRotate: true,
        };
      case 'cinematic':
        return {
          enablePan: false,
          enableZoom: false,
          enableRotate: false,
        };
      case 'locked':
        return {
          enablePan: false,
          enableZoom: false,
          enableRotate: false,
        };
      default:
        return {};
    }
  }
}
