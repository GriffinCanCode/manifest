//! Game commands module
//! 
//! Handles all Tauri commands for frontend-backend communication
//! Organized by functional area for maintainability

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager, State};
use tracing::{info, error, debug, warn, instrument};

use crate::ecs::{GameWorld, SaveInfo, SaveSystem};
use crate::ecs::saves::{SaveThumbnailMetadata, ThumbnailDimensions};
use crate::core::{logging::LoggingSystem, caching::{GameCache, CacheKey, PlayerCacheKey, CachePriority}};

// Import validation and events modules
use self::events::IPCEventEmitter;
use self::validation::{IPCErrorType, Validator};

// Sub-modules for enhanced IPC functionality
pub mod events;
pub mod validation;
pub mod tile_streaming;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub turn: u32,
    pub player_name: String,
    pub civilization: String,
    pub is_paused: bool,
}

/// Shared application state
#[derive(Debug)]
pub struct AppState {
    pub world: Mutex<GameWorld>,
    pub command_cache: GameCache,
    pub save_system: SaveSystem,
}

/// Enhanced command context with metrics and event emission
struct CommandContext {
    correlation_id: String,
    command_name: String,
    start_time: Instant,
    events: IPCEventEmitter,
}

impl CommandContext {
    fn new(command_name: &str, app_handle: AppHandle) -> Self {
        let correlation_id = LoggingSystem::generate_correlation_id();
        
        Self {
            correlation_id: correlation_id.to_string(),
            command_name: command_name.to_string(),
            start_time: Instant::now(),
            events: IPCEventEmitter::new(app_handle),
        }
    }

    /// Record command completion and emit events
    fn complete<T>(&self, result: &Result<T, String>) {
        let duration = self.start_time.elapsed();
        let duration_ms = duration.as_millis() as u64;

        match result {
            Ok(_) => {
                info!(
                    target: "manifest::commands",
                    correlation_id = %self.correlation_id,
                    command = %self.command_name,
                    duration_ms = duration_ms,
                    "Command completed successfully"
                );
                
                self.events.command_completed(
                    self.correlation_id.clone(),
                    self.command_name.clone(),
                    duration_ms,
                );

                // Check for slow commands
                if duration > Duration::from_millis(1000) {
                    self.events.performance_warning(
                        format!("slow_command_{}", self.command_name),
                        duration_ms as f64,
                        1000.0,
                    );
                }
            },
            Err(error) => {
                error!(
                    target: "manifest::commands",
                    correlation_id = %self.correlation_id,
                    command = %self.command_name,
                    duration_ms = duration_ms,
                    error = %error,
                    "Command failed"
                );
                
                self.events.command_failed(
                    self.correlation_id.clone(),
                    self.command_name.clone(),
                    error.clone(),
                );
                
                self.events.error_occurred(
                    self.command_name.clone(),
                    error.clone(),
                    Some(self.correlation_id.clone()),
                );
            }
        }
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            turn: 1,
            player_name: "Player".to_string(),
            civilization: "Ancient Empire".to_string(),
            is_paused: false,
        }
    }
}

#[tauri::command]
#[instrument(skip(app))]
pub async fn greet(name: String, app: AppHandle) -> Result<String, String> {
    let ctx = CommandContext::new("greet", app);
    
    ctx.events.command_started(ctx.correlation_id.clone(), ctx.command_name.clone());
    
    let result = async {
        // Validate input
        if let Err(e) = Validator::validate_player_name(&name) {
            return Err(e.to_error_string(Some(ctx.correlation_id.clone())));
        }

        let greeting = format!("Hello, {}! Welcome to Manifest.", name);
        
        // Emit success notification
        ctx.events.success(
            "Welcome!".to_string(),
            format!("Welcome to Manifest, {}!", name),
        );
        
        Ok(greeting)
    }.await;
    
    ctx.complete(&result);
    result
}

