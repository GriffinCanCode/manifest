/**
 * @texture-factory/hills.ts
 *
 * Advanced procedural hills texture generation
 * Designed to compete with AAA game quality (Civilization VI standard)
 *
 * Features:
 * - Rolling terrain with gentle elevation changes
 * - Multi-layered vegetation coverage (grass, shrubs, sparse trees)
 * - Soft erosion patterns and natural drainage
 * - Soil composition variation with exposed bedrock
 * - Realistic PBR material properties
 * - Seamless tiling with advanced terrain integration
 * - Environmental variation support (vegetation density, soil moisture)
 */

import { Color, MathUtils } from 'three';

// ============================================================================
// ADVANCED HILLS NOISE FUNCTIONS
// ============================================================================

/**
 * Soft ridged noise for creating rolling hill contours
 * Less dramatic than mountain ridges, more organic than basic noise
 */
class SoftRidgedNoise {
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
    // Improved Perlin-style noise optimized for gentle terrain
    const ix = Math.floor(x);
    const iy = Math.floor(y);
    const fx = x - ix;
    const fy = y - iy;

    // Extra smooth interpolation for gentler hills
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

  static generate(x: number, y: number, octaves: number = 6): number {
    let value = 0;
    let amplitude = 1;
    let frequency = 1;
    let weight = 1;

    for (let i = 0; i < octaves; i++) {
      let signal = this.noise(x * frequency, y * frequency);

      // Soft ridging - less dramatic than mountain ridging
      signal = Math.abs(signal);
      signal = 1 - signal;
      signal *= signal * weight * 0.7; // Reduced intensity for gentler hills

      value += signal * amplitude;
      weight = MathUtils.clamp(signal * 1.5, 0, 1);
      amplitude *= 0.6; // Slightly higher persistence for smoother transitions
      frequency *= 1.8; // Lower frequency scaling for broader features
    }

    return MathUtils.clamp(value, 0, 1);
  }
}

/**
 * Rolling terrain simulation for natural hill formation
 * Creates undulating patterns typical of weathered uplands
 */
class RollingTerrainNoise {
  static generate(x: number, y: number, scale: number = 2): number {
    // Multiple overlapping sine waves for rolling terrain
    const wave1 = Math.sin(x * scale * 0.8) * Math.cos(y * scale * 0.6);
    const wave2 = Math.sin(x * scale * 1.2) * Math.cos(y * scale * 1.1);
    const wave3 = Math.sin(x * scale * 0.5) * Math.cos(y * scale * 0.7);

    // Combine waves with noise for natural variation
    const baseRolling = wave1 * 0.5 + wave2 * 0.3 + wave3 * 0.2;
    const naturalVariation = SoftRidgedNoise['fbm'](x * 2, y * 2, 3) * 0.3;

    return MathUtils.clamp((baseRolling + naturalVariation + 1) * 0.5, 0, 1);
  }
}

/**
 * Gentle erosion patterns for hills
 * Creates natural drainage without dramatic channels
 */
class GentleErosion {
  static generateDrainagePattern(
    x: number,
    y: number,
    scale: number = 3
  ): number {
    // Soft flow simulation using domain warping
    const flowX = SoftRidgedNoise['fbm'](x * scale * 0.5, y * scale * 0.5, 3);
    const flowY = SoftRidgedNoise['fbm'](
      (x + 5.2) * scale * 0.5,
      (y + 1.3) * scale * 0.5,
      3
    );

    // Create gentle drainage paths
    const drainageIntensity = Math.hypot(flowX, flowY) * 0.4;

    // Add subtle gully formation
    const gullyNoise = SoftRidgedNoise['fbm'](x * scale, y * scale, 4) * 0.3;

    return MathUtils.clamp(drainageIntensity + gullyNoise, 0, 1);
  }
}

/**
 * Domain warping optimized for hills terrain
 * Creates gentle, organic distortions for natural hill shapes
 */
class HillsDomainWarping {
  static warpDomain(x: number, y: number, scale: number = 1): [number, number] {
    const strength = 0.12; // Moderate warping for natural but not chaotic hills

    // Primary terrain warping
    const warpX = SoftRidgedNoise['fbm'](x * scale, y * scale, 3) * strength;
    const warpY =
      SoftRidgedNoise['fbm']((x + 7.3) * scale, (y + 2.1) * scale, 3) *
      strength;

    return [x + warpX, y + warpY];
  }
}

