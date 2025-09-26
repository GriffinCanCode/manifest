/**
 * Diagnostic Test Runner for Browser Console
 * Simplified version of the test suite that can be run directly in the browser
 */

import { invoke } from '@tauri-apps/api/core';
import type { TileStreamingRequest } from './tile-data-service';
import { TileDataService } from './tile-data-service';

interface TestResult {
  name: string;
  success: boolean;
  error?: string;
  data?: any;
}

/**
 * Simple diagnostic test runner that can be executed from browser console
 */
export class SimpleDiagnosticRunner {
  private results: TestResult[] = [];

  /**
   * Run all diagnostic tests
   */
  async runTests(): Promise<TestResult[]> {
    this.results = [];

    console.log('🔍 Running Simplified Diagnostic Tests...');
    console.log('='.repeat(50));

    await this.testBackendConnection();
    await this.testTileStreaming();
    await this.testWebGLSupport();
    await this.testDataStructure();

    this.printResults();
    return this.results;
  }

  /**
   * Test backend connection
   */
  private async testBackendConnection(): Promise<void> {
    console.log('📡 Testing backend connection...');

    try {
      const response = await invoke<string>('get_backend_status');

      this.results.push({
        name: 'Backend Connection',
        success: true,
        data: response,
      });
    } catch (error) {
      this.results.push({
        name: 'Backend Connection',
        success: false,
        error: `Failed to connect: ${error}`,
      });
    }
  }

  /**
   * Test tile streaming
   */
  private async testTileStreaming(): Promise<void> {
    console.log('🌍 Testing tile streaming...');

    try {
      const service = new TileDataService();
      const request: TileStreamingRequest = {
        camera_position: [0, 0, 0],
        view_radius: 20,
        max_tiles: 100,
        lod_levels: [0],
        generation: 0,
      };

      const response = await service.streamTiles(request);

      const isValid =
        response.tiles &&
        Array.isArray(response.tiles) &&
        response.tiles.length > 0;

      this.results.push({
        name: 'Tile Streaming',
        success: isValid,
        data: {
          tileCount: response.tiles?.length || 0,
          hasInstanceData: response.instance_data?.length > 0,
        },
        error: isValid
          ? undefined
          : 'No tiles received or invalid response structure',
      });
    } catch (error) {
      this.results.push({
        name: 'Tile Streaming',
        success: false,
        error: `Streaming failed: ${error}`,
      });
    }
  }

  /**
   * Test WebGL support
   */
  private async testWebGLSupport(): Promise<void> {
    console.log('🖥️ Testing WebGL support...');

    try {
      const canvas = document.createElement('canvas');
      const gl = canvas.getContext('webgl2') || canvas.getContext('webgl');

      if (!gl) {
        this.results.push({
          name: 'WebGL Support',
          success: false,
          error: 'WebGL not available in this browser',
        });
        return;
      }

      const version = gl.getParameter(gl.VERSION);
      const renderer = gl.getParameter(gl.RENDERER);

      this.results.push({
        name: 'WebGL Support',
        success: true,
        data: {
          version,
          renderer,
          maxTextureUnits: gl.getParameter(gl.MAX_TEXTURE_IMAGE_UNITS),
        },
      });
    } catch (error) {
      this.results.push({
        name: 'WebGL Support',
        success: false,
        error: `WebGL test failed: ${error}`,
      });
    }
  }

  /**
   * Test data structure consistency
   */
  private async testDataStructure(): Promise<void> {
    console.log('🔍 Testing data structure...');

    try {
      const service = new TileDataService();
      const response = await service.streamTiles({
        camera_position: [0, 0, 0],
        view_radius: 15,
        max_tiles: 50,
        lod_levels: [0],
        generation: 0,
      });

      if (!response.tiles || response.tiles.length === 0) {
        this.results.push({
          name: 'Data Structure',
          success: false,
          error: 'No tiles available for structure testing',
        });
        return;
      }

      const sampleTile = response.tiles[0];
      const structureValid =
        typeof sampleTile.id === 'number' &&
        sampleTile.hex &&
        typeof sampleTile.hex.q === 'number' &&
        typeof sampleTile.hex.r === 'number' &&
        typeof sampleTile.terrain === 'string' &&
        typeof sampleTile.elevation === 'number';

      this.results.push({
        name: 'Data Structure',
        success: structureValid,
        data: {
          sampleTile,
          totalTiles: response.tiles.length,
        },
        error: structureValid ? undefined : 'Tile data structure is invalid',
      });
    } catch (error) {
      this.results.push({
        name: 'Data Structure',
        success: false,
        error: `Structure test failed: ${error}`,
      });
    }
  }

  /**
   * Print results to console
   */
  private printResults(): void {
    console.log('\n📊 TEST RESULTS');
    console.log('='.repeat(50));

    let passCount = 0;
    let failCount = 0;

    this.results.forEach(result => {
      const status = result.success ? '✅ PASS' : '❌ FAIL';
      console.log(`${status} ${result.name}`);

      if (result.success) {
        passCount++;
        if (result.data) {
          console.log('  Data:', result.data);
        }
      } else {
        failCount++;
        if (result.error) {
          console.log(`  Error: ${result.error}`);
        }
      }
    });

    console.log(`\n📈 SUMMARY: ${passCount} passed, ${failCount} failed`);

    // Provide diagnosis
    if (failCount === 0) {
      console.log(
        '✅ All basic tests passed! The issue may be in the rendering pipeline.'
      );
      console.log('💡 Try checking:');
      console.log('  - Browser console for Three.js/rendering errors');
      console.log('  - HexInstanceRenderer initialization');
      console.log('  - Shader compilation issues');
    } else {
      console.log('❌ Issues detected:');

      const failedTests = this.results.filter(r => !r.success);
      if (failedTests.some(t => t.name === 'Backend Connection')) {
        console.log('🔧 PRIORITY: Fix backend connection first');
        console.log('  - Ensure backend server is running');
        console.log('  - Check Tauri IPC registration');
      }

      if (failedTests.some(t => t.name === 'Tile Streaming')) {
        console.log('🌍 ISSUE: Tile streaming not working');
        console.log('  - Check backend tile generation');
        console.log('  - Verify streaming command implementation');
      }

      if (failedTests.some(t => t.name === 'WebGL Support')) {
        console.log('🖥️ ISSUE: WebGL not supported');
        console.log('  - Try a different browser');
        console.log('  - Enable hardware acceleration');
      }
    }

    console.log('\n' + '='.repeat(50));
  }
}

/**
 * Global function for easy browser console access
 */
export function runQuickDiagnostics(): Promise<TestResult[]> {
  const runner = new SimpleDiagnosticRunner();
  return runner.runTests();
}

// Make available globally for browser console
if (typeof window !== 'undefined') {
  (window as any).runQuickDiagnostics = runQuickDiagnostics;
}
