//! Tectonic Plate Generation
//!
//! High-performance plate generation using Delaunator triangulation for
//! realistic plate boundaries and Voronoi cell-based plate regions.

use super::TectonicsConfig;
use serde::{Deserialize, Serialize};
use nalgebra::Vector2;
use delaunator::{triangulate, Point};
use rand::{Rng, RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;
use rand_distr::{Distribution, Normal};
use rayon::prelude::*;
use geo::{Polygon, Coord, Contains};
use crate::core::scheduler::SchedulerError;

/// Represents a single tectonic plate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TectonicPlate {
    pub id: u32,
    pub center: Vector2<f64>,
    pub velocity: Vector2<f64>,
    pub age_million_years: f64,
    pub plate_type: PlateType,
    pub density: f64, // kg/m³, affects subduction
    pub area: f64,
    pub boundary_points: Vec<Vector2<f64>>,
    pub polygon: Vec<(f64, f64)>, // For contains_point checks
}

impl Default for TectonicPlate {
    fn default() -> Self {
        Self {
            id: 0,
            center: Vector2::new(0.0, 0.0),
            velocity: Vector2::new(0.0, 0.0),
            age_million_years: 50.0,
            plate_type: PlateType::Continental,
            density: 2700.0, // Continental crust density
            area: 1000000.0, // Default 1M km²
            boundary_points: Vec::new(),
            polygon: Vec::new(),
        }
    }
}

impl TectonicPlate {
    /// Check if a point is contained within this plate
    pub fn contains_point(&self, x: f64, y: f64) -> bool {
        if self.polygon.is_empty() {
            // Fallback to distance from center
            let dx = x - self.center.x;
            let dy = y - self.center.y;
            (dx * dx + dy * dy).sqrt() < 200.0 // Default radius
        } else {
            // Use polygon containment
            let polygon = Polygon::new(
                geo::LineString::from(
                    self.polygon.iter().map(|(x, y)| Coord { x: *x, y: *y }).collect::<Vec<_>>()
                ),
                vec![]
            );
            polygon.contains(&Coord { x, y })
        }
    }
    
    /// Calculate plate's kinetic energy for collision calculations
    pub fn kinetic_energy(&self) -> f64 {
        let mass = self.area * self.density * 35000.0; // Assuming 35km average thickness
        let velocity_magnitude = self.velocity.magnitude_squared();
        0.5 * mass * velocity_magnitude
    }
}

/// Types of tectonic plates
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum PlateType {
    Continental, // Thicker, less dense
    Oceanic,     // Thinner, more dense
    Mixed,       // Contains both types
}

/// Plate generation using Delaunay triangulation and Voronoi regions
#[derive(Debug, Clone)]
pub struct PlateGenerator {
    config: TectonicsConfig,
    rng: ChaCha8Rng,
}

impl PlateGenerator {
    /// Create new plate generator
    pub fn new(config: &TectonicsConfig) -> Self {
        let rng = ChaCha8Rng::seed_from_u64(config.seed);
        Self {
            config: config.clone(),
            rng,
        }
    }

    /// Generate tectonic plates using Delaunay triangulation
    pub fn generate_plates(&self) -> Result<Vec<TectonicPlate>, SchedulerError> {
        let mut rng = self.rng.clone();
        
        // Generate seed points for plates using Poisson disk sampling for even distribution
        let seed_points = self.generate_plate_seeds(&mut rng)?;
        
        // Create Delaunay triangulation
        let triangulation = self.create_triangulation(&seed_points)?;
        
        // Generate Voronoi regions from triangulation
        let voronoi_regions = self.extract_voronoi_regions(&triangulation, &seed_points)?;
        
        // Create plates from regions with realistic properties
        let plates = self.create_plates_from_regions(voronoi_regions, &mut rng)?;
        
        Ok(plates)
    }

