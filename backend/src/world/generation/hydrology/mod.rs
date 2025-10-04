//! Hydrological Systems
//!
//! High-performance hydrological simulation using Zig backend for procedural world generation
//! including watersheds, rivers, lakes, wetlands, aquifers, and flooding.
//! Integrated with the core scheduler for coordinated parallel execution.

pub mod zig_ffi;
pub mod types;
pub mod flow;
pub mod watersheds;
pub mod rivers;
pub mod lakes;
pub mod wetlands;
pub mod flooding;
pub mod aquifers;
pub mod springs;

// Re-export public API
pub use types::*;
pub use zig_ffi::*;
pub use flow::*;
pub use watersheds::*;
pub use rivers::*;
pub use lakes::*;
pub use wetlands::*;
pub use flooding::*;
pub use aquifers::*;
pub use springs::*;

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use nalgebra::Vector2;
use std::sync::Arc;

use crate::core::{
    caching::GameCache,
    scheduler::{Scheduler, TaskBatch, SchedulerError},
};

/// Hydrological simulation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HydrologyConfig {
    /// World bounds for hydrological simulation
    pub world_bounds: (f64, f64, f64, f64), // (min_x, min_y, max_x, max_y)
    /// Grid resolution for flow calculations
    pub grid_resolution: u32,
    /// Minimum flow threshold for river formation
    pub river_threshold: f64,
    /// Lake minimum depth for formation
    pub lake_min_depth: f32,
    /// Wetland formation parameters
    pub wetland_threshold: f32,
    /// Aquifer depth range
    pub aquifer_depth_range: (f32, f32),
    /// Spring formation probability
    pub spring_probability: f64,
    /// Random seed for deterministic generation
    pub seed: u64,
}

impl Default for HydrologyConfig {
    fn default() -> Self {
        Self {
            world_bounds: (-1000.0, -1000.0, 1000.0, 1000.0),
            grid_resolution: 512,
            river_threshold: 10.0,
            lake_min_depth: 2.0,
            wetland_threshold: 0.8,
            aquifer_depth_range: (10.0, 100.0),
            spring_probability: 0.001,
            seed: 98765,
        }
    }
}

/// Comprehensive hydrological simulation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HydrologyResult {
    pub watersheds: Vec<Watershed>,
    pub rivers: Vec<River>,
    pub lakes: Vec<Lake>,
    pub wetlands: Vec<Wetland>,
    pub aquifers: Vec<Aquifer>,
    pub springs: Vec<Spring>,
    pub flow_accumulation: FlowAccumulation,
}

/// Main hydrological generation system
#[derive(Debug)]
pub struct HydrologyGenerator {
    config: HydrologyConfig,
    cache: GameCache,
    watershed_analyzer: watersheds::WatershedAnalyzer,
    river_generator: rivers::RiverGenerator,
    flow_calculator: flow::FlowCalculator,
    lake_detector: lakes::LakeDetector,
    wetland_generator: wetlands::WetlandGenerator,
    flood_simulator: flooding::FloodSimulator,
    aquifer_system: aquifers::AquiferSystem,
    spring_generator: springs::SpringGenerator,
}

impl HydrologyGenerator {
    pub fn new(config: HydrologyConfig, cache: GameCache) -> Self {
        Self {
            watershed_analyzer: watersheds::WatershedAnalyzer::new(&config),
            river_generator: rivers::RiverGenerator::new(&config),
            flow_calculator: flow::FlowCalculator::new(&config),
            lake_detector: lakes::LakeDetector::new(&config),
            wetland_generator: wetlands::WetlandGenerator::new(&config),
            flood_simulator: flooding::FloodSimulator::new(&config),
            aquifer_system: aquifers::AquiferSystem::new(&config),
            spring_generator: springs::SpringGenerator::new(&config),
            config,
            cache,
        }
    }
}

/// Main hydrological simulation system using Zig backend
#[derive(Debug, Resource)]
pub struct HydrologicalSystem {
    config: HydrologyConfig,
    cache: Arc<GameCache>,
}

impl HydrologicalSystem {
    pub fn new(config: HydrologyConfig, cache: Arc<GameCache>) -> Self {
        Self {
            config,
            cache,
        }
    }
    
