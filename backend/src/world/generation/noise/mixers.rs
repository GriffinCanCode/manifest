//! Advanced noise mixing and combination systems
//!
//! Provides sophisticated noise blending operations using multiple
//! mathematical approaches for complex terrain generation.

use super::types::*;
use super::NoiseResult;
use crate::core::hashing::{FastHasher, HashStrategies};
use ordered_float::OrderedFloat;
use std::collections::HashMap;
use rayon::prelude::*;

/// Advanced noise mixer with multiple blending operations
#[derive(Debug)]
pub struct NoiseMixer {
    layers: Vec<NoiseLayer>,
    blend_cache: HashMap<u64, f32>,
    turbulence_octaves: u32,
    turbulence_frequency: OrderedFloat<f64>,
}

impl Default for NoiseMixer {
    fn default() -> Self {
        Self::new()
    }
}

impl NoiseMixer {
    /// Create new noise mixer
    pub fn new() -> Self {
        Self {
            layers: Vec::new(),
            blend_cache: HashMap::new(),
            turbulence_octaves: 4,
            turbulence_frequency: OrderedFloat(0.02),
        }
    }

    /// Add noise layer for mixing
    pub fn add_layer(&mut self, layer: NoiseLayer) {
        self.layers.push(layer);
    }

    /// Remove noise layer by index
    pub fn remove_layer(&mut self, index: usize) -> Option<NoiseLayer> {
        if index < self.layers.len() {
            Some(self.layers.remove(index))
        } else {
            None
        }
    }

    /// Mix two noise values using specified operation
    pub fn mix_values(&self, value1: f32, value2: f32, operation: MixOperation, weight: f32) -> f32 {
        match operation {
            MixOperation::Add => value1 + value2 * weight,
            MixOperation::Subtract => value1 - value2 * weight,
            MixOperation::Multiply => value1 * (1.0 + value2 * weight),
            MixOperation::Divide => {
                let divisor = 1.0 + value2 * weight;
                if divisor.abs() > f32::EPSILON {
                    value1 / divisor
                } else {
                    value1
                }
            }
            MixOperation::Max => value1.max(value2 * weight),
            MixOperation::Min => value1.min(value2 * weight),
            MixOperation::Blend => Self::smooth_blend(value1, value2, weight),
            MixOperation::Overlay => Self::overlay_blend(value1, value2, weight),
            MixOperation::Turbulence => self.turbulence_mix(value1, value2, weight),
        }
    }

    /// Smooth blend using cosine interpolation
    fn smooth_blend(a: f32, b: f32, t: f32) -> f32 {
        let smooth_t = (1.0 - (t * std::f32::consts::PI).cos()) * 0.5;
        a * (1.0 - smooth_t) + b * smooth_t
    }

    /// Overlay blend (photoshop-style overlay)
    fn overlay_blend(base: f32, overlay: f32, weight: f32) -> f32 {
        let normalized_base = (base + 1.0) * 0.5; // Convert from [-1,1] to [0,1]
        let normalized_overlay = (overlay + 1.0) * 0.5;
        
        let result = if normalized_base < 0.5 {
            2.0 * normalized_base * normalized_overlay
        } else {
            1.0 - 2.0 * (1.0 - normalized_base) * (1.0 - normalized_overlay)
        };
        
        let final_result = result * 2.0 - 1.0; // Convert back to [-1,1]
        base * (1.0 - weight) + final_result * weight
    }

    /// Turbulence mixing using fractal distortion
    fn turbulence_mix(&self, value1: f32, value2: f32, weight: f32) -> f32 {
        // Create turbulent distortion
        let hash_input = ((value1 * 1000.0) as i32, (value2 * 1000.0) as i32);
        let hash = HashStrategies::hash_bytes(&bincode::serialize(&hash_input).unwrap_or_default());
        
        // Generate turbulence from hash
        let turbulence = Self::hash_to_float(hash) * 0.1;
        let distorted_weight = (weight + turbulence).clamp(0.0, 1.0);
        
        Self::smooth_blend(value1, value2, distorted_weight)
    }

