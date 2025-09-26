/**
 * IPC Performance Metrics
 * Tracks and analyzes IPC command performance
 */

export interface CommandMetrics {
  count: number;
  totalDuration: number;
  averageDuration: number;
  minDuration: number;
  maxDuration: number;
  lastExecuted: number;
  successCount: number;
  failureCount: number;
  successRate: number;
}

export interface BatchMetrics {
  count: number;
  totalCommands: number;
  averageCommandCount: number;
  totalDuration: number;
  averageDuration: number;
}

export interface OverallMetrics {
  totalCommands: number;
  totalBatches: number;
  totalDuration: number;
  averageCommandDuration: number;
  commandsPerSecond: number;
  errorRate: number;
  uptime: number;
  memoryUsage?: number;
}

export interface PerformanceSnapshot {
  timestamp: number;
  commands: Record<string, CommandMetrics>;
  batches: BatchMetrics;
  overall: OverallMetrics;
}

/**
 * Tracks performance metrics for IPC commands
 */
export class IPCMetrics {
  private readonly commandMetrics = new Map<string, CommandMetrics>();
  private readonly recentExecutions: Array<{
    command: string;
    duration: number;
    success: boolean;
    timestamp: number;
  }> = [];
  private batchMetrics: BatchMetrics;
  private readonly startTime: number;
  private readonly enabled: boolean;
  private readonly maxRecentExecutions = 1000;

  // Performance thresholds (ms)
  private readonly slowCommandThreshold = 1000;
  private readonly verySlowCommandThreshold = 5000;

  constructor(enabled = true) {
    this.enabled = enabled;
    this.startTime = Date.now();
    this.batchMetrics = {
      count: 0,
      totalCommands: 0,
      averageCommandCount: 0,
      totalDuration: 0,
      averageDuration: 0,
    };
  }

  /**
   * Record command execution
   */
  recordCommand(command: string, duration: number, success: boolean): void {
    if (!this.enabled) return;

    // Update recent executions
    this.recentExecutions.push({
      command,
      duration,
      success,
      timestamp: Date.now(),
    });

    // Keep only recent executions
    if (this.recentExecutions.length > this.maxRecentExecutions) {
      this.recentExecutions.shift();
    }

    // Update command metrics
    let metrics = this.commandMetrics.get(command);
    if (!metrics) {
      metrics = {
        count: 0,
        totalDuration: 0,
        averageDuration: 0,
        minDuration: Infinity,
        maxDuration: 0,
        lastExecuted: 0,
        successCount: 0,
        failureCount: 0,
        successRate: 0,
      };
      this.commandMetrics.set(command, metrics);
    }

    // Update metrics
    metrics.count += 1;
    metrics.totalDuration += duration;
    metrics.averageDuration = metrics.totalDuration / metrics.count;
    metrics.minDuration = Math.min(metrics.minDuration, duration);
    metrics.maxDuration = Math.max(metrics.maxDuration, duration);
    metrics.lastExecuted = Date.now();

    if (success) {
      metrics.successCount += 1;
    } else {
      metrics.failureCount += 1;
    }

    metrics.successRate = metrics.successCount / metrics.count;

    // Check for performance issues
    this.checkPerformanceThresholds(command, duration);
  }

  /**
   * Record batch execution
   */
  recordBatch(commandCount: number, duration: number): void {
    if (!this.enabled) return;

    this.batchMetrics.count += 1;
    this.batchMetrics.totalCommands += commandCount;
    this.batchMetrics.totalDuration += duration;
    this.batchMetrics.averageCommandCount =
      this.batchMetrics.totalCommands / this.batchMetrics.count;
    this.batchMetrics.averageDuration =
      this.batchMetrics.totalDuration / this.batchMetrics.count;
  }

  /**
   * Get metrics for a specific command
   */
  getCommandMetrics(command: string): CommandMetrics | undefined {
    return this.commandMetrics.get(command);
  }

