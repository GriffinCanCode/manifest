/**
 * @texture-factory/mountain.ts
 *
 * Advanced procedural mountain texture generation
 * Designed to compete with AAA game quality (Civilization VI standard)
 *
 * Features:
 * - Geological layering with stratified noise
 * - Erosion channel simulation using flow fields
 * - Multi-scale rock face weathering
 * - Snow line elevation mapping
 * - Realistic PBR material properties
 * - Seamless tiling with advanced terrain integration
 * - Environmental variation support (elevation, erosion, weathering)
 */

import { Color, MathUtils } from 'three';

// ============================================================================
// ADVANCED GEOLOGICAL NOISE FUNCTIONS
// ============================================================================

/**
 * Stratified noise for realistic geological layering
 * Simulates sedimentary rock formations and mineral veins
 */
class StratifiedNoise {
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
    // Improved Perlin-style noise optimized for geological patterns
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

  static generate(x: number, y: number, scale: number = 1): number {
    // Create horizontal stratification typical of sedimentary rock
    const stratification = Math.sin(y * scale * 8) * 0.3;

    // Add geological noise for realistic variation
    const geoNoise = this.fbm(x * scale, y * scale, 4) * 0.4;

    // Combine for layered rock appearance
    return MathUtils.clamp(stratification + geoNoise, -1, 1);
  }
}

/**
 * Erosion simulation for realistic weathering patterns
 * Creates channels and flow patterns typical of mountain erosion
 */
class ErosionSimulation {
  static generateFlowField(
    x: number,
    y: number,
    scale: number = 1
  ): [number, number] {
    // Simulate water flow patterns using gradient noise
    const flowX = StratifiedNoise['fbm'](x * scale, y * scale, 3);
    const flowY = StratifiedNoise['fbm'](
      (x + 5.2) * scale,
      (y + 1.3) * scale,
      3
    );

    return [flowX, flowY];
  }

  static generateErosionChannels(
    x: number,
    y: number,
    scale: number = 4
  ): number {
    // Create branching erosion patterns
    let erosion = 0;

    // Large-scale drainage patterns
    const [flowX, flowY] = this.generateFlowField(x, y, scale * 0.1);
    const drainageIntensity = Math.hypot(flowX, flowY);
    erosion += drainageIntensity * 0.6;

    // Medium-scale gully formation
    const gullyNoise = Math.abs(
      StratifiedNoise['fbm'](x * scale * 0.5, y * scale * 0.5, 6)
    );
    erosion += (1 - gullyNoise) * 0.4;

    // Fine-scale weathering detail
    const weathering =
      StratifiedNoise['fbm'](x * scale * 2, y * scale * 2, 4) * 0.2;
    erosion += weathering;

    return MathUtils.clamp(erosion, 0, 1);
  }
}

/**
 * Domain warping optimized for mountain terrain
 * Creates realistic rock formation distortions
 */
class MountainDomainWarping {
  static warpDomain(x: number, y: number, scale: number = 1): [number, number] {
    const strength = 0.08; // Reduced from grassland for more structured rock formations

    // Primary geological warping
    const warpX = StratifiedNoise['fbm'](x * scale, y * scale, 3) * strength;
    const warpY =
      StratifiedNoise['fbm']((x + 7.3) * scale, (y + 2.1) * scale, 3) *
      strength;

    return [x + warpX, y + warpY];
  }
}

/**
 * Cellular noise optimized for rock formations and mineral deposits
 */
class RockFormationNoise {
  static generate(x: number, y: number, scale: number = 6): number {
    const points = [];
    const cellSize = 1 / scale;

    // Generate rock formation points
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

    // Create rock formation patterns using distance fields
    let minDist = Infinity;
    let secondMinDist = Infinity;

    for (const point of points) {
      const dist = Math.hypot(x - point.x, y - point.y);
      if (dist < minDist) {
        secondMinDist = minDist;
        minDist = dist;
      } else if (dist < secondMinDist) {
        secondMinDist = dist;
      }
    }

    // Create rock face patterns
    const rockPattern = secondMinDist - minDist;
    return MathUtils.clamp(rockPattern * scale * 3, 0, 1);
  }
}

// ============================================================================
// MOUNTAIN MATERIAL PROPERTIES
// ============================================================================

interface MountainVariation {
  name: string;
  rockColor: Color;
  weatheredColor: Color;
  snowColor: Color;
  roughness: number;
  metallic: number;
  normalStrength: number;
  elevationThreshold: number; // For snow line
  erosionResistance: number;
}

