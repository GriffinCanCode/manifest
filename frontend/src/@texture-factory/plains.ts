/**
 * @texture-factory/plains.ts
 *
 * Advanced procedural plains texture generation
 * Designed to compete with AAA game quality (Civilization VI standard)
 *
 * Features:
 * - Vast, rolling terrain with subtle elevation changes
 * - Wind-swept grass patterns with directional bias
 * - Multi-scale detail layering for realistic plains appearance
 * - Seasonal variation and environmental responsiveness
 * - Seamless integration with adjacent terrain types
 * - Advanced noise techniques for natural-looking expanses
 */

import { Color, MathUtils } from 'three';

// ============================================================================
// ADVANCED NOISE FUNCTIONS
// ============================================================================

/**
 * Domain warping implementation optimized for vast open plains
 * Creates subtle, rolling patterns characteristic of grassland plains
 */
class PlainsNoiseSystem {
  private static fbm(x: number, y: number, octaves: number = 6): number {
    let value = 0;
    let amplitude = 0.5;
    let frequency = 1;

    for (let i = 0; i < octaves; i++) {
      value += amplitude * this.noise(x * frequency, y * frequency);
      amplitude *= 0.45; // Slightly lower persistence for smoother plains
      frequency *= 1.8; // Lower lacunarity for gentler transitions
    }

    return value;
  }

