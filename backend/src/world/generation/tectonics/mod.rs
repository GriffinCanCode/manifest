//! Tectonic Simulation System
//!
//! High-performance tectonic plate simulation with deterministic plate generation,
//! movement vectors, geological feature creation, and volcanic/seismic activity.
//! Integrated with the core scheduler for coordinated parallel execution.

pub mod plates;
pub mod movement;  
pub mod boundaries;
pub mod features;
pub mod volcanic;
pub mod seismic;
pub mod zig_ffi;
pub mod errors;
pub mod serialization;

// Re-export public API
pub use plates::*;
pub use movement::*;
pub use boundaries::*;
pub use features::*;
pub use volcanic::*;
pub use seismic::*;
pub use zig_ffi::*;
pub use errors::*;
pub use serialization::*;

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use nalgebra::Vector2;
use chrono::{DateTime, Utc};
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;
use std::sync::Arc;

use crate::core::{
    caching::GameCache,
    scheduler::{Scheduler, TaskBatch, Stage, Resource, SchedulerError},
};

/// Comprehensive tectonic simulation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TectonicsConfig {
    /// Number of tectonic plates to generate
    pub plate_count: u32,
    /// World bounds for plate generation
    pub world_bounds: (f64, f64, f64, f64), // (min_x, min_y, max_x, max_y)
    /// Plate movement speed multiplier
    pub movement_speed: f64,
    /// Age simulation parameters
    pub max_plate_age_million_years: f64,
    /// Volcanic activity intensity
    pub volcanic_intensity: f64,
    /// Earthquake frequency multiplier
    pub earthquake_frequency: f64,
    /// Random seed for deterministic generation
    pub seed: u64,
}

impl Default for TectonicsConfig {
    fn default() -> Self {
        Self {
            plate_count: 12,
            world_bounds: (-1000.0, -1000.0, 1000.0, 1000.0),
            movement_speed: 1.0,
            max_plate_age_million_years: 200.0,
            volcanic_intensity: 1.0,
            earthquake_frequency: 1.0,
            seed: 54321,
        }
    }
}

/// Complete tectonic simulation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TectonicResult {
    pub plates: Vec<TectonicPlate>,
    pub boundaries: Vec<PlateBoundary>,
    pub mountain_ranges: Vec<MountainRange>,
    pub rift_valleys: Vec<RiftValley>,
    pub transform_faults: Vec<TransformFault>,
    pub volcanic_zones: Vec<VolcanicZone>,
    pub earthquake_zones: Vec<EarthquakeZone>,
    pub generation_time: DateTime<Utc>,
}

/// Main tectonic simulation engine
#[derive(Debug, Resource)]
pub struct TectonicSimulator {
    config: TectonicsConfig,
    cache: GameCache,
    rng: ChaCha8Rng,
    
    // Component generators
    plate_generator: PlateGenerator,
    movement_engine: MovementEngine,
    boundary_detector: BoundaryDetector,
    feature_generator: FeatureGenerator,
    volcanic_system: VolcanicSystem,
    seismic_system: SeismicSystem,
}

impl TectonicSimulator {
    /// Create new tectonic simulator with configuration
    pub fn new(config: TectonicsConfig, cache: GameCache) -> Self {
        use rand::SeedableRng;
        
        let rng = ChaCha8Rng::seed_from_u64(config.seed);
        
        Self {
            config: config.clone(),
            cache: cache.clone(),
            rng,
            plate_generator: PlateGenerator::new(&config),
            movement_engine: MovementEngine::new(&config),
            boundary_detector: BoundaryDetector::new(&config),
            feature_generator: FeatureGenerator::new(&config),
            volcanic_system: VolcanicSystem::new(&config),
            seismic_system: SeismicSystem::new(&config),
        }
    }

    /// Generate complete tectonic simulation
    pub fn generate_tectonics(&mut self) -> Result<TectonicResult, SchedulerError> {
        let start_time = Utc::now();
        
        // Generate plates first
        let plates = self.plate_generator.generate_plates()?;
        
        // Calculate movement vectors
        let updated_plates = self.movement_engine.update_plate_movement(&plates)?;
        
        // Detect boundaries
        let boundaries = self.boundary_detector.detect_boundaries(&updated_plates)?;
        
        // Generate geological features
        let features = self.feature_generator.generate_features(&updated_plates, &boundaries)?;
        
        // Generate volcanic zones
        let volcanic_zones = self.volcanic_system.generate_volcanic_zones(&updated_plates, &boundaries)?;
        
        // Generate earthquake zones
        let earthquake_zones = self.seismic_system.generate_earthquake_zones(&updated_plates, &boundaries)?;
        
        Ok(TectonicResult {
            plates: updated_plates,
            boundaries,
            mountain_ranges: features.mountain_ranges,
            rift_valleys: features.rift_valleys,
            transform_faults: features.transform_faults,
            volcanic_zones,
            earthquake_zones,
            generation_time: start_time,
        })
    }

