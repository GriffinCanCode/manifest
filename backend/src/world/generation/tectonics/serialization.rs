//! Optimized serialization for tectonic data structures
//!
//! Provides space-efficient and performance-optimized serialization
//! for large tectonic simulation results using bincode and compression.

use std::io::{Read, Write};
use serde::{Serialize, Deserialize};
use bincode::{self, Options};
use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use super::{TectonicResult, TectonicPlate, PlateBoundary, MountainRange, VolcanicZone, EarthquakeZone};
use crate::core::hashing::FastHasher;

/// Compressed serialization format for tectonic results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedTectonicResult {
    /// Metadata about the compression
    pub metadata: CompressionMetadata,
    /// Compressed data
    pub compressed_data: Vec<u8>,
}

/// Metadata about compressed tectonic data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionMetadata {
    /// Original data size in bytes
    pub original_size: usize,
    /// Compressed data size in bytes
    pub compressed_size: usize,
    /// Compression ratio achieved
    pub compression_ratio: f64,
    /// Hash of original data for integrity checking
    pub data_hash: u64,
    /// Compression format used
    pub format: CompressionFormat,
    /// Serialization format used
    pub serialization_format: SerializationFormat,
}

/// Available compression formats
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CompressionFormat {
    None,
    Gzip,
    Zstd,
    Lz4,
}

/// Available serialization formats
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SerializationFormat {
    Bincode,
    MessagePack,
    Postcard,
}

/// Optimized serialization trait for tectonic data
pub trait TectonicSerialize: Sized {
    /// Serialize to bytes with optimization
    fn serialize_optimized(&self) -> Result<Vec<u8>, TectonicSerializationError>;
    
    /// Serialize with compression
    fn serialize_compressed(&self, format: CompressionFormat) -> Result<CompressedTectonicResult, TectonicSerializationError>;
    
    /// Deserialize from bytes
    fn deserialize_optimized(data: &[u8]) -> Result<Self, TectonicSerializationError>;
    
    /// Deserialize from compressed format
    fn deserialize_compressed(compressed: &CompressedTectonicResult) -> Result<Self, TectonicSerializationError>;
    
    /// Calculate serialized size estimate
    fn size_estimate(&self) -> usize;
}

/// Serialization errors
#[derive(Debug, Clone)]
pub enum TectonicSerializationError {
    SerializationFailed(String),
    DeserializationFailed(String),
    CompressionFailed(String),
    DecompressionFailed(String),
    IntegrityCheckFailed { expected: u64, actual: u64 },
    UnsupportedFormat(String),
}

impl std::fmt::Display for TectonicSerializationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TectonicSerializationError::SerializationFailed(msg) => write!(f, "Serialization failed: {}", msg),
            TectonicSerializationError::DeserializationFailed(msg) => write!(f, "Deserialization failed: {}", msg),
            TectonicSerializationError::CompressionFailed(msg) => write!(f, "Compression failed: {}", msg),
            TectonicSerializationError::DecompressionFailed(msg) => write!(f, "Decompression failed: {}", msg),
            TectonicSerializationError::IntegrityCheckFailed { expected, actual } => {
                write!(f, "Integrity check failed: expected hash {}, got {}", expected, actual)
            }
            TectonicSerializationError::UnsupportedFormat(format) => {
                write!(f, "Unsupported format: {}", format)
            }
        }
    }
}

impl std::error::Error for TectonicSerializationError {}

/// High-performance bincode configuration
pub fn optimized_bincode_config() -> impl Options {
    bincode::DefaultOptions::new()
        .with_big_endian()
        .with_fixint_encoding()
        .allow_trailing_bytes()
}

/// Compress data using specified format
pub fn compress_data(data: &[u8], format: CompressionFormat) -> Result<Vec<u8>, TectonicSerializationError> {
    match format {
        CompressionFormat::None => Ok(data.to_vec()),
        CompressionFormat::Gzip => {
            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(data)
                .map_err(|e| TectonicSerializationError::CompressionFailed(e.to_string()))?;
            encoder.finish()
                .map_err(|e| TectonicSerializationError::CompressionFailed(e.to_string()))
        }
        CompressionFormat::Zstd => {
            zstd::bulk::compress(data, 3)
                .map_err(|e| TectonicSerializationError::CompressionFailed(e.to_string()))
        }
        CompressionFormat::Lz4 => {
            Ok(lz4_flex::compress_prepend_size(data))
        }
    }
}

/// Decompress data using specified format
pub fn decompress_data(data: &[u8], format: CompressionFormat) -> Result<Vec<u8>, TectonicSerializationError> {
    match format {
        CompressionFormat::None => Ok(data.to_vec()),
        CompressionFormat::Gzip => {
            let mut decoder = GzDecoder::new(data);
            let mut result = Vec::new();
            decoder.read_to_end(&mut result)
                .map_err(|e| TectonicSerializationError::DecompressionFailed(e.to_string()))?;
            Ok(result)
        }
        CompressionFormat::Zstd => {
            zstd::bulk::decompress(data, 100 * 1024 * 1024) // 100MB limit
                .map_err(|e| TectonicSerializationError::DecompressionFailed(e.to_string()))
        }
        CompressionFormat::Lz4 => {
            lz4_flex::decompress_size_prepended(data)
                .map_err(|e| TectonicSerializationError::DecompressionFailed(e.to_string()))
        }
    }
}

