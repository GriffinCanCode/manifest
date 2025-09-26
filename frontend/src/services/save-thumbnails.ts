/**
 * Save Thumbnail Service
 *
 * Generates and manages save file thumbnails for better visual browsing.
 * Integrates with existing save system and provides efficient caching.
 */

import { invoke } from '@tauri-apps/api/core';
import { toPng } from 'html-to-image';

import { StorageLogger } from './logger';

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

    const timer = StorageLogger.startTimer('thumbnail-generation');

    try {
      StorageLogger.debug('Starting thumbnail generation', {
        saveName,
        options: { width, height, quality, canvasSelector },
      });

      // Find the game canvas
      const canvas = document.querySelector(
        canvasSelector
      ) as HTMLCanvasElement;
      if (!canvas) {
        const error = new Error(
          `Canvas not found with selector: ${canvasSelector}`
        );
        StorageLogger.error(
          'Canvas not found for thumbnail generation',
          error,
          {
            saveName,
            canvasSelector,
          }
        );
        throw error;
      }

      // Get original dimensions
      const originalDimensions = {
        width: canvas.width,
        height: canvas.height,
      };

      StorageLogger.debug('Canvas found for thumbnail', {
        saveName,
        originalDimensions,
        canvasSelector,
      });

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

      timer.end('Thumbnail generation completed successfully', {
        saveName,
        thumbnailSize: dataUrl.length,
        cacheSize: this.thumbnailCache.size,
      });

      StorageLogger.info('Thumbnail generated successfully', {
        saveName,
        originalDimensions,
        thumbnailSize: { width, height },
        dataUrlSize: dataUrl.length,
        quality,
      });

      return thumbnail;
    } catch (error) {
      timer.end('Thumbnail generation failed');
      StorageLogger.error('Failed to generate save thumbnail', error as Error, {
        saveName,
        options: { width, height, quality, canvasSelector },
      });

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

    try {
      StorageLogger.debug('Generating placeholder thumbnail', {
        width,
        height,
      });

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

      StorageLogger.info('Placeholder thumbnail generated', {
        width,
        height,
        dataUrlSize: dataUrl.length,
      });

      return {
        thumbnail: dataUrl,
        generatedAt: Date.now(),
        dimensions: { width, height },
        size: { width, height },
      };
    } catch (error) {
      StorageLogger.error(
        'Failed to generate placeholder thumbnail',
        error as Error,
        {
          width,
          height,
        }
      );

      // Return minimal fallback
      return {
        thumbnail: '',
        generatedAt: Date.now(),
        dimensions: { width, height },
        size: { width, height },
      };
    }
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
      StorageLogger.debug('Returning cached thumbnail', {
        saveName,
        cacheAge: Date.now() - cached.generatedAt,
      });
      return cached;
    }

    StorageLogger.debug('Cache miss, generating new thumbnail', { saveName });
    return this.generateThumbnail(saveName, options);
  }

  /**
   * Save thumbnail as part of save metadata
   */
  async saveThumbnailWithSave(
    saveName: string,
    options: SaveThumbnailOptions = {}
  ): Promise<void> {
    const timer = StorageLogger.startTimer('save-thumbnail-with-save');

    try {
      StorageLogger.info('Starting thumbnail save operation', {
        saveName,
        options,
      });

      // Generate thumbnail
      const thumbnail = await this.generateThumbnail(saveName, options);

      // Send thumbnail to backend for storage as metadata
      await invoke('save_thumbnail_metadata', {
        saveName,
        thumbnailData: thumbnail,
      });

      timer.end('Thumbnail saved successfully with save', { saveName });
      StorageLogger.info('Thumbnail saved successfully', {
        saveName,
        thumbnailDataSize: thumbnail.thumbnail.length,
        dimensions: thumbnail.dimensions,
      });
    } catch (error) {
      timer.end('Thumbnail save failed');
      StorageLogger.warn(
        'Failed to save thumbnail (save operation continues)',
        {
          saveName,
          error: error instanceof Error ? error.message : String(error),
        }
      );
      // Don't fail the save operation for thumbnail issues
    }
  }

  /**
   * Load thumbnail from save metadata
   */
  async loadThumbnail(saveName: string): Promise<SaveThumbnail | null> {
    const timer = StorageLogger.startTimer('load-thumbnail');

    try {
      StorageLogger.debug('Loading thumbnail from save metadata', { saveName });

      const thumbnailData = await invoke<SaveThumbnail | null>(
        'load_thumbnail_metadata',
        { saveName }
      );

      if (thumbnailData) {
        this.thumbnailCache.set(saveName, thumbnailData);

        timer.end('Thumbnail loaded successfully', { saveName });
        StorageLogger.info('Thumbnail loaded from save metadata', {
          saveName,
          dimensions: thumbnailData.dimensions,
          age: Date.now() - thumbnailData.generatedAt,
        });

        return thumbnailData;
      }

      timer.end('No thumbnail found in save metadata', { saveName });
      StorageLogger.debug('No thumbnail found in save metadata', { saveName });
      return null;
    } catch (error) {
      timer.end('Thumbnail load failed');
      StorageLogger.warn('Failed to load thumbnail from save metadata', {
        saveName,
        error: error instanceof Error ? error.message : String(error),
      });
      return null;
    }
  }

  /**
   * Clear thumbnail cache
   */
  clearCache(): void {
    const cacheSize = this.thumbnailCache.size;
    const memoryUsage = this.estimateMemoryUsage();

    this.thumbnailCache.clear();

    StorageLogger.info('Thumbnail cache cleared', {
      clearedItems: cacheSize,
      freedMemoryKB: memoryUsage,
    });
  }

  /**
   * Remove specific thumbnail from cache
   */
  removeThumbnail(saveName: string): void {
    const wasDeleted = this.thumbnailCache.delete(saveName);

    if (wasDeleted) {
      StorageLogger.debug('Thumbnail removed from cache', { saveName });
    } else {
      StorageLogger.debug('Thumbnail not found in cache for removal', {
        saveName,
      });
    }
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