    /// Batch mix multiple noise values with parallel processing
    pub fn mix_batch(&self, values: &[(f32, f32)], operation: MixOperation, weights: &[f32]) -> Vec<f32> {
        if values.len() != weights.len() {
            return Vec::new();
        }

        // Use parallel processing for large batches
        if values.len() > 100 {
            values.par_iter()
                .zip(weights.par_iter())
                .map(|((v1, v2), &weight)| self.mix_values(*v1, *v2, operation, weight))
                .collect()
        } else {
            values.iter()
                .zip(weights.iter())
                .map(|((v1, v2), &weight)| self.mix_values(*v1, *v2, operation, weight))
                .collect()
        }
    }

    /// Advanced multi-layer mixing
    pub fn mix_layers(&self, base_values: &[f32]) -> Vec<f32> {
        if self.layers.is_empty() {
            return base_values.to_vec();
        }

        let mut result = base_values.to_vec();
        
        for layer in &self.layers {
            if !layer.enabled {
                continue;
            }

            // Generate noise values for this layer (placeholder - would use actual generators)
            let layer_values: Vec<f32> = base_values.iter()
                .enumerate()
                .map(|(i, &base)| {
                    // Use deterministic hash-based generation for this example
                    let hash = HashStrategies::combine_hashes(&[
                        i as u64, 
                        (*layer.weight).to_bits() as u64,
                        layer.noise_type as u8 as u64
                    ]);
                    Self::hash_to_float(hash) * (*layer.weight as f32)
                })
                .collect();

            // Mix with current result
            let weights = vec![*layer.weight as f32; result.len()];
            result = self.mix_batch(
                &result.iter().zip(layer_values.iter()).map(|(&r, &l)| (r, l)).collect::<Vec<_>>(),
                layer.operation,
                &weights
            );
        }

        result
    }

    /// Selective mixing based on value thresholds
    pub fn selective_mix(&self, value1: f32, value2: f32, operation: MixOperation, weight: f32, threshold: f32) -> f32 {
        if value1.abs() > threshold {
            self.mix_values(value1, value2, operation, weight)
        } else {
            value1
        }
    }

    /// Gradient-based mixing (smooth transitions between different terrains)
    pub fn gradient_mix(&self, samples: &[NoiseResult], gradient_x: f32, gradient_y: f32) -> NoiseResult {
        if samples.is_empty() {
            return NoiseResult { height: 0.0, temperature: 0.0, moisture: 0.0 };
        }

        if samples.len() == 1 {
            return samples[0];
        }

        // Use gradient magnitude to determine mixing weights
        let gradient_strength = (gradient_x * gradient_x + gradient_y * gradient_y).sqrt();
        let normalized_strength = gradient_strength.clamp(0.0, 1.0);

        // Bilinear interpolation for multiple samples
        match samples.len() {
            2 => {
                let weight = normalized_strength;
                NoiseResult {
                    height: Self::smooth_blend(samples[0].height, samples[1].height, weight),
                    temperature: Self::smooth_blend(samples[0].temperature, samples[1].temperature, weight),
                    moisture: Self::smooth_blend(samples[0].moisture, samples[1].moisture, weight),
                }
            }
            4 => {
                // Bilinear interpolation for 2x2 grid
                let w_x = (gradient_x + 1.0) * 0.5; // Normalize to [0,1]
                let w_y = (gradient_y + 1.0) * 0.5;
                
                let top = NoiseResult {
                    height: Self::smooth_blend(samples[0].height, samples[1].height, w_x),
                    temperature: Self::smooth_blend(samples[0].temperature, samples[1].temperature, w_x),
                    moisture: Self::smooth_blend(samples[0].moisture, samples[1].moisture, w_x),
                };
                
                let bottom = NoiseResult {
                    height: Self::smooth_blend(samples[2].height, samples[3].height, w_x),
                    temperature: Self::smooth_blend(samples[2].temperature, samples[3].temperature, w_x),
                    moisture: Self::smooth_blend(samples[2].moisture, samples[3].moisture, w_x),
                };
                
                NoiseResult {
                    height: Self::smooth_blend(top.height, bottom.height, w_y),
                    temperature: Self::smooth_blend(top.temperature, bottom.temperature, w_y),
                    moisture: Self::smooth_blend(top.moisture, bottom.moisture, w_y),
                }
            }
            _ => {
                // Weighted average for arbitrary number of samples
                let total_weight: f32 = samples.len() as f32;
                let height_sum: f32 = samples.iter().map(|s| s.height).sum();
                let temp_sum: f32 = samples.iter().map(|s| s.temperature).sum();
                let moisture_sum: f32 = samples.iter().map(|s| s.moisture).sum();
                
                NoiseResult {
                    height: height_sum / total_weight,
                    temperature: temp_sum / total_weight,
                    moisture: moisture_sum / total_weight,
                }
            }
        }
    }

