/**
 * IPC Notification System
 * Handles user notifications for IPC operations
 */

import toast, { type ToastOptions } from 'react-hot-toast';
import sift from 'sift';

import type { CommandName, EventName } from './schemas';

export interface IPCNotification {
  id: string;
  type: 'info' | 'success' | 'warning' | 'error';
  title: string;
  message: string;
  duration?: number;
  timestamp: number;
  source: 'command' | 'event' | 'system';
  commandName?: CommandName;
  eventName?: EventName;
  metadata?: Record<string, unknown>;
}

export interface NotificationConfig {
  enableToasts: boolean;
  enableHistory: boolean;
  maxHistorySize: number;
  defaultDuration: number;
  groupSimilar: boolean;
  showCommandNotifications: boolean;
  showEventNotifications: boolean;
  showSystemNotifications: boolean;
  customStyles?: Record<string, React.CSSProperties>;
}

const DEFAULT_CONFIG: NotificationConfig = {
  enableToasts: true,
  enableHistory: true,
  maxHistorySize: 100,
  defaultDuration: 4000,
  groupSimilar: true,
  showCommandNotifications: true,
  showEventNotifications: true,
  showSystemNotifications: true,
};

/**
 * Notification manager for IPC operations
 */
export class IPCNotifications {
  private config: NotificationConfig;
  private history: IPCNotification[] = [];
  private activeToasts = new Map<string, string>(); // notification id -> toast id
  private groupedNotifications = new Map<
    string,
    { count: number; lastId: string }
  >();

  constructor(config: Partial<NotificationConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config };
  }

  /**
   * Show a notification
   */
  notify(notification: Omit<IPCNotification, 'id' | 'timestamp'>): string {
    const id = this.generateId();
    const fullNotification: IPCNotification = {
      id,
      timestamp: Date.now(),
      duration: notification.duration ?? this.config.defaultDuration,
      ...notification,
    };

    // Add to history
    if (this.config.enableHistory) {
      this.addToHistory(fullNotification);
    }

    // Show toast if enabled
    if (this.config.enableToasts && this.shouldShowToast(fullNotification)) {
      this.showToast(fullNotification);
    }

    return id;
  }

  /**
   * Show success notification
   */
  success(
    title: string,
    message: string,
    options: Partial<IPCNotification> = {}
  ): string {
    return this.notify({
      type: 'success',
      title,
      message,
      source: 'system',
      ...options,
    });
  }

  /**
   * Show error notification
   */
  error(
    title: string,
    message: string,
    options: Partial<IPCNotification> = {}
  ): string {
    return this.notify({
      type: 'error',
      title,
      message,
      source: 'system',
      duration: 8000, // Errors stay longer
      ...options,
    });
  }

  /**
   * Show warning notification
   */
  warning(
    title: string,
    message: string,
    options: Partial<IPCNotification> = {}
  ): string {
    return this.notify({
      type: 'warning',
      title,
      message,
      source: 'system',
      duration: 6000,
      ...options,
    });
  }

  /**
   * Show info notification
   */
  info(
    title: string,
    message: string,
    options: Partial<IPCNotification> = {}
  ): string {
    return this.notify({
      type: 'info',
      title,
      message,
      source: 'system',
      ...options,
    });
  }

  /**
   * Show command-related notification
   */
  commandNotification(
    commandName: CommandName,
    type: IPCNotification['type'],
    title: string,
    message: string,
    options: Partial<IPCNotification> = {}
  ): string {
    return this.notify({
      type,
      title,
      message,
      source: 'command',
      commandName,
      ...options,
    });
  }

  /**
   * Show event-related notification
   */
  eventNotification(
    eventName: EventName,
    type: IPCNotification['type'],
    title: string,
    message: string,
    options: Partial<IPCNotification> = {}
  ): string {
    return this.notify({
      type,
      title,
      message,
      source: 'event',
      eventName,
      ...options,
    });
  }

  /**
   * Dismiss a notification
   */
  dismiss(notificationId: string): void {
    const toastId = this.activeToasts.get(notificationId);
    if (toastId) {
      toast.dismiss(toastId);
      this.activeToasts.delete(notificationId);
    }
  }

  /**
   * Dismiss all notifications
   */
  dismissAll(): void {
    toast.dismiss();
    this.activeToasts.clear();
  }

  /**
   * Get notification history
   */
  getHistory(): IPCNotification[] {
    return [...this.history];
  }

  /**
   * Clear notification history
   */
  clearHistory(): void {
    this.history = [];
  }

  /**
   * Get notifications by criteria using sift.js for advanced filtering
   */
  getNotifications(
    criteria: {
      type?: IPCNotification['type'];
      source?: IPCNotification['source'];
      commandName?: CommandName;
      eventName?: EventName;
      since?: number;
      // Advanced sift.js query support
      $query?: Record<string, any>;
    } = {}
  ): IPCNotification[] {
    // If advanced query is provided, use it directly
    if (criteria.$query) {
      return this.history.filter(sift(criteria.$query));
    }

    // Build sift.js query from simple criteria
    const query: Record<string, any> = {};

    if (criteria.type) {
      query.type = criteria.type;
    }

    if (criteria.source) {
      query.source = criteria.source;
    }

    if (criteria.commandName) {
      query.commandName = criteria.commandName;
    }

    if (criteria.eventName) {
      query.eventName = criteria.eventName;
    }

    if (criteria.since) {
      query.timestamp = { $gte: criteria.since };
    }

    // Use sift.js for filtering
    return this.history.filter(sift(query));
  }

  /**
   * Get notification statistics
   */
  getStats() {
    const typeCounts: Record<string, number> = {};
    const sourceCounts: Record<string, number> = {};

    this.history.forEach(notification => {
      typeCounts[notification.type] = (typeCounts[notification.type] || 0) + 1;
      sourceCounts[notification.source] =
        (sourceCounts[notification.source] || 0) + 1;
    });

    return {
      total: this.history.length,
      typeCounts,
      sourceCounts,
      recent: this.history.filter(n => Date.now() - n.timestamp < 300000)
        .length, // Last 5 minutes
    };
  }

  /**
   * Update configuration
   */
  updateConfig(config: Partial<NotificationConfig>): void {
    this.config = { ...this.config, ...config };
  }

  /**
   * Get current configuration
   */
  getConfig(): NotificationConfig {
    return { ...this.config };
  }

  // Private methods

  private shouldShowToast(notification: IPCNotification): boolean {
    switch (notification.source) {
      case 'command':
        return this.config.showCommandNotifications;
      case 'event':
        return this.config.showEventNotifications;
      case 'system':
        return this.config.showSystemNotifications;
      default:
        return true;
    }
  }

  private showToast(notification: IPCNotification): void {
    const groupKey = this.getGroupKey(notification);

    if (this.config.groupSimilar && groupKey) {
      const existing = this.groupedNotifications.get(groupKey);

      if (existing) {
        // Update existing grouped notification
        existing.count++;
        existing.lastId = notification.id;

        // Update the toast with new count
        const title = `${notification.title} (${existing.count})`;
        toast(this.formatToastMessage(title, notification.message), {
          id: existing.lastId,
          ...this.getToastOptions(notification),
        });

        return;
      } else {
        this.groupedNotifications.set(groupKey, {
          count: 1,
          lastId: notification.id,
        });
      }
    }

    const toastOptions = this.getToastOptions(notification);
    let toastId: string;

    switch (notification.type) {
      case 'success':
        toastId = toast.success(
          this.formatToastMessage(notification.title, notification.message),
          toastOptions
        );
        break;
      case 'error':
        toastId = toast.error(
          this.formatToastMessage(notification.title, notification.message),
          toastOptions
        );
        break;
      case 'warning':
        toastId = toast(
          this.formatToastMessage(notification.title, notification.message),
          { ...toastOptions, icon: '⚠️' }
        );
        break;
      case 'info':
      default:
        toastId = toast(
          this.formatToastMessage(notification.title, notification.message),
          { ...toastOptions, icon: 'ℹ️' }
        );
        break;
    }

    this.activeToasts.set(notification.id, toastId);
  }

  private formatToastMessage(title: string, message: string): string {
    return `${title}: ${message}`;
  }

  private getToastOptions(notification: IPCNotification): ToastOptions {
    return {
      duration: notification.duration,
      position: 'top-right',
      style: this.config.customStyles?.[notification.type] ?? {},
      // Add custom styling based on source
      className: `ipc-notification ipc-notification--${notification.type} ipc-notification--${notification.source}`,
    };
  }

  private getGroupKey(notification: IPCNotification): string | null {
    if (notification.source === 'command' && notification.commandName) {
      return `command:${notification.commandName}:${notification.type}`;
    }

    if (notification.source === 'event' && notification.eventName) {
      return `event:${notification.eventName}:${notification.type}`;
    }

    // Group system notifications by title
    if (notification.source === 'system') {
      return `system:${notification.title}:${notification.type}`;
    }

    return null;
  }

  private addToHistory(notification: IPCNotification): void {
    this.history.push(notification);

    // Maintain max history size
    if (this.history.length > this.config.maxHistorySize) {
      this.history.shift();
    }
  }

  private generateId(): string {
    return `notification_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`;
  }
}