  private static noise(x: number, y: number): number {
    // Improved Perlin-style noise with better distribution for plains
    const ix = Math.floor(x);
    const iy = Math.floor(y);
    const fx = x - ix;
    const fy = y - iy;

    // Smoother interpolation curve for plains
    const ux = fx * fx * fx * (fx * (fx * 6 - 15) + 10);
    const uy = fy * fy * fy * (fy * (fy * 6 - 15) + 10);

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

  static warpDomain(
    x: number,
    y: number,
    scale: number = 1,
    strength: number = 0.08
  ): [number, number] {
    // Gentler warping for smoother plains appearance
    const warpX = this.fbm(x * scale, y * scale, 4) * strength;
    const warpY = this.fbm((x + 5.2) * scale, (y + 1.3) * scale, 4) * strength;

    return [x + warpX, y + warpY];
  }

  /**
   * Generate wind patterns for plains grass directionality
   */
  static windPattern(
    x: number,
    y: number,
    windDirection: number = 0.7,
    windStrength: number = 0.3
  ): number {
    const windX = Math.cos(windDirection * Math.PI * 2);
    const windY = Math.sin(windDirection * Math.PI * 2);

    // Create flowing grass patterns based on wind direction
    const flowNoise = this.fbm(x * 8 + windX * 2, y * 8 + windY * 2, 3);

    return flowNoise * windStrength;
  }
}

/**
 * Rolling noise for gentle hills and elevation changes
 */
class RollingNoise {
  static generate(x: number, y: number, octaves: number = 4): number {
    let value = 0;
    let amplitude = 1;
    let frequency = 0.5; // Lower frequency for broader hills

    for (let i = 0; i < octaves; i++) {
      let signal = PlainsNoiseSystem['noise'](x * frequency, y * frequency);

      // Apply smoothstep for gentler rolling hills
      signal = MathUtils.smoothstep(-0.5, 0.5, signal);
      signal = signal * 2 - 1; // Normalize to -1 to 1

      value += signal * amplitude;
      amplitude *= 0.6; // Gentle amplitude decay
      frequency *= 1.5; // Moderate frequency increase
    }

    return MathUtils.clamp(value, -1, 1);
  }
}

/**
 * Erosion patterns for realistic plains weathering
 */
class PlainsErosion {
  static generate(x: number, y: number, intensity: number = 0.5): number {
    // Gentle erosion patterns for plains
    const erosionChannels = PlainsNoiseSystem['fbm'](x * 4, y * 4, 5);
    const windErosion = PlainsNoiseSystem.windPattern(x, y, 0.3, 0.2);

    return MathUtils.clamp(
      (erosionChannels * 0.7 + windErosion * 0.3) * intensity,
      -1,
      1
    );
  }
}

// ============================================================================
// PLAINS MATERIAL PROPERTIES
// ============================================================================

interface PlainsVariation {
  name: string;
  baseColor: Color;
  secondaryColor: Color;
  dryColor: Color;
  soilColor: Color;
  roughness: number;
  metallic: number;
  normalStrength: number;
  grassDensity: number;
  elevationVariation: number;
  windIntensity: number;
}

const PLAINS_VARIATIONS: PlainsVariation[] = [
  {
    name: 'vast_prairie',
    baseColor: new Color(0.18, 0.45, 0.15), // Rich prairie green
    secondaryColor: new Color(0.22, 0.55, 0.18), // Lighter green
    dryColor: new Color(0.4, 0.38, 0.2), // Dried grass
    soilColor: new Color(0.25, 0.2, 0.12), // Dark prairie soil
    roughness: 0.75,
    metallic: 0.0,
    normalStrength: 1.0,
    grassDensity: 0.8,
    elevationVariation: 0.3,
    windIntensity: 0.4,
  },
  {
    name: 'rolling_hills',
    baseColor: new Color(0.16, 0.4, 0.13), // Deeper green for hills
    secondaryColor: new Color(0.24, 0.5, 0.19),
    dryColor: new Color(0.38, 0.35, 0.18),
    soilColor: new Color(0.22, 0.18, 0.1),
    roughness: 0.8,
    metallic: 0.0,
    normalStrength: 1.2,
    grassDensity: 0.7,
    elevationVariation: 0.6,
    windIntensity: 0.3,
  },
  {
    name: 'dry_plains',
    baseColor: new Color(0.25, 0.35, 0.15), // More yellowish
    secondaryColor: new Color(0.3, 0.42, 0.18),
    dryColor: new Color(0.45, 0.4, 0.22),
    soilColor: new Color(0.3, 0.25, 0.15), // Lighter, drier soil
    roughness: 0.9,
    metallic: 0.0,
    normalStrength: 0.9,
    grassDensity: 0.5,
    elevationVariation: 0.4,
    windIntensity: 0.6,
  },
  {
    name: 'fertile_valley',
    baseColor: new Color(0.14, 0.5, 0.12), // Lush valley green
    secondaryColor: new Color(0.2, 0.6, 0.16),
    dryColor: new Color(0.32, 0.4, 0.18),
    soilColor: new Color(0.18, 0.15, 0.08), // Rich dark soil
    roughness: 0.7,
    metallic: 0.0,
    normalStrength: 1.3,
    grassDensity: 0.9,
    elevationVariation: 0.2,
    windIntensity: 0.2,
  },
];

// ============================================================================
// ADVANCED PLAINS TEXTURE GENERATOR
// ============================================================================

export class AdvancedPlainsGenerator {
  private resolution: number;
  private variation: PlainsVariation;
  private environmentalFactors: {
    moisture: number; // 0-1, affects grass color and density
    temperature: number; // 0-1, affects growth patterns
    season: number; // 0-1, affects color variation
    elevation: number; // 0-1, affects elevation changes
    windDirection: number; // 0-1, prevailing wind direction
  };

  constructor(
    resolution: number = 1024,
    variationName: string = 'vast_prairie',
    environmentalFactors = {
      moisture: 0.6,
      temperature: 0.7,
      season: 0.5,
      elevation: 0.4,
      windDirection: 0.3,
    }
  ) {
    this.resolution = resolution;
    this.variation =
      PLAINS_VARIATIONS.find(v => v.name === variationName) ??
      PLAINS_VARIATIONS[0];
    this.environmentalFactors = environmentalFactors;
  }

