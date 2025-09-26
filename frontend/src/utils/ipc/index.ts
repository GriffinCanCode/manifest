/**
 * IPC Communication Layer
 * Comprehensive type-safe communication system for Tauri backend
 */

import * as React from 'react';
import { ErrorBoundary } from 'react-error-boundary';

// Import types for proper type safety
import type { HistoryConfig } from './history';
import type { NotificationConfig } from './notifications';
import type { ProgressConfig } from './progress';
import type { CommandInput, CommandName, CommandOutput } from './schemas';
import type { IPCConfig } from './service';

// Performance memory types
interface PerformanceMemory {
  usedJSHeapSize: number;
  totalJSHeapSize: number;
  jsHeapSizeLimit: number;
}

// Extended Performance interface
interface ExtendedPerformance extends Performance {
  memory?: PerformanceMemory;
}

// Core exports
export { EventEmitter } from './events';
export { CommandHistory, commandHistory } from './history';
export { IPCMetrics } from './metrics';
export {
  createCommandNotifications,
  IPCNotifications,
  ipcNotifications,
} from './notifications';
export {
  createProgressNotifications,
  ipcProgress,
  IPCProgressManager,
  ipcProgressManager,
} from './progress';
export { IPCService, ipcService } from './service';
export {
  createValtioIntegration,
  valtioIntegration,
  ValtioStateSync,
  valtioStateSync,
} from './valtio-sync';
export {
  createVitalsIntegration,
  vitalsIntegration,
  WebVitalsMonitor,
  webVitalsMonitor,
} from './web-vitals';

// Schema exports
export * from './schemas';

// Hook exports
export * from './hooks';

// Type exports
export type {
  IPCCommand,
  IPCCommandResult,
  IPCConfig,
  IPCError,
} from './service';

export type { HistoryConfig, HistoryEntry, HistoryState } from './history';

export type {
  BatchMetrics,
  CommandMetrics,
  PerformanceSnapshot,
} from './metrics';

export type {
  AnyEventHandler,
  EventHandler,
  ExtendedEventData,
  ExtendedEventName,
  IPCEvents,
} from './events';

export type { IPCNotification, NotificationConfig } from './notifications';

export type { ProgressConfig, ProgressEvents } from './progress';

export type { WebVitalsConfig, WebVitalsData } from './web-vitals';

export type { ValtioSyncConfig } from './valtio-sync';

// Helper type imports for initialization function
import type { ValtioSyncConfig } from './valtio-sync';
import type { WebVitalsConfig } from './web-vitals';

// Utility functions and constants

/**
 * Initialize the IPC system with configuration
 */
export const initializeIPC = async (
  config: {
    service?: Partial<IPCConfig>;
    history?: Partial<HistoryConfig>;
    notifications?: Partial<NotificationConfig>;
    progress?: Partial<ProgressConfig>;
    webVitals?: Partial<WebVitalsConfig>;
    valtio?: Partial<ValtioSyncConfig>;
  } = {}
) => {
  // Import classes dynamically
  const { IPCService } = await import('./service');
  const { CommandHistory } = await import('./history');
  const { IPCNotifications, createCommandNotifications } = await import(
    './notifications'
  );
  const { IPCProgressManager, createProgressNotifications } = await import(
    './progress'
  );
  const { WebVitalsMonitor, createVitalsIntegration } = await import(
    './web-vitals'
  );
  const { ValtioStateSync, createValtioIntegration } = await import(
    './valtio-sync'
  );

  // Initialize services with config
  const service = new IPCService(config.service);
  const history = new CommandHistory(config.history);
  const notifications = new IPCNotifications(config.notifications);
  const progress = new IPCProgressManager(config.progress);
  const webVitals = new WebVitalsMonitor(config.webVitals);
  const valtio = new ValtioStateSync(config.valtio);
  const commandNotifications = createCommandNotifications(notifications);
  const progressNotifications = createProgressNotifications(progress);
  const vitalsIntegration = createVitalsIntegration(webVitals);
  const valtioIntegration = createValtioIntegration(valtio);

  // Wire up automatic history tracking
  const originalCommand = service.command.bind(service);
  service.command = async <T extends CommandName>(
    name: T,
    input: CommandInput<T>,
    options: {
      priority?: 'low' | 'normal' | 'high';
      timeout?: number;
      retries?: number;
      validate?: boolean;
    } = {}
  ): Promise<CommandOutput<T>> => {
    // Add to history
    const historyId = history.addCommand(name, input, {
      metadata: {
        sessionId: 'current', // Could be dynamic
      },
    });

    try {
      const startTime = performance.now();
      const result: CommandOutput<T> = await originalCommand(
        name,
        input,
        options
      );
      const duration = performance.now() - startTime;

      // Complete history entry
      history.completeCommand(historyId, result, duration);

      // Show success notification for important commands
      if (shouldNotifyForCommand(name)) {
        commandNotifications.commandSucceeded(name, duration);
      }

      return result;
    } catch (error) {
      // Show error notification
      commandNotifications.commandFailed(name, (error as Error).message);
      throw error;
    }
  };

  // Wire up event notifications
  service.onEvent('performance_warning', data => {
    commandNotifications.performanceWarning(data.metric, data.value);
  });

  service.onEvent('error_occurred', data => {
    notifications.error(
      'Command Failed',
      `${data.command} failed: ${data.error}`
    );
  });

  return {
    service,
    history,
    notifications,
    progress,
    webVitals,
    valtio,
    commandNotifications,
    progressNotifications,
    vitalsIntegration,
    valtioIntegration,
  };
};

