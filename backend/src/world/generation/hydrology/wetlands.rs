//! Wetland Generation System
//!
//! Generates wetlands using spatial queries and ecological modeling.
//! Uses kdtree for efficient spatial queries and proximity analysis.

use super::{HydrologyConfig, Wetland, WetlandType, FlowAccumulation, Lake};
use super::zig_ffi::{ZigSpatialTree, zig_evaluate_wetland_candidates, ZigWetlandEvaluation};
use crate::core::scheduler::SchedulerError;
use nalgebra::Vector2;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// Wetland generation system
#[derive(Debug)]
pub struct WetlandGenerator {
    config: HydrologyConfig,
    rng: ChaCha8Rng,
}

impl WetlandGenerator {
    pub fn new(config: &HydrologyConfig) -> Self {
        Self {
            config: config.clone(),
            rng: ChaCha8Rng::seed_from_u64(config.seed + 1000), // Different seed offset
        }
    }

    /// Generate wetlands based on hydrology and terrain using Zig backend
    pub fn generate_wetlands(
        &mut self,
        elevation_data: &[f32],
        flow_accumulation: &FlowAccumulation,
        lakes: &[Lake],
        world_size: (u32, u32)
    ) -> Result<Vec<Wetland>, SchedulerError> {
        // Create spatial index for efficient queries using Zig backend
        let mut water_bodies_tree = self.create_water_bodies_index(lakes)
            .ok_or_else(|| SchedulerError::TaskFailed("Failed to create spatial tree".to_string()))?;
        
        // Find potential wetland locations using optimized Zig evaluation
        let candidates = self.find_wetland_candidates_zig(
            elevation_data,
            flow_accumulation,
            world_size,
            &mut water_bodies_tree
        )?;
        
        // Generate wetlands from candidates
        let wetlands = self.create_wetlands_from_candidates_zig(candidates, &water_bodies_tree);
        
        Ok(wetlands)
    }

    /// Create spatial index of water bodies for proximity queries using Zig backend
    fn create_water_bodies_index(&self, lakes: &[Lake]) -> Option<ZigSpatialTree> {
        let mut zig_tree = ZigSpatialTree::new()?;
        
        for (idx, lake) in lakes.iter().enumerate() {
            zig_tree.add_point(lake.center.x, lake.center.y, idx);
        }
        
        Some(zig_tree)
    }

    /// Find potential wetland locations using Zig backend for high-performance evaluation
    fn find_wetland_candidates_zig(
        &mut self,
        elevation_data: &[f32],
        flow_accumulation: &FlowAccumulation,
        world_size: (u32, u32),
        water_bodies_tree: &mut ZigSpatialTree
    ) -> Result<Vec<ZigWetlandCandidate>, SchedulerError> {
        let (width, height) = world_size;
        let mut positions = Vec::new();
        let mut elevations = Vec::new();
        let mut flow_values = Vec::new();
        
        // Sample points across the terrain
        let sample_step = 16; // Sample every 16th cell for performance
        
        for y in (0..height).step_by(sample_step) {
            for x in (0..width).step_by(sample_step) {
                let world_pos = self.grid_to_world(x as usize, y as usize);
                let cell_idx = (y * width + x) as usize;
                
                if cell_idx >= elevation_data.len() {
                    continue;
                }
                
                let elevation = elevation_data[cell_idx];
                let flow_value = flow_accumulation.get_flow_value(world_pos.x, world_pos.y);
                
                positions.push(world_pos);
                elevations.push(elevation);
                flow_values.push(flow_value);
            }
        }
        
        if positions.is_empty() {
            return Ok(Vec::new());
        }
        
        // Use Zig backend for batch evaluation of wetland candidates
        let zig_evaluation = zig_evaluate_wetland_candidates(
            &positions,
            &elevations,
            &flow_values,
            water_bodies_tree,
        );
        
        // Convert to candidate structures and filter by suitability
        let mut candidates: Vec<ZigWetlandCandidate> = positions
            .into_iter()
            .zip(elevations.into_iter())
            .zip(flow_values.into_iter())
            .zip(zig_evaluation.suitability_scores.into_iter())
            .zip(zig_evaluation.wetland_types.into_iter())
            .filter_map(|((((position, elevation), flow_value), suitability_score), wetland_type)| {
                // Filter by minimum suitability threshold
                if suitability_score > self.config.wetland_threshold {
                    Some(ZigWetlandCandidate {
                        position,
                        elevation,
                        flow_value,
                        wetland_type: WetlandType::from_u8(wetland_type),
                        suitability_score,
                    })
                } else {
                    None
                }
            })
            .collect();
        
        // Sort by suitability and limit number
        candidates.sort_by(|a, b| b.suitability_score.partial_cmp(&a.suitability_score).unwrap());
        candidates.truncate(200); // Limit for performance
        
        Ok(candidates)
    }

