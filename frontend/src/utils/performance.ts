/**
 * Performance monitoring system with Stats.js integration
 * Provides real-time FPS, frame time, and memory monitoring
 */

import React from 'react';
import Stats from 'stats.js';

import type { PerformanceMetrics } from '../stores/render-store';

export interface PerformanceConfig {
  enableFPS: boolean;
  enableMemory: boolean;
  enableCustom: boolean;
  position: 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right';
  opacity: number;
  scale: number;
}

export interface FrameData {
  fps: number;
  frameTime: number;
  timestamp: number;
}

/**
 * Advanced performance monitor with multiple metric tracking
 */
class PerformanceMonitor {
  private stats: Stats | null = null;
  private customPanel: Stats.Panel | null = null;
  private isActive = false;
  private frameHistory: FrameData[] = [];
  private maxHistorySize = 60; // Store 60 frames for analysis

  private onMetricsUpdate: ((metrics: PerformanceMetrics) => void) | null =
    null;
  private rafId: number | null = null;

  private lastTime = 0;
  private frameCount = 0;
  private fpsUpdateInterval = 100; // Update FPS every 100ms
  private lastFpsUpdate = 0;

  constructor(
    private config: PerformanceConfig = {
      enableFPS: true,
      enableMemory: true,
      enableCustom: true,
      position: 'top-left',
      opacity: 0.9,
      scale: 1,
    }
  ) {}

  /**
   * Initialize performance monitoring
   */
  init(container: HTMLElement = document.body): void {
    if (this.isActive) return;

    this.stats = new Stats();
    this.setupStats();
    this.attachToDOM(container);
    this.startMonitoring();
    this.isActive = true;
  }

  /**
   * Cleanup and destroy monitoring
   */
  destroy(): void {
    if (!this.isActive) return;

    this.stopMonitoring();
    this.detachFromDOM();
    this.stats = null;
    this.customPanel = null;
    this.isActive = false;
  }

  /**
   * Set callback for metrics updates
   */
  onUpdate(callback: (metrics: PerformanceMetrics) => void): void {
    this.onMetricsUpdate = callback;
  }

  /**
   * Update configuration
   */
  updateConfig(newConfig: Partial<PerformanceConfig>): void {
    this.config = { ...this.config, ...newConfig };

    if (this.stats) {
      this.setupStats();
    }
  }

  /**
   * Get current performance metrics
   */
  getMetrics(): PerformanceMetrics {
    const currentFrame = this.getCurrentFrameData();
    const avgFps = this.getAverageFPS();
    const memoryInfo = this.getMemoryInfo();

    return {
      fps: avgFps,
      frameTime: currentFrame.frameTime,
      drawCalls: 0, // Will be updated by renderer
      triangles: 0, // Will be updated by renderer
      points: 0, // Will be updated by renderer
      lines: 0, // Will be updated by renderer
      memoryUsage: {
        geometries: 0, // Will be updated by renderer
        textures: 0, // Will be updated by renderer
        programs: 0, // Will be updated by renderer
      },
      gpuMemoryUsage: memoryInfo,
    };
  }

  /**
   * Begin frame measurement
   */
  beginFrame(): void {
    if (this.stats) {
      this.stats.begin();
    }
  }

  /**
   * End frame measurement
   */
  endFrame(): void {
    if (this.stats) {
      this.stats.end();
    }

    // Update frame history
    const now = performance.now();
    const frameTime = now - this.lastTime;
    this.lastTime = now;

    this.frameHistory.push({
      fps: 1000 / frameTime,
      frameTime,
      timestamp: now,
    });

    // Keep history size manageable
    if (this.frameHistory.length > this.maxHistorySize) {
      this.frameHistory.shift();
    }

    // Update metrics periodically
    this.frameCount++;
    if (now - this.lastFpsUpdate > this.fpsUpdateInterval) {
      this.updateMetrics();
      this.lastFpsUpdate = now;
    }
  }

  /**
   * Update custom metrics (called by renderer)
   */
  updateRenderMetrics(metrics: Partial<PerformanceMetrics>): void {
    if (this.onMetricsUpdate) {
      const currentMetrics = this.getMetrics();
      const mergedMetrics = { ...currentMetrics, ...metrics };
      this.onMetricsUpdate(mergedMetrics);
    }

    // Update custom panel if available
    if (this.customPanel && metrics.drawCalls !== undefined) {
      this.customPanel.update(metrics.drawCalls, 1000);
    }
  }