const MOUNTAIN_VARIATIONS: MountainVariation[] = [
  {
    name: 'rocky_peaks',
    rockColor: new Color(0.25, 0.22, 0.18), // Dark granite
    weatheredColor: new Color(0.35, 0.32, 0.28), // Weathered rock
    snowColor: new Color(0.95, 0.95, 0.98), // Clean snow
    roughness: 0.95,
    metallic: 0.02,
    normalStrength: 1.5,
    elevationThreshold: 0.7,
    erosionResistance: 0.8,
  },
  {
    name: 'weathered_hills',
    rockColor: new Color(0.3, 0.25, 0.2), // Weathered sandstone
    weatheredColor: new Color(0.4, 0.35, 0.25), // Heavily weathered
    snowColor: new Color(0.9, 0.9, 0.95), // Patchy snow
    roughness: 0.85,
    metallic: 0.0,
    normalStrength: 1.0,
    elevationThreshold: 0.8,
    erosionResistance: 0.4,
  },
  {
    name: 'alpine_ridges',
    rockColor: new Color(0.2, 0.18, 0.15), // Dark metamorphic rock
    weatheredColor: new Color(0.28, 0.25, 0.2), // Less weathered
    snowColor: new Color(0.98, 0.98, 1.0), // Pristine snow
    roughness: 0.9,
    metallic: 0.05, // Slight mineral content
    normalStrength: 1.8,
    elevationThreshold: 0.5,
    erosionResistance: 0.9,
  },
  {
    name: 'red_canyon',
    rockColor: new Color(0.45, 0.25, 0.15), // Red sandstone
    weatheredColor: new Color(0.5, 0.35, 0.2), // Oxidized weathering
    snowColor: new Color(0.9, 0.9, 0.9), // Rare snow
    roughness: 0.8,
    metallic: 0.1, // Iron oxide content
    normalStrength: 1.2,
    elevationThreshold: 0.9, // Rarely snowy
    erosionResistance: 0.3,
  },
];

// ============================================================================
// ADVANCED MOUNTAIN TEXTURE GENERATOR
// ============================================================================

export class AdvancedMountainGenerator {
  private resolution: number;
  private variation: MountainVariation;
  private environmentalFactors: {
    elevation: number; // 0-1, affects snow coverage and erosion patterns
    temperature: number; // 0-1, affects snow line and weathering
    precipitation: number; // 0-1, affects erosion intensity
    age: number; // 0-1, affects overall weathering (0=young sharp peaks, 1=old rounded)
  };

  constructor(
    resolution: number = 1024,
    variationName: string = 'rocky_peaks',
    environmentalFactors = {
      elevation: 0.8,
      temperature: 0.3,
      precipitation: 0.6,
      age: 0.5,
    }
  ) {
    this.resolution = resolution;
    this.variation =
      MOUNTAIN_VARIATIONS.find(v => v.name === variationName) ??
      MOUNTAIN_VARIATIONS[0];
    this.environmentalFactors = environmentalFactors;
  }

