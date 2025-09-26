/**
 * Valtio State Synchronization for IPC
 * Provides reactive state synchronization between backend events and frontend
 */

import { proxy, snapshot, subscribe } from 'valtio';
import { subscribeKey } from 'valtio/utils';

import type {
  CommandName,
  CommandOutput,
  EventData,
  EventName,
  GameState,
} from './schemas';

export interface ValtioSyncConfig {
  enabled: boolean;
  autoSync: boolean;
  syncInterval: number;
  persistState: boolean;
  storageKey?: string;
}

interface IPCState {
  gameState: GameState | null;
  lastCommand: {
    name: CommandName;
    timestamp: number;
    output: unknown;
  } | null;
  connectionStatus: 'connected' | 'disconnected' | 'connecting';
  metrics: {
    commandsExecuted: number;
    averageLatency: number;
    errorRate: number;
  };
  notifications: Array<{
    id: string;
    type: 'info' | 'success' | 'warning' | 'error';
    message: string;
    timestamp: number;
  }>;
}

const DEFAULT_CONFIG: ValtioSyncConfig = {
  enabled: true,
  autoSync: true,
  syncInterval: 1000,
  persistState: true,
  storageKey: 'ipc-valtio-state',
};

/**
 * Valtio-based state synchronization for IPC operations
 */
export class ValtioStateSync {
  private config: ValtioSyncConfig;
  private state: IPCState;
  private syncTimer?: NodeJS.Timeout;
  private subscriptions = new Set<() => void>();

  constructor(config: Partial<ValtioSyncConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config };

    // Create initial state with mutable arrays
    const initialState: IPCState = {
      gameState: null,
      lastCommand: null,
      connectionStatus: 'disconnected',
      metrics: {
        commandsExecuted: 0,
        averageLatency: 0,
        errorRate: 0,
      },
      notifications: [],
    };

    this.state = proxy(initialState);