  /**
   * Generate complete PBR material set for plains
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
   * Generate high-quality albedo map with vast plains characteristics
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

        // Apply gentle domain warping for natural plains flow
        const [warpedU, warpedV] = PlainsNoiseSystem.warpDomain(u, v, 2, 0.06);

        // Large-scale plains patterns
        const plainsMacro =
          RollingNoise.generate(warpedU * 2, warpedV * 2, 3) * 0.4;

        // Grass texture patterns
        const grassMicro =
          PlainsNoiseSystem['fbm'](warpedU * 32, warpedV * 32, 4) * 0.3;

        // Wind-swept grass patterns
        const windPattern = PlainsNoiseSystem.windPattern(
          warpedU,
          warpedV,
          this.environmentalFactors.windDirection,
          this.variation.windIntensity
        );

        // Erosion and weathering
        const erosionPattern = PlainsErosion.generate(warpedU, warpedV, 0.3);

        // Environmental variation
        const moistureNoise =
          PlainsNoiseSystem['fbm'](u * 1.5, v * 1.5, 2) * 0.2 + 0.8;
        const actualMoisture =
          this.environmentalFactors.moisture * moistureNoise;

        // Seasonal color changes
        const seasonalShift =
          Math.sin(u * Math.PI) * Math.cos(v * Math.PI) * 0.08;
        const actualSeason = MathUtils.clamp(
          this.environmentalFactors.season + seasonalShift,
          0,
          1
        );

        // Combine all factors for grass density and appearance
        const combinedPattern = MathUtils.clamp(
          plainsMacro + grassMicro + windPattern * 0.5 + erosionPattern * 0.2,
          -1,
          1
        );

        const grassDensity = MathUtils.clamp(
          (combinedPattern + 1) * 0.5 * this.variation.grassDensity,
          0,
          1
        );

        const moistureFactor = MathUtils.clamp(actualMoisture, 0.1, 1);
        const seasonFactor = 1 - actualSeason * 0.3; // Autumn browning
        const temperatureFactor =
          0.7 + this.environmentalFactors.temperature * 0.3;

        // Color selection based on grass density and environmental factors
        let finalColor: Color;

        if (grassDensity > 0.8 && moistureFactor > 0.7) {
          // Lush, healthy plains grass
          finalColor = this.variation.baseColor
            .clone()
            .lerp(this.variation.secondaryColor, windPattern + 0.5);
        } else if (grassDensity > 0.5) {
          // Normal plains grass
          finalColor = this.variation.baseColor
            .clone()
            .lerp(this.variation.dryColor, (1 - moistureFactor) * 0.6);
        } else if (grassDensity > 0.2) {
          // Sparse grass with soil showing through
          finalColor = this.variation.dryColor
            .clone()
            .lerp(this.variation.soilColor, 1 - grassDensity);
        } else {
          // Mostly soil/bare patches
          finalColor = this.variation.soilColor
            .clone()
            .lerp(this.variation.dryColor, grassDensity * 2);
        }

        // Apply environmental modulation
        finalColor.multiplyScalar(seasonFactor * temperatureFactor);

        // Add subtle variation
        const variation = (grassMicro + windPattern) * 0.1;
        finalColor.r = MathUtils.clamp(finalColor.r * (1 + variation), 0, 1);
        finalColor.g = MathUtils.clamp(finalColor.g * (1 + variation), 0, 1);
        finalColor.b = MathUtils.clamp(finalColor.b * (1 + variation), 0, 1);

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
   * Generate detailed normal map for plains micro-detail
   */
  private generateNormalMap(): ImageData {
    const canvas = new OffscreenCanvas(this.resolution, this.resolution);
    const ctx = canvas.getContext('2d');
    if (!ctx) {
      throw new Error('Failed to get 2D context for normal map generation');
    }
    const imageData = ctx.createImageData(this.resolution, this.resolution);
    const { data } = imageData;

    const heightScale = this.variation.normalStrength * 0.008; // Gentler for plains

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
    const [warpedU, warpedV] = PlainsNoiseSystem.warpDomain(u, v, 2, 0.04);

    // Rolling terrain height
    const rollingHeight =
      RollingNoise.generate(warpedU * 1.5, warpedV * 1.5, 4) *
      this.variation.elevationVariation *
      0.8;

    // Grass blade micro-detail
    const grassDetail =
      PlainsNoiseSystem['fbm'](warpedU * 64, warpedV * 64, 5) * 0.15;

    // Wind-swept patterns
    const windDetail = PlainsNoiseSystem.windPattern(
      warpedU * 8,
      warpedV * 8,
      this.environmentalFactors.windDirection,
      0.1
    );

    // Erosion detail
    const erosionDetail = PlainsErosion.generate(warpedU * 4, warpedV * 4, 0.1);

    return rollingHeight + grassDetail + windDetail + erosionDetail;
  }

