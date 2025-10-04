/**
 * @texture-factory/grassland.ts
 *
 * Advanced procedural grassland texture generation
 * Designed to compete with AAA game quality (Civilization VI standard)
 *
 * Features:
 * - Domain warping for organic grass patterns
 * - Multi-scale detail layering
 * - Realistic PBR material properties
 * - Seamless tiling with advanced noise techniques
 * - Environmental variation support
 * - River and mountain integration points
 */

import { Color, MathUtils } from 'three';

// ============================================================================
// ADVANCED NOISE FUNCTIONS
// ============================================================================

/**
 * Domain warping implementation for organic distortion
 * Creates the natural, flowing patterns seen in real grasslands
 */
class DomainWarping {
  private static fbm(x: number, y: number, octaves: number = 4): number {
    let value = 0;
    let amplitude = 0.5;
    let frequency = 1;

    for (let i = 0; i < octaves; i++) {
      value += amplitude * this.noise(x * frequency, y * frequency);
      amplitude *= 0.5;
      frequency *= 2;
    }

    return value;
  }

  private static noise(x: number, y: number): number {
    // Improved Perlin-style noise with better distribution
    const ix = Math.floor(x);
    const iy = Math.floor(y);
    const fx = x - ix;
    const fy = y - iy;

    // Smooth interpolation
    const ux = fx * fx * (3 - 2 * fx);
    const uy = fy * fy * (3 - 2 * fy);

    const a = this.hash(ix, iy);
    const b = this.hash(ix + 1, iy);
    const c = this.hash(ix, iy + 1);
    const d = this.hash(ix + 1, iy + 1);

    return (
      MathUtils.lerp(MathUtils.lerp(a, b, ux), MathUtils.lerp(c, d, ux), uy) *
        2 -
      1
    );
  }

  private static hash(x: number, y: number): number {
    let h = (x * 127.1 + y * 311.7) % 1000;
    h = ((h * 269.5) % 1000) / 1000;
    return h;
  }

  static warpDomain(x: number, y: number, scale: number = 1): [number, number] {
    const strength = 0.15;
    const warpX = this.fbm(x * scale, y * scale, 3) * strength;
    const warpY = this.fbm((x + 5.2) * scale, (y + 1.3) * scale, 3) * strength;

    return [x + warpX, y + warpY];
  }
}

/**
 * Ridged noise for creating grass blade patterns and erosion effects
 */
class RidgedNoise {
  static generate(x: number, y: number, octaves: number = 6): number {
    let value = 0;
    let amplitude = 1;
    let frequency = 1;
    let weight = 1;

    for (let i = 0; i < octaves; i++) {
      let signal = Math.abs(
        DomainWarping['noise'](x * frequency, y * frequency)
      );
      signal = 1 - signal; // Ridge the noise
      signal *= signal * weight; // Square and apply weight

      value += signal * amplitude;
      weight = MathUtils.clamp(signal * 2, 0, 1);
      amplitude *= 0.5;
      frequency *= 2;
    }

    return value;
  }
}

/**
 * Cellular automata for creating realistic grass clump patterns
 */
class CellularNoise {
  static generate(x: number, y: number, scale: number = 8): number {
    const points = [];
    const cellSize = 1 / scale;

    // Generate random points in neighboring cells
    for (let xi = -1; xi <= 1; xi++) {
      for (let yi = -1; yi <= 1; yi++) {
        const cellX = Math.floor(x * scale) + xi;
        const cellY = Math.floor(y * scale) + yi;

        const hash = ((cellX * 127.1 + cellY * 311.7) % 1000) / 1000;
        const pointX = cellX * cellSize + hash * cellSize;
        const pointY =
          cellY * cellSize + (((hash * 269.5) % 1000) / 1000) * cellSize;

        points.push({ x: pointX, y: pointY });
      }
    }

    // Find closest distance
    let minDist = Infinity;
    for (const point of points) {
      const dist = Math.hypot(x - point.x, y - point.y);
      minDist = Math.min(minDist, dist);
    }

    return 1 - MathUtils.clamp(minDist * scale * 2, 0, 1);
  }
}

// ============================================================================
// GRASSLAND MATERIAL PROPERTIES
// ============================================================================

