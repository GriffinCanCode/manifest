/**
 * Camera and Control System
 * Clean exports for all camera and control components
 */

// Main controller
export { default as CameraController } from './CameraController';

// Individual camera components
export { default as CinematicCamera } from './components/CinematicCamera';
export { default as FreeCamera } from './components/FreeCamera';
export { default as OrbitalCamera } from './components/OrbitalCamera';

// Hooks
export { useCameraEffects } from './hooks/use-camera-effects';
export { useInputHandling } from './hooks/use-input-handling';

// Utilities
export { CameraUtils } from './utils/CameraUtils';
export { cinematicUtils } from './utils/cinematic-utils';

// Types
export type {
  CameraBookmark,
  CameraConstraints,
  CameraControlsProps,
  CameraMode,
  CameraShake,
  CameraTransition,
  ContextMenuOptions,
  InputHandlerOptions,
  SelectionBox,
  TooltipData,
} from './types';

// Store (re-export from stores)
export { useCameraStore } from '../../stores/camera-store';
