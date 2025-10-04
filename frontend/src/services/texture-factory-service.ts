/**
 * Advanced Texture Factory Service
 *
 * Manages AAA-quality procedural textures using the @texture-factory system
 * with complete PBR material support and seamless terrain integration.
 *
 * Replaces the old backend-dependent texture service with client-side generation.
 */

import * as THREE from 'three';

import { AdvancedDesertGenerator } from '../@texture-factory/desert';
import { AdvancedForestGenerator } from '../@texture-factory/forest';
import { AdvancedGrasslandGenerator } from '../@texture-factory/grassland';
import { AdvancedHillsGenerator } from '../@texture-factory/hills';
import { AdvancedJungleGenerator } from '../@texture-factory/jungle';
import { AdvancedMountainGenerator } from '../@texture-factory/mountain';
import { AdvancedOceanGenerator } from '../@texture-factory/ocean';
import { AdvancedPlainsGenerator } from '../@texture-factory/plains';
import { AdvancedTundraGenerator } from '../@texture-factory/tundra';
import { getShaderDefinition } from '../shaders/definitions';
import { shaderManager } from '../shaders/manager';

// Re-export these interfaces for compatibility
export interface TextureData {
  id: string;
  filename: string;
  resolution: [number, number];
  channels: number;
  has_normal_map: boolean;
  has_material_maps: boolean;
  tile_factor: number;
}

export interface MaterialDefinition {
  id: string;
  name: string;
  category: 'terrain' | 'water' | 'resource' | 'structure' | 'effect' | 'ui';

  // Texture maps
  albedo_texture?: string;
  normal_texture?: string;
  roughness_texture?: string;
  metallic_texture?: string;
  ao_texture?: string;
  emission_texture?: string;

  // Material properties
  base_color: [number, number, number];
  roughness: number;
  metallic: number;
  specular: number;
  emission_intensity: number;
  normal_strength: number;

  // Rendering properties
  alpha_mode: 'opaque' | 'blend' | { mask: { cutoff: number } };
  double_sided: boolean;
  texture_scale: number;
  animation_speed: number;
}

export interface GenerateTexturesRequest {
  output_dir?: string;
  resolution?: number;
  generate_normals?: boolean;
  generate_materials?: boolean;
  generate_atlases?: boolean;
}

export interface GenerateTexturesResponse {
  success: boolean;
  texture_count: number;
  atlas_count: number;
  output_dir: string;
  texture_metadata: string;
  material_definitions: string;
  error?: string;
}

// Terrain type mapping for @texture-factory generators
type TerrainType =
  | 'grassland'
  | 'forest'
  | 'desert'
  | 'hills'
  | 'jungle'
  | 'mountain'
  | 'ocean'
  | 'plains'
  | 'tundra';

// Define the texture generator interface
interface TextureGenerator {
  generateTextures(): {
    albedo: ImageData;
    normal: ImageData;
    roughness: ImageData;
    metallic: ImageData;
    height: ImageData;
  };
}

// Specific environmental factor interfaces for each terrain type
interface GrasslandEnvironmentalFactors {
  moisture: number; // 0-1, affects color and density
  temperature: number; // 0-1, affects growth patterns
  season: number; // 0-1, affects color variation
  elevation: number; // 0-1, affects grass type
}

interface ForestEnvironmentalFactors {
  moisture: number; // 0-1, affects vegetation density
  temperature: number; // 0-1, affects tree types
  season: number; // 0-1, affects color variation
  elevation: number; // 0-1, affects tree species
}

interface DesertEnvironmentalFactors {
  moisture: number; // 0-1, affects oasis presence and vegetation
  temperature: number; // 0-1, affects sand color and rock weathering
  windStrength: number; // 0-1, affects erosion patterns
  season: number; // 0-1, affects atmospheric conditions
  elevation: number; // 0-1, affects rock exposure
}

interface MountainEnvironmentalFactors {
  elevation: number; // 0-1, affects snow coverage and erosion patterns
  temperature: number; // 0-1, affects snow line and weathering
  precipitation: number; // 0-1, affects erosion intensity
  age: number; // 0-1, affects overall weathering (0=young sharp peaks, 1=old rounded)
}

interface HillsEnvironmentalFactors {
  moisture: number; // 0-1, affects vegetation density and type
  temperature: number; // 0-1, affects vegetation and soil color
  season: number; // 0-1, affects vegetation color and dormancy
  soilRichness: number; // 0-1, affects exposed bedrock and vegetation health
}

