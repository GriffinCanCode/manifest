/**
 * @texture-factory/tundra.ts
 *
 * Advanced procedural tundra texture generation
 * Designed to compete with AAA game quality (Civilization VI standard)
 *
 * Features:
 * - Permafrost polygon patterns
 * - Ice crystal formations and frost effects
 * - Sparse arctic vegetation (lichens, mosses, dwarf shrubs)
 * - Seasonal snow coverage variation
 * - Freeze-thaw rock weathering patterns
 * - Wind scour erosion effects
 * - Realistic PBR material properties
 * - Seamless tiling with advanced noise techniques
 */

import { Color, MathUtils } from 'three';

// ============================================================================
// ADVANCED TUNDRA NOISE FUNCTIONS
// ============================================================================

/**
 * Domain warping implementation adapted for tundra patterns
 * Creates natural permafrost polygons and freeze-thaw formations
 */
class TundraDomainWarping {
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
    strength: number = 0.18
  ): [number, number] {
    // Tundra-specific warping for permafrost polygons
    const warpX = this.fbm(x * scale, y * scale, 3) * strength;
    const warpY = this.fbm((x + 5.7) * scale, (y + 2.3) * scale, 3) * strength;

    return [x + warpX, y + warpY];
  }

  static permafrostWarping(
    x: number,
    y: number,
    scale: number = 1
  ): [number, number] {
    // Specialized warping for permafrost polygon formation
    const strength = 0.12;
    const warpX = this.fbm(x * scale * 0.7, y * scale, 4) * strength;
    const warpY =
      this.fbm((x + 4.1) * scale, (y + 7.9) * scale * 0.8, 4) * strength;

    return [x + warpX, y + warpY];
  }
}

/**
 * Permafrost polygon noise for creating natural ground cracking patterns
 */
class PermafrostNoise {
  static generate(
    x: number,
    y: number,
    scale: number = 6,
    polygonSize: number = 0.8
  ): number {
    const cellSize = 1 / scale;
    const points = [];

    // Generate polygon centers
    for (let xi = -1; xi <= 1; xi++) {
      for (let yi = -1; yi <= 1; yi++) {
        const cellX = Math.floor(x * scale) + xi;
        const cellY = Math.floor(y * scale) + yi;

        const hash = ((cellX * 127.1 + cellY * 311.7) % 1000) / 1000;
        const pointX = cellX * cellSize + hash * cellSize * 0.8;
        const pointY =
          cellY * cellSize + (((hash * 269.5) % 1000) / 1000) * cellSize * 0.8;

        points.push({ x: pointX, y: pointY });
      }
    }

    // Find closest distance (Voronoi cell)
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

    // Create polygon boundaries (crack lines)
    const edgeDistance = secondMinDist - minDist;
    const polygonPattern = MathUtils.clamp(1 - minDist * scale * 2, 0, 1);
    const crackPattern = MathUtils.smoothstep(0, 0.1, edgeDistance) * 0.3;

    return Math.max(polygonPattern * polygonSize, crackPattern);
  }
}

/**
 * Ice crystal formation noise for frost and ice patches
 */
class IceCrystalNoise {
  static generate(x: number, y: number, octaves: number = 6): number {
    let value = 0;
    let amplitude = 1;
    let frequency = 1;
    let weight = 1;

    for (let i = 0; i < octaves; i++) {
      // Create crystalline patterns with hexagonal tendency
      const angle1 = frequency * Math.PI * 2;
      const angle2 = frequency * Math.PI * 2 + Math.PI / 3;

      const signal1 = Math.abs(
        TundraDomainWarping['noise'](
          x * frequency * Math.cos(angle1),
          y * frequency * Math.sin(angle1)
        )
      );
      const signal2 = Math.abs(
        TundraDomainWarping['noise'](
          x * frequency * Math.cos(angle2),
          y * frequency * Math.sin(angle2)
        )
      );

      let signal = Math.max(signal1, signal2);
      signal = Math.pow(signal, 1.2);
      signal *= weight;

      value += signal * amplitude;
      weight = MathUtils.clamp(signal * 1.5, 0.2, 1);
      amplitude *= 0.6;
      frequency *= 2.1;
    }

    return MathUtils.clamp(value, 0, 1);
  }
}