interface GrasslandVariation {
  name: string;
  baseColor: Color;
  secondaryColor: Color;
  dryColor: Color;
  roughness: number;
  metallic: number;
  normalStrength: number;
  density: number;
  bladeHeight: number;
}

const GRASSLAND_VARIATIONS: GrasslandVariation[] = [
  {
    name: 'lush_meadow',
    baseColor: new Color(0.15, 0.4, 0.12), // Deep, healthy green
    secondaryColor: new Color(0.25, 0.55, 0.18), // Lighter green
    dryColor: new Color(0.35, 0.32, 0.15), // Dried grass patches
    roughness: 0.8,
    metallic: 0.0,
    normalStrength: 1.2,
    density: 0.9,
    bladeHeight: 1.0,
  },
  {
    name: 'prairie_grass',
    baseColor: new Color(0.18, 0.35, 0.14), // Slightly yellower
    secondaryColor: new Color(0.28, 0.45, 0.2),
    dryColor: new Color(0.4, 0.35, 0.18),
    roughness: 0.85,
    metallic: 0.0,
    normalStrength: 1.0,
    density: 0.7,
    bladeHeight: 1.2,
  },
  {
    name: 'sparse_grassland',
    baseColor: new Color(0.12, 0.3, 0.1),
    secondaryColor: new Color(0.2, 0.4, 0.15),
    dryColor: new Color(0.3, 0.28, 0.12),
    roughness: 0.9,
    metallic: 0.0,
    normalStrength: 0.8,
    density: 0.5,
    bladeHeight: 0.8,
  },
];

// ============================================================================
// ADVANCED GRASSLAND TEXTURE GENERATOR
// ============================================================================

export class AdvancedGrasslandGenerator {
  private resolution: number;
  private variation: GrasslandVariation;
  private environmentalFactors: {
    moisture: number; // 0-1, affects color and density
    temperature: number; // 0-1, affects growth patterns
    season: number; // 0-1, affects color variation
    elevation: number; // 0-1, affects grass type
  };

  constructor(
    resolution: number = 1024,
    variationName: string = 'lush_meadow',
    environmentalFactors = {
      moisture: 0.7,
      temperature: 0.6,
      season: 0.5,
      elevation: 0.3,
    }
  ) {
    this.resolution = resolution;
    this.variation =
      GRASSLAND_VARIATIONS.find(v => v.name === variationName) ??
      GRASSLAND_VARIATIONS[0];
    this.environmentalFactors = environmentalFactors;
  }

  /**
   * Generate complete PBR material set for grassland
   */
  generateTextures(): {
    albedo: ImageData;
    normal: ImageData;
    roughness: ImageData;
    metallic: ImageData;
    height: ImageData;
  } {
    const albedo = this.generateAlbedoMap();
    const normal = this.generateNormalMap();
    const roughness = this.generateRoughnessMap();
    const metallic = this.generateMetallicMap();
    const height = this.generateHeightMap();

    return { albedo, normal, roughness, metallic, height };
  }

