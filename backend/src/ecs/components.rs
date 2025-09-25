//! Core game components with advanced serde patterns
//!
//! Each component follows single responsibility principle with sophisticated
//! serialization, validation, and strong typing to minimize tech debt.

use bevy_ecs::prelude::*;
use glam::{IVec2, Vec2};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tracing::{info, debug, warn, error, instrument};
use std::fmt;
use nalgebra::{Point2, Point3};
use crate::core::zig_ffi::{hex_distance as zig_hex_distance, hex_to_pixel as zig_hex_to_pixel, HexCoord, PixelPos};

use crate::core::{
    logging::{LoggingSystem, game_logging},
    interpolate::{Interpolate, InterpolatedProperty, Color as InterpolateColor}
};

/// Validation trait for component constraints and business rules
pub trait Validate {
    type Error: std::error::Error + Send + Sync + 'static;
    
    /// Validate component state
    fn validate(&self) -> Result<(), Self::Error>;
    
    /// Get validation constraints as human-readable text
    fn constraints() -> &'static str;
}

/// Component error types for strong error handling
#[derive(Debug, thiserror::Error)]
pub enum ComponentError {
    #[error("Invalid position: {0}")]
    InvalidPosition(String),
    #[error("Invalid movement: {0}")]
    InvalidMovement(String),
    #[error("Invalid health: {0}")]
    InvalidHealth(String),
    #[error("Invalid name: {0}")]
    InvalidName(String),
    #[error("Invalid hierarchy: {0}")]
    InvalidHierarchy(String),
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
    pub fn new(q: i32, r: i32) -> Result<Self, ComponentError> {
        let correlation_id = LoggingSystem::generate_correlation_id();
        
        if q.abs() > 10000 || r.abs() > 10000 {
            error!(
                target: "game::components::position",
                correlation_id = correlation_id,
                q = q,
                r = r,
                max_bound = 10000,
                "Position coordinates exceed world bounds"
            );
            return Err(ComponentError::InvalidPosition(
                format!("Coordinates ({}, {}) exceed world bounds", q, r)
            ));
        }
        
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

    /// Get hex coordinates
    pub fn hex(&self) -> IVec2 {
        self.hex
    }

    /// Get pixel coordinates
    pub fn pixel(&self) -> Vec2 {
        self.pixel
    }

    /// Update hex position with validation and recalculate pixel coordinates
    pub fn set_hex(&mut self, q: i32, r: i32) -> Result<(), ComponentError> {
        if q.abs() > 10000 || r.abs() > 10000 {
            return Err(ComponentError::InvalidPosition(
                format!("Coordinates ({}, {}) exceed world bounds", q, r)
            ));
        }
        
        self.hex = IVec2::new(q, r);
        self.pixel = hex_to_pixel(self.hex);
        Ok(())
    }

    /// Calculate distance to another position using Zig SIMD optimizations
    pub fn distance_to(&self, other: &Position) -> u32 {
        let a = HexCoord::new(self.hex.x, self.hex.y);
        let b = HexCoord::new(other.hex.x, other.hex.y);
        zig_hex_distance(a, b)
    }
}

impl Validate for Position {
    type Error = ComponentError;

    fn validate(&self) -> Result<(), Self::Error> {
        if self.hex.x.abs() > 10000 || self.hex.y.abs() > 10000 {
            return Err(ComponentError::InvalidPosition(
                format!("Position ({}, {}) exceeds world bounds", self.hex.x, self.hex.y)
            ));
        }
        Ok(())
    }

    fn constraints() -> &'static str {
        "Hex coordinates must be within [-10000, 10000] range"
    }
}

impl Serialize for Position {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Only serialize hex coordinates, pixel is derived
        self.hex.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Position {
    fn deserialize<D>(deserializer: D) -> Result<Position, D::Error>
    where
        D: Deserializer<'de>,
    {
        let hex = IVec2::deserialize(deserializer)?;
        Ok(Position {
            hex,
            pixel: hex_to_pixel(hex),
        })
    }
}

/// Movement capabilities and current movement state with validation
#[derive(Component, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "MovementData", into = "MovementData")]
pub struct Movement {
    /// Current movement points available this turn
    points: f32,
    /// Maximum movement points per turn
    max_points: f32,
    /// Movement cost multiplier (1.0 = normal, 0.5 = fast, 2.0 = slow)
    cost_modifier: f32,
}

/// Serialization data structure for Movement
#[derive(Serialize, Deserialize)]
struct MovementData {
    points: f32,
    max_points: f32,
    cost_modifier: f32,
}

impl Movement {
    /// Create new movement with validation
    pub fn new(max_points: f32) -> Result<Self, ComponentError> {
        if max_points <= 0.0 || max_points > 1000.0 {
            return Err(ComponentError::InvalidMovement(
                format!("Max points {} must be between 0.1 and 1000.0", max_points)
            ));
        }

        Ok(Self {
            points: max_points,
            max_points,
            cost_modifier: 1.0,
        })
    }

