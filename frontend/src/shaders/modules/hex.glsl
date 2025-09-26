/**
 * Hexagonal coordinate system utilities
 * Integrates with Rust backend hex mathematics (Zig SIMD optimized)
 */

#include common.glsl

// Hex grid constants
#define HEX_SIZE 1.0
#define HEX_SPACING 1.15
#define HEX_HEIGHT 0.866025404  // sqrt(3) / 2

// Axial to cube coordinate conversion
vec3 axialToCube(vec2 axial) {
  float x = axial.x;
  float z = axial.y;
  float y = -x - z;
  return vec3(x, y, z);
}

// Cube to axial coordinate conversion
vec2 cubeToAxial(vec3 cube) {
  return vec2(cube.x, cube.z);
}

// Hex to pixel conversion (flat-top orientation)
vec2 hexToPixel(vec2 hex, float size) {
  float x = size * (1.5 * hex.x);
  float y = size * (HEX_HEIGHT * (hex.x + 2.0 * hex.y));
  return vec2(x, y);
}

// Pixel to hex conversion (flat-top orientation)
vec2 pixelToHex(vec2 pixel, float size) {
  float x = (2.0 / 3.0) * pixel.x / size;
  float y = (-1.0 / 3.0 * pixel.x + HEX_HEIGHT / 3.0 * pixel.y) / size;
  return vec2(x, y);
}

// Round fractional cube coordinates to nearest hex
vec3 cubeRound(vec3 cube) {
  float rx = round(cube.x);
  float ry = round(cube.y);
  float rz = round(cube.z);

  float dx = abs(rx - cube.x);
  float dy = abs(ry - cube.y);
  float dz = abs(rz - cube.z);

  if (dx > dy && dx > dz) {
    rx = -ry - rz;
  } else if (dy > dz) {
    ry = -rx - rz;
  } else {
    rz = -rx - ry;
  }

  return vec3(rx, ry, rz);
}

// Round fractional axial coordinates
vec2 axialRound(vec2 axial) {
  return cubeToAxial(cubeRound(axialToCube(axial)));
}

// Get hex neighbors (6 directions)
vec2 getHexNeighbor(vec2 hex, int direction) {
  vec2 neighbors[6];
  neighbors[0] = vec2(1, 0);   // East
  neighbors[1] = vec2(1, -1);  // Northeast
  neighbors[2] = vec2(0, -1);  // Northwest
  neighbors[3] = vec2(-1, 0);  // West
  neighbors[4] = vec2(-1, 1);  // Southwest
  neighbors[5] = vec2(0, 1);   // Southeast
  
  return hex + neighbors[direction];
}

// Hex distance calculation
float hexDistance(vec2 a, vec2 b) {
  vec3 cubeA = axialToCube(a);
  vec3 cubeB = axialToCube(b);
  vec3 diff = abs(cubeA - cubeB);
  return max(diff.x, max(diff.y, diff.z));
}

// Generate hex ring at distance
vec2 hexRing(vec2 center, int radius, int index) {
  if (radius == 0) return center;
  
  // Start at direction 4 (southwest) and walk
  vec2 hex = center + vec2(-radius, radius);
  
  for (int i = 0; i < 6; i++) {
    int sideLength = radius;
    if (index < sideLength) {
      break;
    }
    index -= sideLength;
    hex = getHexNeighbor(hex, i);
  }
  
  // Walk along current side
  for (int i = 0; i < index; i++) {
    hex = getHexNeighbor(hex, (index / radius) % 6);
  }
  
  return hex;
}

// Check if point is inside hex
bool isInsideHex(vec2 point, vec2 center, float size) {
  point = point - center;
  point = abs(point);
  
  float dx = point.x / size;
  float dy = point.y / size;
  
  return (dx <= 1.0) && (dy <= HEX_HEIGHT) && 
         (dx * HEX_HEIGHT + dy * 0.5 <= HEX_HEIGHT);
}

// Generate hex edge mask
float hexEdge(vec2 uv, float thickness) {
  uv = abs(uv);
  
  // Distance to hex edge
  float dx = uv.x;
  float dy = uv.y * 0.866025404; // cos(30°)
  
  float d1 = max(dx * 0.866025404 + dy * 0.5, dy) - 0.866025404;
  float d2 = dx - 1.0;
  
  float d = max(d1, d2);
  return 1.0 - smoothstep(-thickness, thickness, d);
}

// Generate hex grid pattern
float hexGrid(vec2 uv, float scale) {
  vec2 hexCoord = pixelToHex(uv * scale, 1.0);
  vec2 hexCenter = hexToPixel(axialRound(hexCoord), 1.0);
  
  vec2 localUV = (uv * scale - hexCenter) / scale;
  return hexEdge(localUV, 0.05);
}

// Hex tile ID from coordinates
float hexTileID(vec2 hex) {
  // Simple hash for unique tile IDs
  return fract(sin(dot(hex, vec2(127.1, 311.7))) * 43758.5453123);
}

// Optimized hex pattern for instanced rendering
vec4 hexPattern(vec2 uv, float scale, float time) {
  vec2 hexCoord = pixelToHex(uv * scale, 1.0);
  vec2 roundedHex = axialRound(hexCoord);
  vec2 hexCenter = hexToPixel(roundedHex, 1.0);
  
  vec2 localUV = (uv * scale - hexCenter) / scale;
  float tileID = hexTileID(roundedHex);
  
  // Animated hex pattern
  float pulse = sin(time * 2.0 + tileID * TAU) * 0.5 + 0.5;
  float edge = hexEdge(localUV, 0.02);
  float fill = 1.0 - isInsideHex(localUV, vec2(0.0), 0.8) ? 0.0 : 1.0;
  
  return vec4(localUV, edge * pulse, fill);
}

#pragma glslify: export(axialToCube)
#pragma glslify: export(cubeToAxial)
#pragma glslify: export(hexToPixel)
#pragma glslify: export(pixelToHex)
#pragma glslify: export(cubeRound)
#pragma glslify: export(axialRound)
#pragma glslify: export(getHexNeighbor)
#pragma glslify: export(hexDistance)
#pragma glslify: export(hexRing)
#pragma glslify: export(isInsideHex)
#pragma glslify: export(hexEdge)
#pragma glslify: export(hexGrid)
#pragma glslify: export(hexTileID)
#pragma glslify: export(hexPattern)
