/**
 * IPC Schema Definitions
 * Type-safe command and event schemas using Zod
 */

import { z } from 'zod';

// Base schemas
export const HexCoordSchema = z.object({
  q: z.number().int(),
  r: z.number().int(),
});

export const GameStateSchema = z.object({
  turn: z.number().int().positive(),
  player_name: z.string().min(1),
  civilization: z.string().min(1),
  is_paused: z.boolean(),
});

export const SaveInfoSchema = z.object({
  name: z.string(),
  created_at: z.string(),
  modified_at: z.string(),
  size_bytes: z.number(),
  turn: z.number(),
  civilization: z.string(),
  player_name: z.string(),
});

// Command Schemas
export const CommandSchemas = {
  // Basic commands
  greet: z.object({
    input: z.object({ name: z.string() }),
    output: z.string(),
  }),

  get_game_state: z.object({
    input: z.object({}),
    output: GameStateSchema,
  }),

  initialize_game: z.object({
    input: z.object({
      playerName: z.string().min(1).max(50),
      civilization: z.string().min(1).max(50),
    }),
    output: GameStateSchema,
  }),

  save_game: z.object({
    input: z.object({
      saveName: z.string().min(1).max(100),
    }),
    output: z.string(),
  }),

  load_game: z.object({
    input: z.object({
      saveName: z.string().min(1),
    }),
    output: GameStateSchema,
  }),

  list_saves: z.object({
    input: z.object({}),
    output: z.array(SaveInfoSchema),
  }),

  // Tile streaming commands
  stream_tiles: z.object({
    input: z.object({
      request: z.object({
        camera_position: z.tuple([z.number(), z.number(), z.number()]),
        view_radius: z.number().positive(),
        max_tiles: z.number().int().positive().max(20000),
        lod_levels: z.array(z.number().int().min(0).max(5)),
        generation: z.number().int().nonnegative(),
      }),
    }),
    output: z.object({
      tiles: z.array(z.any()), // GameTile schema would be complex
      instance_data: z.array(z.any()), // TileInstanceData schema
      generation: z.number().int(),
      has_more: z.boolean(),
      next_offset: z.number().int().optional(),
    }),
  }),

  get_tile: z.object({
    input: z.object({
      tileId: z.number().int().positive(),
    }),
    output: z.any().optional(), // GameTile or null
  }),

  // Debug commands
  get_scheduler_metrics: z.object({
    input: z.object({}),
    output: z.object({
      tasks_executed: z.number().int(),
      average_task_time_ms: z.number(),
      last_frame_time_ms: z.number(),
      total_time_ms: z.number(),
      peak_memory_mb: z.number(),
    }),
  }),

  // Batch commands
  execute_batch_commands: z.object({
    input: z.object({
      commands: z.array(
        z.object({
          name: z.string(),
          input: z.any(),
        })
      ),
      options: z.object({
        parallel: z.boolean().optional(),
        fail_fast: z.boolean().optional(),
        timeout_ms: z.number().int().positive().optional(),
      }),
    }),
    output: z.object({
      results: z.array(
        z.object({
          success: z.boolean(),
          output: z.any().optional(),
          error: z.string().optional(),
          duration_ms: z.number().int(),
        })
      ),
      summary: z.object({
        total_commands: z.number().int(),
        successful_commands: z.number().int(),
        failed_commands: z.number().int(),
        total_duration_ms: z.number().int(),
      }),
    }),
  }),

  // Health check
  health_check: z.object({
    input: z.object({}),
    output: z.object({
      status: z.string(),
      timestamp: z.number().int(),
      version: z.string(),
    }),
  }),

  // Tile updates
  get_tile_updates: z.object({
    input: z.object({
      tileIds: z.array(z.number().int().positive()),
      lastUpdateTime: z.number().int().nonnegative(),
    }),
    output: z.object({
      updated_tiles: z.array(z.number().int()), // Updated tile IDs
      removed_tiles: z.array(z.number().int()), // Removed tile IDs
      timestamp: z.number().int(),
    }),
  }),

  // Save thumbnail commands
  save_thumbnail_metadata: z.object({
    input: z.object({
      saveName: z.string().min(1),
      thumbnailData: z.object({
        thumbnail: z.string(), // base64 encoded image
        dimensions: z.object({
          width: z.number().int().positive(),
          height: z.number().int().positive(),
        }),
        created_at: z.number().int(),
        file_size: z.number().int().positive(),
      }),
    }),
    output: z.void(), // Returns void/null
  }),

  load_thumbnail_metadata: z.object({
    input: z.object({
      saveName: z.string().min(1),
    }),
    output: z
      .object({
        thumbnail: z.string(),
        dimensions: z.object({
          width: z.number().int().positive(),
          height: z.number().int().positive(),
        }),
        created_at: z.number().int(),
        file_size: z.number().int().positive(),
      })
      .optional(),
  }),
} as const;

// Event Schemas
export const EventSchemas = {
  game_state_changed: z.object({
    state: GameStateSchema,
    timestamp: z.number(),
  }),

  tile_updated: z.object({
    tile_ids: z.array(z.number().int()),
    timestamp: z.number(),
  }),

  error_occurred: z.object({
    command: z.string(),
    error: z.string(),
    correlation_id: z.string().optional(),
    timestamp: z.number(),
  }),

  performance_warning: z.object({
    metric: z.string(),
    value: z.number(),
    threshold: z.number(),
    timestamp: z.number(),
  }),

  notification: z.object({
    id: z.string(),
    type: z.enum(['info', 'success', 'warning', 'error']),
    title: z.string(),
    message: z.string(),
    duration: z.number().optional(),
    timestamp: z.number(),
  }),
} as const;

// Type inference
export type HexCoord = z.infer<typeof HexCoordSchema>;
export type GameState = z.infer<typeof GameStateSchema>;
export type SaveInfo = z.infer<typeof SaveInfoSchema>;

// Command type inference
export type CommandName = keyof typeof CommandSchemas;
export type CommandInput<T extends CommandName> = z.infer<
  (typeof CommandSchemas)[T]['shape']['input']
>;
export type CommandOutput<T extends CommandName> = z.infer<
  (typeof CommandSchemas)[T]['shape']['output']
>;

// Event type inference
export type EventName = keyof typeof EventSchemas;
export type EventData<T extends EventName> = z.infer<(typeof EventSchemas)[T]>;

// Utility types
export type AnyCommandSchema = (typeof CommandSchemas)[CommandName];
export type AnyEventSchema = (typeof EventSchemas)[EventName];
