//! Geological Feature Generation
//!
//! Creates mountain ranges, rift valleys, and transform faults based on
//! tectonic plate interactions using line drawing algorithms and spatial analysis.

use super::{TectonicsConfig, TectonicPlate, PlateBoundary, BoundaryType};
use serde::{Deserialize, Serialize};
use nalgebra::Vector2;
use line_drawing::{Bresenham, Point};
use spade::{Triangulation as SpadeTriangulation, Point2};
use rstar::Point as RStarPoint;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use rand_distr::{Distribution, Normal, Uniform};
use rayon::prelude::*;
use geo::{Coord, Polygon, Point as GeoPoint};
use crate::core::scheduler::SchedulerError;

/// Mountain range created by convergent boundaries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountainRange {
    pub id: u32,
    pub name: String,
    pub boundary_id: u32,
    pub spine: Vec<Vector2<f64>>, // Central ridge line
    pub peaks: Vec<MountainPeak>,
    pub max_elevation: f64,
    pub average_elevation: f64,
    pub age_million_years: f64,
    pub range_type: MountainType,
    pub width: f64, // Average width perpendicular to spine
}

/// Individual mountain peak
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountainPeak {
    pub position: Vector2<f64>,
    pub elevation: f64,
    pub prominence: f64, // Height above surrounding terrain
    pub peak_type: PeakType,
}

/// Types of mountain ranges based on formation
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum MountainType {
    Fold,          // Folded mountain belts
    FaultBlock,    // Block mountains
    Volcanic,      // Volcanic mountain chains
    Complex,       // Multiple formation processes
}

/// Types of peaks
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PeakType {
    Summit,        // Highest point
    Secondary,     // Major secondary peak
    Ridge,         // Ridge point
    Volcanic,      // Volcanic peak
}

/// Rift valley created by divergent boundaries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiftValley {
    pub id: u32,
    pub name: String,
    pub boundary_id: u32,
    pub center_line: Vec<Vector2<f64>>,
    pub width: f64,
    pub depth: f64,
    pub escarpments: Vec<Vec<Vector2<f64>>>, // Cliff faces on each side
    pub age_million_years: f64,
    pub spreading_rate: f64, // mm/year
}

/// Transform fault created by transform boundaries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformFault {
    pub id: u32,
    pub name: String,
    pub boundary_id: u32,
    pub trace: Vec<Vector2<f64>>, // Fault line
    pub displacement: f64, // Total lateral displacement
    pub fault_zones: Vec<FaultZone>,
    pub age_million_years: f64,
    pub slip_rate: f64, // mm/year
}

/// Fault zone segment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaultZone {
    pub segment: Vec<Vector2<f64>>,
    pub width: f64,
    pub recent_activity: f64, // Years since last major movement
}

/// Complete geological feature set
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeologicalFeatures {
    pub mountain_ranges: Vec<MountainRange>,
    pub rift_valleys: Vec<RiftValley>,
    pub transform_faults: Vec<TransformFault>,
}

/// Feature generation engine
#[derive(Debug, Clone)]
pub struct FeatureGenerator {
    config: TectonicsConfig,
    rng: ChaCha8Rng,
}

impl FeatureGenerator {
    /// Create new feature generator
    pub fn new(config: &TectonicsConfig) -> Self {
        let rng = ChaCha8Rng::seed_from_u64(config.seed + 2);
        Self {
            config: config.clone(),
            rng,
        }
    }

    /// Generate all geological features from plates and boundaries
    pub fn generate_features(
        &self,
        plates: &[TectonicPlate],
        boundaries: &[PlateBoundary],
    ) -> Result<GeologicalFeatures, SchedulerError> {
        
        // Generate features in parallel by type
        let mountain_ranges = self.generate_mountain_ranges(plates, boundaries)?;
        let rift_valleys = self.generate_rift_valleys(plates, boundaries)?;
        let transform_faults = self.generate_transform_faults(plates, boundaries)?;

        Ok(GeologicalFeatures {
            mountain_ranges,
            rift_valleys,
            transform_faults,
        })
    }