    /// Get current movement points
    pub fn points(&self) -> f32 {
        self.points
    }

    /// Get maximum movement points
    pub fn max_points(&self) -> f32 {
        self.max_points
    }

    /// Get cost modifier
    pub fn cost_modifier(&self) -> f32 {
        self.cost_modifier
    }

    /// Alias for points() - get current movement points
    pub fn current(&self) -> f32 {
        self.points
    }

    /// Alias for max_points() - get maximum movement points
    pub fn max(&self) -> f32 {
        self.max_points
    }

    /// Set cost modifier with validation
    pub fn set_cost_modifier(&mut self, modifier: f32) -> Result<(), ComponentError> {
        if modifier <= 0.0 || modifier > 10.0 {
            return Err(ComponentError::InvalidMovement(
                format!("Cost modifier {} must be between 0.1 and 10.0", modifier)
            ));
        }
        self.cost_modifier = modifier;
        Ok(())
    }

    /// Check if entity can move with given cost
    pub fn can_move(&self, cost: f32) -> bool {
        cost >= 0.0 && self.points >= cost * self.cost_modifier
    }

    /// Consume movement points, returns actual cost consumed
    pub fn consume(&mut self, cost: f32) -> Result<f32, ComponentError> {
        if cost < 0.0 {
            return Err(ComponentError::InvalidMovement(
                "Movement cost cannot be negative".to_string()
            ));
        }

        let actual_cost = cost * self.cost_modifier;
        if actual_cost > self.points {
            return Err(ComponentError::InvalidMovement(
                format!("Insufficient movement points: need {}, have {}", actual_cost, self.points)
            ));
        }

        self.points = (self.points - actual_cost).max(0.0);
        Ok(actual_cost)
    }

    /// Restore movement points for new turn
    pub fn restore(&mut self) {
        self.points = self.max_points;
    }

    /// Get movement efficiency (0.0 to 1.0)
    pub fn efficiency(&self) -> f32 {
        if self.max_points > 0.0 {
            self.points / self.max_points
        } else {
            0.0
        }
    }
}

impl Validate for Movement {
    type Error = ComponentError;

    fn validate(&self) -> Result<(), Self::Error> {
        if self.max_points <= 0.0 || self.max_points > 1000.0 {
            return Err(ComponentError::InvalidMovement(
                format!("Max points {} out of valid range [0.1, 1000.0]", self.max_points)
            ));
        }
        if self.points < 0.0 || self.points > self.max_points {
            return Err(ComponentError::InvalidMovement(
                format!("Current points {} out of valid range [0, {}]", self.points, self.max_points)
            ));
        }
        if self.cost_modifier <= 0.0 || self.cost_modifier > 10.0 {
            return Err(ComponentError::InvalidMovement(
                format!("Cost modifier {} out of valid range [0.1, 10.0]", self.cost_modifier)
            ));
        }
        Ok(())
    }

    fn constraints() -> &'static str {
        "Max points: [0.1, 1000.0], current points: [0, max_points], cost modifier: [0.1, 10.0]"
    }
}

impl From<Movement> for MovementData {
    fn from(movement: Movement) -> Self {
        Self {
            points: movement.points,
            max_points: movement.max_points,
            cost_modifier: movement.cost_modifier,
        }
    }
}

impl TryFrom<MovementData> for Movement {
    type Error = ComponentError;

    fn try_from(data: MovementData) -> Result<Self, Self::Error> {
        let movement = Self {
            points: data.points,
            max_points: data.max_points,
            cost_modifier: data.cost_modifier,
        };
        movement.validate()?;
        Ok(movement)
    }
}

/// Visual representation data for rendering with strong validation
#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "RenderableData", into = "RenderableData")]
pub struct Renderable {
    /// Sprite/model identifier
    sprite: String,
    /// Rendering layer (0 = terrain, 1 = improvements, 2 = units, etc.)
    layer: u8,
    /// Visual scale multiplier
    scale: f32,
    /// Color tint (RGBA)
    tint: [f32; 4],
    /// Whether this entity is visible
    visible: bool,
}

