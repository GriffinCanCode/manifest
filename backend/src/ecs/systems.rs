//! Core game systems that operate on components and resources
//!
//! Systems contain the game logic and operate on entities with specific
//! component combinations. They are designed to be small, focused, and testable.

use bevy_ecs::prelude::*;
use tracing::{info, debug, warn, error, instrument, Span};

use crate::core::{Stage, SimulationState, logging::{LoggingSystem, game_logging}};
use crate::ecs::{components::*, resources::*, EcsScheduler, hierarchy::{sync_hierarchy_system, cleanup_hierarchy_system}};

/// Time management system with time controller integration
#[instrument(name = "time_system", skip_all)]
pub fn time_system(
    mut game_time: ResMut<GameTime>,
    simulation_state: Option<Res<SimulationState>>
) {
    let start_time = std::time::Instant::now();
    
    // Use default simulation if not available (for backward compatibility)
    let default_sim = SimulationState::new(42, None);
    let sim = simulation_state.as_deref().unwrap_or(&default_sim);
    
    let old_tick = game_time.tick;
    let old_turn = game_time.turn;
    let old_mode = game_time.playback_mode();
    
    // Update with time controller integration
    game_time.update(1.0 / 60.0, sim); // TODO: Get real delta time
    
    let new_mode = game_time.playback_mode();
    
    // Log tick changes
    if game_time.tick != old_tick {
        debug!(
            target: "game::systems::time",
            tick = game_time.tick,
            turn = game_time.turn,
            speed = game_time.speed(),
            delta_time = game_time.delta_time,
            mode = ?new_mode,
            "Game tick updated"
        );
    }
    
    // Log turn changes
    if game_time.turn != old_turn {
        info!(
            target: "game::systems::time",
            new_turn = game_time.turn,
            total_ticks = game_time.tick,
            "Turn advanced"
        );
    }
    
    // Log mode changes
    if new_mode != old_mode {
        info!(
            target: "game::systems::time",
            old_mode = ?old_mode,
            new_mode = ?new_mode,
            "Playback mode changed"
        );
    }
    
    let duration_ms = start_time.elapsed().as_secs_f64() * 1000.0;
    game_logging::log_performance_event("time_system", duration_ms, 1);
}

/// Interpolation system - updates interpolation factor for smooth rendering
#[instrument(name = "interpolation_system", skip_all)]
pub fn interpolation_system(
    mut game_time: ResMut<GameTime>,
    mut interpolated_positions: Query<&mut InterpolatedPosition>,
    mut interpolated_health: Query<&mut InterpolatedHealth>,
    mut interpolated_renderables: Query<&mut InterpolatedRenderable>,
) {
    let start_time = std::time::Instant::now();
    
    // Update interpolation factor based on time since last tick
    let current_time = instant::Instant::now();
    static mut LAST_TICK_TIME: Option<instant::Instant> = None;
    
    unsafe {
        if LAST_TICK_TIME.is_none() {
            LAST_TICK_TIME = Some(current_time);
        }
        
        if let Some(last_tick) = LAST_TICK_TIME {
            let time_since_tick = current_time.duration_since(last_tick).as_secs_f32();
            let tick_duration = 1.0 / 60.0; // Target 60 TPS
            game_time.update_interpolation(time_since_tick, tick_duration);
            
            // Reset tick time when we advance a tick
            if game_time.tick > 0 && time_since_tick >= tick_duration {
                LAST_TICK_TIME = Some(current_time);
            }
        }
    }
    
    // Note: Actual interpolation queries would happen in rendering systems
    // This system just updates the global interpolation factor
    
    let duration_ms = start_time.elapsed().as_secs_f64() * 1000.0;
    game_logging::log_performance_event("interpolation_system", duration_ms, 1);
    
    debug!(
        target: "game::systems::interpolation",
        interpolation_factor = game_time.interpolation_factor().into_inner(),
        interpolated_positions = interpolated_positions.iter().len(),
        interpolated_health = interpolated_health.iter().len(),
        interpolated_renderables = interpolated_renderables.iter().len(),
        "Interpolation factor updated"
    );
}

// Movement restoration, position sync, and health cleanup are now handled 
// by the unified_change_system in changes.rs - no duplicate systems needed

