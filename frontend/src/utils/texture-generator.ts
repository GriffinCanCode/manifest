/**
 * Utility functions for generating textures programmatically
 */

import { textureService } from '../services/texture-factory-service';

/**
 * Generate all textures with specific options
 */
export async function generateAllTextures(options?: {
  resolution?: number;
  clearCacheFirst?: boolean;
  generateNormals?: boolean;
  generateMaterials?: boolean;
}): Promise<void> {
  const {
    resolution = 512,
    clearCacheFirst = true,
    generateNormals = true,
    generateMaterials = true,
  } = options || {};

  try {
    console.log('🎨 Starting texture generation...');

    // Clear cache to force regeneration
    if (clearCacheFirst) {
      console.log('🗑️ Clearing texture cache...');
      textureService.clearCache();
    }

    // Generate all textures
    const result = await textureService.generateTextures({
      resolution,
      generate_normals: generateNormals,
      generate_materials: generateMaterials,
      generate_atlases: true,
    });

    if (result.success) {
      console.log(
        `✅ Successfully generated ${result.texture_count} textures!`
      );
      console.log(`📊 Texture metadata:`, JSON.parse(result.texture_metadata));
    } else {
      console.error('❌ Texture generation failed:', result.error);
    }
  } catch (error) {
    console.error('❌ Texture generation error:', error);
    throw error;
  }
}

/**
 * Regenerate specific biome textures
 */
export async function regenerateBiomeTextures(biomes: string[]): Promise<void> {
  console.log('🔄 Regenerating specific biome textures:', biomes);

  for (const biome of biomes) {
    try {
      await textureService.regenerateTexture(`biome_${biome}`);
      console.log(`✅ Regenerated ${biome} texture`);
    } catch (error) {
      console.error(`❌ Failed to regenerate ${biome}:`, error);
    }
  }
}

/**
 * Generate high-resolution textures for production
 */
export async function generateHighResTextures(): Promise<void> {
  await generateAllTextures({
    resolution: 2048, // High resolution
    clearCacheFirst: true,
    generateNormals: true,
    generateMaterials: true,
  });
}

/**
 * Quick texture regeneration for development
 */
export async function quickRegenerate(): Promise<void> {
  await generateAllTextures({
    resolution: 512, // Lower resolution for speed
    clearCacheFirst: true,
    generateNormals: false, // Skip normals for speed
    generateMaterials: true,
  });
}