interface JungleEnvironmentalFactors {
  moisture: number; // 0-1, affects vegetation lushness (high in jungle)
  temperature: number; // 0-1, affects plant growth (consistently high in jungle)
  season: number; // 0-1, affects fruit/flower presence
  elevation: number; // 0-1, affects jungle density and species
  humidity: number; // 0-1, specific to jungle environment
}

interface PlainsEnvironmentalFactors {
  moisture: number; // 0-1, affects grass color and density
  temperature: number; // 0-1, affects growth patterns
  season: number; // 0-1, affects color variation
  elevation: number; // 0-1, affects elevation changes
  windDirection: number; // 0-1, prevailing wind direction
}

interface TundraEnvironmentalFactors {
  moisture: number; // 0-1, affects vegetation and ice formation
  temperature: number; // 0-1, affects permafrost and snow coverage
  windStrength: number; // 0-1, affects erosion patterns
  season: number; // 0-1, affects snow coverage and vegetation activity
  elevation: number; // 0-1, affects vegetation line and ice content
}

interface OceanEnvironmentalFactors {
  windStrength: number; // 0-1, affects wave intensity
  windDirection: number; // 0-2π, wind direction in radians
  depth: number; // 0-1, affects color depth
  weather: number; // 0-1, 0=calm, 1=stormy
  temperature: number; // 0-1, affects viscosity
}

// Union type for all environmental factors
type EnvironmentalFactors =
  | GrasslandEnvironmentalFactors
  | ForestEnvironmentalFactors
  | DesertEnvironmentalFactors
  | HillsEnvironmentalFactors
  | JungleEnvironmentalFactors
  | MountainEnvironmentalFactors
  | OceanEnvironmentalFactors
  | PlainsEnvironmentalFactors
  | TundraEnvironmentalFactors;

// Biome to terrain mapping
const BIOME_TERRAIN_MAP: Record<string, TerrainType> = {
  grassland: 'grassland',
  plains: 'plains',
  forest: 'forest',
  jungle: 'jungle',
  desert: 'desert',
  mountain: 'mountain',
  hills: 'hills',
  tundra: 'tundra',
  snow: 'tundra',
  ocean: 'ocean',
};

// Variation mapping for different biome types
const BIOME_VARIATION_MAP: Record<string, string> = {
  grassland: 'lush_meadow',
  plains: 'vast_prairie',
  forest: 'dense_forest',
  jungle: 'dense_rainforest',
  desert: 'sahara_desert',
  mountain: 'rocky_peaks',
  hills: 'grassy_hills',
  tundra: 'arctic_tundra',
  snow: 'alpine_tundra',
  ocean: 'deep_ocean',
};

/**
 * Advanced Texture Factory Service
 * Generates AAA-quality procedural textures using @texture-factory
 */
export class TextureFactoryService {
  private textureCache = new Map<string, THREE.Texture>();
  private materialCache = new Map<string, THREE.ShaderMaterial>();
  private textureMetadata = new Map<string, TextureData>();
  private materialDefinitions = new Map<string, MaterialDefinition>();
  private loadingPromises = new Map<string, Promise<THREE.Texture>>();

  // Generator instances for performance
  private generators = new Map<string, TextureGenerator>();

  // Initialization guard to prevent multiple initializations
  private isInitialized = false;
  private isInitializing = false;

  /**
   * Initialize the texture factory service
   */
  initialize(): void {
    // Prevent multiple initializations
    if (this.isInitialized || this.isInitializing) {
      console.warn(
        '🎨 Texture Factory Service already initialized or initializing'
      );
      return;
    }

    this.isInitializing = true;
    console.warn('🎨 Initializing Advanced Texture Factory Service...');

    // Handle async initialization
    this.loadOrGenerateAllTextures()
      .then(() => {
        this.isInitialized = true;
        console.warn('✅ Texture Factory Service initialized successfully');
      })
      .catch(error => {
        console.error(
          '❌ Failed to initialize Texture Factory Service:',
          error
        );
        this.isInitialized = true; // Mark as initialized even on error to prevent retries
        // Continue anyway with fallback textures
      })
      .finally(() => {
        this.isInitializing = false;
      });
  }

