/**
 * WebGL2/WebGPU Initialization Component
 * Handles device detection, capability assessment, and optimal render configuration
 */

import { Canvas } from '@react-three/fiber';
import { Leva } from 'leva';
import React, { useCallback, useEffect, useState } from 'react';

import { useLogger, usePerformanceLogger } from '../../../../hooks/use-logger';
import { useRenderStore } from '../../../../stores/render-store';
import {
  detectCapabilities,
  getOptimalRenderingSettings,
  type DeviceCapabilities,
  type RenderingSettings,
} from '../../../../utils/capabilities';
import { performanceMonitor } from '../../../../utils/performance';
import { ShaderProvider } from '../providers/ShaderProvider';

// Browser-compatible environment check with proper typing
interface ViteImportMeta {
  env?: {
    MODE?: string;
    [key: string]: unknown;
  };
}

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
  enableDevTools = (import.meta as ViteImportMeta)?.env?.MODE === 'development',
  onInitialized,
  onError,
}) => {
  const [initState, setInitState] = useState<InitializationState>({
    phase: 'detecting',
    progress: 0,
    message: 'Detecting device capabilities...',
  });

  // Initialize logging
  const renderLogger = useLogger('render', 'RenderInitializer');
  const performanceLogger = usePerformanceLogger(
    'performance',
    'RenderInitializer'
  );

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
    const timer = performanceLogger.startTimer('render-initialization');

    try {
      renderLogger.info('Starting render system initialization', {
        enableDevTools,
        devMode,
        viewport: {
          width: window.innerWidth,
          height: window.innerHeight,
          pixelRatio: window.devicePixelRatio,
        },
      });

      setInitState({
        phase: 'detecting',
        progress: 10,
        message: 'Detecting device capabilities...',
      });

      // Detect device capabilities
      const deviceCapabilities = await detectCapabilities();
      setCapabilities(deviceCapabilities);

      renderLogger.info('Device capabilities detected', {
        gpuTier: deviceCapabilities.gpuTier,
        preferredBackend: deviceCapabilities.preferredBackend,
        maxTextureSize: deviceCapabilities.maxTextureSize,
        supportsHDR: deviceCapabilities.supportsHDR,
        supportsShadows: deviceCapabilities.supportsShadows,
        supportsInstancing: deviceCapabilities.supportsInstancing,
        supportsFloatTextures: deviceCapabilities.supportsFloatTextures,
        maxAnisotropy: deviceCapabilities.maxAnisotropy,
      });

      setInitState({
        phase: 'initializing',
        progress: 40,
        message: `Detected ${deviceCapabilities.preferredBackend.toUpperCase()} support...`,
      });

      // Generate optimal settings
      const optimalSettings = await getOptimalRenderingSettings();
      setSettings(optimalSettings);

      renderLogger.info('Optimal render settings generated', {
        backend: optimalSettings.backend,
        powerPreference: optimalSettings.powerPreference,
        antialias: optimalSettings.antialias,
        shadows: optimalSettings.shadows,
        precision: optimalSettings.precision,
        logarithmicDepthBuffer: optimalSettings.logarithmicDepthBuffer,
      });

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

      renderLogger.debug('Viewport configured', {
        width: window.innerWidth,
        height: window.innerHeight,
        pixelRatio: window.devicePixelRatio,
      });

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
        renderLogger.info('Performance monitoring initialized');
      }

      setInitState({
        phase: 'ready',
        progress: 100,
        message: 'Render system initialized successfully!',
      });

      setInitialized(true);

      timer.end('Render system initialization completed', {
        backend: deviceCapabilities.preferredBackend,
        powerPreference: optimalSettings.powerPreference,
        devToolsEnabled: enableDevTools && devMode,
      });

      onInitialized?.(deviceCapabilities, optimalSettings);
    } catch (error) {
      const err =
        error instanceof Error ? error : new Error('Initialization failed');

      timer.end('Render system initialization failed');

      renderLogger.error('Render initialization failed', err, {
        phase: initState.phase,
        progress: initState.progress,
        enableDevTools,
        devMode,
      });

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
    renderLogger,
    performanceLogger,
    initState.phase,
    initState.progress,
  ]);

  /**
   * Handle window resize
   */
  const handleResize = useCallback(() => {
    const newWidth = window.innerWidth;
    const newHeight = window.innerHeight;
    const newPixelRatio = window.devicePixelRatio;

    renderLogger.debug('Viewport resized', {
      oldViewport: viewport,
      newViewport: {
        width: newWidth,
        height: newHeight,
        pixelRatio: newPixelRatio,
      },
    });

    setViewport(newWidth, newHeight, newPixelRatio);
  }, [setViewport, viewport, renderLogger]);

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
        renderLogger.info('Cleaning up render initialization');
        performanceMonitor.destroy();
      }
    };
  }, [enableDevTools, renderLogger]);

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
        <ShaderProvider>
          <RenderMetricsTracker />
          {children}
        </ShaderProvider>
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

    if ((import.meta as ViteImportMeta)?.env?.MODE === 'development') {
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

    <style>{`
      .initialization-screen {
        position: fixed;
        top: 0;
        left: 0;
        width: 100vw;
        height: 100vh;
        background: linear-gradient(135deg, #1a1a2e 0%, #16213e 100%);
        display: flex;
        align-items: center;
        justify-content: center;
        color: white;
        font-family: 'Inter', system-ui, sans-serif;
        z-index: 1000;
      }

      .init-content {
        text-align: center;
        max-width: 500px;
        padding: 2rem;
      }

      .init-logo {
        margin-bottom: 3rem;
      }

      .init-logo h1 {
        font-size: 3rem;
        font-weight: 700;
        margin: 1rem 0 0 0;
        background: linear-gradient(45deg, #64b5f6, #42a5f5, #2196f3);
        -webkit-background-clip: text;
        -webkit-text-fill-color: transparent;
        background-clip: text;
      }

      .spinner {
        width: 60px;
        height: 60px;
        border: 4px solid rgba(255, 255, 255, 0.1);
        border-left: 4px solid #2196f3;
        border-radius: 50%;
        margin: 0 auto;
        animation: spin 1s linear infinite;
      }

      @keyframes spin {
        0% { transform: rotate(0deg); }
        100% { transform: rotate(360deg); }
      }

      .init-progress {
        margin: 2rem 0;
      }

      .progress-bar {
        width: 100%;
        height: 8px;
        background: rgba(255, 255, 255, 0.1);
        border-radius: 4px;
        overflow: hidden;
        margin-bottom: 1rem;
      }

      .progress-fill {
        height: 100%;
        background: linear-gradient(90deg, #2196f3, #42a5f5);
        transition: width 0.3s ease;
        border-radius: 4px;
      }

      .progress-message {
        font-size: 1.1rem;
        margin: 0.5rem 0;
        color: rgba(255, 255, 255, 0.9);
      }

      .progress-percent {
        font-size: 0.9rem;
        color: rgba(255, 255, 255, 0.7);
        font-weight: 600;
      }

      .init-details {
        margin-top: 2rem;
        text-align: left;
      }

      .init-details p {
        margin-bottom: 1rem;
        color: rgba(255, 255, 255, 0.8);
      }

      .init-details ul {
        list-style: none;
        padding: 0;
      }

      .init-details li {
        padding: 0.25rem 0;
        color: rgba(255, 255, 255, 0.6);
        font-size: 0.9rem;
      }

      .init-details li:before {
        content: '⚡';
        margin-right: 0.5rem;
      }
    `}</style>
  </div>
);

