//! Resource distribution algorithms and patterns
//!
//! Implements sophisticated distribution algorithms for realistic
//! resource placement based on geological and environmental factors.

use std::collections::HashMap;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;
use bevy_ecs::prelude::*;
use tracing::{debug, info};

use crate::world::tiles::{TileId, chunks::ChunkManager};
use crate::core::zig_ffi::HexCoord;
use crate::world::generation::noise::{NoiseGenerator, NoiseConfig};
use crate::world::generation::tectonics::TectonicPlate;

use super::types::*;
use super::{ResourceResult, ResourceDistributionError};

/// Advanced resource distribution engine
pub struct ResourceDistributionEngine {
    /// Noise generator for procedural patterns
    noise_generator: NoiseGenerator,
    /// Deterministic RNG
    rng: ChaCha8Rng,
    /// Distribution algorithms cache
    algorithms: HashMap<String, Box<dyn DistributionAlgorithm + Send + Sync>>,
}

/// Trait for resource distribution algorithms
pub trait DistributionAlgorithm: Send + Sync {
    /// Generate resource locations based on rules
    fn generate_locations(
        &self,
        resource_type: &ResourceType,
        world_bounds: (i32, i32, i32, i32), // min_q, max_q, min_r, max_r
        tectonic_data: &TectonicPlate,
        rng: &mut ChaCha8Rng,
    ) -> ResourceResult<Vec<ResourceCandidate>>;
}

/// Candidate resource location with probability
#[derive(Debug, Clone)]
pub struct ResourceCandidate {
    pub position: HexCoord,
    pub probability: f32,
    pub quality_modifier: f32,
    pub quantity_modifier: f32,
    pub geological_context: GeologicalContext,
}

/// Geological context for resource placement
#[derive(Debug, Clone)]
pub struct GeologicalContext {
    pub elevation: f32,
    pub plate_age: f32,
    pub tectonic_features: Vec<String>,
    pub distance_to_boundary: f32,
    pub volcanic_proximity: f32,
}

impl ResourceDistributionEngine {
    /// Create new distribution engine
    pub fn new(seed: u64) -> Self {
        let mut engine = Self {
            noise_generator: NoiseGenerator::new(&NoiseConfig::default()),
            rng: ChaCha8Rng::seed_from_u64(seed),
            algorithms: HashMap::new(),
        };
        
        // Register built-in algorithms
        engine.register_algorithm("scattered", Box::new(ScatteredDistribution::new(seed)));
        engine.register_algorithm("clustered", Box::new(ClusteredDistribution::new(seed)));
        engine.register_algorithm("vein", Box::new(VeinDistribution::new(seed)));
        engine.register_algorithm("geological", Box::new(GeologicalDistribution::new(seed)));
        
        engine
    }
    
    /// Register a distribution algorithm
    pub fn register_algorithm(&mut self, name: &str, algorithm: Box<dyn DistributionAlgorithm + Send + Sync>) {
        self.algorithms.insert(name.to_string(), algorithm);
        debug!("📐 Registered distribution algorithm: {}", name);
    }
    
    /// Generate resource candidates using specified algorithm
    pub fn generate_candidates(
        &mut self,
        resource_type: &ResourceType,
        algorithm_name: &str,
        world_bounds: (i32, i32, i32, i32),
        tectonic_data: &TectonicPlate,
    ) -> ResourceResult<Vec<ResourceCandidate>> {
        let algorithm = self.algorithms.get(algorithm_name)
            .ok_or_else(|| ResourceDistributionError::RuleNotFound(algorithm_name.to_string()))?;
        
        algorithm.generate_locations(resource_type, world_bounds, tectonic_data, &mut self.rng)
    }
    
