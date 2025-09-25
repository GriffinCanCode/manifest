//! Global resources for the ECS world
//!
//! Resources are singleton-like data that is globally accessible
//! to all systems. They represent shared game state and configuration.

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::core::{
    hashing::{collections, FastHashMap},
    control::{TimeController, PlaybackMode},
    interpolate::{InterpolationFactor, lerp_factor}
};

/// Core timing and turn management with time control integration
#[derive(Resource, Debug)]
pub struct GameTime {
    /// Current turn number (1-based)
    pub turn: u32,
    /// Current tick within the turn (for game logic)
    pub tick: u64,
    /// Time since last tick in seconds
    pub delta_time: f32,
    /// Interpolation factor for smooth rendering (0.0 = previous tick, 1.0 = current tick)
    pub interpolation_factor: InterpolationFactor,
    /// Time controller for advanced playback control
    pub controller: TimeController,
}

impl Default for GameTime {
    fn default() -> Self {
        Self {
            turn: 1,
            tick: 0,
            delta_time: 0.0,
            interpolation_factor: lerp_factor(0.0),
            controller: TimeController::new(),
        }
    }
}

impl GameTime {
    /// Create new game time with custom controller
    pub fn with_controller(controller: TimeController) -> Self {
        Self {
            turn: 1,
            tick: 0,
            delta_time: 0.0,
            interpolation_factor: lerp_factor(0.0),
            controller,
        }
    }

    /// Advance to next turn
    pub fn advance_turn(&mut self) {
        self.turn += 1;
        self.tick = 0;
    }

    /// Update with time controller integration
    pub fn update(&mut self, real_delta_time: f32, simulation: &crate::core::SimulationState) {
        // Update time controller and get effective delta
        self.delta_time = self.controller.update().into_inner();
        
        // Only advance if controller allows it
        if self.controller.should_advance(simulation) {
            self.tick += 1;
        }
    }

    /// Update interpolation factor for smooth rendering
    pub fn update_interpolation(&mut self, time_since_last_tick: f32, tick_duration: f32) {
        if tick_duration > 0.0 {
            let factor = (time_since_last_tick / tick_duration).clamp(0.0, 1.0);
            self.interpolation_factor = lerp_factor(factor);
        }
    }

    /// Get current playback mode
    pub fn playback_mode(&self) -> PlaybackMode {
        self.controller.mode()
    }

    /// Get playback speed multiplier
    pub fn speed(&self) -> f32 {
        self.controller.speed()
    }

    /// Check if game is paused
    pub fn is_paused(&self) -> bool {
        matches!(self.controller.mode(), PlaybackMode::Paused)
    }

    /// Play the game
    pub fn play(&self) -> Result<(), crate::core::control::ControlError> {
        self.controller.play()
    }

    /// Pause the game
    pub fn pause(&self) -> Result<(), crate::core::control::ControlError> {
        self.controller.pause()
    }

    /// Toggle play/pause
    pub fn toggle(&self) -> Result<PlaybackMode, crate::core::control::ControlError> {
        self.controller.toggle()
    }

    /// Step one tick and pause
    pub fn step(&self) -> Result<(), crate::core::control::ControlError> {
        self.controller.step()
    }

    /// Set playback speed
    pub fn set_speed(&self, speed: f32) -> Result<(), crate::core::control::ControlError> {
        self.controller.set_speed(speed)
    }

    /// Get interpolation factor for rendering
    pub fn interpolation_factor(&self) -> InterpolationFactor {
        self.interpolation_factor
    }
}

/// Player and civilization management
#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct Players {
    /// Map of player ID to player data
    /// Player data indexed by player ID (optimized for u32 keys)
    pub data: FastHashMap<u32, PlayerData>,
    /// Current human player ID
    pub current_player: u32,
    /// Turn order for multiplayer
    pub turn_order: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerData {
    pub name: String,
    pub civilization: String,
    pub is_human: bool,
    pub is_active: bool,
    pub color: [f32; 3], // RGB color
}

