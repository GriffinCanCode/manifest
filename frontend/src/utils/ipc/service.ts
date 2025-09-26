/**
 * IPC Service
 * Type-safe, validated communication with backend
 */

import { invoke } from '@tauri-apps/api/core';
import { nanoid } from 'nanoid';
import PQueue from 'p-queue';
import type { z } from 'zod';

import {
  EventEmitter,
  type ExtendedEventData,
  type ExtendedEventName,
} from './events.js';
import { IPCMetrics } from './metrics.js';
import { ipcProgress } from './progress.js';
import {
  CommandSchemas,
  type CommandInput,
  type CommandName,
  type CommandOutput,
} from './schemas';
import { valtioIntegration } from './valtio-sync.js';
import { vitalsIntegration } from './web-vitals.js';

export interface IPCCommand<T extends CommandName> {
  id: string;
  name: T;
  input: CommandInput<T>;
  timestamp: number;
  retries?: number;
  priority?: 'low' | 'normal' | 'high';
}

export interface IPCCommandResult<T extends CommandName> {
  id: string;
  name: T;
  output: CommandOutput<T>;
  duration: number;
  timestamp: number;
}

export interface IPCError extends Error {
  command: string;
  commandId: string;
  input?: unknown;
  correlationId?: string;
}

export interface IPCConfig {
  maxConcurrentCommands: number;
  defaultTimeout: number;
  retryAttempts: number;
  retryDelay: number;
  enableMetrics: boolean;
  enableBatching: boolean;
  batchDelay: number;
  maxBatchSize: number;
}

const DEFAULT_CONFIG: IPCConfig = {
  maxConcurrentCommands: 10,
  defaultTimeout: 10000,
  retryAttempts: 3,
  retryDelay: 1000,
  enableMetrics: true,
  enableBatching: true,
  batchDelay: 50,
  maxBatchSize: 100,
};

/**
 * Type-safe IPC Service with validation and performance monitoring
 */
export class IPCService {
  private readonly queue: PQueue;
  private readonly metrics: IPCMetrics;
  private readonly events: EventEmitter;
  private readonly config: IPCConfig;

  constructor(config: Partial<IPCConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config };
    this.queue = new PQueue({
      concurrency: this.config.maxConcurrentCommands,
      interval: 100,
      intervalCap: this.config.maxConcurrentCommands,
    });
    this.metrics = new IPCMetrics(this.config.enableMetrics);
    this.events = new EventEmitter();

