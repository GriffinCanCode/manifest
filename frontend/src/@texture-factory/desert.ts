/**
 * @texture-factory/desert.ts
 *
 * Advanced procedural desert texture generation
 * Designed to compete with AAA game quality (Civilization VI standard)
 *
 * Features:
 * - Wind erosion pattern simulation
 * - Sand dune formation with realistic ripples
 * - Rock outcrop placement and weathering
 * - Oasis transition zones
 * - Heat shimmer and atmospheric effects
 * - Multi-scale sand grain detail
 * - Environmental variation support
 * - River and mountain integration points
 */

import { Color, MathUtils } from 'three';

// ============================================================================
// ADVANCED DESERT NOISE FUNCTIONS
// ============================================================================

/**
 * Domain warping implementation optimized for desert patterns
 * Creates natural sand dune flows and wind-carved formations
 */
class DesertDomainWarping {
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

  static warpDomain(
    x: number,
    y: number,
    scale: number = 1,
    strength: number = 0.2
  ): [number, number] {
    // Desert-specific warping for wind patterns
    const warpX = this.fbm(x * scale, y * scale, 4) * strength;
    const warpY = this.fbm((x + 7.1) * scale, (y + 2.7) * scale, 4) * strength;

    // Add directional wind bias (prevailing wind from southwest)
    const windBias = 0.08;
    const windAngle = Math.PI * 0.75; // Southwest wind
    const windX = Math.cos(windAngle) * windBias;
    const windY = Math.sin(windAngle) * windBias;

    return [x + warpX + windX, y + warpY + windY];
  }
}

/**
 * Sand ripple noise for creating realistic wind-carved surface patterns
 */
class SandRippleNoise {
  static generate(x: number, y: number, octaves: number = 8): number {
    let value = 0;
    let amplitude = 1;
    let frequency = 1;
    let weight = 1;

    for (let i = 0; i < octaves; i++) {
      // Create anisotropic noise for wind direction
      const windDirection = Math.PI * 0.75; // Southwest wind
      const rotX = x * Math.cos(windDirection) - y * Math.sin(windDirection);
      const rotY = x * Math.sin(windDirection) + y * Math.cos(windDirection);

      let signal = Math.abs(
        DesertDomainWarping['noise'](rotX * frequency, rotY * frequency * 0.3)
      );
      signal = 1 - signal;
      signal *= signal * weight;

      value += signal * amplitude;
      weight = MathUtils.clamp(signal * 2, 0, 1);
      amplitude *= 0.6;
      frequency *= 1.8;
    }

    return value;
  }
}

/**
 * Rock outcrop placement using advanced cellular noise
 */
class RockOutcropNoise {
  static generate(x: number, y: number, scale: number = 4): number {
    const points = [];
    const cellSize = 1 / scale;

    // Generate rock center points
    for (let xi = -1; xi <= 1; xi++) {
      for (let yi = -1; yi <= 1; yi++) {
        const cellX = Math.floor(x * scale) + xi;
        const cellY = Math.floor(y * scale) + yi;

        const hash = ((cellX * 127.1 + cellY * 311.7) % 1000) / 1000;
        const pointX = cellX * cellSize + hash * cellSize;
        const pointY =
          cellY * cellSize + (((hash * 269.5) % 1000) / 1000) * cellSize;

        points.push({ x: pointX, y: pointY, size: hash });
      }
    }

    // Create rock formation with realistic fall-off
    let rockValue = 0;
    for (const point of points) {
      const dist = Math.hypot(x - point.x, y - point.y);
      const rockRadius = 0.05 + point.size * 0.08;

      if (dist < rockRadius) {
        const falloff = Math.cos((dist / rockRadius) * Math.PI * 0.5);
        rockValue = Math.max(rockValue, falloff * falloff);
      }
    }

    return rockValue;
  }
}

/**
 * Dune formation noise for large-scale desert topography
 */
