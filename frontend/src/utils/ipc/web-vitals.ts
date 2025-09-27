/**
 * Web Vitals Integration for IPC Performance Monitoring
 * Tracks Core Web Vitals and integrates with IPC metrics
 */

import type { Metric } from 'web-vitals';
import { onCLS, onFCP, onINP, onLCP, onTTFB } from 'web-vitals';

import type { CommandName } from './schemas';

export interface WebVitalsData {
  cls: number | null; // Cumulative Layout Shift
  fcp: number | null; // First Contentful Paint
  inp: number | null; // Interaction to Next Paint (replaces FID)
  lcp: number | null; // Largest Contentful Paint
  ttfb: number | null; // Time to First Byte
}

export interface WebVitalsConfig {
  enabled: boolean;
  reportThreshold: number; // ms
  sampleRate: number; // 0-1
  trackCommandImpact: boolean;
  enableAnalytics: boolean;
  reportEndpoint?: string;
}

interface VitalsEntry extends Metric {
  commandContext?: {
    commandName: CommandName;
    commandId: string;
    startTime: number;
  };
}

const DEFAULT_CONFIG: WebVitalsConfig = {
  enabled: true,
  reportThreshold: 100,
  sampleRate: 1.0,
  trackCommandImpact: true,
  enableAnalytics: false,
};

/**
 * Web Vitals performance monitor for IPC operations
 */
export class WebVitalsMonitor {
  private config: WebVitalsConfig;
  private vitalsData: WebVitalsData = {
    cls: null,
    fcp: null,
    inp: null,
    lcp: null,
    ttfb: null,
  };
  private listeners = new Set<(data: WebVitalsData) => void>();
  private activeCommands = new Map<
    string,
    {
      commandName: CommandName;
      startTime: number;
      vitalsSnapshot: Partial<WebVitalsData>;
    }
  >();

  constructor(config: Partial<WebVitalsConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config };

