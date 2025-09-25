import React from 'react';

import { useGameStore } from '@/stores/game-store';

interface GameUIProps {
  onSave: () => void;
  onLoad: (saveName: string) => void;
}

const GameUI: React.FC<GameUIProps> = ({ onSave, onLoad }) => {
  const { gameState } = useGameStore();

  if (!gameState) {
    return null;
  }

  return (
    <div className='game-ui'>
      {/* Top HUD */}
      <div className='top-hud'>
        <div className='game-info'>
          <div className='info-item'>
            <span className='label'>Turn:</span>
            <span className='value'>{gameState.turn}</span>
          </div>
          <div className='info-item'>
            <span className='label'>Civilization:</span>
            <span className='value'>{gameState.civilization}</span>
          </div>
          <div className='info-item'>
            <span className='label'>Leader:</span>
            <span className='value'>{gameState.player_name}</span>
          </div>
        </div>

        <div className='game-controls'>
          <button className='ui-button' onClick={onSave}>
            Save Game
          </button>
          <button className='ui-button' onClick={() => onLoad('test_save')}>
            Load Game
          </button>
          <button
            className='ui-button pause-button'
            onClick={() => console.warn('Pause toggle')}
          >
            {gameState.is_paused ? 'Resume' : 'Pause'}
          </button>
        </div>
      </div>

      {/* Bottom HUD */}
      <div className='bottom-hud'>
        <div className='action-panel'>
          <button className='action-button'>Build City</button>
          <button className='action-button'>Train Unit</button>
          <button className='action-button'>Research Tech</button>
          <button className='action-button'>Diplomacy</button>
        </div>
      </div>

      <style>{`
        .game-ui {
          position: absolute;
          top: 0;
          left: 0;
          width: 100%;
          height: 100%;
          pointer-events: none;
          z-index: 100;
          font-family: 'Inter', system-ui, sans-serif;
        }

        .top-hud {
          position: absolute;
          top: 20px;
          left: 20px;
          right: 20px;
          display: flex;
          justify-content: space-between;
          align-items: center;
          pointer-events: auto;
        }

        .game-info {
          display: flex;
          gap: 2rem;
          background: rgba(0, 0, 0, 0.7);
          backdrop-filter: blur(10px);
          padding: 1rem 1.5rem;
          border-radius: 12px;
          border: 1px solid rgba(255, 255, 255, 0.1);
        }

        .info-item {
          display: flex;
          flex-direction: column;
          align-items: center;
          gap: 0.25rem;
        }

        .label {
          font-size: 0.8rem;
          color: rgba(255, 255, 255, 0.7);
          text-transform: uppercase;
          letter-spacing: 0.05em;
        }

        .value {
          font-size: 1.1rem;
          color: white;
          font-weight: 600;
        }

        .game-controls {
          display: flex;
          gap: 0.75rem;
        }

        .ui-button {
          background: rgba(33, 150, 243, 0.9);
          color: white;
          border: none;
          padding: 0.75rem 1.5rem;
          border-radius: 8px;
          font-size: 0.9rem;
          font-weight: 500;
          cursor: pointer;
          transition: all 0.2s ease;
          backdrop-filter: blur(10px);
        }

        .ui-button:hover {
          background: rgba(33, 150, 243, 1);
          transform: translateY(-1px);
        }

        .pause-button {
          background: rgba(255, 152, 0, 0.9);
        }

        .pause-button:hover {
          background: rgba(255, 152, 0, 1);
        }

        .bottom-hud {
          position: absolute;
          bottom: 20px;
          left: 50%;
          transform: translateX(-50%);
          pointer-events: auto;
        }

        .action-panel {
          display: flex;
          gap: 0.5rem;
          background: rgba(0, 0, 0, 0.7);
          backdrop-filter: blur(10px);
          padding: 1rem;
          border-radius: 12px;
          border: 1px solid rgba(255, 255, 255, 0.1);
        }

        .action-button {
          background: rgba(255, 255, 255, 0.1);
          color: white;
          border: 1px solid rgba(255, 255, 255, 0.2);
          padding: 0.75rem 1.25rem;
          border-radius: 8px;
          font-size: 0.9rem;
          cursor: pointer;
          transition: all 0.2s ease;
          backdrop-filter: blur(5px);
        }

        .action-button:hover {
          background: rgba(255, 255, 255, 0.2);
          border-color: rgba(255, 255, 255, 0.4);
          transform: translateY(-1px);
        }
      `}</style>
    </div>
  );
};

export default GameUI;
