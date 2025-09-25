//! Error types for the tectonic simulation system
//!
//! Provides structured error handling for all tectonic operations,
//! replacing generic string errors with proper typed errors.

use std::fmt;
use serde::{Serialize, Deserialize};
use crate::core::scheduler::SchedulerError;

/// Comprehensive error type for tectonic simulation operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TectonicError {
    /// Plate generation failed
    PlateGeneration(PlateGenerationError),
    /// Movement calculation failed
    MovementCalculation(MovementError),
    /// Boundary detection failed
    BoundaryDetection(BoundaryError),
    /// Feature generation failed
    FeatureGeneration(FeatureError),
    /// Volcanic system failed
    VolcanicSystem(VolcanicError),
    /// Seismic system failed
    SeismicSystem(SeismicError),
    /// Zig FFI operation failed
    ZigFFI(ZigFFIError),
    /// Configuration error
    Configuration(ConfigError),
    /// Cache operation failed
    Cache(CacheError),
    /// Scheduler error
    Scheduler(SchedulerError),
}

impl fmt::Display for TectonicError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TectonicError::PlateGeneration(e) => write!(f, "Plate generation error: {}", e),
            TectonicError::MovementCalculation(e) => write!(f, "Movement calculation error: {}", e),
            TectonicError::BoundaryDetection(e) => write!(f, "Boundary detection error: {}", e),
            TectonicError::FeatureGeneration(e) => write!(f, "Feature generation error: {}", e),
            TectonicError::VolcanicSystem(e) => write!(f, "Volcanic system error: {}", e),
            TectonicError::SeismicSystem(e) => write!(f, "Seismic system error: {}", e),
            TectonicError::ZigFFI(e) => write!(f, "Zig FFI error: {}", e),
            TectonicError::Configuration(e) => write!(f, "Configuration error: {}", e),
            TectonicError::Cache(e) => write!(f, "Cache error: {}", e),
            TectonicError::Scheduler(e) => write!(f, "Scheduler error: {}", e),
        }
    }
}

impl std::error::Error for TectonicError {}

/// Plate generation specific errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlateGenerationError {
    /// Insufficient seed points generated
    InsufficientSeedPoints { expected: u32, actual: usize },
    /// Triangulation failed
    TriangulationFailed(String),
    /// Voronoi region extraction failed
    VoronoiExtractionFailed,
    /// Empty polygon for plate
    EmptyPolygon { plate_id: u32 },
    /// Invalid plate bounds
    InvalidBounds { min_x: f64, min_y: f64, max_x: f64, max_y: f64 },
    /// Plate area too small
    PlateAreaTooSmall { plate_id: u32, area: f64, minimum: f64 },
}

impl fmt::Display for PlateGenerationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlateGenerationError::InsufficientSeedPoints { expected, actual } => {
                write!(f, "Expected {} seed points, but only generated {}", expected, actual)
            }
            PlateGenerationError::TriangulationFailed(msg) => {
                write!(f, "Triangulation failed: {}", msg)
            }
            PlateGenerationError::VoronoiExtractionFailed => {
                write!(f, "Failed to extract Voronoi regions from triangulation")
            }
            PlateGenerationError::EmptyPolygon { plate_id } => {
                write!(f, "Empty polygon generated for plate {}", plate_id)
            }
            PlateGenerationError::InvalidBounds { min_x, min_y, max_x, max_y } => {
                write!(f, "Invalid world bounds: ({}, {}) to ({}, {})", min_x, min_y, max_x, max_y)
            }
            PlateGenerationError::PlateAreaTooSmall { plate_id, area, minimum } => {
                write!(f, "Plate {} area ({:.2}) is below minimum ({:.2})", plate_id, area, minimum)
            }
        }
    }
}

/// Movement calculation errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MovementError {
    /// No plates provided for movement calculation
    NoPlates,
    /// Force calculation failed
    ForceCalculationFailed { plate_id: u32, reason: String },
    /// Velocity update failed
    VelocityUpdateFailed { plate_id: u32, reason: String },
    /// Invalid plate physics parameters
    InvalidPhysics { plate_id: u32, parameter: String, value: f64 },
}

