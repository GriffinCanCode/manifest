//! Interpolated components for smooth animations
//!
//! Contains components that support smooth interpolation for visual effects.

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

use crate::core::interpolate::{Interpolate, InterpolatedProperty, Color as InterpolateColor};
use super::{
    core::{Position, Health},
    rendering::{Renderable, Color},
    validation::{Validate, ComponentError}
};

/// Interpolated position for smooth movement animations
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct InterpolatedPosition {
    /// Current interpolated position property
    pub position: InterpolatedProperty<Position>,
    /// Animation duration for movements
    pub animation_duration: Duration,
    /// Whether animation is currently active
    pub animating: bool,
}

impl InterpolatedPosition {
    /// Create new interpolated position
    pub fn new(initial_position: Position, animation_duration: Duration) -> Self {
        Self {
            position: InterpolatedProperty::new(initial_position),
            animation_duration,
            animating: false,
        }
    }

    /// Start animation to new position
    pub fn animate_to(&mut self, target: Position, duration: Option<Duration>) {
        let duration = duration.unwrap_or(self.animation_duration);
        self.position.animate_to(target, duration);
        self.animating = true;
    }

    /// Update animation (call each frame)
    pub fn update(&mut self, delta_time: Duration) {
        let was_animating = self.animating;
        self.position.update(delta_time);
        
        // Check if animation finished
        if was_animating && !self.position.is_animating() {
            self.animating = false;
        }
    }

    /// Get current interpolated position
    pub fn current(&self) -> Position {
        self.position.current()
    }

    /// Get target position
    pub fn target(&self) -> Position {
        self.position.target()
    }

    /// Check if currently animating
    pub fn is_animating(&self) -> bool {
        self.position.is_animating()
    }

    /// Stop animation immediately
    pub fn stop_animation(&mut self) {
        self.position.stop();
        self.animating = false;
    }

    /// Snap to target position immediately
    pub fn snap_to_target(&mut self) {
        self.position.snap_to_target();
        self.animating = false;
    }

    /// Set new position without animation
    pub fn set_immediate(&mut self, position: Position) {
        self.position.set_immediate(position);
        self.animating = false;
    }
}

impl Validate for InterpolatedPosition {
    type Error = ComponentError;

    fn validate(&self) -> Result<(), Self::Error> {
        self.position.current().validate()
    }

    fn constraints() -> &'static str {
        "Position must be valid according to Position constraints"
    }
}

/// Interpolated health for smooth health bar animations
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct InterpolatedHealth {
    /// Current interpolated health property
    pub health: InterpolatedProperty<Health>,
    /// Animation duration for health changes
    pub animation_duration: Duration,
    /// Whether animation is currently active
    pub animating: bool,
    /// Damage flash effect
    pub damage_flash: Option<DamageFlash>,
}

/// Damage flash visual effect
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DamageFlash {
    /// Flash start time (stored as elapsed time since creation)
    #[serde(skip, default = "std::time::Instant::now")]
    pub start_time: Instant,
    /// Flash duration
    pub duration: Duration,
    /// Flash color
    pub color: Color,
    /// Flash intensity (0.0 to 1.0)
    pub intensity: f32,
}

impl InterpolatedHealth {
    /// Create new interpolated health
    pub fn new(initial_health: Health, animation_duration: Duration) -> Self {
        Self {
            health: InterpolatedProperty::new(initial_health),
            animation_duration,
            animating: false,
            damage_flash: None,
        }
    }

    /// Start animation to new health
    pub fn animate_to(&mut self, target: Health, duration: Option<Duration>) {
        let duration = duration.unwrap_or(self.animation_duration);
        self.health.animate_to(target, duration);
        self.animating = true;
    }

    /// Trigger damage flash effect
    pub fn flash_damage(&mut self, color: Color, intensity: f32, duration: Duration) {
        self.damage_flash = Some(DamageFlash {
            start_time: Instant::now(),
            duration,
            color,
            intensity: intensity.clamp(0.0, 1.0),
        });
    }

    /// Update animation and effects (call each frame)
    pub fn update(&mut self, delta_time: Duration) {
        let was_animating = self.animating;
        self.health.update(delta_time);
        
        // Check if health animation finished
        if was_animating && !self.health.is_animating() {
            self.animating = false;
        }
        
        // Update damage flash
        if let Some(flash) = &self.damage_flash {
            if flash.start_time.elapsed() >= flash.duration {
                self.damage_flash = None;
            }
        }
    }

    /// Get current interpolated health
    pub fn current(&self) -> Health {
        self.health.current()
    }

    /// Get target health
    pub fn target(&self) -> Health {
        self.health.target()
    }

    /// Check if currently animating
    pub fn is_animating(&self) -> bool {
        self.health.is_animating()
    }

    /// Check if damage flash is active
    pub fn is_flashing(&self) -> bool {
        self.damage_flash.is_some()
    }

    /// Get current flash intensity (0.0 to 1.0)
    pub fn flash_intensity(&self) -> f32 {
        if let Some(flash) = &self.damage_flash {
            let elapsed = flash.start_time.elapsed();
            let progress = elapsed.as_secs_f32() / flash.duration.as_secs_f32();
            if progress < 1.0 {
                // Fade out flash intensity
                flash.intensity * (1.0 - progress)
            } else {
                0.0
            }
        } else {
            0.0
        }
    }