/// Serialization data for Renderable
#[derive(Serialize, Deserialize)]
struct RenderableData {
    sprite: String,
    layer: u8,
    scale: f32,
    tint: [f32; 4],
    visible: bool,
}

impl Renderable {
    /// Create new renderable with validation
    pub fn new(sprite: impl Into<String>, layer: u8) -> Result<Self, ComponentError> {
        let sprite = sprite.into();
        if sprite.is_empty() {
            return Err(ComponentError::InvalidName(
                "Sprite identifier cannot be empty".to_string()
            ));
        }

        Ok(Self {
            sprite,
            layer,
            scale: 1.0,
            tint: [1.0, 1.0, 1.0, 1.0],
            visible: true,
        })
    }

    /// Get sprite identifier
    pub fn sprite(&self) -> &str {
        &self.sprite
    }

    /// Get rendering layer
    pub fn layer(&self) -> u8 {
        self.layer
    }

    /// Get scale
    pub fn scale(&self) -> f32 {
        self.scale
    }

    /// Get tint color
    pub fn tint(&self) -> [f32; 4] {
        self.tint
    }

    /// Check if visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Set sprite with validation
    pub fn set_sprite(&mut self, sprite: impl Into<String>) -> Result<(), ComponentError> {
        let sprite = sprite.into();
        if sprite.is_empty() {
            return Err(ComponentError::InvalidName(
                "Sprite identifier cannot be empty".to_string()
            ));
        }
        self.sprite = sprite;
        Ok(())
    }

    /// Set scale with validation
    pub fn set_scale(&mut self, scale: f32) -> Result<(), ComponentError> {
        if scale <= 0.0 || scale > 100.0 {
            return Err(ComponentError::InvalidName(
                format!("Scale {} must be between 0.1 and 100.0", scale)
            ));
        }
        self.scale = scale;
        Ok(())
    }

    /// Set tint color with validation
    pub fn set_tint(&mut self, tint: [f32; 4]) -> Result<(), ComponentError> {
        if tint.iter().any(|&c| c < 0.0 || c > 1.0) {
            return Err(ComponentError::InvalidName(
                "Tint values must be between 0.0 and 1.0".to_string()
            ));
        }
        self.tint = tint;
        Ok(())
    }

    /// Set visibility
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }
}

impl Validate for Renderable {
    type Error = ComponentError;

    fn validate(&self) -> Result<(), Self::Error> {
        if self.sprite.is_empty() {
            return Err(ComponentError::InvalidName(
                "Sprite identifier cannot be empty".to_string()
            ));
        }
        if self.scale <= 0.0 || self.scale > 100.0 {
            return Err(ComponentError::InvalidName(
                format!("Scale {} out of valid range [0.1, 100.0]", self.scale)
            ));
        }
        if self.tint.iter().any(|&c| c < 0.0 || c > 1.0) {
            return Err(ComponentError::InvalidName(
                "Tint values must be between 0.0 and 1.0".to_string()
            ));
        }
        Ok(())
    }

    fn constraints() -> &'static str {
        "Sprite: non-empty string, scale: [0.1, 100.0], tint: RGBA in [0.0, 1.0]"
    }
}

impl From<Renderable> for RenderableData {
    fn from(renderable: Renderable) -> Self {
        Self {
            sprite: renderable.sprite,
            layer: renderable.layer,
            scale: renderable.scale,
            tint: renderable.tint,
            visible: renderable.visible,
        }
    }
}

impl TryFrom<RenderableData> for Renderable {
    type Error = ComponentError;

    fn try_from(data: RenderableData) -> Result<Self, Self::Error> {
        let renderable = Self {
            sprite: data.sprite,
            layer: data.layer,
            scale: data.scale,
            tint: data.tint,
            visible: data.visible,
        };
        renderable.validate()?;
        Ok(renderable)
    }
}

/// Ownership and control information with validation
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Owner {
    /// Player/civilization ID (0 = neutral, 1+ = players)  
    player_id: u32,
    /// Whether this is controlled by human or AI
    is_human: bool,
}

impl Owner {
    /// Create neutral ownership (no player)
    pub fn neutral() -> Self {
        Self {
            player_id: 0,
            is_human: false,
        }
    }