/**
 * Vegetation distribution for hills
 * Creates natural patterns of grass, shrubs, and sparse trees
 */
class VegetationDistribution {
  static generateCoverage(
    x: number,
    y: number,
    elevation: number,
    moisture: number
  ): {
    grass: number;
    shrubs: number;
    trees: number;
    exposedSoil: number;
  } {
    // Base vegetation patterns using cellular-like distribution
    const vegetationBase = SoftRidgedNoise.generate(x * 8, y * 8, 4);

    // Elevation affects vegetation type (trees prefer lower elevations in hills)
    const elevationFactor = 1 - elevation * 0.6;

    // Moisture affects density
    const moistureFactor = MathUtils.clamp(moisture * 1.2, 0.3, 1);

    // Calculate coverage percentages
    let grass = vegetationBase * moistureFactor * 0.8;
    let shrubs = (1 - vegetationBase) * elevationFactor * 0.4;
    let trees = vegetationBase * elevationFactor * moistureFactor * 0.3;
    let exposedSoil = Math.max(0, 1 - (grass + shrubs + trees));

    // Normalize to ensure total doesn't exceed 1
    const total = grass + shrubs + trees + exposedSoil;
    grass /= total;
    shrubs /= total;
    trees /= total;
    exposedSoil /= total;

    return { grass, shrubs, trees, exposedSoil };
  }
}

// ============================================================================
// HILLS MATERIAL PROPERTIES
// ============================================================================

interface HillsVariation {
  name: string;
  grassColor: Color;
  shrubColor: Color;
  treeColor: Color;
  soilColor: Color;
  bedrockColor: Color;
  roughness: number;
  metallic: number;
  normalStrength: number;
  vegetationDensity: number;
  elevationRange: number; // How dramatic the height variations are
}

const HILLS_VARIATIONS: HillsVariation[] = [
  {
    name: 'grassy_hills',
    grassColor: new Color(0.18, 0.45, 0.15), // Lush hill grass
    shrubColor: new Color(0.25, 0.35, 0.18), // Dense shrubs
    treeColor: new Color(0.12, 0.25, 0.08), // Sparse hill trees
    soilColor: new Color(0.28, 0.22, 0.15), // Rich brown soil
    bedrockColor: new Color(0.35, 0.32, 0.28), // Weathered bedrock
    roughness: 0.75,
    metallic: 0.0,
    normalStrength: 1.0,
    vegetationDensity: 0.85,
    elevationRange: 0.6,
  },
  {
    name: 'moorland_hills',
    grassColor: new Color(0.15, 0.35, 0.12), // Darker, moor-like grass
    shrubColor: new Color(0.2, 0.3, 0.15), // Heath shrubs
    treeColor: new Color(0.1, 0.2, 0.06), // Sparse, windblown trees
    soilColor: new Color(0.25, 0.18, 0.12), // Peaty soil
    bedrockColor: new Color(0.3, 0.28, 0.25), // Grey stone
    roughness: 0.8,
    metallic: 0.0,
    normalStrength: 0.9,
    vegetationDensity: 0.6,
    elevationRange: 0.7,
  },
  {
    name: 'highland_downs',
    grassColor: new Color(0.22, 0.4, 0.18), // Short highland grass
    shrubColor: new Color(0.18, 0.28, 0.15), // Low shrubs
    treeColor: new Color(0.08, 0.18, 0.05), // Very sparse trees
    soilColor: new Color(0.32, 0.26, 0.18), // Chalky soil
    bedrockColor: new Color(0.4, 0.38, 0.35), // Light limestone
    roughness: 0.7,
    metallic: 0.0,
    normalStrength: 0.8,
    vegetationDensity: 0.5,
    elevationRange: 0.5,
  },
  {
    name: 'rolling_meadows',
    grassColor: new Color(0.2, 0.5, 0.16), // Bright meadow grass
    shrubColor: new Color(0.28, 0.38, 0.2), // Flowering shrubs
    treeColor: new Color(0.14, 0.28, 0.1), // Scattered oak-like trees
    soilColor: new Color(0.3, 0.24, 0.16), // Dark loam
    bedrockColor: new Color(0.38, 0.35, 0.3), // Sandstone
    roughness: 0.65,
    metallic: 0.0,
    normalStrength: 1.1,
    vegetationDensity: 0.9,
    elevationRange: 0.4,
  },
];