    /// Generate complete hydrological features using Zig backend and scheduler integration
    pub fn generate_with_scheduler(
        &self,
        scheduler: &mut Scheduler,
        elevation_data: &[f32],
        world_size: (u32, u32),
    ) -> Result<HydrologyResult, SchedulerError> {
        // Convert to f64 for Zig processing
        let elevation_data_f64: Vec<f64> = elevation_data.iter().map(|&x| x as f64).collect();
        let config = self.config.clone();
        let results = Arc::new(std::sync::Mutex::new(None));
        let results_clone = results.clone();
        
        let mut batch = TaskBatch::new(crate::core::scheduler::Stage::WorldGeneration);
        batch.add_task("Hydrology Generation (Zig)", move || -> Result<(), SchedulerError> {
            let hydrology_result = Self::generate_hydrology_internal(&config, &elevation_data_f64, world_size)?;
            *results_clone.lock().unwrap() = Some(hydrology_result);
            Ok(())
        });
        
        scheduler.add_batch(batch);
        scheduler.run_stage(crate::core::scheduler::Stage::WorldGeneration).map_err(|errors| {
            errors.into_iter().next().unwrap_or_else(|| SchedulerError::TaskFailed("Unknown scheduler error".to_string()))
        })?;
        
        let final_result = results.lock().unwrap().take().ok_or_else(|| 
            SchedulerError::TaskFailed("Hydrology generation failed to produce results".to_string())
        );
        
        final_result
    }

    /// Direct generation using Zig backend
    pub fn generate_hydrology(
        &mut self,
        elevation_data: &[f32],
        world_size: (u32, u32),
    ) -> Result<HydrologyResult, SchedulerError> {
        let elevation_data_f64: Vec<f64> = elevation_data.iter().map(|&x| x as f64).collect();
        Self::generate_hydrology_internal(&self.config, &elevation_data_f64, world_size)
    }

    // Internal generation method using Zig FFI
    fn generate_hydrology_internal(
        config: &HydrologyConfig,
        elevation_data: &[f64],
        world_size: (u32, u32),
    ) -> Result<HydrologyResult, SchedulerError> {
        let (width, height) = world_size;
        let cell_size = (config.world_bounds.2 - config.world_bounds.0) / width as f64;

        // Create flow grid using Zig backend
        let mut flow_grid = FlowGrid::new(width as usize, height as usize, cell_size, elevation_data)
            .ok_or_else(|| SchedulerError::TaskFailed("Failed to create flow grid".to_string()))?;

        // Calculate flow directions and accumulation
        flow_grid.calculate_flow_directions();
        if !flow_grid.calculate_flow_accumulation() {
            return Err(SchedulerError::TaskFailed("Failed to calculate flow accumulation".to_string()));
        }

        // Generate watersheds from major outlets
        let watersheds = Self::generate_watersheds(&mut flow_grid, config)?;

        // Generate rivers based on flow accumulation
        let rivers = Self::generate_rivers(&flow_grid, config.river_threshold)?;

        // Generate lakes
        let lakes = Self::generate_lakes(elevation_data, world_size, config)?;

        // Generate aquifers and springs using Zig calculations
        let aquifers = Self::generate_aquifers(elevation_data, world_size, config)?;
        let springs = Self::generate_springs(&aquifers, elevation_data, world_size, config)?;

        // Generate wetlands
        let wetlands = Self::generate_wetlands(elevation_data, world_size, &lakes, config)?;

        // Create flow accumulation grid
        let flow_accumulation = FlowAccumulation::new(
            width,
            height,
            config.world_bounds,
        )?;

        Ok(HydrologyResult {
            watersheds,
            rivers,
            lakes,
            wetlands,
            aquifers,
            springs,
            flow_accumulation,
        })
    }