    /// Create player ownership with validation
    pub fn player(player_id: u32, is_human: bool) -> Result<Self, ComponentError> {
        if player_id == 0 {
            return Err(ComponentError::InvalidName(
                "Player ID must be greater than 0 (0 is reserved for neutral)".to_string()
            ));
        }
        if player_id > 1000 {
            return Err(ComponentError::InvalidName(
                format!("Player ID {} exceeds maximum of 1000", player_id)
            ));
        }
        
        Ok(Self { player_id, is_human })
    }

    /// Get player ID
    pub fn player_id(&self) -> u32 {
        self.player_id
    }

    /// Check if controlled by human
    pub fn is_human(&self) -> bool {
        self.is_human
    }

    /// Check if neutral (no owner)
    pub fn is_neutral(&self) -> bool {
        self.player_id == 0
    }
}

impl Validate for Owner {
    type Error = ComponentError;

    fn validate(&self) -> Result<(), Self::Error> {
        if self.player_id > 1000 {
            return Err(ComponentError::InvalidName(
                format!("Player ID {} exceeds maximum of 1000", self.player_id)
            ));
        }
        Ok(())
    }

    fn constraints() -> &'static str {
        "Player ID: [0, 1000] where 0 = neutral"
    }
}

/// Health and damage system with validation and sophisticated state tracking
#[derive(Component, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "HealthData", into = "HealthData")]
pub struct Health {
    /// Current health points
    current: f32,
    /// Maximum health points
    maximum: f32,
}

/// Serialization data for Health
#[derive(Serialize, Deserialize)]
struct HealthData {
    current: f32,
    maximum: f32,
}

impl Health {
    /// Create new health with validation
    #[instrument(name = "health_new", fields(max_health = max_health))]
    pub fn new(max_health: f32) -> Result<Self, ComponentError> {
        let correlation_id = LoggingSystem::generate_correlation_id();
        
        if max_health <= 0.0 || max_health > 100000.0 {
            error!(
                target: "game::components::health",
                correlation_id = correlation_id,
                max_health = max_health,
                min_bound = 0.1,
                max_bound = 100000.0,
                "Invalid health value - exceeds bounds"
            );
            return Err(ComponentError::InvalidHealth(
                format!("Max health {} must be between 0.1 and 100000.0", max_health)
            ));
        }

        debug!(
            target: "game::components::health",
            correlation_id = correlation_id,
            max_health = max_health,
            current_health = max_health,
            "Health component created successfully"
        );

        Ok(Self {
            current: max_health,
            maximum: max_health,
        })
    }

    /// Get current health
    pub fn current(&self) -> f32 {
        self.current
    }

    /// Get maximum health
    pub fn maximum(&self) -> f32 {
        self.maximum
    }

    /// Alias for maximum() - get maximum health
    pub fn max(&self) -> f32 {
        self.maximum
    }

    /// Apply damage with validation, returns actual damage dealt
    pub fn damage(&mut self, amount: f32) -> Result<f32, ComponentError> {
        if amount < 0.0 {
            return Err(ComponentError::InvalidHealth(
                "Damage amount cannot be negative".to_string()
            ));
        }
        if amount > 100000.0 {
            return Err(ComponentError::InvalidHealth(
                format!("Damage amount {} exceeds maximum of 100000.0", amount)
            ));
        }

        let old_health = self.current;
        self.current = (self.current - amount).max(0.0);
        Ok(old_health - self.current)
    }

    /// Apply healing with validation, returns actual health restored
    pub fn heal(&mut self, amount: f32) -> Result<f32, ComponentError> {
        if amount < 0.0 {
            return Err(ComponentError::InvalidHealth(
                "Healing amount cannot be negative".to_string()
            ));
        }
        if amount > 100000.0 {
            return Err(ComponentError::InvalidHealth(
                format!("Healing amount {} exceeds maximum of 100000.0", amount)
            ));
        }

        let old_health = self.current;
        self.current = (self.current + amount).min(self.maximum);
        Ok(self.current - old_health)
    }

    /// Set current health with validation
    pub fn set_current(&mut self, health: f32) -> Result<(), ComponentError> {
        if health < 0.0 || health > self.maximum {
            return Err(ComponentError::InvalidHealth(
                format!("Current health {} must be between 0 and {}", health, self.maximum)
            ));
        }
        self.current = health;
        Ok(())
    }

    /// Set maximum health with validation
    pub fn set_maximum(&mut self, max_health: f32) -> Result<(), ComponentError> {
        if max_health <= 0.0 || max_health > 100000.0 {
            return Err(ComponentError::InvalidHealth(
                format!("Max health {} must be between 0.1 and 100000.0", max_health)
            ));
        }
        
        self.maximum = max_health;
        // Clamp current health to new maximum
        self.current = self.current.min(self.maximum);
        Ok(())
    }

