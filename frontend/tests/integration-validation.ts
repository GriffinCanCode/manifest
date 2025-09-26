/**
 * Integration Validation Tests
 * Tests to verify data flow from backend to frontend and complete rendering pipeline
 */

import { validateBackendData } from './backend-data-validation';
import { validateFrontendRendering } from './frontend-rendering-validation';

interface IntegrationValidationResult {
  isValid: boolean;
  errors: string[];
  warnings: string[];
  data?: any;
  backendResult?: any;
  frontendResult?: any;
}

/**
 * Test suite for integration validation between backend and frontend
 */
export class IntegrationValidator {
  /**
   * Run all integration validation tests
   */
  async runAllTests(): Promise<IntegrationValidationResult> {
    const results: IntegrationValidationResult = {
      isValid: true,
      errors: [],
      warnings: [],
    };

    console.log('🔗 Starting Integration Validation Tests...');

    const tests = [
      this.testBackendToFrontendFlow,
      this.testTileStreamingIntegration,
      this.testRenderingPipelineIntegration,
      this.testDataConsistency,
      this.testPerformanceIntegration,
      this.testErrorHandling,
    ];

    for (const test of tests) {
      try {
        const testResult = await test.call(this);
        if (!testResult.isValid) {
          results.isValid = false;
        }
        results.errors.push(...testResult.errors);
        results.warnings.push(...testResult.warnings);

        if (testResult.data) {
          results.data = { ...results.data, [test.name]: testResult.data };
        }
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
   * Test 1: Backend to Frontend Data Flow
   */
  async testBackendToFrontendFlow(): Promise<IntegrationValidationResult> {
    console.log('🔄 Testing backend to frontend data flow...');

    const errors: string[] = [];
    const warnings: string[] = [];
    let backendResult: any = null;
    let frontendResult: any = null;

    try {
      // Test backend independently
      console.log('  → Testing backend data provision...');
      backendResult = await validateBackendData();

      if (!backendResult.isValid) {
        errors.push('Backend validation failed - data flow blocked at source');
        errors.push(...backendResult.errors);
      }

      // Test frontend independently
      console.log('  → Testing frontend rendering capabilities...');
      frontendResult = await validateFrontendRendering();

      if (!frontendResult.isValid) {
        errors.push('Frontend validation failed - rendering pipeline broken');
        errors.push(...frontendResult.errors);
      }

      // Check compatibility between backend and frontend
      if (backendResult.isValid && frontendResult.isValid) {
        console.log('  → Both backend and frontend tests passed');

        if (backendResult.data && frontendResult.data) {
          // Check if frontend can handle backend data load
          const backendTileCount = backendResult.data.tileCount || 0;
          const frontendInstanceCapacity =
            frontendResult.data.instanceCount || 0;

          if (backendTileCount > frontendInstanceCapacity * 2) {
            warnings.push(
              `Backend provides ${backendTileCount} tiles but frontend tested with only ${frontendInstanceCapacity} instances`
            );
          }
        }
      }

      return {
        isValid: errors.length === 0,
        errors,
        warnings,
        backendResult,
        frontendResult,
        data: {
          backendValid: backendResult.isValid,
          frontendValid: frontendResult.isValid,
          flowBlocked: !backendResult.isValid || !frontendResult.isValid,
        },
      };
    } catch (error) {
      return {
        isValid: false,
        errors: [
          `Backend to frontend flow test failed: ${error instanceof Error ? error.message : String(error)}`,
        ],
        warnings: [],
        backendResult,
        frontendResult,
      };
    }
  }

  /**
   * Test 2: Tile Streaming Hook Integration
   */
  async testTileStreamingIntegration(): Promise<IntegrationValidationResult> {
    console.log('📡 Testing tile streaming hook integration...');

    const errors: string[] = [];
    const warnings: string[] = [];

    try {
      // Since we can't use React hooks outside of a component,
      // we'll test the underlying service directly and validate the hook logic

      const { TileDataService } = await import(
        '../src/utils/tile-data-service'
      );

      const service = new TileDataService();
      const startTime = performance.now();

      const response = await service.streamTiles({
        camera_position: [0, 5, 10],
        view_radius: 25,
        max_tiles: 500,
        lod_levels: [0],
        generation: 0,
      });

      const endTime = performance.now();
      const streamingTime = endTime - startTime;

      // Validate response matches expected hook behavior
      if (!response.tiles || response.tiles.length === 0) {
        errors.push(
          'Tile streaming service returned no tiles - hook would fail'
        );
      }

      if (
        !response.instance_data ||
        response.instance_data.length !== response.tiles.length
      ) {
        errors.push(
          'Instance data count does not match tile count - rendering would fail'
        );
      }

      // Check if tiles have proper structure for rendering
      if (response.tiles.length > 0) {
        const sampleTile = response.tiles[0];

        if (typeof sampleTile.id !== 'number') {
          errors.push(
            'Tile ID not a number - rendering identification will fail'
          );
        }

        if (
          !sampleTile.hex ||
          typeof sampleTile.hex.q !== 'number' ||
          typeof sampleTile.hex.r !== 'number'
        ) {
          errors.push('Tile hex coordinates invalid - positioning will fail');
        }

        if (typeof sampleTile.elevation !== 'number') {
          errors.push('Tile elevation invalid - height calculation will fail');
        }
      }

      // Performance check
      if (streamingTime > 500) {
        warnings.push(
          `Slow streaming performance: ${streamingTime.toFixed(2)}ms - may cause rendering delays`
        );
      }

      return {
        isValid: errors.length === 0,
        errors,
        warnings,
        data: {
          tilesReceived: response.tiles.length,
          instanceDataCount: response.instance_data.length,
          streamingTimeMs: streamingTime,
          sampleTile: response.tiles[0] || null,
        },
      };
    } catch (error) {
      return {
        isValid: false,
        errors: [
          `Tile streaming integration test failed: ${error instanceof Error ? error.message : String(error)}`,
        ],
        warnings: [],
      };
    }
  }

  /**
   * Test 3: Rendering Pipeline Integration
   */
  async testRenderingPipelineIntegration(): Promise<IntegrationValidationResult> {
    console.log('🎨 Testing rendering pipeline integration...');

    const errors: string[] = [];
    const warnings: string[] = [];

    try {
      // Test the key integration points between data and rendering
      const { TileDataService } = await import(
        '../src/utils/tile-data-service'
      );
      const { HexUtils } = await import('../src/utils/game-types');

      const service = new TileDataService();
      const response = await service.streamTiles({
        camera_position: [0, 0, 0],
        view_radius: 15,
        max_tiles: 100,
        lod_levels: [0],
        generation: 0,
      });

      if (response.tiles.length === 0) {
        errors.push('No tiles to test rendering pipeline integration');
        return { isValid: false, errors, warnings };
      }

      // Test hex-to-pixel conversion integration
      let conversionErrors = 0;
      const positionTest = response.tiles.slice(0, 10);

      for (const tile of positionTest) {
        try {
          const [x, z] = HexUtils.hexToPixel(tile.hex);

          if (!isFinite(x) || !isFinite(z)) {
            conversionErrors++;
          }

          // Check if pixel coordinates are reasonable
          if (Math.abs(x) > 10000 || Math.abs(z) > 10000) {
            warnings.push(
              `Extreme pixel coordinates: (${x}, ${z}) for tile ${tile.id}`
            );
          }
        } catch (error) {
          conversionErrors++;
        }
      }

      if (conversionErrors > 0) {
        errors.push(
          `${conversionErrors} tiles failed hex-to-pixel conversion - positioning will fail`
        );
      }

      // Test terrain color mapping
      const { TerrainType } = await import('../src/utils/game-types');
      const terrainColors: Record<string, string> = {
        ocean: '#1e40af',
        grassland: '#22c55e',
        plains: '#84cc16',
        desert: '#eab308',
        tundra: '#64748b',
        snow: '#f1f5f9',
        forest: '#166534',
        jungle: '#14532d',
        hills: '#a3a3a3',
        mountain: '#525252',
      };

      let colorMappingErrors = 0;
      for (const tile of response.tiles.slice(0, 20)) {
        if (!terrainColors[tile.terrain]) {
          colorMappingErrors++;
        }
      }

      if (colorMappingErrors > 0) {
        errors.push(
          `${colorMappingErrors} tiles have unmappable terrain types - coloring will fail`
        );
      }

      // Test instancing data format
      if (response.instance_data.length > 0) {
        const instanceData = response.instance_data[0];

        if (
          !Array.isArray(instanceData.position) ||
          instanceData.position.length !== 3
        ) {
          errors.push(
            'Instance position data format invalid - instanced rendering will fail'
          );
        }

        if (typeof instanceData.height !== 'number') {
          errors.push('Instance height data format invalid - sizing will fail');
        }

        if (
          !Array.isArray(instanceData.color) ||
          instanceData.color.length !== 3
        ) {
          errors.push(
            'Instance color data format invalid - coloring will fail'
          );
        }
      }

      return {
        isValid: errors.length === 0,
        errors,
        warnings,
        data: {
          tilesTestedForPositioning: positionTest.length,
          conversionErrors,
          colorMappingErrors,
          instanceDataValid: response.instance_data.length > 0,
        },
      };
    } catch (error) {
      return {
        isValid: false,
        errors: [
          `Rendering pipeline integration test failed: ${error instanceof Error ? error.message : String(error)}`,
        ],
        warnings: [],
      };
    }
  }

  /**
   * Test 4: Data Consistency Between Systems
   */
  async testDataConsistency(): Promise<IntegrationValidationResult> {
    console.log('🔍 Testing data consistency between systems...');

    const errors: string[] = [];
    const warnings: string[] = [];

    try {
      const { TileDataService } = await import(
        '../src/utils/tile-data-service'
      );

      const service = new TileDataService();

      // Request same data twice to check consistency
      const request = {
        camera_position: [0, 0, 0] as const,
        view_radius: 20,
        max_tiles: 200,
        lod_levels: [0] as const,
        generation: 0,
      };

      const [response1, response2] = await Promise.all([
        service.streamTiles(request),
        service.streamTiles(request),
      ]);

      // Check if responses are consistent
      if (response1.tiles.length !== response2.tiles.length) {
        errors.push(
          `Inconsistent tile count: first request ${response1.tiles.length}, second request ${response2.tiles.length}`
        );
      }

      // Check tile ID consistency (assuming deterministic generation)
      const ids1 = new Set(response1.tiles.map(t => t.id));
      const ids2 = new Set(response2.tiles.map(t => t.id));

      const commonIds = new Set([...ids1].filter(id => ids2.has(id)));
      const uniqueToFirst = ids1.size - commonIds.size;
      const uniqueToSecond = ids2.size - commonIds.size;

      if (
        uniqueToFirst > response1.tiles.length * 0.1 ||
        uniqueToSecond > response2.tiles.length * 0.1
      ) {
        warnings.push(
          `High tile ID variation between requests: ${uniqueToFirst}/${uniqueToSecond} unique tiles`
        );
      }

      // Check coordinate consistency for common tiles
      const tile1Map = new Map(response1.tiles.map(t => [t.id, t]));
      const tile2Map = new Map(response2.tiles.map(t => [t.id, t]));

      let coordinateInconsistencies = 0;
      let elevationInconsistencies = 0;

      for (const id of commonIds) {
        const t1 = tile1Map.get(id);
        const t2 = tile2Map.get(id);

        if (t1 && t2) {
          if (t1.hex.q !== t2.hex.q || t1.hex.r !== t2.hex.r) {
            coordinateInconsistencies++;
          }

          if (Math.abs(t1.elevation - t2.elevation) > 0.001) {
            elevationInconsistencies++;
          }
        }
      }

      if (coordinateInconsistencies > 0) {
        errors.push(
          `${coordinateInconsistencies} tiles have inconsistent coordinates`
        );
      }

      if (elevationInconsistencies > 0) {
        errors.push(
          `${elevationInconsistencies} tiles have inconsistent elevations`
        );
      }

      return {
        isValid: errors.length === 0,
        errors,
        warnings,
        data: {
          request1TileCount: response1.tiles.length,
          request2TileCount: response2.tiles.length,
          commonTileCount: commonIds.size,
          coordinateInconsistencies,
          elevationInconsistencies,
        },
      };
    } catch (error) {
      return {
        isValid: false,
        errors: [
          `Data consistency test failed: ${error instanceof Error ? error.message : String(error)}`,
        ],
        warnings: [],
      };
    }
  }

  /**
   * Test 5: Performance Integration
   */
  async testPerformanceIntegration(): Promise<IntegrationValidationResult> {
    console.log('⚡ Testing performance integration...');

    const errors: string[] = [];
    const warnings: string[] = [];

    try {
      const { TileDataService } = await import(
        '../src/utils/tile-data-service'
      );

      const service = new TileDataService();

      // Test different load scenarios
      const scenarios = [
        { name: 'light', tiles: 100, radius: 10 },
        { name: 'medium', tiles: 500, radius: 20 },
        { name: 'heavy', tiles: 1000, radius: 30 },
      ];

      const results: Array<{
        scenario: string;
        requestedTiles: number;
        actualTiles: number;
        durationMs: number;
        tilesPerMs: number;
      }> = [];

      for (const scenario of scenarios) {
        const startTime = performance.now();

        const response = await service.streamTiles({
          camera_position: [0, 0, 0],
          view_radius: scenario.radius,
          max_tiles: scenario.tiles,
          lod_levels: [0],
          generation: 0,
        });

        const endTime = performance.now();
        const duration = endTime - startTime;

        const result = {
          scenario: scenario.name,
          requestedTiles: scenario.tiles,
          actualTiles: response.tiles.length,
          durationMs: duration,
          tilesPerMs: response.tiles.length / duration,
        };

        results.push(result);

        // Performance thresholds
        if (duration > 1000) {
          warnings.push(
            `Slow performance for ${scenario.name} load: ${duration.toFixed(2)}ms`
          );
        }

        if (response.tiles.length < scenario.tiles * 0.3) {
          warnings.push(
            `Low tile yield for ${scenario.name} load: got ${response.tiles.length}/${scenario.tiles}`
          );
        }
      }

      // Check for performance degradation
      const lightResult = results.find(r => r.scenario === 'light');
      const heavyResult = results.find(r => r.scenario === 'heavy');

      if (lightResult && heavyResult) {
        const performanceRatio =
          heavyResult.tilesPerMs / lightResult.tilesPerMs;

        if (performanceRatio < 0.5) {
          warnings.push(
            `Significant performance degradation under heavy load: ${(performanceRatio * 100).toFixed(1)}% efficiency`
          );
        }
      }

      return {
        isValid: errors.length === 0,
        errors,
        warnings,
        data: {
          scenarioResults: results,
          performanceDegradation:
            lightResult && heavyResult
              ? heavyResult.tilesPerMs / lightResult.tilesPerMs
              : null,
        },
      };
    } catch (error) {
      return {
        isValid: false,
        errors: [
          `Performance integration test failed: ${error instanceof Error ? error.message : String(error)}`,
        ],
        warnings: [],
      };
    }
  }

  /**
   * Test 6: Error Handling Integration
   */
  async testErrorHandling(): Promise<IntegrationValidationResult> {
    console.log('🚨 Testing error handling integration...');

    const errors: string[] = [];
    const warnings: string[] = [];

    try {
      const { TileDataService } = await import(
        '../src/utils/tile-data-service'
      );

      const service = new TileDataService();

      // Test invalid requests
      const invalidRequests = [
        {
          name: 'invalid_camera_position',
          request: {
            camera_position: [NaN, NaN, NaN] as const,
            view_radius: 20,
            max_tiles: 100,
            lod_levels: [0] as const,
            generation: 0,
          },
        },
        {
          name: 'negative_radius',
          request: {
            camera_position: [0, 0, 0] as const,
            view_radius: -10,
            max_tiles: 100,
            lod_levels: [0] as const,
            generation: 0,
          },
        },
        {
          name: 'zero_tiles',
          request: {
            camera_position: [0, 0, 0] as const,
            view_radius: 20,
            max_tiles: 0,
            lod_levels: [0] as const,
            generation: 0,
          },
        },
      ];

      let handledErrors = 0;
      let unhandledErrors = 0;

      for (const { name, request } of invalidRequests) {
        try {
          const response = await service.streamTiles(request);

          // If we get here, the invalid request was processed
          if (!response.tiles || response.tiles.length === 0) {
            handledErrors++; // Graceful handling with empty result
          } else {
            warnings.push(
              `Invalid request '${name}' returned data when it should have failed`
            );
          }
        } catch (error) {
          handledErrors++; // Proper error thrown
        }
      }

      // Test frontend resilience with empty data
      try {
        const { HexUtils } = await import('../src/utils/game-types');

        // Test with malformed tile data
        const malformedTile = {
          id: 1,
          hex: null as any,
          terrain: 'invalid' as any,
          elevation: NaN,
          worldX: undefined as any,
          worldZ: undefined as any,
        };

        try {
          const [x, z] = HexUtils.hexToPixel(malformedTile.hex);
          if (isFinite(x) && isFinite(z)) {
            warnings.push(
              'HexUtils did not handle null hex coordinates properly'
            );
          }
        } catch {
          handledErrors++; // Proper error handling
        }
      } catch (error) {
        unhandledErrors++;
      }

      if (unhandledErrors > 0) {
        errors.push(
          `${unhandledErrors} error handling tests failed with unhandled exceptions`
        );
      }

      return {
        isValid: errors.length === 0,
        errors,
        warnings,
        data: {
          invalidRequestsTested: invalidRequests.length,
          handledErrors,
          unhandledErrors,
          errorHandlingRatio: handledErrors / (handledErrors + unhandledErrors),
        },
      };
    } catch (error) {
      return {
        isValid: false,
        errors: [
          `Error handling integration test failed: ${error instanceof Error ? error.message : String(error)}`,
        ],
        warnings: [],
      };
    }
  }
}

/**
 * Utility function to run all integration validation tests
 */
export async function validateIntegration(): Promise<IntegrationValidationResult> {
  const validator = new IntegrationValidator();
  return await validator.runAllTests();
}