    /// Mask-based mixing using threshold values
    pub fn mask_mix(&self, base: f32, overlay: f32, mask: f32, threshold: f32, falloff: f32) -> f32 {
        if mask < threshold - falloff {
            base
        } else if mask > threshold + falloff {
            overlay
        } else {
            // Smooth transition in falloff region
            let t = (mask - (threshold - falloff)) / (2.0 * falloff);
            let smooth_t = Self::smooth_step(t);
            base * (1.0 - smooth_t) + overlay * smooth_t
        }
    }

    /// Smooth step function for natural transitions
    fn smooth_step(t: f32) -> f32 {
        let clamped = t.clamp(0.0, 1.0);
        clamped * clamped * (3.0 - 2.0 * clamped)
    }

    /// Hash to float conversion for deterministic pseudo-randomness
    fn hash_to_float(hash: u64) -> f32 {
        // Convert hash to float in range [-1, 1]
        ((hash as i64) as f32 / i64::MAX as f32).clamp(-1.0, 1.0)
    }

    /// Frequency-based mixing for multi-octave combination
    pub fn frequency_mix(&self, low_freq: f32, high_freq: f32, blend_frequency: f64, x: f64, y: f64) -> f32 {
        // Use spatial position to determine mixing weights
        let spatial_hash = HashStrategies::combine_hashes(&[
            (x * blend_frequency) as u64,
            (y * blend_frequency) as u64,
        ]);
        
        let blend_weight = (Self::hash_to_float(spatial_hash) + 1.0) * 0.5; // Normalize to [0,1]
        Self::smooth_blend(low_freq, high_freq, blend_weight)
    }

    /// Clear blend cache
    pub fn clear_cache(&mut self) {
        self.blend_cache.clear();
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, usize) {
        (self.blend_cache.len(), self.blend_cache.capacity())
    }

    /// Set turbulence parameters
    pub fn set_turbulence_params(&mut self, octaves: u32, frequency: f64) {
        self.turbulence_octaves = octaves;
        self.turbulence_frequency = OrderedFloat(frequency);
    }
}

/// Specialized mixer for terrain generation
#[derive(Debug)]
pub struct TerrainMixer {
    base_mixer: NoiseMixer,
    elevation_influence: f32,
    temperature_influence: f32,
    moisture_influence: f32,
}

impl TerrainMixer {
    /// Create terrain-specific mixer
    pub fn new() -> Self {
        Self {
            base_mixer: NoiseMixer::new(),
            elevation_influence: 0.7,
            temperature_influence: 0.8,
            moisture_influence: 0.6,
        }
    }

    /// Mix terrain values considering elevation, temperature, and moisture
    pub fn mix_terrain(&self, samples: &[NoiseResult], elevation: f32, temperature: f32, moisture: f32) -> NoiseResult {
        let mut result = self.base_mixer.gradient_mix(samples, 0.0, 0.0);

        // Apply environmental influences
        result.height *= 1.0 + elevation * self.elevation_influence * 0.1;
        result.temperature = Self::apply_elevation_to_temperature(result.temperature, elevation);
        result.moisture = Self::apply_temperature_to_moisture(result.moisture, result.temperature);

        result
    }

    /// Apply elevation effects to temperature (higher = cooler)
    fn apply_elevation_to_temperature(base_temp: f32, elevation: f32) -> f32 {
        let lapse_rate = 0.0065; // Temperature decreases with altitude
        let elevation_effect = elevation * lapse_rate;
        (base_temp - elevation_effect).clamp(0.0, 1.0)
    }

