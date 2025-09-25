//! Interpolation system for smooth animations between simulation ticks
//!
//! Provides nalgebra-based interpolation for positions, rotations, colors, and
//! other properties to achieve smooth 60fps rendering from lower-tick simulations.

use nalgebra::{
    Point2, Point3, Vector2, Vector3, Unit,
    UnitQuaternion, Isometry2, Isometry3
};
use crate::core::time::DeterministicFloat;
use crate::core::zig_ffi::{simd_add_4, simd_mul_4, simd_dot_4, Vec4};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::Hash;

/// Interpolation factor type (0.0 = previous state, 1.0 = current state)
pub type InterpolationFactor = DeterministicFloat;

/// Create interpolation factor from f32
pub fn lerp_factor(t: f32) -> InterpolationFactor {
    crate::core::time::det_f32(t.clamp(0.0, 1.0))
}

/// Generic interpolation trait for any type that can be smoothly blended
pub trait Interpolate {
    /// Interpolate between self (previous) and other (current) by factor t
    fn interpolate(&self, other: &Self, t: InterpolationFactor) -> Self;
}

/// 2D position interpolation
impl Interpolate for Point2<f32> {
    fn interpolate(&self, other: &Self, t: InterpolationFactor) -> Self {
        self.lerp(other, t.into_inner())
    }
}

/// 3D position interpolation  
impl Interpolate for Point3<f32> {
    fn interpolate(&self, other: &Self, t: InterpolationFactor) -> Self {
        self.lerp(other, t.into_inner())
    }
}

/// 2D vector interpolation
impl Interpolate for Vector2<f32> {
    fn interpolate(&self, other: &Self, t: InterpolationFactor) -> Self {
        self.lerp(other, t.into_inner())
    }
}

/// 3D vector interpolation
impl Interpolate for Vector3<f32> {
    fn interpolate(&self, other: &Self, t: InterpolationFactor) -> Self {
        self.lerp(other, t.into_inner())
    }
}

/// Quaternion rotation interpolation (spherical linear interpolation)
impl Interpolate for UnitQuaternion<f32> {
    fn interpolate(&self, other: &Self, t: InterpolationFactor) -> Self {
        self.slerp(other, t.into_inner())
    }
}

/// 2D rotation interpolation (complex number unit circle)
impl Interpolate for Unit<nalgebra::Complex<f32>> {
    fn interpolate(&self, other: &Self, t: InterpolationFactor) -> Self {
        self.slerp(other, t.into_inner())
    }
}

/// 2D transform interpolation
impl Interpolate for Isometry2<f32> {
    fn interpolate(&self, other: &Self, t: InterpolationFactor) -> Self {
        let translation = self.translation.vector.interpolate(&other.translation.vector, t);
        let rotation = self.rotation.interpolate(&other.rotation, t);
        
        Isometry2::from_parts(translation.into(), rotation)
    }
}

/// 3D transform interpolation
impl Interpolate for Isometry3<f32> {
    fn interpolate(&self, other: &Self, t: InterpolationFactor) -> Self {
        let translation = self.translation.vector.interpolate(&other.translation.vector, t);
        let rotation = self.rotation.interpolate(&other.rotation, t);
        
        Isometry3::from_parts(translation.into(), rotation)
    }
}

/// Color interpolation (RGB)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
    
    pub fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self::new(r, g, b, 1.0)
    }
}

impl Interpolate for Color {
    fn interpolate(&self, other: &Self, t: InterpolationFactor) -> Self {
        // Use SIMD for faster color interpolation
        simd_lerp_color(*self, *other, t.into_inner())
    }
}

/// Scalar value interpolation
impl Interpolate for f32 {
    fn interpolate(&self, other: &Self, t: InterpolationFactor) -> Self {
        self.lerp(*other, t.into_inner())
    }
}

/// Linear interpolation for f32
trait LerpExt {
    fn lerp(self, other: Self, t: f32) -> Self;
}

