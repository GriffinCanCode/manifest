/**
 * Game data types and utilities
 * Extracted from GameCanvas for reusability
 */

// Terrain type definitions matching backend
export enum TerrainType {
  Ocean = 'ocean',
  Grassland = 'grassland',
  Plains = 'plains',
  Desert = 'desert',
  Tundra = 'tundra',
  Snow = 'snow',
  Forest = 'forest',
  Jungle = 'jungle',
  Hills = 'hills',
  Mountain = 'mountain',
}

// Hex coordinate system
export interface HexCoord {
  q: number;
  r: number;
}

// Tile data structure
export interface GameTile {
  id: number;
  hex: HexCoord;
  terrain: TerrainType;
  elevation: number;
  worldX: number;
  worldZ: number;
  biome?: number;
  resourceMask?: number;
  resources?: string[];
}

// Unit data structure
export interface GameUnit {
  id: number;
  type: string;
  position: HexCoord;
  playerId: number;
  health: number;
}

// Game world container
export interface GameWorld {
  tiles: GameTile[];
  units: GameUnit[];
}

/**
 * Hex utility functions
 */
export class HexUtils {
  private static readonly SQRT_3 = Math.sqrt(3);
  private static readonly HEX_SIZE = 1;

  /**
   * Convert hex coordinates to pixel coordinates (pointy-top hexagons - Civ6 style)
   */
  static hexToPixel(hex: HexCoord): [number, number] {
    // Pointy-top hexagon layout for Civ6 style
    const size = this.HEX_SIZE * 1.1; // ALIGNED with backend hex_to_pixel() spacing
    const x = size * ((3 / 2) * hex.q);
    const z = size * ((this.SQRT_3 / 2) * hex.q + this.SQRT_3 * hex.r);
    return [x, z];
  }

  /**
   * Convert pixel coordinates to hex coordinates (pointy-top hexagons - Civ6 style)
   */
  static pixelToHex(x: number, z: number): HexCoord {
    const size = this.HEX_SIZE * 1.1; // ALIGNED with backend hex_to_pixel() spacing
    const q = ((2 / 3) * x) / size;
    const r = ((-1 / 3) * x + (this.SQRT_3 / 3) * z) / size;
    return this.roundHex({ q, r });
  }

  /**
   * Round fractional hex coordinates to nearest hex
   */
  static roundHex(hex: { q: number; r: number }): HexCoord {
    const s = -hex.q - hex.r;
    let rq = Math.round(hex.q);
    let rr = Math.round(hex.r);
    const rs = Math.round(s);

    const qDiff = Math.abs(rq - hex.q);
    const rDiff = Math.abs(rr - hex.r);
    const sDiff = Math.abs(rs - s);

    if (qDiff > rDiff && qDiff > sDiff) {
      rq = -rr - rs;
    } else if (rDiff > sDiff) {
      rr = -rq - rs;
    }

    return { q: rq, r: rr };
  }

  /**
   * Calculate distance between two hex coordinates
   */
  static distance(a: HexCoord, b: HexCoord): number {
    return (
      (Math.abs(a.q - b.q) +
        Math.abs(a.q + a.r - b.q - b.r) +
        Math.abs(a.r - b.r)) /
      2
    );
  }

  /**
   * Get neighbors of a hex coordinate (pointy-top hexagons - Civ6 style)
   */
  static neighbors(hex: HexCoord): HexCoord[] {
    // Pointy-top hexagon neighbor directions
    const directions = [
      { q: 1, r: 0 }, // East
      { q: 0, r: 1 }, // Southeast
      { q: -1, r: 1 }, // Southwest
      { q: -1, r: 0 }, // West
      { q: 0, r: -1 }, // Northwest
      { q: 1, r: -1 }, // Northeast
    ];

    return directions.map(dir => ({
      q: hex.q + dir.q,
      r: hex.r + dir.r,
    }));
  }
}

/**
 * Create a mock game world for testing
 * Replace with actual game data loading
 */
