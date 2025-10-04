/**
 * Centralized World and Camera Configuration
 *
 * This file defines consistent camera positions, map layouts, and coordinate
 * systems to be used across the entire application. ALL other files should
 * import from this configuration for consistency.
 */

import * as THREE from 'three';

// =============================================================================
// WORLD DIMENSIONS AND COORDINATE SYSTEM
// =============================================================================

/**
 * World generation parameters - MUST match backend exactly
 * Backend uses world_size = 75i32, generating -75 to +75 hex grid
 */
export const WORLD_CONFIG = {
  /** Backend world size parameter (generates -size to +size grid) */
  WORLD_SIZE: 75 as const,

  /** Total tiles per axis (2 * WORLD_SIZE + 1) */
  TILES_PER_AXIS: 151 as const,

  /** Total number of tiles in world */
  TOTAL_TILES: 22801 as const, // 151 * 151

  /** Hex spacing factor - ALIGNED with backend hex_to_pixel() */
  HEX_SPACING: 1.1 as const,

  /** Hex size for coordinate calculations */
  HEX_SIZE: 1.0 as const,

  /** Combined hex size with spacing */
  get EFFECTIVE_HEX_SIZE() {
    return this.HEX_SIZE * this.HEX_SPACING;
  },
} as const;

// =============================================================================
// CAMERA CONFIGURATION
// =============================================================================

/**
 * Standard camera positions and settings for different views
 * All camera positions should use these centralized values
 */
export const CAMERA_CONFIG = {
  /** Default spawn/initialization camera position */
  SPAWN: {
    position: [0, 50, 40] as const,
    target: [0, 0, 0] as const,
    zoom: 1.0 as const,
    fov: 65 as const,
  },

  /** Strategic overview for large world view */
  STRATEGIC: {
    position: [0, 120, 80] as const,
    target: [0, 0, 0] as const,
    zoom: 0.5 as const,
    fov: 75 as const,
  },

  /** Close-up tactical view */
  TACTICAL: {
    position: [0, 25, 20] as const,
    target: [0, 0, 0] as const,
    zoom: 2.0 as const,
    fov: 55 as const,
  },

  /** Camera constraints for orbital controls */
  CONSTRAINTS: {
    minDistance: 10,
    maxDistance: 500,
    minPolarAngle: Math.PI / 8, // Prevent looking from below
    maxPolarAngle: Math.PI / 2.1, // Prevent looking straight down
    enablePan: true,
    enableZoom: true,
    enableRotate: true,
  },

  /** Camera movement and animation settings */
  MOVEMENT: {
    dampingFactor: 0.05,
    enableDamping: true,
    autoRotate: false,
    autoRotateSpeed: 2.0,
    rotateSpeed: 0.5,
    panSpeed: 1.0,
    zoomSpeed: 1.0,
  },
} as const;

// =============================================================================
// MAP VISUALIZATION SETTINGS
// =============================================================================

/**
 * Settings for map rendering and tile streaming
 */
export const MAP_CONFIG = {
  /** Default tile streaming radius around camera */
  STREAMING_RADIUS: 100.0 as const,

  /** Maximum tiles to stream at once */
  MAX_STREAMING_TILES: 5000 as const,

  /** LOD (Level of Detail) settings */
  LOD: {
    HIGH_DISTANCE: 25.0 as const, // Full detail within this range
    MEDIUM_DISTANCE: 50.0 as const, // Medium detail within this range
    LOW_DISTANCE: 100.0 as const, // Low detail within this range
    // Beyond LOW_DISTANCE: culled
  },

  /** Tile rendering settings */
  TILES: {
    /** Default hex radius for rendering */
    HEX_RADIUS: 1.2 as const,

    /** Height scaling for elevation */
    ELEVATION_SCALE: 0.5 as const,

    /** Maximum instances in a single mesh */
    MAX_INSTANCES: 25000 as const,
  },
} as const;

// =============================================================================
// COORDINATE CONVERSION UTILITIES
// =============================================================================

/**
 * Convert hex coordinates to world pixel coordinates
 * ALIGNED with backend hex_to_pixel() function
 */
