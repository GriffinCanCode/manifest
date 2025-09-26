/**
 * Camera and Control System Types
 * Centralized type definitions for camera system
 */

import type * as THREE from 'three';

export type CameraMode = 'orbital' | 'free' | 'cinematic' | 'locked';

export interface CameraConstraints {
  minDistance: number;
  maxDistance: number;
  minPolarAngle: number;
  maxPolarAngle: number;
  minAzimuthAngle?: number;
  maxAzimuthAngle?: number;
  bounds?: THREE.Box3;
  enablePan: boolean;
  enableZoom: boolean;
  enableRotate: boolean;
}

export interface CameraBookmark {
  id: string;
  name: string;
  position: [number, number, number];
  target: [number, number, number];
  mode: CameraMode;
  fov?: number;
  createdAt: number;
}

export interface CameraShake {
  intensity: number;
  duration: number;
  decay: number;
}

export interface CameraTransition {
  position: [number, number, number];
  target: [number, number, number];
  duration: number;
  easing?: string;
}

export interface InputHandlerOptions {
  enableKeyboard: boolean;
  enableMouse: boolean;
  enableTouch: boolean;
  enableGamepad: boolean;
  edgeScrollThreshold: number;
  scrollSpeed: number;
}

export interface SelectionBox {
  start: THREE.Vector2;
  end: THREE.Vector2;
  active: boolean;
}

export interface ContextMenuOptions {
  position: THREE.Vector2;
  items: Array<{
    id: string;
    label: string;
    action: () => void;
    disabled?: boolean;
  }>;
  visible: boolean;
}

export interface TooltipData {
  content: string;
  position: THREE.Vector2;
  visible: boolean;
  delay?: number;
}

export interface CameraControlsProps {
  mode?: CameraMode;
  constraints?: Partial<CameraConstraints>;
  enableShake?: boolean;
  enableFocus?: boolean;
  smoothTransitions?: boolean;
  onModeChange?: (mode: CameraMode) => void;
}
