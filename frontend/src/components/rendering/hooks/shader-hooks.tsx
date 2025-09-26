/**
 * Shader hooks and utility components
 * Separated from ShaderProvider for Fast Refresh compatibility
 */

import React, { useContext } from 'react';
import type { ShaderMaterial } from 'three';

import type { ShaderName } from '../../../shaders/definitions';
import { shaderManager } from '../../../shaders/manager';
import { useRenderStore } from '../../../stores/render-store';
import type { ShaderUniforms } from '../../../types/shaders';

interface ShaderContextType {
  getShader: (name: ShaderName) => ShaderMaterial | null;
  updateShaderUniforms: (
    name: ShaderName,
    uniforms: Partial<ShaderUniforms>
  ) => void;
  isReady: boolean;
}

// This will be created in ShaderProvider
export const ShaderContext = React.createContext<ShaderContextType | null>(
  null
);

/**
 * Hook to access shader context
 */
export const useShaders = (): ShaderContextType => {
  const context = useContext(ShaderContext);
  if (!context) {
    throw new Error('useShaders must be used within a ShaderProvider');
  }
  return context;
};

/**
 * Hook for accessing specific shader
 */
export const useShader = (name: ShaderName): ShaderMaterial | null => {
  const { getShader } = useShaders();
  return getShader(name);
};

/**
 * Development helper component
 */
export const ShaderDebugInfo: React.FC = () => {
  const { isReady } = useShaders();
  const { devMode } = useRenderStore();

  if (!devMode || !isReady) return null;

  return (
    <div
      style={{
        position: 'fixed',
        top: 10,
        right: 10,
        background: 'rgba(0,0,0,0.7)',
        color: 'white',
        padding: '8px',
        fontSize: '12px',
        fontFamily: 'monospace',
        borderRadius: '4px',
        zIndex: 1000,
      }}
    >
      <div>🎨 Shader System: Active</div>
      <div>📊 Compiled Shaders: {shaderManager.getStats().cacheSize}</div>
      <div>🔥 Hot Reload: {shaderManager.getStats().hotReloadCount}</div>
    </div>
  );
};
