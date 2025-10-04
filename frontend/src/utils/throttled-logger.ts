/**
 * Throttled logging utility to prevent console spam
 */

interface ThrottledLogger {
  warn: (message: string, ...args: unknown[]) => void;
  log: (message: string, ...args: unknown[]) => void;
  error: (message: string, ...args: unknown[]) => void;
}

class ThrottleManager {
  private static instance: ThrottleManager;
  private lastLogTimes = new Map<string, number>();

  static getInstance(): ThrottleManager {
    if (!ThrottleManager.instance) {
      ThrottleManager.instance = new ThrottleManager();
    }
    return ThrottleManager.instance;
  }

  shouldLog(key: string, throttleMs: number = 60000): boolean {
    const now = Date.now();
    const lastLog = this.lastLogTimes.get(key) ?? 0;

    if (now - lastLog >= throttleMs) {
      this.lastLogTimes.set(key, now);
      return true;
    }

    return false;
  }
}

/**
 * Creates a throttled logger that limits log messages to once per specified interval
 * @param throttleMs Throttle interval in milliseconds (default: 60000ms = 1 minute)
 */
export const createThrottledLogger = (
  throttleMs: number = 60000
): ThrottledLogger => {
  const throttleManager = ThrottleManager.getInstance();

  return {
    warn: (message: string, ...args: unknown[]) => {
      const key = `warn:${message}`;
      if (throttleManager.shouldLog(key, throttleMs)) {
        console.warn(`[THROTTLED] ${message}`, ...args);
      }
    },

    log: (message: string, ...args: unknown[]) => {
      const key = `log:${message}`;
      if (throttleManager.shouldLog(key, throttleMs)) {
        // Using warn instead of log as per linter rules
        console.warn(`[THROTTLED LOG] ${message}`, ...args);
      }
    },

    error: (message: string, ...args: unknown[]) => {
      const key = `error:${message}`;
      if (throttleManager.shouldLog(key, throttleMs)) {
        console.error(`[THROTTLED] ${message}`, ...args);
      }
    },
  };
};

/**
 * Convenience function for throttling a single log message
 * @param key Unique key for this log message
 * @param logFn The console function to use (warn, log, error)
 * @param message The message to log
 * @param args Additional arguments to pass to the log function
 * @param throttleMs Throttle interval in milliseconds (default: 60000ms = 1 minute)
 */
export const throttledLog = (
  key: string,
  logFn: 'warn' | 'log' | 'error',
  message: string,
  args: unknown[] = [],
  throttleMs: number = 60000
): void => {
  const throttleManager = ThrottleManager.getInstance();

  if (throttleManager.shouldLog(key, throttleMs)) {
    if (logFn === 'log') {
      // Using warn instead of log as per linter rules
      console.warn(`[THROTTLED LOG] ${message}`, ...args);
    } else if (logFn === 'warn') {
      console.warn(`[THROTTLED] ${message}`, ...args);
    } else if (logFn === 'error') {
      console.error(`[THROTTLED] ${message}`, ...args);
    }
  }
};