    /// Evaluate if a location is suitable for wetland formation
    fn evaluate_wetland_suitability(
        &self,
        position: Vector2<f64>,
        elevation: f32,
        flow_value: f32,
        water_bodies_kdtree: &KdTree<f64, usize, [f64; 2]>
    ) -> bool {
        // Low elevation areas
        let elevation_suitable = elevation < 50.0;
        
        // Moderate flow accumulation (not too high, not too low)
        let flow_suitable = flow_value > 5.0 && flow_value < 100.0;
        
        // Near water bodies but not too close
        let point = [position.x, position.y];
        let nearest_water = water_bodies_kdtree.nearest(&point, 1, &squared_euclidean).unwrap();
        let distance_to_water = if !nearest_water.is_empty() {
            nearest_water[0].0.sqrt()
        } else {
            f64::MAX
        };
        let proximity_suitable = distance_to_water > 50.0 && distance_to_water < 500.0;
        
        // Check wetland threshold from config
        let wetland_score = self.calculate_wetland_score(elevation, flow_value, distance_to_water as f32);
        let threshold_suitable = wetland_score > self.config.wetland_threshold;
        
        elevation_suitable && flow_suitable && proximity_suitable && threshold_suitable
    }

    /// Calculate wetland formation score
    fn calculate_wetland_score(&self, elevation: f32, flow_value: f32, distance_to_water: f32) -> f32 {
        let elevation_factor = (100.0 - elevation.max(0.0)).max(0.0) / 100.0;
        let flow_factor = (flow_value / 50.0).min(1.0);
        let proximity_factor = (1.0 - (distance_to_water / 1000.0)).max(0.0);
        
        (elevation_factor + flow_factor + proximity_factor) / 3.0
    }

    /// Determine the type of wetland based on environmental conditions
    fn determine_wetland_type(
        &self,
        position: Vector2<f64>,
        elevation: f32,
        flow_value: f32,
        water_bodies_kdtree: &KdTree<f64, usize, [f64; 2]>
    ) -> WetlandType {
        let point = [position.x, position.y];
        let distance_to_water = if let Ok(nearest) = water_bodies_kdtree.nearest(&point, 1, &squared_euclidean) {
            if !nearest.is_empty() { nearest[0].0.sqrt() } else { f64::MAX }
        } else {
            f64::MAX
        };

        match (elevation, flow_value, distance_to_water) {
            // Near rivers with high flow
            (_, flow, dist) if flow > 20.0 && dist < 100.0 => WetlandType::Delta,
            
            // Low elevation, moderate flow
            (elev, flow, _) if elev < 20.0 && flow > 10.0 => WetlandType::Marsh,
            
            // Higher elevation, woody areas (simulated)
            (elev, _, _) if elev > 30.0 => WetlandType::Swamp,
            
            // Nutrient-poor, acidic conditions (simulated by isolation)
            (_, _, dist) if dist > 300.0 => WetlandType::Bog,
            
            // Default to fen for other suitable areas
            _ => WetlandType::Fen,
        }
    }