  /**
   * Get performance analysis
   */
  getAnalysis(): {
    averageFPS: number;
    minFPS: number;
    maxFPS: number;
    frameTimeVariance: number;
    isStable: boolean;
  } {
    if (this.frameHistory.length < 10) {
      return {
        averageFPS: 60,
        minFPS: 60,
        maxFPS: 60,
        frameTimeVariance: 0,
        isStable: true,
      };
    }

    const fps = this.frameHistory.map(f => f.fps);
    const frameTimes = this.frameHistory.map(f => f.frameTime);

    const averageFPS = fps.reduce((a, b) => a + b) / fps.length;
    const minFPS = Math.min(...fps);
    const maxFPS = Math.max(...fps);

    // Calculate variance in frame times
    const avgFrameTime = frameTimes.reduce((a, b) => a + b) / frameTimes.length;
    const variance =
      frameTimes.reduce((sum, ft) => sum + Math.pow(ft - avgFrameTime, 2), 0) /
      frameTimes.length;
    const frameTimeVariance = Math.sqrt(variance);

    // Consider stable if variance is low and FPS is consistent
    const isStable = frameTimeVariance < 5 && maxFPS - minFPS < 10;

    return {
      averageFPS,
      minFPS,
      maxFPS,
      frameTimeVariance,
      isStable,
    };
  }

  private setupStats(): void {
    if (!this.stats) return;

    // Configure appearance
    this.stats.dom.style.opacity = this.config.opacity.toString();
    this.stats.dom.style.transform = `scale(${this.config.scale})`;
    this.stats.dom.style.transformOrigin = 'top left';

    // Position
    this.positionStats();

    // Add custom panels
    if (this.config.enableCustom) {
      this.customPanel = this.stats.addPanel(
        new Stats.Panel('DC', '#ff8', '#221')
      );
    }

    // Show appropriate panels
    let mode = 0;
    if (this.config.enableFPS) this.stats.showPanel(0); // FPS
    if (this.config.enableMemory) {
      mode = 2; // Memory panel
      this.stats.showPanel(mode);
    }
  }

  private positionStats(): void {
    if (!this.stats) return;

    const { style } = this.stats.dom;
    style.position = 'fixed';
    style.zIndex = '10000';

    // Reset all positions
    style.top = '';
    style.right = '';
    style.bottom = '';
    style.left = '';

    switch (this.config.position) {
      case 'top-left':
        style.top = '0px';
        style.left = '0px';
        break;
      case 'top-right':
        style.top = '0px';
        style.right = '0px';
        break;
      case 'bottom-left':
        style.bottom = '0px';
        style.left = '0px';
        break;
      case 'bottom-right':
        style.bottom = '0px';
        style.right = '0px';
        break;
    }
  }

  private attachToDOM(container: HTMLElement): void {
    if (this.stats) {
      container.appendChild(this.stats.dom);
    }
  }

  private detachFromDOM(): void {
    this.stats?.dom.parentElement?.removeChild(this.stats.dom);
  }

  private startMonitoring(): void {
    this.lastTime = performance.now();
    this.lastFpsUpdate = this.lastTime;
  }

  private stopMonitoring(): void {
    if (this.rafId) {
      cancelAnimationFrame(this.rafId);
      this.rafId = null;
    }
  }

  private updateMetrics(): void {
    if (this.onMetricsUpdate) {
      const metrics = this.getMetrics();
      this.onMetricsUpdate(metrics);
    }
  }

  private getCurrentFrameData(): FrameData {
    const latest = this.frameHistory[this.frameHistory.length - 1];
    return latest || { fps: 60, frameTime: 16.67, timestamp: 0 };
  }

  private getAverageFPS(): number {
    if (this.frameHistory.length === 0) return 60;

    const recentFrames = this.frameHistory.slice(-30); // Last 30 frames
    const avgFps =
      recentFrames.reduce((sum, frame) => sum + frame.fps, 0) /
      recentFrames.length;
    return Math.round(avgFps);
  }

  private getMemoryInfo() {
    // @ts-expect-error - performance.memory is non-standard but widely supported
    const { memory } = performance;
    if (!memory) return undefined;

    return {
      buffer: 0, // Will be set by renderer
      texture: 0, // Will be set by renderer
      renderBuffer: 0, // Will be set by renderer
    };
  }
}

// Singleton instance
export const performanceMonitor = new PerformanceMonitor();

/**
 * React hook for performance monitoring
 */
export const usePerformanceMonitor = (
  config?: Partial<PerformanceConfig>,
  onUpdate?: (metrics: PerformanceMetrics) => void
) => {
  const monitor = performanceMonitor;

  React.useEffect(() => {
    if (config) {
      monitor.updateConfig(config);
    }

    if (onUpdate) {
      monitor.onUpdate(onUpdate);
    }

    // Auto-initialize in development
    if (process.env.NODE_ENV === 'development') {
      monitor.init();
    }

    return () => {
      monitor.destroy();
    };
  }, [config, onUpdate, monitor]);

  return {
    init: monitor.init.bind(monitor),
    destroy: monitor.destroy.bind(monitor),
    beginFrame: monitor.beginFrame.bind(monitor),
    endFrame: monitor.endFrame.bind(monitor),
    updateRenderMetrics: monitor.updateRenderMetrics.bind(monitor),
    getMetrics: monitor.getMetrics.bind(monitor),
    getAnalysis: monitor.getAnalysis.bind(monitor),
  };
};
