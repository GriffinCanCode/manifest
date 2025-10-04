/**
 * Hook for using procedural textures in components
 *
 * Provides easy access to procedural texture functionality with
 * automatic material creation and texture binding.
 */

import { useCallback, useEffect, useRef, useState } from 'react';
import type * as THREE from 'three';

import { useRendering } from '../components/rendering/hooks/rendering-hooks';
import type { MaterialDefinition } from '../services/texture-factory-service';

export interface UseProceduralTexturesProps {
  /** Biome type for material selection */
  biomeType?: string;
  /** Custom material ID to use */
  materialId?: string;
  /** Texture scale override */
  textureScale?: number;
  /** Whether to enable animations */
  enableAnimations?: boolean;
}

export interface UseProceduralTexturesReturn {
  /** Generated shader material */
  material: THREE.ShaderMaterial | null;
  /** Whether material is loading */
  isLoading: boolean;
  /** Material definition */
  materialDefinition: MaterialDefinition | null;
  /** Error if material failed to load */
  error: string | null;
  /** Function to update shader uniforms */
  updateUniforms: (uniforms: Record<string, unknown>) => void;
  /** Function to regenerate material */
  regenerateMaterial: () => Promise<void>;
}

/**
 * Hook for using procedural textures in components
 */
export const useProceduralTextures = ({
  biomeType,
  materialId,
  textureScale = 1.0,
  enableAnimations = true,
}: UseProceduralTexturesProps = {}): UseProceduralTexturesReturn => {
  const { textureService, isReady: isInitialized } = useRendering();

  const [material, setMaterial] = useState<THREE.ShaderMaterial | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [materialDefinition, setMaterialDefinition] =
    useState<MaterialDefinition | null>(null);
  const [error, setError] = useState<string | null>(null);

  const materialRef = useRef<THREE.ShaderMaterial | null>(null);

  // Determine material ID to use
  const targetMaterialId =
    materialId ?? (biomeType ? `terrain_${biomeType}` : 'terrain_grassland');

  // Update texture scale when prop changes
  useEffect(() => {
    if (material?.uniforms?.u_textureScale) {
      material.uniforms.u_textureScale.value = textureScale;
    }
  }, [material, textureScale]);

  // Enable/disable animations
  useEffect(() => {
    if (material?.uniforms?.u_animationSpeed) {
      // Get material definition to determine if it should animate
      const definition = textureService
        .getMaterialsByCategory('terrain')
        .find(m => m.id === targetMaterialId);

      if (definition && enableAnimations) {
        material.uniforms.u_animationSpeed.value = definition.animation_speed;
      } else {
        material.uniforms.u_animationSpeed.value = 0;
      }
    }
  }, [material, enableAnimations, targetMaterialId, textureService]);

  const loadMaterial = useCallback(async (): Promise<void> => {
    try {
      setIsLoading(true);
      setError(null);

      // Get material definition first
      const definition = textureService
        .getMaterialsByCategory('terrain')
        .find(m => m.id === targetMaterialId);

      if (!definition) {
        throw new Error(`Material definition not found: ${targetMaterialId}`);
      }

      setMaterialDefinition(definition);

      // Create material
      const newMaterial = await textureService.createMaterial(targetMaterialId);

      // Apply custom texture scale
      if (newMaterial.uniforms.u_textureScale) {
        newMaterial.uniforms.u_textureScale.value = textureScale;
      }

      // Set animation speed
      if (newMaterial.uniforms.u_animationSpeed) {
        newMaterial.uniforms.u_animationSpeed.value = enableAnimations
          ? definition.animation_speed
          : 0;
      }

      // Dispose of previous material
      if (materialRef.current) {
        materialRef.current.dispose();
      }

      materialRef.current = newMaterial;
      setMaterial(newMaterial);

      console.warn(`🎭 Loaded procedural material: ${targetMaterialId}`);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Unknown error';
      setError(errorMessage);
      console.error(`❌ Failed to load material ${targetMaterialId}:`, err);
    } finally {
      setIsLoading(false);
    }
  }, [targetMaterialId, textureService, textureScale, enableAnimations]);

  // Load material when service is ready
  useEffect(() => {
    if (!isInitialized || !targetMaterialId) {
      return;
    }

    void loadMaterial();
  }, [isInitialized, targetMaterialId, loadMaterial]);

  const updateUniforms = (uniforms: Record<string, unknown>): void => {
    if (!material) return;

    for (const [key, value] of Object.entries(uniforms)) {
      if (material.uniforms[key]) {
        material.uniforms[key].value = value;
      }
    }
  };

  const regenerateMaterial = async (): Promise<void> => {
    if (!targetMaterialId) return;

    try {
      // Regenerate texture on backend
      await textureService.regenerateTexture(targetMaterialId);

      // Clear from cache and reload
      textureService.clearCache();
      await loadMaterial();
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Unknown error';
      setError(errorMessage);
      throw err;
    }
  };

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      if (materialRef.current) {
        materialRef.current.dispose();
      }
    };
  }, []);

  return {
    material,
    isLoading,
    materialDefinition,
    error,
    updateUniforms,
    regenerateMaterial,
  };
};

/**
 * Hook for binding procedural textures to existing shader materials
 */
export const useTextureBinding = (
  material: THREE.ShaderMaterial | null,
  textureId: string
): {
  isLoading: boolean;
  error: string | null;
  bindTexture: () => Promise<void>;
} => {
  const { textureService, isReady: isInitialized } = useRendering();
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const bindTexture = async (): Promise<void> => {
    if (!material || !isInitialized || !textureId) {
      return;
    }

    try {
      setIsLoading(true);
      setError(null);

      const texture = await textureService.loadTexture(textureId);

      // Bind to appropriate uniform based on texture type
      if (textureId.includes('_albedo') && material.uniforms.u_albedoTexture) {
        material.uniforms.u_albedoTexture.value = texture;
        material.uniforms.u_hasAlbedoTexture.value = true;
      } else if (
        textureId.includes('_normal') &&
        material.uniforms.u_normalTexture
      ) {
        material.uniforms.u_normalTexture.value = texture;
        material.uniforms.u_hasNormalTexture.value = true;
      } else if (
        textureId.includes('_roughness') &&
        material.uniforms.u_roughnessTexture
      ) {
        material.uniforms.u_roughnessTexture.value = texture;
        material.uniforms.u_hasRoughnessTexture.value = true;
      } else if (
        textureId.includes('_metallic') &&
        material.uniforms.u_metallicTexture
      ) {
        material.uniforms.u_metallicTexture.value = texture;
        material.uniforms.u_hasMetallicTexture.value = true;
      }

      material.needsUpdate = true;
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Unknown error';
      setError(errorMessage);
      throw err;
    } finally {
      setIsLoading(false);
    }
  };

  return {
    isLoading,
    error,
    bindTexture,
  };
};