  /**
   * Generate complete PBR material set for mountain terrain
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
   * Generate high-quality albedo map with rock layers, weathering, and snow
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

        // Apply domain warping for natural rock formation distortion
        const [warpedU, warpedV] = MountainDomainWarping.warpDomain(u, v, 2);

        // Multi-scale geological features
        const stratification = StratifiedNoise.generate(warpedU, warpedV, 6);
        const rockFormations = RockFormationNoise.generate(warpedU, warpedV, 8);
        const erosionChannels = ErosionSimulation.generateErosionChannels(
          warpedU,
          warpedV,
          4
        );

        // Fine detail weathering
        const fineWeathering =
          StratifiedNoise['fbm'](warpedU * 64, warpedV * 64, 4) * 0.15;

        // Environmental elevation simulation
        const baseElevation = (stratification + rockFormations) * 0.5 + 0.5;
        const actualElevation = MathUtils.clamp(
          baseElevation +
            this.environmentalFactors.elevation * 0.3 -
            erosionChannels * 0.2,
          0,
          1
        );

        // Snow coverage based on elevation and temperature
        const snowThreshold =
          this.variation.elevationThreshold -
          this.environmentalFactors.temperature * 0.2;
        const snowCoverage = MathUtils.clamp(
          MathUtils.smoothstep(
            snowThreshold,
            snowThreshold + 0.1,
            actualElevation
          ),
          0,
          1
        );

        // Weathering intensity based on age and precipitation
        const weatheringIntensity =
          this.environmentalFactors.age * 0.6 +
          this.environmentalFactors.precipitation * 0.4 +
          erosionChannels * 0.3;

        // Color blending based on geological and environmental factors
        let finalColor: Color;

        if (snowCoverage > 0.3) {
          // Snow-covered areas
          const snowVariation =
            fineWeathering +
            StratifiedNoise['fbm'](warpedU * 32, warpedV * 32, 3) * 0.1;
          finalColor = this.variation.snowColor.clone();

          // Add subtle blue tinting for shadow areas
          if (snowVariation < -0.05) {
            finalColor.lerp(new Color(0.85, 0.9, 0.95), 0.2);
          }

          // Mix with underlying rock for patchy snow
          if (snowCoverage < 0.8) {
            const underlyingRock = this.variation.rockColor
              .clone()
              .lerp(this.variation.weatheredColor, weatheringIntensity);
            finalColor.lerp(underlyingRock, 1 - snowCoverage);
          }
        } else if (weatheringIntensity > 0.5) {
          // Heavily weathered rock
          finalColor = this.variation.weatheredColor.clone();

          // Add color variation based on stratification
          if (stratification > 0) {
            finalColor.lerp(this.variation.rockColor, stratification * 0.3);
          }
        } else {
          // Fresh rock face
          finalColor = this.variation.rockColor.clone();

          // Add mineral variation
          const mineralVariation = rockFormations * 0.2;
          finalColor.r = MathUtils.clamp(finalColor.r + mineralVariation, 0, 1);
          finalColor.g = MathUtils.clamp(
            finalColor.g + mineralVariation * 0.8,
            0,
            1
          );
          finalColor.b = MathUtils.clamp(
            finalColor.b + mineralVariation * 0.6,
            0,
            1
          );
        }

        // Apply environmental lighting variation
        const lightingVariation = 0.8 + fineWeathering * 0.4;
        finalColor.multiplyScalar(lightingVariation);

        // Erosion darkening for channels and crevices
        if (erosionChannels > 0.6) {
          finalColor.multiplyScalar(0.7 - erosionChannels * 0.2);
        }

        // Write to image data
        const idx = (y * this.resolution + x) * 4;
        data[idx] = Math.floor(MathUtils.clamp(finalColor.r, 0, 1) * 255); // R
        data[idx + 1] = Math.floor(MathUtils.clamp(finalColor.g, 0, 1) * 255); // G
        data[idx + 2] = Math.floor(MathUtils.clamp(finalColor.b, 0, 1) * 255); // B
        data[idx + 3] = 255; // A
      }
    }

    return imageData;
  }

  /**
   * Generate detailed normal map for rock face micro-detail and erosion patterns
   */
  private generateNormalMap(): ImageData {
    const canvas = new OffscreenCanvas(this.resolution, this.resolution);
    const ctx = canvas.getContext('2d')!;
    const imageData = ctx.createImageData(this.resolution, this.resolution);
    const { data } = imageData;

    const heightScale = this.variation.normalStrength * 0.015;

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

        // Convert to [0, 255] range (will appear purple/blue - this is correct!)
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
    const [warpedU, warpedV] = MountainDomainWarping.warpDomain(u, v, 2);

    // Large-scale mountain structure
    const mountainStructure =
      StratifiedNoise.generate(warpedU, warpedV, 3) * 0.8;

    // Rock formation detail
    const rockDetail = RockFormationNoise.generate(warpedU, warpedV, 12) * 0.4;

    // Erosion channels (negative contribution)
    const erosion =
      ErosionSimulation.generateErosionChannels(warpedU, warpedV, 8) * -0.3;

    // Fine surface detail
    const surfaceDetail =
      StratifiedNoise['fbm'](warpedU * 128, warpedV * 128, 6) * 0.2;

    return mountainStructure + rockDetail + erosion + surfaceDetail;
  }

