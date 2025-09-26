/**
 * Comprehensive render state management for WebGL2/WebGPU pipeline
 * Handles device capabilities, performance monitoring, and render settings
 */

import { create } from 'zustand';
import { devtools, subscribeWithSelector } from 'zustand/middleware';
import { immer } from 'zustand/middleware/immer';

import { PerformanceLogger, RenderLogger } from '../services/logger';
import type {
  DeviceCapabilities,
  RenderingSettings,
} from '../utils/capabilities';

// Type-safe import.meta interface for Vite environment
interface ViteImportMeta {
  env?: {
    MODE?: string;
    [key: string]: unknown;
  };
}

// Extend Window interface for performance tracking
declare global {
  interface Window {
    lastPerformanceWarning?: number;
    lastPerformanceDebug?: number;
  }
}

export interface PerformanceMetrics {
  fps: number;
  frameTime: number;
  drawCalls: number;
  triangles: number;
  points: number;
  lines: number;
  memoryUsage: {
    geometries: number;
    textures: number;
    programs: number;
  };
  gpuMemoryUsage?: {
    buffer: number;
    texture: number;
    renderBuffer: number;
  };
}

export interface RenderQuality {
  level: 'low' | 'medium' | 'high' | 'ultra';
  shadows: boolean;
  antialias: boolean;
  postProcessing: boolean;
  particleQuality: number; // 0.1 to 1.0
  lodBias: number; // 0.5 to 2.0
  renderScale: number; // 0.5 to 2.0
}

export interface RenderDebug {
  showWireframe: boolean;
  showBounds: boolean;
  showStats: boolean;
  showLOD: boolean;
  freezeCulling: boolean;
  disableFog: boolean;
  showGizmos: boolean;
  logFrameTime: boolean;
}

export interface CameraState {
  position: [number, number, number];
  target: [number, number, number];
  zoom: number;
  fov: number;
  near: number;
  far: number;
  isDirty: boolean;
}

export interface RenderState {
  // Device & Initialization
  capabilities: DeviceCapabilities | null;
  settings: RenderingSettings | null;
  isInitialized: boolean;
  initError: string | null;

  // Performance
  metrics: PerformanceMetrics;
  targetFPS: number;
  adaptiveQuality: boolean;

  // Quality Settings
  quality: RenderQuality;

  // Camera
  camera: CameraState;

  // Debug
  debug: RenderDebug;
  devMode: boolean;

  // Viewport
  viewport: {
    width: number;
    height: number;
    pixelRatio: number;
  };

  // Culling & LOD
  culling: {
    frustumCulling: boolean;
    occlusionCulling: boolean;
    maxDistance: number;
    lodLevels: number[];
  };

  // Postprocessing
  postprocessing: {
    enabled: boolean;
    bloom: boolean;
    ssao: boolean;
    fxaa: boolean;
    toneMappingExposure: number;
  };

  // Shadow system
  shadows: {
    enabled: boolean;
    cascades: number;
    mapSize: number;
    maxDistance: number;
    bias: number;
    normalBias: number;
    quality: 'low' | 'medium' | 'high' | 'ultra';
  };
}

interface RenderActions {
  // Initialization
  setCapabilities: (capabilities: DeviceCapabilities) => void;
  setSettings: (settings: RenderingSettings) => void;
  setInitialized: (initialized: boolean, error?: string) => void;

  // Performance
  updateMetrics: (metrics: Partial<PerformanceMetrics>) => void;
  setTargetFPS: (fps: number) => void;
  toggleAdaptiveQuality: (enabled?: boolean) => void;

  // Quality
  setQuality: (quality: Partial<RenderQuality>) => void;
  setQualityPreset: (level: RenderQuality['level']) => void;

  // Camera
  updateCamera: (camera: Partial<CameraState>) => void;
  resetCamera: () => void;

  // Debug
  setDebug: (debug: Partial<RenderDebug>) => void;
  toggleDevMode: (enabled?: boolean) => void;

  // Viewport
  setViewport: (width: number, height: number, pixelRatio: number) => void;

  // Culling
  setCulling: (culling: Partial<RenderState['culling']>) => void;

  // Postprocessing
  setPostprocessing: (pp: Partial<RenderState['postprocessing']>) => void;