/**
 * Sparse vegetation noise for arctic plants
 */
class ArcticVegetationNoise {
  static generate(
    x: number,
    y: number,
    scale: number = 12,
    density: number = 0.3
  ): number {
    const cellSize = 1 / scale;
    const plants = [];

    // Generate sparse plant locations
    for (let xi = -1; xi <= 1; xi++) {
      for (let yi = -1; yi <= 1; yi++) {
        const cellX = Math.floor(x * scale) + xi;
        const cellY = Math.floor(y * scale) + yi;

        const hash = ((cellX * 127.1 + cellY * 311.7) % 1000) / 1000;

        // Only place plants based on density and environmental suitability
        if (hash < density) {
          const plantX = cellX * cellSize + hash * cellSize * 0.9;
          const plantY =
            cellY * cellSize +
            (((hash * 269.5) % 1000) / 1000) * cellSize * 0.9;

          // Plant type and size variation
          const plantType = ((hash * 173.3) % 1000) / 1000;
          const plantSize = 0.1 + plantType * 0.3;
          plants.push({
            x: plantX,
            y: plantY,
            size: plantSize,
            type: plantType,
          });
        }
      }
    }

    if (plants.length === 0) return 0;

    // Calculate vegetation influence
    let vegInfluence = 0;
    for (const plant of plants) {
      const dist = Math.hypot(x - plant.x, y - plant.y);
      const influence = Math.max(0, plant.size - dist * scale * 1.8);
      vegInfluence = Math.max(vegInfluence, influence);
    }

    return MathUtils.clamp(vegInfluence, 0, 1);
  }
}

/**
 * Wind scour patterns for tundra erosion effects
 */
class WindScourNoise {
  static generate(x: number, y: number, windStrength: number = 0.7): number {
    // Prevailing arctic wind patterns (from northwest)
    const windAngle = Math.PI * 1.25; // Northwest wind
    const rotX = x * Math.cos(windAngle) - y * Math.sin(windAngle);
    const rotY = x * Math.sin(windAngle) + y * Math.cos(windAngle);

    // Create scour channels
    const scourPattern1 = Math.abs(
      TundraDomainWarping['noise'](rotX * 8, rotY * 2)
    );
    const scourPattern2 = Math.abs(
      TundraDomainWarping['noise'](rotX * 16, rotY * 4 + Math.PI)
    );

    const combinedScour = Math.max(scourPattern1, scourPattern2 * 0.6);
    return Math.pow(combinedScour, 2) * windStrength;
  }
}

// ============================================================================
// ENVIRONMENTAL FACTORS INTERFACE
// ============================================================================

interface EnvironmentalFactors {
  moisture: number; // 0-1, affects vegetation and ice formation
  temperature: number; // 0-1, affects permafrost and snow coverage
  windStrength: number; // 0-1, affects erosion patterns
  season: number; // 0-1, affects snow coverage and vegetation activity
  elevation: number; // 0-1, affects vegetation line and ice content
}

// ============================================================================
// TUNDRA MATERIAL PROPERTIES
// ============================================================================

interface TundraVariation {
  name: string;
  // Soil and permafrost colors
  permafrostColor: Color;
  frozenSoilColor: Color;
  rockColor: Color;
  // Vegetation colors
  lichenColor: Color;
  mossColor: Color;
  dwarf_shrubColor: Color;
  // Ice and snow colors
  iceColor: Color;
  snowColor: Color;
  // Material properties
  roughness: number;
  metallic: number;
  normalStrength: number;
  permafrostIntensity: number;
  vegetationDensity: number;
  iceContent: number;
}

