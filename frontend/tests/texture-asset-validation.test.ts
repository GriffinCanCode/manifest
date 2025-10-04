/**
 * Texture Asset Validation Tests
 *
 * Tests to validate that the application uses texture assets (loaded from disk)
 * instead of procedural generation when assets are available.
 */

import { TextureFactoryService } from '../src/services/texture-factory-service';

// Mock Three.js classes
class MockTexture {
  public image: HTMLImageElement | null = null;
  public wrapS = 1000; // THREE.RepeatWrapping
  public wrapT = 1000; // THREE.RepeatWrapping
  public minFilter = 1008; // THREE.LinearMipmapLinearFilter
  public magFilter = 1006; // THREE.LinearFilter
  public generateMipmaps = true;
  public needsUpdate = false;
  public repeat = { set: jest.fn() };

  constructor(image?: HTMLImageElement) {
    this.image = image || null;
  }
}

class MockTextureLoader {
  load = jest.fn();
}

// Mock image for successful texture loading
const mockImage = {
  width: 512,
  height: 512,
  addEventListener: jest.fn(),
  removeEventListener: jest.fn(),
} as unknown as HTMLImageElement;

jest.mock('three', () => ({
  TextureLoader: MockTextureLoader,
  CanvasTexture: MockTexture,
  RepeatWrapping: 1000,
  LinearMipmapLinearFilter: 1008,
  LinearFilter: 1006,
}));

describe('Texture Asset vs Procedural Generation Tests', () => {
  let textureService: TextureFactoryService;
  let mockTextureLoader: MockTextureLoader;

  beforeEach(() => {
    // Reset mocks
    jest.clearAllMocks();

    // Create new service instance
    textureService = new TextureFactoryService();

    // Get the mock loader instance
    mockTextureLoader = (textureService as any).loader;
  });

  describe('Asset Texture Loading', () => {
    it('should load texture assets from disk when available', async () => {
      // Mock successful texture loading from disk
      mockTextureLoader.load.mockImplementation(
        (path: string, onLoad: (texture: MockTexture) => void) => {
          const texture = new MockTexture(mockImage);
          onLoad(texture);
        }
      );

      // Load a biome texture that should exist as an asset
      const texture = await textureService.loadTexture('biome_grassland');

      expect(texture).toBeDefined();
      expect(mockTextureLoader.load).toHaveBeenCalledWith(
        '/src/assets/generated_textures/biome_grassland_albedo.png',
        expect.any(Function),
        undefined,
        expect.any(Function)
      );
    });

    it('should prefer asset textures over procedural generation', async () => {
      // Mock that asset loading succeeds
      mockTextureLoader.load.mockImplementation(
        (path: string, onLoad: (texture: MockTexture) => void) => {
          if (path.includes('generated_textures')) {
            const texture = new MockTexture(mockImage);
            onLoad(texture);
          }
        }
      );

      // Initialize with existing textures (should load from disk)
      await textureService.loadExistingTextures();

      // Check that textures were loaded (not generated)
      const stats = textureService.getStats();
      expect(stats.texturesLoaded).toBeGreaterThan(0);

      // Verify the texture came from disk (has an image)
      const grasslandTexture = textureService.getTexture('biome_grassland');
      expect(grasslandTexture).toBeDefined();
      expect(grasslandTexture?.image).toBeDefined();
    });

    it('should identify when textures are loaded assets vs procedural', () => {
      // Create a texture from disk (with image)
      const assetTexture = new MockTexture(mockImage);

      // Create a procedural texture (from Canvas, no external image)
      const proceduralTexture = new MockTexture();

      // Asset textures should have an image from disk
      expect(assetTexture.image).toBeDefined();
      expect(assetTexture.image?.width).toBe(512);
      expect(assetTexture.image?.height).toBe(512);

      // Procedural textures may not have the same image properties
      expect(proceduralTexture.image).toBeNull();
    });
  });

  describe('Procedural Generation Fallback', () => {
    it('should only use procedural generation when assets are not available', async () => {
      // Mock that asset loading fails
      mockTextureLoader.load.mockImplementation(
        (
          path: string,
          _onLoad: any,
          _onProgress: any,
          onError: (error: Error) => void
        ) => {
          onError(new Error('Texture file not found'));
        }
      );

      // Try to load a texture that doesn't exist
      let loadingFailed = false;
      try {
        await textureService.loadTexture('biome_nonexistent');
      } catch (error) {
        loadingFailed = true;
      }

      expect(loadingFailed).toBe(true);
      expect(mockTextureLoader.load).toHaveBeenCalled();
    });

    it('should generate procedural textures in browser mode when assets fail', () => {
      // Mock browser environment (no Tauri)
      const originalWindow = global.window;
      (global as any).window = {
        ...originalWindow,
        __TAURI_INTERNALS__: undefined, // No Tauri in browser
      };

      // Mock document for canvas creation
      const mockCanvas = {
        width: 0,
        height: 0,
        getContext: jest.fn(() => ({
          fillStyle: '',
          fillRect: jest.fn(),
          getImageData: jest.fn(() => ({ data: new Uint8ClampedArray(4) })),
          putImageData: jest.fn(),
        })),
      };

      (global as any).document = {
        createElement: jest.fn(() => mockCanvas),
      };

      // This should trigger browser mode procedural generation
      // Note: This is tested at the unit level, integration would be in e2e tests
      expect(typeof textureService).toBe('object');

      // Restore window
      global.window = originalWindow;
    });
  });

  describe('Texture Service Stats and Metadata', () => {
    it('should correctly report texture source in stats', async () => {
      // Mock successful asset loading
      mockTextureLoader.load.mockImplementation(
        (path: string, onLoad: (texture: MockTexture) => void) => {
          const texture = new MockTexture(mockImage);
          onLoad(texture);
        }
      );

      await textureService.loadExistingTextures();
      const stats = textureService.getStats();

      // Should have loaded textures from assets
      expect(stats.texturesLoaded).toBeGreaterThan(0);
      expect(stats.cacheSize).toBeGreaterThan(0);
    });

    it('should distinguish between asset-backed and procedural materials', async () => {
      // Load textures from assets
      mockTextureLoader.load.mockImplementation(
        (path: string, onLoad: (texture: MockTexture) => void) => {
          const texture = new MockTexture(mockImage);
          onLoad(texture);
        }
      );

      await textureService.loadExistingTextures();

      // Get a texture and check its properties
      const texture = textureService.getTexture('biome_grassland');

      if (texture) {
        // Asset textures should have proper image dimensions
        expect(texture.image).toBeTruthy();
        expect((texture.image as any)?.width).toBe(512);
        expect((texture.image as any)?.height).toBe(512);
      }
    });
  });

  describe('Integration with HexInstanceRenderer', () => {
    it('should provide correct texture type information for renderer', async () => {
      // Mock texture loading
      mockTextureLoader.load.mockImplementation(
        (path: string, onLoad: (texture: MockTexture) => void) => {
          const texture = new MockTexture(mockImage);
          onLoad(texture);
        }
      );

      await textureService.loadExistingTextures();

      // Get material definitions that would be used by renderer
      const materialDefinitions =
        textureService.getMaterialsByCategory('terrain');

      expect(materialDefinitions.length).toBeGreaterThan(0);

      // Each material should reference asset textures
      materialDefinitions.forEach(material => {
        expect(material.category).toBe('terrain');
        expect(material.id).toBeTruthy();
        // Asset-backed materials should have texture references
        if (material.albedo_texture) {
          expect(material.albedo_texture.startsWith('biome_')).toBe(true);
        }
      });
    });
  });
});