  /**
   * Get all metrics
   */
  getMetrics(): PerformanceSnapshot {
    const now = Date.now();
    const uptime = now - this.startTime;
    const recentWindow = 60000; // 1 minute

    // Calculate recent metrics
    const recentExecutions = this.recentExecutions.filter(
      exec => now - exec.timestamp < recentWindow
    );

    const totalCommands = this.recentExecutions.length;
    const totalDuration = this.recentExecutions.reduce(
      (sum, exec) => sum + exec.duration,
      0
    );
    const failures = this.recentExecutions.filter(exec => !exec.success).length;

    const commands: Record<string, CommandMetrics> = {};
    this.commandMetrics.forEach((metrics, command) => {
      commands[command] = { ...metrics };
    });

    return {
      timestamp: now,
      commands,
      batches: { ...this.batchMetrics },
      overall: {
        totalCommands,
        totalBatches: this.batchMetrics.count,
        totalDuration,
        averageCommandDuration:
          totalCommands > 0 ? totalDuration / totalCommands : 0,
        commandsPerSecond: recentExecutions.length / (recentWindow / 1000),
        errorRate: totalCommands > 0 ? failures / totalCommands : 0,
        uptime,
        memoryUsage: this.getMemoryUsage(),
      },
    };
  }

  /**
   * Get slow commands
   */
  getSlowCommands(
    threshold: number = this.slowCommandThreshold
  ): Array<{ command: string; metrics: CommandMetrics }> {
    const slowCommands: Array<{ command: string; metrics: CommandMetrics }> =
      [];

    this.commandMetrics.forEach((metrics, command) => {
      if (metrics.averageDuration > threshold) {
        slowCommands.push({ command, metrics });
      }
    });

    return slowCommands.sort(
      (a, b) => b.metrics.averageDuration - a.metrics.averageDuration
    );
  }

  /**
   * Get commands with high failure rates
   */
  getUnreliableCommands(
    threshold = 0.1
  ): Array<{ command: string; metrics: CommandMetrics }> {
    const unreliableCommands: Array<{
      command: string;
      metrics: CommandMetrics;
    }> = [];

    this.commandMetrics.forEach((metrics, command) => {
      if (metrics.count >= 5 && metrics.successRate < 1 - threshold) {
        unreliableCommands.push({ command, metrics });
      }
    });

    return unreliableCommands.sort(
      (a, b) => a.metrics.successRate - b.metrics.successRate
    );
  }

  /**
   * Get performance summary
   */
  getPerformanceSummary() {
    const metrics = this.getMetrics();
    const slowCommands = this.getSlowCommands();
    const unreliableCommands = this.getUnreliableCommands();

    return {
      ...metrics.overall,
      slowCommands: slowCommands.slice(0, 5), // Top 5 slowest
      unreliableCommands: unreliableCommands.slice(0, 5), // Top 5 most unreliable
      healthScore: this.calculateHealthScore(),
    };
  }

  /**
   * Reset all metrics
   */
  reset(): void {
    this.commandMetrics.clear();
    this.recentExecutions.length = 0;
    this.batchMetrics = {
      count: 0,
      totalCommands: 0,
      averageCommandCount: 0,
      totalDuration: 0,
      averageDuration: 0,
    };
  }

  /**
   * Export metrics to JSON
   */
  exportMetrics(): string {
    return JSON.stringify(this.getMetrics(), null, 2);
  }

  /**
   * Cleanup resources
   */
  destroy(): void {
    this.reset();
  }

  // Private methods

  private checkPerformanceThresholds(command: string, duration: number): void {
    if (duration > this.verySlowCommandThreshold) {
      console.warn(`IPC: Very slow command '${command}' took ${duration}ms`);
    } else if (duration > this.slowCommandThreshold) {
      console.warn(`IPC: Slow command '${command}' took ${duration}ms`);
    }
  }

  private getMemoryUsage(): number | undefined {
    try {
      if (
        typeof window !== 'undefined' &&
        'performance' in window &&
        'memory' in window.performance
      ) {
        // Type the performance memory interface properly
        const performance = window.performance as Performance & {
          memory?: {
            usedJSHeapSize: number;
            totalJSHeapSize: number;
            jsHeapSizeLimit: number;
          };
        };
        return performance.memory?.usedJSHeapSize;
      }
    } catch (_error) {
      // Memory API not available
    }
    return undefined;
  }

  private calculateHealthScore(): number {
    const metrics = this.getMetrics();
    let score = 100;

    // Penalty for high error rate
    score -= metrics.overall.errorRate * 50;

    // Penalty for slow average duration
    if (metrics.overall.averageCommandDuration > this.slowCommandThreshold) {
      score -= 20;
    }
    if (
      metrics.overall.averageCommandDuration > this.verySlowCommandThreshold
    ) {
      score -= 30;
    }

    // Penalty for unreliable commands
    const unreliableCount = this.getUnreliableCommands().length;
    score -= unreliableCount * 5;

    // Penalty for slow commands
    const slowCount = this.getSlowCommands().length;
    score -= slowCount * 3;

    return Math.max(0, Math.min(100, score));
  }
}
