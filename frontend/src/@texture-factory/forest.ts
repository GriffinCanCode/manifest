/**
 * @texture-factory/forest.ts
 *
 * Advanced procedural forest texture generation
 * Designed to compete with AAA game quality (Civilization VI standard)
 *
 * Features:
 * - Cellular automata for realistic tree placement
 * - Fractal branching patterns for bark texture
 * - Multi-layer canopy with leaf density variation
 * - Seasonal color adaptation
 * - Undergrowth and forest floor detail
 * - Realistic PBR material properties
 * - Seamless tiling with advanced noise techniques
 */

import { Color, MathUtils } from 'three';

// ============================================================================
// ADVANCED FOREST NOISE FUNCTIONS
// ============================================================================

/**
 * Domain warping implementation adapted for forest patterns
 * Creates organic tree distribution and bark texture flow
 */
class ForestDomainWarping {
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

  static hash(x: number, y: number): number {
    let h = (x * 127.1 + y * 311.7) % 1000;
    h = ((h * 269.5) % 1000) / 1000;
    return h;
  }

  static warpDomain(x: number, y: number, scale: number = 1): [number, number] {
    const strength = 0.2; // Stronger warping for tree placement
    const warpX = this.fbm(x * scale, y * scale, 4) * strength;
    const warpY = this.fbm((x + 7.3) * scale, (y + 2.7) * scale, 4) * strength;

    return [x + warpX, y + warpY];
  }

  static treeFlowWarping(
    x: number,
    y: number,
    scale: number = 1
  ): [number, number] {
    // Specialized warping for tree trunk and branch patterns
    const strength = 0.15;
    const warpX = this.fbm(x * scale * 0.5, y * scale, 3) * strength;
    const warpY =
      this.fbm((x + 3.2) * scale, (y + 8.7) * scale * 0.8, 3) * strength;

    return [x + warpX, y + warpY];
  }
}

/**
 * Fractal branching noise for bark texture and tree structure
 */
class BranchingNoise {
  static generate(x: number, y: number, octaves: number = 6): number {
    let value = 0;
    let amplitude = 1;
    let frequency = 1;
    let weight = 1;

    for (let i = 0; i < octaves; i++) {
      let signal = Math.abs(
        ForestDomainWarping.noise(x * frequency, y * frequency)
      );

      // Create branching patterns by emphasizing directional flow
      signal = Math.pow(signal, 1.5);
      signal *= weight;

      value += signal * amplitude;
      weight = MathUtils.clamp(signal * 1.8, 0.1, 1);
      amplitude *= 0.6;
      frequency *= 1.8;
    }

    return MathUtils.clamp(value, 0, 1);
  }
}

/**
 * Tree placement using cellular automata for natural clustering
 */
class TreePlacementNoise {
  static generate(
    x: number,
    y: number,
    scale: number = 6,
    density: number = 0.7
  ): number {
    const cellSize = 1 / scale;
    const points = [];

    // Generate tree centers in surrounding cells
    for (let xi = -1; xi <= 1; xi++) {
      for (let yi = -1; yi <= 1; yi++) {
        const cellX = Math.floor(x * scale) + xi;
        const cellY = Math.floor(y * scale) + yi;

        const hash = ((cellX * 127.1 + cellY * 311.7) % 1000) / 1000;

        // Only place trees based on density threshold
        if (hash > 1 - density) {
          const pointX = cellX * cellSize + hash * cellSize * 0.8;
          const pointY =
            cellY * cellSize +
            (((hash * 269.5) % 1000) / 1000) * cellSize * 0.8;

          // Tree size variation
          const treeSize = 0.3 + hash * 0.4; // 0.3-0.7 relative size
          points.push({ x: pointX, y: pointY, size: treeSize });
        }
      }
    }

    if (points.length === 0) return 0;

    // Find influence from closest trees
    let treeInfluence = 0;
    for (const tree of points) {
      const dist = Math.hypot(x - tree.x, y - tree.y);
      const influence = Math.max(0, tree.size - dist * scale * 1.5);
      treeInfluence = Math.max(treeInfluence, influence);
    }

    return MathUtils.clamp(treeInfluence, 0, 1);
  }
}

