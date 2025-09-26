//! Seismic Activity System
//!
//! Generates earthquake zones, fault systems, and seismic activity patterns
//! based on tectonic stress accumulation and release using realistic distributions.

use super::{TectonicsConfig, TectonicPlate, PlateBoundary, BoundaryType};
use serde::{Deserialize, Serialize};
use nalgebra::Vector2;
use ndarray::{Array2, s};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use rand_distr::{Distribution, Normal, Exp, Uniform, Gamma, LogNormal};
use rayon::prelude::*;
use crate::core::scheduler::SchedulerError;

/// Earthquake zone with seismic characteristics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EarthquakeZone {
    pub id: u32,
    pub name: String,
    pub zone_type: SeismicZoneType,
    pub boundary: Vec<Vector2<f64>>,
    pub seismic_map: SeismicMap,
    pub fault_network: Vec<ActiveFault>,
    pub historical_events: Vec<HistoricalEarthquake>,
    pub current_stress: f64,
    pub associated_boundary_id: Option<u32>,
}

/// Types of seismic zones
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SeismicZoneType {
    SubductionZone,    // Major plate subduction
    TransformFault,    // Transform plate boundary
    RiftZone,          // Divergent boundary
    IntratPlate,       // Within-plate seismicity
    VolcanicSeismic,   // Volcano-related earthquakes
    InducedSeismic,    // Human-induced seismicity
}

/// 2D seismic hazard and stress map
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeismicMap {
    pub width: usize,
    pub height: usize,
    pub resolution: f64, // km per cell
    pub stress_field: Array2<f64>,
    pub hazard_level: Array2<u8>, // 0-10 hazard scale
    pub b_value_map: Array2<f64>, // Gutenberg-Richter b-value
    pub max_magnitude: Array2<f64>,
}

/// Active fault within seismic zone
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveFault {
    pub id: u32,
    pub name: String,
    pub fault_type: FaultType,
    pub trace: Vec<Vector2<f64>>,
    pub length: f64,
    pub slip_rate: f64, // mm/year
    pub accumulated_slip: f64,
    pub locking_depth: f64,
    pub stress_level: f64,
    pub last_rupture_years_ago: f64,
    pub rupture_segments: Vec<FaultSegment>,
}

/// Types of faults
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum FaultType {
    Normal,      // Extension
    Reverse,     // Compression
    StrikeSlip,  // Lateral motion
    Oblique,     // Mixed motion
    Thrust,      // Low-angle reverse
}

/// Fault segment for rupture modeling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaultSegment {
    pub start_point: Vector2<f64>,
    pub end_point: Vector2<f64>,
    pub depth: f64,
    pub width: f64,
    pub stress_drop: f64,
    pub rupture_probability: f64,
}

/// Historical earthquake record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalEarthquake {
    pub magnitude: f64,
    pub location: Vector2<f64>,
    pub depth: f64,
    pub years_ago: f64,
    pub fault_id: Option<u32>,
    pub intensity_map: Option<Array2<f64>>, // Modified Mercalli intensity
}

/// Seismic system generator
#[derive(Debug, Clone)]
pub struct SeismicSystem {
    config: TectonicsConfig,
    rng: ChaCha8Rng,
}

impl SeismicSystem {
    /// Create new seismic system
    pub fn new(config: &TectonicsConfig) -> Self {
        let rng = ChaCha8Rng::seed_from_u64(config.seed + 4);
        Self {
            config: config.clone(),
            rng,
        }
    }

    /// Generate earthquake zones from plates and boundaries
    pub fn generate_earthquake_zones(
        &self,
        plates: &[TectonicPlate],
        boundaries: &[PlateBoundary],
    ) -> Result<Vec<EarthquakeZone>, SchedulerError> {
        // Generate zones from boundaries in parallel
        let boundary_zones: Result<Vec<_>, _> = boundaries
            .par_iter()
            .enumerate()
            .map(|(id, boundary)| {
                self.create_seismic_zone_from_boundary(id as u32, boundary, plates)
            })
            .collect();

        let mut zones = boundary_zones.map_err(|e| SchedulerError::TaskFailed(e.to_string()))?;

        // Add intraplate seismic zones
        let intraplate_zones = self.generate_intraplate_zones(plates)?;
        zones.extend(intraplate_zones);

        Ok(zones)
    }

