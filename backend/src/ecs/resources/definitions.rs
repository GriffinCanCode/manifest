//! Global resources for the ECS world
//!
//! Resources are singleton-like data that is globally accessible
//! to all systems. They represent shared game state and configuration.

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use crate::core::{
    hashing::{collections, FastHashMap},
    control::{TimeController, PlaybackMode},
    interpolate::{InterpolationFactor, lerp_factor}
};

/// Core timing and turn management with time control integration
#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
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
    #[serde(skip)]
    pub controller: TimeController,
    /// Whether the game is paused (synchronized with controller state)
    pub paused: bool,
}

impl Default for GameTime {
    fn default() -> Self {
        Self {
            turn: 1,
            tick: 0,
            delta_time: 0.0,
            interpolation_factor: lerp_factor(0.0),
            controller: TimeController::new(),
            paused: false,
        }
    }
}

impl GameTime {
    /// Create new game time with custom controller
    pub fn with_controller(controller: TimeController) -> Self {
        let paused = matches!(controller.mode(), PlaybackMode::Paused);
        Self {
            turn: 1,
            tick: 0,
            delta_time: 0.0,
            interpolation_factor: lerp_factor(0.0),
            controller,
            paused,
        }
    }

    /// Advance to next turn
    pub fn advance_turn(&mut self) {
        self.turn += 1;
        self.tick = 0;
    }

    /// Update with time controller integration
    pub fn update(&mut self, _real_delta_time: f32, simulation: &crate::core::SimulationState) {
        // Update time controller and get effective delta
        self.delta_time = self.controller.update().into_inner();
        
        // Sync paused field with controller state
        self.paused = matches!(self.controller.mode(), PlaybackMode::Paused);
        
        // Only advance if controller allows it
        if self.controller.should_advance(simulation) {
            self.tick += 1;
        }
    }

    /// Set paused state directly (synchronizes with controller)
    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
        if paused {
            let _ = self.controller.pause();
        } else {
            let _ = self.controller.play();
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
        self.paused
    }

    /// Play the game
    pub fn play(&mut self) -> Result<(), crate::core::control::ControlError> {
        let result = self.controller.play();
        if result.is_ok() {
            self.paused = matches!(self.controller.mode(), PlaybackMode::Paused);
        }
        result
    }

    /// Pause the game
    pub fn pause(&mut self) -> Result<(), crate::core::control::ControlError> {
        let result = self.controller.pause();
        if result.is_ok() {
            self.paused = matches!(self.controller.mode(), PlaybackMode::Paused);
        }
        result
    }

    /// Toggle play/pause
    pub fn toggle(&mut self) -> Result<PlaybackMode, crate::core::control::ControlError> {
        let result = self.controller.toggle();
        if let Ok(_) = result {
            self.paused = matches!(self.controller.mode(), PlaybackMode::Paused);
        }
        result
    }

