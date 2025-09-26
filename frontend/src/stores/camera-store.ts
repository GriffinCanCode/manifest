/**
 * Camera-specific state management
 * Focused only on camera modes, bookmarks, and effects
 * Does not duplicate render-store responsibilities
 */

import type * as THREE from 'three';
import { create } from 'zustand';
import { devtools, subscribeWithSelector } from 'zustand/middleware';
import { immer } from 'zustand/middleware/immer';

export type CameraMode = 'orbital' | 'free' | 'cinematic' | 'locked';

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

interface CameraState {
  // Current mode
  currentMode: CameraMode;

  // Effects
  shake: CameraShake;
  focusTarget: THREE.Vector3 | null;

  // Transitions
  transitionTo: CameraTransition | null;
  isTransitioning: boolean;

  // Bookmarks
  bookmarks: CameraBookmark[];

  // Constraints per mode
  constraints: Record<CameraMode, Record<string, unknown>>;
}

interface CameraActions {
  setMode: (mode: CameraMode) => void;
  setShake: (shake: Partial<CameraShake>) => void;
  setFocusTarget: (target: THREE.Vector3 | null) => void;

  // Transitions
  startTransition: (transition: CameraTransition) => void;
  completeTransition: () => void;

  // Bookmarks
  addBookmark: (bookmark: CameraBookmark) => void;
  removeBookmark: (id: string) => void;
  goToBookmark: (id: string) => void;

  // Constraints
  setConstraints: (
    mode: CameraMode,
    constraints: Record<string, unknown>
  ) => void;

  // Reset
  reset: () => void;
}

const DEFAULT_STATE: CameraState = {
  currentMode: 'orbital',
  shake: { intensity: 0, duration: 0, decay: 0.95 },
  focusTarget: null,
  transitionTo: null,
  isTransitioning: false,
  bookmarks: [],
  constraints: {
    orbital: {
      minDistance: 5,
      maxDistance: 1000,
      minPolarAngle: Math.PI / 6,
      maxPolarAngle: Math.PI / 2.2,
    },
    free: {
      movementSpeed: 10,
      rollSpeed: 0.3,
    },
    cinematic: {
      smoothness: 0.1,
      lookAhead: 2,
    },
    locked: {},
  },
};

type CameraStoreState = CameraState & CameraActions;

export const useCameraStore = create<CameraStoreState>()(
  subscribeWithSelector(
    devtools(
      immer((set, get) => ({
        ...DEFAULT_STATE,

        setMode: (mode: CameraMode) =>
          set(
            state => {
              state.currentMode = mode;
            },
            false,
            'setMode'
          ),

        setShake: (shake: Partial<CameraShake>) =>
          set(
            state => {
              Object.assign(state.shake, shake);
            },
            false,
            'setShake'
          ),

        setFocusTarget: (target: THREE.Vector3 | null) =>
          set(
            state => {
              state.focusTarget = target;
            },
            false,
            'setFocusTarget'
          ),

        startTransition: (transition: CameraTransition) =>
          set(
            state => {
              state.transitionTo = transition;
              state.isTransitioning = true;
            },
            false,
            'startTransition'
          ),

        completeTransition: () =>
          set(
            state => {
              state.transitionTo = null;
              state.isTransitioning = false;
            },
            false,
            'completeTransition'
          ),

        addBookmark: (bookmark: CameraBookmark) =>
          set(
            state => {
              state.bookmarks.push(bookmark);
            },
            false,
            'addBookmark'
          ),

        removeBookmark: (id: string) =>
          set(
            state => {
              state.bookmarks = state.bookmarks.filter(b => b.id !== id);
            },
            false,
            'removeBookmark'
          ),

        goToBookmark: (id: string) => {
          const bookmark = get().bookmarks.find(b => b.id === id);
          if (bookmark) {
            get().startTransition({
              position: bookmark.position,
              target: bookmark.target,
              duration: 1000,
            });
            get().setMode(bookmark.mode);
          }
        },

        setConstraints: (
          mode: CameraMode,
          constraints: Record<string, unknown>
        ) =>
          set(
            state => {
              state.constraints[mode] = {
                ...state.constraints[mode],
                ...constraints,
              };
            },
            false,
            'setConstraints'
          ),

        reset: () => set(() => ({ ...DEFAULT_STATE }), false, 'reset'),
      })),
      { name: 'camera-store' }
    )
  )
);
