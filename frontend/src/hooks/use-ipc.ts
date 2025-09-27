/**
 * Convenient hooks for using the sophisticated IPC system
 * Wraps the global IPC instance for easy component usage
 */

import { useCallback } from 'react';

import { getGlobalIPC } from '@/utils/ipc';
import type {
  CommandInput,
  CommandName,
  CommandOutput,
} from '@/utils/ipc/schemas';

/**
 * Hook for executing IPC commands with the sophisticated system
 * Includes validation, error handling, progress tracking, and notifications
 */
export const useIPCCommand = () => {
  const executeCommand = useCallback(
    async <T extends CommandName>(
      name: T,
      input: CommandInput<T>,
      options: {
        priority?: 'low' | 'normal' | 'high';
        timeout?: number;
        retries?: number;
        validate?: boolean;
      } = {}
    ): Promise<CommandOutput<T>> => {
      const ipc = getGlobalIPC();

      const result = await ipc.service.command(name, input, options);
      // eslint-disable-next-line @typescript-eslint/no-unsafe-return
      return result as CommandOutput<T>;
    },
    []
  );

  return { executeCommand };
};

/**
 * Hook for common game commands with proper typing
 */
export const useGameCommands = () => {
  const { executeCommand } = useIPCCommand();

  const greet = useCallback(
    async (name: string) => {
      return await executeCommand('greet', { name });
    },
    [executeCommand]
  );

  const initializeGame = useCallback(
    async (playerName: string, civilization: string) => {
      return await executeCommand('initialize_game', {
        playerName,
        civilization,
      });
    },
    [executeCommand]
  );

  const getGameState = useCallback(async () => {
    return await executeCommand('get_game_state', {});
  }, [executeCommand]);

  const saveGame = useCallback(
    async (saveName: string) => {
      return await executeCommand('save_game', { saveName });
    },
    [executeCommand]
  );

  const loadGame = useCallback(
    async (saveName: string) => {
      return await executeCommand('load_game', { saveName });
    },
    [executeCommand]
  );

  const listSaves = useCallback(async () => {
    return await executeCommand('list_saves', {});
  }, [executeCommand]);

  return {
    greet,
    initializeGame,
    getGameState,
    saveGame,
    loadGame,
    listSaves,
  };
};

/**
 * Hook for tile streaming commands
 */
export const useTileCommands = () => {
  const { executeCommand } = useIPCCommand();

  const streamTiles = useCallback(
    async (request: CommandInput<'stream_tiles'>['request']) => {
      return await executeCommand('stream_tiles', { request });
    },
    [executeCommand]
  );

  const getTile = useCallback(
    async (tileId: number) => {
      // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
      const result = await executeCommand('get_tile', { tileId });
      // eslint-disable-next-line @typescript-eslint/no-unsafe-return
      return result;
    },
    [executeCommand]
  );

  const getTileUpdates = useCallback(
    async (tileIds: number[], lastUpdateTime: number) => {
      return await executeCommand('get_tile_updates', {
        tileIds,
        lastUpdateTime,
      });
    },
    [executeCommand]
  );

  return {
    streamTiles,
    getTile,
    getTileUpdates,
  };
};

/**
 * Hook for save thumbnail commands
 */
export const useThumbnailCommands = () => {
  const { executeCommand } = useIPCCommand();

  const saveThumbnailMetadata = useCallback(
    async (
      saveName: string,
      thumbnailData: CommandInput<'save_thumbnail_metadata'>['thumbnailData']
    ) => {
      return await executeCommand('save_thumbnail_metadata', {
        saveName,
        thumbnailData,
      });
    },
    [executeCommand]
  );

  const loadThumbnailMetadata = useCallback(
    async (saveName: string) => {
      return await executeCommand('load_thumbnail_metadata', { saveName });
    },
    [executeCommand]
  );

  return {
    saveThumbnailMetadata,
    loadThumbnailMetadata,
  };
};

/**
 * Hook for debug commands
 */
export const useDebugCommands = () => {
  const { executeCommand } = useIPCCommand();

  const healthCheck = useCallback(async () => {
    return await executeCommand('health_check', {});
  }, [executeCommand]);

  const getSchedulerMetrics = useCallback(async () => {
    return await executeCommand('get_scheduler_metrics', {});
  }, [executeCommand]);

  return {
    healthCheck,
    getSchedulerMetrics,
  };
};

/**
 * Hook for IPC system metrics and status
 */
export const useIPCStatus = () => {
  const getIPCHistory = useCallback(() => {
    const ipc = getGlobalIPC();
    return ipc.history.getEntries();
  }, []);

  const getIPCMetrics = useCallback(() => {
    const ipc = getGlobalIPC();
    return ipc.service.getMetrics();
  }, []);

  const clearIPCHistory = useCallback(() => {
    const ipc = getGlobalIPC();
    ipc.history.clear();
  }, []);

  return {
    getIPCHistory,
    getIPCMetrics,
    clearIPCHistory,
  };
};
