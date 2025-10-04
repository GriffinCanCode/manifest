/**
 * @texture-factory/jungle.ts
 *
 * Advanced procedural jungle texture generation
 * Designed to compete with AAA game quality (Civilization VI standard)
 *
 * Features:
 * - Dense tropical vegetation with multi-layer canopy
 * - Complex vine and undergrowth systems
 * - Vibrant tropical colors with humidity effects
 * - Advanced cellular automata for plant placement
 * - Realistic PBR material properties for tropical environment
 * - Seamless tiling with advanced noise techniques
 * - Environmental variation support for tropical climate
 */

import { Color, MathUtils } from 'three';

// ============================================================================
// ADVANCED JUNGLE NOISE FUNCTIONS
// ============================================================================

/**
 * Domain warping implementation optimized for dense tropical vegetation
 * Creates organic jungle growth patterns and vine flow
 */
class JungleDomainWarping {
  static fbm(x: number, y: number, octaves: number = 4): number {
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

  static noise(x: number, y: number): number {
    const ix = Math.floor(x);
    const iy = Math.floor(y);
    const fx = x - ix;
    const fy = y - iy;

    // Smoother interpolation for organic jungle patterns
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

  static hash(x: number, y: number): number {
    let h = (x * 127.1 + y * 311.7) % 1000;
    h = ((h * 269.5) % 1000) / 1000;
    return h;
  }

  static warpDomain(x: number, y: number, scale: number = 1): [number, number] {
    const strength = 0.25; // Stronger warping for dense jungle growth
    const warpX = this.fbm(x * scale, y * scale, 5) * strength;
    const warpY = this.fbm((x + 9.7) * scale, (y + 3.1) * scale, 5) * strength;

    return [x + warpX, y + warpY];
  }

  static vineFlowWarping(
    x: number,
    y: number,
    scale: number = 1
  ): [number, number] {
    // Specialized warping for vine patterns and hanging vegetation
    const strength = 0.3;
    const warpX = this.fbm(x * scale * 0.3, y * scale, 4) * strength;
    const warpY =
      this.fbm((x + 5.3) * scale, (y + 12.7) * scale * 1.2, 4) * strength;

    return [x + warpX, y + warpY];
  }
}

/**
 * Tropical vegetation noise using advanced cellular automata
 */
class TropicalVegetationNoise {
  static generate(
    x: number,
    y: number,
    scale: number = 8,
    density: number = 0.85
  ): number {
    const cellSize = 1 / scale;
    const plants = [];

    // Generate multiple plant types in cells
    for (let xi = -1; xi <= 1; xi++) {
      for (let yi = -1; yi <= 1; yi++) {
        const cellX = Math.floor(x * scale) + xi;
        const cellY = Math.floor(y * scale) + yi;

        const hash = ((cellX * 127.1 + cellY * 311.7) % 1000) / 1000;
        const hash2 =
          (((cellX + 1) * 269.5 + (cellY + 1) * 183.3) % 1000) / 1000;

        // Multiple plants per cell for jungle density
        if (hash > 1 - density) {
          const plantX = cellX * cellSize + hash * cellSize * 0.7;
          const plantY = cellY * cellSize + hash2 * cellSize * 0.7;

          const plantSize = 0.4 + hash * 0.6; // Larger tropical plants
          const plantType =
            hash2 > 0.6 ? 'large_tree' : hash2 > 0.3 ? 'shrub' : 'fern';

          plants.push({
            x: plantX,
            y: plantY,
            size: plantSize,
            type: plantType,
          });
        }

        // Add secondary vegetation
        if (hash2 > 1 - density * 0.8) {
          const plantX = cellX * cellSize + hash2 * cellSize * 0.8;
          const plantY = cellY * cellSize + hash * cellSize * 0.8;

          plants.push({
            x: plantX,
            y: plantY,
            size: 0.2 + hash2 * 0.4,
            type: 'undergrowth',
          });
        }
      }
    }

    if (plants.length === 0) return 0;

    // Calculate combined influence from all plants
    let totalInfluence = 0;
    for (const plant of plants) {
      const dist = Math.hypot(x - plant.x, y - plant.y);
      const sizeMultiplier =
        plant.type === 'large_tree' ? 1.5 : plant.type === 'shrub' ? 1.0 : 0.7;
      const influence = Math.max(
        0,
        plant.size * sizeMultiplier - dist * scale * 1.2
      );
      totalInfluence += influence;
    }

    return MathUtils.clamp(totalInfluence, 0, 1);
  }
}

/**
 * Complex canopy system with multiple layers for jungle
 */
class JungleCanopyNoise {
  static generate(
    x: number,
    y: number,
    vegetationInfluence: number
  ): {
    emergent: number;
    canopy: number;
    understory: number;
  } {
    if (vegetationInfluence < 0.1) {
      return { emergent: 0, canopy: 0, understory: 0 };
    }

    const [warpedX, warpedY] = JungleDomainWarping.warpDomain(x, y, 6);

    // Emergent layer (tallest trees)
    const emergent =
      JungleDomainWarping.fbm(warpedX * 8, warpedY * 8, 3) *
      0.3 *
      (vegetationInfluence > 0.8 ? 1 : 0);

    // Main canopy layer
    const canopy =
      JungleDomainWarping.fbm(warpedX * 16, warpedY * 16, 4) * 0.6 +
      JungleDomainWarping.fbm(warpedX * 32, warpedY * 32, 3) * 0.2;

    // Understory layer
    const understory =
      JungleDomainWarping.fbm(warpedX * 64, warpedY * 64, 4) * 0.4 +
      JungleDomainWarping.fbm(warpedX * 128, warpedY * 128, 2) * 0.2;

    return {
      emergent: MathUtils.clamp(emergent * vegetationInfluence, 0, 1),
      canopy: MathUtils.clamp(canopy * vegetationInfluence, 0, 1),
      understory: MathUtils.clamp(understory * vegetationInfluence * 1.2, 0, 1),
    };
  }
}

/**
 * Vine and hanging vegetation patterns
 */
class VineNoise {
  static generate(x: number, y: number, treeInfluence: number): number {
    if (treeInfluence < 0.3) return 0;

    const [vineX, vineY] = JungleDomainWarping.vineFlowWarping(x, y, 12);

    // Create flowing vine patterns
    const vineFlow =
      Math.abs(JungleDomainWarping.fbm(vineX * 32, vineY * 8, 4)) * 0.6;
    const vineDetail =
      JungleDomainWarping.fbm(vineX * 128, vineY * 32, 3) * 0.3;

    const vines = (vineFlow + vineDetail) * treeInfluence * 0.8;
    return MathUtils.clamp(vines, 0, 1);
  }
}

// ============================================================================
// JUNGLE MATERIAL PROPERTIES
// ============================================================================

interface EnvironmentalFactors {
  moisture: number; // 0-1, affects vegetation lushness (high in jungle)
  temperature: number; // 0-1, affects plant growth (consistently high in jungle)
  season: number; // 0-1, affects fruit/flower presence
  elevation: number; // 0-1, affects jungle density and species
  humidity: number; // 0-1, specific to jungle environment
}

interface JungleVariation {
  name: string;
  // Tree/bark colors
  barkBaseColor: Color;
  barkSecondaryColor: Color;
  barkMossColor: Color;
  // Leaf colors (more vibrant than temperate forest)
  leafBaseColor: Color;
  leafSecondaryColor: Color;
  leafHighlightColor: Color;
  leafShadowColor: Color;
  // Tropical specific
  vineColor: Color;
  flowerAccentColor: Color;
  undergrowthColor: Color;
  jungleFloorColor: Color;
  // Material properties
  roughness: number;
  metallic: number;
  normalStrength: number;
  vegetationDensity: number;
  canopyThickness: number;
  veinDensity: number;
}

const JUNGLE_VARIATIONS: JungleVariation[] = [
  {
    name: 'dense_rainforest',
    barkBaseColor: new Color(0.12, 0.08, 0.06), // Very dark, humid bark
    barkSecondaryColor: new Color(0.2, 0.15, 0.1), // Moss-covered bark
    barkMossColor: new Color(0.1, 0.25, 0.08), // Moss and algae
    leafBaseColor: new Color(0.08, 0.4, 0.06), // Deep jungle green
    leafSecondaryColor: new Color(0.15, 0.55, 0.1), // Bright canopy green
    leafHighlightColor: new Color(0.25, 0.65, 0.15), // Sunlit leaves
    leafShadowColor: new Color(0.05, 0.3, 0.04), // Deep shadow green
    vineColor: new Color(0.18, 0.45, 0.12), // Hanging vine color
    flowerAccentColor: new Color(0.8, 0.4, 0.1), // Tropical flowers
    undergrowthColor: new Color(0.06, 0.3, 0.05), // Dense undergrowth
    jungleFloorColor: new Color(0.15, 0.12, 0.08), // Rich, decomposing matter
    roughness: 0.9,
    metallic: 0.0,
    normalStrength: 1.6,
    vegetationDensity: 0.95,
    canopyThickness: 0.9,
    veinDensity: 0.8,
  },
  {
    name: 'tropical_woodland',
    barkBaseColor: new Color(0.16, 0.12, 0.08),
    barkSecondaryColor: new Color(0.24, 0.18, 0.12),
    barkMossColor: new Color(0.14, 0.28, 0.1),
    leafBaseColor: new Color(0.12, 0.42, 0.08),
    leafSecondaryColor: new Color(0.18, 0.52, 0.12),
    leafHighlightColor: new Color(0.28, 0.62, 0.18),
    leafShadowColor: new Color(0.08, 0.32, 0.06),
    vineColor: new Color(0.2, 0.48, 0.14),
    flowerAccentColor: new Color(0.85, 0.35, 0.15),
    undergrowthColor: new Color(0.1, 0.32, 0.07),
    jungleFloorColor: new Color(0.18, 0.14, 0.1),
    roughness: 0.85,
    metallic: 0.0,
    normalStrength: 1.4,
    vegetationDensity: 0.8,
    canopyThickness: 0.75,
    veinDensity: 0.6,
  },
  {
    name: 'montane_jungle',
    barkBaseColor: new Color(0.14, 0.1, 0.07),
    barkSecondaryColor: new Color(0.22, 0.16, 0.11),
    barkMossColor: new Color(0.12, 0.26, 0.09),
    leafBaseColor: new Color(0.1, 0.38, 0.07),
    leafSecondaryColor: new Color(0.16, 0.48, 0.1),
    leafHighlightColor: new Color(0.24, 0.58, 0.14),
    leafShadowColor: new Color(0.06, 0.28, 0.05),
    vineColor: new Color(0.16, 0.42, 0.11),
    flowerAccentColor: new Color(0.75, 0.45, 0.2),
    undergrowthColor: new Color(0.08, 0.28, 0.06),
    jungleFloorColor: new Color(0.16, 0.13, 0.09),
    roughness: 0.8,
    metallic: 0.0,
    normalStrength: 1.2,
    vegetationDensity: 0.7,
    canopyThickness: 0.6,
    veinDensity: 0.5,
  },
];

// ============================================================================
// ADVANCED JUNGLE TEXTURE GENERATOR
// ============================================================================

export class AdvancedJungleGenerator {
  private resolution: number;
  private variation: JungleVariation;
  private environmentalFactors: EnvironmentalFactors;

  constructor(
    resolution: number = 1024,
    variationName: string = 'dense_rainforest',
    environmentalFactors: EnvironmentalFactors = {
      moisture: 0.95, // Very high moisture in jungle
      temperature: 0.85, // High tropical temperature
      season: 0.5,
      elevation: 0.3,
      humidity: 0.9, // High humidity
    }
  ) {
    this.resolution = resolution;
    this.variation =
      JUNGLE_VARIATIONS.find(v => v.name === variationName) ??
      JUNGLE_VARIATIONS[0];
    this.environmentalFactors = environmentalFactors;
  }

  /**
   * Generate complete PBR material set for jungle
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
   * Generate high-quality jungle albedo with dense tropical vegetation
   */
  private generateAlbedoMap(): ImageData {
    const canvas = new OffscreenCanvas(this.resolution, this.resolution);
    const ctx = canvas.getContext('2d');
    if (!ctx) {
      throw new Error('Failed to get 2D context from OffscreenCanvas');
    }
    const imageData = ctx.createImageData(this.resolution, this.resolution);
    const { data } = imageData;

    for (let y = 0; y < this.resolution; y++) {
      for (let x = 0; x < this.resolution; x++) {
        const u = x / this.resolution;
        const v = y / this.resolution;

        // Apply domain warping for organic jungle distribution
        const [warpedU, warpedV] = JungleDomainWarping.warpDomain(u, v, 4);
        const [vineWarpU, vineWarpV] = JungleDomainWarping.vineFlowWarping(
          u,
          v,
          8
        );

        // Generate vegetation placement and influence
        const vegetationInfluence = TropicalVegetationNoise.generate(
          warpedU,
          warpedV,
          10,
          this.variation.vegetationDensity * this.environmentalFactors.moisture
        );

        // Generate multi-layer canopy
        const canopyLayers = JungleCanopyNoise.generate(
          u,
          v,
          vegetationInfluence
        );

        // Generate vine coverage
        const vines = VineNoise.generate(u, v, vegetationInfluence);

        // Trunk/bark patterns for visible tree parts
        const barkPattern = Math.abs(
          JungleDomainWarping.fbm(vineWarpU * 16, vineWarpV * 32, 5)
        );
        const barkDetail =
          JungleDomainWarping.fbm(vineWarpU * 64, vineWarpV * 128, 4) * 0.4;

        // Undergrowth and floor patterns
        const undergrowthNoise = JungleDomainWarping.fbm(
          warpedU * 24,
          warpedV * 24,
          5
        );
        const floorDetail =
          JungleDomainWarping.fbm(warpedU * 48, warpedV * 48, 4) * 0.3;

        // Environmental variation for humidity and light
        const humidityNoise =
          JungleDomainWarping.fbm(u * 6, v * 6, 3) * 0.3 + 0.7;
        const actualHumidity =
          this.environmentalFactors.humidity * humidityNoise;

        // Light filtering through dense canopy
        const lightLevel =
          1 - (canopyLayers.canopy + canopyLayers.emergent) * 0.6;

        // Seasonal flowering/fruiting
        const seasonalAccent =
          Math.sin(this.environmentalFactors.season * Math.PI * 2) * 0.3;

        // Determine dominant material at this pixel
        let finalColor: Color;

        // Emergent layer (tallest trees)
        if (canopyLayers.emergent > 0.5) {
          finalColor = this.variation.leafHighlightColor.clone();

          // Add sunlight variation
          const sunVariation = JungleDomainWarping.fbm(
            warpedU * 32,
            warpedV * 32,
            2
          );
          finalColor.lerp(this.variation.leafBaseColor, sunVariation * 0.4);

          // Environmental brightness
          finalColor.multiplyScalar(0.9 + lightLevel * 0.4);
        }
        // Main canopy
        else if (canopyLayers.canopy > 0.4) {
          const canopyIntensity = canopyLayers.canopy;
          finalColor = this.variation.leafBaseColor.clone();

          // Vary between different leaf colors
          if (canopyIntensity > 0.7) {
            finalColor.lerp(
              this.variation.leafSecondaryColor,
              (canopyIntensity - 0.7) / 0.3
            );
          }

          // Add vine overlay
          if (vines > 0.3) {
            finalColor.lerp(this.variation.vineColor, vines * 0.4);
          }

          // Add seasonal flowers/fruits
          if (seasonalAccent > 0.2) {
            const flowerNoise = TropicalVegetationNoise.generate(u, v, 32, 0.1);
            if (flowerNoise > 0.8) {
              finalColor.lerp(
                this.variation.flowerAccentColor,
                seasonalAccent * 0.3
              );
            }
          }

          // Apply humidity and light effects
          finalColor.multiplyScalar(
            0.7 + actualHumidity * 0.2 + lightLevel * 0.3
          );
        }
        // Tree trunk/bark areas
        else if (vegetationInfluence > 0.6 && canopyLayers.understory < 0.3) {
          finalColor = this.variation.barkBaseColor.clone();

          // Bark texture variation
          const barkIntensity = MathUtils.clamp(barkPattern + barkDetail, 0, 1);
          if (barkIntensity > 0.6) {
            finalColor.lerp(
              this.variation.barkSecondaryColor,
              (barkIntensity - 0.6) / 0.4
            );
          }

          // Moss coverage from humidity
          if (actualHumidity > 0.8) {
            const mossNoise = JungleDomainWarping.fbm(
              warpedU * 64,
              warpedV * 64,
              3
            );
            if (mossNoise > 0.3) {
              finalColor.lerp(
                this.variation.barkMossColor,
                ((mossNoise - 0.3) / 0.7) * actualHumidity
              );
            }
          }

          // Humidity darkening effect
          finalColor.multiplyScalar(0.6 + actualHumidity * 0.3);
        }
        // Understory
        else if (canopyLayers.understory > 0.2 || vegetationInfluence > 0.3) {
          finalColor = this.variation.undergrowthColor.clone();

          // Add understory variation
          const undergrowthIntensity = undergrowthNoise * 0.5 + 0.5;
          if (undergrowthIntensity > 0.6) {
            finalColor.lerp(
              this.variation.leafShadowColor,
              undergrowthIntensity - 0.6
            );
          }

          // Vine integration
          if (vines > 0.2) {
            finalColor.lerp(this.variation.vineColor, vines * 0.5);
          }

          // Deep forest lighting
          finalColor.multiplyScalar(
            0.5 + lightLevel * 0.4 + actualHumidity * 0.1
          );
        }
        // Jungle floor
        else {
          finalColor = this.variation.jungleFloorColor.clone();

          // Rich organic matter variation
          const organicVariation = floorDetail * 0.5 + 0.5;
          finalColor.multiplyScalar(0.8 + organicVariation * 0.4);

          // Occasional undergrowth sprouting
          const sproutNoise = TropicalVegetationNoise.generate(u, v, 16, 0.2);
          if (sproutNoise > 0.7) {
            finalColor.lerp(
              this.variation.undergrowthColor,
              ((sproutNoise - 0.7) / 0.3) * 0.6
            );
          }
        }

        // Apply final environmental modulation
        finalColor.r = MathUtils.clamp(
          finalColor.r * (0.9 + floorDetail * 0.2),
          0,
          1
        );
        finalColor.g = MathUtils.clamp(
          finalColor.g * (0.9 + floorDetail * 0.2),
          0,
          1
        );
        finalColor.b = MathUtils.clamp(
          finalColor.b * (0.9 + floorDetail * 0.2),
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
   * Generate detailed normal map for complex jungle surface details
   */
  private generateNormalMap(): ImageData {
    const canvas = new OffscreenCanvas(this.resolution, this.resolution);
    const ctx = canvas.getContext('2d');
    if (!ctx) {
      throw new Error('Failed to get 2D context from OffscreenCanvas');
    }
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
   * Sample height data for normal map generation with jungle-specific details
   */
  private sampleHeightForNormal(u: number, v: number): number {
    const [warpedU, warpedV] = JungleDomainWarping.warpDomain(u, v, 4);
    const [vineWarpU, vineWarpV] = JungleDomainWarping.vineFlowWarping(u, v, 8);

    const vegetationInfluence = TropicalVegetationNoise.generate(
      warpedU,
      warpedV,
      10,
      this.variation.vegetationDensity
    );

    const canopyLayers = JungleCanopyNoise.generate(u, v, vegetationInfluence);
    const vines = VineNoise.generate(u, v, vegetationInfluence);

    let heightContribution = 0;

    // Emergent layer height (tallest)
    if (canopyLayers.emergent > 0.5) {
      const emergentDetail =
        JungleDomainWarping.fbm(warpedU * 64, warpedV * 64, 4) * 1.2;
      const leafMicro =
        JungleDomainWarping.fbm(warpedU * 256, warpedV * 256, 3) * 0.4;
      heightContribution = emergentDetail + leafMicro;
    }
    // Main canopy height
    else if (canopyLayers.canopy > 0.4) {
      const leafDetail =
        JungleDomainWarping.fbm(warpedU * 96, warpedV * 96, 4) * 0.8;
      const leafTexture =
        JungleDomainWarping.fbm(warpedU * 384, warpedV * 384, 3) * 0.3;

      // Add vine bump detail
      const vineBumps = vines > 0.3 ? vines * 0.4 : 0;

      heightContribution = leafDetail + leafTexture + vineBumps;
    }
    // Trunk/bark areas
    else if (vegetationInfluence > 0.6 && canopyLayers.understory < 0.3) {
      const barkDetail =
        Math.abs(JungleDomainWarping.fbm(vineWarpU * 32, vineWarpV * 64, 6)) *
        1.0;
      const barkTexture =
        JungleDomainWarping.fbm(vineWarpU * 128, vineWarpV * 256, 4) * 0.5;
      const mossTexture =
        JungleDomainWarping.fbm(warpedU * 96, warpedV * 96, 3) * 0.2;
      heightContribution = barkDetail + barkTexture + mossTexture;
    }
    // Understory and floor
    else {
      const undergrowthDetail =
        JungleDomainWarping.fbm(warpedU * 48, warpedV * 48, 4) * 0.5;
      const floorTexture =
        JungleDomainWarping.fbm(warpedU * 128, warpedV * 128, 4) * 0.3;
      const organicDetail =
        JungleDomainWarping.fbm(warpedU * 192, warpedV * 192, 3) * 0.2;
      heightContribution = undergrowthDetail + floorTexture + organicDetail;
    }

    return heightContribution;
  }

  /**
   * Generate roughness map with realistic jungle surface properties
   */
  private generateRoughnessMap(): ImageData {
    const canvas = new OffscreenCanvas(this.resolution, this.resolution);
    const ctx = canvas.getContext('2d');
    if (!ctx) {
      throw new Error('Failed to get 2D context from OffscreenCanvas');
    }
    const imageData = ctx.createImageData(this.resolution, this.resolution);
    const { data } = imageData;

    for (let y = 0; y < this.resolution; y++) {
      for (let x = 0; x < this.resolution; x++) {
        const u = x / this.resolution;
        const v = y / this.resolution;

        const [warpedU, warpedV] = JungleDomainWarping.warpDomain(u, v, 2);

        const vegetationInfluence = TropicalVegetationNoise.generate(
          warpedU,
          warpedV,
          10,
          this.variation.vegetationDensity
        );
        const canopyLayers = JungleCanopyNoise.generate(
          u,
          v,
          vegetationInfluence
        );
        const vines = VineNoise.generate(u, v, vegetationInfluence);

        let { roughness } = this.variation;

        // Different roughness for different jungle materials
        if (canopyLayers.emergent > 0.5 || canopyLayers.canopy > 0.4) {
          // Leaves - smooth when wet from humidity
          const leafRoughness = 0.4 - this.environmentalFactors.humidity * 0.2;
          const leafVariation =
            JungleDomainWarping.fbm(warpedU * 64, warpedV * 64, 3) * 0.2;
          roughness = leafRoughness + leafVariation;

          // Vine overlay increases roughness
          if (vines > 0.3) {
            roughness += vines * 0.3;
          }
        } else if (vegetationInfluence > 0.6 && canopyLayers.understory < 0.3) {
          // Bark - very rough, with moss making it smoother when wet
          const barkRoughness =
            0.95 - this.environmentalFactors.humidity * 0.15;
          const barkVariation =
            JungleDomainWarping.fbm(warpedU * 32, warpedV * 32, 3) * 0.1;
          roughness = barkRoughness + barkVariation;
        } else {
          // Jungle floor and undergrowth - variable roughness
          const floorVariation =
            JungleDomainWarping.fbm(warpedU * 24, warpedV * 24, 4) * 0.3;
          roughness = 0.7 + floorVariation;
        }

        // High humidity reduces overall roughness
        roughness *= 0.8 + this.environmentalFactors.humidity * 0.2;
        roughness = MathUtils.clamp(roughness, 0, 1);

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
   * Generate metallic map (jungle materials are non-metallic except for very wet conditions)
   */
  private generateMetallicMap(): ImageData {
    const canvas = new OffscreenCanvas(this.resolution, this.resolution);
    const ctx = canvas.getContext('2d');
    if (!ctx) {
      throw new Error('Failed to get 2D context from OffscreenCanvas');
    }
    const imageData = ctx.createImageData(this.resolution, this.resolution);
    const { data } = imageData;

    for (let y = 0; y < this.resolution; y++) {
      for (let x = 0; x < this.resolution; x++) {
        const u = x / this.resolution;
        const v = y / this.resolution;

        // Jungle materials are generally non-metallic
        let { metallic } = this.variation;

        // High humidity can create slight metallicism from water film
        if (this.environmentalFactors.humidity > 0.9) {
          const waterFilmNoise = TropicalVegetationNoise.generate(
            u,
            v,
            48,
            0.15
          );
          if (waterFilmNoise > 0.8) {
            metallic = Math.min(metallic + 0.04, 0.06);
          }
        }

        // Wet season effects
        if (this.environmentalFactors.moisture > 0.95) {
          const rainDropNoise = JungleDomainWarping.fbm(u * 96, v * 96, 2);
          if (rainDropNoise > 0.6) {
            metallic = Math.min(metallic + 0.02, 0.04);
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
   * Generate height/displacement map for terrain interaction
   */
  private generateHeightMap(): ImageData {
    const canvas = new OffscreenCanvas(this.resolution, this.resolution);
    const ctx = canvas.getContext('2d');
    if (!ctx) {
      throw new Error('Failed to get 2D context from OffscreenCanvas');
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
    adjacentTerrain: 'grassland' | 'forest' | 'mountain' | 'river' | 'desert'
  ): ImageData {
    const canvas = new OffscreenCanvas(this.resolution, this.resolution);
    const ctx = canvas.getContext('2d');
    if (!ctx) {
      throw new Error('Failed to get 2D context from OffscreenCanvas');
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
            // Jungle-grassland transition with edge degradation
            const edgeDistance = Math.min(u, 1 - u, v, 1 - v);
            const edgeVegetation = TropicalVegetationNoise.generate(
              u,
              v,
              6,
              0.5
            );
            blendFactor = MathUtils.smoothstep(
              0,
              0.5,
              edgeDistance + edgeVegetation * 0.4
            );
            break;

          case 'forest':
            // Natural jungle-forest transition
            const transitionNoise = JungleDomainWarping.fbm(u * 4, v * 4, 3);
            blendFactor = MathUtils.clamp(0.7 + transitionNoise * 0.3, 0.3, 1);
            break;

          case 'mountain':
            // Elevation-based jungle coverage (jungles at lower elevations)
            const elevationNoise = JungleDomainWarping.fbm(u * 3, v * 3, 3);
            const maxElevation = 0.4 + elevationNoise * 0.2;
            blendFactor =
              1 -
              MathUtils.smoothstep(
                maxElevation,
                maxElevation + 0.4,
                this.environmentalFactors.elevation
              );
            break;

          case 'river':
            // Dense riverside jungle vegetation
            const distToWater = Math.min(u, 1 - u, v, 1 - v);
            const riparianDensity = TropicalVegetationNoise.generate(
              u,
              v,
              8,
              0.95
            );
            blendFactor = MathUtils.clamp(
              riparianDensity * (1.2 - distToWater),
              0.5,
              1
            );
            break;

          case 'desert':
            // Sharp jungle-desert boundary (rare in nature)
            const desertDistance = Math.min(u, 1 - u);
            const humidityGradient = JungleDomainWarping.fbm(u * 6, v * 6, 2);
            blendFactor =
              MathUtils.smoothstep(
                0,
                0.3,
                desertDistance + humidityGradient * 0.1
              ) * 0.2; // Very sparse jungle at desert edge
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
      reject(new Error('Failed to get 2D context from OffscreenCanvas'));
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
 * Generate and export complete jungle texture set
 */
export const generateJungleTextureSet = async (
  variation: string = 'dense_rainforest',
  resolution: number = 1024,
  environmentalFactors?: EnvironmentalFactors
): Promise<{
  albedo: Blob;
  normal: Blob;
  roughness: Blob;
  metallic: Blob;
  height: Blob;
}> => {
  const generator = new AdvancedJungleGenerator(
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
