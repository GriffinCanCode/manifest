/**
 * Rendering contexts - separated for Fast Refresh compatibility
 */

import React from 'react';
import type * as THREE from 'three';

import type { MaterialService } from '../../../services/materials';
import type { TextureFactoryService } from '../../../services/texture-factory-service';
import type { UniformService } from '../../../services/uniforms';
import type { TerrainType } from '../../../utils/game-types';

export interface MaterialConfig {
  terrainType: TerrainType;
  texture?: THREE.Texture;
  useShader?: boolean;
  wireframe?: boolean;
}

export interface RenderingStats {
  materials: {
    cached: number;
    compiled: number;
    textured: number;
    fallback: number;
    cacheSize: number;
  };
  textures: {
    texturesLoaded: number;
    materialsCreated: number;
    cacheSize: number;
  };
  uniforms: {
    registered: number;
    updated: number;
    skipped: number;
    errors: number;
    activeCount: number;
  };
}

export interface RenderingContextType {
  // Services
  materialService: MaterialService;
  textureService: TextureFactoryService;
  uniformService: UniformService;

  // State
  isReady: boolean;
  isGenerating: boolean;
  stats: RenderingStats;

  // Material operations
  getMaterial: (config: MaterialConfig) => THREE.Material;
  getTerrainMaterial: (
    terrainType: TerrainType,
    texture?: THREE.Texture,
    wireframe?: boolean
  ) => THREE.Material;

  // Texture operations
  generateTextures: () => Promise<void>;

  // Cleanup
  clearCache: () => void;
}

export const RenderingContext =
  React.createContext<RenderingContextType | null>(null);