impl LerpExt for f32 {
    fn lerp(self, other: Self, t: f32) -> Self {
        self + t * (other - self)
    }
}

/// SIMD-optimized interpolation functions
pub mod simd_interp {
    use super::*;
    
    /// SIMD-optimized color interpolation
    pub fn simd_lerp_color(a: Color, b: Color, t: f32) -> Color {
        let vec_a = Vec4::new(a.r, a.g, a.b, a.a);
        let vec_b = Vec4::new(b.r, b.g, b.b, b.a);
        let vec_t = Vec4::new(t, t, t, t);
        let one_minus_t = Vec4::new(1.0 - t, 1.0 - t, 1.0 - t, 1.0 - t);
        
        // SIMD: result = a * (1 - t) + b * t
        let a_scaled = simd_mul_4(vec_a, one_minus_t);
        let b_scaled = simd_mul_4(vec_b, vec_t);
        let result = simd_add_4(a_scaled, b_scaled);
        
        Color::new(result.x, result.y, result.z, result.w)
    }
    
    /// SIMD-optimized 4-element vector interpolation
    pub fn simd_lerp_vec4(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
        let vec_a = Vec4::new(a[0], a[1], a[2], a[3]);
        let _vec_b = Vec4::new(b[0], b[1], b[2], b[3]);
        let vec_t = Vec4::new(t, t, t, t);
        let one_minus_t = Vec4::new(1.0 - t, 1.0 - t, 1.0 - t, 1.0 - t);
        
        let a_scaled = simd_mul_4(vec_a, one_minus_t);
        let b_scaled = simd_mul_4(_vec_b, vec_t);
        let result = simd_add_4(a_scaled, b_scaled);
        
        [result.x, result.y, result.z, result.w]
    }
    
    /// SIMD-optimized distance calculation for 4D vectors
    pub fn simd_distance_4d(a: [f32; 4], b: [f32; 4]) -> f32 {
        let vec_a = Vec4::new(a[0], a[1], a[2], a[3]);
        let vec_b = Vec4::new(b[0], b[1], b[2], b[3]);
        let diff = simd_add_4(vec_a, Vec4::new(-b[0], -b[1], -b[2], -b[3]));
        
        // Dot product gives us squared distance
        simd_dot_4(diff, diff).sqrt()
    }
}

// Re-export SIMD functions for easy access
pub use simd_interp::*;

/// Interpolated property that tracks previous and current values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterpolatedProperty<T: Interpolate + Clone> {
    /// Previous tick value
    pub previous: T,
    /// Current tick value  
    pub current: T,
    /// Has been updated this tick
    pub updated: bool,
}

impl<T: Interpolate + Clone> InterpolatedProperty<T> {
    /// Create new interpolated property
    pub fn new(initial_value: T) -> Self {
        Self {
            previous: initial_value.clone(),
            current: initial_value,
            updated: false,
        }
    }

    /// Update to new value (call once per simulation tick)
    pub fn update(&mut self, delta_time: std::time::Duration) {
        // For now, just mark as updated. Animation logic would go here.
        self.updated = true;
    }

    /// Set a new current value (moves current to previous)
    pub fn set_value(&mut self, new_value: T) {
        self.previous = self.current.clone();
        self.current = new_value;
        self.updated = true;
    }

    /// Get interpolated value for rendering
    pub fn interpolate(&self, factor: InterpolationFactor) -> T {
        self.previous.interpolate(&self.current, factor)
    }

    /// Get current (latest) value
    pub fn current(&self) -> T {
        self.current.clone()
    }

    /// Get previous value
    pub fn previous(&self) -> &T {
        &self.previous
    }

    /// Start animation to target value
    pub fn animate_to(&mut self, target: T, _duration: std::time::Duration) {
        self.previous = self.current.clone();
        self.current = target;
        self.updated = true;
    }

