//! Environmental components for tiles
//!
//! Contains climate, fertility, and other environmental components that affect
//! tile properties and gameplay mechanics.

use bevy_ecs::prelude::Component;
use serde::{Deserialize, Serialize};

/// Climate data component for environmental simulation
#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub struct Climate {
    /// Average temperature (-50 to 50 Celsius)
    pub temperature: i8,
    /// Rainfall amount (0-255mm annually)
    pub rainfall: u8,
    /// Humidity percentage (0-100)
    pub humidity: u8,
    /// Wind strength (0-255 arbitrary units)
    pub wind_strength: u8,
}

impl Default for Climate {
    fn default() -> Self {
        Self {
            temperature: 20, // 20°C
            rainfall: 100,   // 100mm
            humidity: 50,    // 50%
            wind_strength: 10, // Light wind
        }
    }
}

/// Fertility component for agricultural potential
#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub struct Fertility {
    /// Base fertility value (0.0 to 1.0)
    pub base_fertility: f32,
    /// Current fertility (affected by usage/improvements)
    pub current_fertility: f32,
    /// Fertility regeneration rate
    pub regen_rate: f32,
}

impl Default for Fertility {
    fn default() -> Self {
        Self {
            base_fertility: 0.5,
            current_fertility: 0.5,
            regen_rate: 0.01,
        }
    }
}