    /// Calculate comprehensive suitability score
    fn calculate_suitability_score(
        &self,
        position: Vector2<f64>,
        elevation: f32,
        flow_value: f32,
        water_bodies_kdtree: &KdTree<f64, usize, [f64; 2]>
    ) -> f32 {
        let base_score = self.calculate_wetland_score(elevation, flow_value, 0.0);
        
        // Add randomness for natural variation
        let random_factor = self.rng.gen::<f32>() * 0.2 - 0.1; // ±0.1
        
        // Biodiversity bonus for certain locations
        let biodiversity_bonus = match position {
            pos if pos.x.abs() < 100.0 && pos.y.abs() < 100.0 => 0.1, // Central areas
            _ => 0.0,
        };
        
        base_score + random_factor + biodiversity_bonus
    }

    /// Create wetlands from Zig candidates with spatial distribution using Zig spatial tree
    fn create_wetlands_from_candidates_zig(
        &mut self,
        candidates: Vec<ZigWetlandCandidate>,
        water_bodies_tree: &ZigSpatialTree
    ) -> Vec<Wetland> {
        let mut wetlands = Vec::new();
        let mut wetland_tree = ZigSpatialTree::new().expect("Failed to create wetland spatial tree");
        let min_distance_between_wetlands = 100.0; // Minimum distance between wetlands
        
        for (wetland_id, candidate) in candidates.into_iter().enumerate() {
            // Check if too close to existing wetlands using Zig spatial tree
            let nearby_wetlands = wetland_tree.nearest(
                candidate.position.x,
                candidate.position.y,
                1
            );
            
            let too_close = nearby_wetlands.first()
                .map(|(_, distance)| distance.sqrt() < min_distance_between_wetlands)
                .unwrap_or(false);
            
            if too_close {
                continue;
            }
            
            // Calculate wetland properties
            let radius = self.calculate_wetland_radius_zig(&candidate);
            let biodiversity_index = self.calculate_biodiversity_index_zig(&candidate, water_bodies_tree);
            let seasonal_variation = self.calculate_seasonal_variation_zig(&candidate);
            
            let wetland = Wetland {
                id: wetland_id as u32,
                center: candidate.position,
                radius,
                wetland_type: candidate.wetland_type,
                biodiversity_index,
                seasonal_variation,
            };
            
            // Add to spatial index for future distance checks
            wetland_tree.add_point(candidate.position.x, candidate.position.y, wetland_id);
            wetlands.push(wetland);
        }
        
        wetlands
    }

    /// Calculate wetland radius for Zig candidates
    fn calculate_wetland_radius_zig(&self, candidate: &ZigWetlandCandidate) -> f32 {
        let base_radius = match candidate.wetland_type {
            WetlandType::Delta => 200.0,   // Large deltas
            WetlandType::Marsh => 100.0,   // Medium marshes
            WetlandType::Swamp => 150.0,   // Large swamps
            WetlandType::Bog => 80.0,      // Smaller bogs
            WetlandType::Fen => 60.0,      // Smaller fens
        };
        
        // Scale by suitability and flow
        let scale_factor = (candidate.suitability_score * candidate.flow_value / 20.0).min(2.0).max(0.5);
        base_radius * scale_factor
    }

    /// Calculate biodiversity index using Zig spatial tree
    fn calculate_biodiversity_index_zig(&self, candidate: &ZigWetlandCandidate, water_bodies_tree: &ZigSpatialTree) -> f32 {
        let nearby_water_bodies = water_bodies_tree.within_radius(
            candidate.position.x, 
            candidate.position.y, 
            500.0
        );
        
        let base_biodiversity = match candidate.wetland_type {
            WetlandType::Delta => 0.9,     // High biodiversity
            WetlandType::Marsh => 0.8,     // High biodiversity
            WetlandType::Swamp => 0.7,     // Good biodiversity
            WetlandType::Fen => 0.6,       // Moderate biodiversity
            WetlandType::Bog => 0.4,       // Lower biodiversity (specialized)
        };
        
        // Bonus for connectivity to other water bodies
        let connectivity_bonus = (nearby_water_bodies.len() as f32 * 0.05).min(0.2);
        
        (base_biodiversity + connectivity_bonus).min(1.0)
    }