  /**
   * Generate roughness map with realistic plains surface properties
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

        const [warpedU, warpedV] = PlainsNoiseSystem.warpDomain(
          u,
          v,
          1.5,
          0.03
        );

        // Base roughness from variation
        let { roughness } = this.variation;

        // Modify based on moisture (wet grass is smoother)
        const moistureNoise = PlainsNoiseSystem['fbm'](
          warpedU * 3,
          warpedV * 3,
          2
        );
        const localMoisture =
          this.environmentalFactors.moisture * (0.8 + moistureNoise * 0.2);
        roughness *= 0.5 + localMoisture * 0.5;

        // Add grass density variation
        const grassDensity =
          (RollingNoise.generate(warpedU, warpedV, 3) + 1) * 0.5;
        roughness = MathUtils.lerp(
          roughness * 1.2,
          roughness * 0.8,
          grassDensity
        );

        // Surface variation from wind patterns
        const windVariation =
          PlainsNoiseSystem.windPattern(warpedU * 2, warpedV * 2) * 0.05;
        roughness = MathUtils.clamp(roughness + windVariation, 0, 1);

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
   * Generate metallic map (plains grass is non-metallic)
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
        // Plains grass is essentially non-metallic
        let { metallic } = this.variation;

        // Very slight metallicism for dewdrops or wet conditions
        if (this.environmentalFactors.moisture > 0.9) {
          const u = x / this.resolution;
          const v = y / this.resolution;
          const dewNoise = PlainsNoiseSystem['fbm'](u * 16, v * 16, 2) * 0.02;
          metallic = Math.min(metallic + dewNoise, 0.03);
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

        const height = this.sampleHeightForNormal(u, v);
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
    adjacentTerrain: 'river' | 'mountain' | 'forest' | 'desert' | 'grassland'
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
            // Create gradual transition near water
            const distToWater = Math.min(u, 1 - u, v, 1 - v);
            blendFactor = MathUtils.smoothstep(0, 0.4, distToWater);
            break;

          case 'mountain':
            // Elevation-based blending with some plains extending up slopes
            const elevationNoise = PlainsNoiseSystem['fbm'](
              u * 1.5,
              v * 1.5,
              3
            );
            blendFactor = MathUtils.clamp(0.6 + elevationNoise * 0.4, 0, 1);
            break;

          case 'forest':
            // Natural transition with scattered plains patches
            const forestTransition = RollingNoise.generate(u * 3, v * 3, 3);
            blendFactor = MathUtils.clamp(
              (forestTransition + 1) * 0.4,
              0.1,
              0.8
            );
            break;

          case 'desert':
            // Gradual transition from fertile plains to arid conditions
            const desertTransition = PlainsNoiseSystem['fbm'](u * 2, v * 2, 4);
            const distance = Math.min(u, 1 - u);
            blendFactor =
              MathUtils.smoothstep(0, 0.5, distance) *
              MathUtils.clamp(0.5 + desertTransition * 0.3, 0.2, 0.9);
            break;

          case 'grassland':
            // Smooth transition between plains and denser grassland
            const grasslandTransition = PlainsNoiseSystem['fbm'](
              u * 4,
              v * 4,
              3
            );
            blendFactor = MathUtils.clamp(
              0.7 + grasslandTransition * 0.3,
              0.4,
              1
            );
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
 * Generate and export complete plains texture set
 */
export const generatePlainsTextureSet = async (
  variation: string = 'vast_prairie',
  resolution: number = 1024,
  environmentalFactors?: {
    moisture: number;
    temperature: number;
    season: number;
    elevation: number;
    windDirection: number;
  }
): Promise<{
  albedo: Blob;
  normal: Blob;
  roughness: Blob;
  metallic: Blob;
  height: Blob;
}> => {
  const generator = new AdvancedPlainsGenerator(
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
