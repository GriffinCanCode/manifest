/**
 * Comprehensive Logging Service for Manifest Frontend
 * Provides structured logging with file output, performance monitoring, and context tracking
 * Browser-compatible implementation without Node.js dependencies
 */

import { appDataDir, join } from '@tauri-apps/api/path';
import { create, exists, writeTextFile } from '@tauri-apps/plugin-fs';

import { getLoggingConfig, isLoggingEnabled } from '../config/logging';

// Browser-compatible environment check with proper typing
interface ViteImportMeta {
  env?: {
    MODE?: string;
    [key: string]: string | boolean | undefined;
  };
}

// Log levels and configuration
export type LogLevel = 'error' | 'warn' | 'info' | 'debug' | 'verbose';
export type LogCategory =
  | 'app'
  | 'render'
  | 'game'
  | 'performance'
  | 'network'
  | 'storage'
  | 'ui'
  | 'shader'
  | 'streaming';

export interface LogContext {
  category: LogCategory;
  component?: string;
  userId?: string;
  sessionId?: string;
  timestamp?: string;
  performance?: {
    duration?: number;
    memory?: number;
    fps?: number;
  };
  metadata?: Record<string, unknown>;
}

export interface LogEntry {
  level: LogLevel;
  message: string;
  context: LogContext;
  error?: Error;
  stack?: string;
}

// Browser-compatible log formatting utilities
const formatTimestamp = (date: Date = new Date()): string => {
  return date.toISOString().replace('T', ' ').slice(0, -5);
};

// Browser console formatter for development
const formatConsoleMessage = (info: {
  timestamp?: string;
  level: string;
  message: string;
  category?: string;
  component?: string;
  [key: string]: unknown;
}): string => {
  const { level, message, category, component } = info;
  const categoryStr = typeof category === 'string' ? category : 'app';
  const componentStr = typeof component === 'string' ? component : undefined;
  const prefix = componentStr
    ? `[${categoryStr}:${componentStr}]`
    : `[${categoryStr}]`;

  const timestamp = new Date().toLocaleTimeString('en-US', {
    hour12: false,
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    fractionalSecondDigits: 3,
  });

  return `${timestamp} ${String(level).toUpperCase()} ${prefix} ${String(message)}`;
};

/**
 * Enhanced file logging implementation with fallback options
 */
const writeLogToTauriFile = async (entry: LogEntry): Promise<void> => {
  try {
    // Try Tauri app data directory first
    const appDir = await appDataDir();
    const logDir = await join(appDir, 'logs');

    if (!(await exists(logDir))) {
      await create(logDir);
    }

    const date = new Date().toISOString().split('T')[0];
    const logFileName = `manifest-${date}.log`;
    const logFilePath = await join(logDir, logFileName);

    // Enhanced log format with readable timestamp
    const logLine = `[${new Date().toISOString()}] ${entry.level.toUpperCase()} [${entry.context.category}${entry.context.component ? `:${entry.context.component}` : ''}] ${entry.message}${entry.error ? ` ERROR: ${entry.error.message}` : ''}${entry.context.metadata ? ` ${JSON.stringify(entry.context.metadata)}` : ''}\n`;

    // Append to file (create if doesn't exist)
    await writeTextFile(logFilePath, logLine, { append: true });

    // Also write to project logs directory for easier access during development
    try {
      const projectLogDir = '../backend/logs';
      const projectLogPath = `${projectLogDir}/frontend-${date}.log`;
      await writeTextFile(projectLogPath, logLine, { append: true });
    } catch (projectError) {
      // Silently fail for project logs - not critical
      console.warn('Could not write to project logs directory:', projectError);
    }
  } catch (error) {
    console.error('Failed to write log to file:', error);

    // Fallback: try to write to browser localStorage for later retrieval
    try {
      const storageKey = `manifest-logs-${new Date().toISOString().split('T')[0]}`;
      const existingLogs = localStorage.getItem(storageKey) ?? '';
      const logLine = `${new Date().toISOString()} ${entry.level.toUpperCase()} [${entry.context.category}] ${entry.message}\n`;
      localStorage.setItem(storageKey, existingLogs + logLine);
    } catch (storageError) {
      console.error('Failed to write to localStorage backup:', storageError);
    }
  }
};

// No longer needed - using direct console method calls

// Get environment mode in browser-compatible way
const getEnvironmentMode = (): string => {
  return (import.meta as ViteImportMeta)?.env?.MODE ?? 'development';
};

/**
 * Browser-Compatible Logger Service Class
 */
class LoggerService {
  private sessionId: string;
  private isProduction: boolean;

  constructor() {
    this.sessionId = this.generateSessionId();
    this.isProduction = getEnvironmentMode() === 'production';

    // Only log initialization in development if debug logging is enabled
    if (!this.isProduction && getLoggingConfig().level === 'debug') {
      console.warn(
        '🔧 LOGGER: LoggerService initialized for session:',
        this.sessionId
      );
    }

    // Log initialization using normal logging flow
    this.logToBrowser('info', 'Logging system initialized', 'app', undefined, {
      sessionId: this.sessionId,
      environment: getEnvironmentMode(),
    });
  }

  private generateSessionId(): string {
    return `session-${Date.now()}-${Math.random().toString(36).slice(2, 11)}`;
  }

