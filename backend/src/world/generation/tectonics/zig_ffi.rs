//! Zig FFI Integration for Tectonic Calculations
//!
//! High-performance SIMD-optimized calculations using Zig functions
//! for performance-critical tectonic computations.

use nalgebra::Vector2;

// External Zig function declarations
extern "C" {
    // Plate physics calculations
    fn manifest_calculate_ridge_push(
        plate_center_x: f64, plate_center_y: f64,
        plate_vel_x: f64, plate_vel_y: f64,
        age_million_years: f64, area: f64, movement_speed: f64,
        result_x: *mut f64, result_y: *mut f64,
    );

    fn manifest_calculate_basal_drag(
        plate_vel_x: f64, plate_vel_y: f64, area: f64,
        result_x: *mut f64, result_y: *mut f64,
    );

    fn manifest_calculate_mantle_convection(
        plate_center_x: f64, plate_center_y: f64, area: f64, movement_speed: f64,
        result_x: *mut f64, result_y: *mut f64,
    );

    // Geometric calculations
    fn manifest_point_to_segment_distance(
        point_x: f64, point_y: f64,
        seg_start_x: f64, seg_start_y: f64,
        seg_end_x: f64, seg_end_y: f64,
    ) -> f64;

    fn manifest_polygon_contains_point(
        vertices_x: *const f64, vertices_y: *const f64, vertex_count: usize,
        point_x: f64, point_y: f64,
    ) -> bool;

    fn manifest_polygon_area(
        vertices_x: *const f64, vertices_y: *const f64, vertex_count: usize,
    ) -> f64;

    // Stress field calculations
    fn manifest_stress_von_mises(stress_xx: f64, stress_yy: f64, stress_xy: f64) -> f64;
    fn manifest_stress_max_principal(stress_xx: f64, stress_yy: f64, stress_xy: f64) -> f64;
    fn manifest_stress_principal_angle(stress_xx: f64, stress_yy: f64, stress_xy: f64) -> f64;

    // Volcanic hazard calculations
    fn manifest_volcanic_pyroclastic_hazard(
        volcano_x: f64, volcano_y: f64, vei_scale: u32, hazard_radius: f64,
        target_x: f64, target_y: f64,
        wind_direction: f64, wind_speed: f64,
    ) -> f64;

    fn manifest_volcanic_ash_hazard(
        volcano_x: f64, volcano_y: f64, vei_scale: u32,
        target_x: f64, target_y: f64,
        wind_direction: f64, wind_speed: f64, column_height: f64,
    ) -> f64;

    // Batch calculations
    fn manifest_batch_plate_distances(
        plates_x: *const f64, plates_y: *const f64, plate_count: usize,
        distances: *mut f64,
    );
}

/// Safe Rust wrapper for ridge push force calculation
pub fn calculate_ridge_push_zig(
    plate_center: Vector2<f64>,
    plate_velocity: Vector2<f64>,
    age_million_years: f64,
    area: f64,
    movement_speed: f64,
) -> Vector2<f64> {
    let mut result_x: f64 = 0.0;
    let mut result_y: f64 = 0.0;

    unsafe {
        manifest_calculate_ridge_push(
            plate_center.x, plate_center.y,
            plate_velocity.x, plate_velocity.y,
            age_million_years, area, movement_speed,
            &mut result_x, &mut result_y,
        );
    }

    Vector2::new(result_x, result_y)
}

/// Safe Rust wrapper for basal drag calculation
pub fn calculate_basal_drag_zig(plate_velocity: Vector2<f64>, area: f64) -> Vector2<f64> {
    let mut result_x: f64 = 0.0;
    let mut result_y: f64 = 0.0;

    unsafe {
        manifest_calculate_basal_drag(
            plate_velocity.x, plate_velocity.y, area,
            &mut result_x, &mut result_y,
        );
    }

    Vector2::new(result_x, result_y)
}

/// Safe Rust wrapper for mantle convection calculation
pub fn calculate_mantle_convection_zig(
    plate_center: Vector2<f64>,
    area: f64,
    movement_speed: f64,
) -> Vector2<f64> {
    let mut result_x: f64 = 0.0;
    let mut result_y: f64 = 0.0;

    unsafe {
        manifest_calculate_mantle_convection(
            plate_center.x, plate_center.y, area, movement_speed,
            &mut result_x, &mut result_y,
        );
    }

    Vector2::new(result_x, result_y)
}