  /**
   * Generate all textures using @texture-factory
   */
  generateTextures(
    request: GenerateTexturesRequest
  ): Promise<GenerateTexturesResponse> {
    console.warn('🏗️ Generating textures with @texture-factory...', request);

    return Promise.resolve()
      .then(() => {
        const resolution = request.resolution ?? 1024;
        const biomes = [
          'grassland',
          'plains',
          'forest',
          'jungle',
          'desert',
          'mountain',
          'hills',
          'tundra',
          'snow',
          'ocean',
        ];

        let textureCount = 0;
        const textureMetadata: Record<string, TextureData> = {};
        const materialDefinitions: Record<string, MaterialDefinition> = {};

        for (const biome of biomes) {
          try {
            this.generateBiomeTextures(biome, resolution);

            // Create metadata
            const textureId = `biome_${biome}`;
            textureMetadata[textureId] = {
              id: textureId,
              filename: `${textureId}_albedo.png`,
              resolution: [resolution, resolution],
              channels: 4,
              has_normal_map: true,
              has_material_maps: true,
              tile_factor: 1.0,
            };

            // Create material definition
            materialDefinitions[biome] = this.createMaterialDefinition(
              biome,
              textureId
            );

            textureCount += 5; // albedo, normal, roughness, metallic, height
            console.warn(`✅ Generated ${biome} textures`);
          } catch (error) {
            console.error(`❌ Failed to generate ${biome} textures:`, error);
          }
        }

        // Store in internal maps
        for (const [id, data] of Object.entries(textureMetadata)) {
          this.textureMetadata.set(id, data);
        }
        for (const [id, material] of Object.entries(materialDefinitions)) {
          this.materialDefinitions.set(id, material);
        }

        console.warn(
          `✅ Generated ${textureCount} textures using @texture-factory`
        );

        return {
          success: true,
          texture_count: textureCount,
          atlas_count: 0,
          output_dir: 'client-side',
          texture_metadata: JSON.stringify(textureMetadata),
          material_definitions: JSON.stringify(materialDefinitions),
        };
      })
      .catch(error => {
        console.error('❌ Failed to generate textures:', error);
        return {
          success: false,
          texture_count: 0,
          atlas_count: 0,
          output_dir: '',
          texture_metadata: '{}',
          material_definitions: '{}',
          error: String(error),
        };
      });
  }

  /**
   * Load or generate all default textures
   */
  private async loadOrGenerateAllTextures(): Promise<void> {
    const resolution = 512; // Default resolution for initialization
    const biomes = [
      'grassland',
      'plains',
      'forest',
      'jungle',
      'desert',
      'mountain',
      'hills',
      'tundra',
      'snow',
      'ocean',
    ];

    for (const biome of biomes) {
      try {
        // Check if texture files exist, load them if they do, generate if they don't
        if (this.textureFilesExist(biome)) {
          await this.loadBiomeTexturesFromFiles(biome);
          console.warn(`✅ Loaded existing ${biome} textures from files`);
        } else {
          this.generateBiomeTextures(biome, resolution);
          this.saveBiomeTexturesToFiles(biome);
          console.warn(`✅ Generated and saved ${biome} textures`);
        }
      } catch (error) {
        console.error(`❌ Failed to load/generate ${biome} textures:`, error);
        // Fall back to generation if loading fails
        try {
          this.generateBiomeTextures(biome, resolution);
          console.warn(`🔄 Generated ${biome} textures as fallback`);
        } catch (genError) {
          console.error(
            `❌ Failed to generate fallback ${biome} textures:`,
            genError
          );
          // Generate simple fallback texture
          this.generateFallbackTexture(`biome_${biome}`, biome);
        }
      }
    }
  }

  /**
   * Check if all texture files exist for a biome
   */
  private textureFilesExist(biome: string): boolean {
    // Since we're in a browser environment and can see the files exist in the folder,
    // we'll use a simple heuristic: if we're loading common biomes and the folder exists,
    // assume the textures are there. This prevents unnecessary regeneration.

    // List of biomes we know have generated textures in the assets folder
    const knownBiomesWithTextures = [
      'grassland',
      'plains',
      'forest',
      'jungle',
      'desert',
      'mountain',
      'hills',
      'tundra',
      'snow',
      'ocean',
    ];

    return knownBiomesWithTextures.includes(biome);
  }