class DuneFormationNoise {
  static generate(x: number, y: number): number {
    // Primary dune waves (large scale)
    const primary =
      Math.sin(x * Math.PI * 2) * Math.cos(y * Math.PI * 0.8) * 0.6;

    // Secondary dune crests
    const secondary = Math.sin(x * Math.PI * 6 + Math.PI * 0.3) * 0.3;

    // Wind-carved variation
    const windVariation = DesertDomainWarping['fbm'](x * 3, y * 2, 3) * 0.2;

    // Combine and normalize
    return MathUtils.clamp(
      (primary + secondary + windVariation + 1) * 0.5,
      0,
      1
    );
  }
}

// ============================================================================
// DESERT MATERIAL PROPERTIES
// ============================================================================

interface DesertVariation {
  name: string;
  sandColor: Color;
  rockColor: Color;
  darkSandColor: Color;
  roughness: number;
  metallic: number;
  normalStrength: number;
  duneHeight: number;
  rockDensity: number;
}

const DESERT_VARIATIONS: DesertVariation[] = [
  {
    name: 'sahara_desert',
    sandColor: new Color(0.76, 0.65, 0.45), // Warm sand
    rockColor: new Color(0.35, 0.28, 0.22), // Weathered rock
    darkSandColor: new Color(0.55, 0.45, 0.32), // Shadow areas
    roughness: 0.85,
    metallic: 0.0,
    normalStrength: 1.1,
    duneHeight: 1.0,
    rockDensity: 0.15,
  },
  {
    name: 'gobi_desert',
    sandColor: new Color(0.72, 0.58, 0.42), // Cooler sand tones
    rockColor: new Color(0.45, 0.35, 0.28),
    darkSandColor: new Color(0.52, 0.4, 0.28),
    roughness: 0.9,
    metallic: 0.0,
    normalStrength: 1.2,
    duneHeight: 0.8,
    rockDensity: 0.25,
  },
  {
    name: 'mojave_desert',
    sandColor: new Color(0.68, 0.55, 0.38),
    rockColor: new Color(0.42, 0.32, 0.25),
    darkSandColor: new Color(0.48, 0.35, 0.24),
    roughness: 0.88,
    metallic: 0.02,
    normalStrength: 1.0,
    duneHeight: 0.6,
    rockDensity: 0.35,
  },
  {
    name: 'red_desert',
    sandColor: new Color(0.72, 0.48, 0.32), // Iron oxide tinting
    rockColor: new Color(0.48, 0.25, 0.18),
    darkSandColor: new Color(0.55, 0.35, 0.22),
    roughness: 0.82,
    metallic: 0.05,
    normalStrength: 1.3,
    duneHeight: 1.2,
    rockDensity: 0.2,
  },
];

// ============================================================================
// ADVANCED DESERT TEXTURE GENERATOR
// ============================================================================

export class AdvancedDesertGenerator {
  private resolution: number;
  private variation: DesertVariation;
  private environmentalFactors: {
    moisture: number; // 0-1, affects oasis presence and vegetation
    temperature: number; // 0-1, affects sand color and rock weathering
    windStrength: number; // 0-1, affects erosion patterns
    season: number; // 0-1, affects atmospheric conditions
    elevation: number; // 0-1, affects rock exposure
  };

  constructor(
    resolution: number = 1024,
    variationName: string = 'sahara_desert',
    environmentalFactors = {
      moisture: 0.1,
      temperature: 0.9,
      windStrength: 0.7,
      season: 0.5,
      elevation: 0.4,
    }
  ) {
    this.resolution = resolution;
    this.variation =
      DESERT_VARIATIONS.find(v => v.name === variationName) ||
      DESERT_VARIATIONS[0];
    this.environmentalFactors = environmentalFactors;
  }