    fn generate_watersheds(
        flow_grid: &mut FlowGrid,
        config: &HydrologyConfig,
    ) -> Result<Vec<Watershed>, SchedulerError> {
        let mut watersheds = Vec::new();
        let mut watershed_id = 1u32;
        
        // Find major drainage points (simplified - use quarter points as potential outlets)
        let width = flow_grid.width();
        let height = flow_grid.height();
        let cell_size = flow_grid.cell_size();
        
        for y in (height / 8)..(7 * height / 8) {
            for x in (width / 8)..(7 * width / 8) {
                if let Some(watershed_result) = delineate_watershed(
                    flow_grid,
                    x,
                    y,
                    watershed_id,
                    1000, // Max boundary points
                ) {
                    if watershed_result.area > 1000.0 * cell_size * cell_size { // Minimum watershed area
                        let watershed = Watershed {
                            id: WatershedId(watershed_id),
                            outlet_position: Vector2::new(x as f64 * cell_size, y as f64 * cell_size),
                            boundary_points: watershed_result.boundary_points,
                            area: watershed_result.area,
                            perimeter: watershed_result.perimeter,
                            mean_elevation: watershed_result.mean_elevation,
                            relief: watershed_result.relief,
                            shape_factor: watershed_result.shape_factor,
                        };
                        watersheds.push(watershed);
                        watershed_id += 1;
                        
                        // Limit number of watersheds
                        if watersheds.len() >= 50 {
                            break;
                        }
                    }
                }
            }
            if watersheds.len() >= 50 {
                break;
            }
        }

        Ok(watersheds)
    }

    fn generate_rivers(
        flow_grid: &FlowGrid,
        river_threshold: f64,
    ) -> Result<Vec<River>, SchedulerError> {
        let mut rivers = Vec::new();
        let mut river_id = 1u32;
        
        // Simplified river generation - in practice would trace from flow accumulation peaks
        let width = flow_grid.width();
        let height = flow_grid.height();
        let cell_size = flow_grid.cell_size();
        
        for y in 0..height {
            for x in 0..width {
                // This is a placeholder - real implementation would trace flow paths
                if (x + y) % 100 == 0 && rivers.len() < 20 { // Simple pattern for demo
                    let start_pos = Vector2::new(x as f64 * cell_size, y as f64 * cell_size);
                    let end_pos = Vector2::new((x + 50) as f64 * cell_size, (y + 50) as f64 * cell_size);
                    
                    let river = River {
                        id: RiverId(river_id),
                        segments: vec![
                            RiverSegment {
                                position: start_pos,
                                width: 5.0,
                                depth: 1.0,
                                flow_rate: 2.0,
                                elevation: 100.0,
                            },
                            RiverSegment {
                                position: end_pos,
                                width: 8.0,
                                depth: 1.5,
                                flow_rate: 3.0,
                                elevation: 95.0,
                            },
                        ],
                        length: start_pos.metric_distance(&end_pos),
                        discharge: 2.5,
                    };
                    
                    rivers.push(river);
                    river_id += 1;
                }
            }
        }

        Ok(rivers)
    }

    fn generate_lakes(
        _elevation_data: &[f64],
        world_size: (u32, u32),
        config: &HydrologyConfig,
    ) -> Result<Vec<Lake>, SchedulerError> {
        let mut lakes = Vec::new();
        let (width, height) = world_size;
        let cell_size = (config.world_bounds.2 - config.world_bounds.0) / width as f64;
        
        // Simplified lake generation
        for i in 0..10 {
            let x = (width / 4 + i * width / 20) as f64 * cell_size;
            let y = (height / 4 + i * height / 20) as f64 * cell_size;
            let radius = 50.0 + i as f32 * 10.0;
            
            lakes.push(Lake {
                id: LakeId(i as u32 + 1),
                center: Vector2::new(x, y),
                radius,
                depth: config.lake_min_depth + i as f32,
                volume: std::f32::consts::PI * radius * radius * (config.lake_min_depth + i as f32),
                water_level: 100.0 + i as f32,
                surface_elevation: 100.0 + i as f32, // Same as water level initially
                drainage_rivers: Vec::new(), // Will be populated later
            });
        }

        Ok(lakes)
    }