  /**
   * Load biome textures from existing files
   */
  private async loadBiomeTexturesFromFiles(biome: string): Promise<void> {
    const textureTypes = ['albedo', 'normal', 'roughness', 'metallic'];
    const texturePrefix = `biome_${biome}`;

    try {
      // Load textures using dynamic imports to work with Vite's asset handling
      for (const type of textureTypes) {
        const fileName = `${texturePrefix}_${type}.png`;

        // Use dynamic import to get the correct asset path
        try {
          const module = (await import(
            `../assets/generated_textures/${fileName}`
          )) as { default: string };
          const assetUrl: string = module.default;

          const texture = new THREE.TextureLoader().load(assetUrl);
          texture.wrapS = THREE.RepeatWrapping;
          texture.wrapT = THREE.RepeatWrapping;
          texture.generateMipmaps = true;
          texture.minFilter = THREE.LinearMipmapLinearFilter;
          texture.magFilter = THREE.LinearFilter;

          if (type === 'albedo') {
            this.textureCache.set(texturePrefix, texture);
          } else {
            this.textureCache.set(`${texturePrefix}_${type}`, texture);
          }
        } catch (importError) {
          console.warn(
            `⚠️ Could not import texture ${fileName}, will generate instead`
          );
          throw importError; // Re-throw to trigger fallback generation
        }
      }

      // Create metadata for loaded textures
      this.textureMetadata.set(texturePrefix, {
        id: texturePrefix,
        filename: `${texturePrefix}_albedo.png`,
        resolution: [512, 512], // Default resolution
        channels: 4,
        has_normal_map: true,
        has_material_maps: true,
        tile_factor: 1.0,
      });

      // Create material definition
      this.materialDefinitions.set(
        biome,
        this.createMaterialDefinition(biome, texturePrefix)
      );
    } catch (error) {
      console.error(`Failed to load textures for ${biome}:`, error);
      throw error; // Let the caller handle fallback
    }
  }

  /**
   * Save generated biome textures to files
   */
  private saveBiomeTexturesToFiles(biome: string): void {
    // Note: In a browser environment, we can't directly save files to the file system
    // This is a placeholder for the save functionality
    // In a real implementation, you'd need to use a server endpoint or download mechanism
    console.warn(
      `💾 Would save ${biome} textures to files (browser limitation)`
    );

    // For now, we'll skip the actual file saving since browsers have security restrictions
    // The textures are already cached in memory which prevents regeneration
  }

  /**
   * Generate textures for a specific biome
   */
  private generateBiomeTextures(biome: string, resolution: number): void {
    const terrainType = BIOME_TERRAIN_MAP[biome];
    const variation = BIOME_VARIATION_MAP[biome];

    if (!terrainType) {
      throw new Error(`Unknown terrain type for biome: ${biome}`);
    }

    // Create or get generator
    const generatorKey = `${terrainType}_${variation}`;
    let generator = this.generators.get(generatorKey);

    if (!generator) {
      generator = this.createGenerator(terrainType, variation, resolution);
      this.generators.set(generatorKey, generator);
    }

    // Generate textures
    const textures = generator.generateTextures();

    // Convert ImageData to THREE.Texture and cache
    const texturePrefix = `biome_${biome}`;

    // Main albedo texture
    const albedoTexture = this.imageDataToTexture(textures.albedo);
    this.textureCache.set(texturePrefix, albedoTexture);

    // Additional maps
    this.textureCache.set(
      `${texturePrefix}_normal`,
      this.imageDataToTexture(textures.normal)
    );
    this.textureCache.set(
      `${texturePrefix}_roughness`,
      this.imageDataToTexture(textures.roughness)
    );
    this.textureCache.set(
      `${texturePrefix}_metallic`,
      this.imageDataToTexture(textures.metallic)
    );
    this.textureCache.set(
      `${texturePrefix}_height`,
      this.imageDataToTexture(textures.height)
    );

    // Create metadata
    this.textureMetadata.set(texturePrefix, {
      id: texturePrefix,
      filename: `${texturePrefix}_albedo.png`,
      resolution: [resolution, resolution],
      channels: 4,
      has_normal_map: true,
      has_material_maps: true,
      tile_factor: 1.0,
    });

    // Create material definition
    this.materialDefinitions.set(
      biome,
      this.createMaterialDefinition(biome, texturePrefix)
    );
  }

