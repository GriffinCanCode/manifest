//! Rendering-related components
//!
//! Contains components used for visual representation and rendering.

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::core::interpolate::{Color as InterpolateColor, Interpolate, InterpolationFactor};
use super::validation::{Validate, ComponentError, ComponentResult};

/// Visual representation component for entities
#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Renderable {
    /// Sprite/texture identifier
    pub sprite: String,
    /// Display color (RGBA)
    pub color: Color,
    /// Rendering layer/z-index
    pub layer: i32,
    /// Scale factor for rendering
    pub scale: f32,
    /// Rotation in radians
    pub rotation: f32,
    /// Whether the entity is visible
    pub visible: bool,
    /// Alpha/transparency (0.0 to 1.0)
    pub alpha: f32,
}

/// Color representation with RGBA channels
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    /// Create new color
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Create RGB color with full alpha
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self::new(r, g, b, 1.0)
    }

    /// Create RGBA color
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self::new(r, g, b, a)
    }

    /// White color
    pub const WHITE: Color = Color::new(1.0, 1.0, 1.0, 1.0);
    
    /// Black color
    pub const BLACK: Color = Color::new(0.0, 0.0, 0.0, 1.0);
    
    /// Red color
    pub const RED: Color = Color::new(1.0, 0.0, 0.0, 1.0);
    
    /// Green color
    pub const GREEN: Color = Color::new(0.0, 1.0, 0.0, 1.0);
    
    /// Blue color
    pub const BLUE: Color = Color::new(0.0, 0.0, 1.0, 1.0);
    
    /// Transparent color
    pub const TRANSPARENT: Color = Color::new(0.0, 0.0, 0.0, 0.0);

    /// Convert to hex string
    pub fn to_hex(&self) -> String {
        format!("#{:02X}{:02X}{:02X}{:02X}",
            (self.r * 255.0) as u8,
            (self.g * 255.0) as u8,
            (self.b * 255.0) as u8,
            (self.a * 255.0) as u8
        )
    }

    /// Create from hex string
    pub fn from_hex(hex: &str) -> Result<Self, ComponentError> {
        let hex = hex.trim_start_matches('#');
        
        let (r, g, b, a) = match hex.len() {
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| 
                    ComponentError::InvalidRenderable("Invalid hex color format".to_string()))?;
                let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| 
                    ComponentError::InvalidRenderable("Invalid hex color format".to_string()))?;
                let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| 
                    ComponentError::InvalidRenderable("Invalid hex color format".to_string()))?;
                (r, g, b, 255)
            },
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| 
                    ComponentError::InvalidRenderable("Invalid hex color format".to_string()))?;
                let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| 
                    ComponentError::InvalidRenderable("Invalid hex color format".to_string()))?;
                let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| 
                    ComponentError::InvalidRenderable("Invalid hex color format".to_string()))?;
                let a = u8::from_str_radix(&hex[6..8], 16).map_err(|_| 
                    ComponentError::InvalidRenderable("Invalid hex color format".to_string()))?;
                (r, g, b, a)
            },
            _ => return Err(ComponentError::InvalidRenderable("Hex color must be 6 or 8 characters".to_string())),
        };
        
        Ok(Color::new(
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
            a as f32 / 255.0,
        ))
    }

    /// Lerp between two colors
    pub fn lerp(&self, other: &Color, t: f32) -> Color {
        let t = t.clamp(0.0, 1.0);
        Color::new(
            self.r + (other.r - self.r) * t,
            self.g + (other.g - self.g) * t,
            self.b + (other.b - self.b) * t,
            self.a + (other.a - self.a) * t,
        )
    }

    /// Multiply color by factor
    pub fn multiply(&self, factor: f32) -> Color {
        Color::new(
            (self.r * factor).clamp(0.0, 1.0),
            (self.g * factor).clamp(0.0, 1.0),
            (self.b * factor).clamp(0.0, 1.0),
            self.a,
        )
    }

    /// Add colors together
    pub fn add(&self, other: &Color) -> Color {
        Color::new(
            (self.r + other.r).clamp(0.0, 1.0),
            (self.g + other.g).clamp(0.0, 1.0),
            (self.b + other.b).clamp(0.0, 1.0),
            (self.a + other.a).clamp(0.0, 1.0),
        )
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "rgba({:.3}, {:.3}, {:.3}, {:.3})", self.r, self.g, self.b, self.a)
    }
}

/// Convert to interpolation color
impl From<Color> for InterpolateColor {
    fn from(color: Color) -> Self {
        InterpolateColor { r: color.r, g: color.g, b: color.b, a: color.a }
    }
}

/// Convert from interpolation color
impl From<InterpolateColor> for Color {
    fn from(color: InterpolateColor) -> Self {
        Color::new(color.r, color.g, color.b, color.a)
    }
}

impl Renderable {
    /// Create new renderable component
    pub fn new(sprite: String) -> Self {
        Self {
            sprite,
            color: Color::WHITE,
            layer: 0,
            scale: 1.0,
            rotation: 0.0,
            visible: true,
            alpha: 1.0,
        }
    }