    /// Apply temperature effects to moisture capacity
    fn apply_temperature_to_moisture(base_moisture: f32, temperature: f32) -> f32 {
        // Warmer air holds more moisture
        let temp_factor = 0.5 + temperature * 0.5;
        (base_moisture * temp_factor).clamp(0.0, 1.0)
    }

    /// Set influence parameters
    pub fn set_influences(&mut self, elevation: f32, temperature: f32, moisture: f32) {
        self.elevation_influence = elevation.clamp(0.0, 1.0);
        self.temperature_influence = temperature.clamp(0.0, 1.0);
        self.moisture_influence = moisture.clamp(0.0, 1.0);
    }
}

impl Default for TerrainMixer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noise_mixer_creation() {
        let mixer = NoiseMixer::new();
        assert_eq!(mixer.layers.len(), 0);
        assert_eq!(mixer.blend_cache.len(), 0);
    }

    #[test]
    fn test_mix_operations() {
        let mixer = NoiseMixer::new();
        
        let v1 = 0.5;
        let v2 = 0.3;
        let weight = 0.5;

        let add_result = mixer.mix_values(v1, v2, MixOperation::Add, weight);
        assert!((add_result - 0.65).abs() < f32::EPSILON);

        let blend_result = mixer.mix_values(v1, v2, MixOperation::Blend, weight);
        assert!(blend_result > 0.3 && blend_result < 0.5);
    }

    #[test]
    fn test_batch_mixing() {
        let mixer = NoiseMixer::new();
        let values = vec![(0.1, 0.2), (0.3, 0.4), (0.5, 0.6)];
        let weights = vec![0.5, 0.5, 0.5];
        
        let results = mixer.mix_batch(&values, MixOperation::Add, &weights);
        assert_eq!(results.len(), 3);
        
        for result in results {
            assert!(result >= 0.0 && result <= 1.0);
        }
    }

    #[test]
    fn test_gradient_mixing() {
        let mixer = NoiseMixer::new();
        let samples = vec![
            NoiseResult { height: 0.1, temperature: 0.2, moisture: 0.3 },
            NoiseResult { height: 0.4, temperature: 0.5, moisture: 0.6 },
        ];
        
        let result = mixer.gradient_mix(&samples, 0.5, 0.5);
        
        assert!(result.height > 0.1 && result.height < 0.4);
        assert!(result.temperature > 0.2 && result.temperature < 0.5);
        assert!(result.moisture > 0.3 && result.moisture < 0.6);
    }

    #[test]
    fn test_terrain_mixer() {
        let mut terrain_mixer = TerrainMixer::new();
        terrain_mixer.set_influences(0.8, 0.7, 0.6);
        
        let samples = vec![
            NoiseResult { height: 0.5, temperature: 0.7, moisture: 0.4 },
        ];
        
        let result = terrain_mixer.mix_terrain(&samples, 0.8, 0.6, 0.5);
        
        assert!(result.height >= 0.0);
        assert!(result.temperature >= 0.0 && result.temperature <= 1.0);
        assert!(result.moisture >= 0.0 && result.moisture <= 1.0);
    }

    #[test]
    fn test_smooth_step_function() {
        assert_eq!(NoiseMixer::smooth_step(0.0), 0.0);
        assert_eq!(NoiseMixer::smooth_step(1.0), 1.0);
        assert!(NoiseMixer::smooth_step(0.5) > 0.0 && NoiseMixer::smooth_step(0.5) < 1.0);
    }

    #[test]
    fn test_hash_to_float_range() {
        let hash1 = 12345u64;
        let hash2 = u64::MAX;
        let hash3 = 0u64;
        
        let f1 = NoiseMixer::hash_to_float(hash1);
        let f2 = NoiseMixer::hash_to_float(hash2);
        let f3 = NoiseMixer::hash_to_float(hash3);
        
        assert!(f1 >= -1.0 && f1 <= 1.0);
        assert!(f2 >= -1.0 && f2 <= 1.0);
        assert!(f3 >= -1.0 && f3 <= 1.0);
    }
}
