/**
 * Camera Position Diagnostic
 * Analyzes camera position relative to tile positions and suggests adjustments
 */

export function runCameraPositionDiagnostic() {
  console.group('📷 CAMERA POSITION DIAGNOSTIC');

  const win = window as any;
  const camera = win.__camera;
  const mesh = win.__instancedMesh;

  if (!camera) {
    console.warn('❌ Camera not accessible');
    console.groupEnd();
    return;
  }

  if (!mesh) {
    console.warn('❌ Instanced mesh not accessible');
    console.groupEnd();
    return;
  }

  console.log('🎯 CAMERA STATUS:');
  console.log(
    `   Position: (${camera.position.x.toFixed(2)}, ${camera.position.y.toFixed(2)}, ${camera.position.z.toFixed(2)})`
  );
  console.log(
    `   Rotation: (${camera.rotation.x.toFixed(2)}, ${camera.rotation.y.toFixed(2)}, ${camera.rotation.z.toFixed(2)})`
  );

  if (camera.isPerspectiveCamera) {
    console.log(
      `   FOV: ${camera.fov}°, Near: ${camera.near}, Far: ${camera.far}`
    );
  }

  console.log('\n🎮 TILE MESH STATUS:');
  console.log(`   Visible: ${mesh.visible}`);
  console.log(`   Count: ${mesh.count} instances`);

  // Get direction camera is looking using camera's built-in method
  const direction = camera.getWorldDirection(camera.position.clone());
  console.log(
    `   Looking: (${direction.x.toFixed(2)}, ${direction.y.toFixed(2)}, ${direction.z.toFixed(2)})`
  );

  // Sample some tile positions - use Three.js from imports instead of window
  console.log('\n📍 SAMPLE TILE POSITIONS:');
  const samplePositions = [];

  // Check if we can access Three.js through the mesh object
  if (!mesh.matrix || !mesh.getMatrixAt) {
    console.warn(
      '❌ Cannot access instance matrices - mesh not properly initialized'
    );
    console.groupEnd();
    return;
  }

  for (let i = 0; i < Math.min(10, mesh.count); i++) {
    // Create temporary objects for matrix decomposition
    const tempMatrix = mesh.matrix.clone();
    mesh.getMatrixAt(i, tempMatrix);

    // Extract position from matrix manually since we can't rely on window.THREE
    const { elements } = tempMatrix;
    const pos = {
      x: elements[12],
      y: elements[13],
      z: elements[14],
    };
    samplePositions.push(pos);
    console.log(
      `   Tile ${i}: (${pos.x.toFixed(2)}, ${pos.y.toFixed(2)}, ${pos.z.toFixed(2)})`
    );
  }

  if (samplePositions.length === 0) {
    console.warn('❌ No tile positions found');
    console.groupEnd();
    return;
  }

  // Calculate average tile position
  const avgPos = samplePositions.reduce(
    (acc, pos) => {
      acc.x += pos.x / samplePositions.length;
      acc.y += pos.y / samplePositions.length;
      acc.z += pos.z / samplePositions.length;
      return acc;
    },
    { x: 0, y: 0, z: 0 }
  );

  console.log(
    `\n🎯 AVERAGE TILE POSITION: (${avgPos.x.toFixed(2)}, ${avgPos.y.toFixed(2)}, ${avgPos.z.toFixed(2)})`
  );

  // Calculate distance from camera to average tile position
  const distance = Math.sqrt(
    Math.pow(camera.position.x - avgPos.x, 2) +
      Math.pow(camera.position.y - avgPos.y, 2) +
      Math.pow(camera.position.z - avgPos.z, 2)
  );

  console.log(`📏 DISTANCE TO TILES: ${distance.toFixed(2)} units`);

  // Check if camera is looking toward tiles
  const toTiles = {
    x: avgPos.x - camera.position.x,
    y: avgPos.y - camera.position.y,
    z: avgPos.z - camera.position.z,
  };

  const toTilesLength = Math.hypot(toTiles.x, toTiles.y, toTiles.z);
  const toTilesNorm = {
    x: toTiles.x / toTilesLength,
    y: toTiles.y / toTilesLength,
    z: toTiles.z / toTilesLength,
  };

  const dotProduct =
    direction.x * toTilesNorm.x +
    direction.y * toTilesNorm.y +
    direction.z * toTilesNorm.z;
  const angle =
    Math.acos(Math.max(-1, Math.min(1, dotProduct))) * (180 / Math.PI);

  console.log(
    `🔄 CAMERA ANGLE TO TILES: ${angle.toFixed(1)}° (0° = looking directly at)`
  );

  // Provide recommendations
  console.log('\n💡 RECOMMENDATIONS:');

  if (angle > 90) {
    console.log('❌ Camera is looking AWAY from tiles!');
    console.log('   • Rotate camera to face tiles');
  } else if (angle > 45) {
    console.log('⚠️ Camera is looking partially toward tiles');
    console.log('   • Adjust camera rotation for better view');
  } else {
    console.log('✅ Camera is looking toward tiles');
  }

  if (distance > 200) {
    console.log('⚠️ Camera is very far from tiles');
    console.log('   • Move camera closer or increase render distance');
  } else if (distance < 10) {
    console.log('⚠️ Camera is very close to tiles');
    console.log('   • Move camera back for better overview');
  } else {
    console.log('✅ Camera distance looks reasonable');
  }

  // Suggest optimal camera position
  const optimalDistance = 100;
  const optimalHeight = 50;
  const suggestedPos = {
    x: avgPos.x,
    y: avgPos.y + optimalHeight,
    z: avgPos.z + optimalDistance,
  };

  console.log('\n🎯 SUGGESTED CAMERA POSITION:');
  console.log(
    `   Position: (${suggestedPos.x.toFixed(2)}, ${suggestedPos.y.toFixed(2)}, ${suggestedPos.z.toFixed(2)})`
  );
  console.log('   Looking at: (0, 0, 0)');

  console.groupEnd();
}

