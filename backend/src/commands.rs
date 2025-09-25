use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::Instant;
use tauri::State;
use tracing::{info, error, debug};

use crate::ecs::{GameWorld, SaveInfo, SaveSystem};
use crate::core::{logging::LoggingSystem, caching::{GameCache, CacheKey, PlayerCacheKey, CachePriority}};

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
pub async fn initialize_game(player_name: String, civilization: String, _app_state: State<'_, AppState>) -> Result<GameState, String> {
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
        civilization: save_file.metadata.civilization.clone(),
        is_paused: world_state.game_time.is_paused(),
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