/// Selection validation system - removes invalid entities from selection
#[instrument(name = "selection_validation_system", skip_all)]
pub fn selection_validation_system(
    mut selection: ResMut<Selection>,
    entity_query: Query<Entity>,
) {
    let start_time = std::time::Instant::now();
    let initial_count = selection.entities.len();
    
    // Remove any selected entities that no longer exist
    selection.entities.retain(|&entity| {
        let exists = entity_query.get(entity).is_ok();
        if !exists {
            warn!(
                target: "game::systems::selection",
                entity = ?entity,
                "Removing invalid entity from selection"
            );
        }
        exists
    });
    
    let removed_count = initial_count - selection.entities.len();
    if removed_count > 0 {
        info!(
            target: "game::systems::selection",
            removed_entities = removed_count,
            remaining_entities = selection.entities.len(),
            "Cleaned up invalid selections"
        );
    }
    
    // Update primary selection if it's invalid
    if let Some(primary) = selection.primary {
        if entity_query.get(primary).is_err() {
            let old_primary = primary;
            selection.primary = selection.entities.first().copied();
            
            warn!(
                target: "game::systems::selection",
                old_primary = ?old_primary,
                new_primary = ?selection.primary,
                "Updated invalid primary selection"
            );
        }
    }
    
    let duration_ms = start_time.elapsed().as_secs_f64() * 1000.0;
    game_logging::log_performance_event("selection_validation_system", duration_ms, selection.entities.len());
}

/// Turn advancement system - advances to next turn when appropriate
#[instrument(name = "turn_advancement_system", skip_all)]
pub fn turn_advancement_system(
    mut game_time: ResMut<GameTime>,
    mut players: ResMut<Players>,
) {
    let start_time = std::time::Instant::now();
    
    // Simple turn advancement logic - in a real game this would be more complex
    // For now, advance turn every 3600 ticks (1 minute at 60 FPS)
    if game_time.tick >= 3600 && !game_time.paused {
        let old_turn = game_time.turn;
        let old_player = players.current_player;
        
        game_time.advance_turn();
        players.current_player = players.next_player();
        
        info!(
            target: "game::systems::turn",
            old_turn = old_turn,
            new_turn = game_time.turn,
            old_player = old_player,
            new_player = players.current_player,
            total_ticks = game_time.tick,
            "Turn advanced with player change"
        );
        
        // Log player action
        game_logging::log_entity_operation(
            bevy_ecs::entity::Entity::from_raw(players.current_player),
            "turn_start",
            Some(&format!("Player {} turn {} started", players.current_player, game_time.turn))
        );
        
        let duration_ms = start_time.elapsed().as_secs_f64() * 1000.0;
        game_logging::log_performance_event("turn_advancement_system", duration_ms, players.turn_order.len());
    } else {
        debug!(
            target: "game::systems::turn",
            tick = game_time.tick,
            paused = game_time.paused,
            "Turn advancement conditions not met"
        );
    }
}

// Debug logging is now handled by unified_change_system in changes.rs

/// System for managing camera controls (placeholder for now)
#[instrument(name = "camera_system", skip_all)]
pub fn camera_system(
    mut camera: ResMut<Camera>,
    selection: Res<Selection>,
    position_query: Query<&Position>,
) {
    let start_time = std::time::Instant::now();
    
    // Focus camera on primary selected entity
    if let Some(primary_entity) = selection.primary {
        if let Ok(position) = position_query.get(primary_entity) {
            let old_target = camera.target;
            camera.set_target(position.pixel());
            
            debug!(
                target: "game::systems::camera",
                entity = ?primary_entity,
                hex_pos = ?position.hex(),
                old_target = ?old_target,
                new_target = ?camera.target,
                zoom = camera.zoom,
                "Camera following selected entity"
            );
            
            // Log spatial operation
            game_logging::log_spatial_operation(position.hex(), "camera_follow", None);
        } else {
            debug!(
                target: "game::systems::camera",
                entity = ?primary_entity,
                "Selected entity has no position component"
            );
        }
    } else {
        debug!(
            target: "game::systems::camera",
            "No entity selected for camera to follow"
        );
    }
    
    let duration_ms = start_time.elapsed().as_secs_f64() * 1000.0;
    game_logging::log_performance_event("camera_system", duration_ms, 1);
}