impl fmt::Display for MovementError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MovementError::NoPlates => write!(f, "No plates provided for movement calculation"),
            MovementError::ForceCalculationFailed { plate_id, reason } => {
                write!(f, "Force calculation failed for plate {}: {}", plate_id, reason)
            }
            MovementError::VelocityUpdateFailed { plate_id, reason } => {
                write!(f, "Velocity update failed for plate {}: {}", plate_id, reason)
            }
            MovementError::InvalidPhysics { plate_id, parameter, value } => {
                write!(f, "Invalid physics parameter '{}' = {} for plate {}", parameter, value, plate_id)
            }
        }
    }
}

/// Boundary detection errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BoundaryError {
    /// Not enough plates for boundary detection
    InsufficientPlates { count: usize, minimum: usize },
    /// Plate not found
    PlateNotFound { plate_id: u32 },
    /// Boundary geometry extraction failed
    GeometryExtractionFailed { plate1_id: u32, plate2_id: u32 },
    /// Invalid boundary length
    InvalidBoundaryLength { boundary_id: u32, length: f64 },
}

impl fmt::Display for BoundaryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BoundaryError::InsufficientPlates { count, minimum } => {
                write!(f, "Need at least {} plates for boundary detection, got {}", minimum, count)
            }
            BoundaryError::PlateNotFound { plate_id } => {
                write!(f, "Plate {} not found", plate_id)
            }
            BoundaryError::GeometryExtractionFailed { plate1_id, plate2_id } => {
                write!(f, "Failed to extract boundary geometry between plates {} and {}", plate1_id, plate2_id)
            }
            BoundaryError::InvalidBoundaryLength { boundary_id, length } => {
                write!(f, "Invalid boundary length {} for boundary {}", length, boundary_id)
            }
        }
    }
}

/// Feature generation errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeatureError {
    /// Mountain range creation failed
    MountainRangeCreation { boundary_id: u32, reason: String },
    /// Rift valley creation failed
    RiftValleyCreation { boundary_id: u32, reason: String },
    /// Transform fault creation failed
    TransformFaultCreation { boundary_id: u32, reason: String },
    /// Feature geometry invalid
    InvalidGeometry { feature_type: String, feature_id: u32 },
    /// Spine generation failed
    SpineGenerationFailed { reason: String },
}

impl fmt::Display for FeatureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FeatureError::MountainRangeCreation { boundary_id, reason } => {
                write!(f, "Mountain range creation failed for boundary {}: {}", boundary_id, reason)
            }
            FeatureError::RiftValleyCreation { boundary_id, reason } => {
                write!(f, "Rift valley creation failed for boundary {}: {}", boundary_id, reason)
            }
            FeatureError::TransformFaultCreation { boundary_id, reason } => {
                write!(f, "Transform fault creation failed for boundary {}: {}", boundary_id, reason)
            }
            FeatureError::InvalidGeometry { feature_type, feature_id } => {
                write!(f, "Invalid geometry for {} {}", feature_type, feature_id)
            }
            FeatureError::SpineGenerationFailed { reason } => {
                write!(f, "Spine generation failed: {}", reason)
            }
        }
    }
}

/// Volcanic system errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VolcanicError {
    /// Volcanic zone creation failed
    ZoneCreationFailed { boundary_id: u32, reason: String },
    /// Volcano generation failed
    VolcanoGenerationFailed { zone_id: u32, reason: String },
    /// Invalid volcanic parameters
    InvalidParameters { parameter: String, value: f64 },
    /// Hotspot generation failed
    HotspotGenerationFailed { reason: String },
}

impl fmt::Display for VolcanicError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VolcanicError::ZoneCreationFailed { boundary_id, reason } => {
                write!(f, "Volcanic zone creation failed for boundary {}: {}", boundary_id, reason)
            }
            VolcanicError::VolcanoGenerationFailed { zone_id, reason } => {
                write!(f, "Volcano generation failed for zone {}: {}", zone_id, reason)
            }
            VolcanicError::InvalidParameters { parameter, value } => {
                write!(f, "Invalid volcanic parameter '{}' = {}", parameter, value)
            }
            VolcanicError::HotspotGenerationFailed { reason } => {
                write!(f, "Hotspot generation failed: {}", reason)
            }
        }
    }
}

/// Seismic system errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SeismicError {
    /// Seismic zone creation failed
    ZoneCreationFailed { boundary_id: u32, reason: String },
    /// Fault generation failed
    FaultGenerationFailed { zone_id: u32, reason: String },
    /// Seismic map creation failed
    SeismicMapCreationFailed { reason: String },
    /// Invalid seismic parameters
    InvalidParameters { parameter: String, value: f64 },
}