#[tauri::command]
pub async fn get_game_state(app_state: State<'_, AppState>) -> Result<GameState, String> {
    let correlation_id = LoggingSystem::generate_correlation_id();
    
    debug!(
        target: "manifest::commands",
        correlation_id = correlation_id,
        command = "get_game_state",
        "Retrieving current game state"
    );
    
    // Try cache first for game state using proper cache key generation
    let cache_key = {
        let world = app_state.world.lock().unwrap();
        let current_turn = world.get_turn();
        let current_player = world.world().get_resource::<crate::ecs::resources::Players>()
            .map(|players| players.current_player)
            .unwrap_or(1);
        
        CacheKey::Player(PlayerCacheKey::game_state(current_player, current_turn))
    };
    if let Ok(Some(cached_state)) = app_state.command_cache.get::<GameState>(&cache_key).await {
        debug!(
            target: "manifest::commands",
            correlation_id = correlation_id,
            "Retrieved game state from cache"
        );
        return Ok(cached_state);
    }
    
    // Compute fresh state
    let start_time = Instant::now();
    let state = {
        let world = app_state.world.lock().unwrap();
        GameState {
            turn: world.get_turn(),
            player_name: "Player".to_string(),
            civilization: "Ancient Empire".to_string(),
            is_paused: world.is_paused(),
        }
    }; // MutexGuard is dropped here
    
    // Cache the result
    let computation_time = start_time.elapsed();
    let _ = app_state.command_cache.set(cache_key, state.clone(), CachePriority::High).await;
    
    debug!(
        target: "manifest::commands",
        correlation_id = correlation_id,
        turn = state.turn,
        player = %state.player_name,
        civilization = %state.civilization,
        paused = state.is_paused,
        computation_time_us = computation_time.as_micros(),
        "Game state retrieved and cached successfully"
    );
    
    Ok(state)
}

#[tauri::command]
#[instrument(skip(app_state, app))]
pub async fn initialize_game(
    player_name: String,
    civilization: String,
    app_state: State<'_, AppState>,
    app: AppHandle,
) -> Result<GameState, String> {
    let ctx = CommandContext::new("initialize_game", app);
    
    ctx.events.command_started(ctx.correlation_id.clone(), ctx.command_name.clone());
    
    let result = async {
        // Validate inputs
        if let Err(e) = Validator::validate_player_name(&player_name) {
            return Err(e.to_error_string(Some(ctx.correlation_id.clone())));
        }
        
        if let Err(e) = Validator::validate_civilization_name(&civilization) {
            return Err(e.to_error_string(Some(ctx.correlation_id.clone())));
        }

        // Initialize game state
        let game_state = GameState {
            turn: 1,
            player_name: player_name.clone(),
            civilization: civilization.clone(),
            is_paused: false,
        };

        // TODO: Initialize actual game world here
        // This would involve setting up the ECS world with initial entities
        
        // Emit game state changed event
        ctx.events.game_state_changed(game_state.clone());
        
        // Emit success notification
        ctx.events.success(
            "Game Initialized".to_string(),
            format!("New game started as {} civilization", civilization),
        );

        info!(
            target: "manifest::commands",
            correlation_id = %ctx.correlation_id,
            player_name = %player_name,
            civilization = %civilization,
            "Game initialized successfully"
        );

        Ok(game_state)
    }.await;
    
    ctx.complete(&result);
    result
}