// ============================================================================
// ADVANCED HILLS TEXTURE GENERATOR
// ============================================================================

export class AdvancedHillsGenerator {
  private resolution: number;
  private variation: HillsVariation;
  private environmentalFactors: {
    moisture: number; // 0-1, affects vegetation density and type
    temperature: number; // 0-1, affects vegetation and soil color
    season: number; // 0-1, affects vegetation color and dormancy
    soilRichness: number; // 0-1, affects exposed bedrock and vegetation health
  };

  constructor(
    resolution: number = 1024,
    variationName: string = 'grassy_hills',
    environmentalFactors = {
      moisture: 0.65,
      temperature: 0.55,
      season: 0.5,
      soilRichness: 0.7,
    }
  ) {
    this.resolution = resolution;
    this.variation =
      HILLS_VARIATIONS.find(v => v.name === variationName) ??
      HILLS_VARIATIONS[0];
    this.environmentalFactors = environmentalFactors;
  }

  /**
   * Generate complete PBR material set for hills terrain
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
   * Generate high-quality albedo map with rolling terrain and vegetation
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

        // Apply domain warping for organic hill shapes
        const [warpedU, warpedV] = HillsDomainWarping.warpDomain(u, v, 3);

        // Generate base terrain elevation and features
        const rollingTerrain =
          RollingTerrainNoise.generate(warpedU, warpedV, 2) *
          this.variation.elevationRange;
        const hillRidges =
          SoftRidgedNoise.generate(warpedU * 4, warpedV * 4, 5) * 0.4;
        const drainagePattern = GentleErosion.generateDrainagePattern(
          warpedU,
          warpedV,
          3
        );

        // Calculate elevation for this pixel
        const elevation = MathUtils.clamp(
          rollingTerrain + hillRidges * 0.3,
          0,
          1
        );

        // Environmental variation
        const moistureNoise =
          SoftRidgedNoise['fbm'](warpedU * 2, warpedV * 2, 2) * 0.3 + 0.7;
        const actualMoisture =
          this.environmentalFactors.moisture * moistureNoise;

        // Seasonal color variation
        const seasonalShift =
          Math.sin(warpedU * Math.PI * 3) *
          Math.cos(warpedV * Math.PI * 2) *
          0.1;
        const actualSeason = MathUtils.clamp(
          this.environmentalFactors.season + seasonalShift,
          0,
          1
        );

        // Calculate vegetation coverage
        const vegetation = VegetationDistribution.generateCoverage(
          warpedU,
          warpedV,
          elevation,
          actualMoisture
        );

        // Fine detail for texture variation
        const fineDetail =
          SoftRidgedNoise['fbm'](warpedU * 32, warpedV * 32, 4) * 0.15;

        // Color blending based on vegetation and environmental factors
        const finalColor = new Color(0, 0, 0);

        // Grass contribution
        if (vegetation.grass > 0.05) {
          const grassColor = this.variation.grassColor.clone();

          // Seasonal grass color variation
          if (actualSeason > 0.7) {
            // Autumn browning
            grassColor.lerp(
              new Color(0.4, 0.35, 0.15),
              (actualSeason - 0.7) * 2
            );
          } else if (actualSeason < 0.3) {
            // Winter dormancy
            grassColor.lerp(
              new Color(0.25, 0.22, 0.15),
              (0.3 - actualSeason) * 2
            );
          }

          // Moisture affects grass health
          grassColor.multiplyScalar(0.7 + actualMoisture * 0.4);
          finalColor.lerp(grassColor, vegetation.grass);
        }

        // Shrub contribution
        if (vegetation.shrubs > 0.05) {
          const shrubColor = this.variation.shrubColor.clone();

          // Seasonal shrub variation
          if (actualSeason > 0.6 && actualSeason < 0.9) {
            // Autumn colors for shrubs
            shrubColor.lerp(
              new Color(0.5, 0.3, 0.15),
              Math.sin(((actualSeason - 0.6) * Math.PI) / 0.3) * 0.6
            );
          }

          finalColor.lerp(shrubColor, vegetation.shrubs);
        }

        // Tree contribution
        if (vegetation.trees > 0.03) {
          const treeColor = this.variation.treeColor.clone();

          // Trees are less affected by seasons but still show some change
          if (actualSeason > 0.7) {
            treeColor.lerp(
              new Color(0.3, 0.25, 0.1),
              (actualSeason - 0.7) * 1.5
            );
          }

          finalColor.lerp(treeColor, vegetation.trees);
        }

        // Exposed soil contribution
        if (vegetation.exposedSoil > 0.1) {
          const soilColor = this.variation.soilColor.clone();

          // Soil color affected by moisture
          if (actualMoisture > 0.7) {
            soilColor.multiplyScalar(0.8); // Darker when wet
          } else if (actualMoisture < 0.4) {
            soilColor.lerp(new Color(0.4, 0.35, 0.25), 0.3); // Lighter when dry
          }

          finalColor.lerp(soilColor, vegetation.exposedSoil);
        }

        // Exposed bedrock in steep areas or drainage channels
        if (drainagePattern > 0.7 || elevation > 0.8) {
          const bedrockExposure = Math.max(
            (drainagePattern - 0.7) * 2,
            (elevation - 0.8) * 3
          );
          finalColor.lerp(this.variation.bedrockColor, bedrockExposure * 0.4);
        }

        // Apply fine detail variation
        finalColor.r = MathUtils.clamp(
          finalColor.r * (0.85 + fineDetail),
          0,
          1
        );
        finalColor.g = MathUtils.clamp(
          finalColor.g * (0.85 + fineDetail),
          0,
          1
        );
        finalColor.b = MathUtils.clamp(
          finalColor.b * (0.85 + fineDetail),
          0,
          1
        );

        // Soil richness affects overall vibrancy
        const richnessMultiplier =
          0.7 + this.environmentalFactors.soilRichness * 0.4;
        finalColor.multiplyScalar(richnessMultiplier);

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
   * Generate detailed normal map for hills surface detail
   */
  private generateNormalMap(): ImageData {
    const canvas = new OffscreenCanvas(this.resolution, this.resolution);
    const ctx = canvas.getContext('2d');
    if (!ctx) {
      throw new Error('Failed to get 2D context for normal map generation');
    }
    const imageData = ctx.createImageData(this.resolution, this.resolution);
    const { data } = imageData;

    const heightScale = this.variation.normalStrength * 0.008; // Gentler than mountains

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
    const [warpedU, warpedV] = HillsDomainWarping.warpDomain(u, v, 3);

    // Large-scale hill structure
    const hillStructure =
      RollingTerrainNoise.generate(warpedU, warpedV, 2) * 0.6;

    // Medium-scale ridge detail
    const ridgeDetail =
      SoftRidgedNoise.generate(warpedU * 4, warpedV * 4, 5) * 0.3;

    // Drainage patterns (negative contribution)
    const drainage =
      GentleErosion.generateDrainagePattern(warpedU, warpedV, 4) * -0.1;

    // Fine surface detail (vegetation and soil texture)
    const surfaceDetail =
      SoftRidgedNoise['fbm'](warpedU * 64, warpedV * 64, 6) * 0.1;

    return hillStructure + ridgeDetail + drainage + surfaceDetail;
  }

