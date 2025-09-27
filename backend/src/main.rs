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
use manifest::world::tiles::{TileComponentManager, TerrainType, ChunkCoord};
use manifest::core::zig_ffi::HexCoord;

#[cfg(feature = "bench")]
use manifest::core::benchmarks;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // Startup timestamp for debugging
            let startup_time = std::time::SystemTime::now();
            println!("⏰ STARTUP: Application setup beginning at {:?}", startup_time);
            // Initialize basic console-only logging to avoid Tokio runtime issues
            let mut logging_config = if cfg!(debug_assertions) {
                LoggingConfig::development()
            } else {
                LoggingConfig::production()
            };
            
            // Disable async file logging to avoid Tokio runtime issues during startup
            // The logging system tries to spawn async tasks before Tauri's runtime is ready
            logging_config.files.clear();
            
            println!("⚠️  FILE LOGGING DISABLED: Async file logging conflicts with Tauri startup");
            println!("🔧 WORKAROUND: All logs will go to console only during this session");
            
            let _logging_system = LoggingSystem::init(logging_config)
                .expect("Failed to initialize logging system");

            // Immediate console output to verify logging is working
            println!("🚀 STARTUP: Manifest backend starting...");
            eprintln!("🔍 DEBUG: Backend console logging active");

            // Force console output for debugging
            println!("🎮 Manifest - Grand Strategy Game v{}", env!("CARGO_PKG_VERSION"));
            println!("===========================================");
            println!("Mode: {}", if cfg!(debug_assertions) { "development" } else { "production" });
            
            info!(
                target: "manifest::main",
                version = env!("CARGO_PKG_VERSION"),
                mode = if cfg!(debug_assertions) { "development" } else { "production" },
                "🎮 Manifest - Grand Strategy Game"
            );
            info!("==================================");
            
            // Auto-open devtools in debug mode
            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
                println!("🔧 DEV: Developer tools opened automatically");
            }
            
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
            println!("💾 SETUP: Configuring save system...");
            let app_data_dir = app.path().app_data_dir()
                .unwrap_or_else(|_| PathBuf::from("./saves"));
            
            let saves_dir = app_data_dir.join("saves");
            println!("📁 SAVES DIR: {:?}", saves_dir);
            
            // Initialize save system
            let save_system = SaveSystem::new(saves_dir.clone())
                .expect("Failed to initialize save system");
            
            info!(
                target: "manifest::saves",
                saves_dir = ?saves_dir,
                "Save system initialized"
            );
            
            // Initialize game world
            println!("🌍 ECS: Initializing game world...");
            let world = GameWorld::new();
            println!("✅ ECS: Game world created successfully");
            
            info!(
                target: "manifest::ecs",
                "Game world initialized with ECS systems"
            );
            
            // Initialize command cache
            println!("💾 CACHE: Setting up command cache...");
            let command_cache = GameCacheBuilder::new()
                .max_memory_mb(128)
                .default_ttl(std::time::Duration::from_secs(300))
                .enable_metrics(true)
                .build();
            println!("✅ CACHE: Command cache initialized with 128MB limit");
            
            info!(
                target: "manifest::cache",
                "Command cache initialized with 128MB limit"
            );
            
            // Initialize tile component manager
            println!("🌐 TILES: Initializing tile component manager...");
            let tile_manager = std::sync::Arc::new(manifest::world::tiles::TileComponentManager::new());
            println!("✅ TILES: Tile component manager created");
            
            // Populate some initial terrain for testing
            populate_initial_terrain(&tile_manager);
            
            // Create application state
            println!("🔧 STATE: Creating application state...");
            let state = AppState {
                world: Mutex::new(world),
                command_cache,
                save_system,
                tile_manager,
            };
            
            // Manage state globally
            println!("📊 TAURI: Registering state with Tauri...");
            app.manage(state);
            
            println!("✅ READY: All systems initialized - Game ready to launch!");
            println!("📊 Active components: ECS, Saves, Cache, UI");
            println!("🎮 Manifest is now running and ready for interaction");
            
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

/// Populate initial terrain for testing
fn populate_initial_terrain(tile_manager: &TileComponentManager) {
    use std::sync::Arc;
    use tracing::info;

    info!("🌍 Generating initial terrain...");

    // Generate a reasonable sized world for testing (30x30 hex grid)
    let world_radius = 15i32;
    let mut tiles_created = 0;

    for q in -world_radius..=world_radius {
        for r in -world_radius..=world_radius {
            let s = -q - r;
            if s.abs() <= world_radius {
                let hex = HexCoord { q, r };
                let chunk = ChunkCoord { x: q / 32, y: r / 32 };
                let local_x = (q % 32) as u8;
                let local_y = (r % 32) as u8;

                // Generate varied terrain based on distance from origin
                let distance = ((q * q + r * r + q * r) as f32).sqrt();
                
                let terrain_type = match distance as i32 {
                    0..=3 => {
                        let seed = (q * 73 + r * 37) as u32;
                        match seed % 3 {
                            0 => TerrainType::Grassland,
                            1 => TerrainType::Plains,
                            _ => TerrainType::Forest,
                        }
                    },
                    4..=8 => {
                        let seed = (q * 73 + r * 37) as u32;
                        match seed % 4 {
                            0 => TerrainType::Forest,
                            1 => TerrainType::Hills,
                            2 => TerrainType::Plains,
                            _ => TerrainType::Grassland,
                        }
                    },
                    9..=12 => {
                        let seed = (q * 73 + r * 37) as u32;
                        match seed % 3 {
                            0 => TerrainType::Hills,
                            1 => TerrainType::Mountain,
                            _ => TerrainType::Desert,
                        }
                    },
                    _ => {
                        let seed = (q * 73 + r * 37) as u32;
                        if seed % 10 < 7 {
                            TerrainType::Ocean
                        } else {
                            TerrainType::Snow
                        }
                    }
                };

                tile_manager.create_tile(hex, chunk, local_x, local_y, terrain_type);
                tiles_created += 1;
            }
        }
    }

    info!("✅ Generated {} terrain tiles", tiles_created);
}