#[tauri::command]
#[instrument(skip(state, app))]
pub async fn save_game(
    save_name: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<String, String> {
    let ctx = CommandContext::new("save_game", app);
    
    ctx.events.command_started(ctx.correlation_id.clone(), ctx.command_name.clone());
    
    let result = async {
        // Validate save name
        if let Err(e) = Validator::validate_save_name(&save_name) {
            return Err(e.to_error_string(Some(ctx.correlation_id.clone())));
        }

        // Acquire world lock
        let mut world = state.world.lock().map_err(|e| {
            let err = IPCErrorType::Internal {
                message: format!("Failed to acquire world lock: {}", e),
            };
            err.to_error_string(Some(ctx.correlation_id.clone()))
        })?;

        // Perform save operation
        let save_result = state.save_system.save(&mut *world, &save_name);
        
        match save_result {
            Ok(path) => {
                let message = format!("Game saved as: {}", save_name);
                
                // Emit success notification
                ctx.events.success(
                    "Game Saved".to_string(),
                    format!("Game successfully saved as '{}'", save_name),
                );

                info!(
                    target: "manifest::commands",
                    correlation_id = %ctx.correlation_id,
                    save_name = %save_name,
                    save_path = ?path,
                    "Game saved successfully"
                );

                Ok(message)
            }
            Err(e) => {
                let err = IPCErrorType::FileSystem {
                    message: format!("Failed to save game: {}", e),
                };
                Err(err.to_error_string(Some(ctx.correlation_id.clone())))
            }
        }
    }.await;
    
    ctx.complete(&result);
    result
}

#[tauri::command]
#[instrument(skip(state, app))]
pub async fn load_game(
    save_name: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<GameState, String> {
    let ctx = CommandContext::new("load_game", app);
    
    ctx.events.command_started(ctx.correlation_id.clone(), ctx.command_name.clone());
    
    let result = async {
        // Validate save name
        if let Err(e) = Validator::validate_save_name(&save_name) {
            return Err(e.to_error_string(Some(ctx.correlation_id.clone())));
        }

        // Load save file
        let save_file = state.save_system.load(&save_name).map_err(|e| {
            let err = if e.to_string().contains("not found") {
                IPCErrorType::NotFound {
                    resource: format!("save file '{}'", save_name),
                }
            } else {
                IPCErrorType::FileSystem {
                    message: format!("Failed to load save: {}", e),
                }
            };
            err.to_error_string(Some(ctx.correlation_id.clone()))
        })?;

        // Acquire world lock
        let mut world = state.world.lock().map_err(|e| {
            let err = IPCErrorType::Internal {
                message: format!("Failed to acquire world lock: {}", e),
            };
            err.to_error_string(Some(ctx.correlation_id.clone()))
        })?;

        // Apply save to world
        state.save_system.apply_to_world(save_file.clone(), &mut world).map_err(|e| {
            let err = IPCErrorType::Internal {
                message: format!("Failed to apply save to world: {}", e),
            };
            err.to_error_string(Some(ctx.correlation_id.clone()))
        })?;

        // Convert to frontend GameState
        let world_state = save_file.world_state;
        let game_state = GameState {
            turn: world_state.game_time.turn,
            player_name: save_file.metadata.name.clone(),
            civilization: save_file.metadata.civilization.clone(),
            is_paused: world_state.game_time.is_paused(),
        };

        // Emit game state changed event
        ctx.events.game_state_changed(game_state.clone());
        
        // Emit success notification
        ctx.events.success(
            "Game Loaded".to_string(),
            format!("Game '{}' loaded successfully", save_name),
        );

        info!(
            target: "manifest::commands",
            correlation_id = %ctx.correlation_id,
            save_name = %save_name,
            turn = game_state.turn,
            "Game loaded successfully"
        );

        Ok(game_state)
    }.await;
    
    ctx.complete(&result);
    result
}

#[tauri::command]
pub async fn list_saves(state: State<'_, AppState>) -> Result<Vec<SaveInfo>, String> {
    let correlation_id = LoggingSystem::generate_correlation_id();
    
    debug!(
        target: "manifest::commands",
        correlation_id = correlation_id,
        command = "list_saves",
        "Retrieving available save files"
    );
    
    match state.save_system.list_saves().await {
        Ok(saves) => {
            debug!(
                target: "manifest::commands",
                correlation_id = correlation_id,
                save_count = saves.len(),
                "Save files retrieved successfully"
            );
            Ok(saves)
        }
        Err(e) => {
            error!(
                target: "manifest::commands",
                correlation_id = correlation_id,
                error = %e,
                "Failed to list save files"
            );
            Err(format!("Failed to list saves: {}", e))
        }
    }
}

/// Get hot reload statistics (debug builds only)
#[tauri::command]
#[cfg(debug_assertions)]
pub async fn get_reload_stats(state: State<'_, AppState>) -> Result<Option<crate::core::reloader::ReloadStats>, String> {
    let correlation_id = LoggingSystem::generate_correlation_id();
    
    debug!(
        target: "manifest::commands",
        correlation_id = correlation_id,
        command = "get_reload_stats",
        "Retrieving hot reload statistics"
    );
    
    let stats = {
        let world = state.world.lock().map_err(|e| {
            error!(
                target: "manifest::commands",
                correlation_id = correlation_id,
                error = %e,
                "Failed to acquire world lock for reload stats"
            );
            format!("Failed to lock world: {}", e)
        })?;
        
        world.reload_stats()
    }; // MutexGuard is dropped here
    
    debug!(
        target: "manifest::commands",
        correlation_id = correlation_id,
        has_stats = stats.is_some(),
        "Hot reload statistics retrieved"
    );
    
    Ok(stats)
}

/// Get scheduler performance metrics  
#[tauri::command]
pub async fn get_scheduler_metrics(state: State<'_, AppState>) -> Result<crate::core::SchedulerMetrics, String> {
    let correlation_id = LoggingSystem::generate_correlation_id();
    
    debug!(
        target: "manifest::commands",
        correlation_id = correlation_id,
        command = "get_scheduler_metrics",
        "Retrieving scheduler performance metrics"
    );
    
    let metrics = {
        let world = state.world.lock().map_err(|e| {
            error!(
                target: "manifest::commands", 
                correlation_id = correlation_id,
                error = %e,
                "Failed to acquire world lock for scheduler metrics"
            );
            format!("Failed to lock world: {}", e)
        })?;
        
        world.scheduler_metrics()
    }; // MutexGuard is dropped here
    
    debug!(
        target: "manifest::commands",
        correlation_id = correlation_id,
        tasks_executed = metrics.tasks_executed,
        avg_task_time_ms = metrics.average_task_time_ms,
        last_frame_time_ms = metrics.last_frame_time_ms,
        "Scheduler metrics retrieved"
    );
    
    Ok(metrics)
}

/// Save thumbnail metadata to an existing save file
#[tauri::command]
pub async fn save_thumbnail_metadata(
    save_name: String,
    thumbnail_data: SaveThumbnailMetadata,
    state: State<'_, AppState>
) -> Result<(), String> {
    let correlation_id = LoggingSystem::generate_correlation_id();
    
    info!(
        target: "manifest::commands",
        correlation_id = correlation_id,
        command = "save_thumbnail_metadata",
        save_name = %save_name,
        "Saving thumbnail metadata"
    );
    
    state.save_system.add_thumbnail(&save_name, thumbnail_data)
        .map_err(|e| {
            error!(
                target: "manifest::commands",
                correlation_id = correlation_id,
                save_name = %save_name,
                error = %e,
                "Failed to save thumbnail metadata"
            );
            format!("Failed to save thumbnail: {}", e)
        })?;
    
    info!(
        target: "manifest::commands",
        correlation_id = correlation_id,
        save_name = %save_name,
        "Thumbnail metadata saved successfully"
    );
    
    Ok(())
}

/// Load thumbnail metadata from a save file
#[tauri::command]
pub async fn load_thumbnail_metadata(
    save_name: String,
    state: State<'_, AppState>
) -> Result<Option<SaveThumbnailMetadata>, String> {
    let correlation_id = LoggingSystem::generate_correlation_id();
    
    debug!(
        target: "manifest::commands",
        correlation_id = correlation_id,
        command = "load_thumbnail_metadata",
        save_name = %save_name,
        "Loading thumbnail metadata"
    );
    
    let thumbnail = state.save_system.get_thumbnail(&save_name)
        .map_err(|e| {
            error!(
                target: "manifest::commands",
                correlation_id = correlation_id,
                save_name = %save_name,
                error = %e,
                "Failed to load thumbnail metadata"
            );
            format!("Failed to load thumbnail: {}", e)
        })?;
    
    if thumbnail.is_some() {
        debug!(
            target: "manifest::commands",
            correlation_id = correlation_id,
            save_name = %save_name,
            "Thumbnail metadata loaded successfully"
        );
    }
    
    Ok(thumbnail)
}

/// Batch command execution (for frontend batch operations)
#[derive(Debug, Deserialize)]
pub struct BatchCommandRequest {
    pub commands: Vec<BatchCommand>,
    pub options: BatchOptions,
}

#[derive(Debug, Deserialize)]
pub struct BatchCommand {
    pub name: String,
    pub input: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct BatchOptions {
    pub parallel: Option<bool>,
    pub fail_fast: Option<bool>,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct BatchCommandResponse {
    pub results: Vec<BatchCommandResult>,
    pub summary: BatchSummary,
}

#[derive(Debug, Serialize)]
pub struct BatchCommandResult {
    pub success: bool,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct BatchSummary {
    pub total_commands: usize,
    pub successful_commands: usize,
    pub failed_commands: usize,
    pub total_duration_ms: u64,
}

/// Execute multiple commands in batch
#[tauri::command]
#[instrument(skip(app_state, app))]
pub async fn execute_batch_commands(
    request: BatchCommandRequest,
    app_state: State<'_, AppState>,
    app: AppHandle,
) -> Result<BatchCommandResponse, String> {
    let ctx = CommandContext::new("execute_batch_commands", app);
    
    ctx.events.command_started(ctx.correlation_id.clone(), ctx.command_name.clone());
    
    let batch_id = ctx.correlation_id.clone();
    let command_count = request.commands.len();
    let start_time = Instant::now();
    
    let result = async {
        if request.commands.is_empty() {
            let err = IPCErrorType::Validation {
                message: "Batch cannot be empty".to_string(),
                field: Some("commands".to_string()),
            };
            return Err(err.to_error_string(Some(ctx.correlation_id.clone())));
        }

        if request.commands.len() > 100 {
            let err = IPCErrorType::Validation {
                message: "Batch cannot contain more than 100 commands".to_string(),
                field: Some("commands".to_string()),
            };
            return Err(err.to_error_string(Some(ctx.correlation_id.clone())));
        }

        let mut results = Vec::new();
        let mut successful_commands = 0;
        let parallel = request.options.parallel.unwrap_or(false);
        let fail_fast = request.options.fail_fast.unwrap_or(false);

        info!(
            target: "manifest::commands",
            correlation_id = %ctx.correlation_id,
            batch_id = %batch_id,
            command_count = command_count,
            parallel = parallel,
            "Starting batch command execution"
        );

        if parallel {
            // TODO: Implement parallel execution
            // For now, execute sequentially
            warn!("Parallel batch execution not yet implemented, falling back to sequential");
        }

        // Execute commands sequentially
        for (index, command) in request.commands.iter().enumerate() {
            let command_start = Instant::now();
            
            debug!(
                target: "manifest::commands",
                correlation_id = %ctx.correlation_id,
                batch_id = %batch_id,
                command_index = index,
                command_name = %command.name,
                "Executing batch command"
            );

            // Here you would route to the appropriate command based on command.name
            // For now, we'll simulate command execution
            let (success, output, error) = match command.name.as_str() {
                "greet" => {
                    // Simulate greet command
                    if let Ok(name) = serde_json::from_value::<String>(command.input.clone()) {
                        (true, Some(serde_json::Value::String(format!("Hello, {}!", name))), None)
                    } else {
                        (false, None, Some("Invalid input for greet command".to_string()))
                    }
                }
                _ => {
                    (false, None, Some(format!("Unknown command: {}", command.name)))
                }
            };

            let duration_ms = command_start.elapsed().as_millis() as u64;

            let result = BatchCommandResult {
                success,
                output,
                error: error.clone(),
                duration_ms,
            };

            if success {
                successful_commands += 1;
            } else if fail_fast {
                results.push(result);
                break;
            }

            results.push(result);
        }

        let total_duration_ms = start_time.elapsed().as_millis() as u64;
        let failed_commands = command_count - successful_commands;

        let summary = BatchSummary {
            total_commands: command_count,
            successful_commands,
            failed_commands,
            total_duration_ms,
        };

        // Emit batch completed event
        ctx.events.batch_completed(batch_id, command_count, total_duration_ms, successful_commands);

        info!(
            target: "manifest::commands",
            correlation_id = %ctx.correlation_id,
            successful_commands = successful_commands,
            failed_commands = failed_commands,
            total_duration_ms = total_duration_ms,
            "Batch command execution completed"
        );

        Ok(BatchCommandResponse {
            results,
            summary,
        })
    }.await;
    
    ctx.complete(&result);
    result
}

/// Health check command for connection monitoring
#[tauri::command]
#[instrument(skip(app))]
pub async fn health_check(app: AppHandle) -> Result<serde_json::Value, String> {
    let ctx = CommandContext::new("health_check", app);
    
    let result = async {
        let health_info = serde_json::json!({
            "status": "healthy",
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
            "version": env!("CARGO_PKG_VERSION"),
        });

        Ok(health_info)
    }.await;
    
    ctx.complete(&result);
    result
}

// Re-export tile streaming commands
pub use tile_streaming::{
    stream_tiles, get_tile, get_tile_updates,
    TileStreamingRequest, TileStreamingResponse,
    GameTile, TileInstanceData, TileUpdateBatch,
};