    // Setup event forwarding
    this.setupEventForwarding();
  }

  /**
   * Execute a command with validation and error handling
   */
  async command<T extends CommandName>(
    name: T,
    input: CommandInput<T>,
    options: {
      priority?: IPCCommand<T>['priority'];
      timeout?: number;
      retries?: number;
      validate?: boolean;
    } = {}
  ): Promise<CommandOutput<T>> {
    const commandId = nanoid();
    const command: IPCCommand<T> = {
      id: commandId,
      name,
      input,
      timestamp: Date.now(),
      retries: options.retries ?? this.config.retryAttempts,
      priority: options.priority ?? 'normal',
    };

    // Validate input if enabled
    if (options.validate !== false) {
      this.validateCommandInput(name, input);
    }

    // Add to queue based on priority
    const queueOptions = {
      priority: this.getPriorityValue(command.priority),
    };

    try {
      const result = (await this.queue.add(
        () => this.executeCommand(command, options.timeout),
        queueOptions
      )) as IPCCommandResult<T> | undefined;

      if (!result) {
        throw new Error('Command execution failed - no result returned');
      }

      this.metrics.recordCommand(name, result.duration, true);
      this.events.emit('command_completed', {
        commandId,
        name,
        duration: result.duration,
      });

      // Type assertion for output safety - the output is validated before returning
      // eslint-disable-next-line @typescript-eslint/no-unsafe-return
      return result.output;
    } catch (error) {
      const ipcError = this.createIPCError(error, command);
      this.metrics.recordCommand(name, 0, false);
      this.events.emit('command_failed', {
        commandId,
        name,
        error: ipcError.message,
      });
      throw ipcError;
    }
  }

  /**
   * Execute multiple commands in batch
   */
  async batch<T extends CommandName>(
    commands: Array<{ name: T; input: CommandInput<T> }>,
    options: {
      parallel?: boolean;
      failFast?: boolean;
      timeout?: number;
    } = {}
  ): Promise<Array<CommandOutput<T> | IPCError>> {
    const batchId = nanoid();
    const batchStart = performance.now();

    this.events.emit('batch_started', {
      batchId,
      commandCount: commands.length,
    });

    // Start batch progress
    ipcProgress.commandStarted(
      batchId,
      'execute_batch_commands' as CommandName
    );

    try {
      let results: Array<CommandOutput<T> | IPCError>;
      let completed = 0;

      if (options.parallel) {
        // Execute commands in parallel with progress tracking
        const promises = commands.map(async ({ name, input }) => {
          try {
            const result = await this.command(name, input, {
              timeout: options.timeout,
            });
            completed++;
            ipcProgress.batchProgress(batchId, completed, commands.length);
            return result;
          } catch (error) {
            completed++;
            ipcProgress.batchProgress(batchId, completed, commands.length);
            return error as IPCError;
          }
        });

        results = await Promise.all(promises);
      } else {
        // Execute commands sequentially with progress tracking
        results = [];
        for (let i = 0; i < commands.length; i++) {
          const { name, input } = commands[i];
          try {
            const result = await this.command(name, input, {
              timeout: options.timeout,
            });
            results.push(result);
            completed++;
            ipcProgress.batchProgress(batchId, completed, commands.length);
          } catch (error) {
            const ipcError = error as IPCError;
            results.push(ipcError);
            completed++;
            ipcProgress.batchProgress(batchId, completed, commands.length);

            if (options.failFast) {
              break;
            }
          }
        }
      }

      const duration = performance.now() - batchStart;
      const successCount = results.filter(r => !(r instanceof Error)).length;

      this.metrics.recordBatch(commands.length, duration);

      // Complete batch progress
      ipcProgress.commandCompleted(batchId, successCount > 0);

      this.events.emit('batch_completed', {
        batchId,
        commandCount: commands.length,
        duration,
        successCount,
      });

      return results;
    } catch (error) {
      const duration = performance.now() - batchStart;

      // Fail batch progress
      ipcProgress.commandFailed(batchId);

      this.events.emit('batch_failed', {
        batchId,
        duration,
        error: (error as Error).message,
      });
      throw error;
    }
  }

  /**
   * Listen for backend events
   */
  onEvent<T extends ExtendedEventName>(
    eventName: T,
    handler: (data: ExtendedEventData<T>) => void
  ): () => void {
    return this.events.on(eventName, handler);
  }

  /**
   * Get performance metrics
   */
  getMetrics() {
    const metricsData = this.metrics.getMetrics();
    return {
      ...metricsData,
      queueSize: this.queue.size ?? 0,
      queuePending: this.queue.pending ?? 0,
    };
  }

  /**
   * Clear all queued commands
   */
  clearQueue() {
    this.queue.clear();
    this.events.emit('queue_cleared', { timestamp: Date.now() });
  }

  /**
   * Shutdown the service
   */
  destroy() {
    this.queue.clear();
    this.events.removeAllListeners();
    this.metrics.destroy();
  }

  // Private methods

  private async executeCommand<T extends CommandName>(
    command: IPCCommand<T>,
    timeout?: number
  ): Promise<IPCCommandResult<T>> {
    const startTime = performance.now();

    // Start progress tracking
    ipcProgress.commandStarted(command.id, command.name);

    try {
      // Create timeout promise
      const timeoutMs = timeout ?? this.config.defaultTimeout;
      const timeoutPromise = new Promise<never>((_, reject) => {
        setTimeout(
          () => reject(new Error(`Command timeout after ${timeoutMs}ms`)),
          timeoutMs
        );
      });

      // Simulate progress updates for long commands
      const progressInterval = setInterval(() => {
        const elapsed = performance.now() - startTime;
        const progress = Math.min(0.8, elapsed / timeoutMs);
        ipcProgress.commandProgress(command.id, progress);
      }, 500);

      // Execute command with timeout
      const resultPromise = invoke<CommandOutput<T>>(
        command.name,
        command.input
      );
      const output = (await Promise.race([
        resultPromise,
        timeoutPromise,
      ])) as CommandOutput<T>;

      clearInterval(progressInterval);

      // Validate output
      this.validateCommandOutput(command.name, output);

      const duration = performance.now() - startTime;

      // Complete progress tracking
      ipcProgress.commandCompleted(command.id, true);

      // Complete web vitals tracking and check for performance impacts
      const vitalsImpact = vitalsIntegration.completeCommand(command.id);
      if (vitalsImpact) {
        vitalsIntegration.reportPerformanceIssue(command.name, vitalsImpact);
      }

      // Update valtio state with command result
      valtioIntegration.onCommandCompleted(command.name, output);

      return {
        id: command.id,
        name: command.name,
        output,
        duration,
        timestamp: Date.now(),
      } as IPCCommandResult<T>;
    } catch (error) {
      // Handle retries with progress updates
      if (command.retries && command.retries > 0) {
        ipcProgress.commandProgress(command.id, 0.2); // Show retry progress

        // Retry with exponential backoff
        const delay =
          this.config.retryDelay *
          Math.pow(2, this.config.retryAttempts - command.retries);
        await new Promise(resolve => setTimeout(resolve, delay));

        return this.executeCommand(
          { ...command, retries: command.retries - 1 },
          timeout
        );
      }

      // Fail progress tracking
      ipcProgress.commandFailed(command.id);

      // Complete web vitals tracking for failed command
      const vitalsImpact = vitalsIntegration.completeCommand(command.id);
      if (vitalsImpact) {
        vitalsIntegration.reportPerformanceIssue(command.name, vitalsImpact);
      }

      throw error;
    }
  }

  private validateCommandInput<T extends CommandName>(
    name: T,
    input: CommandInput<T>
  ) {
    try {
      CommandSchemas[name].shape.input.parse(input);
    } catch (error) {
      throw new Error(
        `Invalid command input for ${name}: ${(error as z.ZodError).message}`
      );
    }
  }

  private validateCommandOutput<T extends CommandName>(
    name: T,
    output: unknown
  ) {
    try {
      CommandSchemas[name].shape.output.parse(output);
    } catch (error) {
      console.warn(`Invalid command output for ${name}:`, error);
      // Don't throw - backend might have newer schema
    }
  }

  private createIPCError(
    error: unknown,
    command: IPCCommand<CommandName>
  ): IPCError {
    const message = error instanceof Error ? error.message : String(error);
    const ipcError = new Error(`IPC Command failed: ${message}`) as IPCError;

    Object.assign(ipcError, {
      command: command.name,
      commandId: command.id,
      input: command.input,
      name: 'IPCError',
    });

    return ipcError;
  }

  private getPriorityValue(
    priority: IPCCommand<CommandName>['priority']
  ): number {
    switch (priority) {
      case 'high':
        return 10;
      case 'normal':
        return 5;
      case 'low':
        return 1;
      default:
        return 5;
    }
  }

  private setupEventForwarding() {
    // In a real implementation, this would listen to Tauri events
    // For now, we'll emit mock events for demonstration
    // Listen for Tauri events and forward them
    // listen('game_state_changed', (event) => {
    //   const data = EventSchemas.game_state_changed.parse(event.payload);
    //   this.events.emit('game_state_changed', data);
    //   valtioIntegration.onEvent('game_state_changed', data);
    // });

    // Set up valtio integration for event forwarding
    this.events.on('game_state_changed', data => {
      valtioIntegration.onEvent('game_state_changed', data);
    });

    this.events.on('error_occurred', data => {
      valtioIntegration.onEvent('error_occurred', data);
    });

    this.events.on('performance_warning', data => {
      valtioIntegration.onEvent('performance_warning', data);
    });

    // Update connection status based on command execution
    this.events.on('command_completed', () => {
      valtioIntegration.onConnectionChange('connected');
    });

    this.events.on('command_failed', () => {
      // Don't immediately mark as disconnected, could be a temporary issue
    });
  }
}

// Default singleton instance
export const ipcService = new IPCService();
