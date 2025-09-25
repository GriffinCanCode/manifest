//! Core game components
//!
//! Contains fundamental components like Position, Movement, Health, Owner, and Name.

use bevy_ecs::prelude::*;
use glam::{IVec2, Vec2};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tracing::{debug, warn, instrument};
use std::fmt;

use crate::core::{
    zig_ffi::{hex_distance as zig_hex_distance, hex_to_pixel as zig_hex_to_pixel, HexCoord},
    logging::{LoggingSystem, game_logging},
    interpolate::{Interpolate, InterpolationFactor}
};

use super::validation::{Validate, ComponentError, ComponentResult, utils};

/// Convert IVec2 to pixel coordinates using Zig FFI
fn hex_to_pixel(hex: IVec2) -> Vec2 {
    let hex_coord = HexCoord { q: hex.x, r: hex.y };
    let pixel_coord = zig_hex_to_pixel(hex_coord, 1.0);
    Vec2::new(pixel_coord.x, pixel_coord.y)
}

/// Calculate hex distance between two coordinates
pub fn hex_distance(from: IVec2, to: IVec2) -> u32 {
    let from_coord = HexCoord { q: from.x, r: from.y };
    let to_coord = HexCoord { q: to.x, r: to.y };
    zig_hex_distance(from_coord, to_coord)
}

/// Fundamental positioning component for hex-grid based entities
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Position {
    /// Axial hex coordinates (q, r)
    hex: IVec2,
    /// Pixel-space coordinates for rendering (derived, not serialized)
    pixel: Vec2,
}

impl Position {
    /// Create new position from hex coordinates with validation
    #[instrument(name = "position_new", fields(q = q, r = r))]
    pub fn new(q: i32, r: i32) -> ComponentResult<Self> {
        let correlation_id = LoggingSystem::generate_correlation_id();
        
        utils::validate_coordinates(q, r)?;
        
        let hex = IVec2::new(q, r);
        let pixel = hex_to_pixel(hex);
        
        debug!(
            target: "game::components::position",
            correlation_id = correlation_id,
            hex_q = q,
            hex_r = r,
            pixel_x = pixel.x,
            pixel_y = pixel.y,
            "Position component created successfully"
        );
        
        game_logging::log_spatial_operation(hex, "position_created", None);
        
        Ok(Self { hex, pixel })
    }

    /// Create unchecked position (for internal use)
    pub(crate) fn new_unchecked(q: i32, r: i32) -> Self {
        let hex = IVec2::new(q, r);
        Self {
            hex,
            pixel: hex_to_pixel(hex),
        }
    }

    /// Create from hex coordinates
    pub fn from_hex(hex: IVec2) -> ComponentResult<Self> {
        Self::new(hex.x, hex.y)
    }

    /// Get hex coordinates
    pub fn hex(&self) -> IVec2 {
        self.hex
    }

    /// Get pixel coordinates
    pub fn pixel(&self) -> Vec2 {
        self.pixel
    }

    /// Get q coordinate
    pub fn q(&self) -> i32 {
        self.hex.x
    }

    /// Get r coordinate  
    pub fn r(&self) -> i32 {
        self.hex.y
    }

    /// Get s coordinate (derived: s = -q - r)
    pub fn s(&self) -> i32 {
        -self.hex.x - self.hex.y
    }

    /// Set new hex coordinates with validation
    pub fn set_hex(&mut self, q: i32, r: i32) -> ComponentResult<()> {
        utils::validate_coordinates(q, r)?;
        self.hex = IVec2::new(q, r);
        self.pixel = hex_to_pixel(self.hex);
        Ok(())
    }

    /// Move by offset with validation
    pub fn move_by(&mut self, dq: i32, dr: i32) -> ComponentResult<()> {
        let new_q = self.hex.x + dq;
        let new_r = self.hex.y + dr;
        self.set_hex(new_q, new_r)
    }

    /// Calculate distance to another position
    pub fn distance_to(&self, other: &Position) -> u32 {
        hex_distance(self.hex, other.hex)
    }
}

impl Validate for Position {
    type Error = ComponentError;

    fn validate(&self) -> Result<(), Self::Error> {
        utils::validate_coordinates(self.hex.x, self.hex.y)
    }

    fn constraints() -> &'static str {
        "Position coordinates must be within ±10000 bounds"
    }
}

impl Serialize for Position {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Only serialize hex coordinates, pixel coordinates are derived
        (self.hex.x, self.hex.y).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Position {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let (q, r) = <(i32, i32)>::deserialize(deserializer)?;
        Position::new(q, r).map_err(serde::de::Error::custom)
    }
}