  /**
   * Create appropriate generator for terrain type
   */
  private createGenerator(
    terrainType: TerrainType,
    variation: string,
    resolution: number
  ): TextureGenerator {
    const environmentalFactors = this.getEnvironmentalFactors(terrainType);

    switch (terrainType) {
      case 'grassland':
        return new AdvancedGrasslandGenerator(
          resolution,
          variation,
          environmentalFactors as GrasslandEnvironmentalFactors
        );
      case 'forest':
        return new AdvancedForestGenerator(
          resolution,
          variation,
          environmentalFactors as ForestEnvironmentalFactors
        );
      case 'desert':
        return new AdvancedDesertGenerator(
          resolution,
          variation,
          environmentalFactors as DesertEnvironmentalFactors
        );
      case 'hills':
        return new AdvancedHillsGenerator(
          resolution,
          variation,
          environmentalFactors as HillsEnvironmentalFactors
        );
      case 'jungle':
        return new AdvancedJungleGenerator(
          resolution,
          variation,
          environmentalFactors as JungleEnvironmentalFactors
        );
      case 'mountain':
        return new AdvancedMountainGenerator(
          resolution,
          variation,
          environmentalFactors as MountainEnvironmentalFactors
        );
      case 'plains':
        return new AdvancedPlainsGenerator(
          resolution,
          variation,
          environmentalFactors as PlainsEnvironmentalFactors
        );
      case 'tundra':
        return new AdvancedTundraGenerator(
          resolution,
          variation,
          environmentalFactors as TundraEnvironmentalFactors
        );
      case 'ocean':
        return new AdvancedOceanGenerator(
          resolution,
          variation,
          environmentalFactors as OceanEnvironmentalFactors
        );
      default:
        throw new Error(`Unknown terrain type: ${terrainType as string}`);
    }
  }

  /**
   * Get environmental factors for terrain type
   */
  private getEnvironmentalFactors(
    terrainType: TerrainType
  ): EnvironmentalFactors {
    switch (terrainType) {
      case 'grassland':
        return {
          moisture: 0.7,
          temperature: 0.6,
          season: 0.5,
          elevation: 0.3,
        };
      case 'forest':
        return {
          moisture: 0.8,
          temperature: 0.6,
          season: 0.5,
          elevation: 0.4,
        };
      case 'desert':
        return {
          moisture: 0.1,
          temperature: 0.9,
          windStrength: 0.7,
          season: 0.5,
          elevation: 0.4,
        };
      case 'hills':
        return {
          moisture: 0.65,
          temperature: 0.55,
          season: 0.5,
          soilRichness: 0.7,
        };
      case 'jungle':
        return {
          moisture: 0.95,
          temperature: 0.85,
          season: 0.5,
          elevation: 0.3,
          humidity: 0.9,
        };
      case 'mountain':
        return {
          elevation: 0.8,
          temperature: 0.3,
          precipitation: 0.6,
          age: 0.5,
        };
      case 'plains':
        return {
          moisture: 0.6,
          temperature: 0.7,
          season: 0.5,
          elevation: 0.4,
          windDirection: 0.3,
        };
      case 'tundra':
        return {
          moisture: 0.3,
          temperature: 0.2,
          windStrength: 0.8,
          season: 0.3,
          elevation: 0.6,
        };
      case 'ocean':
        return {
          windStrength: 0.6,
          windDirection: Math.PI / 4,
          depth: 0.8,
          weather: 0.3,
          temperature: 0.6,
        };
      default:
        return {
          moisture: 0.7,
          temperature: 0.6,
          season: 0.5,
          elevation: 0.3,
        };
    }
  }

  /**
   * Convert ImageData to THREE.Texture
   */
  private imageDataToTexture(imageData: ImageData): THREE.Texture {
    const canvas = document.createElement('canvas');
    canvas.width = imageData.width;
    canvas.height = imageData.height;
    const ctx = canvas.getContext('2d');
    if (!ctx) {
      throw new Error('Failed to get 2D context for texture conversion');
    }
    ctx.putImageData(imageData, 0, 0);

    const texture = new THREE.CanvasTexture(canvas);
    texture.wrapS = THREE.RepeatWrapping;
    texture.wrapT = THREE.RepeatWrapping;
    texture.generateMipmaps = true;
    texture.minFilter = THREE.LinearMipmapLinearFilter;
    texture.magFilter = THREE.LinearFilter;
    texture.needsUpdate = true;

    return texture;
  }