    /// Generate well-distributed seed points for plates
    fn generate_plate_seeds(&self, rng: &mut ChaCha8Rng) -> Result<Vec<Point>, SchedulerError> {
        let (min_x, min_y, max_x, max_y) = self.config.world_bounds;
        let width = max_x - min_x;
        let height = max_y - min_y;
        
        // Use Poisson disk sampling for even distribution
        let min_distance = (width * height / self.config.plate_count as f64).sqrt() * 0.7;
        
        let mut points = Vec::new();
        let mut attempts = 0;
        let max_attempts = self.config.plate_count * 30; // Reasonable limit
        
        while points.len() < self.config.plate_count as usize && attempts < max_attempts {
            let x = rng.gen_range(min_x..max_x);
            let y = rng.gen_range(min_y..max_y);
            
            // Check minimum distance constraint
            let too_close = points.iter().any(|existing_point: &Point| {
                let dx = x - existing_point.x;
                let dy = y - existing_point.y;
                (dx * dx + dy * dy).sqrt() < min_distance
            });
            
            if !too_close {
                points.push(Point { x, y });
            }
            
            attempts += 1;
        }
        
        // If we didn't get enough points, fill with random ones
        while points.len() < self.config.plate_count as usize {
            let x = rng.gen_range(min_x..max_x);
            let y = rng.gen_range(min_y..max_y);
            points.push(Point { x, y });
        }
        
        Ok(points)
    }

    /// Create Delaunay triangulation from seed points
    fn create_triangulation(&self, points: &[Point]) -> Result<delaunator::Triangulation, SchedulerError> {
        if points.len() < 3 {
            return Err(SchedulerError::TaskFailed("Not enough points for triangulation".to_string()));
        }
        Ok(triangulate(points))
    }

    /// Extract Voronoi regions from Delaunay triangulation
    fn extract_voronoi_regions(
        &self,
        triangulation: &delaunator::Triangulation,
        seed_points: &[Point],
    ) -> Result<Vec<Vec<(f64, f64)>>, SchedulerError> {
        let mut regions: Vec<Vec<(f64, f64)>> = vec![Vec::new(); seed_points.len()];
        
        // Convert triangulation to Voronoi diagram
        // For each triangle, find its circumcenter and assign to nearest seed point
        let triangles = &triangulation.triangles;
        
        for triangle_idx in (0..triangles.len()).step_by(3) {
            let i = triangles[triangle_idx] as usize;
            let j = triangles[triangle_idx + 1] as usize;
            let k = triangles[triangle_idx + 2] as usize;
            
            if i < seed_points.len() && j < seed_points.len() && k < seed_points.len() {
                let p1 = &seed_points[i];
                let p2 = &seed_points[j];
                let p3 = &seed_points[k];
                
                // Calculate circumcenter
                if let Some(circumcenter) = self.circumcenter(p1, p2, p3) {
                    // Add circumcenter to each vertex's region
                    regions[i].push(circumcenter);
                    regions[j].push(circumcenter);
                    regions[k].push(circumcenter);
                }
            }
        }
        
        // Sort points in each region to form proper polygons
        for region in &mut regions {
            if region.len() > 2 {
                self.sort_polygon_points(region);
            }
        }
        
        Ok(regions)
    }

    /// Calculate circumcenter of triangle
    fn circumcenter(&self, p1: &Point, p2: &Point, p3: &Point) -> Option<(f64, f64)> {
        let ax = p1.x;
        let ay = p1.y;
        let bx = p2.x;
        let by = p2.y;
        let cx = p3.x;
        let cy = p3.y;
        
        let d = 2.0 * (ax * (by - cy) + bx * (cy - ay) + cx * (ay - by));
        
        if d.abs() < 1e-10 {
            return None; // Points are collinear
        }
        
        let ux = ((ax * ax + ay * ay) * (by - cy) + (bx * bx + by * by) * (cy - ay) + (cx * cx + cy * cy) * (ay - by)) / d;
        let uy = ((ax * ax + ay * ay) * (cx - bx) + (bx * bx + by * by) * (ax - cx) + (cx * cx + cy * cy) * (bx - ax)) / d;
        
        Some((ux, uy))
    }

    /// Sort polygon points in counter-clockwise order
    fn sort_polygon_points(&self, points: &mut Vec<(f64, f64)>) {
        if points.len() < 3 {
            return;
        }
        
        // Find centroid
        let centroid_x = points.iter().map(|(x, _)| x).sum::<f64>() / points.len() as f64;
        let centroid_y = points.iter().map(|(_, y)| y).sum::<f64>() / points.len() as f64;
        
        // Sort by angle from centroid
        points.sort_by(|a, b| {
            let angle_a = (a.1 - centroid_y).atan2(a.0 - centroid_x);
            let angle_b = (b.1 - centroid_y).atan2(b.0 - centroid_x);
            angle_a.partial_cmp(&angle_b).unwrap()
        });
    }

