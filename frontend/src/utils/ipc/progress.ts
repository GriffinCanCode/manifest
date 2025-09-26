/**
 * IPC Progress Tracking with NProgress
 * Provides visual progress indicators for long-running commands
 */

import NProgress from 'nprogress';
import 'nprogress/nprogress.css';

import type { CommandName } from './schemas';

export interface ProgressConfig {
  enabled: boolean;
  showSpinner: boolean;
  minimum: number;
  speed: number;
  trickle: boolean;
  trickleRate: number;
  trickleSpeed: number;
  barSelector: string;
  spinnerSelector: string;
  parent: string;
  template: string;
}

const DEFAULT_CONFIG: ProgressConfig = {
  enabled: true,
  showSpinner: true,
  minimum: 0.08,
  speed: 200,
  trickle: true,
  trickleRate: 0.02,
  trickleSpeed: 800,
  barSelector: '[role="bar"]',
  spinnerSelector: '[role="spinner"]',
  parent: 'body',
  template:
    '<div class="bar" role="bar"><div class="peg"></div></div><div class="spinner" role="spinner"><div class="spinner-icon"></div></div>',
};

/**
 * Progress manager for IPC commands
 */
export class IPCProgressManager {
  private config: ProgressConfig;
  private activeCommands = new Map<
    string,
    { command: CommandName; startTime: number }
  >();
  private progressTimeout?: NodeJS.Timeout;

