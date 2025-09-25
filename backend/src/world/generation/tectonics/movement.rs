//! Tectonic Plate Movement System
//!
//! Handles plate motion vectors, velocity updates, and interactions between plates
//! using realistic physics and geological constraints.

use super::{TectonicsConfig, TectonicPlate, zig_ffi};
use serde::{Deserialize, Serialize};
use nalgebra::Vector2;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use rand_distr::{Distribution, Normal};
use rayon::prelude::*;
use crate::core::scheduler::SchedulerError;

/// Plate movement engine for updating velocities and positions
#[derive(Debug, Clone)]
pub struct MovementEngine {
    config: TectonicsConfig,
    rng: ChaCha8Rng,
}

impl MovementEngine {
    /// Create new movement engine
    pub fn new(config: &TectonicsConfig) -> Self {
        let rng = ChaCha8Rng::seed_from_u64(config.seed + 1);
        Self {
            config: config.clone(),
            rng,
        }
    }

    /// Update plate movement vectors based on forces and interactions
    pub fn update_plate_movement(&self, plates: &[TectonicPlate]) -> Result<Vec<TectonicPlate>, SchedulerError> {
        if plates.is_empty() {
            return Ok(Vec::new());
        }

        // Calculate forces on each plate in parallel
        let force_results: Result<Vec<_>, _> = plates
            .par_iter()
            .enumerate()
            .map(|(i, plate)| {
                self.calculate_forces_on_plate(plate, plates, i)
            })
            .collect();

        let forces = force_results.map_err(|e| SchedulerError::TaskFailed(e.to_string()))?;

        // Update velocities based on forces
        let updated_plates: Result<Vec<_>, _> = plates
            .par_iter()
            .zip(forces.par_iter())
            .map(|(plate, force)| {
                self.update_plate_velocity(plate, *force)
            })
            .collect();

        updated_plates.map_err(|e| SchedulerError::TaskFailed(e.to_string()))
    }

    /// Calculate net forces acting on a plate
    fn calculate_forces_on_plate(
        &self,
        plate: &TectonicPlate,
        all_plates: &[TectonicPlate],
        plate_index: usize,
    ) -> Result<Vector2<f64>, String> {
        let mut net_force = Vector2::new(0.0, 0.0);

        // 1. Ridge push force (from divergent boundaries)
        let ridge_push = self.calculate_ridge_push_force(plate);
        net_force += ridge_push;

        // 2. Slab pull force (from subduction zones)  
        let slab_pull = self.calculate_slab_pull_force(plate, all_plates, plate_index);
        net_force += slab_pull;

        // 3. Basal drag (resistance from mantle)
        let basal_drag = self.calculate_basal_drag(plate);
        net_force += basal_drag;

        // 4. Plate-plate interactions
        let interaction_force = self.calculate_interaction_forces(plate, all_plates, plate_index);
        net_force += interaction_force;

        // 5. Mantle convection forces
        let convection_force = self.calculate_mantle_convection_force(plate);
        net_force += convection_force;

        Ok(net_force)
    }

    /// Calculate ridge push force from mid-ocean ridges using Zig SIMD optimization
    fn calculate_ridge_push_force(&self, plate: &TectonicPlate) -> Vector2<f64> {
        // Use Zig SIMD-optimized calculation for better performance
        zig_ffi::calculate_ridge_push_zig(
            plate.center,
            plate.velocity,
            plate.age_million_years,
            plate.area,
            self.config.movement_speed,
        )
    }

    /// Calculate slab pull force from subducting edges
    fn calculate_slab_pull_force(&self, plate: &TectonicPlate, all_plates: &[TectonicPlate], plate_index: usize) -> Vector2<f64> {
        let mut slab_pull = Vector2::new(0.0, 0.0);

        // Check interactions with other plates to find subduction zones
        for (other_index, other_plate) in all_plates.iter().enumerate() {
            if other_index == plate_index {
                continue;
            }

            // Calculate if plates are converging
            let relative_velocity = plate.velocity - other_plate.velocity;
            let plate_separation = other_plate.center - plate.center;
            
            if plate_separation.magnitude() < 500.0 && relative_velocity.dot(&plate_separation.normalize()) < 0.0 {
                // Plates are converging - check for subduction
                if self.should_plate_subduct(plate, other_plate) {
                    // Slab pull acts toward the subduction zone
                    let pull_direction = plate_separation.normalize();
                    let pull_magnitude = 3.0e12 * self.config.movement_speed; // Strong force
                    slab_pull += pull_direction * pull_magnitude / plate.area.sqrt();
                }
            }
        }

        slab_pull
    }

    /// Determine if one plate should subduct beneath another
    fn should_plate_subduct(&self, plate: &TectonicPlate, other_plate: &TectonicPlate) -> bool {
        // Oceanic plates subduct beneath continental plates
        // Older/denser oceanic plates subduct beneath younger ones
        match (plate.plate_type, other_plate.plate_type) {
            (super::PlateType::Oceanic, super::PlateType::Continental) => true,
            (super::PlateType::Continental, super::PlateType::Oceanic) => false,
            (super::PlateType::Oceanic, super::PlateType::Oceanic) => {
                // Older/denser plate subducts
                plate.age_million_years > other_plate.age_million_years || 
                plate.density > other_plate.density
            }
            (super::PlateType::Continental, super::PlateType::Continental) => {
                // Continental collision - less likely to subduct, more likely to create mountains
                false
            }
            (super::PlateType::Mixed, _) | (_, super::PlateType::Mixed) => {
                // Mixed plates - decide based on density
                plate.density > other_plate.density
            }
        }
    }

