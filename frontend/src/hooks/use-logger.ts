/**
 * React Hook for Component-Level Logging
 * Provides easy access to structured logging within React components
 */

import {
  createElement,
  Fragment,
  useCallback,
  useEffect,
  useMemo,
} from 'react';

import { createComponentLogger, type LogCategory } from '../services/logger';

// Browser-compatible environment check with proper typing
interface ViteImportMeta extends ImportMeta {
  env: ImportMetaEnv & {
    MODE?: string;
    [key: string]: unknown;
  };
}

/**
 * Hook for component-level logging with automatic component name detection
 */
export const useLogger = (category: LogCategory, componentName?: string) => {
  // Try to get component name from React DevTools or fallback
  const fallbackName = useComponentName();
  const detectedName = componentName ?? fallbackName;

  const logger = useMemo(
    () => createComponentLogger(category, detectedName),
    [category, detectedName]
  );

  // Log component mount/unmount in development
  useEffect(() => {
    if ((import.meta as ViteImportMeta)?.env?.MODE === 'development') {
      logger.debug(`Component ${detectedName} mounted`);

      return () => {
        logger.debug(`Component ${detectedName} unmounted`);
      };
    }
  }, [logger, detectedName]);

  return logger;
};

/**
 * Hook for performance logging with automatic timing
 */
export const usePerformanceLogger = (
  category: LogCategory,
  componentName?: string
) => {
  const logger = useLogger(category, componentName);

  const logPerformance = useCallback(
    (
      operation: string,
      fn: () => void | Promise<void>,
      metadata?: Record<string, unknown>
    ) => {
      const timer = logger.startTimer(operation);

      const result = fn();

      if (result instanceof Promise) {
        return result.finally(() => {
          timer.end(`Async ${operation} completed`, metadata);
        });
      } else {
        timer.end(`${operation} completed`, metadata);
        return result;
      }
    },
    [logger]
  );

  const measureRender = useCallback(
    (renderName: string, _metadata?: Record<string, unknown>) => {
      return logger.startTimer(`render:${renderName}`);
    },
    [logger]
  );

  return {
    ...logger,
    logPerformance,
    measureRender,
  };
};

/**
 * Hook for error boundary logging
 */
export const useErrorLogger = (
  category: LogCategory,
  componentName?: string
) => {
  const logger = useLogger(category, componentName);

  const logError = useCallback(
    (
      error: Error,
      errorInfo?: { componentStack?: string },
      metadata?: Record<string, unknown>
    ) => {
      logger.errorBoundary(error, errorInfo ?? {}, metadata);
    },
    [logger]
  );

  return { ...logger, logError };
};

/**
 * Utility to detect component name (fallback implementation)
 */
const useComponentName = (): string => {
  // In development, try to get from React DevTools
  if ((import.meta as ViteImportMeta)?.env?.MODE === 'development') {
    try {
      const { stack } = new Error();
      if (stack) {
        // Look for React component names in the stack trace
        const lines = stack.split('\n');
        for (const line of lines) {
          // Match function names that start with uppercase (likely React components)
          const match = line.match(/at ([A-Z]\w+)/);
          if (match && match[1] !== 'Error' && match[1] !== 'Object') {
            return match[1];
          }
        }
      }
    } catch {
      // Fallback if stack trace parsing fails
    }
  }

  return 'UnknownComponent';
};

/**
 * Higher-order component for automatic error logging
 */
export const withErrorLogging = <P extends object>(
  WrappedComponent: React.ComponentType<P>,
  category: LogCategory,
  componentName?: string
) => {
  const WithErrorLogging = (props: P) => {
    const { logError } = useErrorLogger(
      category,
      componentName ?? WrappedComponent.name
    );

    const ErrorBoundaryComponent = ({
      children,
    }: {
      children: React.ReactNode;
    }) => {
      useEffect(() => {
        const handleUnhandledRejection = (event: PromiseRejectionEvent) => {
          logError(new Error(String(event.reason)), {
            componentStack: 'Promise rejection',
          });
        };

        window.addEventListener('unhandledrejection', handleUnhandledRejection);

        return () => {
          window.removeEventListener(
            'unhandledrejection',
            handleUnhandledRejection
          );
        };
      }, []);

      return createElement(Fragment, null, children);
    };

    return createElement(
      ErrorBoundaryComponent,
      null,
      createElement(WrappedComponent, props)
    );
  };

  WithErrorLogging.displayName = `withErrorLogging(${componentName ?? WrappedComponent.name})`;

  return WithErrorLogging;
};