    /// Get target value (same as current for now)
    pub fn target(&self) -> T {
        self.current.clone()
    }

    /// Check if currently animating
    pub fn is_animating(&self) -> bool {
        // For now, always false. Would track animation state in full implementation.
        false
    }

    /// Stop animation immediately
    pub fn stop(&mut self) {
        // No-op for now
    }

    /// Snap to target immediately
    pub fn snap_to_target(&mut self) {
        // No-op for now since target == current
    }

    /// Set immediate value without animation
    pub fn set_immediate(&mut self, value: T) {
        self.previous = value.clone();
        self.current = value;
        self.updated = true;
    }
}

/// Interpolation manager for handling multiple entities/properties
pub struct InterpolationManager<K: Hash + Eq + Clone> {
    /// Map of entity/property to interpolated values
    properties: HashMap<K, Box<dyn InterpolateProperty + Send + Sync>>,
    /// Current interpolation factor
    current_factor: InterpolationFactor,
    /// Statistics
    stats: InterpolationStats,
}

impl<K: Hash + Eq + Clone> std::fmt::Debug for InterpolationManager<K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InterpolationManager")
            .field("property_count", &self.properties.len())
            .field("current_factor", &self.current_factor)
            .field("stats", &self.stats)
            .finish()
    }
}

impl<K: Hash + Eq + Clone> InterpolationManager<K> {
    /// Create new interpolation manager
    pub fn new() -> Self {
        Self {
            properties: HashMap::new(),
            current_factor: lerp_factor(0.0),
            stats: InterpolationStats::default(),
        }
    }

    /// Register interpolated property
    pub fn register<T: Interpolate + Clone + Send + Sync + 'static>(
        &mut self,
        key: K,
        initial_value: T,
    ) {
        let property = Box::new(InterpolatedProperty::new(initial_value));
        self.properties.insert(key, property);
        self.stats.total_properties += 1;
    }

    /// Update property value (call during simulation tick)
    pub fn update<T: Interpolate + Clone + Send + Sync + 'static>(
        &mut self,
        key: &K,
        value: T,
    ) -> Result<(), InterpolationError> {
        if let Some(property) = self.properties.get_mut(key) {
            if let Some(typed_property) = property.as_any_mut().downcast_mut::<InterpolatedProperty<T>>() {
                typed_property.set_value(value);
                self.stats.updates_this_frame += 1;
                Ok(())
            } else {
                Err(InterpolationError::TypeMismatch)
            }
        } else {
            Err(InterpolationError::PropertyNotFound)
        }
    }

    /// Get interpolated value for rendering
    pub fn interpolate<T: Interpolate + Clone + Send + Sync + 'static>(
        &self,
        key: &K,
    ) -> Result<T, InterpolationError> {
        if let Some(property) = self.properties.get(key) {
            if let Some(typed_property) = property.as_any().downcast_ref::<InterpolatedProperty<T>>() {
                Ok(typed_property.interpolate(self.current_factor))
            } else {
                Err(InterpolationError::TypeMismatch)
            }
        } else {
            Err(InterpolationError::PropertyNotFound)
        }
    }

    /// Set global interpolation factor (usually based on time since last tick)
    pub fn set_factor(&mut self, factor: InterpolationFactor) {
        self.current_factor = factor;
        self.stats.factor_updates += 1;
    }

    /// Clear all properties
    pub fn clear(&mut self) {
        self.properties.clear();
        self.stats.total_properties = 0;
    }

    /// Get statistics
    pub fn stats(&self) -> &InterpolationStats {
        &self.stats
    }

    /// Reset frame statistics
    pub fn new_frame(&mut self) {
        self.stats.updates_this_frame = 0;
    }
}

