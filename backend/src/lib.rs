//! Manifest Game Engine Library
//! 
//! Core library for the Manifest grand strategy game, providing ECS architecture,
//! game systems, and utilities for game development.

pub mod commands;
pub mod core;
pub mod ecs;
pub mod simulation;

// Re-export specific types to avoid conflicts
pub use core::{Stage, Scheduler, SchedulerError, SchedulerMetrics};
pub use ecs::{GameWorld, EcsScheduler, configure_parallel_systems, configure_change_detection};

// Re-export Bevy ECS essentials
pub use bevy_ecs::prelude::*;

// Re-export command functions for Tauri
pub use commands::{
    AppState, greet, get_game_state, initialize_game,
    save_game, load_game, list_saves, get_scheduler_metrics,
};

// Re-export debug commands
#[cfg(debug_assertions)]
pub use commands::get_reload_stats;
