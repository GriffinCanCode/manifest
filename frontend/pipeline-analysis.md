# 🔍 Hex Rendering Pipeline Analysis

## Overview

The hex rendering system has **multiple overlapping layers** that are potentially conflicting with each other.

---

## 🏗️ **PIPELINE LAYERS**

### **Layer 1: HexInstanceRenderer.tsx (Three.js Component)**

```typescript
// CURRENT STATE: Uses identity matrix (no transformation)
matrix.identity(); // No transformation - let shader do it
instancedMesh.setMatrixAt(i, matrix);

// Sets instance attributes:
instancePosition.array[i * 3] = x; // World position
instanceColor.array[i * 3] = color.r; // Terrain colors
instanceHeight.array[i] = elevation; // Height data
instanceBiome.array[i] = terrainType; // Biome data
```

**Responsibilities:**

- ✅ Instance attribute setup
- ❌ No positioning (delegated to shader)
- ✅ Data streaming to GPU

---

### **Layer 2: Vertex Shader (GLSL)**

```glsl
// CURRENT STATE: Shader handles ALL positioning
vec3 transformed = position * u_hexSize;  // Scale by hex size
transformed += instancePosition;          // Add world position
vec3 worldPos = (modelMatrix * vec4(transformed, 1.0)).xyz;
```

**Responsibilities:**

- ✅ Positioning logic
- ✅ Terrain displacement
- ✅ LOD calculations
- ✅ Normal calculations

**Expected Uniforms:**

```glsl
uniform float u_hexSize;      // 1.0 (from shader definition)
uniform float u_hexSpacing;   // 1.15 (from shader definition)
uniform float u_heightScale;  // 10.0 (from shader definition)
```

---

### **Layer 3: ShaderProvider.tsx (Uniform Management)**

```typescript
// UPDATES EVERY FRAME:
updateShaderUniforms(
  hexTerrainMaterial.uniforms,
  timeRef.current, // u_time
  cameraPosition, // u_cameraPosition
  qualityLevel // u_qualityLevel
);

// Static values from definitions:
u_hexSize: 1.0;
u_hexSpacing: 1.15;
u_heightScale: 10.0;
```

**Responsibilities:**

- ✅ Frame-by-frame uniform updates
- ✅ Camera tracking
- ✅ Time progression
- ✅ Quality-based adjustments

---

## ⚡ **CONFLICTS IDENTIFIED**

### **1. POSITIONING RESPONSIBILITY OVERLAP**

- **HexInstanceRenderer**: `matrix.identity()` (no positioning)
- **Vertex Shader**: `transformed += instancePosition` (full positioning)
- **RESULT**: Shader expects to handle positioning but Three.js instancing usually does it

### **2. SCALING CONFLICTS**

- **Vertex Shader**: `position * u_hexSize` (expects u_hexSize = 1.0)
- **Three.js Geometry**: `CylinderGeometry(0.9, 0.9, 0.1, 6)` (radius=0.9)
- **RESULT**: Double-scaling or incorrect scale values

### **3. COORDINATE SYSTEM MISMATCH**

- **HexUtils.hexToPixel()**: Returns world coordinates for instancePosition
- **Vertex Shader**: `instancePosition.xz / u_hexSpacing` for hex coordinates
- **RESULT**: Coordinate system assumptions may be wrong

### **4. MATRIX TRANSFORMATION ORDER**

```glsl
// Shader does:
vec3 transformed = position * u_hexSize + instancePosition;
vec4 worldPosition = modelMatrix * vec4(transformed, 1.0);

// But Three.js InstancedMesh expects:
// Instance matrices already applied in modelMatrix
```

---

## 🐛 **ROOT CAUSE HYPOTHESIS**

The **vertex shader was designed for a different instancing approach** than what HexInstanceRenderer is using:

### **Shader Expects:**

- Instance attributes contain LOCAL hex coordinates
- Shader calculates world positions from hex coordinates
- u_hexSize/u_hexSpacing control positioning

### **HexInstanceRenderer Provides:**

- Instance attributes contain WORLD coordinates
- Identity matrices (no Three.js positioning)
- Shader gets world coordinates but treats them as local coordinates

---

## 🔧 **SOLUTIONS**

### **Option A: Fix Coordinate System (Recommended)**

Make instancePosition contain hex coordinates, not world coordinates:

```typescript
// Instead of world coordinates:
const [x, z] = HexUtils.hexToPixel(tile.hex);
posArray[i * 3] = x; // ❌ World coordinate
posArray[i * 3 + 2] = z; // ❌ World coordinate

// Use hex coordinates:
posArray[i * 3] = tile.hex.q * u_hexSpacing; // ✅ Hex coordinate
posArray[i * 3 + 2] = tile.hex.r * u_hexSpacing; // ✅ Hex coordinate
```

### **Option B: Simplify Vertex Shader**

Remove shader positioning, let Three.js handle it:

```glsl
// Instead of shader positioning:
vec3 transformed = position * u_hexSize + instancePosition; // ❌

// Use Three.js positioning:
vec3 transformed = position; // ✅ Let modelMatrix handle positioning
```

### **Option C: Hybrid Approach**

Use Three.js for basic positioning, shader for displacement only:

```typescript
// Three.js handles basic positioning:
matrix.makeTranslation(x, 0, z);
matrix.scale(new Vector3(u_hexSize, 1, u_hexSize));

// Shader handles height displacement only:
transformed = position + vec3(0, instanceHeight * u_heightScale, 0);
```

---

## 🎯 **RECOMMENDED FIX**

**Option A** is most consistent with the existing shader design:

1. **Change instancePosition to hex coordinates**
2. **Verify u_hexSize/u_hexSpacing values match HexUtils**
3. **Keep identity matrices in Three.js**
4. **Let shader handle all positioning logic**

This preserves the sophisticated terrain displacement and LOD system while fixing the coordinate system mismatch.

---

## 📊 **TESTING STRATEGY**

1. **Enable u_showBiomes debug mode** - should show colored tiles if working
2. **Log instancePosition values** - verify coordinate system
3. **Check u_hexSize uniform** - ensure it matches expected scale
4. **Test with wireframe mode** - verify geometry positioning
5. **Monitor WebGL console** - catch any compilation errors

The rendering should immediately improve once the coordinate system is aligned!