    /// Calculate basal drag from mantle resistance using Zig SIMD optimization
    fn calculate_basal_drag(&self, plate: &TectonicPlate) -> Vector2<f64> {
        // Use Zig SIMD-optimized calculation for better performance
        zig_ffi::calculate_basal_drag_zig(plate.velocity, plate.area)
    }

    /// Calculate interaction forces between plates
    fn calculate_interaction_forces(&self, plate: &TectonicPlate, all_plates: &[TectonicPlate], plate_index: usize) -> Vector2<f64> {
        let mut interaction_force = Vector2::new(0.0, 0.0);

        for (other_index, other_plate) in all_plates.iter().enumerate() {
            if other_index == plate_index {
                continue;
            }

            let separation = other_plate.center - plate.center;
            let distance = separation.magnitude();

            if distance < 1000.0 { // Within interaction range
                let force_magnitude = self.calculate_interaction_strength(plate, other_plate, distance);
                let force_direction = separation.normalize();
                
                // Force is repulsive at short distances, attractive at medium distances
                let repulsion_threshold = 200.0;
                let final_force = if distance < repulsion_threshold {
                    // Repulsive force to prevent overlap
                    -force_direction * force_magnitude * (repulsion_threshold / distance).powi(2)
                } else {
                    // Weak attractive force at medium distances (boundary effects)
                    force_direction * force_magnitude * 0.1
                };

                interaction_force += final_force / plate.area.sqrt();
            }
        }

        interaction_force
    }

    /// Calculate strength of interaction between two plates
    fn calculate_interaction_strength(&self, plate1: &TectonicPlate, plate2: &TectonicPlate, distance: f64) -> f64 {
        // Interaction strength based on relative sizes and velocities
        let size_factor = (plate1.area * plate2.area).sqrt();
        let velocity_factor = (plate1.velocity - plate2.velocity).magnitude();
        
        let base_strength = 1e10;
        base_strength * size_factor.sqrt() * (1.0 + velocity_factor) / distance.powi(2)
    }

    /// Calculate forces from mantle convection using Zig SIMD optimization
    fn calculate_mantle_convection_force(&self, plate: &TectonicPlate) -> Vector2<f64> {
        // Use Zig SIMD-optimized calculation for better performance
        zig_ffi::calculate_mantle_convection_zig(
            plate.center,
            plate.area,
            self.config.movement_speed,
        )
    }

    /// Update plate velocity based on calculated forces
    fn update_plate_velocity(&self, plate: &TectonicPlate, net_force: Vector2<f64>) -> Result<TectonicPlate, String> {
        // Calculate acceleration F = ma
        let mass = plate.area * plate.density * 35000.0; // 35km average thickness
        let acceleration = net_force / mass;
        
        // Time step (1 million years in seconds)
        let dt = 1e6 * 365.25 * 24.0 * 3600.0;
        
        // Update velocity: v = v0 + a*dt
        let new_velocity = plate.velocity + acceleration * dt;
        
        // Apply realistic velocity constraints (plates don't move too fast)
        let max_velocity = 0.2; // 20 cm/year maximum
        let velocity_magnitude = new_velocity.magnitude();
        let constrained_velocity = if velocity_magnitude > max_velocity {
            new_velocity.normalize() * max_velocity
        } else {
            new_velocity
        };

        // Add small random perturbations for realistic behavior
        let mut rng = self.rng.clone();
        let velocity_noise = Normal::new(0.0, 0.001).unwrap();
        let noise_x = velocity_noise.sample(&mut rng);
        let noise_y = velocity_noise.sample(&mut rng);
        
        let final_velocity = constrained_velocity + Vector2::new(noise_x, noise_y);

        let mut updated_plate = plate.clone();
        updated_plate.velocity = final_velocity;
        
        // Update plate age (simplified aging)
        updated_plate.age_million_years += 1.0; // Add 1 million years per update
        updated_plate.age_million_years = updated_plate.age_million_years.min(self.config.max_plate_age_million_years);

        Ok(updated_plate)
    }
}

/// Represents forces acting on a tectonic plate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlateForces {
    pub ridge_push: Vector2<f64>,
    pub slab_pull: Vector2<f64>,
    pub basal_drag: Vector2<f64>,
    pub plate_interactions: Vector2<f64>,
    pub mantle_convection: Vector2<f64>,
    pub net_force: Vector2<f64>,
}

impl PlateForces {
    /// Calculate net force from all components
    pub fn calculate_net_force(&mut self) {
        self.net_force = self.ridge_push + self.slab_pull + self.basal_drag 
                       + self.plate_interactions + self.mantle_convection;
    }
}

/// Statistics about plate movement for analysis and debugging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovementStats {
    pub average_velocity: f64,
    pub max_velocity: f64,
    pub total_kinetic_energy: f64,
    pub convergent_pairs: u32,
    pub divergent_pairs: u32,
    pub transform_pairs: u32,
}