  private logToBrowser(
    level: LogLevel,
    message: string,
    category: string,
    component?: string,
    metadata?: Record<string, unknown>,
    error?: Error
  ): void {
    const config = getLoggingConfig();

    if (!config.enableConsoleOutput) {
      return;
    }

    const logInfo = {
      timestamp: formatTimestamp(),
      level,
      message,
      category,
      component,
      ...metadata,
      error,
    };

    const formattedMessage = formatConsoleMessage(logInfo);

    // Use only allowed console methods (warn, error) per project linting rules
    switch (level) {
      case 'error':
        if (error) {
          console.error(formattedMessage, error);
        } else {
          console.error(formattedMessage, metadata ?? '');
        }
        break;
      case 'warn':
        console.warn(formattedMessage, metadata ?? '');
        break;
      case 'info':
      case 'debug':
      case 'verbose':
      default:
        // Use warn for non-error levels since only warn and error are allowed
        console.warn(formattedMessage, metadata ?? '');
    }
  }

  /**
   * Create a contextual logger for a specific category/component
   */
  createLogger(category: LogCategory, component?: string) {
    return {
      error: (
        message: string,
        error?: Error,
        metadata?: Record<string, unknown>
      ) => this.log('error', message, { category, component }, error, metadata),

      warn: (message: string, metadata?: Record<string, unknown>) =>
        this.log('warn', message, { category, component }, undefined, metadata),

      info: (message: string, metadata?: Record<string, unknown>) =>
        this.log('info', message, { category, component }, undefined, metadata),

      debug: (message: string, metadata?: Record<string, unknown>) =>
        this.log(
          'debug',
          message,
          { category, component },
          undefined,
          metadata
        ),

      verbose: (message: string, metadata?: Record<string, unknown>) =>
        this.log(
          'verbose',
          message,
          { category, component },
          undefined,
          metadata
        ),

      // Performance logging helper
      performance: (
        message: string,
        performanceData: LogContext['performance'],
        metadata?: Record<string, unknown>
      ) =>
        this.log(
          'info',
          message,
          { category, component, performance: performanceData },
          undefined,
          metadata
        ),

      // Timer utility
      startTimer: (label: string) => {
        const startTime = performance.now();
        return {
          end: (message?: string, metadata?: Record<string, unknown>) => {
            const duration = performance.now() - startTime;
            this.log(
              'debug',
              message ?? `Timer [${label}] completed`,
              { category, component, performance: { duration } },
              undefined,
              metadata
            );
            return duration;
          },
        };
      },

      // Error boundary logging
      errorBoundary: (
        error: Error,
        errorInfo: { componentStack?: string },
        metadata?: Record<string, unknown>
      ) => {
        this.log(
          'error',
          'React Error Boundary caught an error',
          { category, component },
          error,
          { ...metadata, componentStack: errorInfo.componentStack }
        );
      },
    };
  }

  private log(
    level: LogLevel,
    message: string,
    context: Partial<LogContext>,
    error?: Error,
    metadata?: Record<string, unknown>
  ) {
    const category = context.category ?? 'app';

    // Check if logging is enabled for this category and level
    if (!isLoggingEnabled(category, level)) {
      return;
    }

    const logData = {
      message,
      category,
      component: context.component,
      sessionId: this.sessionId,
      performance: context.performance,
      ...metadata,
    };

    // Log to browser console
    this.logToBrowser(
      level,
      message,
      category,
      context.component,
      logData,
      error
    );

    // Also write to Tauri file if available and configured
    const config = getLoggingConfig();
    if (
      config.enableFileOutput &&
      typeof window !== 'undefined' &&
      window &&
      '__TAURI__' in window
    ) {
      const entry: LogEntry = {
        level,
        message,
        context: {
          category,
          component: context.component,
          timestamp: new Date().toISOString(),
          performance: context.performance,
          metadata,
        },
        error,
      };

      void writeLogToTauriFile(entry);
    }
  }

  /**
   * Get current session ID for tracking
   */
  getSessionId(): string {
    return this.sessionId;
  }

  /**
   * Manually flush logs (useful before app exit)
   */
  async flush(): Promise<void> {
    // For browser implementation, this is a no-op since console logs are immediate
    // Could be extended to flush any buffered file writes in the future
    return Promise.resolve();
  }

  /**
   * Update log level dynamically
   */
  setLogLevel(level: LogLevel) {
    // For browser implementation, we'll use the config system
    // This could be extended to maintain runtime state
    this.logToBrowser('info', 'Log level change requested', 'app', undefined, {
      requestedLevel: level,
      sessionId: this.sessionId,
      note: 'Use LoggingConfigManager.setOverride() for runtime level changes',
    });
  }

  /**
   * Get log statistics
   */
  getLogStats() {
    const config = getLoggingConfig();
    return {
      sessionId: this.sessionId,
      isProduction: this.isProduction,
      currentLevel: config.level,
      environment: getEnvironmentMode(),
      consoleOutputEnabled: config.enableConsoleOutput,
      fileOutputEnabled: config.enableFileOutput,
    };
  }
}

// Create singleton instance
const loggerService = new LoggerService();

// Export convenience loggers for different categories
export const AppLogger = loggerService.createLogger('app');
export const RenderLogger = loggerService.createLogger('render');
export const GameLogger = loggerService.createLogger('game');
export const PerformanceLogger = loggerService.createLogger('performance');
export const NetworkLogger = loggerService.createLogger('network');
export const StorageLogger = loggerService.createLogger('storage');
export const UILogger = loggerService.createLogger('ui');
export const ShaderLogger = loggerService.createLogger('shader');
export const StreamingLogger = loggerService.createLogger('streaming');

// Export the service itself for advanced usage
export { loggerService as Logger };

// Export helper functions
export const createComponentLogger = (
  category: LogCategory,
  component: string
) => loggerService.createLogger(category, component);

export default loggerService;