// Helper functions for common notification patterns

export const createCommandNotifications = (notifications: IPCNotifications) => {
  return {
    commandStarted: (command: CommandName) => {
      notifications.commandNotification(
        command,
        'info',
        'Command Started',
        `Executing ${command}...`,
        { duration: 2000 }
      );
    },

    commandSucceeded: (command: CommandName, duration?: number) => {
      const message = duration
        ? `${command} completed in ${duration.toFixed(2)}ms`
        : `${command} completed successfully`;

      notifications.commandNotification(
        command,
        'success',
        'Command Completed',
        message,
        { duration: 3000 }
      );
    },

    commandFailed: (command: CommandName, error: string) => {
      notifications.commandNotification(
        command,
        'error',
        'Command Failed',
        `${command} failed: ${error}`,
        { duration: 8000 }
      );
    },

    commandTimeout: (command: CommandName, timeout: number) => {
      notifications.commandNotification(
        command,
        'warning',
        'Command Timeout',
        `${command} timed out after ${timeout}ms`,
        { duration: 6000 }
      );
    },

    batchCompleted: (
      commandCount: number,
      successCount: number,
      duration: number
    ) => {
      const message = `Batch of ${commandCount} commands completed (${successCount} successful) in ${duration.toFixed(2)}ms`;
      notifications.success('Batch Completed', message);
    },

    performanceWarning: (metric: string, value: number) => {
      notifications.warning(
        'Performance Warning',
        `${metric} is high: ${value}`,
        { metadata: { metric, value } }
      );
    },
  };
};

// Default singleton instance
export const ipcNotifications = new IPCNotifications();