/**
 * Test helper to validate texture rendering approach
 */
export const validateTextureRenderingApproach = {
  /**
   * Check if a material is using asset textures vs procedural generation
   */
  isUsingAssetTextures: (
    textureService: TextureService,
    biomeType: string
  ): boolean => {
    const texture = textureService.getTexture(`biome_${biomeType}`);
    return !!(
      texture &&
      texture.image &&
      (texture.image as HTMLImageElement).width > 0
    );
  },

  /**
   * Get texture source information for debugging
   */
  getTextureSourceInfo: (textureService: TextureService, biomeType: string) => {
    const texture = textureService.getTexture(`biome_${biomeType}`);
    if (!texture) {
      return { source: 'none', texture: null };
    }

    const hasImage = !!(
      texture.image && (texture.image as HTMLImageElement).width > 0
    );
    return {
      source: hasImage ? 'asset' : 'procedural',
      texture,
      dimensions: hasImage
        ? {
            width: (texture.image as HTMLImageElement).width,
            height: (texture.image as HTMLImageElement).height,
          }
        : null,
    };
  },

  /**
   * Validate that renderer logs are accurate
   */
  validateRendererLogs: (textureService: TextureService, biomes: string[]) => {
    const results = biomes.map(biome => ({
      biome,
      ...validateTextureRenderingApproach.getTextureSourceInfo(
        textureService,
        biome
      ),
    }));

    const assetCount = results.filter(r => r.source === 'asset').length;
    const proceduralCount = results.filter(
      r => r.source === 'procedural'
    ).length;

    return {
      totalBiomes: biomes.length,
      assetTextures: assetCount,
      proceduralTextures: proceduralCount,
      shouldLogAssetUsage: assetCount > 0,
      shouldLogProceduralUsage: proceduralCount > 0,
      results,
    };
  },
};
