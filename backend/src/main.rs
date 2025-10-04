#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{path::PathBuf, sync::Mutex};
use tauri::Manager;
use tracing::info;

use manifest::{
    ecs::{GameWorld, SaveSystem},
    core::{
        logging::{LoggingSystem, LoggingConfig},
        caching::GameCacheBuilder,
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
            
            // Generate diverse terrain with rivers and biomes
            println!("🌍 WORLD: Generating diverse terrain with rivers and biomes...");
            populate_diverse_terrain(&tile_manager);
            
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

/// Generate diverse terrain with rivers, biomes, and varied elevation (simplified safe version)
fn populate_diverse_terrain(tile_manager: &TileComponentManager) {
    use tracing::info;

    println!("🌍 DIVERSE: Starting diverse terrain generation with rivers and biomes...");
    info!("🌍 Starting diverse terrain generation");

    let world_size = 75i32;
    let mut tiles_created = 0;
    let mut rivers_placed = 0;
    let mut mountains_placed = 0;
    
    println!("🌊 RIVERS: Generating river networks...");
    println!("🌿 BIOMES: Generating biome diversity...");  
    println!("⛰️ ELEVATION: Creating elevation variations...");

    // Generate diverse terrain using safe deterministic algorithms
    for r in -world_size..=world_size {
        for q in -world_size..=world_size {
            let hex = HexCoord { q, r };
            let chunk = ChunkCoord { x: q / 32, y: r / 32 };
            let local_x = ((q % 32 + 32) % 32) as u8;
            let local_y = ((r % 32 + 32) % 32) as u8;
            
            // Generate diverse terrain using safe algorithms
            let terrain_result = generate_diverse_terrain_at(q, r);
            
            // Track special terrain types
            match terrain_result.terrain_type {
                TerrainType::River => rivers_placed += 1,
                TerrainType::Mountain | TerrainType::Mountains => mountains_placed += 1,
                _ => {}
            }

            tile_manager.create_tile(hex, chunk, local_x, local_y, terrain_result.terrain_type);
            tiles_created += 1;
        }
    }

    println!("✅ DIVERSE: Generated diverse world!");
    println!("   📊 {} total tiles created", tiles_created);
    println!("   🏔️ {} mountain tiles", mountains_placed);
    println!("   🌊 {} river tiles", rivers_placed);
    
    info!("✅ Diverse terrain generation completed successfully");
}

/// Terrain generation result with all necessary data
#[derive(Debug)]
struct TerrainResult {
    terrain_type: TerrainType,
    elevation: f32,
    resource_mask: u32,
    biome_id: u8,
}

/// Generate diverse terrain at a specific hex coordinate using safe deterministic algorithms
fn generate_diverse_terrain_at(q: i32, r: i32) -> TerrainResult {
    // Multiple layers of noise for realistic terrain
    let seed1 = ((q * 73 + r * 37) as u32).wrapping_mul(1103515245).wrapping_add(12345);
    let seed2 = ((q * 131 + r * 97) as u32).wrapping_mul(1664525).wrapping_add(1013904223);
    let seed3 = ((q * 179 + r * 163) as u32).wrapping_mul(214013).wrapping_add(2531011);
    let seed4 = ((q * 211 + r * 199) as u32).wrapping_mul(1140671485).wrapping_add(12820163);
    
    // Create different noise layers
    let elevation_noise = ((seed1 % 1000) as f32) / 1000.0;
    let moisture_noise = ((seed2 % 1000) as f32) / 1000.0;
    let temperature_noise = ((seed3 % 1000) as f32) / 1000.0;
    let feature_noise = ((seed4 % 1000) as f32) / 1000.0;
    
    // Distance from origin for continental features
    let distance_from_origin = ((q * q + r * r + q * r) as f32).sqrt();
    let distance_factor = (distance_from_origin / 100.0).min(1.0);
    
    // Create river networks using deterministic patterns
    let river_seed = (q * 17 + r * 23 + q * r * 7) % 1009;
    let is_river_tile = river_seed % 200 < 8 && // 4% chance base
                       ((q + r) % 7 == 0 || (q - r) % 9 == 1 || (q * 2 + r) % 11 == 3); // River patterns
    
    if is_river_tile {
        return TerrainResult {
            terrain_type: TerrainType::River,
            elevation: -0.1 + elevation_noise * 0.1,
            resource_mask: if feature_noise > 0.9 { 1 << 7 } else { 0 }, // Fish resources
            biome_id: 0, // Water biome
        };
    }
    
    // Continental shelf - ocean at edges
    let continental_factor = 1.0 - (distance_factor * 0.7);
    let is_ocean_likely = elevation_noise + continental_factor < 0.3;
    
    if is_ocean_likely {
        return TerrainResult {
            terrain_type: if elevation_noise > 0.25 { TerrainType::Coast } else { TerrainType::Ocean },
            elevation: -0.3 + elevation_noise * 0.2,
            resource_mask: if feature_noise > 0.8 { 1 << 6 } else { 0 }, // Sea resources
            biome_id: 0,
        };
    }
    
    // Land-based terrain with elevation and climate
    let temperature = temperature_noise + (distance_factor * 0.3); // Colder at edges
    let moisture = moisture_noise;
    let base_elevation = elevation_noise + (distance_factor * 0.2);
    
    // Mountain ranges using patterns
    let mountain_pattern = (q / 8 + r / 6) % 13;
    let is_mountain_range = mountain_pattern < 2 && base_elevation > 0.6;
    
    if is_mountain_range {
        let terrain_type = if base_elevation > 0.8 { TerrainType::Mountain } else { TerrainType::Hills };
        return TerrainResult {
            terrain_type,
            elevation: 0.6 + base_elevation * 0.4,
            resource_mask: if feature_noise > 0.7 { 1 << 0 | 1 << 2 } else { 0 }, // Stone, Metal
            biome_id: if temperature < 0.3 { 5 } else { 8 }, // Snow or Hills biome
        };
    }
    
    // Biome determination based on temperature and moisture
    let (terrain_type, biome_id, elevation, resource_mask) = match (temperature, moisture, base_elevation) {
        // High elevation -> mountains or hills
        (_, _, e) if e > 0.75 => (TerrainType::Mountain, 9, 0.8 + e * 0.2, if feature_noise > 0.8 { 1 << 0 } else { 0 }),
        (_, _, e) if e > 0.6 => (TerrainType::Hills, 8, 0.5 + e * 0.3, if feature_noise > 0.85 { 1 << 2 } else { 0 }),
        
        // Snow and tundra (cold climates)
        (t, _, e) if t < 0.2 && e > 0.3 => (TerrainType::Snow, 5, 0.3 + e * 0.4, 0),
        (t, _, _) if t < 0.25 => (TerrainType::Tundra, 4, 0.1 + base_elevation * 0.3, 0),
        
        // Hot, dry -> desert
        (t, m, _) if t > 0.7 && m < 0.3 => (TerrainType::Desert, 3, 0.2 + base_elevation * 0.2, if feature_noise > 0.9 { 1 << 5 } else { 0 }),
        
        // Hot, wet -> jungle
        (t, m, _) if t > 0.6 && m > 0.7 => (TerrainType::Jungle, 7, 0.1 + base_elevation * 0.3, if feature_noise > 0.75 { 1 << 1 | 1 << 4 } else { 0 }),
        
        // Medium temperature, high moisture -> forest
        (t, m, _) if m > 0.6 && t > 0.3 && t < 0.7 => (TerrainType::Forest, 6, 0.15 + base_elevation * 0.35, if feature_noise > 0.8 { 1 << 1 } else { 0 }),
        
        // Medium moisture -> grassland
        (_, m, _) if m > 0.4 && m < 0.7 => (TerrainType::Grassland, 1, 0.05 + base_elevation * 0.25, if feature_noise > 0.85 { 1 << 3 } else { 0 }),
        
        // Default -> plains
        _ => (TerrainType::Plains, 2, 0.1 + base_elevation * 0.2, if feature_noise > 0.9 { 1 << 3 } else { 0 }),
    };
    
    TerrainResult {
        terrain_type,
        elevation,
        resource_mask,
        biome_id,
    }
}


/// Populate initial terrain for testing (fallback)
fn populate_initial_terrain(tile_manager: &TileComponentManager) {
    use tracing::info;

    println!("🌍 TERRAIN: Generating fallback terrain...");
    info!("🌍 Generating fallback terrain...");

    // Generate a large rectangular world (150x150 hex grid for proper streaming)
    let world_size = 75i32;
    let mut tiles_created = 0;

    for q in -world_size..=world_size {
        for r in -world_size..=world_size {
            let hex = HexCoord { q, r };
            let chunk = ChunkCoord { x: q / 32, y: r / 32 };
            let local_x = ((q % 32 + 32) % 32) as u8;
            let local_y = ((r % 32 + 32) % 32) as u8;

            // Generate realistic terrain using multiple noise layers
            let terrain_type = generate_realistic_terrain(q, r);

            tile_manager.create_tile(hex, chunk, local_x, local_y, terrain_type);
            tiles_created += 1;
        }
    }

    println!("✅ TERRAIN: Generated {} terrain tiles in {}x{} world", tiles_created, world_size * 2 + 1, world_size * 2 + 1);
    info!("✅ Generated {} terrain tiles in {}x{} world", tiles_created, world_size * 2 + 1, world_size * 2 + 1);
}

/// Generate realistic terrain using layered noise and biome logic
fn generate_realistic_terrain(q: i32, r: i32) -> TerrainType {
    // Create deterministic pseudo-random based on coordinates
    let seed1 = ((q * 73 + r * 37) as u32).wrapping_mul(1103515245).wrapping_add(12345);
    let seed2 = ((q * 131 + r * 97) as u32).wrapping_mul(1664525).wrapping_add(1013904223);
    let seed3 = ((q * 179 + r * 163) as u32).wrapping_mul(214013).wrapping_add(2531011);
    
    // Create different noise layers
    let elevation_noise = ((seed1 % 1000) as f32) / 1000.0;
    let moisture_noise = ((seed2 % 1000) as f32) / 1000.0;
    let temperature_noise = ((seed3 % 1000) as f32) / 1000.0;
    
    // Add distance from origin for some variety
    let distance_from_origin = ((q * q + r * r + q * r) as f32).sqrt();
    let distance_factor = (distance_from_origin / 100.0).min(1.0);
    
    // Create continental landmasses (most land near origin, ocean at edges)
    let continental_factor = 1.0 - (distance_factor * 0.7);
    let is_ocean_likely = elevation_noise + continental_factor < 0.3;
    
    if is_ocean_likely {
        // Ocean or coastal areas
        if elevation_noise > 0.25 {
            TerrainType::Coast
        } else {
            TerrainType::Ocean
        }
    } else {
        // Land-based terrain determined by temperature and moisture
        let temperature = temperature_noise + (distance_factor * 0.3); // Colder at edges
        let moisture = moisture_noise;
        let elevation = elevation_noise + (distance_factor * 0.2);
        
        match (temperature, moisture, elevation) {
            // High elevation -> mountains or hills
            (_, _, e) if e > 0.8 => TerrainType::Mountain,
            (_, _, e) if e > 0.65 => TerrainType::Hills,
            
            // Snow in cold areas with high elevation
            (t, _, e) if t < 0.2 && e > 0.4 => TerrainType::Snow,
            (t, _, _) if t < 0.15 => TerrainType::Tundra,
            
            // Hot, dry -> desert
            (t, m, _) if t > 0.7 && m < 0.3 => TerrainType::Desert,
            
            // Hot, wet -> jungle
            (t, m, _) if t > 0.6 && m > 0.7 => TerrainType::Jungle,
            
            // Medium temperature, high moisture -> forest
            (t, m, _) if m > 0.6 && t > 0.3 && t < 0.7 => TerrainType::Forest,
            
            // Medium moisture -> grassland
            (_, m, _) if m > 0.4 && m < 0.7 => TerrainType::Grassland,
            
            // Default -> plains
            _ => TerrainType::Plains,
        }
    }
}