/**
 * Error screen for initialization failures
 */
const ErrorScreen: React.FC<{ error: Error; onRetry: () => void }> = ({
  error,
  onRetry,
}) => (
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

    <style>{`
      .error-screen {
        position: fixed;
        top: 0;
        left: 0;
        width: 100vw;
        height: 100vh;
        background: linear-gradient(135deg, #2d1b1b 0%, #1a1a1a 100%);
        display: flex;
        align-items: center;
        justify-content: center;
        color: white;
        font-family: 'Inter', system-ui, sans-serif;
        z-index: 1000;
      }

      .error-content {
        text-align: center;
        max-width: 600px;
        padding: 2rem;
      }

      .error-content h1 {
        color: #f44336;
        font-size: 2.5rem;
        margin-bottom: 1rem;
        font-weight: 700;
      }

      .error-message {
        font-size: 1.1rem;
        color: rgba(255, 255, 255, 0.9);
        margin-bottom: 2rem;
        background: rgba(244, 67, 54, 0.1);
        padding: 1rem;
        border-radius: 8px;
        border-left: 4px solid #f44336;
      }

      .error-details {
        text-align: left;
        margin: 2rem 0;
      }

      .error-details h3 {
        color: rgba(255, 255, 255, 0.9);
        margin-bottom: 1rem;
      }

      .error-details ul {
        list-style: none;
        padding: 0;
      }

      .error-details li {
        padding: 0.5rem 0;
        color: rgba(255, 255, 255, 0.7);
      }

      .error-details li:before {
        content: '💡';
        margin-right: 0.5rem;
      }

      .error-actions {
        margin-top: 2rem;
      }

      .retry-button {
        background: linear-gradient(45deg, #f44336, #e53935);
        color: white;
        border: none;
        padding: 1rem 2rem;
        font-size: 1rem;
        font-weight: 600;
        border-radius: 8px;
        cursor: pointer;
        transition: all 0.2s ease;
      }

      .retry-button:hover {
        transform: translateY(-2px);
        box-shadow: 0 4px 12px rgba(244, 67, 54, 0.3);
      }
    `}</style>
  </div>
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