    /// Generate tectonics using the scheduler for coordinated execution
    pub fn generate_tectonics_scheduled(&mut self, scheduler: &Scheduler) -> Result<TectonicResult, SchedulerError> {
        let start_time = Utc::now();
        let result = Arc::new(std::sync::Mutex::new(None::<TectonicResult>));
        
        // Create coordinated task batch
        let mut batch = TaskBatch::new(Stage::WorldGeneration);
        
        let result_clone = Arc::clone(&result);
        let plate_generator = self.plate_generator.clone();
        let movement_engine = self.movement_engine.clone();
        let boundary_detector = self.boundary_detector.clone();
        let feature_generator = self.feature_generator.clone();
        let volcanic_system = self.volcanic_system.clone();
        let seismic_system = self.seismic_system.clone();
        
        batch.add_task_with_resources(
            "generate_complete_tectonics".to_string(),
            vec![Resource::write::<TectonicResult>()],
            move || -> Result<(), SchedulerError> {
                // Generate plates
                let plates = plate_generator.generate_plates()?;
                
                // Update movement in parallel
                let updated_plates = movement_engine.update_plate_movement(&plates)?;
                
                // Detect boundaries
                let boundaries = boundary_detector.detect_boundaries(&updated_plates)?;
                
                // Generate features in parallel
                let features = feature_generator.generate_features(&updated_plates, &boundaries)?;
                let volcanic_zones = volcanic_system.generate_volcanic_zones(&updated_plates, &boundaries)?;
                let earthquake_zones = seismic_system.generate_earthquake_zones(&updated_plates, &boundaries)?;
                
                let tectonic_result = TectonicResult {
                    plates: updated_plates,
                    boundaries,
                    mountain_ranges: features.mountain_ranges,
                    rift_valleys: features.rift_valleys,  
                    transform_faults: features.transform_faults,
                    volcanic_zones,
                    earthquake_zones,
                    generation_time: start_time,
                };
                
                *result_clone.lock().unwrap() = Some(tectonic_result);
                Ok(())
            },
        );
        
        scheduler.add_batch(batch);
        scheduler.run_stage(Stage::WorldGeneration).map_err(|errors| {
            errors.into_iter().next().unwrap_or_else(|| SchedulerError::TaskFailed("Unknown scheduler error".to_string()))
        })?;
        
        let final_result = result.lock().unwrap().take().ok_or_else(|| {
            SchedulerError::TaskFailed("Failed to generate tectonic result".to_string())
        });
        
        final_result
    }

    /// Sample tectonic influence at specific coordinates
    pub fn sample_tectonic_influence(&self, x: f64, y: f64, result: &TectonicResult) -> TectonicInfluence {
        // Find the plate this point belongs to
        let plate = result.plates.iter()
            .find(|p| p.contains_point(x, y))
            .cloned()
            .unwrap_or_default();
            
        // Calculate distances to various features
        let mut influence = TectonicInfluence {
            plate_id: plate.id,
            plate_age: plate.age_million_years,
            elevation_modifier: 0.0,
            volcanic_activity: 0.0,
            seismic_activity: 0.0,
            mountain_proximity: 1.0,
            rift_proximity: 1.0,
        };
        
        // Calculate influence from boundaries and features
        for boundary in &result.boundaries {
            let distance = boundary.distance_to_point(x, y);
            if distance < 100.0 { // Within influence range
                match boundary.boundary_type {
                    BoundaryType::Convergent => {
                        influence.elevation_modifier += (100.0 - distance) / 100.0 * 2000.0; // Mountains
                        influence.seismic_activity += (100.0 - distance) / 100.0 * 0.8;
                    }
                    BoundaryType::Divergent => {
                        influence.elevation_modifier -= (100.0 - distance) / 100.0 * 500.0; // Rift valleys
                        influence.volcanic_activity += (100.0 - distance) / 100.0 * 0.9;
                    }
                    BoundaryType::Transform => {
                        influence.seismic_activity += (100.0 - distance) / 100.0 * 0.7;
                    }
                }
            }
        }
        
        influence
    }
}

/// Tectonic influence at a specific point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TectonicInfluence {
    pub plate_id: u32,
    pub plate_age: f64,
    pub elevation_modifier: f64,
    pub volcanic_activity: f64,
    pub seismic_activity: f64,
    pub mountain_proximity: f64,
    pub rift_proximity: f64,
}

/// Stage for world generation tasks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorldGenerationStage {
    TerrainGeneration,
    TectonicSimulation,
    ClimateGeneration,
    BiomeAssignment,
}

impl From<WorldGenerationStage> for Stage {
    fn from(stage: WorldGenerationStage) -> Self {
        match stage {
            WorldGenerationStage::TerrainGeneration => Stage::WorldGeneration,
            WorldGenerationStage::TectonicSimulation => Stage::WorldGeneration, 
            WorldGenerationStage::ClimateGeneration => Stage::WorldGeneration,
            WorldGenerationStage::BiomeAssignment => Stage::WorldGeneration,
        }
    }
}
