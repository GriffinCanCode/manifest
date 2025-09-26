/**
 * Simple Shader Diagnostic Utility
 * Quick and clean shader system testing
 */

import * as THREE from 'three';

import { getShaderDefinition } from '../shaders/definitions';
import { shaderManager } from '../shaders/manager';

interface DiagnosticResult {
  success: boolean;
  message: string;
  details: Record<string, unknown>;
}

export const runSimpleShaderDiagnostic = (): Promise<DiagnosticResult> => {
  console.warn('🔍 Running Simple Shader Diagnostic...');
  
  return Promise.resolve().then(() => {
    try {
    // Test 1: Shader definition loading
    const hexTerrainDef = getShaderDefinition('hex-terrain');
    if (!hexTerrainDef) {
      return {
        success: false,
        message: 'Cannot load hex-terrain shader definition',
        details: { step: 'definition_loading' },
      };
    }

    console.warn('✅ Shader definition loaded');

    // Test 2: Shader compilation
    let compiledMaterial: THREE.ShaderMaterial;
    try {
      compiledMaterial = shaderManager.compile('hex-terrain', hexTerrainDef, {
        defines: {
          QUALITY_LEVEL: 3,
          USE_SHADOWS: 0,
          USE_FOG: 1,
          USE_HDR: 1,
        },
      });
    } catch (error) {
      return {
        success: false,
        message: `Shader compilation failed: ${error instanceof Error ? error.message : 'Unknown error'}`,
        details: { step: 'compilation', error: String(error) },
      };
    }

    console.warn('✅ Shader compiled successfully');

    // Test 3: Uniform validation
    const { uniforms } = compiledMaterial;
    const criticalUniforms = [
      'u_time',
      'u_cameraPosition',
      'u_lightDirection',
      'u_hexSize',
      'u_heightScale',
      'u_exposure',
    ];

    const missingUniforms = criticalUniforms.filter(name => !uniforms?.[name]);

    if (missingUniforms.length > 0) {
      return {
        success: false,
        message: `Missing critical uniforms: ${missingUniforms.join(', ')}`,
        details: {
          step: 'uniform_validation',
          missing: missingUniforms,
          available: uniforms ? Object.keys(uniforms) : [],
        },
      };
    }

    console.warn('✅ All critical uniforms present');

    // Test 4: Basic instanced geometry setup
    try {
      const geometry = new THREE.CylinderGeometry(0.9, 0.9, 0.1, 6, 1, false);
      geometry.rotateX(-Math.PI / 2);

      // Add instanced attributes
      const maxInstances = 10;
      geometry.setAttribute(
        'instancePosition',
        new THREE.InstancedBufferAttribute(
          new Float32Array(maxInstances * 3),
          3
        )
      );
      geometry.setAttribute(
        'instanceColor',
        new THREE.InstancedBufferAttribute(
          new Float32Array(maxInstances * 3),
          3
        )
      );
      geometry.setAttribute(
        'instanceHeight',
        new THREE.InstancedBufferAttribute(new Float32Array(maxInstances), 1)
      );

      const instancedMesh = new THREE.InstancedMesh(
        geometry,
        compiledMaterial,
        maxInstances
      );

      console.warn('✅ Instanced mesh created successfully');

      return {
        success: true,
        message:
          'All shader diagnostic tests passed - shader system is working correctly',
        details: {
          materialType: compiledMaterial.type,
          uniformCount: uniforms ? Object.keys(uniforms).length : 0,
          instancedMeshCount: instancedMesh.count,
          geometryAttributes: Object.keys(geometry.attributes),
          defines: compiledMaterial.defines,
        },
      };
    } catch (error) {
      return {
        success: false,
        message: `Instanced mesh creation failed: ${error instanceof Error ? error.message : 'Unknown error'}`,
        details: { step: 'instanced_mesh_creation', error: String(error) },
      };
    }
  } catch (error) {
    return {
      success: false,
      message: `Diagnostic failed with error: ${error instanceof Error ? error.message : 'Unknown error'}`,
      details: { step: 'general_error', error: String(error) },
    };
  }
};

// Global function for console access
(window as Record<string, unknown>).runSimpleShaderDiagnostic =
  runSimpleShaderDiagnostic;

export default runSimpleShaderDiagnostic;