    if (this.config.enabled) {
      this.initialize();
    }
  }

  /**
   * Initialize the state synchronization system
   */
  private initialize(): void {
    // Load persisted state if enabled
    if (this.config.persistState) {
      this.loadPersistedState();
    }

    // Set up state persistence
    this.setupStatePersistence();

    // Start auto-sync if enabled
    if (this.config.autoSync) {
      this.startAutoSync();
    }
  }

  /**
   * Get the reactive state proxy
   */
  getState(): IPCState {
    return this.state;
  }

  /**
   * Get a snapshot of the current state (immutable)
   */
  getSnapshot(): IPCState {
    return snapshot(this.state);
  }

  /**
   * Update game state from command result
   */
  updateGameState<T extends CommandName>(
    command: T,
    result: CommandOutput<T>
  ): void {
    if (!this.config.enabled) return;

    // Update last command
    this.state.lastCommand = {
      name: command,
      timestamp: Date.now(),
      output: result,
    };

    // Update specific state based on command type
    switch (command) {
      case 'get_game_state':
      case 'initialize_game':
      case 'load_game':
        if (this.isGameState(result)) {
          this.state.gameState = result;
        }
        break;

      case 'save_game':
        // Game state might have changed during save
        void this.requestGameStateUpdate();
        break;
    }

    // Update metrics
    this.state.metrics.commandsExecuted++;
  }

  /**
   * Handle backend events and update state accordingly
   */
  handleEvent<T extends EventName>(eventName: T, data: EventData<T>): void {
    if (!this.config.enabled) return;

    switch (eventName) {
      case 'game_state_changed':
        if (this.hasGameState(data)) {
          this.state.gameState = data.state;
        }
        break;

      case 'error_occurred':
        if (this.isErrorEvent(data)) {
          this.addNotification({
            type: 'error',
            message: `Command ${data.command} failed: ${data.error}`,
          });
        }
        break;

      case 'performance_warning':
        if (this.isPerformanceWarningEvent(data)) {
          this.addNotification({
            type: 'warning',
            message: `Performance warning: ${data.metric} = ${data.value}`,
          });
        }
        break;

      case 'notification':
        if (this.isNotificationEvent(data)) {
          this.addNotification({
            type: data.type,
            message: `${data.title}: ${data.message}`,
          });
        }
        break;
    }
  }

  /**
   * Update connection status
   */
  updateConnectionStatus(status: IPCState['connectionStatus']): void {
    if (!this.config.enabled) return;
    this.state.connectionStatus = status;
  }

  /**
   * Update performance metrics
   */
  updateMetrics(metrics: Partial<IPCState['metrics']>): void {
    if (!this.config.enabled) return;
    Object.assign(this.state.metrics, metrics);
  }

  /**
   * Add a notification to the state
   */
  addNotification(
    notification: Omit<IPCState['notifications'][0], 'id' | 'timestamp'>
  ): void {
    if (!this.config.enabled) return;

    const newNotification = {
      id: `notification_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`,
      timestamp: Date.now(),
      ...notification,
    };

    this.state.notifications.push(newNotification);

    // Keep only recent notifications (last 50)
    if (this.state.notifications.length > 50) {
      this.state.notifications.splice(0, this.state.notifications.length - 50);
    }
  }

  /**
   * Clear all notifications
   */
  clearNotifications(): void {
    if (!this.config.enabled) return;
    this.state.notifications.length = 0;
  }

  /**
   * Subscribe to specific state changes
   */
  subscribeToGameState(
    callback: (gameState: GameState | null) => void
  ): () => void {
    if (!this.config.enabled) return () => {};

    const unsubscribe = subscribeKey(this.state, 'gameState', callback);
    this.subscriptions.add(unsubscribe);
    return unsubscribe;
  }

  /**
   * Subscribe to connection status changes
   */
  subscribeToConnectionStatus(
    callback: (status: IPCState['connectionStatus']) => void
  ): () => void {
    if (!this.config.enabled) return () => {};

    const unsubscribe = subscribeKey(this.state, 'connectionStatus', callback);
    this.subscriptions.add(unsubscribe);
    return unsubscribe;
  }

  /**
   * Subscribe to metrics changes
   */
  subscribeToMetrics(
    callback: (metrics: IPCState['metrics']) => void
  ): () => void {
    if (!this.config.enabled) return () => {};

    const unsubscribe = subscribeKey(this.state, 'metrics', callback);
    this.subscriptions.add(unsubscribe);
    return unsubscribe;
  }

  /**
   * Subscribe to all state changes
   */
  subscribeToAll(callback: () => void): () => void {
    if (!this.config.enabled) return () => {};

    const unsubscribe = subscribe(this.state, () => callback());
    this.subscriptions.add(unsubscribe);
    return unsubscribe;
  }

  /**
   * Request a fresh game state update
   */
  private requestGameStateUpdate(): void {
    try {
      // This would trigger a command to get fresh game state
      // Implemented by the consuming code
      this.state.connectionStatus = 'connecting';
    } catch (error) {
      console.error('Failed to request game state update:', error);
    }
  }

  /**
   * Setup state persistence to localStorage
   */
  private setupStatePersistence(): void {
    if (!this.config.persistState || !this.config.storageKey) return;

    // Subscribe to state changes and persist them
    const unsubscribe = subscribe(this.state, ops => {
      void ops; // Ignore ops parameter
      // Debounce persistence to avoid too frequent writes
      void setTimeout(() => {
        this.persistState();
      }, 500);
    });

    this.subscriptions.add(unsubscribe);
  }

  /**
   * Persist current state to localStorage
   */
  private persistState(): void {
    if (!this.config.persistState || !this.config.storageKey) return;

    try {
      const stateSnapshot = snapshot(this.state);
      const serialized = JSON.stringify({
        state: stateSnapshot,
        timestamp: Date.now(),
      });

      localStorage.setItem(this.config.storageKey, serialized);
    } catch (error) {
      console.warn('Failed to persist valtio state:', error);
    }
  }

  /**
   * Load persisted state from localStorage
   */
  private loadPersistedState(): void {
    if (!this.config.persistState || !this.config.storageKey) return;

    try {
      const stored = localStorage.getItem(this.config.storageKey);
      if (!stored) return;

      const parsed = JSON.parse(stored) as {
        state?: unknown;
        timestamp?: number;
      };
      const { state, timestamp } = parsed;

      // Only load if not too old (1 hour) and timestamp is valid
      if (
        timestamp &&
        typeof timestamp === 'number' &&
        Date.now() - timestamp < 3600000 &&
        state &&
        typeof state === 'object' &&
        state !== null
      ) {
        const storedState = state as Record<string, unknown>;

        // Merge stored state with current state
        if (storedState.gameState && this.isGameState(storedState.gameState)) {
          this.state.gameState = storedState.gameState;
        }

        if (
          storedState.connectionStatus &&
          typeof storedState.connectionStatus === 'string'
        ) {
          const { connectionStatus } = storedState;
          if (
            connectionStatus === 'connected' ||
            connectionStatus === 'disconnected' ||
            connectionStatus === 'connecting'
          ) {
            this.state.connectionStatus = connectionStatus;
          }
        }

        if (
          storedState.metrics &&
          typeof storedState.metrics === 'object' &&
          storedState.metrics !== null
        ) {
          const metrics = storedState.metrics as Record<string, unknown>;
          if (typeof metrics.commandsExecuted === 'number') {
            this.state.metrics.commandsExecuted = metrics.commandsExecuted;
          }
          if (typeof metrics.averageLatency === 'number') {
            this.state.metrics.averageLatency = metrics.averageLatency;
          }
          if (typeof metrics.errorRate === 'number') {
            this.state.metrics.errorRate = metrics.errorRate;
          }
        }
      }
    } catch (error) {
      console.warn('Failed to load persisted valtio state:', error);
    }
  }

  /**
   * Start automatic state synchronization
   */
  private startAutoSync(): void {
    if (this.syncTimer) return;

    this.syncTimer = setInterval(() => {
      // Trigger periodic state sync if needed
      // This could ping the backend for fresh data
      this.syncWithBackend();
    }, this.config.syncInterval);
  }

  /**
   * Stop automatic state synchronization
   */
  private stopAutoSync(): void {
    if (this.syncTimer) {
      clearInterval(this.syncTimer);
      this.syncTimer = undefined;
    }
  }

  /**
   * Sync state with backend (placeholder for actual implementation)
   */
  private syncWithBackend(): void {
    // This would be implemented by consuming code to fetch fresh state
    // For now, just update connection status if needed
    if (this.state.connectionStatus === 'connecting') {
      // Simulate connection recovery
      void setTimeout(() => {
        if (this.state.connectionStatus === 'connecting') {
          this.state.connectionStatus = 'connected';
        }
      }, 2000);
    }
  }

  /**
   * Update configuration
   */
  updateConfig(config: Partial<ValtioSyncConfig>): void {
    const wasAutoSync = this.config.autoSync;
    this.config = { ...this.config, ...config };

    // Handle auto-sync changes
    if (!wasAutoSync && this.config.autoSync) {
      this.startAutoSync();
    } else if (wasAutoSync && !this.config.autoSync) {
      this.stopAutoSync();
    }
  }

  /**
   * Get current configuration
   */
  getConfig(): ValtioSyncConfig {
    return { ...this.config };
  }

  /**
   * Get state statistics
   */
  getStats() {
    const stateSnapshot = snapshot(this.state);
    return {
      notificationCount: stateSnapshot.notifications.length,
      hasGameState: stateSnapshot.gameState !== null,
      connectionStatus: stateSnapshot.connectionStatus,
      metrics: stateSnapshot.metrics,
      subscriptionCount: this.subscriptions.size,
    };
  }

  /**
   * Destroy and cleanup the sync system
   */
  destroy(): void {
    this.stopAutoSync();

    // Cleanup all subscriptions
    this.subscriptions.forEach(unsubscribe => unsubscribe());
    this.subscriptions.clear();

    // Final state persistence
    if (this.config.persistState) {
      this.persistState();
    }
  }

  // Type guards
  private isGameState(result: unknown): result is GameState {
    return (
      typeof result === 'object' &&
      result !== null &&
      'turn' in result &&
      'player_name' in result &&
      'civilization' in result &&
      'is_paused' in result
    );
  }

  private hasGameState(data: unknown): data is { state: GameState } {
    return (
      data !== null &&
      typeof data === 'object' &&
      'state' in data &&
      this.isGameState((data as { state: unknown }).state)
    );
  }

  private isErrorEvent(
    data: unknown
  ): data is { command: string; error: string } {
    return (
      data !== null &&
      typeof data === 'object' &&
      'command' in data &&
      'error' in data &&
      typeof (data as { command: unknown }).command === 'string' &&
      typeof (data as { error: unknown }).error === 'string'
    );
  }

  private isPerformanceWarningEvent(
    data: unknown
  ): data is { metric: string; value: number } {
    return (
      data !== null &&
      typeof data === 'object' &&
      'metric' in data &&
      'value' in data &&
      typeof (data as { metric: unknown }).metric === 'string' &&
      typeof (data as { value: unknown }).value === 'number'
    );
  }

  private isNotificationEvent(data: unknown): data is {
    type: string;
    title: string;
    message: string;
  } {
    return (
      data !== null &&
      typeof data === 'object' &&
      'type' in data &&
      'title' in data &&
      'message' in data &&
      typeof (data as { type: unknown }).type === 'string' &&
      typeof (data as { title: unknown }).title === 'string' &&
      typeof (data as { message: unknown }).message === 'string'
    );
  }
}

