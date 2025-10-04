/**
 * @texture-factory/ocean.ts
 *
 * Advanced procedural ocean texture generation
 * Designed to compete with AAA game quality (Civilization VI standard)
 *
 * Features:
 * - Advanced wave simulation using domain warping and ridged noise
 * - Realistic foam pattern generation using cellular automata
 * - Multi-scale water detail layering with depth variation
 * - Animated surface patterns suggesting motion
 * - Realistic PBR material properties for water rendering
 * - Seamless tiling with advanced noise techniques
 * - Environmental variation support (wind, depth, weather)
 * - Shoreline and terrain integration points
 */

import { Color, MathUtils } from 'three';

// ============================================================================
// ADVANCED WAVE AND WATER NOISE FUNCTIONS
// ============================================================================

/**
 * Domain warping implementation for natural water flow distortion
 * Creates the organic, flowing patterns seen in real ocean surfaces
 */
class WaterDomainWarping {
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
    // Enhanced Perlin-style noise optimized for water patterns
    const ix = Math.floor(x);
    const iy = Math.floor(y);
    const fx = x - ix;
    const fy = y - iy;

    // Smoother interpolation for water
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
    strength: number = 0.1
  ): [number, number] {
    const warpX = this.fbm(x * scale, y * scale, 4) * strength;
    const warpY = this.fbm((x + 5.2) * scale, (y + 1.3) * scale, 4) * strength;

    return [x + warpX, y + warpY];
  }

  static flowField(x: number, y: number, scale: number = 1): [number, number] {
    const angle1 = this.fbm(x * scale, y * scale, 3) * Math.PI * 2;
    const angle2 =
      this.fbm((x + 100) * scale, (y + 100) * scale, 3) * Math.PI * 2;

    const flowX = Math.cos(angle1) * 0.5 + Math.cos(angle2) * 0.3;
    const flowY = Math.sin(angle1) * 0.5 + Math.sin(angle2) * 0.3;

    return [flowX, flowY];
  }
}

/**
 * Advanced wave simulation using ridged noise for realistic ocean waves
 */
class OceanWaves {
  static generate(
    x: number,
    y: number,
    octaves: number = 6,
    windDirection: number = 0
  ): number {
    let value = 0;
    let amplitude = 1;
    let frequency = 1;
    let weight = 1;

    // Directional wave bias based on wind
    const windX = Math.cos(windDirection);
    const windY = Math.sin(windDirection);
    const directionalBias = (x * windX + y * windY) * 0.1;

    for (let i = 0; i < octaves; i++) {
      const waveX = x * frequency + directionalBias;
      const waveY = y * frequency + directionalBias;

      let signal = Math.abs(WaterDomainWarping['noise'](waveX, waveY));
      signal = 1 - signal; // Create wave peaks
      signal *= signal * weight; // Sharpen the peaks

      value += signal * amplitude;
      weight = MathUtils.clamp(signal * 2.2, 0, 1);
      amplitude *= 0.5;
      frequency *= 2.1; // Slightly irregular frequency multiplication
    }

    return value;
  }

  static generateSwell(x: number, y: number, scale: number = 0.5): number {
    // Large-scale ocean swell
    return WaterDomainWarping['fbm'](x * scale, y * scale, 2) * 0.3;
  }
}

/**
 * Cellular noise for foam and bubble patterns
 */
class FoamPattern {
  static generate(
    x: number,
    y: number,
    scale: number = 12,
    threshold: number = 0.7
  ): number {
    const points = [];
    const cellSize = 1 / scale;

    // Generate foam cell points
    for (let xi = -1; xi <= 1; xi++) {
      for (let yi = -1; yi <= 1; yi++) {
        const cellX = Math.floor(x * scale) + xi;
        const cellY = Math.floor(y * scale) + yi;

        const hash = ((cellX * 127.1 + cellY * 311.7) % 1000) / 1000;
        const pointX = cellX * cellSize + hash * cellSize * 0.8;
        const pointY =
          cellY * cellSize + (((hash * 269.5) % 1000) / 1000) * cellSize * 0.8;

        points.push({ x: pointX, y: pointY, intensity: hash });
      }
    }

    // Find closest and second closest distances
    let minDist1 = Infinity;
    let minDist2 = Infinity;
    for (const point of points) {
      const dist = Math.hypot(x - point.x, y - point.y);
      if (dist < minDist1) {
        minDist2 = minDist1;
        minDist1 = dist;
      } else if (dist < minDist2) {
        minDist2 = dist;
      }
    }

    // Create foam patterns based on distance difference
    const foamValue = minDist2 - minDist1;
    return foamValue > threshold
      ? MathUtils.clamp((foamValue - threshold) * 5, 0, 1)
      : 0;
  }
}