    /// Generate mountain ranges from convergent boundaries
    fn generate_mountain_ranges(
        &self,
        plates: &[TectonicPlate],
        boundaries: &[PlateBoundary],
    ) -> Result<Vec<MountainRange>, SchedulerError> {
        let convergent_boundaries: Vec<_> = boundaries
            .iter()
            .filter(|b| b.boundary_type == BoundaryType::Convergent)
            .collect();

        let mountain_ranges: Result<Vec<_>, _> = convergent_boundaries
            .par_iter()
            .enumerate()
            .map(|(id, boundary)| {
                self.create_mountain_range_from_boundary(id as u32, boundary, plates)
            })
            .collect();

        mountain_ranges.map_err(|e| SchedulerError::TaskFailed(e.to_string()))
    }

    /// Create mountain range from convergent boundary
    fn create_mountain_range_from_boundary(
        &self,
        id: u32,
        boundary: &PlateBoundary,
        plates: &[TectonicPlate],
    ) -> Result<MountainRange, String> {
        let mut rng = self.rng.clone();
        
        // Find the plates involved
        let plate1 = plates.iter().find(|p| p.id == boundary.plate1_id)
            .ok_or("Plate1 not found")?;
        let plate2 = plates.iter().find(|p| p.id == boundary.plate2_id)
            .ok_or("Plate2 not found")?;

        // Determine mountain type based on plate types
        let range_type = self.determine_mountain_type(plate1, plate2, boundary);

        // Create spine from boundary geometry using line drawing
        let spine = self.create_mountain_spine(&boundary.geometry)?;

        // Generate peaks along the spine
        let peaks = self.generate_mountain_peaks(&spine, boundary, range_type, &mut rng)?;

        // Calculate elevations based on collision energy and plate properties
        let max_elevation = self.calculate_max_elevation(plate1, plate2, boundary);
        let average_elevation = max_elevation * 0.6;

        // Estimate age from plate data
        let age_million_years = (plate1.age_million_years + plate2.age_million_years) / 4.0;

        // Calculate average width
        let width = self.calculate_mountain_width(boundary, range_type);

        Ok(MountainRange {
            id,
            name: format!("Range_{}", id),
            boundary_id: boundary.id,
            spine,
            peaks,
            max_elevation,
            average_elevation,
            age_million_years,
            range_type,
            width,
        })
    }

    /// Determine mountain type based on colliding plates
    fn determine_mountain_type(&self, plate1: &TectonicPlate, plate2: &TectonicPlate, boundary: &PlateBoundary) -> MountainType {
        use super::PlateType;
        
        match (plate1.plate_type, plate2.plate_type) {
            (PlateType::Continental, PlateType::Continental) => {
                // Continental collision creates complex fold mountains
                MountainType::Complex
            }
            (PlateType::Oceanic, PlateType::Continental) | (PlateType::Continental, PlateType::Oceanic) => {
                // Oceanic-continental creates volcanic mountains
                MountainType::Volcanic
            }
            (PlateType::Oceanic, PlateType::Oceanic) => {
                // Oceanic-oceanic creates volcanic island arcs
                MountainType::Volcanic
            }
            _ => {
                // Mixed plates - determine by stress
                if boundary.stress_magnitude > 1e6 {
                    MountainType::FaultBlock
                } else {
                    MountainType::Fold
                }
            }
        }
    }

    /// Create mountain spine using line drawing algorithms
    fn create_mountain_spine(&self, boundary_geometry: &[Vector2<f64>]) -> Result<Vec<Vector2<f64>>, String> {
        if boundary_geometry.len() < 2 {
            return Err("Not enough boundary points".to_string());
        }

        let mut spine = Vec::new();

        // Use Bresenham's line algorithm to create smooth spine
        for i in 0..boundary_geometry.len() - 1 {
            let start = boundary_geometry[i];
            let end = boundary_geometry[i + 1];
            
            let start_point = (start.x as i32, start.y as i32);
            let end_point = (end.x as i32, end.y as i32);
            
            for point in Bresenham::new(start_point, end_point) {
                spine.push(Vector2::new(point.0 as f64, point.1 as f64));
            }
        }

        // Remove duplicates and smooth the line
        spine.dedup_by(|a, b| (*a - *b).magnitude() < 10.0);
        
        Ok(self.smooth_line(&spine, 3))
    }

