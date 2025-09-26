/**
 * Diagnostic Test Runner
 * Runs all validation tests and provides comprehensive diagnosis of rendering issues
 */

import { validateBackendData } from './backend-data-validation';
import { validateFrontendRendering } from './frontend-rendering-validation';
import { validateIntegration } from './integration-validation';

interface DiagnosticReport {
  overall: {
    isValid: boolean;
    rootCause: string | null;
    confidence: number; // 0-100%
  };
  backend: {
    isValid: boolean;
    errors: string[];
    warnings: string[];
    data?: any;
  };
  frontend: {
    isValid: boolean;
    errors: string[];
    warnings: string[];
    data?: any;
  };
  integration: {
    isValid: boolean;
    errors: string[];
    warnings: string[];
    data?: any;
  };
  recommendations: string[];
  summary: string;
}

/**
 * Main diagnostic runner
 */
export class DiagnosticRunner {
  /**
   * Run complete diagnostic suite
   */
  async runDiagnostics(): Promise<DiagnosticReport> {
    console.log('🔍 STARTING COMPREHENSIVE DIAGNOSTIC ANALYSIS');
    console.log('='.repeat(60));

    const report: DiagnosticReport = {
      overall: {
        isValid: true,
        rootCause: null,
        confidence: 0,
      },
      backend: {
        isValid: true,
        errors: [],
        warnings: [],
      },
      frontend: {
        isValid: true,
        errors: [],
        warnings: [],
      },
      integration: {
        isValid: true,
        errors: [],
        warnings: [],
      },
      recommendations: [],
      summary: '',
    };

    try {
      // Run all test suites in parallel for efficiency
      console.log('🚀 Running all test suites...');

      const [backendResult, frontendResult, integrationResult] =
        await Promise.all([
          validateBackendData(),
          validateFrontendRendering(),
          validateIntegration(),
        ]);

      // Process results
      report.backend = {
        isValid: backendResult.isValid,
        errors: backendResult.errors,
        warnings: backendResult.warnings,
        data: backendResult.data,
      };

      report.frontend = {
        isValid: frontendResult.isValid,
        errors: frontendResult.errors,
        warnings: frontendResult.warnings,
        data: frontendResult.data,
      };

      report.integration = {
        isValid: integrationResult.isValid,
        errors: integrationResult.errors,
        warnings: integrationResult.warnings,
        data: integrationResult.data,
      };

      // Analyze root cause
      this.analyzeRootCause(report);

      // Generate recommendations
      this.generateRecommendations(report);

      // Generate summary
      this.generateSummary(report);

      console.log('\n📊 DIAGNOSTIC ANALYSIS COMPLETE');
      console.log('='.repeat(60));

      return report;
    } catch (error) {
      console.error('💥 Diagnostic runner failed:', error);

      report.overall.isValid = false;
      report.overall.rootCause = `Diagnostic system failure: ${error instanceof Error ? error.message : String(error)}`;
      report.overall.confidence = 0;
      report.summary =
        'Unable to complete diagnostic analysis due to system error';

      return report;
    }
  }

  /**
   * Analyze root cause based on test results
   */
  private analyzeRootCause(report: DiagnosticReport): void {
    const { backend, frontend, integration } = report;

    // Determine overall validity
    report.overall.isValid =
      backend.isValid && frontend.isValid && integration.isValid;

    // Root cause analysis
    if (!backend.isValid && !frontend.isValid) {
      report.overall.rootCause = 'BOTH_SYSTEMS_BROKEN';
      report.overall.confidence = 95;
    } else if (!backend.isValid) {
      report.overall.rootCause = 'BACKEND_DATA_ISSUE';
      report.overall.confidence = 90;
    } else if (!frontend.isValid) {
      report.overall.rootCause = 'FRONTEND_RENDERING_ISSUE';
      report.overall.confidence = 90;
    } else if (!integration.isValid) {
      report.overall.rootCause = 'INTEGRATION_ISSUE';
      report.overall.confidence = 85;
    } else if (backend.warnings.length > 5 || frontend.warnings.length > 5) {
      report.overall.rootCause = 'PERFORMANCE_DEGRADATION';
      report.overall.confidence = 70;
    } else if (integration.warnings.length > 3) {
      report.overall.rootCause = 'DATA_FLOW_INEFFICIENCY';
      report.overall.confidence = 60;
    } else {
      report.overall.rootCause = 'MINOR_CONFIGURATION_ISSUES';
      report.overall.confidence = 40;
    }

    // Specific issue analysis
    if (backend.errors.some(e => e.includes('Backend connection failed'))) {
      report.overall.rootCause = 'BACKEND_CONNECTION_FAILURE';
      report.overall.confidence = 100;
    }

    if (frontend.errors.some(e => e.includes('WebGL'))) {
      report.overall.rootCause = 'WEBGL_NOT_SUPPORTED';
      report.overall.confidence = 95;
    }

    if (backend.errors.some(e => e.includes('No tiles'))) {
      report.overall.rootCause = 'EMPTY_WORLD_DATA';
      report.overall.confidence = 90;
    }

    if (
      frontend.errors.some(e => e.includes('shader') || e.includes('Shader'))
    ) {
      report.overall.rootCause = 'SHADER_COMPILATION_FAILURE';
      report.overall.confidence = 95;
    }
  }

