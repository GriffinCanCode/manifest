import React from 'react';
import ReactDOM from 'react-dom/client';

import App from './App';
import { AppLogger } from './services/logger';
import { initializeShaderSystem } from './shaders/index';

import '@styles/index.scss';

// Hide loading screen once React is ready
const hideLoadingScreen = () => {
  const loadingScreen = document.getElementById('loading-screen');
  if (loadingScreen) {
    loadingScreen.style.opacity = '0';
    loadingScreen.style.transition = 'opacity 0.5s ease-out';
    setTimeout(() => {
      loadingScreen.remove();
    }, 500);
  }
};

// Initialize logging system first
console.warn('🚀 FRONTEND: Initializing logging system...');
console.warn('🚀 FRONTEND: Current URL:', window.location.href);
console.warn('🚀 FRONTEND: User Agent:', navigator.userAgent);
console.warn('🚀 FRONTEND: Environment Mode:', import.meta.env.MODE);

try {
  AppLogger.info('Frontend application starting', {
    environment: import.meta.env.MODE,
    timestamp: new Date().toISOString(),
    userAgent: navigator.userAgent,
  });
} catch (error) {
  console.error('🔥 FRONTEND: Logger initialization failed:', error);
}

// Initialize shader system
console.warn('🎨 FRONTEND: Initializing shader system...');
AppLogger.debug('Shader system initialization started');
initializeShaderSystem();
AppLogger.debug('Shader system initialization completed');

// Import shader diagnostics in development
if (import.meta.env.MODE === 'development') {
  // Load texture service for global console access
  void import('./services/texture-factory-service')
    .then(({ textureService }) => {
      (window as any).textureService = textureService;
      console.warn('🎨 TEXTURE SERVICE: Available globally');
      console.warn(
        '   • Type: textureService.clearCache() - Clear texture cache'
      );
      console.warn(
        '   • Type: textureService.generateTextures({resolution: 1024}) - Generate textures'
      );
      console.warn(
        '   • Type: textureService.debugLogTextures() - Show available textures'
      );
    })
    .catch(error => {
      console.error('Failed to load texture service:', error);
    });

  void import('./utils/shader-diagnostics')
    .then(() => {
      console.warn('🔍 SHADER DIAGNOSTICS: Loaded successfully');
      console.warn('   • Type: runShaderDiagnostics() - Run full diagnostics');
      console.warn('   • Type: runShaderSystemTests() - Run unit tests');
      console.warn(
        '   • Type: shaderDiagnostics.getResults() - Get last results'
      );
    })
    .catch(error => {
      console.error('Failed to load shader diagnostics:', error);
    });

  void import('./tests/shader-system.test')
    .then(() => {
      console.warn('🧪 SHADER TESTS: Test suite loaded');
    })
    .catch(error => {
      console.error('Failed to load shader tests:', error);
    });

  // Load tile render diagnostics
  void import('./utils/tile-render-diagnostics')
    .then(() => {
      console.warn('🔍 TILE DIAGNOSTICS: System loaded');
    })
    .catch(error => {
      console.error('Failed to load tile diagnostics:', error);
    });

  // Load camera exposure diagnostics
  void import('./utils/camera-exposure-diagnostics')
    .then(() => {
      console.warn('📷 CAMERA EXPOSURE: Three.js objects will be exposed');
    })
    .catch(error => {
      console.error('Failed to load camera exposure:', error);
    });
}

// Initialize React app
console.warn('⚛️  FRONTEND: Starting React application...');
AppLogger.info('React application initialization started');
ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);

// Hide loading screen after initial render
setTimeout(() => {
  console.warn('🎯 FRONTEND: Hiding loading screen');
  AppLogger.debug('Loading screen hidden, app fully loaded');
  hideLoadingScreen();
}, 1000);

console.warn('✅ FRONTEND: Main initialization complete');

// Auto-run tile render diagnostics after a short delay
setTimeout(async () => {
  const { tileRenderDiagnostics } = await import(
    './utils/tile-render-diagnostics'
  );
  console.log('🔍 AUTO-RUNNING: Tile render diagnostics...');
  await tileRenderDiagnostics.runAllDiagnostics();

  // Also run Three.js rendering diagnostics
  setTimeout(async () => {
    const { threeRenderDiagnostics } = await import(
      './utils/three-render-diagnostics'
    );
    console.log('🎨 AUTO-RUNNING: Three.js render diagnostics...');
    await threeRenderDiagnostics.runRenderingDiagnostics();

    // Run camera position diagnostic
    setTimeout(async () => {
      await import('./utils/camera-position-diagnostic');
      console.log('📷 AUTO-RUNNING: Camera position diagnostic...');
      (window as any).runCameraPositionDiagnostic();
    }, 500);
  }, 1000);
}, 3000);
AppLogger.info('Frontend main initialization completed successfully');