/// Hex coordinate to pixel coordinate conversion using Zig SIMD optimizations
/// Uses flat-topped hexagon layout with size = 1.0
fn hex_to_pixel(hex: glam::IVec2) -> glam::Vec2 {
    use crate::core::zig_ffi::{hex_to_pixel as zig_hex_to_pixel, HexCoord};
    
    const SIZE: f32 = 1.0;
    let coord = HexCoord::new(hex.x, hex.y);
    let pixel = zig_hex_to_pixel(coord, SIZE);
    glam::Vec2::new(pixel.x, pixel.y)
}

/// System set definitions for organizing system execution order
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum GameSystemSet {
    /// Early systems that run first (input, time)
    Early,
    /// Core gameplay systems (movement, combat, etc.)
    Gameplay,
    /// Late systems that run after gameplay (cleanup, rendering prep)
    Late,
}

/// Configure the system schedule with proper ordering
pub fn configure_systems(app: &mut bevy_ecs::schedule::Schedule) {
    use GameSystemSet::*;

    app.configure_sets((Early, Gameplay, Late).chain())
        .add_systems(
            (
                time_system,
                interpolation_system,
            ).in_set(Early)
        )
        .add_systems(
            (
                turn_advancement_system,
                camera_system,
            ).in_set(Gameplay)
        )
        .add_systems(
            (
                selection_validation_system,
            ).in_set(Late)
        );
        
    // Note: Movement restoration, position sync, health cleanup, and debug logging
    // are now handled by unified_change_system in changes.rs
}

/// Configure systems for parallel execution with the new scheduler
/// Note: Change detection systems are configured via changes::configure_change_detection()
pub fn configure_parallel_systems(scheduler: &mut EcsScheduler, world: &mut World) {
    use crate::ecs::{ResourceAccess, resources::*};
    
    // PreUpdate stage - early systems that prepare for main gameplay
    scheduler.add_system_with_accesses(
        Stage::PreUpdate, 
        "time_system", 
        time_system, 
        vec![
            ResourceAccess::write::<GameTime>(),
            ResourceAccess::read::<SimulationState>(),
        ],
        world
    );
    
    scheduler.add_system_with_accesses(
        Stage::PreUpdate,
        "interpolation_system",
        interpolation_system,
        vec![ResourceAccess::write::<GameTime>()],
        world
    );
    
    scheduler.add_system_with_accesses(
        Stage::PreUpdate, 
        "sync_hierarchy_system", 
        sync_hierarchy_system,
        vec![
            ResourceAccess::read::<crate::ecs::hierarchy::HierarchyQueries>(),
            // Note: Queries are handled by Bevy's system, not our resource conflict detection
        ],
        world
    );
    
    // Update stage - main gameplay systems
    // unified_change_system is added via changes::configure_change_detection()
    scheduler.add_system_with_accesses(
        Stage::Update, 
        "turn_advancement_system", 
        turn_advancement_system,
        vec![
            ResourceAccess::write::<GameTime>(),
            ResourceAccess::write::<Players>(),
        ],
        world
    );
    
    scheduler.add_system_with_accesses(
        Stage::Update, 
        "camera_system", 
        camera_system,
        vec![
            ResourceAccess::write::<Camera>(),
            ResourceAccess::read::<Selection>(),
            // Position queries handled by Bevy
        ],
        world
    );
    
    // PostUpdate stage - systems that run after main gameplay
    scheduler.add_system_with_accesses(
        Stage::PostUpdate, 
        "selection_validation_system", 
        selection_validation_system,
        vec![ResourceAccess::write::<Selection>()],
        world
    );
    
    // Cleanup stage - maintenance systems that run after all gameplay
    scheduler.add_system_with_accesses(
        Stage::Cleanup, 
        "cleanup_hierarchy_system", 
        cleanup_hierarchy_system,
        vec![
            ResourceAccess::read::<crate::ecs::hierarchy::HierarchyQueries>(),
            // Commands handled by Bevy
        ],
        world
    );
}
