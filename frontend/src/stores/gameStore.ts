import { create } from 'zustand'
import { devtools } from 'zustand/middleware'

interface GameState {
  turn: number
  player_name: string
  civilization: string
  is_paused: boolean
}

interface GameStoreState {
  gameState: GameState | null
  isLoading: boolean
  error: string | null
  
  // Actions
  setGameState: (state: GameState) => void
  setLoading: (loading: boolean) => void
  setError: (error: string | null) => void
  resetGameState: () => void
}

const initialGameState: GameState = {
  turn: 1,
  player_name: 'Player',
  civilization: 'Ancient Empire',
  is_paused: false
}

export const useGameStore = create<GameStoreState>()(
  devtools(
    (set) => ({
      gameState: null,
      isLoading: false,
      error: null,
      
      setGameState: (state: GameState) => 
        set({ gameState: state, error: null }, false, 'setGameState'),
      
      setLoading: (loading: boolean) => 
        set({ isLoading: loading }, false, 'setLoading'),
      
      setError: (error: string | null) => 
        set({ error }, false, 'setError'),
      
      resetGameState: () => 
        set({ gameState: initialGameState, error: null }, false, 'resetGameState'),
    }),
    {
      name: 'manifest-game-store',
    }
  )
)

export type { GameState }
