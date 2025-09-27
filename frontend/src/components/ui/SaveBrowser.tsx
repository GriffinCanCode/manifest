/**
 * Save Browser Component
 *
 * Provides visual browsing of save files with thumbnails, metadata, and filtering.
 * Integrates with existing save system and thumbnail service.
 */

import { invoke } from '@tauri-apps/api/core';
import { useCallback, useEffect, useState } from 'react';

import { useGameCommands } from '@/hooks/use-ipc';
import { saveThumbnailService } from '@/services/save-thumbnails';
import type { SaveInfo } from '@/utils/ipc/schemas';

// Note: SaveMetadata interface was removed as we now use the schema-defined SaveInfo type

// SaveThumbnailMetadata interface removed - not needed with current schema

interface SaveBrowserProps {
  onSaveSelect: (saveName: string) => void;
  onClose: () => void;
  isOpen: boolean;
}

const SaveBrowser: React.FC<SaveBrowserProps> = ({
  onSaveSelect,
  onClose,
  isOpen,
}) => {
  const [saves, setSaves] = useState<SaveInfo[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [searchTerm, setSearchTerm] = useState('');
  const [sortBy, setSortBy] = useState<'name' | 'timestamp' | 'playtime'>(
    'timestamp'
  );

  // Game commands hook for sophisticated IPC
  const gameCommands = useGameCommands();

  const loadSaves = useCallback(async () => {
    if (!isOpen) return;

    setIsLoading(true);
    setError(null);

    try {
      let savesList: SaveInfo[];

      // Try to use sophisticated IPC system first
      try {
        savesList = await gameCommands.listSaves();
        console.log('✅ Save list loaded via sophisticated IPC system');
      } catch (ipcError) {
        // Fall back to direct invoke
        console.warn(
          '⚠️ Falling back to direct invoke for save list:',
          ipcError
        );
        savesList = await invoke<SaveInfo[]>('list_saves');
        console.log('⚠️ Save list loaded via direct invoke fallback');
      }

      setSaves(savesList);
    } catch (err) {
      setError(`Failed to load saves: ${String(err)}`);
      console.error('Failed to load saves:', err);
    } finally {
      setIsLoading(false);
    }
  }, [isOpen, gameCommands]);

  useEffect(() => {
    if (isOpen) {
      void loadSaves();
    }
  }, [isOpen, loadSaves]);

  const filteredSaves = saves
    .filter(
      save =>
        save.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
        save.civilization.toLowerCase().includes(searchTerm.toLowerCase())
    )
    .sort((a, b) => {
      switch (sortBy) {
        case 'name':
          return a.name.localeCompare(b.name);
        case 'timestamp':
          // Use modified_at timestamp (convert string to number for comparison)
          const aTime = new Date(a.modified_at).getTime();
          const bTime = new Date(b.modified_at).getTime();
          return bTime - aTime; // Most recent first
        case 'playtime':
          // Use turn number as a proxy for playtime since it's not available
          return b.turn - a.turn; // Higher turn count first
        default:
          return 0;
      }
    });

  if (!isOpen) return null;

  return (
    <div className='save-browser-overlay'>
      <div className='save-browser'>
        {/* Header */}
        <div className='save-browser-header'>
          <h2>Load Game</h2>
          <button className='close-button' onClick={onClose}>
            ✕
          </button>
        </div>

        {/* Controls */}
        <div className='save-browser-controls'>
          <div className='search-section'>
            <input
              type='text'
              placeholder='Search saves...'
              value={searchTerm}
              onChange={e => setSearchTerm(e.target.value)}
              className='search-input'
            />
          </div>

          <div className='sort-section'>
            <label htmlFor='sort-select'>Sort by:</label>
            <select
              id='sort-select'
              value={sortBy}
              onChange={e =>
                setSortBy(e.target.value as 'name' | 'timestamp' | 'playtime')
              }
              className='sort-select'
            >
              <option value='timestamp'>Date</option>
              <option value='name'>Name</option>
              <option value='playtime'>Playtime</option>
            </select>
          </div>
        </div>

        {/* Content */}
        <div className='save-browser-content'>
          {isLoading ? (
            <div className='loading-state'>Loading saves...</div>
          ) : error ? (
            <div className='error-state'>
              <p>{error}</p>
              <button onClick={() => void loadSaves()}>Retry</button>
            </div>
          ) : filteredSaves.length === 0 ? (
            <div className='empty-state'>
              {searchTerm ? 'No saves match your search.' : 'No saves found.'}
            </div>
          ) : (
            <div className='saves-grid'>
              {filteredSaves.map(save => (
                <SaveCard
                  key={save.name}
                  save={save}
                  onSelect={() => onSaveSelect(save.name)}
                />
              ))}
            </div>
          )}
        </div>
      </div>

      <style>{`
        .save-browser-overlay {
          position: fixed;
          top: 0;
          left: 0;
          right: 0;
          bottom: 0;
          background: rgba(0, 0, 0, 0.8);
          backdrop-filter: blur(4px);
          z-index: 1000;
          display: flex;
          align-items: center;
          justify-content: center;
          padding: 2rem;
        }

        .save-browser {
          background: linear-gradient(145deg, #1a1a2e, #16213e);
          border-radius: 16px;
          border: 1px solid rgba(255, 255, 255, 0.1);
          width: 90vw;
          max-width: 1200px;
          height: 80vh;
          display: flex;
          flex-direction: column;
          overflow: hidden;
          box-shadow: 0 20px 40px rgba(0, 0, 0, 0.3);
        }

        .save-browser-header {
          display: flex;
          justify-content: space-between;
          align-items: center;
          padding: 1.5rem 2rem;
          border-bottom: 1px solid rgba(255, 255, 255, 0.1);
          background: rgba(255, 255, 255, 0.05);
        }

        .save-browser-header h2 {
          color: white;
          font-size: 1.5rem;
          font-weight: 600;
          margin: 0;
        }

        .close-button {
          background: rgba(255, 255, 255, 0.1);
          border: 1px solid rgba(255, 255, 255, 0.2);
          color: white;
          width: 32px;
          height: 32px;
          border-radius: 50%;
          display: flex;
          align-items: center;
          justify-content: center;
          cursor: pointer;
          transition: all 0.2s ease;
        }

        .close-button:hover {
          background: rgba(255, 255, 255, 0.2);
          transform: scale(1.1);
        }

        .save-browser-controls {
          display: flex;
          gap: 2rem;
          padding: 1rem 2rem;
          border-bottom: 1px solid rgba(255, 255, 255, 0.1);
          align-items: center;
        }

        .search-section {
          flex: 1;
        }

        .search-input {
          width: 100%;
          padding: 0.75rem 1rem;
          background: rgba(255, 255, 255, 0.1);
          border: 1px solid rgba(255, 255, 255, 0.2);
          border-radius: 8px;
          color: white;
          font-size: 0.9rem;
        }

        .search-input::placeholder {
          color: rgba(255, 255, 255, 0.5);
        }

        .sort-section {
          display: flex;
          gap: 0.5rem;
          align-items: center;
          color: rgba(255, 255, 255, 0.8);
          font-size: 0.9rem;
        }

        .sort-select {
          background: rgba(255, 255, 255, 0.1);
          border: 1px solid rgba(255, 255, 255, 0.2);
          border-radius: 6px;
          color: white;
          padding: 0.5rem;
        }

        .save-browser-content {
          flex: 1;
          padding: 1rem 2rem 2rem;
          overflow-y: auto;
        }

        .saves-grid {
          display: grid;
          grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
          gap: 1rem;
        }

        .loading-state,
        .error-state,
        .empty-state {
          display: flex;
          flex-direction: column;
          align-items: center;
          justify-content: center;
          height: 200px;
          color: rgba(255, 255, 255, 0.7);
          text-align: center;
        }

        .error-state button {
          margin-top: 1rem;
          padding: 0.5rem 1rem;
          background: rgba(33, 150, 243, 0.8);
          color: white;
          border: none;
          border-radius: 6px;
          cursor: pointer;
        }
      `}</style>
    </div>
  );
};

interface SaveCardProps {
  save: SaveInfo;
  onSelect: () => void;
}

const SaveCard: React.FC<SaveCardProps> = ({ save, onSelect }) => {
  const [thumbnail, setThumbnail] = useState<string | null>(null);
  const [isLoadingThumbnail, setIsLoadingThumbnail] = useState(true);

  useEffect(() => {
    const loadThumbnail = async () => {
      try {
        // Note: thumbnail metadata is not available in the current schema
        // Skip to loading from thumbnail service

        // Otherwise try to load from thumbnail service
        const thumbnailData = await saveThumbnailService.loadThumbnail(
          save.name
        );
        if (thumbnailData?.thumbnail) {
          setThumbnail(thumbnailData.thumbnail);
        }
      } catch (error) {
        console.warn(`Failed to load thumbnail for ${save.name}:`, error);
      } finally {
        setIsLoadingThumbnail(false);
      }
    };

    void loadThumbnail();
  }, [save.name]);

  const formatTimestamp = (dateString: string): string => {
    return new Date(dateString).toLocaleDateString();
  };

  const formatPlaytime = (turn: number): string => {
    // Use turn number as a proxy for playtime
    return `Turn ${turn}`;
  };

  // formatPlaytimeOriginal function removed - we now use turn numbers instead

  return (
    <div
      className='save-card'
      onClick={onSelect}
      onKeyDown={e => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          onSelect();
        }
      }}
      role='button'
      tabIndex={0}
    >
      <div className='save-card-thumbnail'>
        {isLoadingThumbnail ? (
          <div className='thumbnail-loading'>Loading...</div>
        ) : thumbnail ? (
          <img src={thumbnail} alt={`${save.name} thumbnail`} />
        ) : (
          <div className='thumbnail-placeholder'>
            <div className='placeholder-icon'>🎮</div>
            <div className='placeholder-text'>MANIFEST</div>
          </div>
        )}
      </div>

      <div className='save-card-info'>
        <h3 className='save-name'>{save.name}</h3>
        <div className='save-details'>
          <div className='detail-row'>
            <span className='detail-label'>Civilization:</span>
            <span className='detail-value'>{save.civilization}</span>
          </div>
          <div className='detail-row'>
            <span className='detail-label'>Date:</span>
            <span className='detail-value'>
              {formatTimestamp(save.modified_at)}
            </span>
          </div>
          <div className='detail-row'>
            <span className='detail-label'>Progress:</span>
            <span className='detail-value'>{formatPlaytime(save.turn)}</span>
          </div>
          <div className='detail-row'>
            <span className='detail-label'>Player:</span>
            <span className='detail-value'>{save.player_name}</span>
          </div>
        </div>
      </div>

      <style>{`
        .save-card {
          background: rgba(255, 255, 255, 0.05);
          border: 1px solid rgba(255, 255, 255, 0.1);
          border-radius: 12px;
          overflow: hidden;
          cursor: pointer;
          transition: all 0.2s ease;
        }

        .save-card:hover {
          background: rgba(255, 255, 255, 0.1);
          border-color: rgba(33, 150, 243, 0.5);
          transform: translateY(-2px);
          box-shadow: 0 8px 16px rgba(0, 0, 0, 0.2);
        }

        .save-card-thumbnail {
          width: 100%;
          height: 120px;
          background: #000;
          display: flex;
          align-items: center;
          justify-content: center;
          position: relative;
          overflow: hidden;
        }

        .save-card-thumbnail img {
          width: 100%;
          height: 100%;
          object-fit: cover;
        }

        .thumbnail-loading {
          color: rgba(255, 255, 255, 0.5);
          font-size: 0.8rem;
        }

        .thumbnail-placeholder {
          display: flex;
          flex-direction: column;
          align-items: center;
          gap: 0.5rem;
          color: rgba(255, 255, 255, 0.3);
        }

        .placeholder-icon {
          font-size: 2rem;
        }

        .placeholder-text {
          font-size: 0.8rem;
          font-weight: 600;
          letter-spacing: 0.1em;
        }

        .save-card-info {
          padding: 1rem;
        }

        .save-name {
          color: white;
          font-size: 1.1rem;
          font-weight: 600;
          margin: 0 0 0.75rem 0;
          white-space: nowrap;
          overflow: hidden;
          text-overflow: ellipsis;
        }

        .save-details {
          display: flex;
          flex-direction: column;
          gap: 0.25rem;
        }

        .detail-row {
          display: flex;
          justify-content: space-between;
          align-items: center;
          font-size: 0.8rem;
        }

        .detail-label {
          color: rgba(255, 255, 255, 0.6);
        }

        .detail-value {
          color: rgba(255, 255, 255, 0.9);
          font-weight: 500;
        }
      `}</style>
    </div>
  );
};

export default SaveBrowser;