  // Shadows
  setShadows: (shadows: Partial<RenderState['shadows']>) => void;

  // Utilities
  resetToDefaults: () => void;
  optimizeForDevice: () => void;
}

type RenderStoreState = RenderState & RenderActions;

const DEFAULT_METRICS: PerformanceMetrics = {
  fps: 60,
  frameTime: 16.67,
  drawCalls: 0,
  triangles: 0,
  points: 0,
  lines: 0,
  memoryUsage: {
    geometries: 0,
    textures: 0,
    programs: 0,
  },
};

const DEFAULT_QUALITY: RenderQuality = {
  level: 'high',
  shadows: true,
  antialias: true,
  postProcessing: true,
  particleQuality: 1.0,
  lodBias: 1.0,
  renderScale: 1.0,
};

const DEFAULT_CAMERA: CameraState = {
  position: [15, 15, 15],
  target: [0, 0, 0],
  zoom: 1.0,
  fov: 65,
  near: 0.1,
  far: 1000,
  isDirty: false,
};

const DEFAULT_DEBUG: RenderDebug = {
  showWireframe: false,
  showBounds: false,
  showStats: false,
  showLOD: false,
  freezeCulling: false,
  disableFog: false,
  showGizmos: false,
  logFrameTime: false,
};

export const useRenderStore = create<RenderStoreState>()(
  subscribeWithSelector(
    devtools(
      immer((set, _get) => ({
        // Initial state
        capabilities: null,
        settings: null,
        isInitialized: false,
        initError: null,

        metrics: DEFAULT_METRICS,
        targetFPS: 60,
        adaptiveQuality: true,

        quality: DEFAULT_QUALITY,

        camera: DEFAULT_CAMERA,

        debug: DEFAULT_DEBUG,
        devMode: (import.meta as ViteImportMeta)?.env?.MODE === 'development',

        viewport: {
          width: 1920,
          height: 1080,
          pixelRatio: 1,
        },

        culling: {
          frustumCulling: true,
          occlusionCulling: false,
          maxDistance: 100,
          lodLevels: [10, 25, 50, 100],
        },

        postprocessing: {
          enabled: true,
          bloom: true,
          ssao: true,
          fxaa: true,
          toneMappingExposure: 1.0,
        },

        shadows: {
          enabled: true,
          cascades: 3,
          mapSize: 2048,
          maxDistance: 500,
          bias: -0.0001,
          normalBias: 0.02,
          quality: 'high',
        },

        // Actions
        setCapabilities: capabilities => {
          RenderLogger.info('Device capabilities set', {
            preferredBackend: capabilities.preferredBackend,
            maxTextureSize: capabilities.maxTextureSize,
            supportsHDR: capabilities.supportsHDR,
            supportsShadows: capabilities.supportsShadows,
            supportsWebGPU: capabilities.supportsWebGPU,
            gpuTier: capabilities.gpuTier,
          });

          set(
            state => {
              state.capabilities = capabilities;
            },
            false,
            'setCapabilities'
          );
        },

        setSettings: settings => {
          RenderLogger.info('Render settings applied', {
            backend: settings.backend,
            pixelRatio: settings.pixelRatio,
            antialias: settings.antialias,
            shadows: settings.shadows,
            powerPreference: settings.powerPreference,
            precision: settings.precision,
          });

          set(
            state => {
              state.settings = settings;
            },
            false,
            'setSettings'
          );
        },

        setInitialized: (initialized, error) => {
          if (initialized) {
            RenderLogger.info('Render system marked as initialized');
          } else {
            RenderLogger.error(
              'Render system initialization failed',
              error ? new Error(error) : new Error('Unknown error'),
              {
                errorMessage: error,
              }
            );
          }

          set(
            state => {
              state.isInitialized = initialized;
              state.initError = error ?? null;
            },
            false,
            'setInitialized'
          );
        },

        updateMetrics: metrics => {
          // Only log significant metric changes or performance issues
          const shouldLogWarning = metrics.fps && metrics.fps < 30;
          const shouldLogHighFrameTime =
            metrics.frameTime && metrics.frameTime > 33.33; // >30fps

          // Rate limit performance warnings to avoid spam (max once per 5 seconds)
          const now = Date.now();
          if (shouldLogWarning || shouldLogHighFrameTime) {
            const lastWarning = window.lastPerformanceWarning ?? 0;
            if (now - lastWarning > 5000) {
              PerformanceLogger.warn('Performance metrics indicate issues', {
                ...metrics,
                warning: shouldLogWarning ? 'Low FPS detected' : undefined,
                highFrameTime: shouldLogHighFrameTime
                  ? 'Frame time exceeded threshold'
                  : undefined,
              });
              window.lastPerformanceWarning = now;
            }
          } else if (
            (import.meta as ViteImportMeta)?.env?.MODE === 'development' &&
            metrics.fps
          ) {
            // Debug logging for development (also rate limited to once per 2 seconds)
            const lastDebug = window.lastPerformanceDebug ?? 0;
            if (now - lastDebug > 2000) {
              PerformanceLogger.debug('Performance metrics updated', metrics);
              window.lastPerformanceDebug = now;
            }
          }

          set(
            state => {
              Object.assign(state.metrics, metrics);
            },
            false,
            'updateMetrics'
          );
        },

        setTargetFPS: fps =>
          set(
            state => {
              state.targetFPS = Math.max(30, Math.min(144, fps));
            },
            false,
            'setTargetFPS'
          ),

        toggleAdaptiveQuality: enabled =>
          set(
            state => {
              state.adaptiveQuality = enabled ?? !state.adaptiveQuality;
            },
            false,
            'toggleAdaptiveQuality'
          ),

        setQuality: quality => {
          RenderLogger.debug('Render quality updated', quality);

          set(
            state => {
              Object.assign(state.quality, quality);
            },
            false,
            'setQuality'
          );
        },

        setQualityPreset: level =>
          set(
            state => {
              const presets: Record<
                RenderQuality['level'],
                Partial<RenderQuality>
              > = {
                low: {
                  level,
                  shadows: false,
                  antialias: false,
                  postProcessing: false,
                  particleQuality: 0.3,
                  lodBias: 0.5,
                  renderScale: 0.75,
                },
                medium: {
                  level,
                  shadows: true,
                  antialias: false,
                  postProcessing: false,
                  particleQuality: 0.6,
                  lodBias: 0.75,
                  renderScale: 0.9,
                },
                high: {
                  level,
                  shadows: true,
                  antialias: true,
                  postProcessing: true,
                  particleQuality: 0.8,
                  lodBias: 1.0,
                  renderScale: 1.0,
                },
                ultra: {
                  level,
                  shadows: true,
                  antialias: true,
                  postProcessing: true,
                  particleQuality: 1.0,
                  lodBias: 1.5,
                  renderScale: 1.2,
                },
              };

              Object.assign(state.quality, presets[level]);
            },
            false,
            'setQualityPreset'
          ),

        updateCamera: camera =>
          set(
            state => {
              Object.assign(state.camera, camera);
              state.camera.isDirty = true;
            },
            false,
            'updateCamera'
          ),

        resetCamera: () =>
          set(
            state => {
              state.camera = { ...DEFAULT_CAMERA, isDirty: true };
            },
            false,
            'resetCamera'
          ),

        setDebug: debug =>
          set(
            state => {
              Object.assign(state.debug, debug);
            },
            false,
            'setDebug'
          ),

        toggleDevMode: enabled =>
          set(
            state => {
              state.devMode = enabled ?? !state.devMode;
            },
            false,
            'toggleDevMode'
          ),

        setViewport: (width, height, pixelRatio) => {
          RenderLogger.debug('Viewport updated', {
            width,
            height,
            pixelRatio,
            aspectRatio: width / height,
          });

          set(
            state => {
              state.viewport = { width, height, pixelRatio };
            },
            false,
            'setViewport'
          );
        },

        setCulling: culling =>
          set(
            state => {
              Object.assign(state.culling, culling);
            },
            false,
            'setCulling'
          ),

        setPostprocessing: pp =>
          set(
            state => {
              Object.assign(state.postprocessing, pp);
            },
            false,
            'setPostprocessing'
          ),

        setShadows: shadows =>
          set(
            state => {
              Object.assign(state.shadows, shadows);
            },
            false,
            'setShadows'
          ),

        resetToDefaults: () =>
          set(
            state => {
              state.quality = DEFAULT_QUALITY;
              state.camera = { ...DEFAULT_CAMERA, isDirty: true };
              state.debug = DEFAULT_DEBUG;
              state.targetFPS = 60;
              state.adaptiveQuality = true;
            },
            false,
            'resetToDefaults'
          ),

        optimizeForDevice: () =>
          set(
            state => {
              const { capabilities } = state;
              if (!capabilities) return;

              // Optimize based on GPU tier
              switch (capabilities.gpuTier) {
                case 'low':
                  Object.assign(state.quality, {
                    level: 'low' as const,
                    shadows: false,
                    antialias: false,
                    postProcessing: false,
                    particleQuality: 0.3,
                    lodBias: 0.5,
                    renderScale: 0.75,
                  });
                  state.targetFPS = 30;
                  break;

                case 'medium':
                  Object.assign(state.quality, {
                    level: 'medium' as const,
                    shadows: capabilities.supportsShadows,
                    antialias: false,
                    postProcessing: capabilities.supportsFloatTextures,
                    particleQuality: 0.6,
                    lodBias: 0.75,
                    renderScale: 0.9,
                  });
                  state.targetFPS = 60;
                  break;

                case 'high':
                  Object.assign(state.quality, {
                    level: 'high' as const,
                    shadows: capabilities.supportsShadows,
                    antialias: true,
                    postProcessing: capabilities.supportsHDR,
                    particleQuality: 1.0,
                    lodBias: 1.0,
                    renderScale: 1.0,
                  });
                  state.targetFPS = 60;
                  break;
              }

              // Adjust culling based on capabilities
              state.culling.occlusionCulling = capabilities.gpuTier === 'high';

              // Adjust postprocessing based on capabilities
              state.postprocessing.bloom = capabilities.supportsHDR;
              state.postprocessing.ssao = capabilities.supportsFloatTextures;

              // Adjust shadows based on capabilities and GPU tier
              if (capabilities.supportsShadows) {
                switch (capabilities.gpuTier) {
                  case 'low':
                    state.shadows.quality = 'low';
                    state.shadows.cascades = 2;
                    state.shadows.mapSize = 512;
                    state.shadows.maxDistance = 100;
                    break;
                  case 'medium':
                    state.shadows.quality = 'medium';
                    state.shadows.cascades = 3;
                    state.shadows.mapSize = 1024;
                    state.shadows.maxDistance = 250;
                    break;
                  case 'high':
                    state.shadows.quality = 'high';
                    state.shadows.cascades = 3;
                    state.shadows.mapSize = 2048;
                    state.shadows.maxDistance = 500;
                    break;
                }
              } else {
                state.shadows.enabled = false;
              }
            },
            false,
            'optimizeForDevice'
          ),
      })),
      { name: 'manifest-render-store' }
    )
  )
);

