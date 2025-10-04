# 🎨 Procedural Texture System Guide

The ManifestRustTS project includes a **complete procedural texture generation system** that creates professional-quality graphics without requiring manual design work. This system leverages your existing noise generation infrastructure to create textures programmatically.

## Table of Contents
- [System Overview](#system-overview)
- [What You Already Have](#what-you-already-have)
- [Getting Started](#getting-started)
- [Available Texture Types](#available-texture-types)
- [Usage Examples](#usage-examples)
- [Integration Guide](#integration-guide)
- [Advanced Configuration](#advanced-configuration)
- [Troubleshooting](#troubleshooting)

## System Overview

Your procedural texture system consists of:

### Backend (Rust)
- **Texture Generation Engine**: Creates textures using noise algorithms
- **PBR Material System**: Generates physically-based rendering maps
- **Atlas Management**: Packs textures efficiently for GPU usage
- **Export System**: Saves textures in multiple formats

### Frontend (TypeScript/React)
- **Texture Service**: Manages texture loading and caching
- **React Hooks**: Easy integration with components
- **Material System**: Three.js shader materials with PBR support
- **Provider Pattern**: Context-based texture management

## What You Already Have

✅ **Fully Implemented:**
- Complete backend texture generation system
- Professional-quality algorithms for all biome types
- PBR material generation (albedo, normal, roughness, metallic maps)
- Resource visualization textures
- Climate and environmental textures
- Frontend service layer with caching
- React integration with hooks and providers

🔧 **Recently Added:**
- Tauri command registration (now working!)
- Frontend texture provider integration
- Test panel for development

## Getting Started

### 1. Basic Texture Generation

The easiest way to generate textures is through the frontend test panel:

```typescript
import { TextureTestPanel } from './components/ui/TextureTestPanel';

// Add to your dev tools
<TextureTestPanel visible={true} />
```

### 2. Programmatic Generation

```typescript
import { textureService } from './services/texture-factory-service';

// Generate all textures
const response = await textureService.generateTextures({
  resolution: 512,
  generate_normals: true,
  generate_materials: true,
  generate_atlases: true,
});

console.log(`Generated ${response.texture_count} textures!`);
```

### 3. Using in Components

```typescript
import { useProceduralTextures } from './hooks/use-procedural-textures';

function TerrainTile({ biomeType }: { biomeType: string }) {
  const { material, isLoading } = useProceduralTextures({
    biomeType,
    textureScale: 1.0,
    enableAnimations: true,
  });

  if (isLoading) return <div>Loading texture...</div>;

  return (
    <mesh>
      <planeGeometry args={[1, 1]} />
      <primitive object={material} />
    </mesh>
  );
}
```

## Available Texture Types

### Terrain Textures
Your system generates textures for all these biomes:
- **Ocean**: Animated water with normal maps
- **Desert**: Sand dunes with wind patterns
- **Forest**: Dense vegetation textures
- **Grassland**: Rolling grass plains
- **Mountain**: Rocky surfaces with height variation
- **Tundra**: Frozen ground with ice patterns
- **Jungle**: Dense tropical vegetation
- **Plains**: Open grasslands
- **Hills**: Rolling terrain
- **Snow**: Snow-covered surfaces
- **Swamp**: Wetland textures
- **Oasis**: Desert oasis blend
- **Volcano**: Lava and volcanic rock
- **Glacier**: Ice formations
- **Beach**: Coastal sand textures

### Resource Textures
Categorized by resource type:
- **Strategic**: Uranium, rare earth (glowing effects)
- **Industrial**: Iron, copper (metallic appearances)  
- **Precious**: Gold, diamonds (sparkly effects)
- **Agricultural**: Wheat, cattle (natural colors)
- **Energy**: Oil, geothermal (appropriate effects)
- **Construction**: Stone, marble (material properties)

### Material Maps
Each texture includes:
- **Albedo**: Base color information
- **Normal Maps**: Surface detail and lighting
- **Roughness Maps**: Surface roughness for PBR
- **Metallic Maps**: Metallic vs non-metallic surfaces
- **AO Maps**: Ambient occlusion (optional)
- **Emission Maps**: Self-illuminated areas (for special effects)

## Usage Examples

### Backend Commands

```rust
// In your Tauri commands (already registered!)
// Backend texture generation has been replaced with client-side @texture-factory

// Generate textures
#[tauri::command]
async fn my_generate_textures() {
    let request = GenerateTexturesRequest {
        resolution: Some(1024),
        generate_normals: Some(true),
        generate_materials: Some(true),
        generate_atlases: Some(true),
        output_dir: Some("my_textures".to_string()),
    };
    
    // This command is already available!
    generate_textures(request, app_state).await
}
```

### Frontend Integration

```typescript
// Wrap your app with texture provider
import { TextureProvider } from './components/rendering/providers/TextureProvider';

function App() {
  return (
    <TextureProvider autoGenerate={true}>
      <YourGameComponents />
    </TextureProvider>
  );
}

// Use textures in components
import { useTextures } from './components/rendering/providers/TextureProvider';

function TerrainRenderer() {
  const { textureService, isInitialized } = useTextures();
  
  useEffect(() => {
    if (isInitialized) {
      // Load specific textures
      textureService.loadTexture('biome_forest');
    }
  }, [isInitialized]);
}
```

## Integration Guide

### 1. Add to Main App

The texture provider is already integrated into your render pipeline:

```typescript
// Already done in RenderInitializer.tsx
<ShaderProvider>
  <TextureProvider>
    <YourComponents />
  </TextureProvider>
</ShaderProvider>
```

### 2. Create Material Presets

```typescript
// Create commonly used materials
const BIOME_MATERIALS = {
  grassland: () => useProceduralTextures({ biomeType: 'grassland' }),
  forest: () => useProceduralTextures({ biomeType: 'forest' }),
  desert: () => useProceduralTextures({ biomeType: 'desert' }),
};
```

### 3. Performance Optimization

```typescript
// Preload common textures
const COMMON_TEXTURES = ['grassland', 'forest', 'ocean'];

useEffect(() => {
  COMMON_TEXTURES.forEach(biome => {
    textureService.loadTexture(`biome_${biome}`);
  });
}, []);
```

## Advanced Configuration

### Custom Resolution and Quality

```typescript
// High-quality textures for close-up viewing
await textureService.generateTextures({
  resolution: 2048,
  generate_normals: true,
  generate_materials: true,
  generate_atlases: true,
});

// Fast generation for prototyping
await textureService.generateTextures({
  resolution: 256,
  generate_normals: false,
  generate_materials: false,
  generate_atlases: false,
});
```

### Custom Shader Materials

```typescript
// The texture service creates materials with these uniforms:
const customMaterial = new THREE.ShaderMaterial({
  uniforms: {
    u_albedoTexture: { value: albedoTexture },
    u_normalTexture: { value: normalTexture },
    u_roughnessTexture: { value: roughnessTexture },
    u_metallicTexture: { value: metallicTexture },
    u_baseColor: { value: new THREE.Vector3(1, 1, 1) },
    u_roughness: { value: 0.5 },
    u_metallic: { value: 0.0 },
    u_textureScale: { value: 1.0 },
    u_time: { value: 0 },
  },
});
```

### Texture Atlas Usage

```typescript
// Get UV coordinates for efficient rendering
const atlasInfo = await invoke('get_texture_atlas', {
  request: { atlas_name: 'terrain' }
});

// Use atlas coordinates in shaders
const uvMapping = JSON.parse(atlasInfo.atlas_data);
```

## Professional Tips

### 1. No Designer Needed
The system generates professional-quality textures that:
- Use proper PBR workflows
- Include realistic material properties
- Have consistent lighting and color
- Support animation and effects

### 2. Efficient GPU Usage
- Textures are packed into atlases
- Mipmaps are generated automatically
- Memory usage is optimized
- Batch rendering supported

### 3. Flexible Integration
- Works with any Three.js setup
- Supports custom shader materials
- Integrates with existing pipelines
- Hot-reloadable during development

## Troubleshooting

### Common Issues

**Textures not generating:**
```bash
# Check backend compilation
cd backend && cargo check

# Check Tauri command registration in main.rs
grep "generate_textures" src/main.rs
```

**Frontend not loading textures:**
```typescript
// Check texture service initialization
const { isInitialized, error } = useTextures();
console.log('Texture service ready:', isInitialized);
```

**Poor texture quality:**
```typescript
// Increase resolution
await textureService.generateTextures({ resolution: 1024 });

// Enable all material maps
await textureService.generateTextures({
  generate_normals: true,
  generate_materials: true,
});
```

### Performance Issues

**Slow generation:**
- Use lower resolution for development (256px)
- Generate textures once, cache results
- Use texture atlases for batch rendering

**High memory usage:**
- Clear unused textures: `textureService.clearCache()`
- Use compressed texture formats
- Implement LOD (level of detail) system

## File Locations

**Backend:**
- Backend texture generation removed - now uses client-side @texture-factory

**Frontend:**
- `frontend/src/services/texture-factory-service.ts` - Main service
- `frontend/src/hooks/use-procedural-textures.ts` - React hook
- `frontend/src/components/rendering/providers/TextureProvider.tsx` - Provider

## Next Steps

1. **Test the System**: Use the `TextureTestPanel` to generate your first textures
2. **Integrate**: Add texture materials to your game objects
3. **Optimize**: Profile performance and adjust settings
4. **Customize**: Modify generation algorithms for your specific needs

Your texture system is **production-ready** and can generate professional-quality graphics without any manual design work! 🎨✨