/// Safe Rust wrapper for point to segment distance calculation
pub fn point_to_segment_distance_zig(
    point: Vector2<f64>,
    segment_start: Vector2<f64>,
    segment_end: Vector2<f64>,
) -> f64 {
    unsafe {
        manifest_point_to_segment_distance(
            point.x, point.y,
            segment_start.x, segment_start.y,
            segment_end.x, segment_end.y,
        )
    }
}

/// Safe Rust wrapper for polygon containment test
pub fn polygon_contains_point_zig(polygon: &[(f64, f64)], point: Vector2<f64>) -> bool {
    if polygon.is_empty() || polygon.len() > 32 {
        return false; // Zig function has limits
    }

    let mut vertices_x = Vec::with_capacity(polygon.len());
    let mut vertices_y = Vec::with_capacity(polygon.len());

    for (x, y) in polygon {
        vertices_x.push(*x);
        vertices_y.push(*y);
    }

    unsafe {
        manifest_polygon_contains_point(
            vertices_x.as_ptr(), vertices_y.as_ptr(), polygon.len(),
            point.x, point.y,
        )
    }
}

/// Safe Rust wrapper for polygon area calculation
pub fn polygon_area_zig(polygon: &[(f64, f64)]) -> f64 {
    if polygon.is_empty() || polygon.len() > 32 {
        return 0.0; // Zig function has limits
    }

    let mut vertices_x = Vec::with_capacity(polygon.len());
    let mut vertices_y = Vec::with_capacity(polygon.len());

    for (x, y) in polygon {
        vertices_x.push(*x);
        vertices_y.push(*y);
    }

    unsafe {
        manifest_polygon_area(
            vertices_x.as_ptr(), vertices_y.as_ptr(), polygon.len(),
        )
    }
}

/// Stress tensor representation for FFI
#[derive(Debug, Clone, Copy)]
pub struct StressTensor {
    pub xx: f64,
    pub yy: f64,
    pub xy: f64,
}

impl StressTensor {
    pub fn new(xx: f64, yy: f64, xy: f64) -> Self {
        Self { xx, yy, xy }
    }

    /// Calculate Von Mises stress using Zig
    pub fn von_mises_stress_zig(&self) -> f64 {
        unsafe { manifest_stress_von_mises(self.xx, self.yy, self.xy) }
    }

    /// Calculate maximum principal stress using Zig
    pub fn max_principal_stress_zig(&self) -> f64 {
        unsafe { manifest_stress_max_principal(self.xx, self.yy, self.xy) }
    }

    /// Calculate principal stress angle using Zig
    pub fn principal_stress_angle_zig(&self) -> f64 {
        unsafe { manifest_stress_principal_angle(self.xx, self.yy, self.xy) }
    }
}

/// Volcano data for hazard calculations
pub struct VolcanoData {
    pub position: Vector2<f64>,
    pub vei_scale: u32,
    pub hazard_radius: f64,
}

/// Calculate pyroclastic flow hazard using Zig
pub fn calculate_pyroclastic_hazard_zig(
    volcano: &VolcanoData,
    target: Vector2<f64>,
    wind_direction: f64,
    wind_speed: f64,
) -> f64 {
    unsafe {
        manifest_volcanic_pyroclastic_hazard(
            volcano.position.x, volcano.position.y, volcano.vei_scale, volcano.hazard_radius,
            target.x, target.y,
            wind_direction, wind_speed,
        )
    }
}

/// Calculate ash fall hazard using Zig
pub fn calculate_ash_hazard_zig(
    volcano: &VolcanoData,
    target: Vector2<f64>,
    wind_direction: f64,
    wind_speed: f64,
    column_height: f64,
) -> f64 {
    unsafe {
        manifest_volcanic_ash_hazard(
            volcano.position.x, volcano.position.y, volcano.vei_scale,
            target.x, target.y,
            wind_direction, wind_speed, column_height,
        )
    }
}