// Performance monitoring hook
export const usePerformanceMonitoring = () => {
  const updateMetrics = useRenderStore(state => state.updateMetrics);
  const adaptiveQuality = useRenderStore(state => state.adaptiveQuality);
  const setQualityPreset = useRenderStore(state => state.setQualityPreset);
  const currentQuality = useRenderStore(state => state.quality.level);

  const checkPerformance = (fps: number, frameTime: number) => {
    updateMetrics({ fps, frameTime });

    if (adaptiveQuality) {
      // Adaptive quality logic
      if (fps < 45 && currentQuality !== 'low') {
        const newQuality =
          currentQuality === 'ultra'
            ? 'high'
            : currentQuality === 'high'
              ? 'medium'
              : 'low';

        PerformanceLogger.info('Adaptive quality decreased due to low FPS', {
          currentFPS: fps,
          currentFrameTime: frameTime,
          oldQuality: currentQuality,
          newQuality,
          threshold: 45,
        });

        setQualityPreset(newQuality);
      } else if (fps > 55 && currentQuality !== 'ultra') {
        const newQuality =
          currentQuality === 'low'
            ? 'medium'
            : currentQuality === 'medium'
              ? 'high'
              : 'ultra';

        PerformanceLogger.info('Adaptive quality increased due to good FPS', {
          currentFPS: fps,
          currentFrameTime: frameTime,
          oldQuality: currentQuality,
          newQuality,
          threshold: 55,
        });

        setQualityPreset(newQuality);
      }
    }
  };

  return { checkPerformance };
};

export type { RenderStoreState };