    /// Evaluate resource placement probability at specific location
    pub fn evaluate_location_probability(
        &self,
        position: &TileId,
        resource_type: &ResourceType,
        tectonic_data: &TectonicPlate,
    ) -> f32 {
        let mut probability = 1.0;
        
        // Evaluate terrain affinity
        if let Some(terrain_affinity) = resource_type.distribution.terrain_affinity.get("default") {
            probability *= terrain_affinity;
        }
        
        // Evaluate geological rules
        // Convert TileId to HexCoord for geological context
        let id = position.0;
        let hex_coord = HexCoord::new((id % 1000) as i32, (id / 1000) as i32);
        let geological_context = self.get_geological_context(&hex_coord, tectonic_data);
        probability *= self.evaluate_geological_rules(&resource_type.distribution.geological_rules, &geological_context);
        
        // Add noise-based variation
        // Convert TileId to coordinates for noise sampling
        let id = position.0;
        let x = (id % 1000) as f64;
        let y = (id / 1000) as f64;
        let noise_value = self.noise_generator.sample_height(x, y);
        probability *= (noise_value + 1.0) / 2.0; // Normalize to 0-1
        
        probability.clamp(0.0, 1.0)
    }
    
    /// Get geological context for a position
    fn get_geological_context(&self, position: &HexCoord, tectonic_data: &TectonicPlate) -> GeologicalContext {
        // This would integrate with the actual tectonic system
        // For now, using placeholder values
        GeologicalContext {
            elevation: self.noise_generator.sample_height(position.q as f64 * 0.1, position.r as f64 * 0.1) * 1000.0,
            plate_age: 100.0, // Million years
            tectonic_features: vec!["continental_crust".to_string()],
            distance_to_boundary: 10.0,
            volcanic_proximity: 50.0,
        }
    }
    
    /// Evaluate geological rules for placement
    fn evaluate_geological_rules(&self, rules: &GeologicalRules, context: &GeologicalContext) -> f32 {
        let mut score = 1.0;
        
        // Elevation requirements
        if let Some((min_elev, max_elev)) = rules.elevation_range {
            if context.elevation < min_elev || context.elevation > max_elev {
                score *= 0.1; // Severe penalty for out-of-range elevation
            }
        }
        
        // Plate age requirements
        if let Some((min_age, max_age)) = rules.plate_age_range {
            if context.plate_age < min_age || context.plate_age > max_age {
                score *= 0.3; // Moderate penalty for suboptimal plate age
            }
        }
        
        // Tectonic feature requirements
        for required_feature in &rules.tectonic_features {
            if !context.tectonic_features.contains(required_feature) {
                score *= 0.5;
            }
        }
        
        score
    }
}

/// Scattered distribution algorithm
struct ScatteredDistribution {
    rng: ChaCha8Rng,
}

impl ScatteredDistribution {
    fn new(seed: u64) -> Self {
        Self {
            rng: ChaCha8Rng::seed_from_u64(seed.wrapping_add(1)),
        }
    }
}

impl DistributionAlgorithm for ScatteredDistribution {
    fn generate_locations(
        &self,
        resource_type: &ResourceType,
        world_bounds: (i32, i32, i32, i32),
        _tectonic_data: &TectonicPlate,
        rng: &mut ChaCha8Rng,
    ) -> ResourceResult<Vec<ResourceCandidate>> {
        let (min_q, max_q, min_r, max_r) = world_bounds;
        let world_area = ((max_q - min_q) * (max_r - min_r)) as f32;
        
        // Calculate number of deposits based on rarity
        let base_deposits = (world_area * 0.001) as u32; // 0.1% base coverage
        let num_deposits = ((base_deposits as f32) * (1.0 - resource_type.properties.rarity)) as u32;
        
        let mut candidates = Vec::new();
        
        for _ in 0..num_deposits {
            let q = rng.gen_range(min_q..=max_q);
            let r = rng.gen_range(min_r..=max_r);
            
            let position = HexCoord { q, r };
            let probability = rng.gen_range(0.3..1.0); // Random probability for scattered placement
            
            candidates.push(ResourceCandidate {
                position,
                probability,
                quality_modifier: rng.gen_range(0.7..1.3),
                quantity_modifier: rng.gen_range(0.8..1.2),
                geological_context: GeologicalContext {
                    elevation: 0.0,
                    plate_age: 100.0,
                    tectonic_features: vec![],
                    distance_to_boundary: 0.0,
                    volcanic_proximity: 0.0,
                },
            });
        }
        
        debug!("🎲 Generated {} scattered candidates for {}", candidates.len(), resource_type.name);
        Ok(candidates)
    }
}