  /**
   * Generate roughness map with realistic rock surface properties
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

        const [warpedU, warpedV] = MountainDomainWarping.warpDomain(u, v, 1);

        // Base roughness from variation
        let { roughness } = this.variation;

        // Modify based on weathering (weathered rock is rougher)
        const weatheringNoise = StratifiedNoise['fbm'](
          warpedU * 4,
          warpedV * 4,
          3
        );
        const weatheringFactor =
          this.environmentalFactors.age * (0.7 + weatheringNoise * 0.3);

        // Erosion channels are smoother (polished by water)
        const erosionChannels = ErosionSimulation.generateErosionChannels(
          warpedU,
          warpedV,
          6
        );
        const erosionSmoothening = erosionChannels * -0.15;

        // Snow is very smooth
        const baseElevation =
          (StratifiedNoise.generate(warpedU, warpedV, 4) + 1) * 0.5;
        const actualElevation =
          baseElevation + this.environmentalFactors.elevation * 0.2;
        const snowThreshold =
          this.variation.elevationThreshold -
          this.environmentalFactors.temperature * 0.2;
        const snowSmoothening =
          MathUtils.smoothstep(
            snowThreshold,
            snowThreshold + 0.1,
            actualElevation
          ) * -0.4;

        // Fine surface variation
        const surfaceVariation =
          StratifiedNoise['fbm'](warpedU * 32, warpedV * 32, 4) * 0.1;

        roughness = MathUtils.clamp(
          roughness +
            weatheringFactor * 0.1 +
            erosionSmoothening +
            snowSmoothening +
            surfaceVariation,
          0,
          1
        );

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
   * Generate metallic map (mountains have low metallicism except for mineral veins)
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

        const [warpedU, warpedV] = MountainDomainWarping.warpDomain(u, v, 1);

        // Base metallic value (generally very low for rock)
        let { metallic } = this.variation;

        // Add mineral veins using cellular noise
        const mineralVeins =
          RockFormationNoise.generate(warpedU, warpedV, 16) * 0.08;

        // Stratified mineral deposits
        const stratifiedMinerals =
          Math.abs(StratifiedNoise.generate(warpedU, warpedV, 12)) * 0.06;

        metallic = MathUtils.clamp(
          metallic + mineralVeins + stratifiedMinerals,
          0,
          0.2
        );

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
    const ctx = canvas.getContext('2d')!;
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
    adjacentTerrain: 'grassland' | 'forest' | 'desert' | 'tundra' | 'river'
  ): ImageData {
    const canvas = new OffscreenCanvas(this.resolution, this.resolution);
    const ctx = canvas.getContext('2d')!;
    const imageData = ctx.createImageData(this.resolution, this.resolution);
    const { data } = imageData;

    for (let y = 0; y < this.resolution; y++) {
      for (let x = 0; x < this.resolution; x++) {
        const u = x / this.resolution;
        const v = y / this.resolution;

        const [warpedU, warpedV] = MountainDomainWarping.warpDomain(u, v, 1);
        let blendFactor = 1.0;

        // Different blending strategies based on adjacent terrain
        switch (adjacentTerrain) {
          case 'grassland':
            // Gradual elevation transition from mountain to grassland
            const slopeNoise = StratifiedNoise['fbm'](
              warpedU * 2,
              warpedV * 2,
              3
            );
            const elevationBlend = Math.min(u, 1 - u, v, 1 - v);
            blendFactor = MathUtils.smoothstep(
              0,
              0.4,
              elevationBlend + slopeNoise * 0.1
            );
            break;

          case 'forest':
            // Trees can grow partway up mountains
            const forestElevation = StratifiedNoise['fbm'](
              warpedU * 3,
              warpedV * 3,
              2
            );
            blendFactor = MathUtils.clamp(
              0.6 + forestElevation * 0.4,
              0.2,
              0.9
            );
            break;

          case 'desert':
            // Sharp transition with some rocky outcrops in desert
            const desertRocks = RockFormationNoise.generate(
              warpedU,
              warpedV,
              4
            );
            blendFactor = MathUtils.clamp(desertRocks + 0.3, 0.3, 0.8);
            break;

          case 'tundra':
            // Similar to grassland but with altitude effects
            const tundraTransition = Math.min(u, 1 - u);
            blendFactor = MathUtils.smoothstep(0, 0.5, tundraTransition) * 0.7;
            break;

          case 'river':
            // Rivers carve through mountains creating valleys
            const riverValley = ErosionSimulation.generateErosionChannels(
              warpedU,
              warpedV,
              2
            );
            blendFactor = MathUtils.clamp(1 - riverValley * 0.8, 0.2, 1);
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
 * Generate and export complete mountain texture set
 */
export const generateMountainTextureSet = async (
  variation: string = 'rocky_peaks',
  resolution: number = 1024,
  environmentalFactors?: {
    elevation: number;
    temperature: number;
    precipitation: number;
    age: number;
  }
): Promise<{
  albedo: Blob;
  normal: Blob;
  roughness: Blob;
  metallic: Blob;
  height: Blob;
}> => {
  const generator = new AdvancedMountainGenerator(
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