    fn generate_aquifers(
        _elevation_data: &[f64],
        world_size: (u32, u32),
        config: &HydrologyConfig,
    ) -> Result<Vec<Aquifer>, SchedulerError> {
        let mut aquifers = Vec::new();
        let (width, height) = world_size;
        let cell_size = (config.world_bounds.2 - config.world_bounds.0) / width as f64;
        
        // Generate regional aquifers
        for i in 0..5 {
            let center_x = (width / 6 + i * width / 8) as f64 * cell_size;
            let center_y = (height / 6 + i * height / 8) as f64 * cell_size;
            let extent = 200.0 * cell_size;
            
            aquifers.push(Aquifer {
                id: AquiferId(i as u32 + 1),
                center: Vector2::new(center_x, center_y),
                extent,
                depth: config.aquifer_depth_range.0 + i as f32 * 10.0,
                permeability: 1e-5,
                porosity: 0.3,
                hydraulic_head: 50.0,
                water_table_elevation: 40.0 + i as f32 * 5.0, // Reasonable default
                recharge_rate: 0.1, // m/year, reasonable default
                boundary: vec![ // Simple circular boundary approximation
                    Vector2::new(center_x - extent/2.0, center_y),
                    Vector2::new(center_x, center_y + extent/2.0),
                    Vector2::new(center_x + extent/2.0, center_y),
                    Vector2::new(center_x, center_y - extent/2.0),
                ],
                aquifer_type: match i % 3 {
                    0 => AquiferType::Unconfined,
                    1 => AquiferType::Confined,
                    _ => AquiferType::FracturedRock,
                },
            });
        }

        Ok(aquifers)
    }

    fn generate_springs(
        aquifers: &[Aquifer],
        _elevation_data: &[f64],
        world_size: (u32, u32),
        config: &HydrologyConfig,
    ) -> Result<Vec<Spring>, SchedulerError> {
        let mut springs = Vec::new();
        let mut spring_id = 1u32;
        let cell_size = (config.world_bounds.2 - config.world_bounds.0) / world_size.0 as f64;

        for aquifer in aquifers {
            // Generate 1-3 springs per aquifer
            let num_springs = 1 + (aquifer.id.0 % 3);
            
            for i in 0..num_springs {
                let angle = (i as f64) * 2.0 * std::f64::consts::PI / num_springs as f64;
                let distance = aquifer.extent * 0.7; // Springs near aquifer edge
                
                let spring_x = aquifer.center.x + angle.cos() * distance;
                let spring_y = aquifer.center.y + angle.sin() * distance;
                
                let head_difference = aquifer.hydraulic_head - 30.0; // Spring elevation
                let discharge = calculate_spring_discharge(head_difference as f64, aquifer.aquifer_type);
                
                if discharge > 0.001 { // Minimum discharge threshold
                    springs.push(Spring {
                        id: SpringId(spring_id),
                        position: Vector2::new(spring_x, spring_y),
                        flow_rate: discharge as f32,
                        temperature: 15.0 + aquifer.depth * 0.025, // Geothermal gradient
                        aquifer_id: Some(aquifer.id),
                        mineral_content: 0.3, // Default mineral content
                        spring_type: SpringType::Gravity,
                    });
                    spring_id += 1;
                }
            }
        }

        Ok(springs)
    }

    fn generate_wetlands(
        _elevation_data: &[f64],
        world_size: (u32, u32),
        lakes: &[Lake],
        config: &HydrologyConfig,
    ) -> Result<Vec<Wetland>, SchedulerError> {
        let mut wetlands = Vec::new();
        let mut wetland_id = 1u32;

        // Generate wetlands around lakes
        for lake in lakes {
            let wetland_radius = lake.radius * 1.5;
            if wetland_radius > 10.0 { // Minimum wetland size
                wetlands.push(Wetland {
                    id: WetlandId(wetland_id),
                    center: lake.center,
                    radius: wetland_radius,
                    water_depth: 0.3, // Shallow water
                    vegetation_density: 0.8,
                    wetland_type: WetlandType::Marsh,
                    biodiversity_index: 0.7, // High biodiversity near lakes
                    seasonal_variation: 0.4, // Moderate seasonal changes
                });
                wetland_id += 1;
            }
        }

        // Generate additional independent wetlands
        let (width, height) = world_size;
        let cell_size = (config.world_bounds.2 - config.world_bounds.0) / width as f64;
        
        for i in 0..8 {
            let x = (width / 10 + i * width / 12) as f64 * cell_size;
            let y = (height / 10 + i * height / 12) as f64 * cell_size;
            
            wetlands.push(Wetland {
                id: WetlandId(wetland_id),
                center: Vector2::new(x, y),
                radius: 30.0 + i as f32 * 5.0,
                water_depth: 0.5,
                vegetation_density: 0.6,
                wetland_type: if i % 2 == 0 { WetlandType::Swamp } else { WetlandType::Bog },
                biodiversity_index: 0.5, // Moderate biodiversity
                seasonal_variation: 0.6, // Higher seasonal variation
            });
            wetland_id += 1;
        }

        Ok(wetlands)
    }

