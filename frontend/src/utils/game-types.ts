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
   * Convert hex coordinates to pixel coordinates
   */
  static hexToPixel(hex: HexCoord): [number, number] {
    const x = this.HEX_SIZE * (this.SQRT_3 * hex.q + (this.SQRT_3 / 2) * hex.r);
    const z = this.HEX_SIZE * ((3 / 2) * hex.r);
    return [x, z];
  }

  /**
   * Convert pixel coordinates to hex coordinates
   */
  static pixelToHex(x: number, z: number): HexCoord {
    const q = ((this.SQRT_3 / 3) * x - (1 / 3) * z) / this.HEX_SIZE;
    const r = ((2 / 3) * z) / this.HEX_SIZE;
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
   * Get neighbors of a hex coordinate
   */
  static neighbors(hex: HexCoord): HexCoord[] {
    const directions = [
      { q: 1, r: 0 },
      { q: 1, r: -1 },
      { q: 0, r: -1 },
      { q: -1, r: 0 },
      { q: -1, r: 1 },
      { q: 0, r: 1 },
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

  // Generate hex grid
  const radius = 8;
  for (let q = -radius; q <= radius; q++) {
    const r1 = Math.max(-radius, -q - radius);
    const r2 = Math.min(radius, -q + radius);

    for (let r = r1; r <= r2; r++) {
      const id = tiles.length;
      const hex: HexCoord = { q, r };

      // Assign terrain based on distance from center
      const distance = Math.hypot(q, r);
      let terrain: TerrainType;

      if (distance < 1) {
        terrain = TerrainType.Plains;
      } else if (distance < 2) {
        terrain = TerrainType.Grassland;
      } else if (distance < 3) {
        const rand = Math.random();
        if (rand > 0.7) {
          terrain = TerrainType.Hills;
        } else if (rand > 0.4) {
          terrain = TerrainType.Forest;
        } else {
          terrain = TerrainType.Jungle;
        }
      } else if (distance < 5) {
        terrain =
          Math.random() > 0.7 ? TerrainType.Mountain : TerrainType.Hills;
      } else {
        terrain = TerrainType.Ocean;
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