    /// Create seismic zone from plate boundary
    fn create_seismic_zone_from_boundary(
        &self,
        id: u32,
        boundary: &PlateBoundary,
        plates: &[TectonicPlate],
    ) -> Result<EarthquakeZone, String> {
        let mut rng = self.rng.clone();

        // Determine seismic zone type
        let zone_type = match boundary.boundary_type {
            BoundaryType::Convergent => SeismicZoneType::SubductionZone,
            BoundaryType::Transform => SeismicZoneType::TransformFault,
            BoundaryType::Divergent => SeismicZoneType::RiftZone,
        };

        // Create expanded boundary for seismic zone
        let zone_boundary = self.expand_boundary_for_seismic_zone(&boundary.geometry, zone_type);

        // Generate seismic map
        let seismic_map = self.create_seismic_map(&zone_boundary, boundary, zone_type, &mut rng)?;

        // Generate fault network
        let fault_network = self.generate_fault_network(&zone_boundary, boundary, zone_type, &mut rng)?;

        // Generate historical earthquakes
        let historical_events = self.generate_historical_earthquakes(&fault_network, zone_type, &mut rng)?;

        // Calculate current stress
        let current_stress = self.calculate_current_stress(boundary, &fault_network);

        Ok(EarthquakeZone {
            id,
            name: format!("Seismic_Zone_{}", id),
            zone_type,
            boundary: zone_boundary,
            seismic_map,
            fault_network,
            historical_events,
            current_stress,
            associated_boundary_id: Some(boundary.id),
        })
    }

    /// Expand boundary to create seismic zone
    fn expand_boundary_for_seismic_zone(
        &self,
        boundary_geometry: &[Vector2<f64>],
        zone_type: SeismicZoneType,
    ) -> Vec<Vector2<f64>> {
        if boundary_geometry.is_empty() {
            return Vec::new();
        }

        let expansion_distance = match zone_type {
            SeismicZoneType::SubductionZone => 200.0, // Wide zone
            SeismicZoneType::TransformFault => 100.0, // Narrow zone
            SeismicZoneType::RiftZone => 150.0,       // Medium zone
            _ => 100.0,
        };

        let mut expanded_boundary = Vec::new();

        // Create polygon around the boundary line
        for i in 0..boundary_geometry.len() {
            let current = boundary_geometry[i];
            
            // Calculate perpendicular direction
            let direction = if i == boundary_geometry.len() - 1 {
                (current - boundary_geometry[i-1]).normalize()
            } else {
                (boundary_geometry[i+1] - current).normalize()
            };
            
            let perpendicular = Vector2::new(-direction.y, direction.x);
            
            // Add points on both sides
            expanded_boundary.push(current + perpendicular * expansion_distance);
            expanded_boundary.push(current - perpendicular * expansion_distance);
        }

        // Close the polygon
        if let Some(first) = expanded_boundary.first() {
            expanded_boundary.push(*first);
        }

        expanded_boundary
    }

    /// Create 2D seismic map for the zone
    fn create_seismic_map(
        &self,
        zone_boundary: &[Vector2<f64>],
        boundary: &PlateBoundary,
        zone_type: SeismicZoneType,
        rng: &mut ChaCha8Rng,
    ) -> Result<SeismicMap, String> {
        // Calculate map dimensions
        let min_x = zone_boundary.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
        let max_x = zone_boundary.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
        let min_y = zone_boundary.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
        let max_y = zone_boundary.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);

        let resolution = 5.0; // 5 km per cell
        let width = ((max_x - min_x) / resolution).ceil() as usize;
        let height = ((max_y - min_y) / resolution).ceil() as usize;

        if width == 0 || height == 0 {
            return Err("Invalid map dimensions".to_string());
        }

        // Initialize arrays
        let mut stress_field = Array2::zeros((height, width));
        let mut hazard_level = Array2::zeros((height, width));
        let mut b_value_map = Array2::zeros((height, width));
        let mut max_magnitude = Array2::zeros((height, width));