    /// Check if entity is alive
    pub fn is_alive(&self) -> bool {
        self.current > 0.0
    }

    /// Check if entity is at full health
    pub fn is_full_health(&self) -> bool {
        (self.current - self.maximum).abs() < 0.001
    }

    /// Get health percentage (0.0 to 1.0)
    pub fn percentage(&self) -> f32 {
        if self.maximum > 0.0 {
            self.current / self.maximum
        } else {
            0.0
        }
    }

    /// Get missing health amount
    pub fn missing(&self) -> f32 {
        self.maximum - self.current
    }
}

impl Validate for Health {
    type Error = ComponentError;

    fn validate(&self) -> Result<(), Self::Error> {
        if self.maximum <= 0.0 || self.maximum > 100000.0 {
            return Err(ComponentError::InvalidHealth(
                format!("Max health {} out of valid range [0.1, 100000.0]", self.maximum)
            ));
        }
        if self.current < 0.0 || self.current > self.maximum {
            return Err(ComponentError::InvalidHealth(
                format!("Current health {} out of valid range [0, {}]", self.current, self.maximum)
            ));
        }
        Ok(())
    }

    fn constraints() -> &'static str {
        "Max health: [0.1, 100000.0], current health: [0, max_health]"
    }
}

impl From<Health> for HealthData {
    fn from(health: Health) -> Self {
        Self {
            current: health.current,
            maximum: health.maximum,
        }
    }
}

impl TryFrom<HealthData> for Health {
    type Error = ComponentError;

    fn try_from(data: HealthData) -> Result<Self, Self::Error> {
        let health = Self {
            current: data.current,
            maximum: data.maximum,
        };
        health.validate()?;
        Ok(health)
    }
}

/// Entity name for identification and display with validation
#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Name {
    /// The display name of the entity
    value: String,
}

impl Name {
    /// Create new name with validation
    pub fn new(name: impl Into<String>) -> Result<Self, ComponentError> {
        let value = name.into();
        if value.is_empty() {
            return Err(ComponentError::InvalidName(
                "Name cannot be empty".to_string()
            ));
        }
        if value.len() > 100 {
            return Err(ComponentError::InvalidName(
                format!("Name length {} exceeds maximum of 100 characters", value.len())
            ));
        }
        if !value.chars().all(|c| c.is_alphanumeric() || c.is_whitespace() || "'-_.,()[]".contains(c)) {
            return Err(ComponentError::InvalidName(
                "Name contains invalid characters. Only alphanumeric, whitespace, and '-_.,()[] are allowed".to_string()
            ));
        }

        Ok(Self { value })
    }

    /// Get the name value
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Set new name with validation
    pub fn set(&mut self, name: impl Into<String>) -> Result<(), ComponentError> {
        let new_name = Self::new(name)?;
        self.value = new_name.value;
        Ok(())
    }

    /// Check if name matches (case-insensitive)
    pub fn matches(&self, other: &str) -> bool {
        self.value.to_lowercase() == other.to_lowercase()
    }

    /// Get name length
    pub fn len(&self) -> usize {
        self.value.len()
    }

    /// Check if name is empty (this shouldn't be possible after validation)
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }
}

impl Validate for Name {
    type Error = ComponentError;

    fn validate(&self) -> Result<(), Self::Error> {
        if self.value.is_empty() {
            return Err(ComponentError::InvalidName(
                "Name cannot be empty".to_string()
            ));
        }
        if self.value.len() > 100 {
            return Err(ComponentError::InvalidName(
                format!("Name length {} exceeds maximum of 100 characters", self.value.len())
            ));
        }
        if !self.value.chars().all(|c| c.is_alphanumeric() || c.is_whitespace() || "'-_.,()[]".contains(c)) {
            return Err(ComponentError::InvalidName(
                "Name contains invalid characters".to_string()
            ));
        }
        Ok(())
    }

    fn constraints() -> &'static str {
        "Length: [1, 100], characters: alphanumeric + whitespace + '-_.,()[]"
    }
}

impl From<Name> for String {
    fn from(name: Name) -> Self {
        name.value
    }
}