    /// Step one tick and pause
    pub fn step(&mut self) -> Result<(), crate::core::control::ControlError> {
        let result = self.controller.step();
        if result.is_ok() {
            self.paused = matches!(self.controller.mode(), PlaybackMode::Paused);
        }
        result
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

/// Turn management for turn-based strategy gameplay
#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct TurnManager {
    /// Minimum ticks a turn must last (prevents rapid cycling)
    pub min_turn_duration_ticks: u64,
    /// Maximum ticks a turn can last (forces advancement for slow players/AI)
    pub max_turn_duration_ticks: u64,
    /// Tick when current player's turn started
    pub turn_start_tick: u64,
    /// Current tick (updated each frame)
    pub current_tick: u64,
    /// Turn durations for performance analysis
    pub turn_durations: Vec<u64>,
    /// Player readiness states (for multiplayer synchronization)
    pub player_ready_states: FastHashMap<u32, PlayerTurnState>,
    /// Turn advancement mode
    pub advancement_mode: TurnAdvancementMode,
    /// Players who have completed their turns this cycle
    pub completed_players: FastHashMap<u32, bool>,
    /// Current turn cycle start tick
    pub turn_cycle_start_tick: u64,
    /// AI thinking time multipliers per player
    pub ai_thinking_time: FastHashMap<u32, f32>,
}

/// Player turn state for advanced turn management
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PlayerTurnState {
    /// Player is actively taking their turn
    Playing,
    /// Player has ended their turn and is waiting
    Ready,
    /// Player is thinking (AI or human deliberating)
    Thinking { started_at_tick: u64 },
    /// Player is inactive/disconnected
    Inactive,
}

/// Turn advancement modes for different game types
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TurnAdvancementMode {
    /// Classic turn-based - each player takes a full turn
    Sequential,
    /// Simultaneous - all players plan, then all resolve
    Simultaneous,
    /// Hybrid - planning phase, then sequential resolution
    Hybrid {
        current_phase: HybridTurnPhase,
        planning_duration_ticks: u64,
        resolution_duration_ticks: u64,
    },
    /// Real-time with turn phases
    RealTimePhased { phase_duration_ticks: u64 },
}

/// Phases for hybrid turn management
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HybridTurnPhase {
    /// All players plan their moves simultaneously
    Planning,
    /// Moves are resolved in turn order
    Resolution,
    /// Brief transition between cycles
    Transition,
}

impl Default for TurnManager {
    fn default() -> Self {
        Self {
            min_turn_duration_ticks: 1800, // 30 seconds at 60 FPS
            max_turn_duration_ticks: 18000, // 5 minutes at 60 FPS
            turn_start_tick: 0,
            current_tick: 0,
            turn_durations: Vec::with_capacity(1000),
            player_ready_states: collections::fast_hash_map(),
            advancement_mode: TurnAdvancementMode::Sequential,
            completed_players: collections::fast_hash_map(),
            turn_cycle_start_tick: 0,
            ai_thinking_time: collections::fast_hash_map(),
        }
    }
}

impl TurnManager {
    /// Check if turn should advance based on current conditions
    pub fn should_advance_turn(&self, game_time: &GameTime, players: &Players) -> bool {
        let ticks_since_turn_start = self.current_tick.saturating_sub(self.turn_start_tick);
        
        match self.advancement_mode {
            TurnAdvancementMode::Sequential => {
                // Must meet minimum duration
                if ticks_since_turn_start < self.min_turn_duration_ticks {
                    return false;
                }
                
                let current_player_data = players.get_player(players.current_player);
                
                // Force advancement if maximum duration reached
                if ticks_since_turn_start >= self.max_turn_duration_ticks {
                    return true;
                }
                
                // Check if current player is ready to advance
                if let Some(ready_state) = self.player_ready_states.get(&players.current_player) {
                    match ready_state {
                        PlayerTurnState::Ready => true,
                        PlayerTurnState::Thinking { started_at_tick } => {
                            let thinking_duration = self.current_tick.saturating_sub(*started_at_tick);
                            let max_thinking_time = if current_player_data.map(|p| p.is_human).unwrap_or(false) {
                                self.max_turn_duration_ticks // Humans get full time
                            } else {
                                // AI gets modified time based on difficulty/thinking multiplier
                                let multiplier = self.ai_thinking_time.get(&players.current_player).copied().unwrap_or(1.0);
                                (self.min_turn_duration_ticks as f32 * multiplier * 2.0) as u64
                            };
                            thinking_duration >= max_thinking_time
                        },
                        PlayerTurnState::Inactive => true, // Skip inactive players
                        PlayerTurnState::Playing => false, // Still playing
                    }
                } else {
                    // No explicit state - use time-based advancement
                    ticks_since_turn_start >= self.min_turn_duration_ticks
                }
            },
            
            TurnAdvancementMode::Simultaneous => {
                // All players must be ready, or max time reached
                ticks_since_turn_start >= self.max_turn_duration_ticks ||
                players.turn_order.iter().all(|&player_id| {
                    self.player_ready_states.get(&player_id)
                        .map(|state| matches!(state, PlayerTurnState::Ready | PlayerTurnState::Inactive))
                        .unwrap_or(false)
                })
            },
            
            TurnAdvancementMode::RealTimePhased { phase_duration_ticks } => {
                ticks_since_turn_start >= phase_duration_ticks
            },
            
            TurnAdvancementMode::Hybrid { 
                current_phase, 
                planning_duration_ticks, 
                resolution_duration_ticks 
            } => {
                match current_phase {
                    HybridTurnPhase::Planning => {
                        // Planning phase: advance when all players are ready OR max planning time reached
                        ticks_since_turn_start >= *planning_duration_ticks ||
                        players.turn_order.iter().all(|&player_id| {
                            self.player_ready_states.get(&player_id)
                                .map(|state| matches!(state, PlayerTurnState::Ready | PlayerTurnState::Inactive))
                                .unwrap_or(false)
                        })
                    },
                    HybridTurnPhase::Resolution => {
                        // Resolution phase: advance after resolution duration OR when all moves processed
                        ticks_since_turn_start >= *resolution_duration_ticks ||
                        self.all_moves_resolved(players)
                    },
                    HybridTurnPhase::Transition => {
                        // Brief transition: advance quickly (1 second at 60 FPS)
                        ticks_since_turn_start >= 60
                    },
                }
            },
        }
    }
    
    /// Update current tick
    pub fn update_tick(&mut self, tick: u64) {
        self.current_tick = tick;
    }
    