    /// Calculate seasonal variation for Zig candidates
    fn calculate_seasonal_variation_zig(&self, candidate: &ZigWetlandCandidate) -> f32 {
        match candidate.wetland_type {
            WetlandType::Delta => 0.3,     // Moderate variation
            WetlandType::Marsh => 0.5,     // High variation
            WetlandType::Swamp => 0.2,     // Low variation (permanent)
            WetlandType::Fen => 0.4,       // Moderate-high variation
            WetlandType::Bog => 0.1,       // Very low variation
        }
    }

    /// Calculate wetland radius based on environmental conditions
    fn calculate_wetland_radius(&self, candidate: &WetlandCandidate) -> f32 {
        let base_radius = match candidate.wetland_type {
            WetlandType::Delta => 200.0,   // Large deltas
            WetlandType::Marsh => 100.0,   // Medium marshes
            WetlandType::Swamp => 150.0,   // Large swamps
            WetlandType::Bog => 80.0,      // Smaller bogs
            WetlandType::Fen => 60.0,      // Smaller fens
        };
        
        // Scale by suitability and flow
        let scale_factor = (candidate.suitability_score * candidate.flow_value / 20.0).min(2.0).max(0.5);
        base_radius * scale_factor
    }

    /// Calculate biodiversity index
    fn calculate_biodiversity_index(&self, candidate: &WetlandCandidate, water_bodies_kdtree: &KdTree<f64, usize, [f64; 2]>) -> f32 {
        let point = [candidate.position.x, candidate.position.y];
        let nearby_water_count = water_bodies_kdtree.within(&point, 500.0, &squared_euclidean).unwrap().len();
        
        let base_biodiversity = match candidate.wetland_type {
            WetlandType::Delta => 0.9,     // High biodiversity
            WetlandType::Marsh => 0.8,     // High biodiversity
            WetlandType::Swamp => 0.7,     // Good biodiversity
            WetlandType::Fen => 0.6,       // Moderate biodiversity
            WetlandType::Bog => 0.4,       // Lower biodiversity (specialized)
        };
        
        // Bonus for connectivity to other water bodies
        let connectivity_bonus = (nearby_water_count as f32 * 0.05).min(0.2);
        
        (base_biodiversity + connectivity_bonus).min(1.0)
    }

    /// Calculate seasonal variation
    fn calculate_seasonal_variation(&self, candidate: &WetlandCandidate) -> f32 {
        match candidate.wetland_type {
            WetlandType::Delta => 0.3,     // Moderate variation
            WetlandType::Marsh => 0.5,     // High variation
            WetlandType::Swamp => 0.2,     // Low variation (permanent)
            WetlandType::Fen => 0.4,       // Moderate-high variation
            WetlandType::Bog => 0.1,       // Very low variation
        }
    }

    /// Convert grid coordinates to world coordinates
    fn grid_to_world(&self, grid_x: usize, grid_y: usize) -> Vector2<f64> {
        let (min_x, min_y, max_x, max_y) = self.config.world_bounds;
        let x = min_x + (grid_x as f64 / self.config.grid_resolution as f64) * (max_x - min_x);
        let y = min_y + (grid_y as f64 / self.config.grid_resolution as f64) * (max_y - min_y);
        Vector2::new(x, y)
    }
}

/// Zig-based wetland candidate structure
#[derive(Debug, Clone)]
struct ZigWetlandCandidate {
    position: Vector2<f64>,
    elevation: f32,
    flow_value: f32,
    wetland_type: WetlandType,
    suitability_score: f32,
}

impl WetlandType {
    /// Convert u8 to WetlandType (matching Zig backend enum values)
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => WetlandType::Marsh,
            1 => WetlandType::Swamp,
            2 => WetlandType::Bog,
            3 => WetlandType::Fen,
            4 => WetlandType::Delta,
            _ => WetlandType::Marsh, // Default fallback
        }
    }
}