  constructor(config: Partial<ProgressConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config };
    this.initializeNProgress();
  }

  /**
   * Initialize NProgress with configuration
   */
  private initializeNProgress(): void {
    NProgress.configure({
      showSpinner: this.config.showSpinner,
      minimum: this.config.minimum,
      speed: this.config.speed,
      trickle: this.config.trickle,
      trickleSpeed: this.config.trickleSpeed,
      barSelector: this.config.barSelector,
      spinnerSelector: this.config.spinnerSelector,
      parent: this.config.parent,
      template: this.config.template,
    });

    // Custom styling for IPC progress
    this.addProgressStyles();
  }

  /**
   * Add custom CSS styles for progress bar
   */
  private addProgressStyles(): void {
    if (typeof document === 'undefined') return;

    const style = document.createElement('style');
    style.textContent = `
      /* IPC Progress Bar Custom Styles */
      #nprogress .bar {
        background: linear-gradient(90deg, #3b82f6, #06b6d4) !important;
        box-shadow: 0 0 10px #3b82f6, 0 0 5px #3b82f6;
        height: 3px !important;
      }

      #nprogress .peg {
        box-shadow: 0 0 10px #3b82f6, 0 0 5px #3b82f6 !important;
      }

      #nprogress .spinner-icon {
        border-top-color: #3b82f6 !important;
        border-left-color: #3b82f6 !important;
      }

      /* Command-specific progress colors */
      .nprogress-save .bar {
        background: linear-gradient(90deg, #10b981, #059669) !important;
      }

      .nprogress-load .bar {
        background: linear-gradient(90deg, #f59e0b, #d97706) !important;
      }

      .nprogress-batch .bar {
        background: linear-gradient(90deg, #8b5cf6, #7c3aed) !important;
      }

      .nprogress-error .bar {
        background: linear-gradient(90deg, #ef4444, #dc2626) !important;
      }
    `;

    document.head.appendChild(style);
  }

  /**
   * Start progress for a command
   */
  startProgress(commandId: string, command: CommandName): void {
    if (!this.config.enabled) return;

    this.activeCommands.set(commandId, {
      command,
      startTime: Date.now(),
    });

    // Add command-specific CSS class
    const progressClass = `nprogress-${this.getCommandCategory(command)}`;
    document.body.classList.add(progressClass);

    // Start progress if not already started
    if (this.activeCommands.size === 1) {
      NProgress.start();
      this.startTrickling();
    }

    // Increment progress for multiple commands
    if (this.activeCommands.size > 1) {
      const progress = Math.min(0.3 + this.activeCommands.size * 0.1, 0.8);
      NProgress.set(progress);
    }
  }

  /**
   * Update progress for a command
   */
  updateProgress(commandId: string, progress: number): void {
    if (!this.config.enabled || !this.activeCommands.has(commandId)) return;

    // Calculate overall progress based on active commands
    const commandProgress = Math.max(0.1, Math.min(0.9, progress));
    const overallProgress = commandProgress / this.activeCommands.size;

    NProgress.set(overallProgress);
  }

  /**
   * Complete progress for a command
   */
  completeProgress(commandId: string, success: boolean = true): void {
    if (!this.config.enabled) return;

    const commandInfo = this.activeCommands.get(commandId);
    if (!commandInfo) return;

    this.activeCommands.delete(commandId);

    // Remove command-specific CSS class
    const progressClass = `nprogress-${this.getCommandCategory(commandInfo.command)}`;
    document.body.classList.remove(progressClass);

    if (!success) {
      document.body.classList.add('nprogress-error');
      setTimeout(() => {
        document.body.classList.remove('nprogress-error');
      }, 1000);
    }

    // Complete progress if no more active commands
    if (this.activeCommands.size === 0) {
      this.stopTrickling();
      NProgress.done();
    } else {
      // Update progress based on remaining commands
      const progress = Math.max(0.3, 0.9 - this.activeCommands.size * 0.1);
      NProgress.set(progress);
    }
  }

  /**
   * Fail progress for a command
   */
  failProgress(commandId: string): void {
    this.completeProgress(commandId, false);
  }

  /**
   * Clear all progress
   */
  clearProgress(): void {
    this.activeCommands.clear();
    this.stopTrickling();

    // Remove all command-specific classes
    const classes = Array.from(document.body.classList).filter(cls =>
      cls.startsWith('nprogress-')
    );
    classes.forEach(cls => document.body.classList.remove(cls));

    NProgress.remove();
  }

  /**
   * Get active commands info
   */
  getActiveCommands(): Array<{
    commandId: string;
    command: CommandName;
    duration: number;
  }> {
    const now = Date.now();
    return Array.from(this.activeCommands.entries()).map(
      ([commandId, info]) => ({
        commandId,
        command: info.command,
        duration: now - info.startTime,
      })
    );
  }

  /**
   * Check if progress is active
   */
  isActive(): boolean {
    return this.activeCommands.size > 0;
  }

  /**
   * Update configuration
   */
  updateConfig(config: Partial<ProgressConfig>): void {
    this.config = { ...this.config, ...config };
    this.initializeNProgress();
  }

  /**
   * Get current configuration
   */
  getConfig(): ProgressConfig {
    return { ...this.config };
  }

  // Private methods

  private getCommandCategory(command: CommandName): string {
    const categories: Record<string, string[]> = {
      save: ['save_game', 'save_thumbnail_metadata'],
      load: ['load_game', 'load_thumbnail_metadata', 'list_saves'],
      batch: ['execute_batch_commands'],
      streaming: ['stream_tiles', 'get_tile_updates'],
      query: ['get_game_state', 'get_tile', 'get_scheduler_metrics'],
    };

    for (const [category, commands] of Object.entries(categories)) {
      if (commands.includes(command)) {
        return category;
      }
    }

    return 'default';
  }

  private startTrickling(): void {
    if (this.progressTimeout) return;

    const trickle = () => {
      if (this.activeCommands.size === 0) {
        this.stopTrickling();
        return;
      }

      // Simulate progress for long-running commands
      const longestRunning = Math.max(
        ...Array.from(this.activeCommands.values()).map(
          info => Date.now() - info.startTime
        )
      );

      if (longestRunning > 3000) {
        // 3 seconds
        NProgress.inc(0.01);
      }

      this.progressTimeout = setTimeout(trickle, 500);
    };

    this.progressTimeout = setTimeout(trickle, 1000);
  }

  private stopTrickling(): void {
    if (this.progressTimeout) {
      clearTimeout(this.progressTimeout);
      this.progressTimeout = undefined;
    }
  }
}

// Progress events for different command phases
export interface ProgressEvents {
  command_started: { commandId: string; command: CommandName };
  command_progress: { commandId: string; progress: number };
  command_completed: { commandId: string; success: boolean; duration: number };
  batch_progress: { batchId: string; completed: number; total: number };
}

// Helper functions
export const createProgressNotifications = (
  progressManager: IPCProgressManager
) => {
  return {
    commandStarted: (commandId: string, command: CommandName) => {
      progressManager.startProgress(commandId, command);
    },

    commandProgress: (commandId: string, progress: number) => {
      progressManager.updateProgress(commandId, progress);
    },

    commandCompleted: (commandId: string, success: boolean = true) => {
      progressManager.completeProgress(commandId, success);
    },

    commandFailed: (commandId: string) => {
      progressManager.failProgress(commandId);
    },

    batchProgress: (batchId: string, completed: number, total: number) => {
      const progress = completed / total;
      progressManager.updateProgress(batchId, progress);
    },
  };
};

// Default singleton instance
export const ipcProgressManager = new IPCProgressManager({
  enabled: true,
  showSpinner: true,
  minimum: 0.1,
  speed: 300,
});

// Export progress helper
export const ipcProgress = createProgressNotifications(ipcProgressManager);