    /// Smooth a line using moving average
    fn smooth_line(&self, points: &[Vector2<f64>], window_size: usize) -> Vec<Vector2<f64>> {
        if points.len() <= window_size {
            return points.to_vec();
        }

        let mut smoothed = Vec::new();
        let half_window = window_size / 2;

        for i in 0..points.len() {
            let start = i.saturating_sub(half_window);
            let end = (i + half_window + 1).min(points.len());
            
            let avg_x = points[start..end].iter().map(|p| p.x).sum::<f64>() / (end - start) as f64;
            let avg_y = points[start..end].iter().map(|p| p.y).sum::<f64>() / (end - start) as f64;
            
            smoothed.push(Vector2::new(avg_x, avg_y));
        }

        smoothed
    }

    /// Generate mountain peaks along the spine
    fn generate_mountain_peaks(
        &self,
        spine: &[Vector2<f64>],
        boundary: &PlateBoundary,
        range_type: MountainType,
        rng: &mut ChaCha8Rng,
    ) -> Result<Vec<MountainPeak>, String> {
        let mut peaks = Vec::new();
        
        let peak_spacing = match range_type {
            MountainType::Fold => 20.0,      // Dense peaks
            MountainType::FaultBlock => 40.0, // Sparse peaks
            MountainType::Volcanic => 60.0,   // Very sparse
            MountainType::Complex => 30.0,    // Medium density
        };

        let max_elevation = self.calculate_max_elevation_simple(boundary);
        
        // Place peaks along spine
        let mut distance_along_spine = 0.0;
        let mut next_peak_distance = rng.gen_range(peak_spacing * 0.5..peak_spacing * 1.5);

        for i in 1..spine.len() {
            let segment_length = (spine[i] - spine[i-1]).magnitude();
            distance_along_spine += segment_length;

            if distance_along_spine >= next_peak_distance {
                let elevation = self.generate_peak_elevation(max_elevation, range_type, rng);
                let prominence = elevation * rng.gen_range(0.3..0.8);
                
                let peak_type = if elevation > max_elevation * 0.95 {
                    PeakType::Summit
                } else if elevation > max_elevation * 0.7 {
                    PeakType::Secondary
                } else if matches!(range_type, MountainType::Volcanic) && rng.gen_bool(0.3) {
                    PeakType::Volcanic
                } else {
                    PeakType::Ridge
                };

                peaks.push(MountainPeak {
                    position: spine[i],
                    elevation,
                    prominence,
                    peak_type,
                });

                next_peak_distance = distance_along_spine + rng.gen_range(peak_spacing * 0.7..peak_spacing * 1.3);
            }
        }

        Ok(peaks)
    }

    /// Generate peak elevation with realistic distribution
    fn generate_peak_elevation(&self, max_elevation: f64, range_type: MountainType, rng: &mut ChaCha8Rng) -> f64 {
        let base_elevation = max_elevation * 0.6;
        let variation = max_elevation * 0.4;
        
        let distribution = match range_type {
            MountainType::Fold => Normal::new(0.7, 0.2).unwrap(),
            MountainType::FaultBlock => Normal::new(0.6, 0.3).unwrap(),
            MountainType::Volcanic => Normal::new(0.8, 0.15).unwrap(),
            MountainType::Complex => Normal::new(0.75, 0.25).unwrap(),
        };
        
        let factor: f64 = distribution.sample(rng).clamp(0.2, 1.0);
        base_elevation + variation * factor
    }

    /// Calculate maximum elevation for mountain range
    fn calculate_max_elevation(&self, plate1: &TectonicPlate, plate2: &TectonicPlate, boundary: &PlateBoundary) -> f64 {
        // Base elevation on collision energy and stress
        let kinetic_energy = plate1.kinetic_energy() + plate2.kinetic_energy();
        let base_height = (kinetic_energy.log10() * 1000.0).max(1000.0).min(9000.0);
        
        // Modify by stress magnitude
        let stress_factor = (boundary.stress_magnitude / 1e6).min(3.0);
        
        base_height * stress_factor
    }

