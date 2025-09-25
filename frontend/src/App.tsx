import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
// SCSS styles are imported globally in main.tsx

// Import game components
import GameCanvas from '@/components/game/GameCanvas'
import GameUI from '@/components/ui/GameUI'
import LoadingScreen from '@/components/ui/LoadingScreen'

// Import stores
import { useGameStore } from '@/stores/gameStore'

// Types
interface GameState {
  turn: number;
  player_name: string;
  civilization: string;
  is_paused: boolean;
}

function App() {
  const [isInitialized, setIsInitialized] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const { setGameState, isLoading, setLoading } = useGameStore();

  // Initialize the game on mount
  useEffect(() => {
    initializeGame();
  }, []);

  const initializeGame = async () => {
    try {
      setLoading(true);
      
      // Greet the user
      const greeting = await invoke<string>('greet', { name: 'Player' });
      console.log(greeting);
      
      // Initialize game state
      const initialState = await invoke<GameState>('initialize_game', {
        playerName: 'Player',
        civilization: 'Ancient Empire'
      });
      
      setGameState(initialState);
      setIsInitialized(true);
      
    } catch (err) {
      console.error('Failed to initialize game:', err);
      setError(err as string);
    } finally {
      setLoading(false);
    }
  };

  const handleSaveGame = async () => {
    try {
      const result = await invoke<string>('save_game', { 
        saveName: `manifest_save_${Date.now()}` 
      });
      console.log('Save result:', result);
    } catch (err) {
      console.error('Failed to save game:', err);
    }
  };

  const handleLoadGame = async (saveName: string) => {
    try {
      const loadedState = await invoke<GameState>('load_game', { saveName });
      setGameState(loadedState);
    } catch (err) {
      console.error('Failed to load game:', err);
    }
  };

  // Show loading screen while initializing
  if (isLoading || !isInitialized) {
    return <LoadingScreen message="Initializing Manifest..." />;
  }

  // Show error screen if initialization failed
  if (error) {
    return (
      <div className="error-screen">
        <h1>Failed to Initialize Manifest</h1>
        <p>{error}</p>
        <button onClick={initializeGame}>Retry</button>
      </div>
    );
  }

  return (
    <div className="app">
      <div className="game-container">
        {/* 3D Game Canvas */}
        <GameCanvas />
        
        {/* Game UI Overlay */}
        <GameUI 
          onSave={handleSaveGame}
          onLoad={handleLoadGame}
        />
      </div>
    </div>
  );
}

export default App;