  /**
   * Generate specific recommendations based on findings
   */
  private generateRecommendations(report: DiagnosticReport): void {
    const { rootCause } = report.overall;
    const { backend, frontend, integration } = report;

    switch (rootCause) {
      case 'BACKEND_CONNECTION_FAILURE':
        report.recommendations.push(
          '1. Verify backend server is running (check start.sh or start-backend.sh)',
          '2. Check backend logs for startup errors',
          '3. Ensure Tauri IPC commands are properly registered',
          '4. Verify no firewall/port blocking issues'
        );
        break;

      case 'BACKEND_DATA_ISSUE':
        report.recommendations.push(
          '1. Check backend world generation - may be producing empty/invalid tiles',
          '2. Verify tile data structure matches frontend expectations',
          '3. Check backend tile streaming command implementation',
          '4. Validate hex coordinate calculation in backend'
        );
        break;

      case 'FRONTEND_RENDERING_ISSUE':
        report.recommendations.push(
          '1. Verify WebGL support in browser (try Chrome/Firefox)',
          '2. Check browser console for WebGL/Three.js errors',
          '3. Update graphics drivers if using hardware acceleration',
          '4. Test with different quality settings (low/medium/high)'
        );
        break;

      case 'WEBGL_NOT_SUPPORTED':
        report.recommendations.push(
          '1. Use a modern browser with WebGL support',
          '2. Enable hardware acceleration in browser settings',
          '3. Update graphics drivers',
          '4. Try different browser or device'
        );
        break;

      case 'SHADER_COMPILATION_FAILURE':
        report.recommendations.push(
          '1. Check shader files in src/shaders/ directory',
          '2. Verify shader manager is loading shaders correctly',
          '3. Test with basic shaders first, then complex ones',
          '4. Check for GLSL version compatibility issues'
        );
        break;

      case 'EMPTY_WORLD_DATA':
        report.recommendations.push(
          '1. Verify world generation is working in backend',
          '2. Check tile generation parameters and radius',
          '3. Ensure world state is properly initialized',
          '4. Check if mock data fallback is available'
        );
        break;

      case 'INTEGRATION_ISSUE':
        report.recommendations.push(
          '1. Verify tile streaming hook is properly connected',
          '2. Check data transformation between backend and frontend',
          '3. Ensure HexInstanceRenderer is receiving valid tile data',
          '4. Verify coordinate system consistency'
        );
        break;

      case 'PERFORMANCE_DEGRADATION':
        report.recommendations.push(
          '1. Reduce max_tiles limit in streaming requests',
          '2. Enable frustum culling and LOD systems',
          '3. Check for memory leaks in instanced rendering',
          '4. Optimize shader complexity'
        );
        break;

      default:
        report.recommendations.push(
          '1. Check browser developer console for errors',
          '2. Verify all dependencies are properly installed',
          '3. Try refreshing the page or clearing browser cache',
          '4. Check network connectivity between frontend and backend'
        );
    }

    // Add specific recommendations based on errors
    if (backend.errors.some(e => e.includes('Invalid response structure'))) {
      report.recommendations.push(
        'BACKEND: Update backend response format to match frontend expectations'
      );
    }

    if (frontend.errors.some(e => e.includes('Instance'))) {
      report.recommendations.push(
        'FRONTEND: Check InstancedBVHManager configuration and initialization'
      );
    }

    if (integration.errors.some(e => e.includes('coordinate'))) {
      report.recommendations.push(
        'INTEGRATION: Verify hex coordinate system consistency between backend and frontend'
      );
    }

    // Performance recommendations
    if (backend.warnings.length > 3) {
      report.recommendations.push(
        'PERFORMANCE: Consider optimizing backend tile generation or caching'
      );
    }

    if (frontend.warnings.length > 3) {
      report.recommendations.push(
        'PERFORMANCE: Consider reducing rendering quality or instance count'
      );
    }
  }

