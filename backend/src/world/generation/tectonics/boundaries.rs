//! Plate Boundary Detection and Classification
//!
//! Uses computational geometry to detect boundaries between tectonic plates
//! and classify them as convergent, divergent, or transform boundaries.

use super::{TectonicsConfig, TectonicPlate, zig_ffi};
use serde::{Deserialize, Serialize};
use nalgebra::Vector2;
use geo::{LineString, Coord, Contains, Intersects, Line, Point as GeoPoint};
use robust::{orient2d, incircle};
use rayon::prelude::*;
use std::collections::HashMap;
use crate::core::scheduler::SchedulerError;

/// Represents a boundary between two tectonic plates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlateBoundary {
    pub id: u32,
    pub plate1_id: u32,
    pub plate2_id: u32,
    pub boundary_type: BoundaryType,
    pub geometry: Vec<Vector2<f64>>, // Line segments forming the boundary
    pub length: f64,
    pub relative_velocity: f64,
    pub stress_magnitude: f64,
    pub last_activity: f64, // Geological time since last major activity
}

impl PlateBoundary {
    /// Calculate distance from a point to this boundary using Zig SIMD optimization
    pub fn distance_to_point(&self, x: f64, y: f64) -> f64 {
        if self.geometry.is_empty() {
            return f64::INFINITY;
        }

        let point = Vector2::new(x, y);
        let mut min_distance = f64::INFINITY;

        // Check distance to each segment using Zig SIMD-optimized function
        for i in 0..self.geometry.len().saturating_sub(1) {
            let seg_start = self.geometry[i];
            let seg_end = self.geometry[i + 1];
            
            let distance = zig_ffi::point_to_segment_distance_zig(point, seg_start, seg_end);
            min_distance = min_distance.min(distance);
        }

        min_distance
    }

}

/// Types of plate boundaries
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum BoundaryType {
    Convergent, // Plates moving toward each other
    Divergent,  // Plates moving away from each other
    Transform,  // Plates sliding past each other
}

/// Boundary detection using computational geometry
#[derive(Debug, Clone)]
pub struct BoundaryDetector {
    config: TectonicsConfig,
}

