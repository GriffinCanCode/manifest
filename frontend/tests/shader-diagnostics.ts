#!/usr/bin/env node
/**
 * Shader System Diagnostics
 * Run comprehensive tests on the shader pipeline to identify issues
 */

import { existsSync, readFileSync } from 'fs';
import { dirname, join, resolve } from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const FRONTEND_DIR = resolve(__dirname, '..');
const SHADERS_DIR = join(FRONTEND_DIR, 'src', 'shaders');

interface DiagnosticResult {
  test: string;
  status: 'PASS' | 'FAIL' | 'WARN';
  message: string;
  details?: any;
}

const results: DiagnosticResult[] = [];

function addResult(
  test: string,
  status: 'PASS' | 'FAIL' | 'WARN',
  message: string,
  details?: any
) {
  results.push({ test, status, message, details });
  const icon = status === 'PASS' ? '✅' : status === 'FAIL' ? '❌' : '⚠️';
  console.log(`${icon} ${test}: ${message}`);
  if (details) {
    console.log(`   Details:`, details);
  }
}

function testFileExists(filePath: string, description: string): boolean {
  const fullPath = join(SHADERS_DIR, filePath);
  const exists = existsSync(fullPath);
  addResult(
    `File Existence: ${description}`,
    exists ? 'PASS' : 'FAIL',
    exists ? `Found at ${fullPath}` : `Missing: ${fullPath}`
  );
  return exists;
}

