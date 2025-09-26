#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{path::PathBuf, sync::Mutex};
use tauri::Manager;
use tracing::info;

use manifest::{
    ecs::{GameWorld, SaveSystem},
    core::{
        logging::{LoggingSystem, LoggingConfig},
        caching::{GameCacheBuilder},
    },
};

// Import commands module directly to ensure proper macro expansion
use manifest::commands::{self, AppState};

#[cfg(feature = "bench")]
use manifest::core::benchmarks;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // Initialize basic console-only logging to avoid Tokio runtime issues
            let mut logging_config = if cfg!(debug_assertions) {
                LoggingConfig::development()
            } else {
                LoggingConfig::production()
            };
            
            // Disable file logging temporarily to avoid async runtime issues during startup
            logging_config.files.clear();
            
            let _logging_system = LoggingSystem::init(logging_config)
                .expect("Failed to initialize logging system");

            info!(
                target: "manifest::main",
                version = env!("CARGO_PKG_VERSION"),
                mode = if cfg!(debug_assertions) { "development" } else { "production" },
                "🎮 Manifest - Grand Strategy Game"
            );
            info!("==================================");
            
            // Run quick performance test for hashing
            #[cfg(feature = "bench")]
            benchmarks::quick_performance_test();
            
            // Initialize the game systems...
            info!(
                target: "manifest::core",
                subsystem = "initialization",
                "🔧 Initializing core systems with optimized hashing..."
            );
            
            // Continue with existing setup logic...
            // Get application data directory for saves
            let app_data_dir = app.path().app_data_dir()
                .unwrap_or_else(|_| PathBuf::from("./saves"));
            
            let saves_dir = app_data_dir.join("saves");
            
            // Initialize save system
            let save_system = SaveSystem::new(saves_dir.clone())
                .expect("Failed to initialize save system");
            
            info!(
                target: "manifest::saves",
                saves_dir = ?saves_dir,
                "Save system initialized"
            );
            
            // Initialize game world
            let world = GameWorld::new();
            
            info!(
                target: "manifest::ecs",
                "Game world initialized with ECS systems"
            );
            
            // Initialize command cache
            let command_cache = GameCacheBuilder::new()
                .max_memory_mb(128)
                .default_ttl(std::time::Duration::from_secs(300))
                .enable_metrics(true)
                .build();
            
            info!(
                target: "manifest::cache",
                "Command cache initialized with 128MB limit"
            );
            
            // Create application state
            let state = AppState {
                world: Mutex::new(world),
                command_cache,
                save_system,
            };
            
            // Manage state globally
            app.manage(state);
            
            info!(
                target: "manifest::main",
                status = "ready",
                components = ?["ecs", "saves", "ui"],
                "✅ Game ready to launch!"
            );
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::greet,
            commands::get_game_state,
            commands::initialize_game,
            commands::save_game,
            commands::load_game,
            commands::list_saves,
            commands::get_scheduler_metrics,
            // Save thumbnail commands
            commands::save_thumbnail_metadata,
            commands::load_thumbnail_metadata,
            // Tile streaming commands
            commands::tile_streaming::stream_tiles,
            commands::tile_streaming::get_tile,
            commands::tile_streaming::get_tile_updates,
            // Enhanced IPC commands
            commands::execute_batch_commands,
            commands::health_check,
            #[cfg(debug_assertions)]
            commands::get_reload_stats,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}