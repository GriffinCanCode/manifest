/**
 * IPC Event System
 * Type-safe event emitter for IPC communication using eventemitter3
 */

import EventEmitter3 from 'eventemitter3';

import type { EventData, EventName } from './schemas';

export type EventHandler<T extends EventName> = (data: EventData<T>) => void;
export type AnyEventHandler = (data: unknown) => void;

/**
 * Type-safe event emitter for IPC events using EventEmitter3
 */
export class EventEmitter {
  private readonly emitter: EventEmitter3;
  private readonly maxListeners: number;
  private isDestroyed = false;

  constructor(maxListeners: number = 100) {
    this.maxListeners = maxListeners;
    this.emitter = new EventEmitter3();
    // EventEmitter3 doesn't have setMaxListeners, we'll handle warnings manually
  }

  /**
   * Subscribe to an event
   */
  on<T extends ExtendedEventName>(
    eventName: T,
    handler: (data: ExtendedEventData<T>) => void
  ): () => void {
    this.checkDestroyed();

    this.emitter.on(eventName as string, handler as AnyEventHandler);

    // Return unsubscribe function
    return () => {
      this.emitter.off(eventName as string, handler as AnyEventHandler);
    };
  }

  /**
   * Subscribe to an event once
   */
  once<T extends ExtendedEventName>(
    eventName: T,
    handler: (data: ExtendedEventData<T>) => void
  ): () => void {
    this.checkDestroyed();

    this.emitter.once(eventName as string, handler as AnyEventHandler);

    // Return unsubscribe function
    return () => {
      this.emitter.off(eventName as string, handler as AnyEventHandler);
    };
  }

  /**
   * Remove specific event listener
   */
  off<T extends ExtendedEventName>(
    eventName: T,
    handler: (data: ExtendedEventData<T>) => void
  ): void {
    this.emitter.off(eventName as string, handler as AnyEventHandler);
  }

  /**
   * Remove all listeners for an event
   */
  removeAllListeners(eventName?: ExtendedEventName): void {
    if (eventName) {
      this.emitter.removeAllListeners(eventName as string);
    } else {
      this.emitter.removeAllListeners();
    }
  }

  /**
   * Emit an event to all listeners
   */
  emit<T extends ExtendedEventName>(
    eventName: T,
    data: ExtendedEventData<T>
  ): void {
    this.checkDestroyed();

    try {
      this.emitter.emit(eventName as string, data);
    } catch (error) {
      console.error(
        `EventEmitter: Error in event handler for '${eventName}':`,
        error
      );
    }
  }

  /**
   * Get listener count for an event
   */
  listenerCount(eventName: ExtendedEventName): number {
    return this.emitter.listenerCount(eventName as string);
  }

  /**
   * Get all event names that have listeners
   */
  eventNames(): ExtendedEventName[] {
    return this.emitter.eventNames() as ExtendedEventName[];
  }

  /**
   * Check if event has listeners
   */
  hasListeners(eventName: ExtendedEventName): boolean {
    return this.emitter.listenerCount(eventName as string) > 0;
  }

  /**
   * Wait for a specific event to be emitted
   */
  waitFor<T extends ExtendedEventName>(
    eventName: T,
    timeout?: number
  ): Promise<ExtendedEventData<T>> {
    return new Promise((resolve, reject) => {
      let timeoutId: NodeJS.Timeout | undefined;

      const cleanup = this.once(eventName, data => {
        if (timeoutId) {
          clearTimeout(timeoutId);
        }
        resolve(data);
      });

      if (timeout) {
        timeoutId = setTimeout(() => {
          cleanup();
          reject(new Error(`Event '${eventName}' timeout after ${timeout}ms`));
        }, timeout);
      }
    });
  }

  /**
   * Create a filtered event stream
   */
  filter<T extends ExtendedEventName>(
    eventName: T,
    predicate: (data: ExtendedEventData<T>) => boolean
  ): EventEmitter {
    const filtered = new EventEmitter(this.maxListeners);

    this.on(eventName, data => {
      if (predicate(data)) {
        filtered.emit(eventName, data);
      }
    });

    return filtered;
  }

  /**
   * Create a mapped event stream
   */
  map<T extends ExtendedEventName, U>(
    eventName: T,
    mapper: (data: ExtendedEventData<T>) => U
  ): EventEmitter {
    const mapped = new EventEmitter(this.maxListeners);

    this.on(eventName, data => {
      try {
        const mappedData = mapper(data);
        // Emit with a generic event name since we can't type this properly
        mapped.emit(
          'mapped_event' as ExtendedEventName,
          mappedData as ExtendedEventData<ExtendedEventName>
        );
      } catch (error) {
        console.error(
          `EventEmitter: Error in mapper for '${eventName}':`,
          error
        );
      }
    });

    return mapped;
  }

  /**
   * Get debug information
   */
  getDebugInfo() {
    const eventNames = this.emitter.eventNames();
    const events: Record<string, number> = {};
    let totalListeners = 0;

    for (const eventName of eventNames) {
      const count = this.emitter.listenerCount(eventName as string);
      events[eventName as string] = count;
      totalListeners += count;
    }

    return {
      totalEvents: eventNames.length,
      totalListeners,
      events,
      maxListeners: this.maxListeners,
      isDestroyed: this.isDestroyed,
    };
  }

  /**
   * Destroy the event emitter
   */
  destroy(): void {
    this.removeAllListeners();
    this.isDestroyed = true;
  }

  private checkDestroyed(): void {
    if (this.isDestroyed) {
      throw new Error('EventEmitter has been destroyed');
    }
  }
}

// IPC-specific events (extending the base events from schemas)
export interface IPCEvents {
  // Command events
  command_started: { commandId: string; name: string; timestamp: number };
  command_completed: { commandId: string; name: string; duration: number };
  command_failed: { commandId: string; name: string; error: string };
  command_retrying: { commandId: string; name: string; attempt: number };

  // Batch events
  batch_started: { batchId: string; commandCount: number };
  batch_completed: {
    batchId: string;
    commandCount: number;
    duration: number;
    successCount: number;
  };
  batch_failed: { batchId: string; duration: number; error: string };

  // Queue events
  queue_cleared: { timestamp: number };
  queue_full: { queueSize: number; timestamp: number };

  // Performance events
  performance_warning: {
    metric: string;
    value: number;
    threshold: number;
    timestamp: number;
  };
  memory_warning: { usage: number; threshold: number; timestamp: number };

  // Connection events
  connection_lost: { timestamp: number };
  connection_restored: { timestamp: number };

  // Debug events
  debug_info: { type: string; data: unknown; timestamp: number };
}
// Re-export with extended types
export type ExtendedEventName = EventName | keyof IPCEvents;
export type ExtendedEventData<T extends ExtendedEventName> = T extends EventName
  ? EventData<T>
  : T extends keyof IPCEvents
    ? IPCEvents[T]
    : never;