    if (this.config.enabled && typeof window !== 'undefined') {
      this.initializeVitalsTracking();
    }
  }

  /**
   * Initialize Web Vitals tracking
   */
  private initializeVitalsTracking(): void {
    // Should only track on a sample of sessions to avoid performance impact
    if (Math.random() > this.config.sampleRate) {
      return;
    }

    // Track Core Web Vitals (simplified stub for now)
    try {
      // Import functions dynamically to handle API changes
      onCLS(this.handleMetric.bind(this));
      onFCP(this.handleMetric.bind(this));
      onINP(this.handleMetric.bind(this));
      onLCP(this.handleMetric.bind(this));
      onTTFB(this.handleMetric.bind(this));
    } catch (error) {
      console.warn('Web Vitals API not available:', error);
    }
  }

  /**
   * Handle incoming web vitals metrics
   */
  private handleMetric(metric: Metric): void {
    const entry: VitalsEntry = { ...metric };

    // Check if this metric occurred during a command execution
    if (this.config.trackCommandImpact) {
      const commandContext = this.findCommandContext(metric.value);
      if (commandContext) {
        entry.commandContext = commandContext;
      }
    }

    // Update internal state
    this.updateVitalsData(entry);

    // Report if threshold exceeded
    if (entry.value > this.config.reportThreshold) {
      this.reportMetric(entry);
    }

    // Notify listeners
    this.notifyListeners();
  }

  /**
   * Update internal vitals data
   */
  private updateVitalsData(entry: VitalsEntry): void {
    switch (entry.name) {
      case 'CLS':
        this.vitalsData.cls = entry.value;
        break;
      case 'FCP':
        this.vitalsData.fcp = entry.value;
        break;
      case 'INP':
        this.vitalsData.inp = entry.value;
        break;
      case 'LCP':
        this.vitalsData.lcp = entry.value;
        break;
      case 'TTFB':
        this.vitalsData.ttfb = entry.value;
        break;
    }
  }

  /**
   * Find command context for a metric timestamp
   */
  private findCommandContext(
    timestamp: number
  ): VitalsEntry['commandContext'] | null {
    for (const [commandId, context] of this.activeCommands) {
      const timeDiff = timestamp - context.startTime;
      // If metric occurred within 5 seconds of command start
      if (timeDiff >= 0 && timeDiff <= 5000) {
        return {
          commandName: context.commandName,
          commandId,
          startTime: context.startTime,
        };
      }
    }
    return null;
  }

  /**
   * Start tracking a command's impact on vitals
   */
  startCommandTracking(commandId: string, commandName: CommandName): void {
    if (!this.config.enabled || !this.config.trackCommandImpact) return;

    this.activeCommands.set(commandId, {
      commandName,
      startTime: performance.now(),
      vitalsSnapshot: { ...this.vitalsData },
    });
  }

  /**
   * Stop tracking a command's impact
   */
  stopCommandTracking(commandId: string): WebVitalsData | null {
    if (!this.config.enabled || !this.config.trackCommandImpact) return null;

    const context = this.activeCommands.get(commandId);
    if (!context) return null;

    this.activeCommands.delete(commandId);

    // Calculate vitals diff during command execution
    const diff: WebVitalsData = {
      cls: this.calculateDiff(
        context.vitalsSnapshot.cls ?? null,
        this.vitalsData.cls
      ),
      fcp: this.calculateDiff(
        context.vitalsSnapshot.fcp ?? null,
        this.vitalsData.fcp
      ),
      inp: this.calculateDiff(
        context.vitalsSnapshot.inp ?? null,
        this.vitalsData.inp
      ),
      lcp: this.calculateDiff(
        context.vitalsSnapshot.lcp ?? null,
        this.vitalsData.lcp
      ),
      ttfb: this.calculateDiff(
        context.vitalsSnapshot.ttfb ?? null,
        this.vitalsData.ttfb
      ),
    };

    return diff;
  }

  /**
   * Calculate difference between two metrics
   */
  private calculateDiff(
    before: number | null,
    after: number | null
  ): number | null {
    if (before === null || after === null) return null;
    return after - before;
  }

  /**
   * Report metric to analytics or console
   */
  private reportMetric(entry: VitalsEntry): void {
    const report = {
      metric: entry.name,
      value: entry.value,
      rating: this.getRating(entry.name, entry.value),
      timestamp: Date.now(),
      commandContext: entry.commandContext,
      userAgent: navigator.userAgent,
      connectionType: this.getConnectionType(),
    };

    if (this.config.enableAnalytics && this.config.reportEndpoint) {
      // Send to analytics endpoint
      this.sendToAnalytics(report);
    } else {
      // Log to console in development
      console.warn('Web Vital threshold exceeded:', report);
    }
  }

  /**
   * Get performance rating for a metric
   */
  private getRating(
    name: string,
    value: number
  ): 'good' | 'needs-improvement' | 'poor' {
    const thresholds: Record<string, [number, number]> = {
      CLS: [0.1, 0.25],
      FCP: [1800, 3000],
      INP: [200, 500],
      LCP: [2500, 4000],
      TTFB: [800, 1800],
    };

    const [goodThreshold, poorThreshold] = thresholds[name] || [0, Infinity];

    if (value <= goodThreshold) return 'good';
    if (value <= poorThreshold) return 'needs-improvement';
    return 'poor';
  }

  /**
   * Get connection type information
   */
  private getConnectionType(): string {
    const connection = (navigator as any).connection as
      | { effectiveType?: string }
      | undefined;
    return connection?.effectiveType ?? 'unknown';
  }

  /**
   * Send metric to analytics endpoint
   */
  private sendToAnalytics(report: Record<string, unknown>): void {
    if (!this.config.reportEndpoint) return;

    // Use void to explicitly ignore the promise
    void fetch(this.config.reportEndpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(report),
      keepalive: true, // Ensure delivery even if page unloads
    }).catch(error => {
      console.error('Failed to report web vital:', error);
    });
  }

  /**
   * Get current vitals data
   */
  getVitalsData(): WebVitalsData {
    return { ...this.vitalsData };
  }

  /**
   * Get vitals summary with ratings
   */
  getVitalsSummary(): {
    data: WebVitalsData;
    ratings: Record<
      keyof WebVitalsData,
      'good' | 'needs-improvement' | 'poor' | 'unknown'
    >;
    overallRating: 'good' | 'needs-improvement' | 'poor' | 'unknown';
  } {
    const data = this.getVitalsData();
    const ratings: Record<
      keyof WebVitalsData,
      'good' | 'needs-improvement' | 'poor' | 'unknown'
    > = {
      cls: data.cls !== null ? this.getRating('CLS', data.cls) : 'unknown',
      fcp: data.fcp !== null ? this.getRating('FCP', data.fcp) : 'unknown',
      inp: data.inp !== null ? this.getRating('INP', data.inp) : 'unknown',
      lcp: data.lcp !== null ? this.getRating('LCP', data.lcp) : 'unknown',
      ttfb: data.ttfb !== null ? this.getRating('TTFB', data.ttfb) : 'unknown',
    };

    // Calculate overall rating
    const ratingValues = Object.values(ratings).filter(r => r !== 'unknown');
    const poorCount = ratingValues.filter(r => r === 'poor').length;
    const needsImprovementCount = ratingValues.filter(
      r => r === 'needs-improvement'
    ).length;

    let overallRating: 'good' | 'needs-improvement' | 'poor' | 'unknown' =
      'unknown';
    if (ratingValues.length > 0) {
      if (poorCount > 0) {
        overallRating = 'poor';
      } else if (needsImprovementCount > 0) {
        overallRating = 'needs-improvement';
      } else {
        overallRating = 'good';
      }
    }

    return { data, ratings, overallRating };
  }

  /**
   * Subscribe to vitals updates
   */
  subscribe(listener: (data: WebVitalsData) => void): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  /**
   * Notify all listeners of vitals update
   */
  private notifyListeners(): void {
    this.listeners.forEach(listener => {
      try {
        listener(this.vitalsData);
      } catch (error) {
        console.error('Error in web vitals listener:', error);
      }
    });
  }

  /**
   * Update configuration
   */
  updateConfig(config: Partial<WebVitalsConfig>): void {
    this.config = { ...this.config, ...config };
  }

  /**
   * Get current configuration
   */
  getConfig(): WebVitalsConfig {
    return { ...this.config };
  }

  /**
   * Reset all vitals data
   */
  reset(): void {
    this.vitalsData = {
      cls: null,
      fcp: null,
      inp: null,
      lcp: null,
      ttfb: null,
    };
    this.activeCommands.clear();
    this.notifyListeners();
  }

  /**
   * Get performance insights
   */
  getPerformanceInsights(): {
    criticalMetrics: string[];
    recommendations: string[];
    commandImpacts: Array<{
      commandName: CommandName;
      impactScore: number;
      details: string;
    }>;
  } {
    const summary = this.getVitalsSummary();
    const criticalMetrics: string[] = [];
    const recommendations: string[] = [];

    // Analyze critical metrics
    if (summary.ratings.lcp === 'poor') {
      criticalMetrics.push('Largest Contentful Paint');
      recommendations.push('Optimize images and reduce server response time');
    }
    if (summary.ratings.inp === 'poor') {
      criticalMetrics.push('Interaction to Next Paint');
      recommendations.push(
        'Reduce JavaScript execution time and break up long tasks'
      );
    }
    if (summary.ratings.cls === 'poor') {
      criticalMetrics.push('Cumulative Layout Shift');
      recommendations.push(
        'Add size attributes to images and avoid inserting content above existing content'
      );
    }

    // Analyze command impacts (simplified for now)
    const commandImpacts: Array<{
      commandName: CommandName;
      impactScore: number;
      details: string;
    }> = [];

    return {
      criticalMetrics,
      recommendations,
      commandImpacts,
    };
  }

  /**
   * Export vitals data for analysis
   */
  exportData(): string {
    const data = {
      vitals: this.getVitalsData(),
      summary: this.getVitalsSummary(),
      insights: this.getPerformanceInsights(),
      config: this.getConfig(),
      timestamp: Date.now(),
      userAgent: navigator.userAgent,
    };

    return JSON.stringify(data, null, 2);
  }
}

// Helper functions for IPC integration
export const createVitalsIntegration = (monitor: WebVitalsMonitor) => {
  return {
    trackCommand: (commandId: string, commandName: CommandName) => {
      monitor.startCommandTracking(commandId, commandName);
    },

    completeCommand: (commandId: string): WebVitalsData | null => {
      return monitor.stopCommandTracking(commandId);
    },

    getVitalsSummary: () => {
      return monitor.getVitalsSummary();
    },

    reportPerformanceIssue: (
      commandName: CommandName,
      vitals: WebVitalsData
    ) => {
      const hasIssue = Object.values(vitals).some(
        value => value !== null && value > 100
      );

      if (hasIssue) {
        console.warn(
          `Performance impact detected for command ${commandName}:`,
          vitals
        );
      }
    },
  };
};

// Default singleton instance
export const webVitalsMonitor = new WebVitalsMonitor({
  enabled: (import.meta as any)?.env?.MODE === 'production',
  reportThreshold: 100,
  trackCommandImpact: true,
});

// Export integration helper
export const vitalsIntegration = createVitalsIntegration(webVitalsMonitor);