/// Clustered distribution algorithm
struct ClusteredDistribution {
    rng: ChaCha8Rng,
}

impl ClusteredDistribution {
    fn new(seed: u64) -> Self {
        Self {
            rng: ChaCha8Rng::seed_from_u64(seed.wrapping_add(2)),
        }
    }
}

impl DistributionAlgorithm for ClusteredDistribution {
    fn generate_locations(
        &self,
        resource_type: &ResourceType,
        world_bounds: (i32, i32, i32, i32),
        _tectonic_data: &TectonicPlate,
        rng: &mut ChaCha8Rng,
    ) -> ResourceResult<Vec<ResourceCandidate>> {
        let clustering = &resource_type.distribution.clustering;
        let (min_q, max_q, min_r, max_r) = world_bounds;
        let world_area = ((max_q - min_q) * (max_r - min_r)) as f32;
        
        // Calculate number of cluster centers
        let cluster_density = 0.0001 * clustering.cluster_tendency;
        let num_clusters = ((world_area * cluster_density) as u32).max(1);
        
        let mut candidates = Vec::new();
        
        for _ in 0..num_clusters {
            // Generate cluster center
            let center_q = rng.gen_range(min_q..=max_q);
            let center_r = rng.gen_range(min_r..=max_r);
            
            // Generate deposits around center
            let cluster_size = rng.gen_range(1..=clustering.cluster_size);
            
            for _ in 0..cluster_size {
                // Random offset within cluster radius
                let angle = rng.gen_range(0.0..std::f32::consts::TAU);
                let radius = rng.gen_range(0.0..clustering.cluster_radius as f32);
                
                let offset_q = (radius * angle.cos()) as i32;
                let offset_r = (radius * angle.sin()) as i32;
                
                let q = (center_q + offset_q).clamp(min_q, max_q);
                let r = (center_r + offset_r).clamp(min_r, max_r);
                
                let position = HexCoord { q, r };
                let probability = clustering.cluster_tendency * rng.gen_range(0.5..1.0);
                
                candidates.push(ResourceCandidate {
                    position,
                    probability,
                    quality_modifier: rng.gen_range(0.8..1.2),
                    quantity_modifier: rng.gen_range(0.9..1.1),
                    geological_context: GeologicalContext {
                        elevation: 0.0,
                        plate_age: 100.0,
                        tectonic_features: vec![],
                        distance_to_boundary: 0.0,
                        volcanic_proximity: 0.0,
                    },
                });
            }
        }
        
        debug!("🏔️ Generated {} clustered candidates for {}", candidates.len(), resource_type.name);
        Ok(candidates)
    }
}

/// Linear vein distribution algorithm
struct VeinDistribution {
    rng: ChaCha8Rng,
}

impl VeinDistribution {
    fn new(seed: u64) -> Self {
        Self {
            rng: ChaCha8Rng::seed_from_u64(seed.wrapping_add(3)),
        }
    }
}

