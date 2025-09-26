/**
 * Logging Configuration for Manifest Frontend
 * Manages log levels, output destinations, and environment-specific settings
 */

import type { LogCategory, LogLevel } from '../services/logger';

// Browser-compatible environment check with proper typing
interface ViteImportMeta {
  env?: {
    MODE?: string;
    [key: string]: string | boolean | undefined;
  };
}

export interface LoggingConfig {
  level: LogLevel;
  enableFileOutput: boolean;
  enableConsoleOutput: boolean;
  enableBrowserStorage: boolean;
  maxLogFileSize: number; // KB
  maxLogAge: number; // days
  categories: {
    [key in LogCategory]: {
      level: LogLevel;
      enabled: boolean;
    };
  };
}

// Production logging configuration
const PRODUCTION_CONFIG: LoggingConfig = {
  level: 'warn',
  enableFileOutput: true,
  enableConsoleOutput: false,
  enableBrowserStorage: false,
  maxLogFileSize: 5000, // 5MB
  maxLogAge: 7, // 1 week
  categories: {
    app: { level: 'warn', enabled: true },
    render: { level: 'error', enabled: true },
    game: { level: 'warn', enabled: true },
    performance: { level: 'warn', enabled: true },
    network: { level: 'error', enabled: true },
    storage: { level: 'error', enabled: true },
    ui: { level: 'error', enabled: true },
    shader: { level: 'error', enabled: true },
    streaming: { level: 'error', enabled: true },
  },
};

// Development logging configuration
const DEVELOPMENT_CONFIG: LoggingConfig = {
  level: 'info',
  enableFileOutput: true,
  enableConsoleOutput: true,
  enableBrowserStorage: true,
  maxLogFileSize: 10000, // 10MB
  maxLogAge: 3, // 3 days
  categories: {
    app: { level: 'info', enabled: true },
    render: { level: 'warn', enabled: true },
    game: { level: 'info', enabled: true },
    performance: { level: 'warn', enabled: true },
    network: { level: 'info', enabled: true },
    storage: { level: 'info', enabled: true },
    ui: { level: 'info', enabled: true },
    shader: { level: 'warn', enabled: true },
    streaming: { level: 'warn', enabled: true },
  },
};

// Testing logging configuration
const TESTING_CONFIG: LoggingConfig = {
  level: 'error',
  enableFileOutput: false,
  enableConsoleOutput: false,
  enableBrowserStorage: false,
  maxLogFileSize: 1000, // 1MB
  maxLogAge: 1, // 1 day
  categories: {
    app: { level: 'error', enabled: true },
    render: { level: 'error', enabled: false },
    game: { level: 'error', enabled: true },
    performance: { level: 'error', enabled: false },
    network: { level: 'error', enabled: true },
    storage: { level: 'error', enabled: true },
    ui: { level: 'error', enabled: false },
    shader: { level: 'error', enabled: false },
    streaming: { level: 'error', enabled: true },
  },
};

/**
 * Get logging configuration based on environment
 */
export const getLoggingConfig = (): LoggingConfig => {
  const env = (import.meta as ViteImportMeta)?.env?.MODE ?? 'development';

  switch (env) {
    case 'production':
      return PRODUCTION_CONFIG;
    case 'test':
      return TESTING_CONFIG;
    case 'development':
    default:
      return DEVELOPMENT_CONFIG;
  }
};

/**
 * Check if logging is enabled for a specific category and level
 */
export const isLoggingEnabled = (
  category: LogCategory,
  level: LogLevel
): boolean => {
  const config = getLoggingConfig();

  // Check if category is enabled
  if (!config.categories[category].enabled) {
    return false;
  }

  // Check log level priority
  const levelPriority = {
    verbose: 0,
    debug: 1,
    info: 2,
    warn: 3,
    error: 4,
  };

  const categoryLevel = config.categories[category].level;
  return levelPriority[level] >= levelPriority[categoryLevel];
};