    /// Process end of player turn
    pub fn process_turn_end(&mut self, player_id: u32, tick: u64) {
        let turn_duration = tick.saturating_sub(self.turn_start_tick);
        self.turn_durations.push(turn_duration);
        
        // Mark player as completed for this cycle
        self.completed_players.insert(player_id, true);
        
        // Limit turn duration history to prevent memory growth
        if self.turn_durations.len() > 1000 {
            self.turn_durations.drain(0..100);
        }
    }
    
    /// Process start of player turn
    pub fn process_turn_start(&mut self, player_id: u32, tick: u64) {
        self.turn_start_tick = tick;
        
        // Set initial player state
        self.player_ready_states.insert(player_id, PlayerTurnState::Playing);
    }
    
    /// Check if we've completed a full turn cycle
    pub fn completed_full_turn_cycle(&self, players: &Players) -> bool {
        // All players in turn order have completed their turns
        players.turn_order.iter().all(|&player_id| {
            self.completed_players.get(&player_id).copied().unwrap_or(false)
        })
    }
    
    /// Start new turn cycle
    pub fn start_new_turn_cycle(&mut self, turn: u32) {
        self.completed_players.clear();
        self.turn_cycle_start_tick = self.current_tick;
    }
    
    /// Get last turn duration in ticks
    pub fn last_turn_duration(&self) -> u64 {
        self.turn_durations.last().copied().unwrap_or(0)
    }
    
    /// Get minimum turn duration
    pub fn min_turn_duration_ticks(&self) -> u64 {
        self.min_turn_duration_ticks
    }
    
    /// Get ticks since current turn started
    pub fn ticks_since_turn_start(&self) -> u64 {
        self.current_tick.saturating_sub(self.turn_start_tick)
    }
    
    /// Check if player is ready to advance
    pub fn is_player_ready(&self, player_id: u32) -> bool {
        self.player_ready_states.get(&player_id)
            .map(|state| matches!(state, PlayerTurnState::Ready))
            .unwrap_or(false)
    }
    
    /// Mark player as ready to advance turn
    pub fn mark_player_ready(&mut self, player_id: u32) {
        self.player_ready_states.insert(player_id, PlayerTurnState::Ready);
    }
    
    /// Mark player as thinking
    pub fn mark_player_thinking(&mut self, player_id: u32) {
        self.player_ready_states.insert(player_id, PlayerTurnState::Thinking { 
            started_at_tick: self.current_tick 
        });
    }
    
    /// Set AI thinking time multiplier for a player
    pub fn set_ai_thinking_multiplier(&mut self, player_id: u32, multiplier: f32) {
        self.ai_thinking_time.insert(player_id, multiplier.clamp(0.1, 10.0));
    }
    
    /// Get average turn duration for performance analysis
    pub fn average_turn_duration(&self) -> f64 {
        if self.turn_durations.is_empty() {
            0.0
        } else {
            let sum: u64 = self.turn_durations.iter().sum();
            sum as f64 / self.turn_durations.len() as f64
        }
    }
    
    /// Check if all moves have been resolved in hybrid resolution phase
    pub fn all_moves_resolved(&self, players: &Players) -> bool {
        // In a real implementation, this would check if all planned moves have been processed
        // For now, we check if all players have completed their resolution actions
        players.turn_order.iter().all(|&player_id| {
            self.completed_players.get(&player_id).copied().unwrap_or(false)
        })
    }
    
    /// Advance to next phase in hybrid mode
    pub fn advance_hybrid_phase(&mut self) -> Option<HybridTurnPhase> {
        if let TurnAdvancementMode::Hybrid { 
            current_phase, 
            planning_duration_ticks, 
            resolution_duration_ticks 
        } = &mut self.advancement_mode {
            let next_phase = match current_phase {
                HybridTurnPhase::Planning => HybridTurnPhase::Resolution,
                HybridTurnPhase::Resolution => HybridTurnPhase::Transition,
                HybridTurnPhase::Transition => HybridTurnPhase::Planning,
            };
            *current_phase = next_phase;
            
            // Reset turn start tick for new phase
            self.turn_start_tick = self.current_tick;
            
            // Clear completed players when entering planning phase
            if next_phase == HybridTurnPhase::Planning {
                self.completed_players.clear();
            }
            
            Some(next_phase)
        } else {
            None
        }
    }
    
    /// Get current hybrid phase if in hybrid mode
    pub fn current_hybrid_phase(&self) -> Option<HybridTurnPhase> {
        match &self.advancement_mode {
            TurnAdvancementMode::Hybrid { current_phase, .. } => Some(*current_phase),
            _ => None,
        }
    }
    
    /// Set hybrid mode with specific phase durations
    pub fn set_hybrid_mode(&mut self, planning_ticks: u64, resolution_ticks: u64) {
        self.advancement_mode = TurnAdvancementMode::Hybrid {
            current_phase: HybridTurnPhase::Planning,
            planning_duration_ticks: planning_ticks,
            resolution_duration_ticks: resolution_ticks,
        };
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