/// Movement and pathfinding component with turn-based mechanics
#[derive(Component, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Movement {
    /// Movement speed (tiles per turn)
    pub speed: f32,
    /// Remaining movement points this turn
    pub remaining_moves: u32,
    /// Maximum movement points per turn
    pub max_moves: u32,
    /// Whether entity can move diagonally
    pub can_move_diagonal: bool,
    /// Movement type for pathfinding
    pub movement_type: MovementType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MovementType {
    Land,
    Naval,
    Air,
    Amphibious,
}

impl Movement {
    /// Create new movement component with validation
    pub fn new(speed: f32, max_moves: u32, movement_type: MovementType) -> ComponentResult<Self> {
        utils::validate_movement(speed, max_moves, max_moves)?;
        
        Ok(Self {
            speed,
            remaining_moves: max_moves,
            max_moves,
            can_move_diagonal: false,
            movement_type,
        })
    }

    /// Reset movement points for new turn
    pub fn reset_for_turn(&mut self) {
        self.remaining_moves = self.max_moves;
    }

    /// Use movement points
    pub fn use_moves(&mut self, moves: u32) -> ComponentResult<()> {
        if moves > self.remaining_moves {
            return Err(ComponentError::InvalidMovement(
                format!("Not enough movement points: need {}, have {}", moves, self.remaining_moves)
            ));
        }
        self.remaining_moves -= moves;
        Ok(())
    }

    /// Check if can move
    pub fn can_move(&self, moves_needed: u32) -> bool {
        self.remaining_moves >= moves_needed
    }

    /// Get movement efficiency (0.0 to 1.0)
    pub fn efficiency(&self) -> f32 {
        if self.max_moves == 0 {
            0.0
        } else {
            self.remaining_moves as f32 / self.max_moves as f32
        }
    }
}

impl Validate for Movement {
    type Error = ComponentError;

    fn validate(&self) -> Result<(), Self::Error> {
        utils::validate_movement(self.speed, self.remaining_moves, self.max_moves)
    }

    fn constraints() -> &'static str {
        "Speed must be non-negative and ≤1000; remaining_moves ≤ max_moves"
    }
}

/// Health and damage tracking component
#[derive(Component, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Health {
    /// Current health points
    pub current: f32,
    /// Maximum health points
    pub max: f32,
    /// Health regeneration per turn
    pub regen_rate: f32,
    /// Damage reduction (0.0 to 1.0)
    pub armor: f32,
}

impl Health {
    /// Create new health component with validation
    pub fn new(max_health: f32) -> ComponentResult<Self> {
        utils::validate_health(max_health, max_health)?;
        
        Ok(Self {
            current: max_health,
            max: max_health,
            regen_rate: 0.0,
            armor: 0.0,
        })
    }

    /// Create with custom values
    pub fn with_values(current: f32, max: f32, regen_rate: f32, armor: f32) -> ComponentResult<Self> {
        utils::validate_health(current, max)?;
        
        if armor < 0.0 || armor > 1.0 {
            return Err(ComponentError::InvalidHealth(
                format!("Armor must be between 0.0 and 1.0, got {}", armor)
            ));
        }
        
        Ok(Self {
            current,
            max,
            regen_rate,
            armor,
        })
    }

    /// Take damage with armor calculation
    pub fn take_damage(&mut self, damage: f32) -> ComponentResult<f32> {
        if damage < 0.0 {
            return Err(ComponentError::InvalidHealth("Damage cannot be negative".to_string()));
        }
        
        let actual_damage = damage * (1.0 - self.armor);
        self.current = (self.current - actual_damage).max(0.0);
        
        Ok(actual_damage)
    }

    /// Heal by amount
    pub fn heal(&mut self, amount: f32) -> ComponentResult<f32> {
        if amount < 0.0 {
            return Err(ComponentError::InvalidHealth("Heal amount cannot be negative".to_string()));
        }
        
        let old_health = self.current;
        self.current = (self.current + amount).min(self.max);
        let actual_heal = self.current - old_health;
        
        Ok(actual_heal)
    }

    /// Apply regeneration
    pub fn regenerate(&mut self) -> f32 {
        let old_health = self.current;
        self.current = (self.current + self.regen_rate).min(self.max);
        self.current - old_health
    }

    /// Check if alive
    pub fn is_alive(&self) -> bool {
        self.current > 0.0
    }

    /// Check if at full health
    pub fn is_full_health(&self) -> bool {
        (self.current - self.max).abs() < f32::EPSILON
    }

    /// Get health percentage (0.0 to 1.0)
    pub fn percentage(&self) -> f32 {
        if self.max <= 0.0 {
            0.0
        } else {
            self.current / self.max
        }
    }
}

impl Validate for Health {
    type Error = ComponentError;