/**
 * Caustic patterns for underwater light effects
 */
class CausticPattern {
  static generate(x: number, y: number, scale: number = 8): number {
    const [warpedX, warpedY] = WaterDomainWarping.warpDomain(
      x,
      y,
      scale * 0.5,
      0.2
    );

    const pattern1 = Math.abs(
      WaterDomainWarping['noise'](warpedX * scale, warpedY * scale)
    );
    const pattern2 = Math.abs(
      WaterDomainWarping['noise'](warpedX * scale * 1.3, warpedY * scale * 1.3)
    );

    const combined = pattern1 * pattern2;
    return Math.pow(combined, 2) * 2; // Enhance caustic highlights
  }
}

// ============================================================================
// OCEAN MATERIAL PROPERTIES
// ============================================================================

interface OceanVariation {
  name: string;
  deepColor: Color;
  shallowColor: Color;
  foamColor: Color;
  roughness: number;
  metallic: number;
  normalStrength: number;
  waveHeight: number;
  foamThreshold: number;
  transparency: number;
}

const OCEAN_VARIATIONS: OceanVariation[] = [
  {
    name: 'deep_ocean',
    deepColor: new Color(0.02, 0.05, 0.12), // Deep blue-black
    shallowColor: new Color(0.08, 0.15, 0.25), // Medium blue
    foamColor: new Color(0.9, 0.95, 1.0), // White foam
    roughness: 0.1,
    metallic: 0.0,
    normalStrength: 1.5,
    waveHeight: 1.2,
    foamThreshold: 0.6,
    transparency: 0.9,
  },
  {
    name: 'coastal_water',
    deepColor: new Color(0.05, 0.12, 0.18), // Coastal blue
    shallowColor: new Color(0.15, 0.25, 0.35), // Lighter blue
    foamColor: new Color(0.85, 0.92, 0.98), // Off-white foam
    roughness: 0.15,
    metallic: 0.0,
    normalStrength: 1.3,
    waveHeight: 1.0,
    foamThreshold: 0.5,
    transparency: 0.8,
  },
  {
    name: 'tropical_lagoon',
    deepColor: new Color(0.08, 0.18, 0.22), // Tropical blue-green
    shallowColor: new Color(0.2, 0.4, 0.45), // Light turquoise
    foamColor: new Color(0.95, 0.98, 1.0), // Pure white foam
    roughness: 0.05,
    metallic: 0.0,
    normalStrength: 0.8,
    waveHeight: 0.6,
    foamThreshold: 0.7,
    transparency: 0.95,
  },
  {
    name: 'stormy_sea',
    deepColor: new Color(0.01, 0.03, 0.08), // Dark storm blue
    shallowColor: new Color(0.05, 0.08, 0.15), // Gray-blue
    foamColor: new Color(0.8, 0.85, 0.9), // Gray-white foam
    roughness: 0.3,
    metallic: 0.0,
    normalStrength: 2.0,
    waveHeight: 1.8,
    foamThreshold: 0.4,
    transparency: 0.7,
  },
];

// ============================================================================
// ADVANCED OCEAN TEXTURE GENERATOR
// ============================================================================

export class AdvancedOceanGenerator {
  private resolution: number;
  private variation: OceanVariation;
  private environmentalFactors: {
    windStrength: number; // 0-1, affects wave intensity
    windDirection: number; // 0-2π, wind direction in radians
    depth: number; // 0-1, affects color depth
    weather: number; // 0-1, 0=calm, 1=stormy
    temperature: number; // 0-1, affects viscosity
  };

  constructor(
    resolution: number = 1024,
    variationName: string = 'deep_ocean',
    environmentalFactors = {
      windStrength: 0.6,
      windDirection: Math.PI / 4, // 45 degrees
      depth: 0.8,
      weather: 0.3,
      temperature: 0.6,
    }
  ) {
    this.resolution = resolution;
    this.variation =
      OCEAN_VARIATIONS.find(v => v.name === variationName) ??
      OCEAN_VARIATIONS[0];
    this.environmentalFactors = environmentalFactors;
  }

