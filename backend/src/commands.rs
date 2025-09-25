use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::Instant;
use tauri::State;
use tracing::{info, error, warn, debug, instrument};

use crate::ecs::{GameWorld, SaveInfo, SaveSystem};
use crate::core::{logging::{LoggingSystem, game_logging}, caching::{GameCache, GameCacheBuilder, CacheKey, PlayerCacheKey, CachePriority}};

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
#[instrument(name = "greet_command", fields(player_name = name))]
pub fn greet(name: &str) -> String {
    let correlation_id = LoggingSystem::generate_correlation_id();
    
    info!(
        target: "manifest::commands",
        correlation_id = correlation_id,
        player_name = name,
        command = "greet",
        "Player greeting initiated"
    );
    
    format!("Hello, {}! Welcome to Manifest.", name)
}

#[tauri::command]
#[instrument(name = "get_game_state")]
pub async fn get_game_state(app_state: State<'_, AppState>) -> Result<GameState, String> {
    let correlation_id = LoggingSystem::generate_correlation_id();
    
    debug!(
        target: "manifest::commands",
        correlation_id = correlation_id,
        command = "get_game_state",
        "Retrieving current game state"
    );
    
    // Try cache first for game state
    let cache_key = CacheKey::Player(PlayerCacheKey::resources(1, 1)); // Simplified - would use actual turn
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
    let world = app_state.world.lock().unwrap();
    let state = GameState {
        turn: world.get_turn(),
        player_name: "Player".to_string(),
        civilization: "Ancient Empire".to_string(),
        is_paused: world.is_paused(),
    };
    drop(world);
    
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
#[instrument(name = "initialize_game", fields(player_name = %player_name, civilization = %civilization))]
pub async fn initialize_game(player_name: String, civilization: String, app_state: State<'_, AppState>) -> Result<GameState, String> {
    let correlation_id = LoggingSystem::generate_correlation_id();
    
    info!(
        target: "manifest::commands",
        correlation_id = correlation_id,
        command = "initialize_game",
        player_name = %player_name,
        civilization = %civilization,
        "Initializing new game"
    );
    
    // Initialize game state
    let game_state = GameState {
        turn: 1,
        player_name: player_name.clone(),
        civilization: civilization.clone(),
        is_paused: false,
    };
    
    info!(
        target: "manifest::commands",
        correlation_id = correlation_id,
        player_name = %player_name,
        civilization = %civilization,
        turn = game_state.turn,
        "Game initialization completed successfully"
    );
    
    Ok(game_state)
}

#[tauri::command]
#[instrument(name = "save_game", fields(save_name = %save_name))]
pub async fn save_game(
    save_name: String, 
    state: State<'_, AppState>
) -> Result<String, String> {
    let correlation_id = LoggingSystem::generate_correlation_id();
    
    info!(
        target: "manifest::commands",
        correlation_id = correlation_id,
        command = "save_game",
        save_name = %save_name,
        "Initiating game save operation"
    );
    
    let mut world = state.world.lock().map_err(|e| {
        error!(
            target: "manifest::commands",
            correlation_id = correlation_id,
            error = %e,
            "Failed to acquire world lock for save operation"
        );
        format!("Failed to lock world: {}", e)
    })?;
    
    match state.save_system.save(&mut *world, &save_name) {
        Ok(path) => {
            info!(
                target: "manifest::commands",
                correlation_id = correlation_id,
                save_name = %save_name,
                save_path = ?path,
                "Game saved successfully"
            );
            Ok(format!("Game saved as: {}", save_name))
        }
        Err(e) => {
            error!(
                target: "manifest::commands",
                correlation_id = correlation_id,
                save_name = %save_name,
                error = %e,
                "Failed to save game"
            );
            Err(format!("Failed to save game: {}", e))
        }
    }
}

#[tauri::command]
#[instrument(name = "load_game", fields(save_name = %save_name))]
pub async fn load_game(
    save_name: String,
    state: State<'_, AppState>
) -> Result<GameState, String> {
    let correlation_id = LoggingSystem::generate_correlation_id();
    
    info!(
        target: "manifest::commands",
        correlation_id = correlation_id,
        command = "load_game",
        save_name = %save_name,
        "Initiating game load operation"
    );
    
    let save_file = state.save_system.load(&save_name)
        .map_err(|e| {
            error!(
                target: "manifest::commands",
                correlation_id = correlation_id,
                save_name = %save_name,
                error = %e,
                "Failed to load save file"
            );
            format!("Failed to load save: {}", e)
        })?;
    
    let mut world = state.world.lock()
        .map_err(|e| {
            error!(
                target: "manifest::commands",
                correlation_id = correlation_id,
                error = %e,
                "Failed to acquire world lock for load operation"
            );
            format!("Failed to lock world: {}", e)
        })?;
    
    state.save_system.apply_to_world(save_file.clone(), &mut world)
        .map_err(|e| {
            error!(
                target: "manifest::commands",
                correlation_id = correlation_id,
                save_name = %save_name,
                error = %e,
                "Failed to apply save data to world"
            );
            format!("Failed to apply save to world: {}", e)
        })?;
    
    // Convert world state to GameState for frontend
    let world_state = save_file.world_state;
    let game_state = GameState {
        turn: world_state.game_time.turn,
        player_name: save_file.metadata.name.clone(),
        civilization: "Ancient Empire".to_string(), // TODO: Add to save metadata
        is_paused: world_state.game_time.paused,
    };
    
    info!(
        target: "manifest::commands",
        correlation_id = correlation_id,
        save_name = %save_name,
        turn = game_state.turn,
        player_name = %game_state.player_name,
        "Game loaded successfully"
    );
    
    Ok(game_state)
}

#[tauri::command]
#[instrument(name = "list_saves")]
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
#[instrument(name = "get_reload_stats")]
#[cfg(debug_assertions)]
pub async fn get_reload_stats(state: State<'_, AppState>) -> Result<Option<crate::core::reloader::ReloadStats>, String> {
    let correlation_id = LoggingSystem::generate_correlation_id();
    
    debug!(
        target: "manifest::commands",
        correlation_id = correlation_id,
        command = "get_reload_stats",
        "Retrieving hot reload statistics"
    );
    
    let world = state.world.lock().map_err(|e| {
        error!(
            target: "manifest::commands",
            correlation_id = correlation_id,
            error = %e,
            "Failed to acquire world lock for reload stats"
        );
        format!("Failed to lock world: {}", e)
    })?;
    
    let stats = world.reload_stats();
    
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
#[instrument(name = "get_scheduler_metrics")]
pub async fn get_scheduler_metrics(state: State<'_, AppState>) -> Result<crate::core::SchedulerMetrics, String> {
    let correlation_id = LoggingSystem::generate_correlation_id();
    
    debug!(
        target: "manifest::commands",
        correlation_id = correlation_id,
        command = "get_scheduler_metrics",
        "Retrieving scheduler performance metrics"
    );
    
    let world = state.world.lock().map_err(|e| {
        error!(
            target: "manifest::commands", 
            correlation_id = correlation_id,
            error = %e,
            "Failed to acquire world lock for scheduler metrics"
        );
        format!("Failed to lock world: {}", e)
    })?;
    
    let metrics = world.scheduler_metrics();
    
    debug!(
        target: "manifest::commands",
        correlation_id = correlation_id,
        tasks_executed = metrics.tasks_executed,
        avg_task_time_ms = metrics.average_task_time.as_millis(),
        last_frame_time_ms = metrics.last_frame_time.as_millis(),
        "Scheduler metrics retrieved"
    );
    
    Ok(metrics)
}