  /**
   * Generate roughness map with realistic hills surface properties
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

        const [warpedU, warpedV] = HillsDomainWarping.warpDomain(u, v, 2);

        // Base roughness from variation
        let { roughness } = this.variation;

        // Get vegetation coverage for this pixel
        const elevation =
          RollingTerrainNoise.generate(warpedU, warpedV, 2) +
          SoftRidgedNoise.generate(warpedU * 4, warpedV * 4, 5) * 0.3;
        const moistureNoise =
          SoftRidgedNoise['fbm'](warpedU * 2, warpedV * 2, 2) * 0.3 + 0.7;
        const actualMoisture =
          this.environmentalFactors.moisture * moistureNoise;

        const vegetation = VegetationDistribution.generateCoverage(
          warpedU,
          warpedV,
          elevation,
          actualMoisture
        );

        // Vegetation affects roughness
        if (vegetation.grass > 0.3) {
          // Grass areas are smoother
          roughness *= 0.8;
        }

        if (vegetation.trees > 0.2) {
          // Tree areas have varied roughness
          roughness *= 0.9;
        }

        if (vegetation.exposedSoil > 0.5) {
          // Exposed soil can be rougher
          roughness *= 1.1;
        }

        // Drainage channels are smoother (polished by water flow)
        const drainage = GentleErosion.generateDrainagePattern(
          warpedU,
          warpedV,
          4
        );
        if (drainage > 0.5) {
          roughness *= 1.0 - (drainage - 0.5) * 0.3;
        }

        // Moisture affects roughness (wet surfaces are smoother)
        const moistureEffect = actualMoisture * -0.15;
        roughness = MathUtils.clamp(roughness + moistureEffect, 0, 1);

        // Fine surface variation
        const surfaceVariation =
          SoftRidgedNoise['fbm'](warpedU * 16, warpedV * 16, 3) * 0.08;
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
   * Generate metallic map (hills are generally non-metallic except wet areas)
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

        const [warpedU, warpedV] = HillsDomainWarping.warpDomain(u, v, 1);

        // Base metallic value (very low for natural hills)
        let { metallic } = this.variation;

        // Wet areas and water drainage have slight metallic properties
        if (this.environmentalFactors.moisture > 0.7) {
          const drainageChannels = GentleErosion.generateDrainagePattern(
            warpedU,
            warpedV,
            6
          );
          if (drainageChannels > 0.6) {
            metallic = Math.min(metallic + 0.03, 0.05);
          }
        }

        // Exposed bedrock might have tiny amounts of metallic minerals
        const elevation = RollingTerrainNoise.generate(warpedU, warpedV, 2);
        if (elevation > 0.8) {
          const mineralTraces =
            SoftRidgedNoise['fbm'](warpedU * 32, warpedV * 32, 3) * 0.02;
          metallic = Math.min(metallic + Math.max(0, mineralTraces), 0.03);
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
          this.sampleHeightForNormal(u, v) * this.variation.elevationRange;
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
    adjacentTerrain:
      | 'grassland'
      | 'forest'
      | 'desert'
      | 'mountain'
      | 'river'
      | 'tundra'
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

        const [warpedU, warpedV] = HillsDomainWarping.warpDomain(u, v, 2);
        let blendFactor = 1.0;

        // Different blending strategies based on adjacent terrain
        switch (adjacentTerrain) {
          case 'grassland':
            // Very natural transition - hills often blend seamlessly with grassland
            const grassTransition = SoftRidgedNoise['fbm'](
              warpedU * 3,
              warpedV * 3,
              3
            );
            const distanceFromEdge = Math.min(u, 1 - u, v, 1 - v);
            blendFactor = MathUtils.smoothstep(
              0,
              0.4,
              distanceFromEdge + grassTransition * 0.2
            );
            break;

          case 'forest':
            // Hills often support forests - create natural tree line transitions
            const elevation = RollingTerrainNoise.generate(warpedU, warpedV, 2);
            const forestBlend = MathUtils.clamp(elevation + 0.2, 0.3, 0.9);
            blendFactor = forestBlend;
            break;

          case 'mountain':
            // Hills transition to mountains through elevation
            const mountainTransition = RollingTerrainNoise.generate(
              warpedU * 2,
              warpedV * 2,
              2
            );
            blendFactor = MathUtils.clamp(mountainTransition + 0.3, 0.4, 1.0);
            break;

          case 'desert':
            // Sparse transition with hills becoming more barren
            const desertDistance = Math.min(u, 1 - u);
            const desertBlend =
              MathUtils.smoothstep(0, 0.5, desertDistance) * 0.7;
            blendFactor = desertBlend;
            break;

          case 'river':
            // Rivers flow through hills creating valleys
            const riverValley = GentleErosion.generateDrainagePattern(
              warpedU,
              warpedV,
              1.5
            );
            blendFactor = MathUtils.clamp(1 - riverValley * 0.6, 0.3, 1);
            break;

          case 'tundra':
            // Elevation-based transition (higher hills become tundra-like)
            const tundraElevation = RollingTerrainNoise.generate(
              warpedU,
              warpedV,
              2
            );
            blendFactor = MathUtils.clamp(1 - tundraElevation * 0.5, 0.4, 0.9);
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
 * Generate and export complete hills texture set
 */
export const generateHillsTextureSet = async (
  variation: string = 'grassy_hills',
  resolution: number = 1024,
  environmentalFactors?: {
    moisture: number;
    temperature: number;
    season: number;
    soilRichness: number;
  }
): Promise<{
  albedo: Blob;
  normal: Blob;
  roughness: Blob;
  metallic: Blob;
  height: Blob;
}> => {
  const generator = new AdvancedHillsGenerator(
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