/// Trait for type-erased interpolated properties
trait InterpolateProperty {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

impl<T: Interpolate + Clone + Send + Sync + 'static> InterpolateProperty for InterpolatedProperty<T> {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Interpolation statistics
#[derive(Debug, Default, Clone)]
pub struct InterpolationStats {
    /// Total registered properties
    pub total_properties: usize,
    /// Updates this frame
    pub updates_this_frame: usize,
    /// Total factor updates
    pub factor_updates: u64,
}

/// Interpolation errors
#[derive(Debug, thiserror::Error)]
pub enum InterpolationError {
    #[error("Property not found")]
    PropertyNotFound,
    #[error("Type mismatch - wrong interpolation type")]
    TypeMismatch,
}

/// Commonly used interpolation helper functions
pub mod helpers {
    use super::*;
    
    /// Smooth step interpolation (ease in/out)
    pub fn smooth_step(t: InterpolationFactor) -> InterpolationFactor {
        let t = t.into_inner();
        let smooth = t * t * (3.0 - 2.0 * t);
        lerp_factor(smooth)
    }
    
    /// Ease in interpolation (accelerating)
    pub fn ease_in(t: InterpolationFactor) -> InterpolationFactor {
        let t = t.into_inner();
        lerp_factor(t * t)
    }
    
    /// Ease out interpolation (decelerating) 
    pub fn ease_out(t: InterpolationFactor) -> InterpolationFactor {
        let t = t.into_inner();
        lerp_factor(1.0 - (1.0 - t) * (1.0 - t))
    }
    
    /// Bounce interpolation
    pub fn bounce(t: InterpolationFactor) -> InterpolationFactor {
        let t = t.into_inner();
        let bounce = (t * std::f32::consts::PI).sin().abs();
        lerp_factor(bounce)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nalgebra::{Point2, Vector3};

    #[test]
    fn test_point_interpolation() {
        let p1 = Point2::new(0.0, 0.0);
        let p2 = Point2::new(10.0, 20.0);
        
        let result = p1.interpolate(&p2, lerp_factor(0.5));
        assert_eq!(result, Point2::new(5.0, 10.0));
    }

    #[test]
    fn test_color_interpolation() {
        let c1 = Color::rgb(0.0, 0.0, 0.0);
        let c2 = Color::rgb(1.0, 1.0, 1.0);
        
        let result = c1.interpolate(&c2, lerp_factor(0.5));
        assert_eq!(result, Color::rgb(0.5, 0.5, 0.5));
    }

    #[test]
    fn test_interpolated_property() {
        let mut prop = InterpolatedProperty::new(Point2::new(0.0, 0.0));
        
        prop.update(Point2::new(10.0, 10.0));
        
        let result = prop.interpolate(lerp_factor(0.5));
        assert_eq!(result, Point2::new(5.0, 5.0));
    }

    #[test]
    fn test_interpolation_manager() {
        let mut manager = InterpolationManager::new();
        
        manager.register("position", Point2::new(0.0, 0.0));
        manager.update(&"position", Point2::new(10.0, 10.0)).unwrap();
        manager.set_factor(lerp_factor(0.5));
        
        let result: Point2<f32> = manager.interpolate(&"position").unwrap();
        assert_eq!(result, Point2::new(5.0, 5.0));
    }

    #[test]
    fn test_quaternion_interpolation() {
        let q1 = UnitQuaternion::identity();
        let q2 = UnitQuaternion::from_euler_angles(0.0, 0.0, std::f32::consts::PI);
        
        let result = q1.interpolate(&q2, lerp_factor(0.5));
        
        // Result should be halfway rotation
        assert!((result.angle() - std::f32::consts::PI / 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_smooth_step() {
        // Smooth step should have zero derivative at endpoints
        let t1 = helpers::smooth_step(lerp_factor(0.0));
        let t2 = helpers::smooth_step(lerp_factor(0.5));
        let t3 = helpers::smooth_step(lerp_factor(1.0));
        
        assert_eq!(t1.into_inner(), 0.0);
        assert_eq!(t2.into_inner(), 0.5);
        assert_eq!(t3.into_inner(), 1.0);
    }
}
