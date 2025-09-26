/**
 * Backend Data Validation Tests
 * Tests to validate that the backend is providing correct tile data structure and content
 */

import { invoke } from '@tauri-apps/api/core';

import type { GameTile } from '../src/utils/game-types';
import {
  TileDataService,
  type TileStreamingRequest,
} from '../src/utils/tile-data-service';

interface ValidationResult {
  isValid: boolean;
  errors: string[];
  warnings: string[];
  data?: any;
}

/**
 * Test suite for backend tile data validation
 */
export class BackendDataValidator {
  private tileService: TileDataService;

  constructor() {
    this.tileService = new TileDataService();
  }

  /**
   * Run all backend validation tests
   */
  async runAllTests(): Promise<ValidationResult> {
    const results: ValidationResult = {
      isValid: true,
      errors: [],
      warnings: [],
    };

    console.log('🔍 Starting Backend Data Validation Tests...');

    const tests = [
      this.testBackendConnection,
      this.testTileStreamingResponse,
      this.testTileDataStructure,
      this.testTileCoordinates,
      this.testTileTerrainTypes,
      this.testInstanceData,
      this.testStreamingPerformance,
    ];

    for (const test of tests) {
      try {
        const testResult = await test.call(this);
        if (!testResult.isValid) {
          results.isValid = false;
        }
        results.errors.push(...testResult.errors);
        results.warnings.push(...testResult.warnings);
      } catch (error) {
        results.isValid = false;
        results.errors.push(
          `Test ${test.name} failed with error: ${error instanceof Error ? error.message : String(error)}`
        );
      }
    }

    return results;
  }

  /**
   * Test 1: Backend Connection
   */
  async testBackendConnection(): Promise<ValidationResult> {
    console.log('📡 Testing backend connection...');

    try {
      // Try to invoke a basic backend command
      const response = await invoke<string>('get_backend_status');

      if (response) {
        return {
          isValid: true,
          errors: [],
          warnings: [],
          data: response,
        };
      } else {
        return {
          isValid: false,
          errors: ['Backend returned empty status'],
          warnings: [],
        };
      }
    } catch (error) {
      return {
        isValid: false,
        errors: [
          `Backend connection failed: ${error instanceof Error ? error.message : String(error)}`,
        ],
        warnings: [],
      };
    }
  }

  /**
   * Test 2: Tile Streaming Response Structure
   */
  async testTileStreamingResponse(): Promise<ValidationResult> {
    console.log('🌍 Testing tile streaming response structure...');

    const request: TileStreamingRequest = {
      camera_position: [0, 0, 0],
      view_radius: 20,
      max_tiles: 100,
      lod_levels: [0],
      generation: 0,
    };

    try {
      const response = await this.tileService.streamTiles(request);

      const errors: string[] = [];
      const warnings: string[] = [];

      // Check response structure
      if (!response) {
        errors.push('Tile streaming returned null/undefined response');
        return { isValid: false, errors, warnings };
      }

      if (!Array.isArray(response.tiles)) {
        errors.push('Response.tiles is not an array');
      }

      if (!Array.isArray(response.instance_data)) {
        errors.push('Response.instance_data is not an array');
      }

      if (typeof response.generation !== 'number') {
        errors.push('Response.generation is not a number');
      }

      if (typeof response.has_more !== 'boolean') {
        errors.push('Response.has_more is not a boolean');
      }

      // Check if we got tiles
      if (response.tiles.length === 0) {
        warnings.push('No tiles returned from backend (empty world?)');
      }

      // Check instance data matches tiles
      if (response.tiles.length !== response.instance_data.length) {
        warnings.push(
          `Mismatch between tiles count (${response.tiles.length}) and instance data count (${response.instance_data.length})`
        );
      }

      return {
        isValid: errors.length === 0,
        errors,
        warnings,
        data: {
          tileCount: response.tiles.length,
          instanceDataCount: response.instance_data.length,
          generation: response.generation,
          hasMore: response.has_more,
        },
      };
    } catch (error) {
      return {
        isValid: false,
        errors: [
          `Tile streaming failed: ${error instanceof Error ? error.message : String(error)}`,
        ],
        warnings: [],
      };
    }
  }

