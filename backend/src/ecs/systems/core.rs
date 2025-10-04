//! Core game systems that operate on components and resources
//!
//! Systems contain the game logic and operate on entities with specific
//! component combinations. They are designed to be small, focused, and testable.

use bevy_ecs::prelude::*;
use tracing::{info, debug, warn, instrument};

use crate::core::{Stage, SimulationState, logging::game_logging};
use crate::ecs::{components::*, resources::*, EcsScheduler, hierarchy::{sync_hierarchy_system, cleanup_hierarchy_system}};

/// Time management system with time controller integration
#[instrument(name = "time_system", skip_all)]
pub fn time_system(
    mut game_time: ResMut<GameTime>,
    simulation_state: Option<Res<SimulationState>>,
    mut last_time: Local<Option<std::time::Instant>>,
) {
    let start_time = std::time::Instant::now();
    
    // Calculate real delta time using safe Local parameter
    let delta_time = if let Some(last) = *last_time {
        let delta = start_time.duration_since(last).as_secs_f32();
        *last_time = Some(start_time);
        // Cap delta time to prevent large jumps (e.g., when debugging or alt-tabbing)
        delta.min(1.0 / 30.0) // Maximum 30 FPS minimum for stability
    } else {
        *last_time = Some(start_time);
        1.0 / 60.0 // Default delta for first frame
    };
    
    // Use default simulation if not available (for backward compatibility)
    let default_sim = SimulationState::new(42, None);
    let sim = simulation_state.as_deref().unwrap_or(&default_sim);
    
    let old_tick = game_time.tick;
    let old_turn = game_time.turn;
    let old_mode = game_time.playback_mode();
    
    // Update with real delta time
    game_time.update(delta_time, sim);
    
    let new_mode = game_time.playback_mode();
    
    // Log tick changes
    if game_time.tick != old_tick {
        debug!(
            target: "game::systems::time",
            tick = game_time.tick,
            turn = game_time.turn,
            speed = game_time.speed(),
            real_delta_time = delta_time,
            game_delta_time = game_time.delta_time,
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
    interpolated_positions: Query<&mut InterpolatedPosition>,
    interpolated_health: Query<&mut InterpolatedHealth>,
    interpolated_renderables: Query<&mut InterpolatedRenderable>,
) {
    let start_time = std::time::Instant::now();
    
    // Update interpolation factor based on time since last tick
    let current_time = instant::Instant::now();
    use std::sync::OnceLock;
    static LAST_TICK_TIME: OnceLock<std::sync::Mutex<instant::Instant>> = OnceLock::new();
    
    let last_tick_mutex = LAST_TICK_TIME.get_or_init(|| std::sync::Mutex::new(current_time));
    
    if let Ok(mut last_tick_guard) = last_tick_mutex.lock() {
        let last_tick = *last_tick_guard;
        let time_since_tick = current_time.duration_since(last_tick).as_secs_f32();
        let tick_duration = 1.0 / 60.0; // Target 60 TPS
        game_time.update_interpolation(time_since_tick, tick_duration);
        
        // Reset tick time when we advance a tick
        if game_time.tick > 0 && time_since_tick >= tick_duration {
            *last_tick_guard = current_time;
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
    mut turn_manager: ResMut<TurnManager>,
) {
    let start_time = std::time::Instant::now();
    
    // Skip if game is paused
    if game_time.paused {
        debug!(
            target: "game::systems::turn",
            tick = game_time.tick,
            "Game paused, skipping turn advancement"
        );
        return;
    }

    // Update turn manager with current tick
    turn_manager.update_tick(game_time.tick);
    
    // Check if turn should advance based on various conditions
    if turn_manager.should_advance_turn(&game_time, &players) {
        let old_turn = game_time.turn;
        let old_player = players.current_player;
        
        // Process end of current player's turn
        turn_manager.process_turn_end(old_player, game_time.tick);
        
        // Advance to next player
        let next_player = players.next_player();
        players.current_player = next_player;
        
        // If we've cycled through all players, advance the global turn
        let new_turn = if turn_manager.completed_full_turn_cycle(&players) {
            game_time.advance_turn();
            turn_manager.start_new_turn_cycle(game_time.turn);
            game_time.turn
        } else {
            old_turn
        };
        
        // Process start of new player's turn
        turn_manager.process_turn_start(next_player, game_time.tick);
        
        info!(
            target: "game::systems::turn",
            old_turn = old_turn,
            new_turn = new_turn,
            old_player = old_player,
            new_player = next_player,
            turn_cycle_complete = new_turn != old_turn,
            total_ticks = game_time.tick,
            turn_duration_ticks = turn_manager.last_turn_duration(),
            "Turn advanced"
        );
        
        // Log turn progression
        if new_turn != old_turn {
            // Full turn cycle completed
            game_logging::log_entity_operation(
                bevy_ecs::entity::Entity::from_raw(1), // System entity
                "turn_cycle_complete",
                Some(&format!("Global turn {} completed, starting turn {}", old_turn, new_turn))
            );
        }
        
        // Log player turn start
        let current_player_data = players.get_player(next_player);
        let player_type = if current_player_data.map(|p| p.is_human).unwrap_or(false) {
            "human"
        } else {
            "ai"
        };
        
        game_logging::log_entity_operation(
            bevy_ecs::entity::Entity::from_raw(next_player),
            "player_turn_start",
            Some(&format!("{} player {} turn started (global turn {})", 
                         player_type, next_player, new_turn))
        );
        
        // Performance logging
        let duration_ms = start_time.elapsed().as_secs_f64() * 1000.0;
        game_logging::log_performance_event(
            "turn_advancement_system", 
            duration_ms, 
            players.turn_order.len()
        );
    } else {
        // Log why turn didn't advance (for debugging)
        if game_time.tick % 3600 == 0 { // Log every minute worth of ticks
            debug!(
                target: "game::systems::turn",
                tick = game_time.tick,
                current_player = players.current_player,
                min_turn_duration = turn_manager.min_turn_duration_ticks(),
                time_since_turn_start = turn_manager.ticks_since_turn_start(),
                player_ready = turn_manager.is_player_ready(players.current_player),
                "Turn advancement conditions not yet met"
            );
        }
    }
}

// Debug logging is now handled by unified_change_system in changes.rs

/// System for managing camera controls with input handling and smooth movement
#[instrument(name = "camera_system", skip_all)]
pub fn camera_system(
    mut camera: ResMut<Camera>,
    mut camera_input: Local<CameraInputState>,
    selection: Res<Selection>,
    position_query: Query<&Position>,
    game_time: Res<GameTime>,
) {
    let start_time = std::time::Instant::now();
    let delta_time = game_time.delta_time;
    
    // Update camera input state (in real implementation, this would come from input events)
    camera_input.update();
    
    // Calculate camera movement based on input
    let mut target_velocity = glam::Vec2::ZERO;
    
    // Base pan speed scales with zoom level (zoom out = faster pan)
    let base_pan_speed = 1000.0; // pixels per second at zoom 1.0
    let zoom_adjusted_speed = base_pan_speed / camera.zoom.max(0.1);
    
    // Handle directional movement
    if camera_input.move_left {
        target_velocity.x -= zoom_adjusted_speed;
    }
    if camera_input.move_right {
        target_velocity.x += zoom_adjusted_speed;
    }
    if camera_input.move_up {
        target_velocity.y -= zoom_adjusted_speed;
    }
    if camera_input.move_down {
        target_velocity.y += zoom_adjusted_speed;
    }
    
    // Handle edge scrolling (simulate mouse near screen edge)
    if camera_input.edge_scroll_enabled {
        let edge_margin = 50.0; // pixels from edge
        let edge_speed = zoom_adjusted_speed * 0.5; // slower than direct input
        
        if camera_input.mouse_pos.x < edge_margin {
            target_velocity.x -= edge_speed;
        } else if camera_input.mouse_pos.x > camera.viewport_size.x - edge_margin {
            target_velocity.x += edge_speed;
        }
        
        if camera_input.mouse_pos.y < edge_margin {
            target_velocity.y -= edge_speed;
        } else if camera_input.mouse_pos.y > camera.viewport_size.y - edge_margin {
            target_velocity.y += edge_speed;
        }
    }
    
    // Handle zoom input
    if camera_input.zoom_in > 0.0 {
        let zoom_speed = 2.0; // zoom factor per second
        let new_zoom = camera.zoom * (1.0 + zoom_speed * delta_time * camera_input.zoom_in);
        camera.set_zoom(new_zoom);
    }
    if camera_input.zoom_out > 0.0 {
        let zoom_speed = 2.0;
        let new_zoom = camera.zoom * (1.0 - zoom_speed * delta_time * camera_input.zoom_out);
        camera.set_zoom(new_zoom);
    }
    
    // Apply smooth movement with velocity damping
    let smoothing_factor = 10.0; // how quickly to reach target velocity
    camera_input.current_velocity = glam::Vec2::lerp(
        camera_input.current_velocity,
        target_velocity,
        smoothing_factor * delta_time
    );
    
    // Update camera position
    if camera_input.current_velocity.length() > 1.0 { // Only move if velocity is significant
        let old_target = camera.target;
        let new_target = old_target + camera_input.current_velocity * delta_time;
        
        // Apply camera bounds constraints
        let constrained_target = constrain_camera_target(new_target, &camera);
        camera.set_target(constrained_target);
        
        debug!(
            target: "game::systems::camera",
            velocity = ?camera_input.current_velocity,
            old_target = ?old_target,
            new_target = ?camera.target,
            zoom = camera.zoom,
            "Camera moved by input"
        );
    }
    
    // Follow selected entity (with lower priority than manual input)
    if camera_input.follow_selection && target_velocity.length() < 10.0 {
        if let Some(primary_entity) = selection.primary {
            if let Ok(position) = position_query.get(primary_entity) {
                let target_pos = position.pixel();
                
                // Smoothly move towards selected entity
                let follow_speed = 500.0; // pixels per second
                let direction = (target_pos - camera.target).normalize_or_zero();
                let distance = target_pos.distance(camera.target);
                
                if distance > 50.0 { // Only follow if entity is far from current view
                    let follow_velocity = direction * (follow_speed as f32).min(distance * 2.0);
                    let new_target = camera.target + follow_velocity * delta_time;
                    camera.set_target(new_target);
                    
                    debug!(
                        target: "game::systems::camera",
                        entity = ?primary_entity,
                        entity_pos = ?target_pos,
                        camera_target = ?camera.target,
                        distance = distance,
                        "Camera smoothly following selected entity"
                    );
                    
                    // Log spatial operation
                    game_logging::log_spatial_operation(position.hex(), "camera_follow", None);
                }
            }
        }
    }
    
    // Handle instant focus requests
    if camera_input.instant_focus_requested {
        if let Some(primary_entity) = selection.primary {
            if let Ok(position) = position_query.get(primary_entity) {
                let old_target = camera.target;
                camera.set_target(position.pixel());
                camera_input.current_velocity = glam::Vec2::ZERO; // Stop movement
                
                info!(
                    target: "game::systems::camera",
                    entity = ?primary_entity,
                    old_target = ?old_target,
                    new_target = ?camera.target,
                    "Camera instantly focused on selected entity"
                );
                
                game_logging::log_spatial_operation(position.hex(), "camera_instant_focus", None);
            }
        }
        camera_input.instant_focus_requested = false;
    }
    
    // Log camera state periodically for debugging
    if game_time.tick % 300 == 0 { // Every 5 seconds at 60 TPS
        debug!(
            target: "game::systems::camera",
            target = ?camera.target,
            zoom = camera.zoom,
            viewport_size = ?camera.viewport_size,
            velocity = ?camera_input.current_velocity,
            follow_selection = camera_input.follow_selection,
            "Camera state update"
        );
    }
    
    let duration_ms = start_time.elapsed().as_secs_f64() * 1000.0;
    game_logging::log_performance_event("camera_system", duration_ms, 1);
}

/// Camera input state for smooth movement and control
#[derive(Debug, Default)]
pub struct CameraInputState {
    // Movement inputs
    pub move_left: bool,
    pub move_right: bool,
    pub move_up: bool,
    pub move_down: bool,
    
    // Zoom inputs (0.0 to 1.0 strength)
    pub zoom_in: f32,
    pub zoom_out: f32,
    
    // Mouse state for edge scrolling
    pub mouse_pos: glam::Vec2,
    pub edge_scroll_enabled: bool,
    
    // Current velocity for smooth movement
    pub current_velocity: glam::Vec2,
    
    // Following behavior
    pub follow_selection: bool,
    pub instant_focus_requested: bool,
    
    // Performance tracking
    last_update_time: Option<std::time::Instant>,
}

impl CameraInputState {
    /// Update input state (placeholder for input system integration)
    pub fn update(&mut self) {
        let now = std::time::Instant::now();
        
        // In a real implementation, this would be updated by input events
        // For now, simulate some basic input patterns for testing
        
        // Reset single-frame inputs
        self.instant_focus_requested = false;
        
        // Update last update time
        self.last_update_time = Some(now);
    }
    
    /// Set movement input from keyboard
    pub fn set_movement(&mut self, left: bool, right: bool, up: bool, down: bool) {
        self.move_left = left;
        self.move_right = right;
        self.move_up = up;
        self.move_down = down;
    }
    
    /// Set zoom input
    pub fn set_zoom(&mut self, zoom_in: f32, zoom_out: f32) {
        self.zoom_in = zoom_in.clamp(0.0, 1.0);
        self.zoom_out = zoom_out.clamp(0.0, 1.0);
    }
    
    /// Set mouse position for edge scrolling
    pub fn set_mouse_position(&mut self, pos: glam::Vec2) {
        self.mouse_pos = pos;
    }
    
    /// Enable/disable edge scrolling
    pub fn set_edge_scroll(&mut self, enabled: bool) {
        self.edge_scroll_enabled = enabled;
    }
    
    /// Enable/disable following selection
    pub fn set_follow_selection(&mut self, follow: bool) {
        self.follow_selection = follow;
    }
    
    /// Request instant focus on selected entity
    pub fn request_instant_focus(&mut self) {
        self.instant_focus_requested = true;
    }
    
    /// Stop all camera movement
    pub fn stop_movement(&mut self) {
        self.move_left = false;
        self.move_right = false;
        self.move_up = false;
        self.move_down = false;
        self.current_velocity = glam::Vec2::ZERO;
        self.zoom_in = 0.0;
        self.zoom_out = 0.0;
    }
}

/// Constrain camera target to reasonable world bounds
fn constrain_camera_target(target: glam::Vec2, camera: &Camera) -> glam::Vec2 {
    // Define world bounds (in a real implementation, this would come from world size)
    let world_size = glam::Vec2::new(10000.0, 10000.0); // 10km x 10km world
    let world_bounds = rstar::AABB::from_corners([-world_size.x/2.0, -world_size.y/2.0], [world_size.x/2.0, world_size.y/2.0]);
    
    // Calculate visible area based on zoom and viewport
    let visible_width = camera.viewport_size.x / camera.zoom;
    let visible_height = camera.viewport_size.y / camera.zoom;
    let half_visible = glam::Vec2::new(visible_width, visible_height) * 0.5;
    
    // Constrain target so that camera doesn't go outside world bounds
    let min_bounds = glam::Vec2::new(world_bounds.lower()[0], world_bounds.lower()[1]);
    let max_bounds = glam::Vec2::new(world_bounds.upper()[0], world_bounds.upper()[1]);
    let min_target = min_bounds + half_visible;
    let max_target = max_bounds - half_visible;
    
    glam::Vec2::new(
        target.x.clamp(min_target.x, max_target.x),
        target.y.clamp(min_target.y, max_target.y)
    )
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
    
    // Add tile properties systems
    scheduler.add_system_with_accesses(
        Stage::Update,
        "update_tile_properties",
        crate::world::tiles::update_tile_properties,
        vec![
            ResourceAccess::read::<crate::world::tiles::TilePropertiesSystem>(),
        ],
        world
    );
    
    scheduler.add_system_with_accesses(
        Stage::PostUpdate,
        "update_cultural_influence",
        crate::world::tiles::update_cultural_influence,
        vec![
            ResourceAccess::read::<crate::world::tiles::TilePropertiesSystem>(),
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