    /// Create with color
    pub fn with_color(sprite: String, color: Color) -> Self {
        Self {
            sprite,
            color,
            layer: 0,
            scale: 1.0,
            rotation: 0.0,
            visible: true,
            alpha: 1.0,
        }
    }

    /// Create with full parameters
    pub fn with_params(
        sprite: String,
        color: Color,
        layer: i32,
        scale: f32,
        rotation: f32,
        visible: bool,
        alpha: f32,
    ) -> ComponentResult<Self> {
        if scale < 0.0 {
            return Err(ComponentError::InvalidRenderable(
                format!("Scale cannot be negative: {}", scale)
            ));
        }
        
        if alpha < 0.0 || alpha > 1.0 {
            return Err(ComponentError::InvalidRenderable(
                format!("Alpha must be between 0.0 and 1.0: {}", alpha)
            ));
        }
        
        Ok(Self {
            sprite,
            color,
            layer,
            scale,
            rotation,
            visible,
            alpha,
        })
    }

    /// Set sprite
    pub fn set_sprite(&mut self, sprite: String) {
        self.sprite = sprite;
    }

    /// Set color
    pub fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    /// Set layer
    pub fn set_layer(&mut self, layer: i32) {
        self.layer = layer;
    }

    /// Set scale with validation
    pub fn set_scale(&mut self, scale: f32) -> ComponentResult<()> {
        if scale < 0.0 {
            return Err(ComponentError::InvalidRenderable(
                format!("Scale cannot be negative: {}", scale)
            ));
        }
        self.scale = scale;
        Ok(())
    }

    /// Set rotation
    pub fn set_rotation(&mut self, rotation: f32) {
        self.rotation = rotation;
    }

    /// Set visibility
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    /// Set alpha with validation
    pub fn set_alpha(&mut self, alpha: f32) -> ComponentResult<()> {
        if alpha < 0.0 || alpha > 1.0 {
            return Err(ComponentError::InvalidRenderable(
                format!("Alpha must be between 0.0 and 1.0: {}", alpha)
            ));
        }
        self.alpha = alpha;
        Ok(())
    }

    /// Hide entity
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Show entity
    pub fn show(&mut self) {
        self.visible = true;
    }

    /// Toggle visibility
    pub fn toggle_visibility(&mut self) {
        self.visible = !self.visible;
    }

    /// Check if should be rendered (visible and alpha > 0)
    pub fn should_render(&self) -> bool {
        self.visible && self.alpha > 0.0
    }

    /// Get effective color with alpha
    pub fn effective_color(&self) -> Color {
        Color::new(
            self.color.r,
            self.color.g,
            self.color.b,
            self.color.a * self.alpha,
        )
    }
}

impl Validate for Renderable {
    type Error = ComponentError;

    fn validate(&self) -> Result<(), Self::Error> {
        if self.scale < 0.0 {
            return Err(ComponentError::InvalidRenderable(
                format!("Scale cannot be negative: {}", self.scale)
            ));
        }
        
        if self.alpha < 0.0 || self.alpha > 1.0 {
            return Err(ComponentError::InvalidRenderable(
                format!("Alpha must be between 0.0 and 1.0: {}", self.alpha)
            ));
        }
        
        if self.color.r < 0.0 || self.color.r > 1.0 ||
           self.color.g < 0.0 || self.color.g > 1.0 ||
           self.color.b < 0.0 || self.color.b > 1.0 ||
           self.color.a < 0.0 || self.color.a > 1.0 {
            return Err(ComponentError::InvalidRenderable(
                "Color channels must be between 0.0 and 1.0".to_string()
            ));
        }
        
        Ok(())
    }

    fn constraints() -> &'static str {
        "Scale ≥ 0.0; alpha between 0.0-1.0; color channels between 0.0-1.0"
    }
}

impl Default for Renderable {
    fn default() -> Self {
        Self::new("default".to_string())
    }
}

/// Implement Interpolate for Color
impl Interpolate for Color {
    fn interpolate(&self, other: &Self, t: InterpolationFactor) -> Self {
        let t_val = t.into_inner();
        Self::new(
            self.r + (other.r - self.r) * t_val,
            self.g + (other.g - self.g) * t_val,
            self.b + (other.b - self.b) * t_val,
            self.a + (other.a - self.a) * t_val,
        )
    }
}

/// Implement Interpolate for Renderable
impl Interpolate for Renderable {
    fn interpolate(&self, other: &Self, t: InterpolationFactor) -> Self {
        let t_val = t.into_inner();
        Self {
            sprite: other.sprite.clone(), // Don't interpolate sprite name
            color: self.color.interpolate(&other.color, t),
            layer: other.layer, // Don't interpolate layer
            scale: self.scale + (other.scale - self.scale) * t_val,
            rotation: self.rotation + (other.rotation - self.rotation) * t_val,
            visible: other.visible, // Don't interpolate boolean
            alpha: self.alpha + (other.alpha - self.alpha) * t_val,
        }
    }
}