/**
 * Default configuration presets
 */
export const IPCPresets = {
  development: {
    service: {
      enableMetrics: true,
      maxConcurrentCommands: 5,
      defaultTimeout: 30000,
    },
    history: {
      maxSize: 500,
      persistToStorage: true,
      enablePatches: true,
    },
    notifications: {
      enableToasts: true,
      showCommandNotifications: true,
      showSystemNotifications: true,
    },
    progress: {
      enabled: true,
      showSpinner: true,
      minimum: 0.1,
    },
    webVitals: {
      enabled: false, // Disabled in dev to avoid noise
      trackCommandImpact: true,
      reportThreshold: 200,
    },
    valtio: {
      enabled: true,
      autoSync: true,
      syncInterval: 2000,
      persistState: false, // No persistence in dev
    },
  },

  production: {
    service: {
      enableMetrics: true,
      maxConcurrentCommands: 10,
      defaultTimeout: 15000,
    },
    history: {
      maxSize: 200,
      persistToStorage: true,
      enablePatches: false, // Disable for performance
    },
    notifications: {
      enableToasts: true,
      showCommandNotifications: false, // Less noise
      showSystemNotifications: true,
    },
    progress: {
      enabled: true,
      showSpinner: false, // Less UI noise in production
      minimum: 0.08,
    },
    webVitals: {
      enabled: true, // Important for production monitoring
      trackCommandImpact: true,
      reportThreshold: 100,
      enableAnalytics: true,
    },
    valtio: {
      enabled: true,
      autoSync: true,
      syncInterval: 5000, // Less frequent in production
      persistState: true, // Enable persistence in production
    },
  },

  testing: {
    service: {
      enableMetrics: false,
      maxConcurrentCommands: 1,
      defaultTimeout: 5000,
    },
    history: {
      maxSize: 50,
      persistToStorage: false,
      enablePatches: false,
    },
    notifications: {
      enableToasts: false,
      showCommandNotifications: false,
      showSystemNotifications: false,
    },
    progress: {
      enabled: false, // No progress in tests
      showSpinner: false,
    },
    webVitals: {
      enabled: false, // No web vitals tracking in tests
      trackCommandImpact: false,
    },
    valtio: {
      enabled: false, // No valtio state sync in tests
      autoSync: false,
      persistState: false,
    },
  },
} as const;

/**
 * Utility functions
 */
export const shouldNotifyForCommand = (command: CommandName): boolean => {
  // Don't notify for frequent read operations
  const quietCommands = [
    'get_game_state',
    'stream_tiles',
    'get_tile',
    'get_tile_updates',
    'get_scheduler_metrics',
  ];

  return !quietCommands.includes(command);
};

