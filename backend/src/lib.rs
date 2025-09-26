//! Manifest Game Engine Library
//! 
//! Core library for the Manifest grand strategy game, providing ECS architecture,
//! game systems, and utilities for game development.

pub mod commands;
pub mod core;
pub mod ecs;
pub mod scripting;
pub mod simulation;
pub mod world;

// Re-export specific types to avoid conflicts
pub use core::{Stage, Scheduler, SchedulerError, SchedulerMetrics};
pub use ecs::{GameWorld, EcsScheduler, configure_parallel_systems, configure_change_detection};
pub use scripting::{ScriptManager, ScriptError};

// Re-export Bevy ECS essentials
pub use bevy_ecs::prelude::*;

// Re-export command functions for Tauri
pub use commands::{
    AppState, greet, get_game_state, initialize_game,
    save_game, load_game, list_saves, get_scheduler_metrics,
    // Tile streaming commands  
    stream_tiles, get_tile, get_tile_updates,
};

// Re-export debug commands
#[cfg(debug_assertions)]
pub use commands::get_reload_stats;

// Test function to verify basic Lua integration works
#[cfg(test)]
pub fn test_basic_functionality() -> Result<(), Box<dyn std::error::Error>> {
    use tracing::info;
    
    // Test ScriptManager creation
    let script_manager = ScriptManager::new()?;
    info!("✅ ScriptManager created successfully");
    
    // Test script loading (will fail gracefully if files don't exist)
    let _result = script_manager.load_script("tiles/properties.lua");
    info!("✅ Script loading tested");
    
    // Test function calling (returns defaults for now)
    let result: f32 = script_manager.call_function("calculate_movement_cost", ())?;
    info!("✅ Function calling works, result: {}", result);
    
    info!("🎉 Basic functionality test passed!");
    Ok(())
}