export const createMockGameWorld = (): GameWorld => {
  const tiles: GameTile[] = [];
  const units: GameUnit[] = [];

  // Generate hex grid - LARGER radius to demonstrate texture variety
  const radius = 18; // Increased from 6 to 18 for ~1000+ tiles
  for (let q = -radius; q <= radius; q++) {
    const r1 = Math.max(-radius, -q - radius);
    const r2 = Math.min(radius, -q + radius);

    for (let r = r1; r <= r2; r++) {
      const id = tiles.length;
      const hex: HexCoord = { q, r };

      // Assign terrain based on distance from center - more variety
      const distance = Math.hypot(q, r);
      let terrain: TerrainType;

      // Create diverse biome zones to showcase different textures
      const angle = Math.atan2(r, q); // Angle for creating directional biomes

      // Deterministic pseudo-random based on coordinates
      const seed = Math.abs(q * 17 + r * 23) % 1000;
      const noise = seed / 1000;

      if (distance < 2) {
        // Central grassland and plains core
        terrain = noise > 0.5 ? TerrainType.Plains : TerrainType.Grassland;
      } else if (distance < 5) {
        // Inner fertile ring with varied terrain - ensure all biome types appear
        if (noise < 0.15) terrain = TerrainType.Forest;
        else if (noise < 0.3) terrain = TerrainType.Jungle;
        else if (noise < 0.45) terrain = TerrainType.Grassland;
        else if (noise < 0.6) terrain = TerrainType.Plains;
        else if (noise < 0.75) terrain = TerrainType.Hills;
        else terrain = TerrainType.Desert;
      } else if (distance < 10) {
        // Climate zones based on angle to create distinct regions
        const climateZone = ((angle + Math.PI) / (2 * Math.PI)) * 8;

        if (climateZone < 1) terrain = TerrainType.Desert;
        else if (climateZone < 2) terrain = TerrainType.Plains;
        else if (climateZone < 3) terrain = TerrainType.Forest;
        else if (climateZone < 4) terrain = TerrainType.Jungle;
        else if (climateZone < 5) terrain = TerrainType.Grassland;
        else if (climateZone < 6) terrain = TerrainType.Hills;
        else if (climateZone < 7) terrain = TerrainType.Tundra;
        else terrain = TerrainType.Mountain;

        // Add mountainous variation
        if (noise > 0.9) terrain = TerrainType.Mountain;
        else if (noise > 0.85) terrain = TerrainType.Hills;
      } else if (distance < 15) {
        // Outer harsh terrain
        if (noise < 0.3) terrain = TerrainType.Mountain;
        else if (noise < 0.5) terrain = TerrainType.Hills;
        else if (noise < 0.7) terrain = TerrainType.Tundra;
        else if (noise < 0.85) terrain = TerrainType.Desert;
        else terrain = TerrainType.Forest;
      } else {
        // Ocean boundary with islands
        if (noise < 0.8) {
          terrain = TerrainType.Ocean;
        } else {
          // Remote islands with varied terrain
          if (noise > 0.95) terrain = TerrainType.Mountain;
          else if (noise > 0.9) terrain = TerrainType.Hills;
          else terrain = TerrainType.Forest;
        }
      }

      // Generate elevation based on terrain
      let elevation: number;
      switch (terrain) {
        case TerrainType.Ocean:
          elevation = -0.2 + Math.random() * 0.1;
          break;
        case TerrainType.Plains:
        case TerrainType.Grassland:
          elevation = 0.1 + Math.random() * 0.3;
          break;
        case TerrainType.Forest:
        case TerrainType.Jungle:
          elevation = 0.2 + Math.random() * 0.4;
          break;
        case TerrainType.Hills:
          elevation = 0.4 + Math.random() * 0.6;
          break;
        case TerrainType.Mountain:
          elevation = 0.8 + Math.random() * 0.4;
          break;
        default:
          elevation = 0.1 + Math.random() * 0.2;
      }

      const [worldX, worldZ] = HexUtils.hexToPixel(hex);

      tiles.push({
        id,
        hex,
        terrain,
        elevation,
        worldX,
        worldZ,
        resources: Math.random() > 0.8 ? ['Iron'] : undefined,
      });
    }
  }

  // Add some units
  const unitPositions: HexCoord[] = [
    { q: -1, r: 1 },
    { q: 2, r: -1 },
    { q: -2, r: 0 },
    { q: 1, r: 1 },
    { q: 0, r: -2 },
  ];

  unitPositions.forEach((position, index) => {
    units.push({
      id: index,
      type: index % 2 === 0 ? 'Warrior' : 'Scout',
      position,
      playerId: index % 3,
      health: 80 + Math.random() * 20,
    });
  });

  return { tiles, units };
};