/**
 * Canopy coverage noise for leaf density and light filtering
 */
class CanopyNoise {
  static generate(x: number, y: number, treeInfluence: number): number {
    if (treeInfluence < 0.1) return 0;

    const [warpedX, warpedY] = ForestDomainWarping.warpDomain(x, y, 8);

    // Multi-layer canopy
    const upperCanopy =
      ForestDomainWarping.fbm(warpedX * 16, warpedY * 16, 4) * 0.4;
    const midCanopy =
      ForestDomainWarping.fbm(warpedX * 32, warpedY * 32, 3) * 0.3;
    const lowerCanopy =
      ForestDomainWarping.fbm(warpedX * 64, warpedY * 64, 2) * 0.2;

    const totalCanopy =
      (upperCanopy + midCanopy + lowerCanopy + 0.1) * treeInfluence;
    return MathUtils.clamp(totalCanopy, 0, 1);
  }
}

// ============================================================================
// FOREST MATERIAL PROPERTIES
// ============================================================================

interface EnvironmentalFactors {
  moisture: number; // 0-1, affects vegetation lushness
  temperature: number; // 0-1, affects tree types and growth
  season: number; // 0-1, affects leaf colors and density
  elevation: number; // 0-1, affects tree line and species
}

interface ForestVariation {
  name: string;
  // Bark colors
  barkBaseColor: Color;
  barkSecondaryColor: Color;
  barkHighlightColor: Color;
  // Leaf colors
  leafBaseColor: Color;
  leafSecondaryColor: Color;
  leafAutumnColor: Color;
  // Undergrowth
  undergrowthColor: Color;
  forestFloorColor: Color;
  // Material properties
  roughness: number;
  metallic: number;
  normalStrength: number;
  treeDensity: number;
  canopyCoverage: number;
  undergrowthDensity: number;
}

const FOREST_VARIATIONS: ForestVariation[] = [
  {
    name: 'dense_forest',
    barkBaseColor: new Color(0.15, 0.12, 0.08), // Dark brown bark
    barkSecondaryColor: new Color(0.25, 0.2, 0.15), // Lighter bark
    barkHighlightColor: new Color(0.35, 0.28, 0.2), // Bark highlights
    leafBaseColor: new Color(0.12, 0.35, 0.08), // Deep forest green
    leafSecondaryColor: new Color(0.18, 0.45, 0.12), // Lighter green
    leafAutumnColor: new Color(0.45, 0.35, 0.15), // Autumn colors
    undergrowthColor: new Color(0.08, 0.25, 0.06), // Dark undergrowth
    forestFloorColor: new Color(0.2, 0.15, 0.1), // Rich soil
    roughness: 0.85,
    metallic: 0.0,
    normalStrength: 1.4,
    treeDensity: 0.9,
    canopyCoverage: 0.85,
    undergrowthDensity: 0.7,
  },
  {
    name: 'mixed_woodland',
    barkBaseColor: new Color(0.18, 0.14, 0.1),
    barkSecondaryColor: new Color(0.28, 0.22, 0.16),
    barkHighlightColor: new Color(0.38, 0.3, 0.22),
    leafBaseColor: new Color(0.15, 0.38, 0.1),
    leafSecondaryColor: new Color(0.22, 0.48, 0.15),
    leafAutumnColor: new Color(0.48, 0.38, 0.18),
    undergrowthColor: new Color(0.12, 0.28, 0.08),
    forestFloorColor: new Color(0.22, 0.17, 0.12),
    roughness: 0.8,
    metallic: 0.0,
    normalStrength: 1.2,
    treeDensity: 0.7,
    canopyCoverage: 0.65,
    undergrowthDensity: 0.5,
  },
  {
    name: 'sparse_forest',
    barkBaseColor: new Color(0.2, 0.16, 0.12),
    barkSecondaryColor: new Color(0.3, 0.24, 0.18),
    barkHighlightColor: new Color(0.4, 0.32, 0.24),
    leafBaseColor: new Color(0.18, 0.4, 0.12),
    leafSecondaryColor: new Color(0.25, 0.5, 0.18),
    leafAutumnColor: new Color(0.5, 0.4, 0.2),
    undergrowthColor: new Color(0.15, 0.3, 0.1),
    forestFloorColor: new Color(0.25, 0.2, 0.15),
    roughness: 0.75,
    metallic: 0.0,
    normalStrength: 1.0,
    treeDensity: 0.5,
    canopyCoverage: 0.45,
    undergrowthDensity: 0.3,
  },
];