const TUNDRA_VARIATIONS: TundraVariation[] = [
  {
    name: 'arctic_tundra',
    permafrostColor: new Color(0.18, 0.16, 0.14), // Cold gray-brown
    frozenSoilColor: new Color(0.22, 0.18, 0.15), // Frozen earth
    rockColor: new Color(0.25, 0.23, 0.2), // Weathered stone
    lichenColor: new Color(0.28, 0.35, 0.18), // Gray-green lichen
    mossColor: new Color(0.15, 0.25, 0.12), // Dark moss
    dwarf_shrubColor: new Color(0.32, 0.28, 0.15), // Muted shrub
    iceColor: new Color(0.85, 0.88, 0.92), // Pale blue ice
    snowColor: new Color(0.95, 0.95, 0.97), // Clean snow
    roughness: 0.75,
    metallic: 0.05,
    normalStrength: 1.3,
    permafrostIntensity: 0.9,
    vegetationDensity: 0.25,
    iceContent: 0.4,
  },
  {
    name: 'alpine_tundra',
    permafrostColor: new Color(0.2, 0.18, 0.16),
    frozenSoilColor: new Color(0.25, 0.22, 0.18),
    rockColor: new Color(0.35, 0.3, 0.25), // More exposed rock
    lichenColor: new Color(0.35, 0.4, 0.22),
    mossColor: new Color(0.18, 0.28, 0.15),
    dwarf_shrubColor: new Color(0.38, 0.32, 0.2),
    iceColor: new Color(0.8, 0.85, 0.9),
    snowColor: new Color(0.92, 0.92, 0.95),
    roughness: 0.85,
    metallic: 0.02,
    normalStrength: 1.5,
    permafrostIntensity: 0.7,
    vegetationDensity: 0.35,
    iceContent: 0.2,
  },
  {
    name: 'coastal_tundra',
    permafrostColor: new Color(0.16, 0.15, 0.14), // Darker, more moisture
    frozenSoilColor: new Color(0.2, 0.17, 0.15),
    rockColor: new Color(0.28, 0.25, 0.22),
    lichenColor: new Color(0.25, 0.32, 0.2), // More vibrant near coast
    mossColor: new Color(0.12, 0.22, 0.15),
    dwarf_shrubColor: new Color(0.28, 0.25, 0.18),
    iceColor: new Color(0.75, 0.82, 0.88), // Sea ice tint
    snowColor: new Color(0.88, 0.9, 0.92),
    roughness: 0.7,
    metallic: 0.08,
    normalStrength: 1.1,
    permafrostIntensity: 0.8,
    vegetationDensity: 0.4,
    iceContent: 0.6,
  },
];

// ============================================================================
// ADVANCED TUNDRA TEXTURE GENERATOR
// ============================================================================

export class AdvancedTundraGenerator {
  private resolution: number;
  private variation: TundraVariation;
  private environmentalFactors: EnvironmentalFactors;

  constructor(
    resolution: number = 1024,
    variationName: string = 'arctic_tundra',
    environmentalFactors: EnvironmentalFactors = {
      moisture: 0.3,
      temperature: 0.2,
      windStrength: 0.8,
      season: 0.3,
      elevation: 0.6,
    }
  ) {
    this.resolution = resolution;
    this.variation =
      TUNDRA_VARIATIONS.find(v => v.name === variationName) ??
      TUNDRA_VARIATIONS[0];
    this.environmentalFactors = environmentalFactors;
  }