  /**
   * Test 3: Individual Tile Data Structure
   */
  async testTileDataStructure(): Promise<ValidationResult> {
    console.log('🔍 Testing individual tile data structure...');

    const request: TileStreamingRequest = {
      camera_position: [0, 0, 0],
      view_radius: 10,
      max_tiles: 50,
      lod_levels: [0],
      generation: 0,
    };

    try {
      const response = await this.tileService.streamTiles(request);

      if (!response.tiles || response.tiles.length === 0) {
        return {
          isValid: false,
          errors: ['No tiles available for structure testing'],
          warnings: [],
        };
      }

      const errors: string[] = [];
      const warnings: string[] = [];

      // Test first few tiles
      const testTiles = response.tiles.slice(
        0,
        Math.min(10, response.tiles.length)
      );

      for (let i = 0; i < testTiles.length; i++) {
        const tile = testTiles[i];
        const tileError = (msg: string) => `Tile ${i} (id: ${tile.id}): ${msg}`;

        // Required fields
        if (typeof tile.id !== 'number') {
          errors.push(tileError('id is not a number'));
        }

        if (
          !tile.hex ||
          typeof tile.hex.q !== 'number' ||
          typeof tile.hex.r !== 'number'
        ) {
          errors.push(tileError('hex coordinates are invalid'));
        }

        if (!tile.terrain || typeof tile.terrain !== 'string') {
          errors.push(tileError('terrain type is missing or invalid'));
        }

        if (typeof tile.elevation !== 'number') {
          errors.push(tileError('elevation is not a number'));
        }

        if (
          typeof tile.worldX !== 'number' ||
          typeof tile.worldZ !== 'number'
        ) {
          errors.push(tileError('world coordinates are not numbers'));
        }

        // Value range validation
        if (Math.abs(tile.elevation) > 100) {
          warnings.push(
            tileError(`extreme elevation value: ${tile.elevation}`)
          );
        }

        if (Math.abs(tile.worldX) > 10000 || Math.abs(tile.worldZ) > 10000) {
          warnings.push(
            tileError(
              `extreme world coordinates: (${tile.worldX}, ${tile.worldZ})`
            )
          );
        }
      }

      return {
        isValid: errors.length === 0,
        errors,
        warnings,
        data: {
          tilesChecked: testTiles.length,
          sampleTile: testTiles[0],
        },
      };
    } catch (error) {
      return {
        isValid: false,
        errors: [
          `Tile structure test failed: ${error instanceof Error ? error.message : String(error)}`,
        ],
        warnings: [],
      };
    }
  }

  /**
   * Test 4: Hex Coordinate Consistency
   */
  async testTileCoordinates(): Promise<ValidationResult> {
    console.log('🗺️ Testing tile coordinate consistency...');

    const request: TileStreamingRequest = {
      camera_position: [0, 0, 0],
      view_radius: 15,
      max_tiles: 200,
      lod_levels: [0],
      generation: 0,
    };

    try {
      const response = await this.tileService.streamTiles(request);

      if (!response.tiles || response.tiles.length === 0) {
        return {
          isValid: false,
          errors: ['No tiles available for coordinate testing'],
          warnings: [],
        };
      }

      const errors: string[] = [];
      const warnings: string[] = [];
      const coordinateSet = new Set<string>();
      const duplicates: GameTile[] = [];

      for (const tile of response.tiles) {
        const coordKey = `${tile.hex.q},${tile.hex.r}`;

        if (coordinateSet.has(coordKey)) {
          duplicates.push(tile);
          errors.push(
            `Duplicate hex coordinate found: q=${tile.hex.q}, r=${tile.hex.r}`
          );
        } else {
          coordinateSet.add(coordKey);
        }

        // Check coordinate ranges
        if (Math.abs(tile.hex.q) > 1000 || Math.abs(tile.hex.r) > 1000) {
          warnings.push(
            `Extreme hex coordinates for tile ${tile.id}: q=${tile.hex.q}, r=${tile.hex.r}`
          );
        }
      }

      return {
        isValid: errors.length === 0,
        errors,
        warnings,
        data: {
          totalTiles: response.tiles.length,
          uniqueCoordinates: coordinateSet.size,
          duplicateCount: duplicates.length,
        },
      };
    } catch (error) {
      return {
        isValid: false,
        errors: [
          `Coordinate consistency test failed: ${error instanceof Error ? error.message : String(error)}`,
        ],
        warnings: [],
      };
    }
  }