export const hexToPixel = (q: number, r: number): [number, number] => {
  const sqrt3 = Math.sqrt(3);
  const size = WORLD_CONFIG.EFFECTIVE_HEX_SIZE;

  const x = size * (sqrt3 * q + (sqrt3 / 2) * r);
  const z = size * (1.5 * r);

  return [x, z];
};

/**
 * Convert world pixel coordinates to hex coordinates
 * ALIGNED with backend pixel_to_hex() function
 */
export const pixelToHex = (x: number, z: number): { q: number; r: number } => {
  const sqrt3 = Math.sqrt(3);
  const size = WORLD_CONFIG.EFFECTIVE_HEX_SIZE;

  const q = ((sqrt3 / 3) * x - (1 / 3) * z) / size;
  const r = ((2 / 3) * z) / size;

  return { q: Math.round(q), r: Math.round(r) };
};

/**
 * Calculate world bounds for the entire map
 */
export const getWorldBounds = (): {
  minX: number;
  maxX: number;
  minZ: number;
  maxZ: number;
  centerX: number;
  centerZ: number;
} => {
  const size = WORLD_CONFIG.WORLD_SIZE;

  // Calculate extreme corners of the hex world
  const corners = [
    hexToPixel(-size, -size),
    hexToPixel(size, -size),
    hexToPixel(-size, size),
    hexToPixel(size, size),
    hexToPixel(0, -size),
    hexToPixel(0, size),
    hexToPixel(-size, 0),
    hexToPixel(size, 0),
  ];

  const xs = corners.map(([x, _]) => x);
  const zs = corners.map(([_, z]) => z);

  const minX = Math.min(...xs);
  const maxX = Math.max(...xs);
  const minZ = Math.min(...zs);
  const maxZ = Math.max(...zs);

  return {
    minX,
    maxX,
    minZ,
    maxZ,
    centerX: (minX + maxX) / 2,
    centerZ: (minZ + maxZ) / 2,
  };
};

/**
 * Get camera position that properly frames the entire world
 */
export const getWorldFramingCameraPosition = (): {
  position: [number, number, number];
  target: [number, number, number];
} => {
  const bounds = getWorldBounds();
  const worldWidth = bounds.maxX - bounds.minX;
  const worldHeight = bounds.maxZ - bounds.minZ;
  const worldSize = Math.max(worldWidth, worldHeight);

  // Position camera to frame the entire world with some padding
  const distance = worldSize * 0.8; // Distance factor for good viewing
  const height = worldSize * 0.6; // Height for good angle

  return {
    position: [bounds.centerX, height, bounds.centerZ + distance],
    target: [bounds.centerX, 0, bounds.centerZ],
  };
};

// =============================================================================
// CONSISTENCY HELPERS
// =============================================================================

type CameraType = 'SPAWN' | 'STRATEGIC' | 'TACTICAL';

/**
 * Creates a THREE.js camera with consistent settings
 */
export const createStandardCamera = (
  aspect = 1,
  cameraType: CameraType = 'SPAWN'
): THREE.PerspectiveCamera => {
  // Type-safe config access
  const config = CAMERA_CONFIG[cameraType];

  const camera = new THREE.PerspectiveCamera(config.fov, aspect, 0.1, 1000);

  camera.position.set(
    config.position[0],
    config.position[1],
    config.position[2]
  );
  camera.lookAt(config.target[0], config.target[1], config.target[2]);
  camera.zoom = config.zoom;
  camera.updateProjectionMatrix();

  return camera;
};

/**
 * Applies standard camera settings to an existing camera
 */
export const applyStandardCameraSettings = (
  camera: THREE.Camera,
  cameraType: CameraType = 'SPAWN'
): void => {
  // Type-safe config access
  const config = CAMERA_CONFIG[cameraType];

  camera.position.set(
    config.position[0],
    config.position[1],
    config.position[2]
  );

  if ('fov' in camera) {
    (camera as THREE.PerspectiveCamera).fov = config.fov;
    (camera as THREE.PerspectiveCamera).zoom = config.zoom;
    (camera as THREE.PerspectiveCamera).updateProjectionMatrix();
  }

  // For lookAt, we need to be more careful with Three.js
  if ('lookAt' in camera) {
    camera.lookAt(
      new THREE.Vector3(config.target[0], config.target[1], config.target[2])
    );
  }
};