// Quick camera positioning function
export function setCameraPosition(
  x: number,
  y: number,
  z: number,
  lookX = 0,
  lookY = 0,
  lookZ = 0
) {
  const win = window as any;
  const camera = win.__camera;

  if (!camera) {
    console.error('❌ Camera not accessible');
    return;
  }

  camera.position.set(x, y, z);
  camera.lookAt(lookX, lookY, lookZ);
  console.log(
    `🎥 Camera moved to (${x}, ${y}, ${z}) looking at (${lookX}, ${lookY}, ${lookZ})`
  );
}

// Quick tile scale adjustment function
export function setTileScale(scale: number) {
  const win = window as any;
  const mesh = win.__instancedMesh;

  if (!mesh) {
    console.error('❌ Instanced mesh not accessible');
    return;
  }

  // Scale all instances
  mesh.scale.set(scale, scale, scale);
  console.log(`🏔️ Tiles scaled to ${scale}x size`);
}

// Debug mesh visibility and material
export function debugMeshVisibility() {
  const win = window as any;
  const mesh = win.__instancedMesh;

  if (!mesh) {
    console.error('❌ Instanced mesh not accessible');
    return;
  }

  console.log('🔍 MESH DEBUG:');
  console.log(`   Visible: ${mesh.visible}`);
  console.log(`   Count: ${mesh.count}`);
  console.log(`   Material: ${mesh.material?.type}`);
  console.log(`   Material color: ${mesh.material?.color?.getHexString()}`);
  console.log(`   Scale: (${mesh.scale.x}, ${mesh.scale.y}, ${mesh.scale.z})`);
  console.log(
    `   Position: (${mesh.position.x}, ${mesh.position.y}, ${mesh.position.z})`
  );

  // Force bright red material for testing
  if (win.THREE) {
    mesh.material = new win.THREE.MeshBasicMaterial({
      color: 0xff0000, // Bright red
      wireframe: false,
    });
    console.log('🔴 Forced bright red material for testing');
  }
}

// Expose globally
if (typeof window !== 'undefined') {
  (window as any).runCameraPositionDiagnostic = runCameraPositionDiagnostic;
  (window as any).setCameraPosition = setCameraPosition;
  (window as any).setTileScale = setTileScale;
  (window as any).debugMeshVisibility = debugMeshVisibility;
}