function testShaderCompilation(
  shaderPath: string,
  shaderName: string
): boolean {
  try {
    const fullPath = join(SHADERS_DIR, shaderPath);
    const content = readFileSync(fullPath, 'utf8');

    // Check for basic GLSL syntax
    const hasMain = content.includes('void main()');
    const hasVersion =
      content.includes('#ifdef GL_ES') || content.includes('#version');

    // Check for required hex-terrain features
    const hasInstanceAttributes =
      content.includes('instancePosition') &&
      content.includes('instanceColor') &&
      content.includes('instanceHeight');

    const hasIncludes = content.includes('#include');
    const includeCount = (content.match(/#include/g) || []).length;

    addResult(
      `Shader Syntax: ${shaderName}`,
      hasMain && hasVersion ? 'PASS' : 'FAIL',
      `Main: ${hasMain}, Version: ${hasVersion}`,
      {
        lines: content.split('\n').length,
        hasInstanceAttributes,
        hasIncludes,
        includeCount,
        size: content.length,
      }
    );

    return hasMain && hasVersion;
  } catch (error) {
    addResult(
      `Shader Read: ${shaderName}`,
      'FAIL',
      `Cannot read shader: ${error.message}`
    );
    return false;
  }
}

function testShaderModules(): void {
  console.log('\n🔍 TESTING SHADER MODULES...\n');

  const modules = [
    'modules/common.glsl',
    'modules/hex.glsl',
    'modules/noise.glsl',
    'modules/shadows.glsl',
  ];

  for (const module of modules) {
    const exists = testFileExists(module, `Shader Module: ${module}`);
    if (exists) {
      try {
        const fullPath = join(SHADERS_DIR, module);
        const content = readFileSync(fullPath, 'utf8');
        const functions = (
          content.match(/^\s*\w+\s+\w+\s*\([^)]*\)\s*{/gm) || []
        ).length;
        const constants = (content.match(/^\s*const\s+/gm) || []).length;

        addResult(
          `Module Content: ${module}`,
          content.length > 0 ? 'PASS' : 'WARN',
          `${content.length} chars, ${functions} functions, ${constants} constants`
        );
      } catch (error) {
        addResult(
          `Module Read: ${module}`,
          'FAIL',
          `Cannot read module: ${error.message}`
        );
      }
    }
  }
}

function testHexTerrainShader(): void {
  console.log('\n🎨 TESTING HEX-TERRAIN SHADER...\n');

  // Test vertex shader
  const vertexExists = testFileExists(
    'terrain/hex-terrain.vert',
    'Hex-Terrain Vertex Shader'
  );
  if (vertexExists) {
    testShaderCompilation('terrain/hex-terrain.vert', 'Vertex Shader');
  }

  // Test fragment shader
  const fragmentExists = testFileExists(
    'terrain/hex-terrain.frag',
    'Hex-Terrain Fragment Shader'
  );
  if (fragmentExists) {
    testShaderCompilation('terrain/hex-terrain.frag', 'Fragment Shader');
  }

  // Test shader definitions
  const defExists = testFileExists('definitions.ts', 'Shader Definitions');
  if (defExists) {
    try {
      const defPath = join(SHADERS_DIR, 'definitions.ts');
      const content = readFileSync(defPath, 'utf8');

      const hasHexTerrain = content.includes('HEX_TERRAIN_SHADER');
      const hasUniforms = content.includes('uniforms:');
      const uniformCount = (content.match(/u_\w+:/g) || []).length;

      addResult(
        'Shader Definitions',
        hasHexTerrain && hasUniforms ? 'PASS' : 'FAIL',
        `Hex-terrain defined: ${hasHexTerrain}, Uniforms: ${uniformCount}`,
        { hasHexTerrain, hasUniforms, uniformCount }
      );

      // Extract and validate uniforms
      const uniformMatches = content.match(/u_\w+:\s*{\s*value:/g) || [];
      addResult(
        'Uniform Definitions',
        uniformMatches.length > 10 ? 'PASS' : 'WARN',
        `Found ${uniformMatches.length} uniform definitions`,
        { uniforms: uniformMatches.map(u => u.split(':')[0]) }
      );
    } catch (error) {
      addResult(
        'Shader Definitions Read',
        'FAIL',
        `Cannot read definitions: ${error.message}`
      );
    }
  }
}

function testShaderManager(): void {
  console.log('\n⚙️ TESTING SHADER MANAGER...\n');

  const managerExists = testFileExists('manager.ts', 'Shader Manager');
  if (managerExists) {
    try {
      const managerPath = join(SHADERS_DIR, 'manager.ts');
      const content = readFileSync(managerPath, 'utf8');

      const hasCompileMethod =
        content.includes('compile(') || content.includes('compile =');
      const hasUpdateMethod =
        content.includes('updateUniforms') || content.includes('update');
      const hasShaderClass =
        content.includes('class') && content.includes('Manager');

      addResult(
        'Shader Manager Structure',
        hasCompileMethod && hasUpdateMethod ? 'PASS' : 'WARN',
        `Compile: ${hasCompileMethod}, Update: ${hasUpdateMethod}, Class: ${hasShaderClass}`
      );
    } catch (error) {
      addResult(
        'Shader Manager Read',
        'FAIL',
        `Cannot read manager: ${error.message}`
      );
    }
  }
}

function testShaderHooks(): void {
  console.log('\n🪝 TESTING SHADER HOOKS...\n');

  const hooksPath = 'components/rendering/hooks/shader-hooks.tsx';
  const fullHooksPath = join(FRONTEND_DIR, 'src', hooksPath);

  const exists = existsSync(fullHooksPath);
  addResult(
    'Shader Hooks File',
    exists ? 'PASS' : 'FAIL',
    exists ? `Found at ${fullHooksPath}` : `Missing: ${fullHooksPath}`
  );

  if (exists) {
    try {
      const content = readFileSync(fullHooksPath, 'utf8');

      const hasUseShader = content.includes('useShader');
      const hasUseShaders = content.includes('useShaders');
      const hasContext = content.includes('ShaderContext');

      addResult(
        'Shader Hooks Content',
        hasUseShader && hasUseShaders && hasContext ? 'PASS' : 'WARN',
        `useShader: ${hasUseShader}, useShaders: ${hasUseShaders}, Context: ${hasContext}`
      );
    } catch (error) {
      addResult(
        'Shader Hooks Read',
        'FAIL',
        `Cannot read hooks: ${error.message}`
      );
    }
  }
}

function analyzeShaderIncludes(): void {
  console.log('\n📋 ANALYZING SHADER INCLUDES...\n');

  try {
    const vertPath = join(SHADERS_DIR, 'terrain/hex-terrain.vert');
    const fragPath = join(SHADERS_DIR, 'terrain/hex-terrain.frag');

    for (const [shaderPath, shaderName] of [
      [vertPath, 'Vertex'],
      [fragPath, 'Fragment'],
    ]) {
      if (existsSync(shaderPath)) {
        const content = readFileSync(shaderPath, 'utf8');
        const includes = content.match(/#include\s+[^\n]+/g) || [];

        for (const include of includes) {
          const modulePath = include.replace('#include', '').trim();
          const fullModulePath = join(SHADERS_DIR, modulePath);
          const moduleExists = existsSync(fullModulePath);

          addResult(
            `${shaderName} Include: ${modulePath}`,
            moduleExists ? 'PASS' : 'FAIL',
            moduleExists ? 'Module found' : 'Module missing',
            { include, fullModulePath }
          );
        }
      }
    }
  } catch (error) {
    addResult('Include Analysis', 'FAIL', `Analysis failed: ${error.message}`);
  }
}

function generateReport(): void {
  console.log('\n📊 DIAGNOSTIC SUMMARY\n');

  const passed = results.filter(r => r.status === 'PASS').length;
  const failed = results.filter(r => r.status === 'FAIL').length;
  const warned = results.filter(r => r.status === 'WARN').length;

  console.log(`✅ PASSED: ${passed}`);
  console.log(`❌ FAILED: ${failed}`);
  console.log(`⚠️  WARNINGS: ${warned}`);
  console.log(`📊 TOTAL TESTS: ${results.length}`);

  if (failed > 0) {
    console.log('\n🚨 CRITICAL ISSUES:');
    results
      .filter(r => r.status === 'FAIL')
      .forEach(result => {
        console.log(`   • ${result.test}: ${result.message}`);
      });
  }

  if (warned > 0) {
    console.log('\n⚠️  WARNINGS:');
    results
      .filter(r => r.status === 'WARN')
      .forEach(result => {
        console.log(`   • ${result.test}: ${result.message}`);
      });
  }

  // Specific shader pipeline recommendations
  console.log('\n💡 SHADER PIPELINE ANALYSIS:');

  const hasVertexShader = results.some(
    r => r.test.includes('Vertex Shader') && r.status === 'PASS'
  );
  const hasFragmentShader = results.some(
    r => r.test.includes('Fragment Shader') && r.status === 'PASS'
  );
  const hasModules = results.some(
    r => r.test.includes('Module') && r.status === 'PASS'
  );
  const hasDefinitions = results.some(
    r => r.test.includes('Definitions') && r.status === 'PASS'
  );

  if (hasVertexShader && hasFragmentShader && hasModules && hasDefinitions) {
    console.log(
      '   ✅ Core shader files present - issue likely in runtime compilation or uniform binding'
    );
  } else {
    console.log(
      '   ❌ Missing critical shader components - file structure issue'
    );
  }
}

async function runDiagnostics(): Promise<void> {
  console.log('🔍 SHADER SYSTEM DIAGNOSTICS\n');
  console.log(`Frontend Directory: ${FRONTEND_DIR}`);
  console.log(`Shaders Directory: ${SHADERS_DIR}\n`);

  // Run all diagnostic tests
  testShaderModules();
  testHexTerrainShader();
  testShaderManager();
  testShaderHooks();
  analyzeShaderIncludes();

  // Generate final report
  generateReport();
}

// Run diagnostics
runDiagnostics().catch(console.error);