impl DistributionAlgorithm for VeinDistribution {
    fn generate_locations(
        &self,
        resource_type: &ResourceType,
        world_bounds: (i32, i32, i32, i32),
        _tectonic_data: &TectonicPlate,
        rng: &mut ChaCha8Rng,
    ) -> ResourceResult<Vec<ResourceCandidate>> {
        let (min_q, max_q, min_r, max_r) = world_bounds;
        let world_area = ((max_q - min_q) * (max_r - min_r)) as f32;
        
        // Calculate number of veins
        let vein_density = 0.00005 * (1.0 - resource_type.properties.rarity);
        let num_veins = ((world_area * vein_density) as u32).max(1);
        
        let mut candidates = Vec::new();
        
        for _ in 0..num_veins {
            // Generate vein starting point
            let start_q = rng.gen_range(min_q..=max_q);
            let start_r = rng.gen_range(min_r..=max_r);
            
            // Generate vein direction and length
            let direction = rng.gen_range(0.0..std::f32::consts::TAU);
            let length = rng.gen_range(5..20);
            let width = rng.gen_range(1..4);
            
            // Generate positions along the vein
            for i in 0..length {
                let progress = i as f32 / length as f32;
                let base_q = start_q + (direction.cos() * progress * length as f32) as i32;
                let base_r = start_r + (direction.sin() * progress * length as f32) as i32;
                
                // Add width variation
                for w in 0..width {
                    let offset = (w as f32 - width as f32 / 2.0) / width as f32;
                    let vein_q = base_q + (direction.sin() * offset * width as f32) as i32;
                    let vein_r = base_r - (direction.cos() * offset * width as f32) as i32;
                    
                    if vein_q >= min_q && vein_q <= max_q && vein_r >= min_r && vein_r <= max_r {
                        let position = HexCoord { q: vein_q, r: vein_r };
                        let probability = 0.8 * (1.0 - progress * 0.3); // Decrease probability with distance
                        
                        candidates.push(ResourceCandidate {
                            position,
                            probability,
                            quality_modifier: rng.gen_range(0.9..1.1),
                            quantity_modifier: rng.gen_range(0.8..1.2),
                            geological_context: GeologicalContext {
                                elevation: 0.0,
                                plate_age: 100.0,
                                tectonic_features: vec!["linear_structure".to_string()],
                                distance_to_boundary: 0.0,
                                volcanic_proximity: 0.0,
                            },
                        });
                    }
                }
            }
        }
        
        debug!("⚡ Generated {} vein candidates for {}", candidates.len(), resource_type.name);
        Ok(candidates)
    }
}

/// Geological feature-based distribution
struct GeologicalDistribution {
    rng: ChaCha8Rng,
}

impl GeologicalDistribution {
    fn new(seed: u64) -> Self {
        Self {
            rng: ChaCha8Rng::seed_from_u64(seed.wrapping_add(4)),
        }
    }
}

impl DistributionAlgorithm for GeologicalDistribution {
    fn generate_locations(
        &self,
        resource_type: &ResourceType,
        world_bounds: (i32, i32, i32, i32),
        tectonic_data: &TectonicPlate,
        rng: &mut ChaCha8Rng,
    ) -> ResourceResult<Vec<ResourceCandidate>> {
        let (min_q, max_q, min_r, max_r) = world_bounds;
        let mut candidates = Vec::new();
        
        // This would integrate with actual tectonic data
        // For now, generate based on simple geological rules
        
        for q in (min_q..=max_q).step_by(2) {
            for r in (min_r..=max_r).step_by(2) {
                let position = HexCoord { q, r };
                let context = GeologicalContext {
                    elevation: (q + r) as f32 * 10.0, // Placeholder
                    plate_age: 100.0,
                    tectonic_features: vec!["continental_crust".to_string()],
                    distance_to_boundary: 5.0,
                    volcanic_proximity: 20.0,
                };
                
                // Evaluate geological fitness
                let mut probability = 0.1; // Base probability
                
                // Elevation preferences
                if let Some((min_elev, max_elev)) = resource_type.distribution.geological_rules.elevation_range {
                    if context.elevation >= min_elev && context.elevation <= max_elev {
                        probability *= 2.0;
                    }
                }
                
                // Random geological sampling
                if rng.gen::<f32>() < probability * resource_type.properties.rarity {
                    candidates.push(ResourceCandidate {
                        position,
                        probability,
                        quality_modifier: rng.gen_range(0.7..1.3),
                        quantity_modifier: rng.gen_range(0.8..1.2),
                        geological_context: context,
                    });
                }
            }
        }
        
        debug!("🏔️ Generated {} geological candidates for {}", candidates.len(), resource_type.name);
        Ok(candidates)
    }
}
