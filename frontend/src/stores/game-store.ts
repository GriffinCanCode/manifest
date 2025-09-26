import { create } from 'zustand';
import { devtools } from 'zustand/middleware';

import { GameLogger } from '../services/logger';

interface GameState {
  turn: number;
  player_name: string;
  civilization: string;
  is_paused: boolean;
}

interface GameStoreState {
  gameState: GameState | null;
  isLoading: boolean;
  error: string | null;

  // Actions
  setGameState: (state: GameState) => void;
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;
  resetGameState: () => void;
}

const initialGameState: GameState = {
  turn: 1,
  player_name: 'Player',
  civilization: 'Ancient Empire',
  is_paused: false,
};

export const useGameStore = create<GameStoreState>()(
  devtools(
    (set, get) => ({
      gameState: null,
      isLoading: false,
      error: null,

      setGameState: (state: GameState) => {
        const previousState = get().gameState;

        GameLogger.info('Game state updated', {
          previousState: previousState
            ? {
                turn: previousState.turn,
                playerName: previousState.player_name,
                civilization: previousState.civilization,
                isPaused: previousState.is_paused,
              }
            : null,
          newState: {
            turn: state.turn,
            playerName: state.player_name,
            civilization: state.civilization,
            isPaused: state.is_paused,
          },
          turnChanged: !previousState || previousState.turn !== state.turn,
          pausedChanged:
            !previousState || previousState.is_paused !== state.is_paused,
        });

        set({ gameState: state, error: null }, false, 'setGameState');
      },

      setLoading: (loading: boolean) => {
        const previousLoading = get().isLoading;

        if (previousLoading !== loading) {
          GameLogger.debug('Loading state changed', {
            from: previousLoading,
            to: loading,
            gameStatePresent: !!get().gameState,
          });
        }

        set({ isLoading: loading }, false, 'setLoading');
      },

      setError: (error: string | null) => {
        const previousError = get().error;

        if (error) {
          GameLogger.error('Game store error occurred', new Error(error), {
            previousError,
            currentGameState: get().gameState
              ? {
                  turn: get().gameState!.turn,
                  playerName: get().gameState!.player_name,
                }
              : null,
            isLoading: get().isLoading,
          });
        } else if (previousError) {
          GameLogger.info('Game store error cleared', { previousError });
        }

        set({ error }, false, 'setError');
      },

      resetGameState: () => {
        const currentState = get().gameState;

        GameLogger.info('Game state reset', {
          previousState: currentState
            ? {
                turn: currentState.turn,
                playerName: currentState.player_name,
                civilization: currentState.civilization,
              }
            : null,
          resetToState: {
            turn: initialGameState.turn,
            playerName: initialGameState.player_name,
            civilization: initialGameState.civilization,
          },
        });

        set(
          { gameState: initialGameState, error: null },
          false,
          'resetGameState'
        );
      },
    }),
    {
      name: 'manifest-game-store',
    }
  )
);

export type { GameState };
