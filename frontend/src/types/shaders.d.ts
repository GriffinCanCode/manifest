/**
 * TypeScript declarations for GLSL shader imports
 * Enables importing .glsl, .vert, .frag files as strings
 */

/// <reference types="vite-plugin-glsl/ext" />

declare module '*.glsl' {
  const source: string;
  export default source;
}

declare module '*.vert' {
  const source: string;
  export default source;
}

declare module '*.frag' {
  const source: string;
  export default source;
}

declare module '*.vs' {
  const source: string;
  export default source;
}

declare module '*.fs' {
  const source: string;
  export default source;
}

// Shader-specific types for our hex terrain system
export interface ShaderUniforms {
  [key: string]: {
    value: any;
    type?: string;
  };
}

export interface ShaderDefinition {
  name: string;
  vertexShader: string;
  fragmentShader: string;
  uniforms?: ShaderUniforms;
  defines?: Record<string, string | number>;
}

export interface TerrainShaderUniforms extends ShaderUniforms {
  // Note: Camera matrices and position are automatically provided by Three.js
  // u_viewMatrix, u_projectionMatrix, cameraPosition are built-in uniforms

  // Time and animation
  u_time: { value: number };
  u_deltaTime: { value: number };

  // Terrain properties
  u_heightScale: { value: number };
  u_hexSize: { value: number };
  u_hexSpacing: { value: number };

  // Rendering settings
  u_wireframe: { value: boolean };
  u_fogColor: { value: THREE.Color };
  u_fogNear: { value: number };
  u_fogFar: { value: number };

  // Texture maps
  u_heightMap?: { value: THREE.Texture };
  u_biomeMap?: { value: THREE.Texture };
  u_resourceMap?: { value: THREE.Texture };
}
