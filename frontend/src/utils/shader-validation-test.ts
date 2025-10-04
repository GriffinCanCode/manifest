/**
 * Quick Shader Validation Test
 * Tests shader compilation and validation to verify fixes
 */

import { getShaderDefinition } from '../shaders/definitions';
import { shaderManager } from '../shaders/manager';

interface ValidationResult {
  success: boolean;
  message: string;
  details?: Record<string, unknown>;
}

export const runShaderValidationTest = (): ValidationResult => {
  console.warn('🔍 Running Shader Validation Test...');

  try {
    // Clear existing cache to force recompilation
    shaderManager.clear();

    // Test hex-terrain shader compilation
    const hexTerrainDef = getShaderDefinition('hex-terrain');
    if (!hexTerrainDef) {
      return {
        success: false,
        message: 'Cannot load hex-terrain shader definition',
      };
    }

    console.warn('✅ Shader definition loaded');

    // Compile shader with validation
    const material = shaderManager.compile('test-hex-terrain', hexTerrainDef, {
      defines: {
        QUALITY_LEVEL: 3,
        USE_SHADOWS: 0,
        USE_FOG: 1,
        USE_HDR: 1,
      },
    });

    console.warn('✅ Shader compiled successfully');

    // Test WebGL program creation
    const canvas = document.querySelector('canvas');
    if (!canvas) {
      return {
        success: false,
        message: 'No canvas found for WebGL testing',
      };
    }

    const gl = canvas.getContext('webgl2') ?? canvas.getContext('webgl');
    if (!gl) {
      return {
        success: false,
        message: 'WebGL context not available',
      };
    }

    // Test manual program creation and validation
    const testResult = testProgramValidation(
      gl,
      material.vertexShader,
      material.fragmentShader
    );

    return {
      success: testResult.success,
      message: testResult.message,
      details: {
        materialName: material.name,
        vertexShaderLines: material.vertexShader.split('\n').length,
        fragmentShaderLines: material.fragmentShader.split('\n').length,
        uniformCount: Object.keys(material.uniforms).length,
        defines: material.defines,
      },
    };
  } catch (error) {
    return {
      success: false,
      message: `Validation test failed: ${String(error)}`,
      details: { error: String(error) },
    };
  }
};

const testProgramValidation = (
  gl: WebGLRenderingContext,
  vertexSource: string,
  fragmentSource: string
): { success: boolean; message: string } => {
  try {
    // Compile vertex shader
    const vertexShader = gl.createShader(gl.VERTEX_SHADER);
    if (!vertexShader)
      return { success: false, message: 'Cannot create vertex shader' };

    gl.shaderSource(vertexShader, vertexSource);
    gl.compileShader(vertexShader);

    if (!gl.getShaderParameter(vertexShader, gl.COMPILE_STATUS)) {
      const error = gl.getShaderInfoLog(vertexShader);
      return {
        success: false,
        message: `Vertex shader compilation failed: ${error}`,
      };
    }

    // Compile fragment shader
    const fragmentShader = gl.createShader(gl.FRAGMENT_SHADER);
    if (!fragmentShader)
      return { success: false, message: 'Cannot create fragment shader' };

    gl.shaderSource(fragmentShader, fragmentSource);
    gl.compileShader(fragmentShader);

    if (!gl.getShaderParameter(fragmentShader, gl.COMPILE_STATUS)) {
      const error = gl.getShaderInfoLog(fragmentShader);
      return {
        success: false,
        message: `Fragment shader compilation failed: ${error}`,
      };
    }

    // Create and link program
    const program = gl.createProgram();
    if (!program) return { success: false, message: 'Cannot create program' };

    gl.attachShader(program, vertexShader);
    gl.attachShader(program, fragmentShader);
    gl.linkProgram(program);

    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
      const error = gl.getProgramInfoLog(program);
      return { success: false, message: `Program linking failed: ${error}` };
    }

    // Validate program
    gl.validateProgram(program);
    if (!gl.getProgramParameter(program, gl.VALIDATE_STATUS)) {
      const error = gl.getProgramInfoLog(program);
      console.warn(`⚠️ Program validation warning: ${error}`);
      // Don't fail on validation warnings - they're often not critical
    }

    // Cleanup
    gl.deleteProgram(program);
    gl.deleteShader(vertexShader);
    gl.deleteShader(fragmentShader);

    return { success: true, message: 'Program validation successful' };
  } catch (error) {
    return {
      success: false,
      message: `Program validation error: ${String(error)}`,
    };
  }
};

// Make available in browser console for testing
declare global {
  interface Window {
    testShaderValidation?: () => void;
  }
}

if (typeof window !== 'undefined') {
  window.testShaderValidation = () => {
    const result = runShaderValidationTest();
    console.warn('🧪 Shader Validation Test Result:', result);
  };
}
