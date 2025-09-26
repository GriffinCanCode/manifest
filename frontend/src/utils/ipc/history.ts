/**
 * IPC Command History and Undo/Redo System
 * Tracks command execution and provides undo/redo functionality
 */

import { enablePatches, produce, type Patch } from 'immer';
import sift from 'sift';

import type { CommandInput, CommandName, CommandOutput } from './schemas';

// Enable immer patches for undo/redo
enablePatches();

export interface HistoryEntry<T extends CommandName = CommandName> {
  id: string;
  command: T;
  input: CommandInput<T>;
  output?: CommandOutput<T>;
  timestamp: number;
  duration?: number;
  patches?: Patch[];
  inversePatches?: Patch[];
  metadata?: {
    userId?: string;
    sessionId?: string;
    description?: string;
    tags?: string[];
  };
}

export interface HistoryState {
  entries: HistoryEntry[];
  currentIndex: number;
  maxSize: number;
  canUndo: boolean;
  canRedo: boolean;
}

export interface HistoryConfig {
  maxSize: number;
  enablePatches: boolean;
  persistToStorage: boolean;
  storageKey?: string;
  excludeCommands?: CommandName[];
}

const DEFAULT_CONFIG: HistoryConfig = {
  maxSize: 100,
  enablePatches: true,
  persistToStorage: false,
  storageKey: 'ipc-command-history',
  excludeCommands: ['get_game_state', 'get_tile', 'stream_tiles'], // Read-only commands
};

/**
 * Command history manager with undo/redo functionality
 */
export class CommandHistory {
  private state: HistoryState;
  private config: HistoryConfig;
  private listeners = new Set<(state: HistoryState) => void>();

  constructor(config: Partial<HistoryConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config };
    this.state = {
      entries: [],
      currentIndex: -1,
      maxSize: this.config.maxSize,
      canUndo: false,
      canRedo: false,
    };