impl TryFrom<String> for Name {
    type Error = ComponentError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

/// Hex coordinate to pixel coordinate conversion using Zig SIMD optimizations
/// Uses flat-topped hexagon layout with size = 1.0
fn hex_to_pixel(hex: IVec2) -> Vec2 {
    const SIZE: f32 = 1.0;
    let coord = HexCoord::new(hex.x, hex.y);
    let pixel = zig_hex_to_pixel(coord, SIZE);
    Vec2::new(pixel.x, pixel.y)
}

// Interpolated component implementations for smooth rendering
impl Interpolate for Vec2 {
    fn interpolate(&self, other: &Self, t: crate::core::interpolate::InterpolationFactor) -> Self {
        let t = t.into_inner();
        Vec2::new(
            self.x + t * (other.x - self.x),
            self.y + t * (other.y - self.y),
        )
    }
}

/// Smooth position interpolation component for rendering
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct InterpolatedPosition {
    /// Interpolated pixel position for rendering
    pixel_position: InterpolatedProperty<Vec2>,
    /// Current hex position (for game logic)
    hex_position: IVec2,
}

impl InterpolatedPosition {
    /// Create new interpolated position from hex coordinates
    pub fn new(hex: IVec2) -> Self {
        let pixel = hex_to_pixel(hex);
        Self {
            pixel_position: InterpolatedProperty::new(pixel),
            hex_position: hex,
        }
    }

    /// Update position (call once per simulation tick)
    pub fn update_hex(&mut self, new_hex: IVec2) {
        if new_hex != self.hex_position {
            self.hex_position = new_hex;
            let new_pixel = hex_to_pixel(new_hex);
            self.pixel_position.update(new_pixel);
        }
    }

    /// Get interpolated pixel position for rendering
    pub fn interpolated_pixel(&self, factor: crate::core::interpolate::InterpolationFactor) -> Vec2 {
        self.pixel_position.interpolate(factor)
    }

    /// Get current hex position
    pub fn hex(&self) -> IVec2 {
        self.hex_position
    }

    /// Get current pixel position (no interpolation)
    pub fn pixel(&self) -> Vec2 {
        hex_to_pixel(self.hex_position)
    }
}

/// Smooth health interpolation component for health bar animations
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct InterpolatedHealth {
    /// Interpolated health value for smooth health bars
    health_value: InterpolatedProperty<f32>,
    /// Current actual health (for game logic)
    current_health: f32,
    /// Maximum health
    max_health: f32,
}

impl InterpolatedHealth {
    /// Create new interpolated health
    pub fn new(max_health: f32) -> Result<Self, ComponentError> {
        if max_health <= 0.0 {
            return Err(ComponentError::InvalidHealth(
                "Max health must be positive".to_string()
            ));
        }

        Ok(Self {
            health_value: InterpolatedProperty::new(max_health),
            current_health: max_health,
            max_health,
        })
    }

    /// Update health value (call once per simulation tick)
    pub fn update_health(&mut self, new_health: f32) {
        if new_health != self.current_health {
            self.current_health = new_health.clamp(0.0, self.max_health);
            self.health_value.update(self.current_health);
        }
    }

    /// Get interpolated health for smooth health bars
    pub fn interpolated_value(&self, factor: crate::core::interpolate::InterpolationFactor) -> f32 {
        self.health_value.interpolate(factor)
    }

    /// Get current health (no interpolation)
    pub fn current(&self) -> f32 {
        self.current_health
    }

    /// Get maximum health
    pub fn max(&self) -> f32 {
        self.max_health
    }

    /// Get interpolated percentage (0.0 to 1.0)
    pub fn interpolated_percentage(&self, factor: crate::core::interpolate::InterpolationFactor) -> f32 {
        if self.max_health > 0.0 {
            self.interpolated_value(factor) / self.max_health
        } else {
            0.0
        }
    }
}

/// Smooth rendering interpolation component for visual effects
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct InterpolatedRenderable {
    /// Interpolated scale for smooth scaling animations
    scale: InterpolatedProperty<f32>,
    /// Interpolated color tint for smooth color transitions
    tint: InterpolatedProperty<InterpolateColor>,
    /// Current values (for game logic)
    current_scale: f32,
    current_tint: InterpolateColor,
    /// Static properties that don't need interpolation
    sprite: String,
    layer: u8,
    visible: bool,
}

impl InterpolatedRenderable {
    /// Create new interpolated renderable
    pub fn new(sprite: impl Into<String>, layer: u8) -> Result<Self, ComponentError> {
        let sprite = sprite.into();
        if sprite.is_empty() {
            return Err(ComponentError::InvalidName(
                "Sprite identifier cannot be empty".to_string()
            ));
        }

        let initial_scale = 1.0;
        let initial_tint = InterpolateColor::rgb(1.0, 1.0, 1.0);

        Ok(Self {
            scale: InterpolatedProperty::new(initial_scale),
            tint: InterpolatedProperty::new(initial_tint),
            current_scale: initial_scale,
            current_tint: initial_tint,
            sprite,
            layer,
            visible: true,
        })
    }