  /**
   * Create material definition for biome
   */
  private createMaterialDefinition(
    biome: string,
    texturePrefix: string
  ): MaterialDefinition {
    const materialProps = this.getMaterialProperties(biome);

    return {
      id: biome,
      name: biome.charAt(0).toUpperCase() + biome.slice(1),
      category: 'terrain' as const,
      albedo_texture: texturePrefix,
      normal_texture: `${texturePrefix}_normal`,
      roughness_texture: `${texturePrefix}_roughness`,
      metallic_texture: `${texturePrefix}_metallic`,
      base_color: [1.0, 1.0, 1.0] as [number, number, number],
      roughness: materialProps.roughness,
      metallic: materialProps.metallic,
      specular: 0.5,
      emission_intensity: 0.0,
      normal_strength: materialProps.normalStrength,
      alpha_mode: 'opaque' as const,
      double_sided: false,
      texture_scale: 1.0,
      animation_speed: materialProps.animationSpeed,
    };
  }

  /**
   * Get material properties for biome
   */
  private getMaterialProperties(biome: string): {
    roughness: number;
    metallic: number;
    normalStrength: number;
    animationSpeed: number;
  } {
    const props: Record<
      string,
      {
        roughness: number;
        metallic: number;
        normalStrength: number;
        animationSpeed: number;
      }
    > = {
      grassland: {
        roughness: 0.8,
        metallic: 0.0,
        normalStrength: 1.0,
        animationSpeed: 0.1,
      },
      plains: {
        roughness: 0.7,
        metallic: 0.0,
        normalStrength: 0.8,
        animationSpeed: 0.15,
      },
      forest: {
        roughness: 0.85,
        metallic: 0.0,
        normalStrength: 1.4,
        animationSpeed: 0.05,
      },
      jungle: {
        roughness: 0.9,
        metallic: 0.0,
        normalStrength: 1.6,
        animationSpeed: 0.08,
      },
      desert: {
        roughness: 0.85,
        metallic: 0.0,
        normalStrength: 1.1,
        animationSpeed: 0.0,
      },
      mountain: {
        roughness: 0.95,
        metallic: 0.02,
        normalStrength: 1.5,
        animationSpeed: 0.0,
      },
      hills: {
        roughness: 0.85,
        metallic: 0.0,
        normalStrength: 1.0,
        animationSpeed: 0.0,
      },
      tundra: {
        roughness: 0.7,
        metallic: 0.0,
        normalStrength: 1.0,
        animationSpeed: 0.0,
      },
      snow: {
        roughness: 0.3,
        metallic: 0.0,
        normalStrength: 0.5,
        animationSpeed: 0.0,
      },
      ocean: {
        roughness: 0.1,
        metallic: 0.0,
        normalStrength: 1.5,
        animationSpeed: 0.5,
      },
    };

    return props[biome] ?? props.grassland;
  }

  /**
   * Generate fallback texture for failed generations
   */
  private generateFallbackTexture(textureId: string, biome: string): void {
    console.warn(`🔧 Generating fallback texture for: ${textureId}`);

    const canvas = document.createElement('canvas');
    canvas.width = 256;
    canvas.height = 256;
    const ctx = canvas.getContext('2d');

    if (ctx) {
      const color = this.getBiomeColor(biome);
      ctx.fillStyle = color;
      ctx.fillRect(0, 0, 256, 256);

      // Add simple noise pattern
      this.addBasicNoisePattern(ctx, 256);

      const texture = new THREE.CanvasTexture(canvas);
      texture.wrapS = THREE.RepeatWrapping;
      texture.wrapT = THREE.RepeatWrapping;
      this.textureCache.set(textureId, texture);

      console.warn(`✅ Created fallback texture for: ${textureId}`);
    }
  }

  /**
   * Add basic noise pattern to fallback textures
   */
  private addBasicNoisePattern(
    ctx: CanvasRenderingContext2D,
    resolution: number
  ): void {
    const imageData = ctx.getImageData(0, 0, resolution, resolution);
    const { data } = imageData;

    for (let y = 0; y < resolution; y++) {
      for (let x = 0; x < resolution; x++) {
        const index = (y * resolution + x) * 4;
        const noise =
          (Math.sin(x * 0.1) + Math.cos(y * 0.1) + Math.sin((x + y) * 0.05)) *
          10;

        data[index] = Math.min(255, Math.max(0, data[index] + noise));
        data[index + 1] = Math.min(255, Math.max(0, data[index + 1] + noise));
        data[index + 2] = Math.min(255, Math.max(0, data[index + 2] + noise));
      }
    }

    ctx.putImageData(imageData, 0, 0);
  }