impl fmt::Display for SeismicError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SeismicError::ZoneCreationFailed { boundary_id, reason } => {
                write!(f, "Seismic zone creation failed for boundary {}: {}", boundary_id, reason)
            }
            SeismicError::FaultGenerationFailed { zone_id, reason } => {
                write!(f, "Fault generation failed for zone {}: {}", zone_id, reason)
            }
            SeismicError::SeismicMapCreationFailed { reason } => {
                write!(f, "Seismic map creation failed: {}", reason)
            }
            SeismicError::InvalidParameters { parameter, value } => {
                write!(f, "Invalid seismic parameter '{}' = {}", parameter, value)
            }
        }
    }
}

/// Zig FFI errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ZigFFIError {
    /// FFI call failed
    FFICallFailed { function: String, reason: String },
    /// Invalid parameters passed to FFI
    InvalidFFIParameters { function: String, parameter: String },
    /// FFI result validation failed
    ResultValidationFailed { function: String, result: String },
}

impl fmt::Display for ZigFFIError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ZigFFIError::FFICallFailed { function, reason } => {
                write!(f, "FFI call to '{}' failed: {}", function, reason)
            }
            ZigFFIError::InvalidFFIParameters { function, parameter } => {
                write!(f, "Invalid parameter '{}' for FFI function '{}'", parameter, function)
            }
            ZigFFIError::ResultValidationFailed { function, result } => {
                write!(f, "FFI function '{}' returned invalid result: {}", function, result)
            }
        }
    }
}

/// Configuration errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConfigError {
    /// Invalid configuration parameter
    InvalidParameter { name: String, value: String, expected: String },
    /// Missing required configuration
    MissingConfig { name: String },
    /// Configuration validation failed
    ValidationFailed { reason: String },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::InvalidParameter { name, value, expected } => {
                write!(f, "Invalid config parameter '{}' = '{}', expected {}", name, value, expected)
            }
            ConfigError::MissingConfig { name } => {
                write!(f, "Missing required configuration: {}", name)
            }
            ConfigError::ValidationFailed { reason } => {
                write!(f, "Configuration validation failed: {}", reason)
            }
        }
    }
}

/// Cache operation errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CacheError {
    /// Cache miss for required data
    CacheMiss { key: String },
    /// Cache serialization failed
    SerializationFailed { key: String, reason: String },
    /// Cache deserialization failed
    DeserializationFailed { key: String, reason: String },
    /// Cache capacity exceeded
    CapacityExceeded { current: usize, maximum: usize },
}

impl fmt::Display for CacheError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CacheError::CacheMiss { key } => {
                write!(f, "Cache miss for key: {}", key)
            }
            CacheError::SerializationFailed { key, reason } => {
                write!(f, "Cache serialization failed for key '{}': {}", key, reason)
            }
            CacheError::DeserializationFailed { key, reason } => {
                write!(f, "Cache deserialization failed for key '{}': {}", key, reason)
            }
            CacheError::CapacityExceeded { current, maximum } => {
                write!(f, "Cache capacity exceeded: {} / {} items", current, maximum)
            }
        }
    }
}

/// Helper functions for creating common errors
impl TectonicError {
    /// Create a plate generation error
    pub fn plate_generation(error: PlateGenerationError) -> Self {
        TectonicError::PlateGeneration(error)
    }

    /// Create a movement calculation error
    pub fn movement_calculation(error: MovementError) -> Self {
        TectonicError::MovementCalculation(error)
    }

    /// Create a boundary detection error
    pub fn boundary_detection(error: BoundaryError) -> Self {
        TectonicError::BoundaryDetection(error)
    }

    /// Create a feature generation error
    pub fn feature_generation(error: FeatureError) -> Self {
        TectonicError::FeatureGeneration(error)
    }

    /// Create a volcanic system error
    pub fn volcanic_system(error: VolcanicError) -> Self {
        TectonicError::VolcanicSystem(error)
    }

    /// Create a seismic system error
    pub fn seismic_system(error: SeismicError) -> Self {
        TectonicError::SeismicSystem(error)
    }

    /// Convert from SchedulerError
    pub fn from_scheduler_error(error: SchedulerError) -> Self {
        TectonicError::Scheduler(error)
    }
}

/// Result type for tectonic operations
pub type TectonicResult<T> = Result<T, TectonicError>;