    /// Create realistic tectonic plates from Voronoi regions
    fn create_plates_from_regions(
        &self,
        regions: Vec<Vec<(f64, f64)>>,
        rng: &mut ChaCha8Rng,
    ) -> Result<Vec<TectonicPlate>, SchedulerError> {
        // Generate seed for parallel processing
        let base_seed = rng.next_u64();
        
        let plates: Result<Vec<_>, _> = regions
            .par_iter()
            .enumerate()
            .map(|(id, region)| {
                // Create thread-local RNG with unique seed
                let mut thread_rng = ChaCha8Rng::seed_from_u64(base_seed.wrapping_add(id as u64));
                self.create_single_plate(id as u32, region.clone(), &mut thread_rng)
            })
            .collect();
            
        plates.map_err(|e| SchedulerError::TaskFailed(e.to_string()))
    }

    /// Create a single tectonic plate with realistic properties
    fn create_single_plate(
        &self,
        id: u32,
        polygon: Vec<(f64, f64)>,
        rng: &mut ChaCha8Rng,
    ) -> Result<TectonicPlate, super::PlateGenerationError> {
        if polygon.is_empty() {
            return Err(super::PlateGenerationError::EmptyPolygon { plate_id: id });
        }
        
        // Calculate centroid
        let center_x = polygon.iter().map(|(x, _)| x).sum::<f64>() / polygon.len() as f64;
        let center_y = polygon.iter().map(|(_, y)| y).sum::<f64>() / polygon.len() as f64;
        let center = Vector2::new(center_x, center_y);
        
        // Calculate area using shoelace formula
        let area = self.calculate_polygon_area(&polygon);
        
        // Generate realistic plate properties
        let plate_type = self.determine_plate_type(center, area, rng);
        let density = match plate_type {
            PlateType::Continental => rng.gen_range(2600.0..2800.0), // Continental crust
            PlateType::Oceanic => rng.gen_range(2900.0..3300.0),     // Oceanic crust
            PlateType::Mixed => rng.gen_range(2700.0..3000.0),       // Mixed
        };
        
        // Generate realistic velocity (1-10 cm/year in random direction)
        let velocity_magnitude = rng.gen_range(0.01..0.1); // meters per year
        let velocity_angle = rng.gen_range(0.0..std::f64::consts::TAU);
        let velocity = Vector2::new(
            velocity_magnitude * velocity_angle.cos(),
            velocity_magnitude * velocity_angle.sin(),
        );
        
        // Generate plate age
        let age_normal = Normal::new(self.config.max_plate_age_million_years / 2.0, 
                                  self.config.max_plate_age_million_years / 4.0).unwrap();
        let age_million_years = age_normal.sample(rng).max(1.0).min(self.config.max_plate_age_million_years);
        
        // Extract boundary points
        let boundary_points = polygon.iter()
            .map(|(x, y)| Vector2::new(*x, *y))
            .collect();

        Ok(TectonicPlate {
            id,
            center,
            velocity: velocity * self.config.movement_speed,
            age_million_years,
            plate_type,
            density,
            area: area.abs(),
            boundary_points,
            polygon,
        })
    }

    /// Calculate polygon area using shoelace formula
    fn calculate_polygon_area(&self, polygon: &[(f64, f64)]) -> f64 {
        if polygon.len() < 3 {
            return 0.0;
        }
        
        let mut area = 0.0;
        let n = polygon.len();
        
        for i in 0..n {
            let j = (i + 1) % n;
            area += polygon[i].0 * polygon[j].1;
            area -= polygon[j].0 * polygon[i].1;
        }
        
        area / 2.0
    }

    /// Determine plate type based on location and size
    fn determine_plate_type(&self, center: Vector2<f64>, area: f64, rng: &mut ChaCha8Rng) -> PlateType {
        let (min_x, min_y, max_x, max_y) = self.config.world_bounds;
        let world_area = (max_x - min_x) * (max_y - min_y);
        let relative_area = area / world_area;
        
        // Larger plates are more likely to be continental or mixed
        // Smaller plates are more likely to be oceanic
        let continental_probability = if relative_area > 0.1 {
            0.7 // Large plates favor continental
        } else if relative_area > 0.05 {
            0.4 // Medium plates mixed
        } else {
            0.2 // Small plates favor oceanic
        };
        
        if rng.gen_bool(continental_probability) {
            if relative_area > 0.15 {
                PlateType::Mixed // Very large plates often mixed
            } else {
                PlateType::Continental
            }
        } else {
            PlateType::Oceanic
        }
    }
}