    /// Simple elevation calculation
    fn calculate_max_elevation_simple(&self, boundary: &PlateBoundary) -> f64 {
        // Base elevation on boundary properties
        let base = 2000.0;
        let stress_factor = (boundary.stress_magnitude / 1e6).min(4.0);
        let velocity_factor = (boundary.relative_velocity * 10.0).min(2.0);
        
        base * stress_factor * (1.0 + velocity_factor)
    }

    /// Calculate mountain range width
    fn calculate_mountain_width(&self, boundary: &PlateBoundary, range_type: MountainType) -> f64 {
        let base_width = match range_type {
            MountainType::Fold => 150.0,      // Wide folded ranges
            MountainType::FaultBlock => 80.0, // Narrow block mountains
            MountainType::Volcanic => 60.0,   // Narrow volcanic chains
            MountainType::Complex => 200.0,   // Very wide complex ranges
        };
        
        let stress_factor = (boundary.stress_magnitude / 1e6).min(2.0);
        base_width * stress_factor
    }

    /// Generate rift valleys from divergent boundaries
    fn generate_rift_valleys(
        &self,
        plates: &[TectonicPlate],
        boundaries: &[PlateBoundary],
    ) -> Result<Vec<RiftValley>, SchedulerError> {
        let divergent_boundaries: Vec<_> = boundaries
            .iter()
            .filter(|b| b.boundary_type == BoundaryType::Divergent)
            .collect();

        let rift_valleys: Result<Vec<_>, _> = divergent_boundaries
            .par_iter()
            .enumerate()
            .map(|(id, boundary)| {
                self.create_rift_valley_from_boundary(id as u32, boundary, plates)
            })
            .collect();

        rift_valleys.map_err(|e| SchedulerError::TaskFailed(e.to_string()))
    }

    /// Create rift valley from divergent boundary
    fn create_rift_valley_from_boundary(
        &self,
        id: u32,
        boundary: &PlateBoundary,
        plates: &[TectonicPlate],
    ) -> Result<RiftValley, String> {
        let mut rng = self.rng.clone();
        
        // Create center line from boundary geometry
        let center_line = boundary.geometry.clone();
        
        // Calculate width based on spreading rate and age
        let spreading_rate = boundary.relative_velocity * 1000.0; // Convert to mm/year
        let width = spreading_rate.max(10.0).min(200.0); // 10-200 km wide
        
        // Calculate depth based on spreading rate
        let depth = (spreading_rate / 10.0 * 1000.0).max(500.0).min(3000.0); // 0.5-3 km deep
        
        // Generate escarpments on both sides
        let escarpments = self.generate_rift_escarpments(&center_line, width)?;
        
        // Estimate age
        let age_million_years = rng.gen_range(1.0..50.0);

        Ok(RiftValley {
            id,
            name: format!("Rift_{}", id),
            boundary_id: boundary.id,
            center_line,
            width,
            depth,
            escarpments,
            age_million_years,
            spreading_rate,
        })
    }

    /// Generate escarpments for rift valley
    fn generate_rift_escarpments(&self, center_line: &[Vector2<f64>], width: f64) -> Result<Vec<Vec<Vector2<f64>>>, String> {
        if center_line.len() < 2 {
            return Ok(vec![Vec::new(), Vec::new()]);
        }

        let mut left_escarpment = Vec::new();
        let mut right_escarpment = Vec::new();
        
        let half_width = width / 2.0;

        for i in 0..center_line.len() {
            let direction = if i == center_line.len() - 1 {
                (center_line[i] - center_line[i-1]).normalize()
            } else {
                (center_line[i+1] - center_line[i]).normalize()
            };
            
            let perpendicular = Vector2::new(-direction.y, direction.x);
            
            left_escarpment.push(center_line[i] + perpendicular * half_width);
            right_escarpment.push(center_line[i] - perpendicular * half_width);
        }

        Ok(vec![left_escarpment, right_escarpment])
    }

