/**
 * Save Thumbnail Service
 *
 * Generates and manages save file thumbnails for better visual browsing.
 * Integrates with existing save system and provides efficient caching.
 */

import { invoke } from '@tauri-apps/api/core';
import { toPng } from 'html-to-image';

export interface SaveThumbnail {
  /** Base64 encoded thumbnail image */
  readonly thumbnail: string;
  /** Thumbnail generation timestamp */
  readonly generatedAt: number;
  /** Canvas dimensions when captured */
  readonly dimensions: { width: number; height: number };
  /** Thumbnail size */
  readonly size: { width: number; height: number };
}

export interface SaveThumbnailOptions {
  /** Thumbnail width in pixels */
  width?: number;
  /** Thumbnail height in pixels */
  height?: number;
  /** JPEG quality (0-1) */
  quality?: number;
  /** Canvas selector to capture */
  canvasSelector?: string;
}

/**
 * Service for generating and managing save thumbnails
 */
export class SaveThumbnailService {
  private static readonly DEFAULT_THUMBNAIL_WIDTH = 320;
  private static readonly DEFAULT_THUMBNAIL_HEIGHT = 180;
  private static readonly DEFAULT_QUALITY = 0.8;
  private static readonly CANVAS_SELECTOR = 'canvas';

  private thumbnailCache = new Map<string, SaveThumbnail>();

  /**
   * Generate thumbnail from current game canvas
   */
  async generateThumbnail(
    saveName: string,
    options: SaveThumbnailOptions = {}
  ): Promise<SaveThumbnail> {
    const {
      width = SaveThumbnailService.DEFAULT_THUMBNAIL_WIDTH,
      height = SaveThumbnailService.DEFAULT_THUMBNAIL_HEIGHT,
      quality = SaveThumbnailService.DEFAULT_QUALITY,
      canvasSelector = SaveThumbnailService.CANVAS_SELECTOR,
    } = options;

    try {
      // Find the game canvas
      const canvas = document.querySelector(
        canvasSelector
      ) as HTMLCanvasElement;
      if (!canvas) {
        throw new Error(`Canvas not found with selector: ${canvasSelector}`);
      }

      // Get original dimensions
      const originalDimensions = {
        width: canvas.width,
        height: canvas.height,
      };

      // Generate thumbnail using html-to-image
      const dataUrl = await toPng(canvas, {
        width,
        height,
        quality,
        backgroundColor: '#000000',
        cacheBust: false,
        pixelRatio: 1,
      });

      const thumbnail: SaveThumbnail = {
        thumbnail: dataUrl,
        generatedAt: Date.now(),
        dimensions: originalDimensions,
        size: { width, height },
      };

      // Cache the thumbnail
      this.thumbnailCache.set(saveName, thumbnail);

      return thumbnail;
    } catch (error) {
      console.error('Failed to generate save thumbnail:', error);

      // Return placeholder thumbnail
      return this.generatePlaceholderThumbnail(options);
    }
  }

  /**
   * Generate placeholder thumbnail for failed captures
   */
  private generatePlaceholderThumbnail(
    options: SaveThumbnailOptions = {}
  ): SaveThumbnail {
    const {
      width = SaveThumbnailService.DEFAULT_THUMBNAIL_WIDTH,
      height = SaveThumbnailService.DEFAULT_THUMBNAIL_HEIGHT,
    } = options;

    // Create a simple placeholder using canvas
    const canvas = document.createElement('canvas');
    canvas.width = width;
    canvas.height = height;

    const ctx = canvas.getContext('2d');
    if (!ctx) {
      throw new Error('Failed to get 2D canvas context');
    }

    // Dark gradient background
    const gradient = ctx.createLinearGradient(0, 0, width, height);
    gradient.addColorStop(0, '#1a1a2e');
    gradient.addColorStop(1, '#16213e');

    ctx.fillStyle = gradient;
    ctx.fillRect(0, 0, width, height);

    // Add game logo or icon
    ctx.fillStyle = '#4a9eff';
    ctx.font = `${Math.floor(height / 8)}px Inter, sans-serif`;
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText('MANIFEST', width / 2, height / 2 - height / 8);

    // Add subtitle
    ctx.fillStyle = '#8a9ba8';
    ctx.font = `${Math.floor(height / 12)}px Inter, sans-serif`;
    ctx.fillText('Save Preview', width / 2, height / 2 + height / 8);

    const dataUrl = canvas.toDataURL('image/png', 0.8);

    return {
      thumbnail: dataUrl,
      generatedAt: Date.now(),
      dimensions: { width, height },
      size: { width, height },
    };
  }

  /**
   * Get cached thumbnail or generate new one
   */
  async getThumbnail(
    saveName: string,
    options: SaveThumbnailOptions = {}
  ): Promise<SaveThumbnail> {
    const cached = this.thumbnailCache.get(saveName);
    if (cached) {
      return cached;
    }

    return this.generateThumbnail(saveName, options);
  }

  /**
   * Save thumbnail as part of save metadata
   */
  async saveThumbnailWithSave(
    saveName: string,
    options: SaveThumbnailOptions = {}
  ): Promise<void> {
    try {
      // Generate thumbnail
      const thumbnail = await this.generateThumbnail(saveName, options);

      // Send thumbnail to backend for storage as metadata
      await invoke('save_thumbnail_metadata', {
        saveName,
        thumbnailData: thumbnail,
      });

      console.warn(`Thumbnail saved for: ${saveName}`);
    } catch (error) {
      console.error(`Failed to save thumbnail for ${saveName}:`, error);
      // Don't fail the save operation for thumbnail issues
    }
  }

  /**
   * Load thumbnail from save metadata
   */
  async loadThumbnail(saveName: string): Promise<SaveThumbnail | null> {
    try {
      const thumbnailData = await invoke<SaveThumbnail | null>(
        'load_thumbnail_metadata',
        { saveName }
      );

      if (thumbnailData) {
        this.thumbnailCache.set(saveName, thumbnailData);
        return thumbnailData;
      }

      return null;
    } catch (error) {
      console.warn(`Failed to load thumbnail for ${saveName}:`, error);
      return null;
    }
  }

  /**
   * Clear thumbnail cache
   */
  clearCache(): void {
    this.thumbnailCache.clear();
  }

  /**
   * Remove specific thumbnail from cache
   */
  removeThumbnail(saveName: string): void {
    this.thumbnailCache.delete(saveName);
  }

  /**
   * Get cache statistics
   */
  getCacheStats() {
    return {
      size: this.thumbnailCache.size,
      memoryUsage: this.estimateMemoryUsage(),
    };
  }

  /**
   * Estimate memory usage of cached thumbnails
   */
  private estimateMemoryUsage(): number {
    let totalSize = 0;
    for (const thumbnail of this.thumbnailCache.values()) {
      // Rough estimate: base64 string length / 1.33 for actual bytes
      totalSize += thumbnail.thumbnail.length * 0.75;
    }
    return Math.round(totalSize / 1024); // Return KB
  }
}

// Export singleton instance
export const saveThumbnailService = new SaveThumbnailService();
