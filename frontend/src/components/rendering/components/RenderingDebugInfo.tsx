/**
 * Development helper component for rendering statistics
 * Separated for Fast Refresh compatibility
 */

import React from 'react';

import { useRenderStore } from '../../../stores/render-store';
import { useRendering } from '../hooks/rendering-hooks';

export const RenderingDebugInfo: React.FC = () => {
  const { stats, isReady } = useRendering();
  const { devMode } = useRenderStore();

  if (!devMode || !isReady) return null;

  return (
    <div
      style={{
        position: 'fixed',
        top: 10,
        right: 10,
        background: 'rgba(0,0,0,0.8)',
        color: 'white',
        padding: '12px',
        fontSize: '11px',
        fontFamily: 'monospace',
        borderRadius: '6px',
        zIndex: 1000,
        minWidth: '200px',
      }}
    >
      <div
        style={{ fontWeight: 'bold', marginBottom: '8px', color: '#4ade80' }}
      >
        🎨 Rendering System
      </div>

      <div style={{ marginBottom: '6px' }}>
        <div style={{ color: '#60a5fa' }}>📦 Materials</div>
        <div>
          Cached: {stats.materials.cached} | Size: {stats.materials.cacheSize}
        </div>
        <div>
          Compiled: {stats.materials.compiled} | Textured:{' '}
          {stats.materials.textured}
        </div>
        <div>Fallback: {stats.materials.fallback}</div>
      </div>

      <div style={{ marginBottom: '6px' }}>
        <div style={{ color: '#f59e0b' }}>🖼️ Textures</div>
        <div>Loaded: {stats.textures.texturesLoaded}</div>
        <div>Materials: {stats.textures.materialsCreated}</div>
      </div>

      <div>
        <div style={{ color: '#ec4899' }}>⚡ Uniforms</div>
        <div>
          Active: {stats.uniforms.activeCount}/{stats.uniforms.registered}
        </div>
        <div>
          Updated: {stats.uniforms.updated} | Errors: {stats.uniforms.errors}
        </div>
      </div>
    </div>
  );
};