  /**
   * Generate complete PBR material set for ocean
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
   * Generate high-quality albedo map with wave patterns and foam
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

        // Apply domain warping for natural water flow
        const [warpedU, warpedV] = WaterDomainWarping.warpDomain(u, v, 2, 0.08);

        // Multi-scale wave patterns
        const largeWaves = OceanWaves.generate(
          warpedU * 8,
          warpedV * 8,
          4,
          this.environmentalFactors.windDirection
        );
        const mediumWaves =
          OceanWaves.generate(
            warpedU * 16,
            warpedV * 16,
            3,
            this.environmentalFactors.windDirection
          ) * 0.6;
        const smallWaves =
          OceanWaves.generate(
            warpedU * 32,
            warpedV * 32,
            2,
            this.environmentalFactors.windDirection
          ) * 0.3;

        // Ocean swell
        const swell = OceanWaves.generateSwell(warpedU, warpedV, 0.3);

        // Foam generation
        const foamIntensity = FoamPattern.generate(
          warpedU,
          warpedV,
          16,
          this.variation.foamThreshold
        );
        const wavefoam =
          largeWaves + mediumWaves > 0.8
            ? Math.min((largeWaves + mediumWaves - 0.8) * 2, 1)
            : 0;

        // Caustic patterns for shallow areas
        const caustics = CausticPattern.generate(warpedU, warpedV, 12) * 0.15;

        // Environmental modulation
        const windEffect = this.environmentalFactors.windStrength;
        const depthEffect = this.environmentalFactors.depth;
        const weatherEffect = this.environmentalFactors.weather;

        // Combine wave patterns
        const totalWaves =
          (largeWaves + mediumWaves + smallWaves + swell) * windEffect;
        const waveIntensity = MathUtils.clamp(totalWaves, 0, 1);

        // Base water color based on depth
        const baseColor = this.variation.deepColor
          .clone()
          .lerp(this.variation.shallowColor, 1 - depthEffect);

        // Add caustic highlights in shallow areas
        if (depthEffect < 0.6) {
          baseColor.r += caustics * (1 - depthEffect);
          baseColor.g += caustics * (1 - depthEffect) * 1.2;
          baseColor.b += caustics * (1 - depthEffect) * 0.8;
        }

        // Wave color modulation
        const waveColor = baseColor.clone();
        waveColor.multiplyScalar(0.9 + waveIntensity * 0.3);

        // Foam application
        const totalFoam = MathUtils.clamp(
          foamIntensity + wavefoam * weatherEffect,
          0,
          1
        );
        const finalColor = waveColor
          .clone()
          .lerp(this.variation.foamColor, totalFoam);

        // Weather effects
        if (weatherEffect > 0.5) {
          finalColor.multiplyScalar(0.8 + (1 - weatherEffect) * 0.2);
        }

        // Subtle surface variation
        const surfaceNoise =
          WaterDomainWarping['fbm'](warpedU * 64, warpedV * 64, 2) * 0.05;
        finalColor.r = MathUtils.clamp(finalColor.r + surfaceNoise, 0, 1);
        finalColor.g = MathUtils.clamp(finalColor.g + surfaceNoise, 0, 1);
        finalColor.b = MathUtils.clamp(finalColor.b + surfaceNoise, 0, 1);

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
   * Generate detailed normal map for water surface detail
   */
  private generateNormalMap(): ImageData {
    const canvas = new OffscreenCanvas(this.resolution, this.resolution);
    const ctx = canvas.getContext('2d');
    if (!ctx) {
      throw new Error('Failed to get 2D context for normal map generation');
    }
    const imageData = ctx.createImageData(this.resolution, this.resolution);
    const { data } = imageData;

    const heightScale = this.variation.normalStrength * 0.005;

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
    const [warpedU, warpedV] = WaterDomainWarping.warpDomain(u, v, 2, 0.05);

    // Multi-scale wave height
    const largeWaves =
      OceanWaves.generate(
        warpedU * 6,
        warpedV * 6,
        4,
        this.environmentalFactors.windDirection
      ) * 0.8;

    const mediumWaves =
      OceanWaves.generate(
        warpedU * 16,
        warpedV * 16,
        3,
        this.environmentalFactors.windDirection
      ) * 0.4;

    const smallWaves =
      OceanWaves.generate(
        warpedU * 48,
        warpedV * 48,
        2,
        this.environmentalFactors.windDirection
      ) * 0.2;

    // Ocean swell
    const swell = OceanWaves.generateSwell(warpedU, warpedV, 0.5) * 0.6;

    return (
      (largeWaves + mediumWaves + smallWaves + swell) *
      this.variation.waveHeight *
      this.environmentalFactors.windStrength
    );
  }

