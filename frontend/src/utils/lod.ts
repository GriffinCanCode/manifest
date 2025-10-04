/**
 * LOD (Level of Detail) system utilities
 * Manages distance-based detail levels for optimal performance
 */

import { HexUtils, type HexCoord } from './game-types';

/** LOD distance thresholds in hex units */
export const LOD_THRESHOLDS = {
  /** Full geometry, textures, resources */
  HIGH_DETAIL: 15,
  /** Simplified geometry, basic colors */
  MEDIUM_DETAIL: 35,
  /** Single triangles, biome colors only */
  LOW_DETAIL: 70,
  /** Not rendered */
  CULLED: 100,
} as const;

/** LOD level enumeration */
export enum LODLevel {
  HIGH = 0,
  MEDIUM = 1,
  LOW = 2,
  CULLED = 3,
}

/** LOD configuration for different quality levels */
export const LOD_CONFIGS = {
  low: [LODLevel.LOW],
  medium: [LODLevel.MEDIUM, LODLevel.LOW],
  high: [LODLevel.HIGH, LODLevel.MEDIUM, LODLevel.LOW],
} as const;

/**
 * Calculate LOD level based on hex distance from camera with zoom scaling
 */
export const calculateLODLevel = (
  cameraHex: HexCoord,
  tileHex: HexCoord,
  zoomLevel: number = 1.0
): LODLevel => {
  const distance = HexUtils.distance(cameraHex, tileHex);

  // Scale LOD thresholds based on zoom level
  // When zoomed out (zoom < 1), increase thresholds to show more tiles
  // When zoomed in (zoom > 1), decrease thresholds for better detail
  const zoomScale = Math.max(0.1, 1.0 / zoomLevel);

  const scaledThresholds = {
    HIGH_DETAIL: LOD_THRESHOLDS.HIGH_DETAIL * zoomScale,
    MEDIUM_DETAIL: LOD_THRESHOLDS.MEDIUM_DETAIL * zoomScale,
    LOW_DETAIL: LOD_THRESHOLDS.LOW_DETAIL * zoomScale,
    CULLED: LOD_THRESHOLDS.CULLED * zoomScale,
  };

  if (distance <= scaledThresholds.HIGH_DETAIL) {
    return LODLevel.HIGH;
  } else if (distance <= scaledThresholds.MEDIUM_DETAIL) {
    return LODLevel.MEDIUM;
  } else if (distance <= scaledThresholds.LOW_DETAIL) {
    return LODLevel.LOW;
  } else {
    return LODLevel.CULLED;
  }
};

/**
 * Check if tile should be rendered at given LOD level
 */
export const shouldRenderAtLOD = (
  cameraHex: HexCoord,
  tileHex: HexCoord,
  requestedLODs: readonly number[],
  zoomLevel: number = 1.0
): boolean => {
  const tileLOD = calculateLODLevel(cameraHex, tileHex, zoomLevel);
  return requestedLODs.includes(tileLOD);
};

/**
 * Batch calculate LOD levels for multiple tiles
 */
export const calculateLODLevels = (
  cameraHex: HexCoord,
  tiles: readonly { hex: HexCoord }[],
  zoomLevel: number = 1.0
): LODLevel[] => {
  return tiles.map(tile => calculateLODLevel(cameraHex, tile.hex, zoomLevel));
};

/**
 * Filter tiles by LOD levels
 */
export const filterTilesByLOD = <T extends { hex: HexCoord }>(
  cameraHex: HexCoord,
  tiles: readonly T[],
  allowedLODs: readonly number[],
  zoomLevel: number = 1.0
): T[] => {
  return tiles.filter(tile =>
    shouldRenderAtLOD(cameraHex, tile.hex, allowedLODs, zoomLevel)
  );
};

/**
 * Get LOD level display name
 */
export const getLODLevelName = (level: LODLevel): string => {
  switch (level) {
    case LODLevel.HIGH:
      return 'High Detail';
    case LODLevel.MEDIUM:
      return 'Medium Detail';
    case LODLevel.LOW:
      return 'Low Detail';
    case LODLevel.CULLED:
      return 'Culled';
    default:
      return 'Unknown';
  }
};

/**
 * Get maximum render distance for quality level
 */
export const getMaxRenderDistance = (
  quality: 'low' | 'medium' | 'high'
): number => {
  switch (quality) {
    case 'low':
      return LOD_THRESHOLDS.LOW_DETAIL;
    case 'medium':
      return LOD_THRESHOLDS.LOW_DETAIL;
    case 'high':
      return LOD_THRESHOLDS.LOW_DETAIL;
    default:
      return LOD_THRESHOLDS.LOW_DETAIL;
  }
};