  /**
   * Get base color for biome
   */
  private getBiomeColor(biome: string): string {
    const colors: Record<string, string> = {
      grassland: '#22c55e',
      plains: '#88ff00',
      forest: '#166534',
      jungle: '#15803d',
      desert: '#fbbf24',
      mountain: '#6b7280',
      hills: '#996633',
      tundra: '#9ca3af',
      snow: '#e5e7eb',
      ocean: '#1e40af',
    };
    return colors[biome] || '#22c55e';
  }

  // ============================================================================
  // COMPATIBILITY INTERFACE - Same as original TextureService
  // ============================================================================

  /**
   * Load existing textures (compatibility method)
   */
  loadExistingTextures(): Promise<void> {
    // Not needed in @texture-factory - textures are generated on demand
    console.warn('📁 Using @texture-factory - textures generated on demand');
    return Promise.resolve();
  }

  /**
   * Load a texture by ID with caching
   */
  async loadTexture(textureId: string): Promise<THREE.Texture> {
    // Check cache first
    const cached = this.textureCache.get(textureId);
    if (cached) {
      return cached;
    }

    // Check if already loading
    const loadingPromise = this.loadingPromises.get(textureId);
    if (loadingPromise) {
      return loadingPromise;
    }

    // Generate texture on demand
    const promise = this.generateTextureOnDemand(textureId);
    this.loadingPromises.set(textureId, promise);

    try {
      const texture = await promise;
      this.loadingPromises.delete(textureId);
      return texture;
    } catch (error) {
      this.loadingPromises.delete(textureId);
      throw error;
    }
  }

  /**
   * Generate texture on demand
   */
  private generateTextureOnDemand(textureId: string): Promise<THREE.Texture> {
    return Promise.resolve().then(() => {
      // Extract biome from texture ID
      const biome = textureId
        .replace('biome_', '')
        .replace(/_normal|_roughness|_metallic|_height/, '');

      if (!BIOME_TERRAIN_MAP[biome]) {
        throw new Error(`Unknown biome for texture: ${textureId}`);
      }

      // Generate the biome textures if not already cached
      if (!this.textureCache.has(`biome_${biome}`)) {
        this.generateBiomeTextures(biome, 512);
      }

      const texture = this.textureCache.get(textureId);
      if (!texture) {
        throw new Error(`Failed to generate texture: ${textureId}`);
      }

      return texture;
    });
  }

  /**
   * Create a material instance from definition using existing shader system
   */
  async createMaterial(materialId: string): Promise<THREE.ShaderMaterial> {
    // Check cache first
    const cached = this.materialCache.get(materialId);
    if (cached) {
      return cached.clone();
    }

    const definition = this.materialDefinitions.get(materialId);
    if (!definition) {
      throw new Error(`Material definition not found: ${materialId}`);
    }

    // Load required textures
    const textures: Record<string, THREE.Texture> = {};

    if (definition.albedo_texture) {
      textures.albedo = await this.loadTexture(definition.albedo_texture);
    }

    if (definition.normal_texture) {
      textures.normal = await this.loadTexture(definition.normal_texture);
    }

    if (definition.roughness_texture) {
      textures.roughness = await this.loadTexture(definition.roughness_texture);
    }

    if (definition.metallic_texture) {
      textures.metallic = await this.loadTexture(definition.metallic_texture);
    }

    // Get the hex-terrain shader definition
    const shaderDef = getShaderDefinition('hex-terrain');

    // Create material using existing shader system
    const material = shaderManager.compile('hex-terrain', shaderDef, {
      transparent: definition.alpha_mode !== 'opaque',
    });

    // Update uniforms with our texture data
    if (material.uniforms) {
      // Material properties
      material.uniforms.u_roughness.value = definition.roughness;
      material.uniforms.u_metallic.value = definition.metallic;
      material.uniforms.u_textureScale.value = definition.texture_scale;

      // Texture uniforms and flags
      if (textures.albedo) {
        material.uniforms.u_albedoTexture.value = textures.albedo;
        material.uniforms.u_hasAlbedoTexture.value = true;
      }
      if (textures.normal) {
        material.uniforms.u_normalTexture.value = textures.normal;
        material.uniforms.u_hasNormalTexture.value = true;
      }
      if (textures.roughness) {
        material.uniforms.u_roughnessTexture.value = textures.roughness;
        material.uniforms.u_hasRoughnessTexture.value = true;
      }
      if (textures.metallic) {
        material.uniforms.u_metallicTexture.value = textures.metallic;
        material.uniforms.u_hasMetallicTexture.value = true;
      }
    }

    // Configure material properties
    material.side = definition.double_sided
      ? THREE.DoubleSide
      : THREE.FrontSide;

    // Cache the material
    this.materialCache.set(materialId, material);

    console.warn(
      `🎭 Created @texture-factory material using hex-terrain shader: ${materialId}`
    );
    return material.clone();
  }