    /// Generate transform faults from transform boundaries
    fn generate_transform_faults(
        &self,
        plates: &[TectonicPlate],
        boundaries: &[PlateBoundary],
    ) -> Result<Vec<TransformFault>, SchedulerError> {
        let transform_boundaries: Vec<_> = boundaries
            .iter()
            .filter(|b| b.boundary_type == BoundaryType::Transform)
            .collect();

        let transform_faults: Result<Vec<_>, _> = transform_boundaries
            .par_iter()
            .enumerate()
            .map(|(id, boundary)| {
                self.create_transform_fault_from_boundary(id as u32, boundary, plates)
            })
            .collect();

        transform_faults.map_err(|e| SchedulerError::TaskFailed(e.to_string()))
    }

    /// Create transform fault from transform boundary
    fn create_transform_fault_from_boundary(
        &self,
        id: u32,
        boundary: &PlateBoundary,
        plates: &[TectonicPlate],
    ) -> Result<TransformFault, String> {
        let mut rng = self.rng.clone();
        
        // Create fault trace from boundary geometry
        let trace = boundary.geometry.clone();
        
        // Calculate slip rate from relative velocity
        let slip_rate = boundary.relative_velocity * 1000.0; // Convert to mm/year
        
        // Estimate total displacement based on age and slip rate
        let age_million_years = rng.gen_range(5.0..100.0);
        let displacement = slip_rate * age_million_years * 1000.0; // Total offset in meters
        
        // Generate fault zones along the trace
        let fault_zones = self.generate_fault_zones(&trace, slip_rate)?;

        Ok(TransformFault {
            id,
            name: format!("Fault_{}", id),
            boundary_id: boundary.id,
            trace,
            displacement,
            fault_zones,
            age_million_years,
            slip_rate,
        })
    }

    /// Generate fault zones along transform fault
    fn generate_fault_zones(&self, trace: &[Vector2<f64>], slip_rate: f64) -> Result<Vec<FaultZone>, String> {
        let mut fault_zones = Vec::new();
        let mut rng = self.rng.clone();
        
        // Divide trace into segments
        let segment_length = 50.0; // 50 km segments
        let mut current_distance = 0.0;
        let mut segment_start = 0;

        for i in 1..trace.len() {
            let distance = (trace[i] - trace[i-1]).magnitude();
            current_distance += distance;

            if current_distance >= segment_length || i == trace.len() - 1 {
                // Create fault zone for this segment
                let segment = trace[segment_start..=i].to_vec();
                let width = (slip_rate / 10.0).max(1.0).min(20.0); // 1-20 km wide
                let recent_activity = rng.gen_range(0.0..10000.0); // Last 10k years

                fault_zones.push(FaultZone {
                    segment,
                    width,
                    recent_activity,
                });

                segment_start = i;
                current_distance = 0.0;
            }
        }

        Ok(fault_zones)
    }

    /// Create convex hull from a set of points (useful for island chains, mountain outlines, etc.)
    pub fn create_convex_hull_outline(&self, points: &[Vector2<f64>]) -> Vec<Vector2<f64>> {
        if points.len() < 3 {
            return points.to_vec();
        }

        // Use Zig backend for convex hull calculation
        crate::world::generation::hydrology::zig_ffi::zig_convex_hull(points)
    }

    /// Create island chain outline from volcanic peaks using convex hull
    pub fn create_island_chain_outline(&self, peaks: &[MountainPeak]) -> Vec<Vector2<f64>> {
        let peak_positions: Vec<Vector2<f64>> = peaks
            .iter()
            .filter(|peak| matches!(peak.peak_type, PeakType::Volcanic))
            .map(|peak| peak.position)
            .collect();

        self.create_convex_hull_outline(&peak_positions)
    }

    /// Create mountain range boundary from all peaks
    pub fn create_mountain_range_boundary(&self, range: &MountainRange) -> Vec<Vector2<f64>> {
        let all_positions: Vec<Vector2<f64>> = range.peaks
            .iter()
            .map(|peak| peak.position)
            .collect();

        self.create_convex_hull_outline(&all_positions)
    }
}
