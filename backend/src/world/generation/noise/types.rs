//! Strongly-typed noise configurations and enums
//!
//! Provides comprehensive type safety for noise generation with
//! deterministic configurations and extensible parameter sets.

use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString, IntoStaticStr};
use ordered_float::OrderedFloat;

/// Noise type enumeration for type-safe selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(EnumIter, EnumString, Display, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum NoiseType {
    Simplex,
    Perlin,
    Voronoi,
    Worley,
    Fbm,
    Ridged,
    DomainWarped,
    Mixed,
}

/// Noise quality settings affecting performance vs. quality tradeoff
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(EnumIter, EnumString, Display)]
pub enum NoiseQuality {
    Low,      // Fast, lower quality
    Medium,   // Balanced
    High,     // Slower, high quality
    Ultra,    // Maximum quality
}

/// Interpolation method for noise smoothing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(EnumIter, EnumString, Display)]
pub enum Interpolation {
    Linear,
    Cosine,
    Cubic,
    Quintic,
}

/// Simplex noise configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimplexConfig {
    /// Noise frequency/scale
    pub frequency: OrderedFloat<f64>,
    /// Amplitude/strength of noise
    pub amplitude: OrderedFloat<f64>,
    /// Number of octaves for detail
    pub octaves: u32,
    /// Lacunarity (frequency multiplier per octave)
    pub lacunarity: OrderedFloat<f64>,
    /// Persistence (amplitude multiplier per octave)
    pub persistence: OrderedFloat<f64>,
    /// Quality setting
    pub quality: NoiseQuality,
}

impl Default for SimplexConfig {
    fn default() -> Self {
        Self {
            frequency: OrderedFloat(0.01),
            amplitude: OrderedFloat(1.0),
            octaves: 4,
            lacunarity: OrderedFloat(2.0),
            persistence: OrderedFloat(0.5),
            quality: NoiseQuality::Medium,
        }
    }
}

/// Perlin noise configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerlinConfig {
    pub frequency: OrderedFloat<f64>,
    pub amplitude: OrderedFloat<f64>,
    pub octaves: u32,
    pub lacunarity: OrderedFloat<f64>,
    pub persistence: OrderedFloat<f64>,
    pub quality: NoiseQuality,
    pub interpolation: Interpolation,
}

impl Default for PerlinConfig {
    fn default() -> Self {
        Self {
            frequency: OrderedFloat(0.01),
            amplitude: OrderedFloat(1.0),
            octaves: 4,
            lacunarity: OrderedFloat(2.0),
            persistence: OrderedFloat(0.5),
            quality: NoiseQuality::Medium,
            interpolation: Interpolation::Quintic,
        }
    }
}

/// Voronoi diagram configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoronoiConfig {
    /// Number of seed points
    pub point_count: u32,
    /// Distance function to use
    pub distance_function: VoronoiDistance,
    /// Enable cellular features
    pub cellular: bool,
    /// Seed for point generation
    pub point_seed: u64,
    /// Jittering amount for organic look
    pub jitter: OrderedFloat<f64>,
}

impl Default for VoronoiConfig {
    fn default() -> Self {
        Self {
            point_count: 100,
            distance_function: VoronoiDistance::Euclidean,
            cellular: false,
            point_seed: 54321,
            jitter: OrderedFloat(0.5),
        }
    }
}

/// Distance functions for Voronoi diagrams
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(EnumIter, EnumString, Display)]
pub enum VoronoiDistance {
    Euclidean,   // Standard distance
    Manhattan,   // City block distance
    Chebyshev,   // Chess king distance
    Minkowski,   // Generalized distance
}

/// Worley noise (cellular noise) configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorleyConfig {
    /// Density of cell points
    pub density: OrderedFloat<f64>,
    /// Which distance to return (1st, 2nd, 3rd closest)
    pub distance_order: u32,
    /// Distance function
    pub distance_function: VoronoiDistance,
    /// Enable fractal Worley
    pub fractal: bool,
    /// Fractal parameters
    pub fractal_octaves: u32,
    pub fractal_frequency: OrderedFloat<f64>,
}