    /// Update scale (call once per simulation tick)
    pub fn update_scale(&mut self, new_scale: f32) -> Result<(), ComponentError> {
        if new_scale <= 0.0 || new_scale > 100.0 {
            return Err(ComponentError::InvalidName(
                format!("Scale {} must be between 0.1 and 100.0", new_scale)
            ));
        }

        if new_scale != self.current_scale {
            self.current_scale = new_scale;
            self.scale.update(new_scale);
        }
        Ok(())
    }

    /// Update tint color (call once per simulation tick)
    pub fn update_tint(&mut self, r: f32, g: f32, b: f32, a: f32) -> Result<(), ComponentError> {
        if [r, g, b, a].iter().any(|&c| c < 0.0 || c > 1.0) {
            return Err(ComponentError::InvalidName(
                "Tint values must be between 0.0 and 1.0".to_string()
            ));
        }

        let new_tint = InterpolateColor::new(r, g, b, a);
        if new_tint != self.current_tint {
            self.current_tint = new_tint;
            self.tint.update(new_tint);
        }
        Ok(())
    }

    /// Get interpolated scale for rendering
    pub fn interpolated_scale(&self, factor: crate::core::interpolate::InterpolationFactor) -> f32 {
        self.scale.interpolate(factor)
    }

    /// Get interpolated tint for rendering
    pub fn interpolated_tint(&self, factor: crate::core::interpolate::InterpolationFactor) -> InterpolateColor {
        self.tint.interpolate(factor)
    }

    /// Get sprite identifier
    pub fn sprite(&self) -> &str {
        &self.sprite
    }

    /// Get rendering layer
    pub fn layer(&self) -> u8 {
        self.layer
    }

    /// Check if visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Set visibility
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    /// Get current scale (no interpolation)
    pub fn current_scale(&self) -> f32 {
        self.current_scale
    }

