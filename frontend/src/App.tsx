import { invoke } from '@tauri-apps/api/core';
import { useCallback, useEffect, useState } from 'react';

import GameCanvas from '@/components/game/GameCanvas';
import GameUI from '@/components/ui/game-ui';
import LoadingScreen from '@/components/ui/LoadingScreen';
import { useLogger, usePerformanceLogger } from '@/hooks/use-logger';
import { saveThumbnailService } from '@/services/save-thumbnails';
import { useGameStore } from '@/stores/game-store';

// Types
interface GameState {
  turn: number;
  player_name: string;
  civilization: string;
  is_paused: boolean;
}

const App = () => {
  const [isInitialized, setIsInitialized] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const { setGameState, isLoading, setLoading } = useGameStore();

  // Initialize logging for the main app
  const logger = useLogger('app', 'App');
  const performanceLogger = usePerformanceLogger('app', 'App');

  const initializeGame = useCallback(async () => {
    const timer = performanceLogger.startTimer('game-initialization');

    try {
      setLoading(true);
      logger.info('🎮 GAME INIT: Starting game initialization...');
      logger.info('Starting game initialization', {
        playerName: 'Player',
        civilization: 'Ancient Empire',
      });

      // Greet the user
      logger.info('🤝 BACKEND: Attempting to connect to backend...');

      // Check if we're running in Tauri environment
      if (typeof window !== 'undefined' && '__TAURI__' in window) {
        const greeting = await invoke<string>('greet', { name: 'Player' });
        logger.info('✅ BACKEND: Connection successful', { greeting });
        logger.info('Game greeting received', { greeting });
      } else {
        logger.warn(
          '⚠️ BROWSER: Running in browser mode - backend connection skipped'
        );
        logger.warn('Running in browser mode, backend commands not available', {
          environment: 'browser',
          tauriAvailable: '__TAURI__' in window,
        });
      }

      // Initialize game state
      let initialState: GameState;

      if (typeof window !== 'undefined' && '__TAURI__' in window) {
        initialState = await invoke<GameState>('initialize_game', {
          playerName: 'Player',
          civilization: 'Ancient Empire',
        });
        logger.info('✅ BACKEND: Game state initialized from backend');
      } else {
        // Browser fallback - create mock initial state
        initialState = {
          turn: 1,
          player_name: 'Player',
          civilization: 'Ancient Empire',
          is_paused: false,
        };
        logger.warn('⚠️ BROWSER: Using mock game state for browser testing');
        logger.warn('Using mock game state in browser mode');
      }

      setGameState(initialState);
      setIsInitialized(true);

      timer.end('Game initialization completed successfully', {
        turn: initialState.turn,
        playerName: initialState.player_name,
        civilization: initialState.civilization,
      });
    } catch (err) {
      timer.end('Game initialization failed');
      console.error('❌ GAME INIT: Initialization failed:', err);
      console.error(
        '🔥 BACKEND: Could not connect to backend or backend error occurred'
      );
      logger.error('🔧 DEBUG: Error details', err as Error, {
        message: (err as Error).message,
        stack: (err as Error).stack,
        type: typeof err,
      });
      logger.error('Failed to initialize game', err as Error, {
        playerName: 'Player',
        civilization: 'Ancient Empire',
      });
      setError(err as string);
    } finally {
      setLoading(false);
    }
  }, [setGameState, setLoading, logger, performanceLogger]);

  // Initialize the game on mount - remove function from dependencies to prevent infinite loop
  useEffect(() => {
    if (!isInitialized) {
      void initializeGame();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isInitialized]); // Only depend on isInitialized, not the function

  // Log app lifecycle and state changes
  useEffect(() => {
    logger.info('Manifest app mounted', {
      environment: import.meta.env.MODE ?? 'development',
      timestamp: new Date().toISOString(),
    });

    // Log when app unmounts
    return () => {
      logger.info('Manifest app unmounting');
    };
  }, [logger]);

  // Log loading state changes
  useEffect(() => {
    logger.debug('Loading state changed', {
      isLoading,
      isInitialized,
      hasError: !!error,
    });
  }, [isLoading, isInitialized, error, logger]);

  // Log initialization state
  useEffect(() => {
    if (isInitialized) {
      logger.info('Game successfully initialized and ready');
    }
  }, [isInitialized, logger]);

  // Log errors
  useEffect(() => {
    if (error) {
      logger.error('App-level error occurred', new Error(error), {
        errorMessage: error,
        isInitialized,
        isLoading,
      });
    }
  }, [error, isInitialized, isLoading, logger]);

  const handleSaveGame = async () => {
    const timer = performanceLogger.startTimer('save-game');
    const saveName = `manifest_save_${Date.now()}`;

    try {
      logger.info('Starting game save operation', { saveName });

      // First, save the game state
      const result = await invoke<string>('save_game', { saveName });
      logger.info('Game state saved successfully', { saveName, result });

      // Then, generate and save thumbnail
      try {
        await saveThumbnailService.saveThumbnailWithSave(saveName);
        logger.info('Thumbnail saved successfully', { saveName });

        timer.end('Save operation completed successfully', { saveName });
      } catch (thumbnailError) {
        logger.warn('Failed to save thumbnail (game still saved)', {
          saveName,
          error:
            thumbnailError instanceof Error
              ? thumbnailError.message
              : String(thumbnailError),
        });
        timer.end('Save operation completed (thumbnail failed)', { saveName });
      }
    } catch (err) {
      timer.end('Save operation failed');
      logger.error('Failed to save game', err as Error, { saveName });
    }
  };

  const handleLoadGame = async (saveName: string) => {
    const timer = performanceLogger.startTimer('load-game');

    try {
      logger.info('Starting game load operation', { saveName });

      const loadedState = await invoke<GameState>('load_game', { saveName });
      setGameState(loadedState);

      timer.end('Load operation completed successfully', {
        saveName,
        turn: loadedState.turn,
        playerName: loadedState.player_name,
        civilization: loadedState.civilization,
      });

      logger.info('Game loaded successfully', {
        saveName,
        gameState: {
          turn: loadedState.turn,
          playerName: loadedState.player_name,
          civilization: loadedState.civilization,
          isPaused: loadedState.is_paused,
        },
      });
    } catch (err) {
      timer.end('Load operation failed');
      logger.error('Failed to load game', err as Error, { saveName });
    }
  };

  // Show loading screen while initializing
  if (isLoading || !isInitialized) {
    return <LoadingScreen message='Initializing Manifest...' />;
  }

  // Show error screen if initialization failed
  if (error) {
    return (
      <div className='error-screen'>
        <h1>Failed to Initialize Manifest</h1>
        <p>{error}</p>
        <button onClick={() => void initializeGame()}>Retry</button>
      </div>
    );
  }

  return (
    <div className='app'>
      <div className='game-container'>
        {/* 3D Game Canvas */}
        <GameCanvas />

        {/* Game UI Overlay */}
        <GameUI
          onSave={() => void handleSaveGame()}
          onLoad={saveName => void handleLoadGame(saveName)}
        />
      </div>
    </div>
  );
};

export default App;