/// Batch calculate distances between plates using Zig
pub fn batch_plate_distances_zig(plates: &[Vector2<f64>]) -> Vec<f64> {
    if plates.is_empty() || plates.len() > 64 {
        return vec![0.0; plates.len() * plates.len()]; // Zig function has limits
    }

    let mut plates_x = Vec::with_capacity(plates.len());
    let mut plates_y = Vec::with_capacity(plates.len());

    for plate in plates {
        plates_x.push(plate.x);
        plates_y.push(plate.y);
    }

    let mut distances = vec![0.0; plates.len() * plates.len()];

    unsafe {
        manifest_batch_plate_distances(
            plates_x.as_ptr(), plates_y.as_ptr(), plates.len(),
            distances.as_mut_ptr(),
        );
    }

    distances
}

/// Test function to verify Zig integration
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ridge_push_calculation() {
        let center = Vector2::new(1000.0, 2000.0);
        let velocity = Vector2::new(0.01, 0.02);
        let age = 50.0;
        let area = 1000000.0;
        let movement_speed = 1.0;

        let force = calculate_ridge_push_zig(center, velocity, age, area, movement_speed);
        
        // Force should be non-zero and reasonable in magnitude
        assert!(force.magnitude() > 0.0);
        assert!(force.magnitude() < 1e15); // Sanity check
    }

    #[test]
    fn test_basal_drag_calculation() {
        let velocity = Vector2::new(0.05, 0.03);
        let area = 2000000.0;

        let drag = calculate_basal_drag_zig(velocity, area);
        
        // Drag should oppose motion
        assert!(drag.dot(&velocity) < 0.0);
    }

    #[test]
    fn test_geometric_calculations() {
        let point = Vector2::new(0.0, 0.0);
        let seg_start = Vector2::new(-10.0, -10.0);
        let seg_end = Vector2::new(10.0, 10.0);

        let distance = point_to_segment_distance_zig(point, seg_start, seg_end);
        
        // Point should be close to the line
        assert!(distance < 1.0);
    }

    #[test]
    fn test_polygon_containment() {
        let square = vec![
            (-1.0, -1.0),
            (1.0, -1.0),
            (1.0, 1.0),
            (-1.0, 1.0),
        ];

        let inside_point = Vector2::new(0.0, 0.0);
        let outside_point = Vector2::new(2.0, 2.0);

        assert!(polygon_contains_point_zig(&square, inside_point));
        assert!(!polygon_contains_point_zig(&square, outside_point));
    }

    #[test]
    fn test_stress_calculations() {
        let stress = StressTensor::new(1e6, 0.5e6, 0.2e6);

        let von_mises = stress.von_mises_stress_zig();
        let max_principal = stress.max_principal_stress_zig();
        let angle = stress.principal_stress_angle_zig();

        assert!(von_mises > 0.0);
        assert!(max_principal >= stress.xx.min(stress.yy));
        assert!(angle >= -std::f64::consts::PI/2.0 && angle <= std::f64::consts::PI/2.0);
    }

    #[test]
    fn test_volcanic_hazard_calculations() {
        let volcano = VolcanoData {
            position: Vector2::new(0.0, 0.0),
            vei_scale: 4,
            hazard_radius: 50000.0,
        };

        let target = Vector2::new(10000.0, 5000.0);
        let wind_direction = 0.0;
        let wind_speed = 20.0;
        let column_height = 25000.0;

        let pyro_hazard = calculate_pyroclastic_hazard_zig(&volcano, target, wind_direction, wind_speed);
        let ash_hazard = calculate_ash_hazard_zig(&volcano, target, wind_direction, wind_speed, column_height);

        assert!(pyro_hazard >= 0.0 && pyro_hazard <= 1.0);
        assert!(ash_hazard >= 0.0 && ash_hazard <= 1.0);
    }

    #[test]
    fn test_batch_distance_calculations() {
        let plates = vec![
            Vector2::new(0.0, 0.0),
            Vector2::new(1000.0, 0.0),
            Vector2::new(0.0, 1000.0),
        ];

        let distances = batch_plate_distances_zig(&plates);

        assert_eq!(distances.len(), 9); // 3x3 matrix
        assert_eq!(distances[0], 0.0); // Distance to self
        assert!(distances[1] > 0.0); // Distance to other plates
        assert!((distances[1] - 1000.0).abs() < 1.0); // Should be ~1000
    }
}