/// Implementation for TectonicResult
impl TectonicSerialize for TectonicResult {
    fn serialize_optimized(&self) -> Result<Vec<u8>, TectonicSerializationError> {
        optimized_bincode_config().serialize(self)
            .map_err(|e| TectonicSerializationError::SerializationFailed(e.to_string()))
    }

    fn serialize_compressed(&self, format: CompressionFormat) -> Result<CompressedTectonicResult, TectonicSerializationError> {
        let original_data = self.serialize_optimized()?;
        let original_size = original_data.len();
        let data_hash = FastHasher::hash_one(&original_data);
        
        let compressed_data = compress_data(&original_data, format)?;
        let compressed_size = compressed_data.len();
        let compression_ratio = original_size as f64 / compressed_size as f64;

        Ok(CompressedTectonicResult {
            metadata: CompressionMetadata {
                original_size,
                compressed_size,
                compression_ratio,
                data_hash,
                format,
                serialization_format: SerializationFormat::Bincode,
            },
            compressed_data,
        })
    }

    fn deserialize_optimized(data: &[u8]) -> Result<Self, TectonicSerializationError> {
        optimized_bincode_config().deserialize(data)
            .map_err(|e| TectonicSerializationError::DeserializationFailed(e.to_string()))
    }

    fn deserialize_compressed(compressed: &CompressedTectonicResult) -> Result<Self, TectonicSerializationError> {
        let decompressed_data = decompress_data(&compressed.compressed_data, compressed.metadata.format)?;
        
        // Verify integrity
        let actual_hash = FastHasher::hash_one(&decompressed_data);
        if actual_hash != compressed.metadata.data_hash {
            return Err(TectonicSerializationError::IntegrityCheckFailed {
                expected: compressed.metadata.data_hash,
                actual: actual_hash,
            });
        }
        
        Self::deserialize_optimized(&decompressed_data)
    }

    fn size_estimate(&self) -> usize {
        // Rough estimates based on data structure sizes
        let plates_size = self.plates.len() * std::mem::size_of::<TectonicPlate>();
        let boundaries_size = self.boundaries.len() * std::mem::size_of::<PlateBoundary>();
        let mountains_size = self.mountain_ranges.len() * std::mem::size_of::<MountainRange>();
        let volcanic_size = self.volcanic_zones.len() * std::mem::size_of::<VolcanicZone>();
        let seismic_size = self.earthquake_zones.len() * std::mem::size_of::<EarthquakeZone>();
        
        plates_size + boundaries_size + mountains_size + volcanic_size + seismic_size + 1024 // Extra for metadata
    }
}

/// Specialized optimized formats for individual components
pub struct TectonicComponentSerializer;

impl TectonicComponentSerializer {
    /// Serialize plates with delta compression for positions
    pub fn serialize_plates_optimized(plates: &[TectonicPlate]) -> Result<Vec<u8>, TectonicSerializationError> {
        // Use delta compression for plate centers and similar values
        let mut optimized_plates = Vec::new();
        
        if !plates.is_empty() {
            // First plate is reference
            optimized_plates.push(DeltaCompressedPlate::from_reference(&plates[0]));
            
            // Subsequent plates are deltas from previous
            for i in 1..plates.len() {
                optimized_plates.push(DeltaCompressedPlate::from_delta(&plates[i], &plates[i-1]));
            }
        }
        
        optimized_bincode_config().serialize(&optimized_plates)
            .map_err(|e| TectonicSerializationError::SerializationFailed(e.to_string()))
    }
    
    /// Serialize boundaries with shared geometry optimization
    pub fn serialize_boundaries_optimized(boundaries: &[PlateBoundary]) -> Result<Vec<u8>, TectonicSerializationError> {
        // Extract unique geometry segments and reference them
        let optimized = OptimizedBoundarySet::from_boundaries(boundaries);
        
        optimized_bincode_config().serialize(&optimized)
            .map_err(|e| TectonicSerializationError::SerializationFailed(e.to_string()))
    }
}

/// Delta-compressed plate representation
#[derive(Serialize, Deserialize)]
struct DeltaCompressedPlate {
    pub id: u32,
    pub center_delta: (i32, i32), // Delta from reference in fixed-point
    pub velocity_delta: (i16, i16), // Delta in milliunits
    pub age_delta: i16, // Delta in years
    pub plate_type: super::PlateType,
    pub density_delta: i16, // Delta in kg/m³
    pub area_delta: i32, // Delta in km²
    pub boundary_points_compressed: Vec<u8>, // Compressed boundary points
    pub polygon_compressed: Vec<u8>, // Compressed polygon data
}