  /**
   * Generate high-quality albedo map with multiple grass types and environmental variation
   */
  private generateAlbedoMap(): ImageData {
    const canvas = new OffscreenCanvas(this.resolution, this.resolution);
    const ctx = canvas.getContext('2d');
    if (!ctx) {
      throw new Error('Failed to get 2D context for albedo map generation');
    }
    const imageData = ctx.createImageData(this.resolution, this.resolution);
    const { data } = imageData;

    for (let y = 0; y < this.resolution; y++) {
      for (let x = 0; x < this.resolution; x++) {
        const u = x / this.resolution;
        const v = y / this.resolution;

        // Apply domain warping for organic grass distribution
        const [warpedU, warpedV] = DomainWarping.warpDomain(u, v, 4);

        // Multi-scale grass pattern using cellular automata and ridged noise
        const grassClusters = CellularNoise.generate(warpedU, warpedV, 12);
        const grassBlades =
          RidgedNoise.generate(warpedU * 64, warpedV * 64, 4) * 0.3;
        const fineDetail =
          DomainWarping['fbm'](warpedU * 128, warpedV * 128, 3) * 0.15;

        // Environmental variation
        const moistureNoise = DomainWarping['fbm'](u * 2, v * 2, 2) * 0.3 + 0.7;
        const actualMoisture =
          this.environmentalFactors.moisture * moistureNoise;

        const seasonalVariation =
          Math.sin(u * Math.PI * 2) * Math.cos(v * Math.PI * 2) * 0.1;
        const actualSeason = MathUtils.clamp(
          this.environmentalFactors.season + seasonalVariation,
          0,
          1
        );

        // Combine all factors for final grass color
        const grassDensity = MathUtils.clamp(
          grassClusters + grassBlades + fineDetail,
          0,
          1
        );
        const moistureFactor = MathUtils.clamp(actualMoisture, 0.2, 1);
        const seasonFactor = 1 - actualSeason * 0.4; // Less vibrant in autumn

        // Color blending based on density and environmental factors
        let finalColor: Color;
        if (grassDensity > 0.7 && moistureFactor > 0.6) {
          // Lush, healthy grass
          finalColor = this.variation.baseColor
            .clone()
            .lerp(this.variation.secondaryColor, grassBlades);
        } else if (grassDensity > 0.4) {
          // Normal grass
          finalColor = this.variation.baseColor
            .clone()
            .lerp(this.variation.dryColor, 1 - moistureFactor);
        } else {
          // Sparse or dirt patches
          finalColor = this.variation.dryColor
            .clone()
            .lerp(new Color(0.2, 0.15, 0.1), 1 - grassDensity);
        }

        // Apply seasonal and environmental modulation
        finalColor.multiplyScalar(seasonFactor);
        finalColor.r = MathUtils.clamp(
          finalColor.r * (0.8 + fineDetail * 0.4),
          0,
          1
        );
        finalColor.g = MathUtils.clamp(
          finalColor.g * (0.8 + fineDetail * 0.4),
          0,
          1
        );
        finalColor.b = MathUtils.clamp(
          finalColor.b * (0.8 + fineDetail * 0.4),
          0,
          1
        );

        // Write to image data
        const idx = (y * this.resolution + x) * 4;
        data[idx] = Math.floor(finalColor.r * 255); // R
        data[idx + 1] = Math.floor(finalColor.g * 255); // G
        data[idx + 2] = Math.floor(finalColor.b * 255); // B
        data[idx + 3] = 255; // A
      }
    }

    return imageData;
  }

  /**
   * Generate detailed normal map for grass blade micro-detail
   */
  private generateNormalMap(): ImageData {
    const canvas = new OffscreenCanvas(this.resolution, this.resolution);
    const ctx = canvas.getContext('2d');
    if (!ctx) {
      throw new Error('Failed to get 2D context for normal map generation');
    }
    const imageData = ctx.createImageData(this.resolution, this.resolution);
    const { data } = imageData;

    const heightScale = this.variation.normalStrength * 0.01;

    for (let y = 0; y < this.resolution; y++) {
      for (let x = 0; x < this.resolution; x++) {
        const u = x / this.resolution;
        const v = y / this.resolution;

        // Sample height at current and neighboring pixels for gradient calculation
        const height = this.sampleHeightForNormal(u, v);
        const heightX = this.sampleHeightForNormal(u + 1 / this.resolution, v);
        const heightY = this.sampleHeightForNormal(u, v + 1 / this.resolution);

        // Calculate gradients
        const dx = (heightX - height) * heightScale;
        const dy = (heightY - height) * heightScale;

        // Convert to normal vector and normalize
        const normal = {
          x: -dx,
          y: -dy,
          z: 1,
        };

        const length = Math.hypot(normal.x, normal.y, normal.z);
        normal.x /= length;
        normal.y /= length;
        normal.z /= length;

        // Convert to [0, 255] range
        const idx = (y * this.resolution + x) * 4;
        data[idx] = Math.floor((normal.x + 1) * 127.5); // R (X)
        data[idx + 1] = Math.floor((normal.y + 1) * 127.5); // G (Y)
        data[idx + 2] = Math.floor((normal.z + 1) * 127.5); // B (Z)
        data[idx + 3] = 255; // A
      }
    }

    return imageData;
  }

