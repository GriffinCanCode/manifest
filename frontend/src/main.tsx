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
console.log('🚀 FRONTEND: Initializing logging system...');
console.log('🚀 FRONTEND: Current URL:', window.location.href);
console.log('🚀 FRONTEND: User Agent:', navigator.userAgent);
console.log('🚀 FRONTEND: Environment Mode:', import.meta.env.MODE);

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
console.log('🎨 FRONTEND: Initializing shader system...');
AppLogger.debug('Shader system initialization started');
initializeShaderSystem();
AppLogger.debug('Shader system initialization completed');

// Initialize React app
console.log('⚛️  FRONTEND: Starting React application...');
AppLogger.info('React application initialization started');
ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);

// Hide loading screen after initial render
setTimeout(() => {
  console.log('🎯 FRONTEND: Hiding loading screen');
  AppLogger.debug('Loading screen hidden, app fully loaded');
  hideLoadingScreen();
}, 1000);

console.log('✅ FRONTEND: Main initialization complete');
AppLogger.info('Frontend main initialization completed successfully');