impl DeltaCompressedPlate {
    fn from_reference(plate: &TectonicPlate) -> Self {
        Self {
            id: plate.id,
            center_delta: (0, 0),
            velocity_delta: (0, 0),
            age_delta: 0,
            plate_type: plate.plate_type,
            density_delta: 0,
            area_delta: 0,
            boundary_points_compressed: compress_points(&plate.boundary_points),
            polygon_compressed: compress_polygon(&plate.polygon),
        }
    }
    
    fn from_delta(plate: &TectonicPlate, reference: &TectonicPlate) -> Self {
        let center_delta = (
            ((plate.center.x - reference.center.x) * 1000.0) as i32,
            ((plate.center.y - reference.center.y) * 1000.0) as i32,
        );
        
        let velocity_delta = (
            ((plate.velocity.x - reference.velocity.x) * 10000.0) as i16,
            ((plate.velocity.y - reference.velocity.y) * 10000.0) as i16,
        );
        
        Self {
            id: plate.id,
            center_delta,
            velocity_delta,
            age_delta: (plate.age_million_years - reference.age_million_years) as i16,
            plate_type: plate.plate_type,
            density_delta: (plate.density - reference.density) as i16,
            area_delta: (plate.area - reference.area) as i32,
            boundary_points_compressed: compress_points(&plate.boundary_points),
            polygon_compressed: compress_polygon(&plate.polygon),
        }
    }
}

/// Optimized boundary set with shared geometry
#[derive(Serialize, Deserialize)]
struct OptimizedBoundarySet {
    pub shared_geometry: Vec<nalgebra::Vector2<f64>>,
    pub boundaries: Vec<OptimizedBoundary>,
}

#[derive(Serialize, Deserialize)]
struct OptimizedBoundary {
    pub id: u32,
    pub plate1_id: u32,
    pub plate2_id: u32,
    pub boundary_type: super::BoundaryType,
    pub geometry_indices: Vec<u16>, // Indices into shared_geometry
    pub length: f32, // Reduced precision
    pub relative_velocity: f32,
    pub stress_magnitude: f32,
    pub last_activity: f32,
}

impl OptimizedBoundarySet {
    fn from_boundaries(boundaries: &[PlateBoundary]) -> Self {
        // This would implement geometry deduplication
        // For now, simplified version
        let mut shared_geometry = Vec::new();
        let mut optimized_boundaries = Vec::new();
        
        for boundary in boundaries {
            let start_index = shared_geometry.len() as u16;
            shared_geometry.extend_from_slice(&boundary.geometry);
            let end_index = shared_geometry.len() as u16;
            
            optimized_boundaries.push(OptimizedBoundary {
                id: boundary.id,
                plate1_id: boundary.plate1_id,
                plate2_id: boundary.plate2_id,
                boundary_type: boundary.boundary_type,
                geometry_indices: (start_index..end_index).collect(),
                length: boundary.length as f32,
                relative_velocity: boundary.relative_velocity as f32,
                stress_magnitude: boundary.stress_magnitude as f32,
                last_activity: boundary.last_activity as f32,
            });
        }
        
        Self {
            shared_geometry,
            boundaries: optimized_boundaries,
        }
    }
}

/// Helper functions for compression
fn compress_points(points: &[nalgebra::Vector2<f64>]) -> Vec<u8> {
    // Simple delta compression of points
    if points.is_empty() {
        return Vec::new();
    }
    
    let mut compressed = Vec::new();
    let mut last_point = points[0];
    
    for point in points.iter().skip(1) {
        let delta_x = ((point.x - last_point.x) * 1000.0) as i32;
        let delta_y = ((point.y - last_point.y) * 1000.0) as i32;
        
        compressed.extend_from_slice(&delta_x.to_le_bytes());
        compressed.extend_from_slice(&delta_y.to_le_bytes());
        
        last_point = *point;
    }
    
    compressed
}

fn compress_polygon(polygon: &[(f64, f64)]) -> Vec<u8> {
    // Similar compression for polygon data
    if polygon.is_empty() {
        return Vec::new();
    }
    
    let mut compressed = Vec::new();
    let mut last_point = polygon[0];
    
    for point in polygon.iter().skip(1) {
        let delta_x = ((point.0 - last_point.0) * 1000.0) as i32;
        let delta_y = ((point.1 - last_point.1) * 1000.0) as i32;
        
        compressed.extend_from_slice(&delta_x.to_le_bytes());
        compressed.extend_from_slice(&delta_y.to_le_bytes());
        
        last_point = *point;
    }
    
    compressed
}

/// Benchmarking utilities for serialization performance
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_compression_ratios() {
        // This would test different compression formats
    }

    #[test]
    fn benchmark_serialization() {
        // Benchmark different serialization approaches
    }
}
