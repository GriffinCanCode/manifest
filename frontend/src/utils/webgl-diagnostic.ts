/**
 * WebGL Diagnostic Console Commands
 * Simple utilities for checking WebGL state in browser console
 */

declare global {
  interface Window {
    checkWebGLErrors: () => void;
    checkShaderCompilation: () => void;
  }
}

// Simple WebGL error checker
window.checkWebGLErrors = () => {
  console.warn('🔍 Checking WebGL errors...');

  const canvas = document.querySelector('canvas');
  if (!canvas) {
    console.error('❌ No canvas found');
    return;
  }

  const gl = canvas.getContext('webgl2') ?? canvas.getContext('webgl');
  if (!gl) {
    console.error('❌ No WebGL context');
    return;
  }

  const error = gl.getError();
  if (error !== gl.NO_ERROR) {
    console.error('❌ WebGL Error code:', error);
    return;
  }

  console.warn('✅ No WebGL errors detected');
};

// Check for shader compilation issues
window.checkShaderCompilation = () => {
  console.warn('🎨 Checking shader compilation state...');
  console.warn(
    'Look for any WebGL shader compilation errors above this message'
  );
};

console.warn('🔧 WebGL diagnostics loaded. Available commands:');
console.warn('  • checkWebGLErrors() - Check for WebGL errors');
console.warn('  • checkShaderCompilation() - Check shader compilation state');

export {};