  /**
   * Generate complete PBR material set for desert terrain
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
   * Generate high-quality albedo map with sand, rock, and atmospheric effects
   */
  private generateAlbedoMap(): ImageData {
    const canvas = new OffscreenCanvas(this.resolution, this.resolution);
    const ctx = canvas.getContext('2d')!;
    const imageData = ctx.createImageData(this.resolution, this.resolution);
    const { data } = imageData;

    for (let y = 0; y < this.resolution; y++) {
      for (let x = 0; x < this.resolution; x++) {
        const u = x / this.resolution;
        const v = y / this.resolution;

        // Apply domain warping for wind erosion patterns
        const [warpedU, warpedV] = DesertDomainWarping.warpDomain(
          u,
          v,
          3,
          0.15
        );

        // Multi-scale desert features
        const duneFormation = DuneFormationNoise.generate(warpedU, warpedV);
        const sandRipples =
          SandRippleNoise.generate(warpedU * 32, warpedV * 32, 6) * 0.4;
        const rockOutcrops = RockOutcropNoise.generate(warpedU, warpedV, 8);
        const fineDetail =
          DesertDomainWarping['fbm'](warpedU * 64, warpedV * 64, 4) * 0.15;

        // Environmental variation
        const temperatureNoise =
          DesertDomainWarping['fbm'](u * 1.5, v * 1.5, 2) * 0.2 + 0.8;
        const actualTemperature =
          this.environmentalFactors.temperature * temperatureNoise;

        // Wind strength affects erosion patterns
        const windErosion =
          this.environmentalFactors.windStrength *
          SandRippleNoise.generate(warpedU * 16, warpedV * 16, 4) *
          0.2;

        // Moisture creates subtle vegetation or oasis hints
        const moistureVariation =
          this.environmentalFactors.moisture > 0.3
            ? DesertDomainWarping['fbm'](warpedU * 8, warpedV * 8, 2) * 0.1
            : 0;

        // Determine primary surface type
        let finalColor: Color;
        let surfaceType: 'sand' | 'rock' | 'darkSand';

        if (rockOutcrops > 0.3) {
          // Rock outcrop areas
          surfaceType = 'rock';
          finalColor = this.variation.rockColor.clone();

          // Add weathering based on temperature and wind
          const weathering =
            actualTemperature * this.environmentalFactors.windStrength * 0.3;
          finalColor = finalColor.lerp(this.variation.sandColor, weathering);
        } else if (duneFormation < 0.3 || sandRipples > 0.6) {
          // Shadow areas or wind-carved depressions
          surfaceType = 'darkSand';
          finalColor = this.variation.darkSandColor.clone();
        } else {
          // Primary sand areas
          surfaceType = 'sand';
          finalColor = this.variation.sandColor.clone();

          // Temperature affects sand color (hotter = lighter)
          const temperatureEffect = (actualTemperature - 0.5) * 0.15;
          finalColor.r = MathUtils.clamp(
            finalColor.r + temperatureEffect,
            0,
            1
          );
          finalColor.g = MathUtils.clamp(
            finalColor.g + temperatureEffect,
            0,
            1
          );
          finalColor.b = MathUtils.clamp(
            finalColor.b + temperatureEffect * 0.5,
            0,
            1
          );
        }

        // Apply environmental modulation
        if (moistureVariation > 0.05) {
          // Subtle vegetation tinting
          const vegetationTint = new Color(0.1, 0.15, 0.08);
          finalColor = finalColor.lerp(vegetationTint, moistureVariation * 0.3);
        }

        // Add atmospheric haze effect
        const atmosphericHaze = Math.pow(actualTemperature, 2) * 0.1;
        finalColor.r = MathUtils.clamp(finalColor.r + atmosphericHaze, 0, 1);
        finalColor.g = MathUtils.clamp(
          finalColor.g + atmosphericHaze * 0.8,
          0,
          1
        );
        finalColor.b = MathUtils.clamp(
          finalColor.b + atmosphericHaze * 0.6,
          0,
          1
        );

        // Add surface detail variation
        const detailVariation = fineDetail + windErosion;
        finalColor.r = MathUtils.clamp(
          finalColor.r * (0.85 + detailVariation * 0.3),
          0,
          1
        );
        finalColor.g = MathUtils.clamp(
          finalColor.g * (0.85 + detailVariation * 0.3),
          0,
          1
        );
        finalColor.b = MathUtils.clamp(
          finalColor.b * (0.85 + detailVariation * 0.3),
          0,
          1
        );

        // Seasonal dust storm effects
        if (this.environmentalFactors.season > 0.7) {
          const dustTint = new Color(0.15, 0.12, 0.08);
          finalColor = finalColor.lerp(
            dustTint,
            (this.environmentalFactors.season - 0.7) * 0.4
          );
        }

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
   * Generate detailed normal map for sand ripples and rock details
   */
  private generateNormalMap(): ImageData {
    const canvas = new OffscreenCanvas(this.resolution, this.resolution);
    const ctx = canvas.getContext('2d')!;
    const imageData = ctx.createImageData(this.resolution, this.resolution);
    const { data } = imageData;

    const heightScale = this.variation.normalStrength * 0.02;

    for (let y = 0; y < this.resolution; y++) {
      for (let x = 0; x < this.resolution; x++) {
        const u = x / this.resolution;
        const v = y / this.resolution;

        // Sample height at current and neighboring pixels
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

        // Convert to [0, 255] range (this creates the purple/blue normal map appearance)
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
    const [warpedU, warpedV] = DesertDomainWarping.warpDomain(u, v, 3);

    // Sand ripple detail (fine surface patterns)
    const rippleDetail =
      SandRippleNoise.generate(warpedU * 128, warpedV * 128, 8) * 0.4;

    // Dune height variation (large scale)
    const duneHeight = DuneFormationNoise.generate(warpedU, warpedV) * 0.5;

    // Rock outcrop height
    const rockHeight = RockOutcropNoise.generate(warpedU, warpedV, 6) * 0.8;

    // Fine grain detail
    const grainDetail =
      DesertDomainWarping['fbm'](warpedU * 256, warpedV * 256, 5) * 0.15;

    // Wind-carved erosion channels
    const erosionChannels =
      SandRippleNoise.generate(warpedU * 64, warpedV * 64, 4) *
      this.environmentalFactors.windStrength *
      0.2;

    return (
      rippleDetail + duneHeight + rockHeight + grainDetail - erosionChannels
    );
  }

  /**
   * Generate roughness map with realistic desert surface properties
   */
  private generateRoughnessMap(): ImageData {
    const canvas = new OffscreenCanvas(this.resolution, this.resolution);
    const ctx = canvas.getContext('2d')!;
    const imageData = ctx.createImageData(this.resolution, this.resolution);
    const { data } = imageData;

    for (let y = 0; y < this.resolution; y++) {
      for (let x = 0; x < this.resolution; x++) {
        const u = x / this.resolution;
        const v = y / this.resolution;

        const [warpedU, warpedV] = DesertDomainWarping.warpDomain(u, v, 2);

        // Base roughness from variation
        let { roughness } = this.variation;

        // Rock areas are rougher than sand
        const rockInfluence = RockOutcropNoise.generate(warpedU, warpedV, 6);
        if (rockInfluence > 0.2) {
          roughness = MathUtils.lerp(roughness, 0.95, rockInfluence);
        }

        // Wind polishing effect (reduces roughness in exposed areas)
        const windPolishing =
          this.environmentalFactors.windStrength *
          SandRippleNoise.generate(warpedU * 8, warpedV * 8, 3) *
          0.15;
        roughness = MathUtils.clamp(roughness - windPolishing, 0.3, 1);

        // Temperature affects surface texture (heat cracking)
        const temperatureRoughening =
          this.environmentalFactors.temperature > 0.8
            ? DesertDomainWarping['fbm'](warpedU * 32, warpedV * 32, 3) * 0.1
            : 0;
        roughness = MathUtils.clamp(roughness + temperatureRoughening, 0, 1);

        // Moisture slightly reduces roughness (rare in desert but possible)
        if (this.environmentalFactors.moisture > 0.4) {
          const moistureSmoothing =
            (this.environmentalFactors.moisture - 0.4) * 0.1;
          roughness = MathUtils.clamp(roughness - moistureSmoothing, 0, 1);
        }

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
   * Generate metallic map (desert materials are generally non-metallic)
   */
  private generateMetallicMap(): ImageData {
    const canvas = new OffscreenCanvas(this.resolution, this.resolution);
    const ctx = canvas.getContext('2d')!;
    const imageData = ctx.createImageData(this.resolution, this.resolution);
    const { data } = imageData;

    for (let y = 0; y < this.resolution; y++) {
      for (let x = 0; x < this.resolution; x++) {
        const u = x / this.resolution;
        const v = y / this.resolution;

        // Base metallic from variation (usually near zero)
        let { metallic } = this.variation;

        // Some desert types (like red desert) have mineral content
        if (this.variation.name === 'red_desert') {
          const mineralDeposits = RockOutcropNoise.generate(u, v, 12) * 0.08;
          metallic = Math.min(metallic + mineralDeposits, 0.15);
        }

        // Rare metallic flecks from mica or other minerals
        const mineralFlecks = DesertDomainWarping['fbm'](u * 64, v * 64, 2);
        if (mineralFlecks > 0.8) {
          metallic = Math.min(metallic + 0.05, 0.1);
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
   * Generate height/displacement map for dune and rock formations
   */
  private generateHeightMap(): ImageData {
    const canvas = new OffscreenCanvas(this.resolution, this.resolution);
    const ctx = canvas.getContext('2d')!;
    const imageData = ctx.createImageData(this.resolution, this.resolution);
    const { data } = imageData;

    for (let y = 0; y < this.resolution; y++) {
      for (let x = 0; x < this.resolution; x++) {
        const u = x / this.resolution;
        const v = y / this.resolution;

        const height =
          this.sampleHeightForNormal(u, v) * this.variation.duneHeight;
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
   */
  generateBlendingMask(
    adjacentTerrain: 'river' | 'mountain' | 'forest' | 'grassland'
  ): ImageData {
    const canvas = new OffscreenCanvas(this.resolution, this.resolution);
    const ctx = canvas.getContext('2d')!;
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
            // Oasis-like transition near water
            const distToWater = Math.min(u, 1 - u, v, 1 - v);
            const oasisTransition = MathUtils.smoothstep(0, 0.4, distToWater);
            blendFactor =
              oasisTransition *
              (0.7 + this.environmentalFactors.moisture * 0.3);
            break;

          case 'mountain':
            // Rocky transition with elevation changes
            const elevationNoise = DesertDomainWarping['fbm'](u * 3, v * 3, 4);
            const rockTransition = RockOutcropNoise.generate(u, v, 4);
            blendFactor = MathUtils.clamp(
              0.8 + elevationNoise * 0.2 + rockTransition * 0.3,
              0,
              1
            );
            break;

          case 'forest':
            // Sparse vegetation at forest edge
            const vegetationTransition = Math.min(u, 1 - u, v, 1 - v);
            const moistureBlend = this.environmentalFactors.moisture * 0.6;
            blendFactor =
              MathUtils.smoothstep(0, 0.5, vegetationTransition) *
              moistureBlend;
            break;

          case 'grassland':
            // Natural transition to sparse grassland
            const grasslandDistance = Math.min(u, 1 - u);
            const humidityGradient = this.environmentalFactors.moisture * 0.8;
            blendFactor =
              MathUtils.smoothstep(0, 0.3, grasslandDistance) *
              humidityGradient;
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
  return new Promise(resolve => {
    const canvas = new OffscreenCanvas(imageData.width, imageData.height);
    const ctx = canvas.getContext('2d')!;
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
 * Generate and export complete desert texture set
 */
export const generateDesertTextureSet = async (
  variation: string = 'sahara_desert',
  resolution: number = 1024,
  environmentalFactors?: any
): Promise<{
  albedo: Blob;
  normal: Blob;
  roughness: Blob;
  metallic: Blob;
  height: Blob;
}> => {
  const generator = new AdvancedDesertGenerator(
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