export const createIPCErrorBoundary = () => {
  const IPCErrorBoundary = ({
    children,
    fallback,
    onError,
  }: {
    children: React.ReactNode;
    fallback?: React.ComponentType<{
      error: Error;
      resetErrorBoundary: () => void;
    }>;
    onError?: (error: Error, errorInfo: React.ErrorInfo) => void;
  }) => {
    const handleError = (error: Error, errorInfo: React.ErrorInfo) => {
      // Import ipcNotifications dynamically to avoid circular imports
      void import('./notifications').then(({ ipcNotifications }) => {
        ipcNotifications.error(
          'IPC Error',
          `An IPC communication error occurred: ${error.message}`,
          { metadata: { error: error.stack } }
        );
      });

      // Call the provided onError callback
      onError?.(error, errorInfo);
    };

    return React.createElement(ErrorBoundary, {
      fallback: fallback ?? DefaultIPCErrorFallback,
      onError: handleError,
      children,
    });
  };

  return IPCErrorBoundary;
};

const DefaultIPCErrorFallback = ({
  error,
  resetErrorBoundary,
}: {
  error: Error;
  resetErrorBoundary: () => void;
}) => {
  return React.createElement(
    'div',
    {
      className: 'ipc-error-boundary',
      style: {
        padding: '20px',
        margin: '10px',
        border: '1px solid #dc3545',
        borderRadius: '4px',
        backgroundColor: '#f8d7da',
        color: '#721c24',
      },
    },
    React.createElement(
      'h2',
      { style: { marginTop: 0 } },
      'IPC Communication Error'
    ),
    React.createElement(
      'p',
      null,
      'Something went wrong with the backend communication:'
    ),
    React.createElement(
      'pre',
      {
        style: {
          backgroundColor: '#fff',
          padding: '10px',
          borderRadius: '4px',
          overflow: 'auto',
        },
      },
      error.message
    ),
    React.createElement(
      'button',
      {
        onClick: resetErrorBoundary,
        style: {
          backgroundColor: '#dc3545',
          color: 'white',
          border: 'none',
          padding: '8px 16px',
          borderRadius: '4px',
          cursor: 'pointer',
        },
      },
      'Try Again'
    )
  );
};

// React-error-boundary now handles the error boundary implementation

/**
 * Performance monitoring utilities
 */
export const IPCPerformance = {
  /**
   * Measure command performance
   */
  measureCommand: async <T extends CommandName>(
    command: T,
    input: CommandInput<T>
  ) => {
    const startTime = performance.now();
    const startMemory =
      (performance as ExtendedPerformance).memory?.usedJSHeapSize ?? 0;

    try {
      const { ipcService } = await import('./service');
      const result = await ipcService.command(command, input);
      const endTime = performance.now();
      const endMemory =
        (performance as ExtendedPerformance).memory?.usedJSHeapSize ?? 0;

      return {
        result,
        metrics: {
          duration: endTime - startTime,
          memoryDelta: endMemory - startMemory,
          success: true,
        },
      };
    } catch (error) {
      const endTime = performance.now();

      return {
        result: null,
        error,
        metrics: {
          duration: endTime - startTime,
          memoryDelta: 0,
          success: false,
        },
      };
    }
  },

  /**
   * Get current performance snapshot
   */
  getSnapshot: async () => {
    const { ipcService } = await import('./service');
    const { commandHistory } = await import('./history');
    const { ipcNotifications } = await import('./notifications');

    return {
      ipc: ipcService.getMetrics(),
      history: commandHistory.getStats(),
      notifications: ipcNotifications.getStats(),
      memory: (() => {
        const mem = (performance as ExtendedPerformance).memory;
        return mem
          ? {
              used: mem.usedJSHeapSize,
              total: mem.totalJSHeapSize,
              limit: mem.jsHeapSizeLimit,
            }
          : null;
      })(),
    };
  },
};

// Debug utilities (development only)
if (process.env.NODE_ENV === 'development') {
  // Expose to window for debugging (load async to avoid circular imports)
  void Promise.all([
    import('./service'),
    import('./history'),
    import('./notifications'),
  ]).then(([{ ipcService }, { commandHistory }, { ipcNotifications }]) => {
    (window as unknown as { __IPC_DEBUG__: unknown }).__IPC_DEBUG__ = {
      service: ipcService,
      history: commandHistory,
      notifications: ipcNotifications,
      performance: IPCPerformance,
    };

    // Development console log is acceptable
    // eslint-disable-next-line no-console
    console.log('🔧 IPC Debug utilities available at window.__IPC_DEBUG__');
  });
}