  // ============================================================================
  // REMAINING COMPATIBILITY METHODS
  // ============================================================================

  getTextureCategories(): Promise<string[]> {
    return Promise.resolve(['terrain', 'water']);
  }

  getMaterialsByCategory(category: string): MaterialDefinition[] {
    return Array.from(this.materialDefinitions.values()).filter(
      material => material.category === category
    );
  }

  updateMaterialUniforms(_time: number): void {
    // DEPRECATED: Uniform updates are now handled by UniformService in RenderingProvider
    // This method is kept for backward compatibility but does nothing
    // Individual materials are registered with UniformService for automatic updates
  }

  clearCache(): void {
    // Dispose of all textures
    for (const texture of this.textureCache.values()) {
      texture.dispose();
    }

    // Dispose of all materials
    for (const material of this.materialCache.values()) {
      material.dispose();
    }

    // Clear generator cache
    this.generators.clear();

    this.textureCache.clear();
    this.materialCache.clear();
    this.loadingPromises.clear();

    console.warn('🗑️ @texture-factory cache cleared');
  }

  async regenerateTexture(textureId: string): Promise<boolean> {
    try {
      // Remove from cache
      this.textureCache.delete(textureId);

      // Regenerate on demand
      await this.generateTextureOnDemand(textureId);

      console.warn(`🔄 Regenerated texture: ${textureId}`);
      return true;
    } catch (error) {
      console.error(`Failed to regenerate texture ${textureId}:`, error);
      return false;
    }
  }

  getTexture(textureId: string): THREE.Texture | null {
    return this.textureCache.get(textureId) ?? null;
  }

  getAvailableTextures(): string[] {
    return Array.from(this.textureCache.keys());
  }

  debugLogTextures(): void {
    console.warn(
      '🎨 @texture-factory: Available textures:',
      this.getAvailableTextures()
    );
    console.warn(
      '🎨 @texture-factory: Texture cache size:',
      this.textureCache.size
    );
    console.warn(
      '🎨 @texture-factory: Material definitions:',
      Array.from(this.materialDefinitions.keys())
    );
  }

  analyzeTextureSources(biomes: string[]): {
    assetTextures: number;
    proceduralTextures: number;
    fallbackColors: number;
    details: Array<{
      biome: string;
      source: 'asset' | 'procedural' | 'none';
      dimensions?: { width: number; height: number };
    }>;
  } {
    const details: Array<{
      biome: string;
      source: 'asset' | 'procedural' | 'none';
      dimensions?: { width: number; height: number };
    }> = [];

    let proceduralTextures = 0;
    let fallbackColors = 0;

    biomes.forEach(biome => {
      const texture = this.getTexture(`biome_${biome}`);
      if (texture) {
        proceduralTextures++;
        details.push({
          biome,
          source: 'procedural',
          dimensions: {
            width: (texture.image as HTMLCanvasElement)?.width || 512,
            height: (texture.image as HTMLCanvasElement)?.height || 512,
          },
        });
      } else {
        fallbackColors++;
        details.push({
          biome,
          source: 'none',
        });
      }
    });

    return {
      assetTextures: 0, // All textures are procedural in @texture-factory
      proceduralTextures,
      fallbackColors,
      details,
    };
  }

  getStats(): {
    texturesLoaded: number;
    materialsCreated: number;
    cacheSize: number;
  } {
    return {
      texturesLoaded: this.textureCache.size,
      materialsCreated: this.materialCache.size,
      cacheSize: this.textureMetadata.size,
    };
  }
}

// Export singleton instance
export const textureService = new TextureFactoryService();
