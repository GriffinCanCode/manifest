/**
 * React hooks for IPC communication
 * Integrates with React Query for caching and state synchronization
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useCallback, useEffect, useMemo, useState } from 'react';

import type {
  CommandInput,
  CommandName,
  CommandOutput,
  EventData,
  EventName,
} from './schemas';
import { ipcService } from './service';

export interface UseIPCCommandOptions<T extends CommandName> {
  enabled?: boolean;
  retry?: number;
  retryDelay?: number;
  staleTime?: number;
  gcTime?: number;
  refetchInterval?: number;
  onSuccess?: (data: CommandOutput<T>) => void;
  onError?: (error: Error) => void;
}

export interface UseIPCMutationOptions<T extends CommandName> {
  onSuccess?: (data: CommandOutput<T>, variables: CommandInput<T>) => void;
  onError?: (error: Error, variables: CommandInput<T>) => void;
  onSettled?: (
    data: CommandOutput<T> | undefined,
    error: Error | null,
    variables: CommandInput<T>
  ) => void;
}

/**
 * Hook for executing IPC queries with React Query caching
 */
export const useIPCQuery = <T extends CommandName>(
  name: T,
  input: CommandInput<T>,
  options: UseIPCCommandOptions<T> = {}
) => {
  const queryKey = useMemo(() => ['ipc', name, input], [name, input]);

  const result = useQuery({
    queryKey,
    queryFn: () => ipcService.command(name, input),
    enabled: options.enabled,
    retry: options.retry,
    retryDelay: options.retryDelay,
    staleTime: options.staleTime ?? 5000,
    gcTime: options.gcTime ?? 300000, // 5 minutes
    refetchInterval: options.refetchInterval,
  });

  // Handle success/error callbacks in a useEffect since they're removed from React Query v5
  useEffect(() => {
    if (result.isSuccess && options.onSuccess) {
      options.onSuccess(result.data);
    }
    if (result.isError && options.onError) {
      options.onError(result.error);
    }
  }, [result.isSuccess, result.isError, result.data, result.error, options]);

  return result;
};

/**
 * Hook for executing IPC mutations
 */
export const useIPCMutation = <T extends CommandName>(
  name: T,
  options: UseIPCMutationOptions<T> = {}
) => {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: CommandInput<T>) => ipcService.command(name, input),
    onSuccess: (data, variables) => {
      // Invalidate related queries
      void queryClient.invalidateQueries({ queryKey: ['ipc', name] });
      options.onSuccess?.(data, variables);
    },
    onError: options.onError,
    onSettled: options.onSettled,
  });
};

/**
 * Hook for listening to IPC events
 */
export const useIPCEvent = <T extends EventName>(
  eventName: T,
  handler: (data: EventData<T>) => void,
  deps: React.DependencyList = []
) => {
  useEffect(() => {
    const unsubscribe = ipcService.onEvent(eventName, handler);
    return unsubscribe;
  }, [eventName, handler, ...deps]);
};

/**
 * Hook for batch IPC operations
 */
export const useIPCBatch = () => {
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const executeBatch = useCallback(
    async <T extends CommandName>(
      commands: Array<{ name: T; input: CommandInput<T> }>,
      options: {
        parallel?: boolean;
        failFast?: boolean;
        timeout?: number;
      } = {}
    ) => {
      setIsLoading(true);
      setError(null);

      try {
        const results = await ipcService.batch(commands, options);
        return results;
      } catch (err) {
        const error = err as Error;
        setError(error);
        throw error;
      } finally {
        setIsLoading(false);
      }
    },
    []
  );

  return {
    executeBatch,
    isLoading,
    error,
  };
};

/**
 * Hook for IPC performance monitoring
 */
export const useIPCMetrics = (refreshInterval: number = 1000) => {
  const [metrics, setMetrics] = useState(ipcService.getMetrics());

  useEffect(() => {
    const interval = setInterval(() => {
      setMetrics(ipcService.getMetrics());
    }, refreshInterval);

    return () => clearInterval(interval);
  }, [refreshInterval]);

  return metrics;
};

/**
 * Hook for managing IPC connection state
 */
export const useIPCConnection = () => {
  const [isConnected, setIsConnected] = useState(true);
  const [lastError, setLastError] = useState<Error | null>(null);

  // Note: Connection events are not implemented in the current backend
  // This would be implemented as actual event listeners in production

  useEffect(() => {
    // Connection monitoring would be implemented here
    // For now, we assume connection is stable
  }, []);

  const reconnect = useCallback(async () => {
    try {
      // Test connection with a simple command
      await ipcService.command('greet', { name: 'Test' });
      setIsConnected(true);
      setLastError(null);
    } catch (error) {
      setLastError(error as Error);
      throw error;
    }
  }, []);

  return {
    isConnected,
    lastError,
    reconnect,
  };
};