// ============================================================================
// ADVANCED FOREST TEXTURE GENERATOR
// ============================================================================

export class AdvancedForestGenerator {
  private resolution: number;
  private variation: ForestVariation;
  private environmentalFactors: EnvironmentalFactors;

  constructor(
    resolution: number = 1024,
    variationName: string = 'dense_forest',
    environmentalFactors: EnvironmentalFactors = {
      moisture: 0.8,
      temperature: 0.6,
      season: 0.5,
      elevation: 0.4,
    }
  ) {
    this.resolution = resolution;
    this.variation =
      FOREST_VARIATIONS.find(v => v.name === variationName) ??
      FOREST_VARIATIONS[0];
    this.environmentalFactors = environmentalFactors;
  }

  /**
   * Generate complete PBR material set for forest
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
   * Generate high-quality forest albedo with trees, undergrowth, and forest floor
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

        // Apply domain warping for natural forest distribution
        const [warpedU, warpedV] = ForestDomainWarping.warpDomain(u, v, 3);
        const [treeWarpU, treeWarpV] = ForestDomainWarping.treeFlowWarping(
          u,
          v,
          6
        );

        // Generate tree placement and influence
        const treeInfluence = TreePlacementNoise.generate(
          warpedU,
          warpedV,
          8,
          this.variation.treeDensity * this.environmentalFactors.moisture
        );

        // Generate canopy coverage
        const canopyCoverage =
          CanopyNoise.generate(u, v, treeInfluence) *
          this.variation.canopyCoverage;

        // Generate bark texture for tree areas
        const barkPattern = BranchingNoise.generate(
          treeWarpU * 32,
          treeWarpV * 64,
          5
        );
        const barkDetail =
          ForestDomainWarping.fbm(treeWarpU * 128, treeWarpV * 256, 4) * 0.3;

        // Undergrowth patterns
        const undergrowthNoise = ForestDomainWarping.fbm(
          warpedU * 16,
          warpedV * 16,
          4
        );
        const undergrowthDensity =
          (undergrowthNoise * 0.5 + 0.5) * this.variation.undergrowthDensity;

        // Forest floor detail
        const floorDetail =
          ForestDomainWarping.fbm(warpedU * 64, warpedV * 64, 3) * 0.2;

        // Environmental variation
        const moistureNoise =
          ForestDomainWarping.fbm(u * 4, v * 4, 2) * 0.2 + 0.8;
        const actualMoisture =
          this.environmentalFactors.moisture * moistureNoise;

        // Seasonal variation for leaf colors
        const seasonalFactor = this.environmentalFactors.season;
        const autumnIntensity = Math.sin(seasonalFactor * Math.PI) * 0.7; // Peak in middle seasons

        // Determine primary material at this pixel
        let finalColor: Color;

        if (treeInfluence > 0.6 && canopyCoverage < 0.5) {
          // Tree trunk/bark areas
          const barkIntensity = MathUtils.clamp(barkPattern + barkDetail, 0, 1);
          finalColor = this.variation.barkBaseColor.clone();

          if (barkIntensity > 0.7) {
            finalColor.lerp(
              this.variation.barkHighlightColor,
              (barkIntensity - 0.7) / 0.3
            );
          } else if (barkIntensity > 0.4) {
            finalColor.lerp(
              this.variation.barkSecondaryColor,
              (barkIntensity - 0.4) / 0.3
            );
          }

          // Add moisture darkening
          finalColor.multiplyScalar(0.7 + actualMoisture * 0.3);
        } else if (canopyCoverage > 0.3) {
          // Canopy/leaf areas
          finalColor = this.variation.leafBaseColor.clone();

          // Seasonal color mixing
          if (autumnIntensity > 0.2) {
            finalColor.lerp(this.variation.leafAutumnColor, autumnIntensity);
          }

          // Add leaf density variation
          const leafVariation =
            ForestDomainWarping.fbm(warpedU * 128, warpedV * 128, 3) * 0.3;
          if (leafVariation > 0) {
            finalColor.lerp(this.variation.leafSecondaryColor, leafVariation);
          }

          // Environmental factors
          finalColor.multiplyScalar(0.8 + actualMoisture * 0.2);
        } else if (undergrowthDensity > 0.3) {
          // Undergrowth areas
          finalColor = this.variation.undergrowthColor.clone();

          // Mix with forest floor based on density
          const floorMix = (0.6 - undergrowthDensity) / 0.3;
          if (floorMix > 0) {
            finalColor.lerp(this.variation.forestFloorColor, floorMix);
          }

          // Add detail variation
          finalColor.multiplyScalar(0.9 + floorDetail);
        } else {
          // Forest floor
          finalColor = this.variation.forestFloorColor.clone();

          // Add soil/leaf litter variation
          const litterVariation =
            ForestDomainWarping.fbm(warpedU * 32, warpedV * 32, 3) * 0.2;
          finalColor.multiplyScalar(0.9 + litterVariation);
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
   * Generate detailed normal map for bark texture, leaf surface detail, and forest floor
   */
  private generateNormalMap(): ImageData {
    const canvas = new OffscreenCanvas(this.resolution, this.resolution);
    const ctx = canvas.getContext('2d');
    if (!ctx) {
      throw new Error('Failed to get 2D context from OffscreenCanvas');
    }
    const imageData = ctx.createImageData(this.resolution, this.resolution);
    const { data } = imageData;

    const heightScale = this.variation.normalStrength * 0.015;

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
   * Sample height data for normal map generation
   */
  private sampleHeightForNormal(u: number, v: number): number {
    const [warpedU, warpedV] = ForestDomainWarping.warpDomain(u, v, 4);
    const [treeWarpU, treeWarpV] = ForestDomainWarping.treeFlowWarping(u, v, 6);

    const treeInfluence = TreePlacementNoise.generate(
      warpedU,
      warpedV,
      8,
      this.variation.treeDensity
    );
    const canopyCoverage = CanopyNoise.generate(u, v, treeInfluence);

    let heightContribution = 0;

    // Tree trunk height (most pronounced)
    if (treeInfluence > 0.6 && canopyCoverage < 0.5) {
      const barkDetail =
        BranchingNoise.generate(treeWarpU * 64, treeWarpV * 128, 6) * 0.8;
      const barkMicro =
        ForestDomainWarping.fbm(treeWarpU * 256, treeWarpV * 512, 4) * 0.3;
      heightContribution = barkDetail + barkMicro;
    }

    // Canopy height variation
    else if (canopyCoverage > 0.3) {
      const leafDetail =
        ForestDomainWarping.fbm(warpedU * 128, warpedV * 128, 4) * 0.4;
      const leafMicro =
        ForestDomainWarping.fbm(warpedU * 512, warpedV * 512, 3) * 0.2;
      heightContribution = leafDetail + leafMicro;
    }

    // Forest floor variation
    else {
      const floorDetail =
        ForestDomainWarping.fbm(warpedU * 64, warpedV * 64, 3) * 0.3;
      const microDetail =
        ForestDomainWarping.fbm(warpedU * 256, warpedV * 256, 4) * 0.15;
      heightContribution = floorDetail + microDetail;
    }

    return heightContribution;
  }

  /**
   * Generate roughness map with realistic forest surface properties
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

        const [warpedU, warpedV] = ForestDomainWarping.warpDomain(u, v, 2);

        const treeInfluence = TreePlacementNoise.generate(
          warpedU,
          warpedV,
          8,
          this.variation.treeDensity
        );
        const canopyCoverage = CanopyNoise.generate(u, v, treeInfluence);

        let { roughness } = this.variation;

        // Different roughness for different materials
        if (treeInfluence > 0.6 && canopyCoverage < 0.5) {
          // Bark - very rough
          roughness =
            0.9 + ForestDomainWarping.fbm(warpedU * 32, warpedV * 32, 2) * 0.1;
        } else if (canopyCoverage > 0.3) {
          // Leaves - moderately rough, affected by moisture
          const leafRoughness = 0.7 - this.environmentalFactors.moisture * 0.2;
          roughness =
            leafRoughness +
            ForestDomainWarping.fbm(warpedU * 64, warpedV * 64, 3) * 0.15;
        } else {
          // Forest floor - variable roughness
          const floorVariation =
            ForestDomainWarping.fbm(warpedU * 16, warpedV * 16, 3) * 0.2;
          roughness = 0.6 + floorVariation;
        }

        // Environmental modulation
        roughness *= 0.7 + this.environmentalFactors.moisture * 0.3;
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
   * Generate metallic map (forest materials are generally non-metallic)
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
        // Forest materials are non-metallic
        let { metallic } = this.variation;

        // Very slight metallicism for wet conditions (morning dew, rain)
        if (this.environmentalFactors.moisture > 0.85) {
          const dewNoise = TreePlacementNoise.generate(
            x / this.resolution,
            y / this.resolution,
            32,
            0.3
          );
          metallic = Math.min(metallic + dewNoise * 0.03, 0.05);
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
    adjacentTerrain: 'grassland' | 'mountain' | 'river' | 'desert'
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
            // Natural forest-grassland transition with scattered trees
            const edgeDistance = Math.min(u, 1 - u, v, 1 - v);
            const forestEdge = TreePlacementNoise.generate(u, v, 4, 0.4);
            blendFactor = MathUtils.smoothstep(
              0,
              0.4,
              edgeDistance + forestEdge * 0.3
            );
            break;

          case 'mountain':
            // Tree line elevation effect
            const elevationNoise = ForestDomainWarping.fbm(u * 3, v * 3, 3);
            const treeLineHeight = 0.6 + elevationNoise * 0.2;
            blendFactor = MathUtils.smoothstep(
              treeLineHeight,
              treeLineHeight + 0.3,
              this.environmentalFactors.elevation
            );
            break;

          case 'river':
            // Dense vegetation near water, thinning with distance
            const distToWater = Math.min(u, 1 - u, v, 1 - v);
            const riparianDensity = TreePlacementNoise.generate(u, v, 6, 0.8);
            blendFactor = MathUtils.clamp(
              riparianDensity * (1 - distToWater * 2),
              0,
              1
            );
            break;

          case 'desert':
            // Very sparse forest at desert edge, mostly stunted trees
            const desertDistance = Math.min(u, 1 - u);
            const desertEdge = ForestDomainWarping.fbm(u * 8, v * 8, 2);
            blendFactor =
              MathUtils.smoothstep(0, 0.5, desertDistance + desertEdge * 0.2) *
              0.3;
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
 * Generate and export complete forest texture set
 */
export const generateForestTextureSet = async (
  variation: string = 'dense_forest',
  resolution: number = 1024,
  environmentalFactors?: EnvironmentalFactors
): Promise<{
  albedo: Blob;
  normal: Blob;
  roughness: Blob;
  metallic: Blob;
  height: Blob;
}> => {
  const generator = new AdvancedForestGenerator(
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