    /// Sample hydrological influence at coordinates
    pub fn sample_hydrological_influence(
        &self, 
        x: f64, 
        y: f64, 
        hydrology_result: &HydrologyResult
    ) -> HydrologicalInfluence {
        HydrologicalInfluence {
            distance_to_water: self.calculate_distance_to_water(x, y, hydrology_result),
            watershed_id: self.find_watershed_id(x, y, &hydrology_result.watersheds),
            flow_direction: self.calculate_flow_direction(x, y, &hydrology_result.flow_accumulation),
            moisture_level: self.calculate_moisture_level(x, y, hydrology_result),
            flood_risk: self.calculate_flood_risk(x, y, hydrology_result),
        }
    }

    /// Calculate distance to nearest water body
    fn calculate_distance_to_water(&self, x: f64, y: f64, hydrology: &HydrologyResult) -> f32 {
        let mut min_distance = f32::MAX;

        // Check rivers
        for river in &hydrology.rivers {
            for segment in &river.segments {
                let dx = x - segment.position.x;
                let dy = y - segment.position.y;
                let distance = (dx * dx + dy * dy).sqrt() as f32;
                min_distance = min_distance.min(distance);
            }
        }

        // Check lakes
        for lake in &hydrology.lakes {
            let dx = x - lake.center.x;
            let dy = y - lake.center.y;
            let distance = ((dx * dx + dy * dy).sqrt() - lake.radius as f64) as f32;
            min_distance = min_distance.min(distance.max(0.0));
        }

        min_distance
    }

    /// Find watershed ID for coordinates
    fn find_watershed_id(&self, x: f64, y: f64, watersheds: &[Watershed]) -> Option<u32> {
        watersheds.iter()
            .find(|w| w.contains_point(x, y))
            .map(|w| w.id.0)
    }

    /// Calculate flow direction at coordinates
    fn calculate_flow_direction(&self, _x: f64, _y: f64, flow: &FlowAccumulation) -> Vector2<f32> {
        flow.get_flow_direction(0.0, 0.0) // Placeholder
    }

    /// Calculate moisture level based on proximity to water
    fn calculate_moisture_level(&self, x: f64, y: f64, hydrology: &HydrologyResult) -> f32 {
        let water_distance = self.calculate_distance_to_water(x, y, hydrology);
        
        // Exponential decay with distance
        let base_moisture = (-water_distance / 100.0).exp();
        
        // Add wetland bonus
        let wetland_bonus = hydrology.wetlands.iter()
            .filter(|w| {
                let dx = x - w.center.x;
                let dy = y - w.center.y;
                (dx * dx + dy * dy).sqrt() < w.radius as f64
            })
            .map(|_| 0.3)
            .sum::<f32>();
        
        (base_moisture + wetland_bonus).min(1.0)
    }

    /// Calculate flood risk at coordinates
    fn calculate_flood_risk(&self, x: f64, y: f64, hydrology: &HydrologyResult) -> f32 {
        let flow_value = hydrology.flow_accumulation.get_flow_value(x, y);
        let water_distance = self.calculate_distance_to_water(x, y, hydrology);
        
        // Higher flow and closer to water = higher flood risk
        let flow_risk = (flow_value / 100.0).min(1.0);
        let distance_risk = (1.0 - (water_distance / 50.0)).max(0.0);
        
        flow_risk * distance_risk
    }
}

/// Hydrological influence at a specific location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HydrologicalInfluence {
    pub distance_to_water: f32,
    pub watershed_id: Option<u32>,
    pub flow_direction: Vector2<f32>,
    pub moisture_level: f32,
    pub flood_risk: f32,
}