  /**
   * Generate diagnostic summary
   */
  private generateSummary(report: DiagnosticReport): void {
    const { overall, backend, frontend, integration } = report;

    if (overall.isValid) {
      report.summary =
        '✅ All systems appear to be functioning correctly. The rendering issue may be subtle or environmental.';
    } else {
      const parts: string[] = [];

      if (!backend.isValid) {
        parts.push(`Backend issues (${backend.errors.length} errors)`);
      }

      if (!frontend.isValid) {
        parts.push(`Frontend issues (${frontend.errors.length} errors)`);
      }

      if (!integration.isValid) {
        parts.push(`Integration issues (${integration.errors.length} errors)`);
      }

      const issueCount =
        backend.errors.length +
        frontend.errors.length +
        integration.errors.length;

      report.summary =
        `❌ ${overall.rootCause} detected with ${overall.confidence}% confidence. ` +
        `Found ${issueCount} critical errors across: ${parts.join(', ')}. ` +
        `Focus on ${overall.rootCause?.toLowerCase().replace(/_/g, ' ') || 'identified issues'} first.`;
    }
  }

  /**
   * Print detailed report to console
   */
  printReport(report: DiagnosticReport): void {
    console.log('\n📋 DIAGNOSTIC REPORT');
    console.log('='.repeat(60));

    // Overall status
    console.log(
      `\n🎯 OVERALL STATUS: ${report.overall.isValid ? '✅ HEALTHY' : '❌ ISSUES DETECTED'}`
    );
    console.log(
      `🔍 ROOT CAUSE: ${report.overall.rootCause || 'None detected'} (${report.overall.confidence}% confidence)`
    );
    console.log(`📝 SUMMARY: ${report.summary}`);

    // Backend results
    console.log(`\n🔧 BACKEND: ${report.backend.isValid ? '✅' : '❌'}`);
    if (report.backend.errors.length > 0) {
      console.log('  Errors:');
      report.backend.errors.forEach(error => console.log(`    • ${error}`));
    }
    if (report.backend.warnings.length > 0) {
      console.log('  Warnings:');
      report.backend.warnings
        .slice(0, 3)
        .forEach(warning => console.log(`    ⚠️  ${warning}`));
      if (report.backend.warnings.length > 3) {
        console.log(
          `    ... and ${report.backend.warnings.length - 3} more warnings`
        );
      }
    }

    // Frontend results
    console.log(`\n🎨 FRONTEND: ${report.frontend.isValid ? '✅' : '❌'}`);
    if (report.frontend.errors.length > 0) {
      console.log('  Errors:');
      report.frontend.errors.forEach(error => console.log(`    • ${error}`));
    }
    if (report.frontend.warnings.length > 0) {
      console.log('  Warnings:');
      report.frontend.warnings
        .slice(0, 3)
        .forEach(warning => console.log(`    ⚠️  ${warning}`));
      if (report.frontend.warnings.length > 3) {
        console.log(
          `    ... and ${report.frontend.warnings.length - 3} more warnings`
        );
      }
    }

    // Integration results
    console.log(
      `\n🔗 INTEGRATION: ${report.integration.isValid ? '✅' : '❌'}`
    );
    if (report.integration.errors.length > 0) {
      console.log('  Errors:');
      report.integration.errors.forEach(error => console.log(`    • ${error}`));
    }
    if (report.integration.warnings.length > 0) {
      console.log('  Warnings:');
      report.integration.warnings
        .slice(0, 3)
        .forEach(warning => console.log(`    ⚠️  ${warning}`));
      if (report.integration.warnings.length > 3) {
        console.log(
          `    ... and ${report.integration.warnings.length - 3} more warnings`
        );
      }
    }

    // Recommendations
    console.log('\n💡 RECOMMENDATIONS:');
    report.recommendations.forEach((rec, i) => console.log(`  ${rec}`));

    console.log('\n' + '='.repeat(60));
  }
}

/**
 * Run complete diagnostic analysis
 */
export async function runDiagnostics(): Promise<DiagnosticReport> {
  const runner = new DiagnosticRunner();
  const report = await runner.runDiagnostics();
  runner.printReport(report);
  return report;
}

// Export for use in browser console or direct execution
if (typeof window !== 'undefined') {
  (window as any).runDiagnostics = runDiagnostics;
}
