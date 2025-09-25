//! Elevation data structures with noise generation support
//!
//! Provides elevation components for tiles with support for noise-based
//! variation and slope calculations for movement and gameplay mechanics.

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

/// Elevation data with noise generation support
#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub struct Elevation {
    /// Base elevation in meters
    pub base: f32,
    /// Noise-generated variation
    pub variation: f32,
    /// Final computed elevation
    pub final_elevation: f32,
    /// Slope gradient (for movement calculations)
    pub slope: f32,
}

impl Default for Elevation {
    fn default() -> Self {
        Self {
            base: 0.0,
            variation: 0.0,
            final_elevation: 0.0,
            slope: 0.0,
        }
    }
}

impl Elevation {
    /// Create elevation with noise variation
    pub fn with_noise(base: f32, noise_value: f32, amplitude: f32) -> Self {
        let variation = noise_value * amplitude;
        let final_elevation = base + variation;
        
        Self {
            base,
            variation,
            final_elevation,
            slope: 0.0, // Will be calculated separately
        }
    }

    /// Create elevation with just base value
    pub fn new(base: f32) -> Self {
        Self {
            base,
            variation: 0.0,
            final_elevation: base,
            slope: 0.0,
        }
    }

    /// Update slope based on neighboring elevations
    pub fn update_slope(&mut self, neighbor_elevations: &[f32]) {
        if neighbor_elevations.is_empty() {
            return;
        }

        let max_diff = neighbor_elevations.iter()
            .map(|&elev| (self.final_elevation - elev).abs())
            .fold(0.0f32, f32::max);
            
        self.slope = max_diff / 100.0; // Normalize slope
    }

    /// Get elevation tier for gameplay mechanics
    pub fn tier(&self) -> ElevationTier {
        match self.final_elevation {
            e if e < -1000.0 => ElevationTier::DeepOcean,
            e if e < 0.0 => ElevationTier::ShallowWater,
            e if e < 200.0 => ElevationTier::Lowland,
            e if e < 800.0 => ElevationTier::Highland,
            e if e < 2000.0 => ElevationTier::Mountain,
            _ => ElevationTier::Peak,
        }
    }

    /// Check if this elevation provides defensive advantage
    pub fn has_defensive_advantage(&self) -> bool {
        self.final_elevation > 200.0 || self.slope > 0.1
    }

    /// Calculate movement cost modifier based on elevation difference
    pub fn movement_cost_modifier(&self, target_elevation: f32) -> f32 {
        let elevation_diff = target_elevation - self.final_elevation;
        if elevation_diff > 0.0 {
            // Moving uphill is harder
            1.0 + (elevation_diff / 1000.0).min(2.0)
        } else {
            // Moving downhill is easier
            (1.0 + (elevation_diff / 1000.0)).max(0.5)
        }
    }
}

/// Elevation tiers for gameplay mechanics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ElevationTier {
    DeepOcean,
    ShallowWater,
    Lowland,
    Highland,
    Mountain,
    Peak,
}

impl ElevationTier {
    /// Get defensive bonus for this elevation tier
    pub fn defensive_bonus(&self) -> f32 {
        match self {
            Self::DeepOcean => 0.0,
            Self::ShallowWater => 0.0,
            Self::Lowland => 0.0,
            Self::Highland => 0.1,
            Self::Mountain => 0.25,
            Self::Peak => 0.4,
        }
    }

    /// Get visibility range modifier for this elevation tier
    pub fn visibility_modifier(&self) -> f32 {
        match self {
            Self::DeepOcean => 0.9,
            Self::ShallowWater => 1.0,
            Self::Lowland => 1.0,
            Self::Highland => 1.2,
            Self::Mountain => 1.5,
            Self::Peak => 2.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elevation_creation() {
        let elevation = Elevation::new(500.0);
        assert_eq!(elevation.base, 500.0);
        assert_eq!(elevation.final_elevation, 500.0);
        assert_eq!(elevation.tier(), ElevationTier::Highland);
    }

    #[test]
    fn test_elevation_with_noise() {
        let elevation = Elevation::with_noise(100.0, 0.5, 50.0);
        assert_eq!(elevation.base, 100.0);
        assert_eq!(elevation.variation, 25.0);
        assert_eq!(elevation.final_elevation, 125.0);
    }

    #[test]
    fn test_slope_calculation() {
        let mut elevation = Elevation::new(200.0);
        let neighbors = vec![150.0, 180.0, 220.0, 250.0];
        elevation.update_slope(&neighbors);
        assert!(elevation.slope > 0.0);
    }

    #[test]
    fn test_elevation_tiers() {
        assert_eq!(Elevation::new(-500.0).tier(), ElevationTier::ShallowWater);
        assert_eq!(Elevation::new(100.0).tier(), ElevationTier::Lowland);
        assert_eq!(Elevation::new(400.0).tier(), ElevationTier::Highland);
        assert_eq!(Elevation::new(1500.0).tier(), ElevationTier::Mountain);
    }

    #[test]
    fn test_movement_cost_modifier() {
        let elevation = Elevation::new(200.0);
        
        // Moving uphill should be harder
        let uphill = elevation.movement_cost_modifier(400.0);
        assert!(uphill > 1.0);
        
        // Moving downhill should be easier
        let downhill = elevation.movement_cost_modifier(100.0);
        assert!(downhill < 1.0);
    }

    #[test]
    fn test_defensive_advantage() {
        let lowland = Elevation::new(100.0);
        assert!(!lowland.has_defensive_advantage());
        
        let highland = Elevation::new(300.0);
        assert!(highland.has_defensive_advantage());
        
        let mut steep_slope = Elevation::new(100.0);
        steep_slope.slope = 0.15;
        assert!(steep_slope.has_defensive_advantage());
    }
}