    /// Get current tint (no interpolation)
    pub fn current_tint(&self) -> InterpolateColor {
        self.current_tint
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_validation() {
        // Valid positions
        assert!(Position::new(0, 0).is_ok());
        assert!(Position::new(100, -50).is_ok());
        assert!(Position::new(-9999, 9999).is_ok());

        // Invalid positions
        assert!(Position::new(10001, 0).is_err());
        assert!(Position::new(0, -10001).is_err());
    }

    #[test]
    fn test_movement_validation() {
        // Valid movement
        let movement = Movement::new(3.0).unwrap();
        assert_eq!(movement.max_points(), 3.0);
        assert_eq!(movement.points(), 3.0);

        // Invalid movement
        assert!(Movement::new(0.0).is_err());
        assert!(Movement::new(1001.0).is_err());
    }

    #[test]
    fn test_health_operations() {
        let mut health = Health::new(100.0).unwrap();
        
        // Test damage
        assert_eq!(health.damage(30.0).unwrap(), 30.0);
        assert_eq!(health.current(), 70.0);
        
        // Test healing
        assert_eq!(health.heal(20.0).unwrap(), 20.0);
        assert_eq!(health.current(), 90.0);
        
        // Test over-healing
        assert_eq!(health.heal(20.0).unwrap(), 10.0);
        assert_eq!(health.current(), 100.0);
    }

    #[test]
    fn test_name_validation() {
        // Valid names
        assert!(Name::new("Test Unit").is_ok());
        assert!(Name::new("Player-1_City").is_ok());
        assert!(Name::new("Fort (Alpha)").is_ok());

        // Invalid names
        assert!(Name::new("").is_err());
        assert!(Name::new("a".repeat(101)).is_err());
        assert!(Name::new("Test@Unit#").is_err());
    }

    #[test]
    fn test_owner_validation() {
        // Valid owners
        assert!(Owner::player(1, true).is_ok());
        assert!(Owner::player(500, false).is_ok());
        
        let neutral = Owner::neutral();
        assert_eq!(neutral.player_id(), 0);
        assert!(neutral.is_neutral());

        // Invalid owners
        assert!(Owner::player(0, true).is_err());
        assert!(Owner::player(1001, false).is_err());
    }

    #[test]
    fn test_component_serialization() {
        use serde_json;
        
        // Test Position serialization
        let pos = Position::new_unchecked(5, -3);
        let json = serde_json::to_string(&pos).unwrap();
        let deserialized: Position = serde_json::from_str(&json).unwrap();
        assert_eq!(pos.hex(), deserialized.hex());
        assert_eq!(pos.pixel(), deserialized.pixel());

        // Test Movement serialization with validation
        let movement = Movement::new(5.0).unwrap();
        let json = serde_json::to_string(&movement).unwrap();
        let deserialized: Movement = serde_json::from_str(&json).unwrap();
        assert_eq!(movement.max_points(), deserialized.max_points());
        assert_eq!(movement.points(), deserialized.points());

        // Test Health serialization with validation
        let health = Health::new(100.0).unwrap();
        let json = serde_json::to_string(&health).unwrap();
        let deserialized: Health = serde_json::from_str(&json).unwrap();
        assert_eq!(health.maximum(), deserialized.maximum());
        assert_eq!(health.current(), deserialized.current());

        // Test Name serialization with validation
        let name = Name::new("Test Entity").unwrap();
        let json = serde_json::to_string(&name).unwrap();
        let deserialized: Name = serde_json::from_str(&json).unwrap();
        assert_eq!(name.value(), deserialized.value());

        // Test Owner serialization
        let owner = Owner::player(5, true).unwrap();
        let json = serde_json::to_string(&owner).unwrap();
        let deserialized: Owner = serde_json::from_str(&json).unwrap();
        assert_eq!(owner.player_id(), deserialized.player_id());
        assert_eq!(owner.is_human(), deserialized.is_human());

        // Test Renderable serialization with validation
        let mut renderable = Renderable::new("test_sprite", 2).unwrap();
        renderable.set_scale(1.5).unwrap();
        renderable.set_tint([0.5, 0.7, 0.9, 1.0]).unwrap();
        
        let json = serde_json::to_string(&renderable).unwrap();
        let deserialized: Renderable = serde_json::from_str(&json).unwrap();
        assert_eq!(renderable.sprite(), deserialized.sprite());
        assert_eq!(renderable.layer(), deserialized.layer());
        assert_eq!(renderable.scale(), deserialized.scale());
        assert_eq!(renderable.tint(), deserialized.tint());
    }

    #[test]
    fn test_serialization_validation_errors() {
        use serde_json;
        
        // Test invalid Movement deserialization
        let invalid_movement_json = r#"{"points": -1.0, "max_points": 5.0, "cost_modifier": 1.0}"#;
        assert!(serde_json::from_str::<Movement>(invalid_movement_json).is_err());

        // Test invalid Health deserialization
        let invalid_health_json = r#"{"current": 150.0, "maximum": 100.0}"#;
        assert!(serde_json::from_str::<Health>(invalid_health_json).is_err());

        // Test invalid Name deserialization
        let invalid_name_json = r#""""#; // Empty string
        assert!(serde_json::from_str::<Name>(invalid_name_json).is_err());

        // Test invalid Renderable deserialization
        let invalid_renderable_json = r#"{"sprite": "", "layer": 0, "scale": 1.0, "tint": [1.0, 1.0, 1.0, 1.0], "visible": true}"#;
        assert!(serde_json::from_str::<Renderable>(invalid_renderable_json).is_err());
    }

    #[test]
    fn test_component_validation_trait() {
        // Test all components implement Validate trait
        let pos = Position::new_unchecked(100, 200);
        assert!(pos.validate().is_ok());

        let movement = Movement::new(3.0).unwrap();
        assert!(movement.validate().is_ok());

        let health = Health::new(50.0).unwrap();
        assert!(health.validate().is_ok());

        let name = Name::new("Valid Name").unwrap();
        assert!(name.validate().is_ok());

        let owner = Owner::player(10, false).unwrap();
        assert!(owner.validate().is_ok());

        let renderable = Renderable::new("sprite", 1).unwrap();
        assert!(renderable.validate().is_ok());

        // Test constraint messages
        assert!(!Position::constraints().is_empty());
        assert!(!Movement::constraints().is_empty());
        assert!(!Health::constraints().is_empty());
        assert!(!Name::constraints().is_empty());
        assert!(!Owner::constraints().is_empty());
        assert!(!Renderable::constraints().is_empty());
    }
}
