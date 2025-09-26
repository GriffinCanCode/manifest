/**
 * WebGL2/WebGPU Initialization Component
 * Handles device detection, capability assessment, and optimal render configuration
 */

import { Html } from '@react-three/drei';
import { Canvas } from '@react-three/fiber';
import { Leva } from 'leva';
import React, { useCallback, useEffect, useState } from 'react';

import { useRenderStore } from '../../../../stores/render-store';
import {
  detectCapabilities,
  getOptimalRenderingSettings,
  type DeviceCapabilities,
  type RenderingSettings,
} from '../../../../utils/capabilities';
import { performanceMonitor } from '../../../../utils/performance';

interface RenderInitializerProps {
  children: React.ReactNode;
  enableDevTools?: boolean;
  onInitialized?: (
    capabilities: DeviceCapabilities,
    settings: RenderingSettings
  ) => void;
  onError?: (error: Error) => void;
}

interface InitializationState {
  phase: 'detecting' | 'initializing' | 'ready' | 'error';
  progress: number;
  message: string;
  error?: Error;
}

/**
 * Comprehensive render initialization with WebGL2/WebGPU detection
 */
export const RenderInitializer: React.FC<RenderInitializerProps> = ({
  children,
  enableDevTools = process.env.NODE_ENV === 'development',
  onInitialized,
  onError,
}) => {
  const [initState, setInitState] = useState<InitializationState>({
    phase: 'detecting',
    progress: 0,
    message: 'Detecting device capabilities...',
  });

  const {
    capabilities,
    settings,
    isInitialized,
    devMode,
    viewport,
    setCapabilities,
    setSettings,
    setInitialized,
    setViewport,
    optimizeForDevice,
  } = useRenderStore();

  /**
   * Initialize device capabilities and rendering settings
   */
  const initializeRenderer = useCallback(async () => {
    try {
      setInitState({
        phase: 'detecting',
        progress: 10,
        message: 'Detecting device capabilities...',
      });

      // Detect device capabilities
      const deviceCapabilities = await detectCapabilities();
      setCapabilities(deviceCapabilities);

      setInitState({
        phase: 'initializing',
        progress: 40,
        message: `Detected ${deviceCapabilities.preferredBackend.toUpperCase()} support...`,
      });

      // Generate optimal settings
      const optimalSettings = await getOptimalRenderingSettings();
      setSettings(optimalSettings);

      setInitState({
        phase: 'initializing',
        progress: 70,
        message: 'Optimizing for device...',
      });

      // Optimize store for device
      optimizeForDevice();

      // Set viewport
      setViewport(
        window.innerWidth,
        window.innerHeight,
        window.devicePixelRatio
      );

      setInitState({
        phase: 'initializing',
        progress: 90,
        message: 'Initializing performance monitoring...',
      });

      // Initialize performance monitoring in dev mode
      if (enableDevTools && devMode) {
        performanceMonitor.init();
        performanceMonitor.onUpdate(metrics => {
          useRenderStore.getState().updateMetrics(metrics);
        });
      }

      setInitState({
        phase: 'ready',
        progress: 100,
        message: 'Render system initialized successfully!',
      });

      setInitialized(true);
      onInitialized?.(deviceCapabilities, optimalSettings);
    } catch (error) {
      const err =
        error instanceof Error ? error : new Error('Initialization failed');

      setInitState({
        phase: 'error',
        progress: 0,
        message: `Initialization failed: ${err.message}`,
        error: err,
      });

      setInitialized(false, err.message);
      onError?.(err);
    }
  }, [
    setCapabilities,
    setSettings,
    setInitialized,
    setViewport,
    optimizeForDevice,
    enableDevTools,
    devMode,
    onInitialized,
    onError,
  ]);

  /**
   * Handle window resize
   */
  const handleResize = useCallback(() => {
    setViewport(window.innerWidth, window.innerHeight, window.devicePixelRatio);
  }, [setViewport]);

  // Initialize on mount
  useEffect(() => {
    if (
      !isInitialized &&
      initState.phase !== 'ready' &&
      initState.phase !== 'error'
    ) {
      void initializeRenderer();
    }
  }, [initializeRenderer, isInitialized, initState.phase]);

  // Handle window resize
  useEffect(() => {
    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, [handleResize]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      if (enableDevTools) {
        performanceMonitor.destroy();
      }
    };
  }, [enableDevTools]);

  // Show error screen if initialization failed
  if (initState.phase === 'error') {
    return (
      <div className='render-initializer'>
        <ErrorScreen
          error={initState.error ?? new Error('Unknown initialization error')}
          onRetry={() => void initializeRenderer()}
        />
        {enableDevTools && devMode && <Leva hidden={!devMode} />}
      </div>
    );
  }

  // Show loading screen during initialization
  if (!isInitialized || initState.phase !== 'ready') {
    return (
      <div className='render-initializer'>
        <InitializationScreen state={initState} />
        {enableDevTools && devMode && <Leva hidden={!devMode} />}
      </div>
    );
  }

  // Render main application with optimized Canvas
  return (
    <div className='render-initializer'>
      <Canvas
        camera={{
          position: [15, 15, 15],
          fov: 65,
          near: settings?.logarithmicDepthBuffer ? 1 : 0.1,
          far: settings?.logarithmicDepthBuffer ? 1000000 : 1000,
        }}
        shadows={
          capabilities?.supportsShadows &&
          settings?.powerPreference === 'high-performance'
        }
        dpr={[1, Math.min(viewport.pixelRatio, 2)]}
        gl={{
          antialias: settings?.antialias ?? true,
          alpha: settings?.alpha ?? false,
          premultipliedAlpha: settings?.premultipliedAlpha ?? true,
          preserveDrawingBuffer: settings?.preserveDrawingBuffer ?? false,
          powerPreference: settings?.powerPreference ?? 'default',
          precision: settings?.precision ?? 'highp',
          logarithmicDepthBuffer: settings?.logarithmicDepthBuffer ?? false,
          // Enable HDR rendering
          outputColorSpace: 'srgb-linear', // Linear color space for HDR
          // Tone mapping disabled (handled by post-processing)
        }}
        performance={{
          min: 0.1,
          max: 1,
          debounce: 200,
        }}
        frameloop='always'
      >
        <RenderMetricsTracker />
        {children}
      </Canvas>

      {/* Development tools */}
      {enableDevTools && devMode && capabilities && settings && (
        <>
          <Leva hidden={!devMode} />
          <DeviceInfoPanel capabilities={capabilities} settings={settings} />
        </>
      )}
    </div>
  );
};