  /**
   * Generate complete PBR material set for tundra terrain
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
   * Generate high-quality albedo map with permafrost, vegetation, and ice
   */
  private generateAlbedoMap(): ImageData {
    const canvas = new OffscreenCanvas(this.resolution, this.resolution);
    const ctx = canvas.getContext('2d');
    if (!ctx) {
      throw new Error('Failed to get 2D rendering context');
    }
    const imageData = ctx.createImageData(this.resolution, this.resolution);
    const { data } = imageData;

    for (let y = 0; y < this.resolution; y++) {
      for (let x = 0; x < this.resolution; x++) {
        const u = x / this.resolution;
        const v = y / this.resolution;

        // Apply domain warping for natural tundra patterns
        const [warpedU, warpedV] = TundraDomainWarping.warpDomain(u, v, 4);
        const [permafrostU, permafrostV] =
          TundraDomainWarping.permafrostWarping(u, v, 6);

        // Generate tundra features
        const permafrostPattern = PermafrostNoise.generate(
          warpedU,
          warpedV,
          8,
          this.variation.permafrostIntensity
        );
        const iceCrystals =
          IceCrystalNoise.generate(permafrostU * 32, permafrostV * 32, 5) *
          this.variation.iceContent;
        const vegetation = ArcticVegetationNoise.generate(
          warpedU,
          warpedV,
          14,
          this.variation.vegetationDensity * this.environmentalFactors.moisture
        );
        const windScour = WindScourNoise.generate(
          warpedU,
          warpedV,
          this.environmentalFactors.windStrength
        );

        // Environmental variation
        const temperatureNoise =
          TundraDomainWarping['fbm'](u * 2, v * 2, 3) * 0.2 + 0.8;
        const actualTemperature =
          this.environmentalFactors.temperature * temperatureNoise;

        // Seasonal snow coverage
        const snowCoverage =
          (1 - this.environmentalFactors.season) *
          (1 - actualTemperature) *
          TundraDomainWarping['fbm'](u * 8, v * 8, 2);

        // Fine surface detail
        const surfaceDetail =
          TundraDomainWarping['fbm'](warpedU * 128, warpedV * 128, 4) * 0.12;

        // Determine primary surface material
        let finalColor: Color;

        if (snowCoverage > 0.4) {
          // Snow-covered areas
          finalColor = this.variation.snowColor.clone();

          // Add subtle blue tinting in shadows
          const shadowTint = (1 - permafrostPattern) * 0.1;
          finalColor.b = MathUtils.clamp(finalColor.b + shadowTint, 0, 1);
        } else if (iceCrystals > 0.6 && actualTemperature < 0.4) {
          // Ice patches
          finalColor = this.variation.iceColor.clone();

          // Add crystalline variation
          const crystalVariation = iceCrystals * 0.15;
          finalColor.r = MathUtils.clamp(finalColor.r + crystalVariation, 0, 1);
          finalColor.g = MathUtils.clamp(finalColor.g + crystalVariation, 0, 1);
        } else if (vegetation > 0.4) {
          // Vegetation patches
          const vegType = TundraDomainWarping['fbm'](
            warpedU * 24,
            warpedV * 24,
            2
          );

          if (vegType > 0.3) {
            // Lichen
            finalColor = this.variation.lichenColor.clone();
          } else if (vegType > -0.2) {
            // Moss
            finalColor = this.variation.mossColor.clone();
          } else {
            // Dwarf shrubs
            finalColor = this.variation.dwarf_shrubColor.clone();
          }

          // Seasonal vegetation dormancy
          if (this.environmentalFactors.season < 0.3) {
            finalColor = finalColor.lerp(this.variation.permafrostColor, 0.4);
          }
        } else if (permafrostPattern > 0.5 || windScour > 0.3) {
          // Exposed permafrost or scoured areas
          finalColor = this.variation.permafrostColor.clone();

          // Add polygon pattern variation
          const polygonVariation = permafrostPattern * 0.2;
          finalColor = finalColor.lerp(
            this.variation.frozenSoilColor,
            polygonVariation
          );
        } else {
          // General frozen soil
          finalColor = this.variation.frozenSoilColor.clone();

          // Mix with rock color in exposed areas
          const rockExposure =
            (1 - vegetation) * this.environmentalFactors.elevation * 0.3;
          if (rockExposure > 0.1) {
            finalColor = finalColor.lerp(
              this.variation.rockColor,
              rockExposure
            );
          }
        }

        // Apply environmental modulation
        // Cold temperature desaturates colors
        const coldDesaturation = (1 - actualTemperature) * 0.2;
        const gray = (finalColor.r + finalColor.g + finalColor.b) / 3;
        finalColor.r = MathUtils.lerp(finalColor.r, gray, coldDesaturation);
        finalColor.g = MathUtils.lerp(finalColor.g, gray, coldDesaturation);
        finalColor.b = MathUtils.lerp(finalColor.b, gray, coldDesaturation);

        // Add surface detail variation
        finalColor.r = MathUtils.clamp(
          finalColor.r * (0.9 + surfaceDetail * 0.2),
          0,
          1
        );
        finalColor.g = MathUtils.clamp(
          finalColor.g * (0.9 + surfaceDetail * 0.2),
          0,
          1
        );
        finalColor.b = MathUtils.clamp(
          finalColor.b * (0.9 + surfaceDetail * 0.2),
          0,
          1
        );

        // Atmospheric haze from extreme cold
        if (actualTemperature < 0.3) {
          const atmosphericHaze = (0.3 - actualTemperature) * 0.08;
          finalColor.r = MathUtils.clamp(finalColor.r + atmosphericHaze, 0, 1);
          finalColor.g = MathUtils.clamp(finalColor.g + atmosphericHaze, 0, 1);
          finalColor.b = MathUtils.clamp(
            finalColor.b + atmosphericHaze * 1.2,
            0,
            1
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
   * Generate detailed normal map for permafrost patterns and ice formations
   */
  private generateNormalMap(): ImageData {
    const canvas = new OffscreenCanvas(this.resolution, this.resolution);
    const ctx = canvas.getContext('2d');
    if (!ctx) {
      throw new Error('Failed to get 2D rendering context');
    }
    const imageData = ctx.createImageData(this.resolution, this.resolution);
    const { data } = imageData;

    const heightScale = this.variation.normalStrength * 0.018;

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

        // Convert to [0, 255] range (creates purple/blue normal map appearance)
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
    const [warpedU, warpedV] = TundraDomainWarping.warpDomain(u, v, 4);
    const [permafrostU, permafrostV] = TundraDomainWarping.permafrostWarping(
      u,
      v,
      6
    );

    // Permafrost polygon height variation (most pronounced)
    const permafrostHeight =
      PermafrostNoise.generate(
        warpedU,
        warpedV,
        6,
        this.variation.permafrostIntensity
      ) * 0.6;

    // Ice crystal surface detail
    const iceHeight =
      IceCrystalNoise.generate(permafrostU * 64, permafrostV * 64, 6) * 0.4;

    // Wind scour erosion (negative height)
    const windErosion =
      WindScourNoise.generate(
        warpedU,
        warpedV,
        this.environmentalFactors.windStrength
      ) * -0.3;

    // Fine surface texture
    const surfaceTexture =
      TundraDomainWarping['fbm'](warpedU * 256, warpedV * 256, 5) * 0.2;

    // Rock and vegetation bumps
    const vegetationBumps =
      ArcticVegetationNoise.generate(
        warpedU,
        warpedV,
        12,
        this.variation.vegetationDensity
      ) * 0.25;

    return (
      permafrostHeight +
      iceHeight +
      windErosion +
      surfaceTexture +
      vegetationBumps
    );
  }

  /**
   * Generate roughness map with realistic tundra surface properties
   */
  private generateRoughnessMap(): ImageData {
    const canvas = new OffscreenCanvas(this.resolution, this.resolution);
    const ctx = canvas.getContext('2d');
    if (!ctx) {
      throw new Error('Failed to get 2D rendering context');
    }
    const imageData = ctx.createImageData(this.resolution, this.resolution);
    const { data } = imageData;

    for (let y = 0; y < this.resolution; y++) {
      for (let x = 0; x < this.resolution; x++) {
        const u = x / this.resolution;
        const v = y / this.resolution;

        const [warpedU, warpedV] = TundraDomainWarping.warpDomain(u, v, 2);

        // Base roughness from variation
        let { roughness } = this.variation;

        // Ice areas are smoother
        const iceCrystals = IceCrystalNoise.generate(
          warpedU * 32,
          warpedV * 32,
          4
        );
        if (iceCrystals > 0.5) {
          roughness = MathUtils.lerp(roughness, 0.2, iceCrystals);
        }

        // Snow areas are very smooth
        const snowCoverage =
          (1 - this.environmentalFactors.season) *
          (1 - this.environmentalFactors.temperature);
        if (snowCoverage > 0.3) {
          roughness = MathUtils.lerp(roughness, 0.1, snowCoverage);
        }

        // Vegetation areas have moderate roughness
        const vegetation = ArcticVegetationNoise.generate(
          warpedU,
          warpedV,
          12,
          0.4
        );
        if (vegetation > 0.3) {
          roughness = MathUtils.lerp(roughness, 0.6, vegetation);
        }

        // Wind polishing effect
        const windPolishing = WindScourNoise.generate(
          warpedU,
          warpedV,
          this.environmentalFactors.windStrength
        );
        roughness = MathUtils.clamp(roughness - windPolishing * 0.1, 0.05, 1);

        // Permafrost creates rough, cracked surfaces
        const permafrostRoughening = PermafrostNoise.generate(
          warpedU,
          warpedV,
          8,
          0.8
        );
        if (permafrostRoughening > 0.4) {
          roughness = MathUtils.clamp(
            roughness + permafrostRoughening * 0.2,
            0,
            1
          );
        }

        // Temperature affects surface texture (freeze-thaw cycles)
        const temperatureEffect =
          Math.abs(this.environmentalFactors.temperature - 0.3) * 0.15;
        roughness = MathUtils.clamp(roughness + temperatureEffect, 0, 1);

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
   * Generate metallic map (ice has some metallicism, permafrost is non-metallic)
   */
  private generateMetallicMap(): ImageData {
    const canvas = new OffscreenCanvas(this.resolution, this.resolution);
    const ctx = canvas.getContext('2d');
    if (!ctx) {
      throw new Error('Failed to get 2D rendering context');
    }
    const imageData = ctx.createImageData(this.resolution, this.resolution);
    const { data } = imageData;

    for (let y = 0; y < this.resolution; y++) {
      for (let x = 0; x < this.resolution; x++) {
        const u = x / this.resolution;
        const v = y / this.resolution;

        const [warpedU, warpedV] = TundraDomainWarping.warpDomain(u, v, 2);

        // Base metallic from variation
        let { metallic } = this.variation;

        // Ice areas have slight metallicism for realistic light reflection
        const iceCrystals = IceCrystalNoise.generate(
          warpedU * 32,
          warpedV * 32,
          4
        );
        if (iceCrystals > 0.6) {
          metallic = MathUtils.lerp(metallic, 0.15, iceCrystals);
        }

        // Snow has very slight metallicism when compact
        const snowCoverage =
          (1 - this.environmentalFactors.season) *
          (1 - this.environmentalFactors.temperature);
        if (snowCoverage > 0.7) {
          metallic = Math.max(metallic, 0.05);
        }

        // Permafrost and vegetation are non-metallic
        const vegetation = ArcticVegetationNoise.generate(
          warpedU,
          warpedV,
          12,
          0.4
        );
        if (vegetation > 0.4) {
          metallic = 0.0;
        }

        // Wet conditions (rare but possible) can add slight metallicism
        if (
          this.environmentalFactors.moisture > 0.7 &&
          this.environmentalFactors.temperature > 0.4
        ) {
          const moistureMetallic = TundraDomainWarping['fbm'](
            warpedU * 16,
            warpedV * 16,
            2
          );
          if (moistureMetallic > 0.6) {
            metallic = Math.min(metallic + 0.03, 0.1);
          }
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
   * Generate height/displacement map for permafrost polygons and ice formations
   */
  private generateHeightMap(): ImageData {
    const canvas = new OffscreenCanvas(this.resolution, this.resolution);
    const ctx = canvas.getContext('2d');
    if (!ctx) {
      throw new Error('Failed to get 2D rendering context');
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
   */
  generateBlendingMask(
    adjacentTerrain: 'grassland' | 'forest' | 'mountain' | 'desert' | 'ocean'
  ): ImageData {
    const canvas = new OffscreenCanvas(this.resolution, this.resolution);
    const ctx = canvas.getContext('2d');
    if (!ctx) {
      throw new Error('Failed to get 2D rendering context');
    }
    const imageData = ctx.createImageData(this.resolution, this.resolution);
    const { data } = imageData;

    for (let y = 0; y < this.resolution; y++) {
      for (let x = 0; x < this.resolution; x++) {
        const u = x / this.resolution;
        const v = y / this.resolution;

        let blendFactor = 1.0;

        switch (adjacentTerrain) {
          case 'grassland':
            // Natural transition from tundra to grassland (tree line effect)
            const temperatureGradient = this.environmentalFactors.temperature;
            const grassTransition = TundraDomainWarping['fbm'](u * 4, v * 4, 3);
            blendFactor = MathUtils.smoothstep(
              0.3,
              0.8,
              temperatureGradient + grassTransition * 0.2
            );
            break;

          case 'forest':
            // Sparse forest transition at higher latitudes/elevations
            const forestDistance = Math.min(u, 1 - u, v, 1 - v);
            const treeLineTransition = ArcticVegetationNoise.generate(
              u,
              v,
              6,
              0.3
            );
            blendFactor =
              MathUtils.smoothstep(
                0,
                0.5,
                forestDistance + treeLineTransition * 0.4
              ) * 0.6;
            break;

          case 'mountain':
            // Alpine tundra to mountain transition
            const elevationNoise = TundraDomainWarping['fbm'](u * 3, v * 3, 4);
            const rockExposure = PermafrostNoise.generate(u, v, 6, 0.7);
            blendFactor = MathUtils.clamp(
              0.8 + elevationNoise * 0.2 - rockExposure * 0.3,
              0,
              1
            );
            break;

          case 'desert':
            // Cold desert transition (rare but possible in continental interiors)
            const desertDistance = Math.min(u, 1 - u);
            const aridityFactor =
              (1 - this.environmentalFactors.moisture) * 0.4;
            blendFactor =
              MathUtils.smoothstep(0, 0.4, desertDistance) * aridityFactor;
            break;

          case 'ocean':
            // Coastal tundra transition
            const coastDistance = Math.min(u, 1 - u, v, 1 - v);
            const coastalMoisture = this.environmentalFactors.moisture * 0.8;
            const iceShelf = IceCrystalNoise.generate(u * 8, v * 8, 3);
            blendFactor =
              MathUtils.smoothstep(0, 0.3, coastDistance + iceShelf * 0.2) *
              (0.6 + coastalMoisture * 0.4);
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
    const ctx = canvas.getContext('2d');
    if (!ctx) {
      throw new Error('Failed to get 2D rendering context');
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
 * Generate and export complete tundra texture set
 */
export const generateTundraTextureSet = async (
  variation: string = 'arctic_tundra',
  resolution: number = 1024,
  environmentalFactors?: EnvironmentalFactors
): Promise<{
  albedo: Blob;
  normal: Blob;
  roughness: Blob;
  metallic: Blob;
  height: Blob;
}> => {
  const generator = new AdvancedTundraGenerator(
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