impl Default for Players {
    fn default() -> Self {
        let mut players = Self {
            data: collections::fast_hash_map(),
            current_player: 1,
            turn_order: vec![1],
        };

        // Add default player
        players.data.insert(
            1,
            PlayerData {
                name: "Player".to_string(),
                civilization: "Ancient Empire".to_string(),
                is_human: true,
                is_active: true,
                color: [0.2, 0.5, 0.8], // Blue
            },
        );

        players
    }
}

impl Players {
    /// Add a new player
    pub fn add_player(&mut self, name: String, civilization: String, is_human: bool) -> u32 {
        let player_id = self.data.len() as u32 + 1;
        
        self.data.insert(
            player_id,
            PlayerData {
                name,
                civilization,
                is_human,
                is_active: true,
                color: generate_player_color(player_id),
            },
        );

        self.turn_order.push(player_id);
        player_id
    }

    /// Get player data by ID
    pub fn get_player(&self, player_id: u32) -> Option<&PlayerData> {
        self.data.get(&player_id)
    }

    /// Get next player in turn order
    pub fn next_player(&self) -> u32 {
        let current_index = self.turn_order
            .iter()
            .position(|&id| id == self.current_player)
            .unwrap_or(0);
        
        let next_index = (current_index + 1) % self.turn_order.len();
        self.turn_order[next_index]
    }
}

/// Camera and viewport configuration
#[derive(Resource, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Camera {
    /// World position the camera is looking at
    pub target: glam::Vec2,
    /// Camera zoom level (1.0 = normal, 2.0 = zoomed in)
    pub zoom: f32,
    /// Viewport size in pixels
    pub viewport_size: glam::Vec2,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            target: glam::Vec2::ZERO,
            zoom: 1.0,
            viewport_size: glam::Vec2::new(1920.0, 1080.0),
        }
    }
}

impl Camera {
    /// Move camera to position
    pub fn set_target(&mut self, target: glam::Vec2) {
        self.target = target;
    }

    /// Adjust zoom level with clamping
    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom.clamp(0.1, 10.0);
    }

    /// Update viewport size
    pub fn set_viewport(&mut self, width: f32, height: f32) {
        self.viewport_size = glam::Vec2::new(width, height);
    }
}

/// Selected entities for UI interaction
#[derive(Resource, Debug, Clone, Default, Serialize, Deserialize)]
pub struct Selection {
    /// Currently selected entities
    pub entities: Vec<Entity>,
    /// Primary selected entity (first in selection)
    pub primary: Option<Entity>,
}

impl Selection {
    /// Clear all selections
    pub fn clear(&mut self) {
        self.entities.clear();
        self.primary = None;
    }

    /// Select a single entity
    pub fn select_single(&mut self, entity: Entity) {
        self.entities.clear();
        self.entities.push(entity);
        self.primary = Some(entity);
    }

    /// Add entity to selection
    pub fn add(&mut self, entity: Entity) {
        if !self.entities.contains(&entity) {
            self.entities.push(entity);
            if self.primary.is_none() {
                self.primary = Some(entity);
            }
        }
    }

    /// Remove entity from selection
    pub fn remove(&mut self, entity: Entity) {
        self.entities.retain(|&e| e != entity);
        if self.primary == Some(entity) {
            self.primary = self.entities.first().copied();
        }
    }

    /// Check if entity is selected
    pub fn contains(&self, entity: Entity) -> bool {
        self.entities.contains(&entity)
    }
}

/// Generate a unique color for each player
fn generate_player_color(player_id: u32) -> [f32; 3] {
    // Generate colors using HSV to ensure good visual separation
    let hue = (player_id as f32 * 137.508) % 360.0; // Golden angle for good distribution
    let saturation = 0.7;
    let value = 0.8;

    // Convert HSV to RGB
    let c = value * saturation;
    let x = c * (1.0 - ((hue / 60.0) % 2.0 - 1.0).abs());
    let m = value - c;

    let (r, g, b) = match (hue / 60.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    [r + m, g + m, b + m]
}