/**
 * Tracks render metrics and updates the store
 */
const RenderMetricsTracker: React.FC = () => {
  const updateMetrics = useRenderStore(state => state.updateMetrics);

  React.useEffect(() => {
    let frameId: number;

    const updateFrameMetrics = () => {
      performanceMonitor.beginFrame();

      // This would be called by the actual render loop
      // For now, we simulate it
      setTimeout(() => {
        performanceMonitor.endFrame();
        const metrics = performanceMonitor.getMetrics();
        updateMetrics(metrics);
      }, 0);

      frameId = requestAnimationFrame(updateFrameMetrics);
    };

    if (process.env.NODE_ENV === 'development') {
      updateFrameMetrics();
    }

    return () => {
      if (frameId) {
        cancelAnimationFrame(frameId);
      }
    };
  }, [updateMetrics]);

  return null;
};

/**
 * Loading screen during initialization
 */
const InitializationScreen: React.FC<{ state: InitializationState }> = ({
  state,
}) => (
  <Html center>
    <div className='initialization-screen'>
      <div className='init-content'>
        <div className='init-logo'>
          <div className='spinner' />
          <h1>Manifest</h1>
        </div>

        <div className='init-progress'>
          <div className='progress-bar'>
            <div
              className='progress-fill'
              style={{ width: `${state.progress}%` }}
            />
          </div>
          <p className='progress-message'>{state.message}</p>
          <span className='progress-percent'>{state.progress}%</span>
        </div>

        <div className='init-details'>
          <p>Initializing rendering pipeline...</p>
          {state.phase === 'detecting' && (
            <ul>
              <li>Detecting WebGPU support</li>
              <li>Checking WebGL2 capabilities</li>
              <li>Analyzing GPU performance</li>
            </ul>
          )}
          {state.phase === 'initializing' && (
            <ul>
              <li>Configuring optimal settings</li>
              <li>Setting up performance monitoring</li>
              <li>Initializing render pipeline</li>
            </ul>
          )}
        </div>
      </div>
    </div>
  </Html>
);

/**
 * Error screen for initialization failures
 */
const ErrorScreen: React.FC<{ error: Error; onRetry: () => void }> = ({
  error,
  onRetry,
}) => (
  <Html center>
    <div className='error-screen'>
      <div className='error-content'>
        <h1>Initialization Failed</h1>
        <p className='error-message'>{error.message}</p>

        <div className='error-details'>
          <h3>Possible solutions:</h3>
          <ul>
            <li>Update your graphics drivers</li>
            <li>Try using a different browser</li>
            <li>Enable hardware acceleration</li>
            <li>Check if WebGL is supported</li>
          </ul>
        </div>

        <div className='error-actions'>
          <button onClick={onRetry} className='retry-button'>
            Retry Initialization
          </button>
        </div>
      </div>
    </div>
  </Html>
);

/**
 * Development panel showing device info
 */
const DeviceInfoPanel: React.FC<{
  capabilities: DeviceCapabilities;
  settings: RenderingSettings;
}> = ({ capabilities, settings }) => (
  <div className='device-info-panel'>
    <h3>Device Information</h3>
    <div className='info-grid'>
      <div>
        <strong>Backend:</strong> {capabilities.preferredBackend.toUpperCase()}
      </div>
      <div>
        <strong>GPU Tier:</strong> {capabilities.gpuTier}
      </div>
      <div>
        <strong>Max Texture Size:</strong> {capabilities.maxTextureSize}
      </div>
      <div>
        <strong>Supports Instancing:</strong>{' '}
        {capabilities.supportsInstancing ? 'Yes' : 'No'}
      </div>
      <div>
        <strong>Supports HDR:</strong> {capabilities.supportsHDR ? 'Yes' : 'No'}
      </div>
      <div>
        <strong>Power Preference:</strong> {settings.powerPreference}
      </div>
    </div>
  </div>
);

export default RenderInitializer;