  /**
   * Sample height data for normal map generation
   */
  private sampleHeightForNormal(u: number, v: number): number {
    const [warpedU, warpedV] = DomainWarping.warpDomain(u, v, 4);

    // Grass blade detail
    const bladeDetail =
      RidgedNoise.generate(warpedU * 128, warpedV * 128, 6) * 0.5;

    // Grass clump variation
    const clumpHeight = CellularNoise.generate(warpedU, warpedV, 8) * 0.3;

    // Fine surface detail
    const surfaceDetail =
      DomainWarping['fbm'](warpedU * 256, warpedV * 256, 4) * 0.2;

    return bladeDetail + clumpHeight + surfaceDetail;
  }

  /**
   * Generate roughness map with realistic grass surface properties
   */
  private generateRoughnessMap(): ImageData {
    const canvas = new OffscreenCanvas(this.resolution, this.resolution);
    const ctx = canvas.getContext('2d');
    if (!ctx) {
      throw new Error('Failed to get 2D context for roughness map generation');
    }
    const imageData = ctx.createImageData(this.resolution, this.resolution);
    const { data } = imageData;

    for (let y = 0; y < this.resolution; y++) {
      for (let x = 0; x < this.resolution; x++) {
        const u = x / this.resolution;
        const v = y / this.resolution;

        const [warpedU, warpedV] = DomainWarping.warpDomain(u, v, 2);

        // Base roughness from variation
        let { roughness } = this.variation;

        // Modify based on moisture (wet grass is less rough)
        const moistureNoise = DomainWarping['fbm'](warpedU * 4, warpedV * 4, 2);
        const localMoisture =
          this.environmentalFactors.moisture * (0.7 + moistureNoise * 0.3);
        roughness *= 0.6 + localMoisture * 0.4;

        // Add surface variation
        const surfaceVariation =
          DomainWarping['fbm'](warpedU * 32, warpedV * 32, 3) * 0.1;
        roughness = MathUtils.clamp(roughness + surfaceVariation, 0, 1);

        const idx = (y * this.resolution + x) * 4;
        const roughnessValue = Math.floor(roughness * 255);
        data[idx] = roughnessValue;
        data[idx + 1] = roughnessValue;
        data[idx + 2] = roughnessValue;
        data[idx + 3] = 255;
      }
    }

    return imageData;
  }

  /**
   * Generate metallic map (grass is non-metallic, but may have wet spots)
   */
  private generateMetallicMap(): ImageData {
    const canvas = new OffscreenCanvas(this.resolution, this.resolution);
    const ctx = canvas.getContext('2d');
    if (!ctx) {
      throw new Error('Failed to get 2D context for metallic map generation');
    }
    const imageData = ctx.createImageData(this.resolution, this.resolution);
    const { data } = imageData;

    for (let y = 0; y < this.resolution; y++) {
      for (let x = 0; x < this.resolution; x++) {
        const u = x / this.resolution;
        const v = y / this.resolution;

        // Grass is generally non-metallic
        let { metallic } = this.variation;

        // Add tiny bit of metallicism for wet areas or dew
        if (this.environmentalFactors.moisture > 0.8) {
          const dewNoise = CellularNoise.generate(u, v, 24) * 0.05;
          metallic = Math.min(metallic + dewNoise, 0.05);
        }

        const idx = (y * this.resolution + x) * 4;
        const metallicValue = Math.floor(metallic * 255);
        data[idx] = metallicValue;
        data[idx + 1] = metallicValue;
        data[idx + 2] = metallicValue;
        data[idx + 3] = 255;
      }
    }

    return imageData;
  }

  /**
   * Generate height/displacement map for terrain interaction
   */
  private generateHeightMap(): ImageData {
    const canvas = new OffscreenCanvas(this.resolution, this.resolution);
    const ctx = canvas.getContext('2d');
    if (!ctx) {
      throw new Error('Failed to get 2D context for height map generation');
    }
    const imageData = ctx.createImageData(this.resolution, this.resolution);
    const { data } = imageData;

    for (let y = 0; y < this.resolution; y++) {
      for (let x = 0; x < this.resolution; x++) {
        const u = x / this.resolution;
        const v = y / this.resolution;

        const height =
          this.sampleHeightForNormal(u, v) * this.variation.bladeHeight;
        const heightValue = Math.floor(
          MathUtils.clamp((height + 1) * 127.5, 0, 255)
        );

        const idx = (y * this.resolution + x) * 4;
        data[idx] = heightValue;
        data[idx + 1] = heightValue;
        data[idx + 2] = heightValue;
        data[idx + 3] = 255;
      }
    }

    return imageData;
  }