    fn validate(&self) -> Result<(), Self::Error> {
        utils::validate_health(self.current, self.max)?;
        
        if self.armor < 0.0 || self.armor > 1.0 {
            return Err(ComponentError::InvalidHealth(
                format!("Armor must be between 0.0 and 1.0, got {}", self.armor)
            ));
        }
        
        Ok(())
    }

    fn constraints() -> &'static str {
        "Max health must be positive; current health ≥ 0 and ≤ max; armor between 0.0 and 1.0"
    }
}

/// Ownership tracking for entities
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Owner {
    pub player_id: u32,
    pub faction_id: Option<u32>,
}

impl Owner {
    /// Create new owner
    pub fn new(player_id: u32) -> Self {
        Self {
            player_id,
            faction_id: None,
        }
    }

    /// Create with faction
    pub fn with_faction(player_id: u32, faction_id: u32) -> Self {
        Self {
            player_id,
            faction_id: Some(faction_id),
        }
    }

    /// Create neutral owner (typically for terrain/resources)
    pub fn neutral() -> Self {
        Self {
            player_id: 0, // Player ID 0 = neutral
            faction_id: None,
        }
    }

    /// Create player owner
    pub fn player(player_id: u32, _is_human: bool) -> ComponentResult<Self> {
        if player_id == 0 {
            return Err(ComponentError::InvalidOwner(
                "Player ID cannot be 0 (reserved for neutral)".to_string()
            ));
        }
        Ok(Self {
            player_id,
            faction_id: None,
        })
    }

    /// Check if owned by player
    pub fn is_owned_by(&self, player_id: u32) -> bool {
        self.player_id == player_id
    }

    /// Check if neutral
    pub fn is_neutral(&self) -> bool {
        self.player_id == 0
    }

    /// Check if same faction
    pub fn is_same_faction(&self, other: &Owner) -> bool {
        match (self.faction_id, other.faction_id) {
            (Some(a), Some(b)) => a == b,
            _ => self.player_id == other.player_id,
        }
    }
}

/// Named entity component with validation
#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Name {
    value: String,
    /// Immutable flag for system-generated names
    immutable: bool,
}

impl Name {
    /// Create new name with validation
    pub fn new(name: String) -> ComponentResult<Self> {
        utils::validate_name(&name)?;
        
        Ok(Self {
            value: name,
            immutable: false,
        })
    }

    /// Create immutable name (for system entities)
    pub fn new_immutable(name: String) -> ComponentResult<Self> {
        utils::validate_name(&name)?;
        
        Ok(Self {
            value: name,
            immutable: true,
        })
    }

    /// Get name value
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Set new name (if mutable)
    pub fn set(&mut self, name: String) -> ComponentResult<()> {
        if self.immutable {
            return Err(ComponentError::InvalidName("Cannot modify immutable name".to_string()));
        }
        
        utils::validate_name(&name)?;
        self.value = name;
        Ok(())
    }

    /// Check if name is immutable
    pub fn is_immutable(&self) -> bool {
        self.immutable
    }
}

impl Validate for Name {
    type Error = ComponentError;

    fn validate(&self) -> Result<(), Self::Error> {
        utils::validate_name(&self.value)
    }

    fn constraints() -> &'static str {
        "Name must be 1-100 characters and contain no control characters"
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

/// Implement Interpolate for Position
impl Interpolate for Position {
    fn interpolate(&self, other: &Self, t: InterpolationFactor) -> Self {
        let t_val = t.into_inner();
        let new_hex = IVec2::new(
            (self.hex.x as f32 + (other.hex.x - self.hex.x) as f32 * t_val) as i32,
            (self.hex.y as f32 + (other.hex.y - self.hex.y) as f32 * t_val) as i32,
        );
        let new_pixel = self.pixel + (other.pixel - self.pixel) * t_val;
        Self {
            hex: new_hex,
            pixel: new_pixel,
        }
    }
}

/// Implement Interpolate for Health
impl Interpolate for Health {
    fn interpolate(&self, other: &Self, t: InterpolationFactor) -> Self {
        let t_val = t.into_inner();
        Self {
            current: self.current + (other.current - self.current) * t_val,
            max: self.max + (other.max - self.max) * t_val,
            regen_rate: self.regen_rate + (other.regen_rate - self.regen_rate) * t_val,
            armor: self.armor + (other.armor - self.armor) * t_val,
        }
    }
}

/// Marker component for entities that are currently selected by the player
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct GameSelection {
    /// Player ID that has selected this entity
    pub player_id: u32,
    /// Selection timestamp for prioritizing multiple selections
    pub selected_at: u64,
}

impl GameSelection {
    /// Create new game selection
    pub fn new(player_id: u32) -> Self {
        Self {
            player_id,
            selected_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }

    /// Check if selected by specific player
    pub fn is_selected_by(&self, player_id: u32) -> bool {
        self.player_id == player_id
    }
}