        // Fill the maps based on distance from plate boundary
        for (i, row) in stress_field.rows_mut().into_iter().enumerate() {
            for (j, stress_cell) in row.into_iter().enumerate() {
                let x = min_x + j as f64 * resolution;
                let y = min_y + i as f64 * resolution;
                let point = Vector2::new(x, y);

                // Calculate distance to nearest boundary point
                let min_distance = boundary.geometry.iter()
                    .map(|boundary_point| (point - boundary_point).magnitude())
                    .fold(f64::INFINITY, f64::min);

                // Stress decreases with distance
                let stress_value = self.calculate_stress_at_distance(min_distance, zone_type, boundary);
                *stress_cell = stress_value;

                // Hazard level (0-10 scale)
                let hazard = ((stress_value / 1e6).min(10.0).max(0.0)) as u8;
                hazard_level[[i, j]] = hazard;

                // B-value (Gutenberg-Richter relation parameter)
                let b_value = self.calculate_b_value(zone_type, stress_value, rng);
                b_value_map[[i, j]] = b_value;

                // Maximum expected magnitude
                let max_mag = self.calculate_max_magnitude(zone_type, stress_value, min_distance);
                max_magnitude[[i, j]] = max_mag;
            }
        }

        Ok(SeismicMap {
            width,
            height,
            resolution,
            stress_field,
            hazard_level,
            b_value_map,
            max_magnitude,
        })
    }

    /// Calculate stress at distance from boundary
    fn calculate_stress_at_distance(
        &self,
        distance: f64,
        zone_type: SeismicZoneType,
        boundary: &PlateBoundary,
    ) -> f64 {
        let max_stress = match zone_type {
            SeismicZoneType::SubductionZone => boundary.stress_magnitude * 2.0,
            SeismicZoneType::TransformFault => boundary.stress_magnitude * 1.5,
            SeismicZoneType::RiftZone => boundary.stress_magnitude * 0.8,
            _ => boundary.stress_magnitude,
        };

        let decay_distance = match zone_type {
            SeismicZoneType::SubductionZone => 150.0,
            SeismicZoneType::TransformFault => 75.0,
            SeismicZoneType::RiftZone => 100.0,
            _ => 100.0,
        };

        max_stress * (-distance / decay_distance).exp()
    }

    /// Calculate b-value for Gutenberg-Richter relation
    fn calculate_b_value(&self, zone_type: SeismicZoneType, stress: f64, rng: &mut ChaCha8Rng) -> f64 {
        let base_b_value = match zone_type {
            SeismicZoneType::SubductionZone => 0.9,  // Lower b-value (more large earthquakes)
            SeismicZoneType::TransformFault => 1.0,  // Standard b-value
            SeismicZoneType::RiftZone => 1.1,        // Higher b-value (more small earthquakes)
            SeismicZoneType::VolcanicSeismic => 1.3,  // Very high b-value
            _ => 1.0,
        };

        // Stress affects b-value
        let stress_factor = (stress / 1e6).min(2.0) * 0.1;
        let random_variation = rng.gen_range(-0.1..0.1);

        (base_b_value - stress_factor + random_variation).clamp(0.5, 2.0)
    }

    /// Calculate maximum expected magnitude
    fn calculate_max_magnitude(&self, zone_type: SeismicZoneType, stress: f64, distance: f64) -> f64 {
        let base_magnitude = match zone_type {
            SeismicZoneType::SubductionZone => 9.0,  // Can produce great earthquakes
            SeismicZoneType::TransformFault => 8.0,  // Large earthquakes possible
            SeismicZoneType::RiftZone => 7.0,        // Moderate earthquakes
            SeismicZoneType::IntratPlate => 7.5,     // Occasionally large
            _ => 6.5,
        };

        // Reduce with distance and increase with stress
        let distance_factor = (-distance / 200.0).exp(); // Decay over 200 km
        let stress_factor = (stress / 1e6).min(1.5);

        (base_magnitude * distance_factor * stress_factor).max(4.0).min(9.5)
    }

    /// Generate fault network within seismic zone
    fn generate_fault_network(
        &self,
        zone_boundary: &[Vector2<f64>],
        boundary: &PlateBoundary,
        zone_type: SeismicZoneType,
        rng: &mut ChaCha8Rng,
    ) -> Result<Vec<ActiveFault>, String> {
        let mut faults = Vec::new();

        // Number of faults based on zone type and size
        let zone_length = self.calculate_zone_length(zone_boundary);
        let fault_density = match zone_type {
            SeismicZoneType::SubductionZone => 0.1,  // One fault per 10 km
            SeismicZoneType::TransformFault => 0.05, // One fault per 20 km
            SeismicZoneType::RiftZone => 0.08,       // One fault per 12.5 km
            _ => 0.03,
        };

        let num_faults = (zone_length * fault_density * self.config.earthquake_frequency) as usize;
        let num_faults = num_faults.max(1).min(20);

        for i in 0..num_faults {
            let fault = self.generate_individual_fault(
                i as u32,
                zone_boundary,
                boundary,
                zone_type,
                rng,
            )?;
            faults.push(fault);
        }

        Ok(faults)
    }

    /// Calculate zone length for fault generation
    fn calculate_zone_length(&self, zone_boundary: &[Vector2<f64>]) -> f64 {
        if zone_boundary.len() < 2 {
            return 0.0;
        }

        zone_boundary.windows(2)
            .map(|pair| (pair[1] - pair[0]).magnitude())
            .sum()
    }

    /// Generate individual fault
    fn generate_individual_fault(
        &self,
        id: u32,
        zone_boundary: &[Vector2<f64>],
        boundary: &PlateBoundary,
        zone_type: SeismicZoneType,
        rng: &mut ChaCha8Rng,
    ) -> Result<ActiveFault, String> {
        // Determine fault type based on zone type
        let fault_type = self.determine_fault_type(zone_type, rng);

        // Generate fault trace
        let trace = self.generate_fault_trace(zone_boundary, fault_type, rng)?;
        if trace.is_empty() {
            return Err("Empty fault trace".to_string());
        }

        // Calculate fault length
        let length = trace.windows(2)
            .map(|pair| (pair[1] - pair[0]).magnitude())
            .sum();

        // Generate fault properties
        let slip_rate = self.generate_slip_rate(fault_type, boundary.relative_velocity, rng);
        let accumulated_slip = rng.gen_range(0.0..slip_rate * 100000.0); // Up to 100k years
        let locking_depth = self.generate_locking_depth(fault_type, rng);
        let stress_level = self.calculate_fault_stress(boundary, length);
        let last_rupture_years_ago = self.generate_last_rupture_time(fault_type, slip_rate, rng);

        // Generate rupture segments
        let rupture_segments = self.generate_rupture_segments(&trace, fault_type, rng)?;

        Ok(ActiveFault {
            id,
            name: format!("Fault_{}", id),
            fault_type,
            trace,
            length,
            slip_rate,
            accumulated_slip,
            locking_depth,
            stress_level,
            last_rupture_years_ago,
            rupture_segments,
        })
    }

    /// Determine fault type based on zone type
    fn determine_fault_type(&self, zone_type: SeismicZoneType, rng: &mut ChaCha8Rng) -> FaultType {
        match zone_type {
            SeismicZoneType::SubductionZone => {
                // Mix of thrust and reverse faults
                if rng.gen_bool(0.7) { FaultType::Thrust } else { FaultType::Reverse }
            }
            SeismicZoneType::TransformFault => {
                // Primarily strike-slip
                if rng.gen_bool(0.8) { FaultType::StrikeSlip } else { FaultType::Oblique }
            }
            SeismicZoneType::RiftZone => {
                // Primarily normal faults
                if rng.gen_bool(0.8) { FaultType::Normal } else { FaultType::Oblique }
            }
            _ => {
                // Mixed fault types
                match rng.gen_range(0..4) {
                    0 => FaultType::Normal,
                    1 => FaultType::Reverse,
                    2 => FaultType::StrikeSlip,
                    _ => FaultType::Oblique,
                }
            }
        }
    }

    /// Generate fault trace within zone
    fn generate_fault_trace(
        &self,
        zone_boundary: &[Vector2<f64>],
        fault_type: FaultType,
        rng: &mut ChaCha8Rng,
    ) -> Result<Vec<Vector2<f64>>, String> {
        if zone_boundary.is_empty() {
            return Ok(Vec::new());
        }

        // Calculate zone center and dimensions
        let center_x = zone_boundary.iter().map(|p| p.x).sum::<f64>() / zone_boundary.len() as f64;
        let center_y = zone_boundary.iter().map(|p| p.y).sum::<f64>() / zone_boundary.len() as f64;
        let center = Vector2::new(center_x, center_y);

        // Generate fault length
        let fault_length = match fault_type {
            FaultType::Thrust => rng.gen_range(50.0..200.0),
            FaultType::StrikeSlip => rng.gen_range(30.0..150.0),
            FaultType::Normal => rng.gen_range(20.0..100.0),
            FaultType::Reverse => rng.gen_range(25.0..120.0),
            FaultType::Oblique => rng.gen_range(25.0..100.0),
        };

        // Generate fault orientation
        let orientation = rng.gen_range(0.0..std::f64::consts::TAU);
        let direction = Vector2::new(orientation.cos(), orientation.sin());

        // Create fault trace as line segments
        let num_segments = (fault_length / 20.0f64).max(2.0) as usize;
        let mut trace = Vec::new();

        for i in 0..num_segments {
            let t = i as f64 / (num_segments - 1) as f64;
            let position = center + direction * (t - 0.5) * fault_length;
            
            // Add some random deviation for realistic fault geometry
            let deviation = Vector2::new(
                rng.gen_range(-10.0..10.0),
                rng.gen_range(-10.0..10.0),
            );
            
            trace.push(position + deviation);
        }

        Ok(trace)
    }

    /// Generate slip rate for fault
    fn generate_slip_rate(&self, fault_type: FaultType, boundary_velocity: f64, rng: &mut ChaCha8Rng) -> f64 {
        let base_rate = boundary_velocity * 1000.0; // Convert to mm/year
        
        let type_factor = match fault_type {
            FaultType::StrikeSlip => 1.0,    // Full relative motion
            FaultType::Thrust => 0.7,       // Partial accommodation
            FaultType::Reverse => 0.6,      // Partial accommodation
            FaultType::Normal => 0.5,       // Slower extension
            FaultType::Oblique => 0.8,      // Mixed motion
        };

        let random_factor = rng.gen_range(0.5..1.5);
        (base_rate * type_factor * random_factor).max(0.1).min(100.0)
    }

    /// Generate locking depth for fault
    fn generate_locking_depth(&self, fault_type: FaultType, rng: &mut ChaCha8Rng) -> f64 {
        let (mean_depth, std_dev) = match fault_type {
            FaultType::Thrust => (25.0, 8.0),      // Deep locking
            FaultType::StrikeSlip => (15.0, 5.0),  // Medium locking
            FaultType::Reverse => (20.0, 6.0),     // Medium-deep locking
            FaultType::Normal => (12.0, 4.0),      // Shallow locking
            FaultType::Oblique => (18.0, 6.0),     // Variable locking
        };

        let normal = Normal::new(mean_depth, std_dev).unwrap();
        (normal.sample(rng) as f64).max(5.0).min(50.0)
    }

    /// Calculate current stress on fault
    fn calculate_fault_stress(&self, boundary: &PlateBoundary, fault_length: f64) -> f64 {
        // Stress scales with boundary stress and fault length
        let base_stress = boundary.stress_magnitude;
        let length_factor = (fault_length / 100.0).min(2.0); // Longer faults accumulate more stress
        
        base_stress * length_factor
    }

    /// Generate time since last fault rupture
    fn generate_last_rupture_time(&self, fault_type: FaultType, slip_rate: f64, rng: &mut ChaCha8Rng) -> f64 {
        // Recurrence interval based on slip rate and typical displacement
        let typical_displacement = match fault_type {
            FaultType::Thrust => 5.0,      // 5m per event
            FaultType::StrikeSlip => 3.0,  // 3m per event
            FaultType::Reverse => 4.0,     // 4m per event
            FaultType::Normal => 2.0,      // 2m per event
            FaultType::Oblique => 3.5,     // 3.5m per event
        };

        let recurrence_interval = (typical_displacement * 1000.0) / slip_rate; // Years
        
        // Use exponential distribution for last rupture time
        let lambda = 1.0 / recurrence_interval;
        let exponential = Exp::new(lambda).unwrap();
        
        exponential.sample(rng).min(50000.0) // Max 50k years
    }

    /// Generate rupture segments for fault
    fn generate_rupture_segments(
        &self,
        trace: &[Vector2<f64>],
        fault_type: FaultType,
        rng: &mut ChaCha8Rng,
    ) -> Result<Vec<FaultSegment>, String> {
        if trace.len() < 2 {
            return Ok(Vec::new());
        }

        let mut segments = Vec::new();
        let segment_length = 25.0; // 25 km segments
        
        let mut current_distance = 0.0;
        let mut segment_start = 0;

        for i in 1..trace.len() {
            let distance = (trace[i] - trace[i-1]).magnitude();
            current_distance += distance;

            if current_distance >= segment_length || i == trace.len() - 1 {
                // Create segment
                let depth = self.generate_segment_depth(fault_type, rng);
                let width = self.generate_segment_width(fault_type, depth, rng);
                let stress_drop = self.generate_stress_drop(fault_type, rng);
                let rupture_probability = rng.gen_range(0.1..0.9);

                segments.push(FaultSegment {
                    start_point: trace[segment_start],
                    end_point: trace[i],
                    depth,
                    width,
                    stress_drop,
                    rupture_probability,
                });

                segment_start = i;
                current_distance = 0.0;
            }
        }

        Ok(segments)
    }

    /// Generate segment depth
    fn generate_segment_depth(&self, fault_type: FaultType, rng: &mut ChaCha8Rng) -> f64 {
        let (mean, std_dev) = match fault_type {
            FaultType::Thrust => (15.0, 5.0),
            FaultType::StrikeSlip => (10.0, 3.0),
            FaultType::Reverse => (12.0, 4.0),
            FaultType::Normal => (8.0, 2.0),
            FaultType::Oblique => (10.0, 4.0),
        };

        let normal = Normal::new(mean, std_dev).unwrap();
        (normal.sample(rng) as f64).max(1.0).min(30.0)
    }

    /// Generate segment width
    fn generate_segment_width(&self, fault_type: FaultType, depth: f64, rng: &mut ChaCha8Rng) -> f64 {
        let base_width = match fault_type {
            FaultType::Thrust => depth * 3.0,      // Wide thrust zones
            FaultType::StrikeSlip => depth * 1.5,  // Narrow strike-slip
            FaultType::Reverse => depth * 2.5,     // Medium reverse
            FaultType::Normal => depth * 2.0,      // Medium normal
            FaultType::Oblique => depth * 2.2,     // Mixed
        };

        let random_factor = rng.gen_range(0.7..1.3);
        (base_width * random_factor).max(5.0).min(100.0)
    }

    /// Generate stress drop for segment
    fn generate_stress_drop(&self, fault_type: FaultType, rng: &mut ChaCha8Rng) -> f64 {
        let (mean, std_dev): (f64, f64) = match fault_type {
            FaultType::Thrust => (5.0, 2.0),      // MPa
            FaultType::StrikeSlip => (3.0, 1.5),
            FaultType::Reverse => (4.0, 2.0),
            FaultType::Normal => (2.0, 1.0),
            FaultType::Oblique => (3.5, 1.8),
        };

        let log_normal = LogNormal::new(mean.ln(), std_dev / mean).unwrap();
        log_normal.sample(rng).max(0.5_f64).min(20.0_f64)
    }

    /// Generate historical earthquakes
    fn generate_historical_earthquakes(
        &self,
        fault_network: &[ActiveFault],
        zone_type: SeismicZoneType,
        rng: &mut ChaCha8Rng,
    ) -> Result<Vec<HistoricalEarthquake>, String> {
        let mut events = Vec::new();
        let time_window = 10000.0; // 10,000 years of history

        for fault in fault_network {
            // Generate events for this fault
            let recurrence_interval = fault.accumulated_slip / fault.slip_rate * 1000.0; // Years
            if recurrence_interval <= 0.0 {
                continue;
            }

            let num_events = (time_window / recurrence_interval) as usize;
            let num_events = num_events.min(20); // Limit to reasonable number

            for i in 0..num_events {
                let years_ago = rng.gen_range(0.0..time_window);
                let magnitude = self.generate_earthquake_magnitude(fault, zone_type, rng);
                let location = self.select_rupture_location(&fault.trace, rng);
                let depth = rng.gen_range(2.0..fault.locking_depth);

                events.push(HistoricalEarthquake {
                    magnitude,
                    location,
                    depth,
                    years_ago,
                    fault_id: Some(fault.id),
                    intensity_map: None, // Could be generated if needed
                });
            }
        }

        // Sort by time
        events.sort_by(|a, b| b.years_ago.partial_cmp(&a.years_ago).unwrap());

        Ok(events)
    }

    /// Generate earthquake magnitude for fault
    fn generate_earthquake_magnitude(&self, fault: &ActiveFault, zone_type: SeismicZoneType, rng: &mut ChaCha8Rng) -> f64 {
        // Use Wells & Coppersmith scaling relations
        let base_magnitude = 4.0 + (fault.length / 10.0).log10() * 1.5;
        
        let zone_modifier = match zone_type {
            SeismicZoneType::SubductionZone => 0.5,  // Can produce larger events
            SeismicZoneType::TransformFault => 0.0,  // Standard scaling
            SeismicZoneType::RiftZone => -0.3,       // Slightly smaller events
            _ => 0.0,
        };

        let random_variation = rng.gen_range(-0.3..0.3);
        (base_magnitude + zone_modifier + random_variation).max(4.0).min(9.5)
    }

    /// Select rupture location along fault
    fn select_rupture_location(&self, trace: &[Vector2<f64>], rng: &mut ChaCha8Rng) -> Vector2<f64> {
        if trace.is_empty() {
            return Vector2::new(0.0, 0.0);
        }

        if trace.len() == 1 {
            return trace[0];
        }

        let index = rng.gen_range(0..trace.len());
        trace[index]
    }

    /// Calculate current stress in zone
    fn calculate_current_stress(&self, boundary: &PlateBoundary, fault_network: &[ActiveFault]) -> f64 {
        let base_stress = boundary.stress_magnitude;
        let fault_stress_sum: f64 = fault_network.iter().map(|f| f.stress_level).sum();
        let average_fault_stress = if fault_network.is_empty() {
            0.0
        } else {
            fault_stress_sum / fault_network.len() as f64
        };

        (base_stress + average_fault_stress) / 2.0
    }

    /// Generate intraplate seismic zones
    fn generate_intraplate_zones(&self, plates: &[TectonicPlate]) -> Result<Vec<EarthquakeZone>, SchedulerError> {
        let mut zones = Vec::new();
        let mut rng = self.rng.clone();

        // Generate 1-3 intraplate zones per large plate
        for plate in plates.iter().filter(|p| p.area > 500000.0) { // Large plates only
            let zone_count = rng.gen_range(1..=3);
            
            for i in 0..zone_count {
                let zone_id = 2000 + plate.id * 10 + i; // Offset intraplate zone IDs
                
                // Generate random location within plate
                let zone_center = plate.center + Vector2::new(
                    rng.gen_range(-200.0..200.0),
                    rng.gen_range(-200.0..200.0),
                );

                let zone_radius = rng.gen_range(50.0..150.0);
                let boundary = vec![
                    zone_center + Vector2::new(-zone_radius, -zone_radius),
                    zone_center + Vector2::new(zone_radius, -zone_radius),
                    zone_center + Vector2::new(zone_radius, zone_radius),
                    zone_center + Vector2::new(-zone_radius, zone_radius),
                    zone_center + Vector2::new(-zone_radius, -zone_radius), // Close polygon
                ];

                // Simplified seismic map for intraplate zone
                let seismic_map = SeismicMap {
                    width: 20,
                    height: 20,
                    resolution: zone_radius * 2.0 / 20.0,
                    stress_field: Array2::from_elem((20, 20), plate.kinetic_energy() / 1e12),
                    hazard_level: Array2::from_elem((20, 20), 3), // Low-moderate hazard
                    b_value_map: Array2::from_elem((20, 20), 1.2), // High b-value
                    max_magnitude: Array2::from_elem((20, 20), 7.0), // Moderate max magnitude
                };

                zones.push(EarthquakeZone {
                    id: zone_id,
                    name: format!("Intraplate_Zone_{}", zone_id),
                    zone_type: SeismicZoneType::IntratPlate,
                    boundary,
                    seismic_map,
                    fault_network: Vec::new(), // Minimal fault network
                    historical_events: Vec::new(),
                    current_stress: plate.kinetic_energy() / 1e12,
                    associated_boundary_id: None,
                });
            }
        }

        Ok(zones)
    }
}