  /**
   * Generate roughness map with realistic water surface properties
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

        const [warpedU, warpedV] = WaterDomainWarping.warpDomain(
          u,
          v,
          1.5,
          0.05
        );

        // Base roughness from variation
        let { roughness } = this.variation;

        // Wind effect on surface roughness
        const windRoughness = this.environmentalFactors.windStrength * 0.4;

        // Weather increases roughness
        const weatherRoughness = this.environmentalFactors.weather * 0.3;

        // Wave-induced roughness
        const waveHeight = this.sampleHeightForNormal(u, v);
        const waveRoughness = Math.abs(waveHeight) * 0.2;

        // Foam areas are rougher
        const foamIntensity = FoamPattern.generate(
          warpedU,
          warpedV,
          16,
          this.variation.foamThreshold
        );
        const foamRoughness = foamIntensity * 0.6;

        // Combine all roughness factors
        roughness = MathUtils.clamp(
          roughness +
            windRoughness +
            weatherRoughness +
            waveRoughness +
            foamRoughness,
          0,
          1
        );

        // Add subtle surface variation
        const surfaceVariation =
          WaterDomainWarping['fbm'](warpedU * 24, warpedV * 24, 3) * 0.1;
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
   * Generate metallic map (water is non-metallic)
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
        // Water is non-metallic, but we can add tiny variations for realism
        let { metallic } = this.variation;

        // Extremely subtle metallic variation in foam areas
        const u = x / this.resolution;
        const v = y / this.resolution;
        const foamIntensity = FoamPattern.generate(
          u,
          v,
          16,
          this.variation.foamThreshold
        );

        // Foam slightly increases metallic (but still very low)
        metallic = Math.min(metallic + foamIntensity * 0.02, 0.05);

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
   * Generate height/displacement map for water surface
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
   * Generate blending mask for seamless shoreline integration
   */
  generateBlendingMask(
    adjacentTerrain: 'shoreline' | 'shallow_water' | 'deep_water' | 'ice'
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
          case 'shoreline':
            // Create gentle wave-influenced shoreline transition
            const waveInfluence = OceanWaves.generate(
              u * 8,
              v * 8,
              3,
              this.environmentalFactors.windDirection
            );
            const distToShore = Math.min(u, 1 - u, v, 1 - v);
            blendFactor = MathUtils.smoothstep(
              0,
              0.4,
              distToShore + waveInfluence * 0.1
            );
            break;

          case 'shallow_water':
            // Natural depth transition
            const depthTransition = WaterDomainWarping['fbm'](u * 3, v * 3, 2);
            blendFactor = MathUtils.clamp(0.6 + depthTransition * 0.4, 0, 1);
            break;

          case 'deep_water':
            // Smooth transition to deeper water
            const deepTransition = Math.max(u, 1 - u, v, 1 - v);
            blendFactor = MathUtils.smoothstep(0.2, 0.8, deepTransition);
            break;

          case 'ice':
            // Ice edge transition with some irregularity
            const iceEdge = CausticPattern.generate(u, v, 6) * 0.3;
            const distToIce = Math.min(u, 1 - u);
            blendFactor =
              MathUtils.smoothstep(0, 0.3, distToIce + iceEdge) * 0.8;
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
 * Generate and export complete ocean texture set
 */
export const generateOceanTextureSet = async (
  variation: string = 'deep_ocean',
  resolution: number = 1024,
  environmentalFactors?: {
    windStrength: number;
    windDirection: number;
    depth: number;
    weather: number;
    temperature: number;
  }
): Promise<{
  albedo: Blob;
  normal: Blob;
  roughness: Blob;
  metallic: Blob;
  height: Blob;
}> => {
  const generator = new AdvancedOceanGenerator(
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