/**
 * React Query key factory for IPC commands
 */
export const ipcQueryKeys = {
  all: ['ipc'] as const,
  command: (name: CommandName) => ['ipc', name] as const,
  commandWithInput: <T extends CommandName>(name: T, input: CommandInput<T>) =>
    ['ipc', name, input] as const,
  gameState: () => ['ipc', 'get_game_state'] as const,
  saves: () => ['ipc', 'list_saves'] as const,
  tiles: (input: CommandInput<'stream_tiles'>) =>
    ['ipc', 'stream_tiles', input] as const,
  metrics: () => ['ipc', 'get_scheduler_metrics'] as const,
};

/**
 * Hook for specific game state queries (convenience wrapper)
 */
export const useGameState = (
  options: UseIPCCommandOptions<'get_game_state'> = {}
) => {
  return useIPCQuery(
    'get_game_state',
    {},
    {
      ...options,
      refetchInterval: options.refetchInterval ?? 5000, // Refresh every 5 seconds
    }
  );
};

/**
 * Hook for game state mutations
 */
export const useGameActions = () => {
  const queryClient = useQueryClient();

  const initializeGame = useIPCMutation('initialize_game', {
    onSuccess: () => {
      void queryClient.invalidateQueries({
        queryKey: ipcQueryKeys.gameState(),
      });
    },
  });

  const saveGame = useIPCMutation('save_game', {
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ipcQueryKeys.saves() });
    },
  });

  const loadGame = useIPCMutation('load_game', {
    onSuccess: () => {
      void queryClient.invalidateQueries({
        queryKey: ipcQueryKeys.gameState(),
      });
    },
  });

  return {
    initializeGame,
    saveGame,
    loadGame,
  };
};

/**
 * Hook for save file management
 */
export const useSaves = (options: UseIPCCommandOptions<'list_saves'> = {}) => {
  const saves = useIPCQuery(
    'list_saves',
    {},
    {
      ...options,
      staleTime: 10000, // Save list is relatively static
    }
  );

  const loadGame = useIPCMutation('load_game');
  const deleteGame = useCallback((saveName: string) => {
    // TODO: Implement delete command in backend
    console.warn('Delete save not yet implemented:', saveName);
  }, []);

  return {
    saves: saves.data ?? [],
    isLoading: saves.isLoading,
    error: saves.error,
    refetch: saves.refetch,
    loadGame: loadGame.mutateAsync,
    deleteGame,
    isLoadingGame: loadGame.isPending,
  };
};

/**
 * Hook for tile streaming
 */
export const useTileStreaming = (
  request: CommandInput<'stream_tiles'>,
  options: UseIPCCommandOptions<'stream_tiles'> = {}
) => {
  return useIPCQuery('stream_tiles', request, {
    ...options,
    enabled: options.enabled ?? Boolean(request.request.camera_position),
    staleTime: 1000, // Tiles change frequently
    gcTime: 30000, // Keep in cache for 30 seconds
  });
};

/**
 * Hook for development/debug features
 */
export const useIPCDebug = () => {
  const [isEnabled, setIsEnabled] = useState(false);

  const metrics = useIPCMetrics(isEnabled ? 1000 : 0);
  const connection = useIPCConnection();

  const clearQueue = useCallback(() => {
    ipcService.clearQueue();
  }, []);

  const exportMetrics = useCallback(() => {
    const metricsData = ipcService.getMetrics();
    const dataStr = JSON.stringify(metricsData, null, 2);
    const blob = new Blob([dataStr], { type: 'application/json' });
    const url = URL.createObjectURL(blob);

    const a = document.createElement('a');
    a.href = url;
    a.download = `ipc-metrics-${Date.now()}.json`;
    a.click();

    URL.revokeObjectURL(url);
  }, []);

  return {
    isEnabled,
    setIsEnabled,
    metrics,
    connection,
    clearQueue,
    exportMetrics,
  };
};

/**
 * Provider hook for setting up IPC in the app root
 */
export const useIPCProvider = () => {
  useEffect(() => {
    // Setup global error handling
    const handleCommandFailed = (event: EventData<'error_occurred'>) => {
      console.error('IPC Command failed:', event);
    };

    const handlePerformanceWarning = (
      event: EventData<'performance_warning'>
    ) => {
      console.warn('IPC Performance warning:', event);
    };

    const unsubscribeError = ipcService.onEvent(
      'error_occurred',
      handleCommandFailed
    );
    const unsubscribeWarning = ipcService.onEvent(
      'performance_warning',
      handlePerformanceWarning
    );

    return () => {
      unsubscribeError();
      unsubscribeWarning();
    };
  }, []);
};