impl Default for WorleyConfig {
    fn default() -> Self {
        Self {
            density: OrderedFloat(0.1),
            distance_order: 1,
            distance_function: VoronoiDistance::Euclidean,
            fractal: false,
            fractal_octaves: 3,
            fractal_frequency: OrderedFloat(2.0),
        }
    }
}

/// Fractal Brownian Motion (FBM) configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FbmConfig {
    /// Base noise type
    pub base_type: NoiseType,
    /// Number of octaves
    pub octaves: u32,
    /// Frequency of base noise
    pub frequency: OrderedFloat<f64>,
    /// Lacunarity (frequency multiplier)
    pub lacunarity: OrderedFloat<f64>,
    /// Persistence (amplitude multiplier)
    pub persistence: OrderedFloat<f64>,
    /// Optional gain curve
    pub gain: OrderedFloat<f64>,
    /// Weighted strength for higher octaves
    pub weighted_strength: OrderedFloat<f64>,
    /// Ping-pong effect amplitude
    pub ping_pong_strength: OrderedFloat<f64>,
}

impl Default for FbmConfig {
    fn default() -> Self {
        Self {
            base_type: NoiseType::Simplex,
            octaves: 6,
            frequency: OrderedFloat(0.01),
            lacunarity: OrderedFloat(2.0),
            persistence: OrderedFloat(0.5),
            gain: OrderedFloat(0.5),
            weighted_strength: OrderedFloat(0.0),
            ping_pong_strength: OrderedFloat(2.0),
        }
    }
}

/// Domain warping configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainWarpConfig {
    /// Amplitude of warping
    pub amplitude: OrderedFloat<f64>,
    /// Frequency of warp pattern
    pub frequency: OrderedFloat<f64>,
    /// Type of noise for warping
    pub warp_type: NoiseType,
    /// Number of warp iterations
    pub iterations: u32,
    /// Rotation angle for warp direction
    pub rotation: OrderedFloat<f64>,
}

impl Default for DomainWarpConfig {
    fn default() -> Self {
        Self {
            amplitude: OrderedFloat(30.0),
            frequency: OrderedFloat(0.02),
            warp_type: NoiseType::Simplex,
            iterations: 1,
            rotation: OrderedFloat(0.0),
        }
    }
}

/// Ridged noise configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RidgeConfig {
    /// Base noise type
    pub base_type: NoiseType,
    /// Ridge sharpness
    pub sharpness: OrderedFloat<f64>,
    /// Ridge offset
    pub offset: OrderedFloat<f64>,
    /// Gain scaling
    pub gain: OrderedFloat<f64>,
    /// Frequency
    pub frequency: OrderedFloat<f64>,
    /// Number of octaves
    pub octaves: u32,
    /// Lacunarity
    pub lacunarity: OrderedFloat<f64>,
}

impl Default for RidgeConfig {
    fn default() -> Self {
        Self {
            base_type: NoiseType::Simplex,
            sharpness: OrderedFloat(1.0),
            offset: OrderedFloat(1.0),
            gain: OrderedFloat(2.0),
            frequency: OrderedFloat(0.01),
            octaves: 4,
            lacunarity: OrderedFloat(2.0),
        }
    }
}

/// Noise mixing operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(EnumIter, EnumString, Display)]
pub enum MixOperation {
    Add,
    Subtract,
    Multiply,
    Divide,
    Max,
    Min,
    Blend,
    Overlay,
    Turbulence,
}

/// Noise layer for mixing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoiseLayer {
    pub noise_type: NoiseType,
    pub weight: OrderedFloat<f64>,
    pub operation: MixOperation,
    pub enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn test_noise_type_enum() {
        // Test enum iteration
        let types: Vec<_> = NoiseType::iter().collect();
        assert!(!types.is_empty());
        
        // Test string conversion
        assert_eq!(NoiseType::Simplex.to_string(), "simplex");
    }

    #[test]
    fn test_ordered_float_determinism() {
        let config1 = SimplexConfig::default();
        let config2 = SimplexConfig::default();
        
        assert_eq!(config1.frequency, config2.frequency);
        assert_eq!(config1.amplitude, config2.amplitude);
    }

    #[test]
    fn test_config_serialization() {
        let config = SimplexConfig::default();
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: SimplexConfig = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(config.frequency, deserialized.frequency);
        assert_eq!(config.octaves, deserialized.octaves);
    }
}