// Helper functions for easier integration

/**
 * Create Valtio integration helpers
 */
export const createValtioIntegration = (stateSync: ValtioStateSync) => {
  return {
    // Command integration
    onCommandCompleted: <T extends CommandName>(
      command: T,
      result: CommandOutput<T>
    ) => {
      stateSync.updateGameState(command, result);
    },

    // Event integration
    onEvent: <T extends EventName>(eventName: T, data: EventData<T>) => {
      stateSync.handleEvent(eventName, data);
    },

    // Connection integration
    onConnectionChange: (status: IPCState['connectionStatus']) => {
      stateSync.updateConnectionStatus(status);
    },

    // Metrics integration
    onMetricsUpdate: (metrics: Partial<IPCState['metrics']>) => {
      stateSync.updateMetrics(metrics);
    },

    // State access
    getState: () => stateSync.getState(),
    getSnapshot: () => stateSync.getSnapshot(),

    // Subscriptions
    subscribeToGameState: stateSync.subscribeToGameState.bind(stateSync),
    subscribeToConnection:
      stateSync.subscribeToConnectionStatus.bind(stateSync),
    subscribeToMetrics: stateSync.subscribeToMetrics.bind(stateSync),
    subscribeToAll: stateSync.subscribeToAll.bind(stateSync),
  };
};

// Default singleton instance
export const valtioStateSync = new ValtioStateSync({
  enabled: true,
  autoSync: true,
  syncInterval: 2000,
  persistState: true,
});

// Export integration helper
export const valtioIntegration = createValtioIntegration(valtioStateSync);