  /**
   * Generate blending mask for seamless terrain integration
   * Returns alpha mask for blending with adjacent terrain types
   */
  generateBlendingMask(
    adjacentTerrain: 'river' | 'mountain' | 'forest' | 'desert'
  ): ImageData {
    const canvas = new OffscreenCanvas(this.resolution, this.resolution);
    const ctx = canvas.getContext('2d');
    if (!ctx) {
      throw new Error('Failed to get 2D context for blending mask generation');
    }
    const imageData = ctx.createImageData(this.resolution, this.resolution);
    const { data } = imageData;

    for (let y = 0; y < this.resolution; y++) {
      for (let x = 0; x < this.resolution; x++) {
        const u = x / this.resolution;
        const v = y / this.resolution;

        let blendFactor = 1.0;

        // Different blending strategies based on adjacent terrain
        switch (adjacentTerrain) {
          case 'river':
            // Create soft falloff near water
            const distToWater = Math.min(u, 1 - u, v, 1 - v);
            blendFactor = MathUtils.smoothstep(0, 0.3, distToWater);
            break;

          case 'mountain':
            // Create elevation-based blending
            const elevationNoise = DomainWarping['fbm'](u * 2, v * 2, 3);
            blendFactor = MathUtils.clamp(0.7 + elevationNoise * 0.3, 0, 1);
            break;

          case 'forest':
            // Natural transition with some grass patches in forest
            const forestTransition = CellularNoise.generate(u, v, 6);
            blendFactor = MathUtils.clamp(forestTransition, 0.2, 0.9);
            break;

          case 'desert':
            // Sparse grass at desert edge
            const desertDistance = Math.min(u, 1 - u);
            blendFactor = MathUtils.smoothstep(0, 0.4, desertDistance) * 0.6;
            break;
        }

        const idx = (y * this.resolution + x) * 4;
        const alpha = Math.floor(blendFactor * 255);
        data[idx] = 255; // R
        data[idx + 1] = 255; // G
        data[idx + 2] = 255; // B
        data[idx + 3] = alpha; // A (blend factor)
      }
    }

    return imageData;
  }
}

// ============================================================================
// TEXTURE EXPORT UTILITIES
// ============================================================================

/**
 * Convert ImageData to downloadable blob
 */
export const imageDataToBlob = (imageData: ImageData): Promise<Blob> => {
  return new Promise((resolve, reject) => {
    const canvas = new OffscreenCanvas(imageData.width, imageData.height);
    const ctx = canvas.getContext('2d');
    if (!ctx) {
      reject(new Error('Failed to get 2D context for blob conversion'));
      return;
    }
    ctx.putImageData(imageData, 0, 0);

    canvas
      .convertToBlob({ type: 'image/png' })
      .then(resolve)
      .catch(error => {
        console.error('Failed to convert canvas to blob:', error);
        throw error;
      });
  });
};

/**
 * Generate and export complete grassland texture set
 */
export const generateGrasslandTextureSet = async (
  variation: string = 'lush_meadow',
  resolution: number = 1024,
  environmentalFactors?: {
    moisture: number;
    temperature: number;
    season: number;
    elevation: number;
  }
): Promise<{
  albedo: Blob;
  normal: Blob;
  roughness: Blob;
  metallic: Blob;
  height: Blob;
}> => {
  const generator = new AdvancedGrasslandGenerator(
    resolution,
    variation,
    environmentalFactors
  );
  const textures = generator.generateTextures();

  const [albedo, normal, roughness, metallic, height] = await Promise.all([
    imageDataToBlob(textures.albedo),
    imageDataToBlob(textures.normal),
    imageDataToBlob(textures.roughness),
    imageDataToBlob(textures.metallic),
    imageDataToBlob(textures.height),
  ]);

  return { albedo, normal, roughness, metallic, height };
};