impl BoundaryDetector {
    /// Create new boundary detector
    pub fn new(config: &TectonicsConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    /// Detect boundaries between all plates
    pub fn detect_boundaries(&self, plates: &[TectonicPlate]) -> Result<Vec<PlateBoundary>, SchedulerError> {
        if plates.len() < 2 {
            return Ok(Vec::new());
        }

        // Find adjacent plate pairs using parallel processing
        let adjacency_pairs = self.find_adjacent_plates(plates)?;

        // Detect boundaries for each adjacent pair in parallel
        let boundaries: Result<Vec<_>, _> = adjacency_pairs
            .par_iter()
            .enumerate()
            .filter_map(|(id, (i, j))| {
                match self.detect_boundary_between_plates(id as u32, &plates[*i], &plates[*j]) {
                    Ok(Some(boundary)) => Some(Ok(boundary)),
                    Ok(None) => None,
                    Err(e) => Some(Err(e)),
                }
            })
            .collect();

        boundaries.map_err(|e| SchedulerError::TaskFailed(e.to_string()))
    }

    /// Find which plates are adjacent to each other
    fn find_adjacent_plates(&self, plates: &[TectonicPlate]) -> Result<Vec<(usize, usize)>, SchedulerError> {
        let mut adjacent_pairs = Vec::new();

        // Check all plate pairs for adjacency
        for i in 0..plates.len() {
            for j in (i + 1)..plates.len() {
                if self.plates_are_adjacent(&plates[i], &plates[j])? {
                    adjacent_pairs.push((i, j));
                }
            }
        }

        Ok(adjacent_pairs)
    }

    /// Check if two plates are adjacent (share a boundary)
    fn plates_are_adjacent(&self, plate1: &TectonicPlate, plate2: &TectonicPlate) -> Result<bool, SchedulerError> {
        // Check if plate boundaries intersect or are very close
        let max_separation = 50.0; // km

        // Simple distance check first
        let center_distance = (plate2.center - plate1.center).magnitude();
        if center_distance > 1000.0 { // Too far apart
            return Ok(false);
        }

        // Check if any boundary points are close
        for point1 in &plate1.boundary_points {
            for point2 in &plate2.boundary_points {
                let distance = (point2 - point1).magnitude();
                if distance < max_separation {
                    return Ok(true);
                }
            }
        }

        // Check for polygon intersection/proximity
        self.check_polygon_proximity(plate1, plate2, max_separation)
    }

    /// Check if two plate polygons are close enough to share a boundary
    fn check_polygon_proximity(&self, plate1: &TectonicPlate, plate2: &TectonicPlate, max_distance: f64) -> Result<bool, SchedulerError> {
        if plate1.polygon.is_empty() || plate2.polygon.is_empty() {
            return Ok(false);
        }

        // Convert to geo types for intersection testing
        let linestring1 = LineString::from(
            plate1.polygon.iter()
                .map(|(x, y)| Coord { x: *x, y: *y })
                .collect::<Vec<_>>()
        );
        
        let linestring2 = LineString::from(
            plate2.polygon.iter()
                .map(|(x, y)| Coord { x: *x, y: *y })
                .collect::<Vec<_>>()
        );

        // Check if linestrings intersect
        if linestring1.intersects(&linestring2) {
            return Ok(true);
        }

        // Check minimum distance between polygons
        let min_distance = self.calculate_minimum_polygon_distance(&linestring1, &linestring2);
        Ok(min_distance < max_distance)
    }

    /// Calculate minimum distance between two polygons
    fn calculate_minimum_polygon_distance(&self, poly1: &LineString<f64>, poly2: &LineString<f64>) -> f64 {
        let mut min_distance = f64::INFINITY;

        // Check distance between all segment pairs
        let coords1 = poly1.coords().collect::<Vec<_>>();
        let coords2 = poly2.coords().collect::<Vec<_>>();

        for i in 0..coords1.len().saturating_sub(1) {
            for j in 0..coords2.len().saturating_sub(1) {
                let seg1 = Line::new(*coords1[i], *coords1[i + 1]);
                let seg2 = Line::new(*coords2[j], *coords2[j + 1]);
                
                let distance = self.segment_to_segment_distance(seg1, seg2);
                min_distance = min_distance.min(distance);
            }
        }

        min_distance
    }

    /// Calculate distance between two line segments
    fn segment_to_segment_distance(&self, seg1: Line<f64>, seg2: Line<f64>) -> f64 {
        // Simplified segment-to-segment distance calculation
        let points = [
            seg1.start, seg1.end, seg2.start, seg2.end
        ];

        let mut min_dist = f64::INFINITY;

        // Point to segment distances
        for i in 0..2 {
            for j in 2..4 {
                let point = points[i];
                let seg = if j == 2 { seg2 } else { seg1 };
                let dist = self.point_to_line_distance(point, seg);
                min_dist = min_dist.min(dist);
            }
        }

        min_dist
    }

    /// Calculate distance from point to line segment
    fn point_to_line_distance(&self, point: Coord<f64>, line: Line<f64>) -> f64 {
        let seg_vec = Coord {
            x: line.end.x - line.start.x,
            y: line.end.y - line.start.y,
        };
        
        let point_vec = Coord {
            x: point.x - line.start.x,
            y: point.y - line.start.y,
        };

        let seg_length_sq = seg_vec.x * seg_vec.x + seg_vec.y * seg_vec.y;
        
        if seg_length_sq < 1e-10 {
            // Degenerate segment
            let dx = point.x - line.start.x;
            let dy = point.y - line.start.y;
            return (dx * dx + dy * dy).sqrt();
        }

        let t = ((point_vec.x * seg_vec.x + point_vec.y * seg_vec.y) / seg_length_sq).clamp(0.0, 1.0);
        
        let projection = Coord {
            x: line.start.x + t * seg_vec.x,
            y: line.start.y + t * seg_vec.y,
        };

        let dx = point.x - projection.x;
        let dy = point.y - projection.y;
        (dx * dx + dy * dy).sqrt()
    }

    /// Detect boundary between two specific plates
    fn detect_boundary_between_plates(
        &self,
        boundary_id: u32,
        plate1: &TectonicPlate,
        plate2: &TectonicPlate,
    ) -> Result<Option<PlateBoundary>, super::BoundaryError> {
        // Find intersection points/segments between plate boundaries
        let boundary_geometry = self.extract_boundary_geometry(plate1, plate2)?;
        
        if boundary_geometry.is_empty() {
            return Ok(None);
        }

        // Calculate boundary length
        let length = self.calculate_boundary_length(&boundary_geometry);

        // Determine boundary type based on relative velocities
        let boundary_type = self.classify_boundary_type(plate1, plate2);

        // Calculate relative velocity and stress
        let relative_velocity = self.calculate_relative_velocity(plate1, plate2);
        let stress_magnitude = self.calculate_boundary_stress(plate1, plate2, boundary_type);

        Ok(Some(PlateBoundary {
            id: boundary_id,
            plate1_id: plate1.id,
            plate2_id: plate2.id,
            boundary_type,
            geometry: boundary_geometry,
            length,
            relative_velocity,
            stress_magnitude,
            last_activity: 0.0, // Will be updated by geological activity systems
        }))
    }

    /// Extract the geometric boundary between two plates
    fn extract_boundary_geometry(&self, plate1: &TectonicPlate, plate2: &TectonicPlate) -> Result<Vec<Vector2<f64>>, super::BoundaryError> {
        let mut boundary_points = Vec::new();

        // Find intersection or closest approach points between plate polygons
        if !plate1.boundary_points.is_empty() && !plate2.boundary_points.is_empty() {
            // Sample points along the boundary between plates
            let center_line = plate2.center - plate1.center;
            let perpendicular = Vector2::new(-center_line.y, center_line.x).normalize();
            
            // Create a line perpendicular to the plate centers
            let mid_point = (plate1.center + plate2.center) * 0.5;
            let line_length = center_line.magnitude() * 0.8;
            
            // Sample points along this boundary line
            let num_points = (line_length / 50.0).max(3.0) as usize; // Every 50km
            
            for i in 0..num_points {
                let t = (i as f64) / (num_points - 1) as f64 - 0.5;
                let point = mid_point + perpendicular * (t * line_length);
                boundary_points.push(point);
            }
        }

        // If we don't have enough points, create a simple boundary
        if boundary_points.len() < 2 {
            let midpoint = (plate1.center + plate2.center) * 0.5;
            let direction = (plate2.center - plate1.center).normalize();
            let perpendicular = Vector2::new(-direction.y, direction.x);
            
            boundary_points = vec![
                midpoint + perpendicular * 100.0,
                midpoint - perpendicular * 100.0,
            ];
        }

        Ok(boundary_points)
    }

    /// Calculate total length of boundary geometry
    fn calculate_boundary_length(&self, geometry: &[Vector2<f64>]) -> f64 {
        if geometry.len() < 2 {
            return 0.0;
        }

        geometry.windows(2)
            .map(|pair| (pair[1] - pair[0]).magnitude())
            .sum()
    }

    /// Classify boundary type based on plate movement
    fn classify_boundary_type(&self, plate1: &TectonicPlate, plate2: &TectonicPlate) -> BoundaryType {
        let relative_velocity = plate2.velocity - plate1.velocity;
        let plate_separation = (plate2.center - plate1.center).normalize();
        
        // Dot product tells us if plates are converging or diverging
        let radial_component = relative_velocity.dot(&plate_separation);
        let tangential_component = relative_velocity.magnitude_squared() - radial_component.powi(2);
        
        if radial_component.abs() > tangential_component.sqrt() * 0.7 {
            // Primarily radial motion
            if radial_component < -0.01 {
                BoundaryType::Convergent
            } else if radial_component > 0.01 {
                BoundaryType::Divergent  
            } else {
                BoundaryType::Transform
            }
        } else {
            // Primarily tangential motion
            BoundaryType::Transform
        }
    }

    /// Calculate relative velocity magnitude between plates
    fn calculate_relative_velocity(&self, plate1: &TectonicPlate, plate2: &TectonicPlate) -> f64 {
        (plate2.velocity - plate1.velocity).magnitude()
    }

    /// Calculate stress magnitude at boundary
    fn calculate_boundary_stress(&self, plate1: &TectonicPlate, plate2: &TectonicPlate, boundary_type: BoundaryType) -> f64 {
        let relative_velocity = (plate2.velocity - plate1.velocity).magnitude();
        let density_contrast = (plate1.density - plate2.density).abs();
        
        // Base stress calculation (simplified)
        let base_stress = relative_velocity * 1e6; // Convert to realistic stress units
        
        // Modify based on boundary type
        let type_multiplier = match boundary_type {
            BoundaryType::Convergent => 2.0, // High compressive stress
            BoundaryType::Divergent => 1.2,  // Moderate tensional stress
            BoundaryType::Transform => 1.5,  // High shear stress
        };
        
        // Account for density differences
        let density_factor = 1.0 + density_contrast / 1000.0;
        
        base_stress * type_multiplier * density_factor
    }
}

/// Statistics about detected boundaries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundaryStats {
    pub total_boundaries: u32,
    pub convergent_count: u32,
    pub divergent_count: u32,
    pub transform_count: u32,
    pub total_length: f64,
    pub average_stress: f64,
    pub max_stress: f64,
}