  /**
   * Test 5: Terrain Type Validation
   */
  async testTileTerrainTypes(): Promise<ValidationResult> {
    console.log('🌲 Testing terrain type validity...');

    const validTerrainTypes = [
      'ocean',
      'grassland',
      'plains',
      'desert',
      'tundra',
      'snow',
      'forest',
      'jungle',
      'hills',
      'mountain',
    ];

    const request: TileStreamingRequest = {
      camera_position: [0, 0, 0],
      view_radius: 20,
      max_tiles: 300,
      lod_levels: [0],
      generation: 0,
    };

    try {
      const response = await this.tileService.streamTiles(request);

      if (!response.tiles || response.tiles.length === 0) {
        return {
          isValid: false,
          errors: ['No tiles available for terrain testing'],
          warnings: [],
        };
      }

      const errors: string[] = [];
      const warnings: string[] = [];
      const terrainCounts: Record<string, number> = {};

      for (const tile of response.tiles) {
        const terrain = tile.terrain;

        if (!validTerrainTypes.includes(terrain)) {
          errors.push(
            `Invalid terrain type '${terrain}' found on tile ${tile.id}`
          );
        }

        terrainCounts[terrain] = (terrainCounts[terrain] || 0) + 1;
      }

      // Check terrain distribution
      const totalTiles = response.tiles.length;
      for (const [terrain, count] of Object.entries(terrainCounts)) {
        const percentage = (count / totalTiles) * 100;
        if (percentage > 80) {
          warnings.push(
            `Terrain '${terrain}' dominates ${percentage.toFixed(1)}% of tiles - possible generation issue`
          );
        }
      }

      return {
        isValid: errors.length === 0,
        errors,
        warnings,
        data: {
          totalTiles,
          terrainDistribution: terrainCounts,
        },
      };
    } catch (error) {
      return {
        isValid: false,
        errors: [
          `Terrain type test failed: ${error instanceof Error ? error.message : String(error)}`,
        ],
        warnings: [],
      };
    }
  }

  /**
   * Test 6: Instance Data Validation
   */
  async testInstanceData(): Promise<ValidationResult> {
    console.log('⚡ Testing instance data structure...');

    const request: TileStreamingRequest = {
      camera_position: [0, 0, 0],
      view_radius: 15,
      max_tiles: 150,
      lod_levels: [0],
      generation: 0,
    };

    try {
      const response = await this.tileService.streamTiles(request);

      if (!response.instance_data || response.instance_data.length === 0) {
        return {
          isValid: false,
          errors: ['No instance data available for testing'],
          warnings: [],
        };
      }

      const errors: string[] = [];
      const warnings: string[] = [];

      // Test first few instance data entries
      const testInstances = response.instance_data.slice(
        0,
        Math.min(10, response.instance_data.length)
      );

      for (let i = 0; i < testInstances.length; i++) {
        const instance = testInstances[i];
        const instanceError = (msg: string) => `Instance ${i}: ${msg}`;

        // Check required fields based on TileInstanceData type
        if (typeof instance.tileId !== 'number') {
          errors.push(instanceError('tileId is not a number'));
        }

        if (
          !Array.isArray(instance.position) ||
          instance.position.length !== 3
        ) {
          errors.push(instanceError('position is not a 3-element array'));
        }

        if (typeof instance.height !== 'number') {
          errors.push(instanceError('height is not a number'));
        }

        if (!Array.isArray(instance.color) || instance.color.length !== 3) {
          errors.push(instanceError('color is not a 3-element array (RGB)'));
        }
      }

      return {
        isValid: errors.length === 0,
        errors,
        warnings,
        data: {
          instanceCount: response.instance_data.length,
          sampleInstance: testInstances[0],
        },
      };
    } catch (error) {
      return {
        isValid: false,
        errors: [
          `Instance data test failed: ${error instanceof Error ? error.message : String(error)}`,
        ],
        warnings: [],
      };
    }
  }

  /**
   * Test 7: Streaming Performance
   */
  async testStreamingPerformance(): Promise<ValidationResult> {
    console.log('⚡ Testing streaming performance...');

    const testSizes = [100, 500, 1000];
    const results: any[] = [];
    const errors: string[] = [];
    const warnings: string[] = [];

    for (const maxTiles of testSizes) {
      const request: TileStreamingRequest = {
        camera_position: [0, 0, 0],
        view_radius: Math.sqrt(maxTiles), // Approximate radius for desired tile count
        max_tiles: maxTiles,
        lod_levels: [0],
        generation: 0,
      };

      try {
        const startTime = performance.now();
        const response = await this.tileService.streamTiles(request);
        const endTime = performance.now();

        const duration = endTime - startTime;
        const tilesPerMs = response.tiles.length / duration;

        results.push({
          requestedTiles: maxTiles,
          actualTiles: response.tiles.length,
          durationMs: duration,
          tilesPerMs: tilesPerMs,
        });

        // Performance warnings
        if (duration > 1000) {
          warnings.push(
            `Slow streaming for ${maxTiles} tiles: ${duration.toFixed(2)}ms`
          );
        }

        if (response.tiles.length < maxTiles * 0.5) {
          warnings.push(
            `Low tile yield: requested ${maxTiles}, got ${response.tiles.length}`
          );
        }
      } catch (error) {
        errors.push(
          `Performance test failed for ${maxTiles} tiles: ${error instanceof Error ? error.message : String(error)}`
        );
      }
    }

    return {
      isValid: errors.length === 0,
      errors,
      warnings,
      data: {
        performanceResults: results,
      },
    };
  }
}

/**
 * Utility function to run backend validation tests
 */
export async function validateBackendData(): Promise<ValidationResult> {
  const validator = new BackendDataValidator();
  return await validator.runAllTests();
}