    /// Get current flash color
    pub fn flash_color(&self) -> Option<Color> {
        if self.is_flashing() {
            self.damage_flash.as_ref().map(|f| f.color)
        } else {
            None
        }
    }

    /// Stop animation immediately
    pub fn stop_animation(&mut self) {
        self.health.stop();
        self.animating = false;
    }

    /// Snap to target health immediately
    pub fn snap_to_target(&mut self) {
        self.health.snap_to_target();
        self.animating = false;
    }

    /// Set new health without animation
    pub fn set_immediate(&mut self, health: Health) {
        self.health.set_immediate(health);
        self.animating = false;
    }
}

impl Validate for InterpolatedHealth {
    type Error = ComponentError;

    fn validate(&self) -> Result<(), Self::Error> {
        self.health.current().validate()
    }

    fn constraints() -> &'static str {
        "Health must be valid according to Health constraints"
    }
}

/// Interpolated renderable for smooth visual effects
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct InterpolatedRenderable {
    /// Current interpolated renderable property
    pub renderable: InterpolatedProperty<Renderable>,
    /// Animation duration for visual changes
    pub animation_duration: Duration,
    /// Whether animation is currently active
    pub animating: bool,
    /// Color pulse effect
    pub color_pulse: Option<ColorPulse>,
}

/// Color pulse visual effect
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorPulse {
    /// Pulse start color
    pub start_color: Color,
    /// Pulse end color
    pub end_color: Color,
    /// Pulse duration
    pub duration: Duration,
    /// Current pulse time
    pub elapsed: Duration,
    /// Whether pulse should loop
    pub looping: bool,
    /// Pulse direction (forward/backward)
    pub forward: bool,
}

impl InterpolatedRenderable {
    /// Create new interpolated renderable
    pub fn new(initial_renderable: Renderable, animation_duration: Duration) -> Self {
        Self {
            renderable: InterpolatedProperty::new(initial_renderable),
            animation_duration,
            animating: false,
            color_pulse: None,
        }
    }

    /// Start animation to new renderable state
    pub fn animate_to(&mut self, target: Renderable, duration: Option<Duration>) {
        let duration = duration.unwrap_or(self.animation_duration);
        self.renderable.animate_to(target, duration);
        self.animating = true;
    }

    /// Start color pulse effect
    pub fn start_color_pulse(&mut self, start_color: Color, end_color: Color, duration: Duration, looping: bool) {
        self.color_pulse = Some(ColorPulse {
            start_color,
            end_color,
            duration,
            elapsed: Duration::ZERO,
            looping,
            forward: true,
        });
    }

    /// Stop color pulse effect
    pub fn stop_color_pulse(&mut self) {
        self.color_pulse = None;
    }

    /// Update animation and effects (call each frame)
    pub fn update(&mut self, delta_time: Duration) {
        let was_animating = self.animating;
        self.renderable.update(delta_time);
        
        // Check if renderable animation finished
        if was_animating && !self.renderable.is_animating() {
            self.animating = false;
        }
        
        // Update color pulse
        if let Some(pulse) = &mut self.color_pulse {
            pulse.elapsed += delta_time;
            
            if pulse.elapsed >= pulse.duration {
                if pulse.looping {
                    if pulse.forward {
                        pulse.forward = false;
                        pulse.elapsed = Duration::ZERO;
                        std::mem::swap(&mut pulse.start_color, &mut pulse.end_color);
                    } else {
                        pulse.forward = true;
                        pulse.elapsed = Duration::ZERO;
                        std::mem::swap(&mut pulse.start_color, &mut pulse.end_color);
                    }
                } else {
                    self.color_pulse = None;
                }
            }
        }
    }

    /// Get current interpolated renderable with pulse effect applied
    pub fn current(&self) -> Renderable {
        let mut renderable = self.renderable.current();
        
        // Apply color pulse effect
        if let Some(pulse) = &self.color_pulse {
            let progress = pulse.elapsed.as_secs_f32() / pulse.duration.as_secs_f32();
            let progress = progress.clamp(0.0, 1.0);
            
            let pulsed_color = pulse.start_color.lerp(&pulse.end_color, progress);
            renderable.set_color(pulsed_color);
        }
        
        renderable
    }

    /// Get target renderable
    pub fn target(&self) -> Renderable {
        self.renderable.target()
    }

    /// Check if currently animating
    pub fn is_animating(&self) -> bool {
        self.renderable.is_animating()
    }

    /// Check if color pulse is active
    pub fn is_pulsing(&self) -> bool {
        self.color_pulse.is_some()
    }

    /// Stop animation immediately
    pub fn stop_animation(&mut self) {
        self.renderable.stop();
        self.animating = false;
    }

    /// Snap to target renderable immediately
    pub fn snap_to_target(&mut self) {
        self.renderable.snap_to_target();
        self.animating = false;
    }

    /// Set new renderable without animation
    pub fn set_immediate(&mut self, renderable: Renderable) {
        self.renderable.set_immediate(renderable);
        self.animating = false;
    }
}

impl Validate for InterpolatedRenderable {
    type Error = ComponentError;

    fn validate(&self) -> Result<(), Self::Error> {
        self.renderable.current().validate()
    }

    fn constraints() -> &'static str {
        "Renderable must be valid according to Renderable constraints"
    }
}