    if (this.config.persistToStorage) {
      this.loadFromStorage();
    }
  }

  /**
   * Add a command to history
   */
  addCommand<T extends CommandName>(
    command: T,
    input: CommandInput<T>,
    options: {
      id?: string;
      metadata?: HistoryEntry['metadata'];
      patches?: Patch[];
      inversePatches?: Patch[];
    } = {}
  ): string {
    // Skip excluded commands
    if (this.config.excludeCommands?.includes(command)) {
      return '';
    }

    const id = options.id ?? this.generateId();
    const entry: HistoryEntry<T> = {
      id,
      command,
      input,
      timestamp: Date.now(),
      metadata: options.metadata,
      patches: options.patches,
      inversePatches: options.inversePatches,
    };

    this.state = produce(this.state, draft => {
      // Remove all entries after current index (they become obsolete)
      if (draft.currentIndex < draft.entries.length - 1) {
        draft.entries.splice(draft.currentIndex + 1);
      }

      // Add new entry
      draft.entries.push(entry);
      draft.currentIndex = draft.entries.length - 1;

      // Maintain max size
      if (draft.entries.length > draft.maxSize) {
        const removeCount = draft.entries.length - draft.maxSize;
        draft.entries.splice(0, removeCount);
        draft.currentIndex -= removeCount;
      }

      // Update capabilities
      this.updateCapabilities(draft);
    });

    this.notifyListeners();
    this.saveToStorage();

    return id;
  }

  /**
   * Mark a command as completed with output
   */
  completeCommand<T extends CommandName>(
    id: string,
    output: CommandOutput<T>,
    duration?: number
  ): void {
    this.state = produce(this.state, draft => {
      const entry = draft.entries.find(e => e.id === id);
      if (entry) {
        // Type assertion needed since output type varies by command
        (entry as HistoryEntry<T>).output = output;
        entry.duration = duration;
      }
    });

    this.notifyListeners();
    this.saveToStorage();
  }

  /**
   * Undo the last command
   */
  undo(): HistoryEntry | null {
    if (!this.state.canUndo) {
      return null;
    }

    const entry = this.state.entries[this.state.currentIndex];

    this.state = produce(this.state, draft => {
      draft.currentIndex--;
      this.updateCapabilities(draft);
    });

    this.notifyListeners();
    this.saveToStorage();

    return entry;
  }

  /**
   * Redo the next command
   */
  redo(): HistoryEntry | null {
    if (!this.state.canRedo) {
      return null;
    }

    this.state = produce(this.state, draft => {
      draft.currentIndex++;
      this.updateCapabilities(draft);
    });

    const entry = this.state.entries[this.state.currentIndex];

    this.notifyListeners();
    this.saveToStorage();

    return entry;
  }

  /**
   * Clear all history
   */
  clear(): void {
    this.state = produce(this.state, draft => {
      draft.entries = [];
      draft.currentIndex = -1;
      this.updateCapabilities(draft);
    });

    this.notifyListeners();
    this.saveToStorage();
  }

  /**
   * Get current history state
   */
  getState(): HistoryState {
    return this.state;
  }

  /**
   * Get all entries
   */
  getEntries(): HistoryEntry[] {
    return this.state.entries;
  }

  /**
   * Get entry by ID
   */
  getEntry(id: string): HistoryEntry | null {
    return this.state.entries.find(entry => entry.id === id) ?? null;
  }

  /**
   * Get entries by command name
   */
  getEntriesByCommand(command: CommandName): HistoryEntry[] {
    return this.state.entries.filter(entry => entry.command === command);
  }

  /**
   * Get recent entries (last N)
   */
  getRecentEntries(count: number = 10): HistoryEntry[] {
    return this.state.entries.slice(-count);
  }

  /**
   * Search entries by criteria using sift.js for advanced filtering
   */
  searchEntries(criteria: {
    command?: CommandName;
    dateRange?: { start: number; end: number };
    tags?: string[];
    text?: string;
    // Advanced sift.js query support
    $query?: Record<string, any>;
  }): HistoryEntry[] {
    // If advanced query is provided, use it directly
    if (criteria.$query) {
      return this.state.entries.filter(sift(criteria.$query));
    }

    // Build sift.js query from criteria
    const query: Record<string, any> = {};

    if (criteria.command) {
      query.command = criteria.command;
    }

    if (criteria.dateRange) {
      query.timestamp = {
        $gte: criteria.dateRange.start,
        $lte: criteria.dateRange.end,
      };
    }

    if (criteria.tags?.length) {
      query['metadata.tags'] = { $all: criteria.tags };
    }

    // For text search, we need custom logic since sift doesn't handle
    // searching across nested JSON strings efficiently
    let filteredEntries = this.state.entries;

    // Apply sift.js query first if we have any
    if (Object.keys(query).length > 0) {
      filteredEntries = this.state.entries.filter(sift(query));
    }

    // Then apply text search if needed
    if (criteria.text) {
      const searchText = criteria.text.toLowerCase();
      filteredEntries = filteredEntries.filter(entry => {
        const description = entry.metadata?.description?.toLowerCase() ?? '';
        const inputStr = JSON.stringify(entry.input).toLowerCase();
        return (
          description.includes(searchText) || inputStr.includes(searchText)
        );
      });
    }

    return filteredEntries;
  }

  /**
   * Get undo/redo stack info
   */
  getStackInfo() {
    const undoStack = this.state.entries.slice(0, this.state.currentIndex + 1);
    const redoStack = this.state.entries.slice(this.state.currentIndex + 1);

    return {
      undoCount: undoStack.length,
      redoCount: redoStack.length,
      totalEntries: this.state.entries.length,
      currentIndex: this.state.currentIndex,
      canUndo: this.state.canUndo,
      canRedo: this.state.canRedo,
    };
  }

  /**
   * Export history as JSON
   */
  exportHistory(): string {
    return JSON.stringify(
      {
        entries: this.state.entries,
        config: this.config,
        exportedAt: Date.now(),
      },
      null,
      2
    );
  }

  /**
   * Import history from JSON
   */
  importHistory(jsonData: string): void {
    try {
      const data = JSON.parse(jsonData) as {
        entries?: HistoryEntry[];
        config?: Partial<HistoryConfig>;
        exportedAt?: number;
      };

      if (Array.isArray(data.entries)) {
        this.state = produce(this.state, draft => {
          if (data.entries) {
            draft.entries = data.entries;
            draft.currentIndex = draft.entries.length - 1;
            this.updateCapabilities(draft);
          }
        });

        this.notifyListeners();
        this.saveToStorage();
      }
    } catch (error) {
      console.error('Failed to import command history:', error);
    }
  }

  /**
   * Subscribe to history changes
   */
  subscribe(listener: (state: HistoryState) => void): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  /**
   * Get performance statistics
   */
  getStats() {
    const { entries } = this.state;
    const commandCounts: Record<string, number> = {};
    let totalDuration = 0;
    let completedCommands = 0;

    entries.forEach(entry => {
      commandCounts[entry.command] = (commandCounts[entry.command] || 0) + 1;

      if (entry.duration) {
        totalDuration += entry.duration;
        completedCommands++;
      }
    });

    const averageDuration =
      completedCommands > 0 ? totalDuration / completedCommands : 0;
    const mostUsedCommand = Object.entries(commandCounts).sort(
      ([, a], [, b]) => b - a
    )[0]?.[0];

    return {
      totalEntries: entries.length,
      commandCounts,
      averageDuration,
      totalDuration,
      completedCommands,
      mostUsedCommand,
      oldestEntry: entries[0]?.timestamp,
      newestEntry: entries[entries.length - 1]?.timestamp,
    };
  }

  // Private methods

  private updateCapabilities(draft: HistoryState) {
    draft.canUndo = draft.currentIndex >= 0;
    draft.canRedo = draft.currentIndex < draft.entries.length - 1;
  }

  private generateId(): string {
    return `cmd_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`;
  }

  private notifyListeners() {
    this.listeners.forEach(listener => {
      try {
        listener(this.state);
      } catch (error) {
        console.error('Error in history listener:', error);
      }
    });
  }

  private saveToStorage() {
    if (!this.config.persistToStorage || !this.config.storageKey) return;

    try {
      const data = {
        entries: this.state.entries.slice(-50), // Only keep last 50 in storage
        currentIndex: Math.min(this.state.currentIndex, 49),
        savedAt: Date.now(),
      };

      localStorage.setItem(this.config.storageKey, JSON.stringify(data));
    } catch (error) {
      console.warn('Failed to save command history to storage:', error);
    }
  }

  private loadFromStorage() {
    if (!this.config.persistToStorage || !this.config.storageKey) return;

    try {
      const stored = localStorage.getItem(this.config.storageKey);
      if (!stored) return;

      const data = JSON.parse(stored) as {
        entries?: HistoryEntry[];
        currentIndex?: number;
        savedAt?: number;
      };

      if (Array.isArray(data.entries)) {
        this.state = produce(this.state, draft => {
          if (data.entries) {
            draft.entries = data.entries;
            draft.currentIndex = data.currentIndex ?? draft.entries.length - 1;
            this.updateCapabilities(draft);
          }
        });
      }
    } catch (error) {
      console.warn('Failed to load command history from storage:', error);
    }
  }
}

// Default singleton instance
export const commandHistory = new CommandHistory({
  persistToStorage: true,
  maxSize: 200,
});