/**
 * Get effective log level for a category
 */
export const getEffectiveLogLevel = (category: LogCategory): LogLevel => {
  const config = getLoggingConfig();
  return config.categories[category].level;
};

/**
 * Environment-specific logging utilities
 */
export const LoggingUtils = {
  /**
   * Check if we're in development mode
   */
  isDevelopment: (): boolean => {
    return (
      ((import.meta as ViteImportMeta)?.env?.MODE ?? 'development') ===
      'development'
    );
  },

  /**
   * Check if we're in production mode
   */
  isProduction: (): boolean => {
    return (
      ((import.meta as ViteImportMeta)?.env?.MODE ?? 'development') ===
      'production'
    );
  },

  /**
   * Check if we're in test mode
   */
  isTest: (): boolean => {
    return (
      ((import.meta as ViteImportMeta)?.env?.MODE ?? 'development') === 'test'
    );
  },

  /**
   * Get current environment
   */
  getEnvironment: (): string => {
    return (import.meta as ViteImportMeta)?.env?.MODE ?? 'development';
  },

  /**
   * Create a performance-aware logger that only logs in development
   */
  createPerformanceLogger: <T extends (...args: unknown[]) => unknown>(
    category: LogCategory,
    fn: T
  ): T => {
    if (!LoggingUtils.isDevelopment()) {
      return fn;
    }

    return ((...args: Parameters<T>) => {
      const start = performance.now();
      const result = fn(...args) as ReturnType<T>;
      const duration = performance.now() - start;

      if (duration > 10) {
        // Only log if operation takes more than 10ms
        // Using conditional logging for development environment
        if (
          ((import.meta as ViteImportMeta)?.env?.MODE ?? 'development') ===
          'development'
        ) {
          // eslint-disable-next-line no-console
          console.debug(
            `[${category}] Performance: ${fn.name} took ${duration.toFixed(2)}ms`
          );
        }
      }

      return result;
    }) as T;
  },

  /**
   * Conditionally enable features based on logging level
   */
  shouldEnableDebugFeatures: (): boolean => {
    const config = getLoggingConfig();
    return config.level === 'debug' || config.level === 'verbose';
  },

  /**
   * Get logging configuration as JSON for debugging
   */
  getConfigSnapshot() {
    return {
      config: getLoggingConfig(),
      environment: this.getEnvironment(),
      timestamp: new Date().toISOString(),
    };
  },
};

/**
 * Runtime configuration overrides (useful for testing or debugging)
 */
export class LoggingConfigManager {
  private static overrides: Partial<LoggingConfig> = {};

  /**
   * Override logging configuration at runtime
   */
  static setOverride(overrides: Partial<LoggingConfig>): void {
    this.overrides = { ...this.overrides, ...overrides };
  }

  /**
   * Override category-specific configuration
   */
  static setCategoryOverride(
    category: LogCategory,
    config: Partial<LoggingConfig['categories'][LogCategory]>
  ): void {
    this.overrides.categories ??= {} as LoggingConfig['categories'];
    this.overrides.categories[category] = {
      ...getLoggingConfig().categories[category],
      ...config,
    };
  }

  /**
   * Clear all overrides
   */
  static clearOverrides(): void {
    this.overrides = {};
  }

  /**
   * Get current overrides
   */
  static getOverrides(): Partial<LoggingConfig> {
    return { ...this.overrides };
  }

  /**
   * Apply overrides to base configuration
   */
  static getEffectiveConfig(): LoggingConfig {
    const baseConfig = getLoggingConfig();
    return {
      ...baseConfig,
      ...this.overrides,
      categories: {
        ...baseConfig.categories,
        ...(this.overrides.categories ?? {}),
      },
    };
  }
}

const loggingModule = {
  getLoggingConfig,
  isLoggingEnabled,
  getEffectiveLogLevel,
  LoggingUtils,
  LoggingConfigManager,
};

export default loggingModule;
