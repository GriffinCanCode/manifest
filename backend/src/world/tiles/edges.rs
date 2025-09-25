//! Edge detection with image crate algorithms for tile boundaries
//!
//! Provides sophisticated edge detection for tile-based systems using image
//! processing algorithms to identify terrain boundaries, political borders,
//! and other significant transitions between tiles.

use image::{ImageBuffer, Luma, GrayImage, Pixel};
use ndarray::{Array2, ArrayView2, s};
use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use rayon::prelude::*;

use crate::core::{
    zig_ffi::HexCoord,
    hashing::{FastHashMap, FastHashSet},
    caching::{GameCache, GameCacheBuilder, CacheKey, CachePriority}
};
use crate::world::tiles::{
    chunks::{TileId, ChunkCoord, ChunkManager, CHUNK_SIZE},
    components::{Tile, TerrainType, TileComponentManager},
    adjacency::{TileAdjacencyGraph, HexDirection}
};
use tracing::{debug, instrument, warn};

/// Types of edges that can be detected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum EdgeType {
    /// Terrain boundary (forest to grassland, etc.)
    TerrainBoundary = 0,
    /// Elevation change (cliff, slope)
    ElevationChange = 1,
    /// Political border (nation, province)
    PoliticalBorder = 2,
    /// Cultural boundary (different cultures)
    CulturalBoundary = 3,
    /// Climate zone transition
    ClimateTransition = 4,
    /// Resource deposit edge
    ResourceBoundary = 5,
    /// River bank
    Riverbank = 6,
    /// Coastline (land to water)
    Coastline = 7,
}

impl EdgeType {
    /// Get all edge types
    pub const ALL: [EdgeType; 8] = [
        EdgeType::TerrainBoundary,
        EdgeType::ElevationChange,
        EdgeType::PoliticalBorder,
        EdgeType::CulturalBoundary,
        EdgeType::ClimateTransition,
        EdgeType::ResourceBoundary,
        EdgeType::Riverbank,
        EdgeType::Coastline,
    ];

    /// Get edge strength threshold (0.0 to 1.0)
    pub fn strength_threshold(self) -> f32 {
        match self {
            EdgeType::TerrainBoundary => 0.3,
            EdgeType::ElevationChange => 0.4,
            EdgeType::PoliticalBorder => 0.2,
            EdgeType::CulturalBoundary => 0.25,
            EdgeType::ClimateTransition => 0.35,
            EdgeType::ResourceBoundary => 0.3,
            EdgeType::Riverbank => 0.5,
            EdgeType::Coastline => 0.6,
        }
    }
}

/// Detected edge between two tiles
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TileEdge {
    /// Source tile ID
    pub from_tile: TileId,
    /// Target tile ID
    pub to_tile: TileId,
    /// Direction of edge from source
    pub direction: HexDirection,
    /// Type of edge detected
    pub edge_type: EdgeType,
    /// Strength of edge (0.0 = no edge, 1.0 = strong edge)
    pub strength: f32,
    /// Additional properties of the edge
    pub properties: EdgeProperties,
}

impl TileEdge {
    /// Create new tile edge
    pub fn new(from_tile: TileId, to_tile: TileId, direction: HexDirection, edge_type: EdgeType, strength: f32) -> Self {
        Self {
            from_tile,
            to_tile,
            direction,
            edge_type,
            strength: strength.clamp(0.0, 1.0),
            properties: EdgeProperties::default(),
        }
    }

    /// Check if edge is significant (above threshold)
    pub fn is_significant(&self) -> bool {
        self.strength >= self.edge_type.strength_threshold()
    }

    /// Get edge intensity category
    pub fn intensity(&self) -> EdgeIntensity {
        if self.strength >= 0.8 { EdgeIntensity::VeryStrong }
        else if self.strength >= 0.6 { EdgeIntensity::Strong }
        else if self.strength >= 0.4 { EdgeIntensity::Moderate }
        else if self.strength >= 0.2 { EdgeIntensity::Weak }
        else { EdgeIntensity::VeryWeak }
    }
}

/// Edge intensity categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeIntensity {
    VeryWeak,
    Weak,
    Moderate,
    Strong,
    VeryStrong,
}

/// Additional properties for edges
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EdgeProperties {
    /// Whether edge blocks movement
    pub blocks_movement: bool,
    /// Movement cost multiplier
    pub movement_cost_modifier: f32,
    /// Visual representation data
    pub visual_style: EdgeVisualStyle,
    /// Custom metadata
    pub metadata: HashMap<String, String>,
}

/// Visual styling for edge rendering
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeVisualStyle {
    /// Line width for rendering
    pub line_width: f32,
    /// Color components (RGBA)
    pub color: [f32; 4],
    /// Whether edge should be dashed
    pub dashed: bool,
    /// Animation speed (if animated)
    pub animation_speed: f32,
}

impl Default for EdgeVisualStyle {
    fn default() -> Self {
        Self {
            line_width: 1.0,
            color: [1.0, 1.0, 1.0, 1.0], // White
            dashed: false,
            animation_speed: 0.0,
        }
    }
}

/// Edge detection algorithms and parameters
#[derive(Debug, Clone)]
pub struct EdgeDetectionConfig {
    /// Sobel operator kernels for edge detection
    pub sobel_threshold: f32,
    /// Canny edge detection parameters
    pub canny_low_threshold: f32,
    pub canny_high_threshold: f32,
    /// Gaussian blur parameters for noise reduction
    pub gaussian_blur_sigma: f32,
    /// Minimum edge length to consider significant
    pub min_edge_length: u32,
    /// Maximum gap size to bridge in edge linking
    pub max_gap_size: u32,
}

impl Default for EdgeDetectionConfig {
    fn default() -> Self {
        Self {
            sobel_threshold: 0.1,
            canny_low_threshold: 0.05,
            canny_high_threshold: 0.15,
            gaussian_blur_sigma: 0.8,
            min_edge_length: 3,
            max_gap_size: 2,
        }
    }
}

/// High-performance edge detection system using image processing algorithms
#[derive(Debug)]
pub struct TileEdgeDetector {
    /// Detected edges indexed by chunk for spatial locality
    edges: Arc<RwLock<HashMap<ChunkCoord, Vec<TileEdge>>>>,
    /// Edge detection configuration
    config: EdgeDetectionConfig,
    /// Cache for edge detection results
    cache: GameCache,
    /// Tile component manager for data access
    tile_manager: Arc<TileComponentManager>,
    /// Chunk manager for spatial data
    chunk_manager: Arc<ChunkManager>,
}

impl TileEdgeDetector {
    /// Create new edge detection system
    pub fn new(tile_manager: Arc<TileComponentManager>, chunk_manager: Arc<ChunkManager>) -> Self {
        let cache = GameCacheBuilder::new()
            .max_memory_mb(64) // 64MB for edge detection cache
            .default_ttl(std::time::Duration::from_secs(300)) // 5 minute TTL
            .turn_based_invalidation(false)
            .build();

        Self {
            edges: Arc::new(RwLock::new(HashMap::new())),
            config: EdgeDetectionConfig::default(),
            cache,
            tile_manager,
            chunk_manager,
        }
    }

    /// Detect all edge types in a chunk using image processing algorithms
    #[instrument(skip(self))]
    pub async fn detect_edges_in_chunk(&self, chunk_coord: ChunkCoord) -> Result<Vec<TileEdge>, EdgeDetectionError> {
        let cache_key = CacheKey::Custom(format!("chunk_edges:{}:{}", chunk_coord.x, chunk_coord.y));
        
        // Check cache first
        if let Ok(Some(edges)) = self.cache.get::<Vec<TileEdge>>(&cache_key).await {
            return Ok(edges);
        }

        debug!("Detecting edges in chunk {:?}", chunk_coord);

        // Generate chunk data as 2D arrays for different properties
        let terrain_data = self.generate_terrain_array(chunk_coord)?;
        let elevation_data = self.generate_elevation_array(chunk_coord)?;
        
        let mut all_edges = Vec::new();

        // Detect terrain boundary edges
        let terrain_edges = self.detect_terrain_edges(&terrain_data, chunk_coord).await?;
        all_edges.extend(terrain_edges);

        // Detect elevation change edges
        let elevation_edges = self.detect_elevation_edges(&elevation_data, chunk_coord).await?;
        all_edges.extend(elevation_edges);

        // Detect coastlines (if applicable)
        let coastline_edges = self.detect_coastlines(&terrain_data, chunk_coord).await?;
        all_edges.extend(coastline_edges);

        // Store edges for this chunk
        {
            let mut edges_map = self.edges.write();
            edges_map.insert(chunk_coord, all_edges.clone());
        }

        // Cache results
        let _ = self.cache.set(cache_key, all_edges.clone(), CachePriority::Medium).await;
        
        debug!("Detected {} edges in chunk {:?}", all_edges.len(), chunk_coord);
        Ok(all_edges)
    }

    /// Generate terrain type array for chunk
    fn generate_terrain_array(&self, chunk_coord: ChunkCoord) -> Result<Array2<u8>, EdgeDetectionError> {
        let mut terrain_array = Array2::zeros((CHUNK_SIZE, CHUNK_SIZE));
        
        // Fill array with terrain type values
        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                let hex = ChunkManager::chunk_to_hex(chunk_coord, x, y);
                if let Some(tile_id) = self.chunk_manager.get_tile(hex) {
                    if let Ok(tile) = self.tile_manager.get_component::<Tile>(tile_id) {
                        terrain_array[[x, y]] = tile.terrain_type as u8;
                    }
                }
            }
        }
        
        Ok(terrain_array)
    }

    /// Generate elevation array for chunk
    fn generate_elevation_array(&self, chunk_coord: ChunkCoord) -> Result<Array2<f32>, EdgeDetectionError> {
        let mut elevation_array = Array2::zeros((CHUNK_SIZE, CHUNK_SIZE));
        
        // Fill array with elevation values
        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                let hex = ChunkManager::chunk_to_hex(chunk_coord, x, y);
                if let Some(tile_id) = self.chunk_manager.get_tile(hex) {
                    if let Ok(tile) = self.tile_manager.get_component::<Tile>(tile_id) {
                        elevation_array[[x, y]] = tile.elevation;
                    }
                }
            }
        }
        
        Ok(elevation_array)
    }

    /// Detect terrain boundary edges using Sobel edge detection
    async fn detect_terrain_edges(&self, terrain_data: &Array2<u8>, chunk_coord: ChunkCoord) -> Result<Vec<TileEdge>, EdgeDetectionError> {
        // Convert to grayscale image for processing
        let gray_image = self.array_to_gray_image(terrain_data.mapv(|v| v as f32));
        
        // Apply Sobel edge detection
        let edges = self.apply_sobel_filter(&gray_image);
        
        // Convert detected edges back to tile edges
        self.image_edges_to_tile_edges(&edges, chunk_coord, EdgeType::TerrainBoundary).await
    }

    /// Detect elevation change edges using gradient-based detection
    async fn detect_elevation_edges(&self, elevation_data: &Array2<f32>, chunk_coord: ChunkCoord) -> Result<Vec<TileEdge>, EdgeDetectionError> {
        // Apply Gaussian blur to reduce noise
        let blurred = self.apply_gaussian_blur(elevation_data);
        
        // Convert to grayscale image
        let gray_image = self.array_to_gray_image(&blurred);
        
        // Apply Canny edge detection for elevation changes
        let edges = self.apply_canny_filter(&gray_image);
        
        // Convert to tile edges
        self.image_edges_to_tile_edges(&edges, chunk_coord, EdgeType::ElevationChange).await
    }

    /// Detect coastlines (land-water boundaries)
    async fn detect_coastlines(&self, terrain_data: &Array2<u8>, chunk_coord: ChunkCoord) -> Result<Vec<TileEdge>, EdgeDetectionError> {
        // Create binary mask for water vs land
        let water_mask = terrain_data.mapv(|terrain_type| {
            if terrain_type == (TerrainType::Ocean as u8) { 255.0 } else { 0.0 }
        });
        
        let gray_image = self.array_to_gray_image(&water_mask);
        
        // Use simple edge detection for binary boundaries
        let edges = self.apply_sobel_filter(&gray_image);
        
        self.image_edges_to_tile_edges(&edges, chunk_coord, EdgeType::Coastline).await
    }

    /// Convert ndarray to grayscale image
    fn array_to_gray_image(&self, data: &Array2<f32>) -> GrayImage {
        let (height, width) = data.dim();
        let mut img = GrayImage::new(width as u32, height as u32);
        
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            let value = data[[y as usize, x as usize]];
            let normalized = (value * 255.0).clamp(0.0, 255.0) as u8;
            *pixel = Luma([normalized]);
        }
        
        img
    }

    /// Apply Sobel edge detection filter
    fn apply_sobel_filter(&self, image: &GrayImage) -> Array2<f32> {
        let (width, height) = image.dimensions();
        let mut edges = Array2::zeros((height as usize, width as usize));
        
        // Sobel kernels
        let sobel_x = [[-1, 0, 1], [-2, 0, 2], [-1, 0, 1]];
        let sobel_y = [[-1, -2, -1], [0, 0, 0], [1, 2, 1]];
        
        // Apply Sobel operator (excluding borders for simplicity)
        for y in 1..(height - 1) {
            for x in 1..(width - 1) {
                let mut gx = 0i32;
                let mut gy = 0i32;
                
                // Apply kernels
                for ky in 0..3 {
                    for kx in 0..3 {
                        let pixel_val = image.get_pixel(x + kx - 1, y + ky - 1).0[0] as i32;
                        gx += sobel_x[ky as usize][kx as usize] * pixel_val;
                        gy += sobel_y[ky as usize][kx as usize] * pixel_val;
                    }
                }
                
                // Calculate gradient magnitude
                let magnitude = ((gx * gx + gy * gy) as f32).sqrt() / 255.0;
                edges[[y as usize, x as usize]] = magnitude;
            }
        }
        
        edges
    }

    /// Apply Canny edge detection filter
    fn apply_canny_filter(&self, image: &GrayImage) -> Array2<f32> {
        // Simplified Canny implementation - in practice would use more sophisticated algorithms
        let sobel_result = self.apply_sobel_filter(image);
        
        // Apply hysteresis thresholding
        let mut edges = sobel_result.clone();
        
        for ((y, x), value) in edges.indexed_iter_mut() {
            if *value < self.config.canny_low_threshold {
                *value = 0.0;
            } else if *value > self.config.canny_high_threshold {
                *value = 1.0;
            } else {
                // Check if connected to strong edge
                let mut connected_to_strong = false;
                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        let ny = y as i32 + dy;
                        let nx = x as i32 + dx;
                        if ny >= 0 && ny < edges.nrows() as i32 && nx >= 0 && nx < edges.ncols() as i32 {
                            if sobel_result[[ny as usize, nx as usize]] > self.config.canny_high_threshold {
                                connected_to_strong = true;
                                break;
                            }
                        }
                    }
                    if connected_to_strong { break; }
                }
                
                *value = if connected_to_strong { 0.8 } else { 0.0 };
            }
        }
        
        edges
    }

    /// Apply Gaussian blur to reduce noise
    fn apply_gaussian_blur(&self, data: &Array2<f32>) -> Array2<f32> {
        // Simplified Gaussian blur - could be optimized with separable kernels
        let sigma = self.config.gaussian_blur_sigma;
        let kernel_size = (6.0 * sigma).ceil() as usize;
        let kernel_center = kernel_size / 2;
        
        // Generate Gaussian kernel
        let mut kernel = vec![vec![0.0; kernel_size]; kernel_size];
        let mut kernel_sum = 0.0;
        
        for y in 0..kernel_size {
            for x in 0..kernel_size {
                let dx = (x as i32 - kernel_center as i32) as f32;
                let dy = (y as i32 - kernel_center as i32) as f32;
                let value = (-0.5 * (dx * dx + dy * dy) / (sigma * sigma)).exp();
                kernel[y][x] = value;
                kernel_sum += value;
            }
        }
        
        // Normalize kernel
        for y in 0..kernel_size {
            for x in 0..kernel_size {
                kernel[y][x] /= kernel_sum;
            }
        }
        
        // Apply convolution
        let (height, width) = data.dim();
        let mut result = Array2::zeros((height, width));
        
        for y in kernel_center..(height - kernel_center) {
            for x in kernel_center..(width - kernel_center) {
                let mut sum = 0.0;
                
                for ky in 0..kernel_size {
                    for kx in 0..kernel_size {
                        let data_y = y + ky - kernel_center;
                        let data_x = x + kx - kernel_center;
                        sum += data[[data_y, data_x]] * kernel[ky][kx];
                    }
                }
                
                result[[y, x]] = sum;
            }
        }
        
        result
    }

    /// Convert image edge data to tile edges
    async fn image_edges_to_tile_edges(&self, edge_data: &Array2<f32>, chunk_coord: ChunkCoord, edge_type: EdgeType) -> Result<Vec<TileEdge>, EdgeDetectionError> {
        let mut tile_edges = Vec::new();
        let (height, width) = edge_data.dim();
        
        // Find significant edges and convert to tile coordinates
        for y in 0..height {
            for x in 0..width {
                let edge_strength = edge_data[[y, x]];
                
                if edge_strength >= edge_type.strength_threshold() {
                    // Convert array coordinates to hex coordinates
                    let hex = ChunkManager::chunk_to_hex(chunk_coord, x, y);
                    
                    if let Some(tile_id) = self.chunk_manager.get_tile(hex) {
                        // Check adjacent tiles to determine edge direction
                        for direction in HexDirection::ALL {
                            let neighbor_hex = HexCoord {
                                q: hex.q + direction.offset().q,
                                r: hex.r + direction.offset().r,
                            };
                            
                            if let Some(neighbor_id) = self.chunk_manager.get_tile(neighbor_hex) {
                                if tile_id != neighbor_id {
                                    let edge = TileEdge::new(tile_id, neighbor_id, direction, edge_type, edge_strength);
                                    tile_edges.push(edge);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(tile_edges)
    }

    /// Get all edges in a chunk
    pub fn get_chunk_edges(&self, chunk_coord: ChunkCoord) -> Vec<TileEdge> {
        self.edges.read().get(&chunk_coord).cloned().unwrap_or_default()
    }

    /// Get edges by type
    pub fn get_edges_by_type(&self, edge_type: EdgeType) -> Vec<TileEdge> {
        let edges = self.edges.read();
        edges.values()
            .flat_map(|chunk_edges| chunk_edges.iter())
            .filter(|edge| edge.edge_type == edge_type)
            .cloned()
            .collect()
    }

    /// Get edges involving a specific tile
    pub fn get_tile_edges(&self, tile_id: TileId) -> Vec<TileEdge> {
        let edges = self.edges.read();
        edges.values()
            .flat_map(|chunk_edges| chunk_edges.iter())
            .filter(|edge| edge.from_tile == tile_id || edge.to_tile == tile_id)
            .cloned()
            .collect()
    }

    /// Clear edges for a chunk (when chunk is modified)
    pub fn clear_chunk_edges(&self, chunk_coord: ChunkCoord) {
        self.edges.write().remove(&chunk_coord);
    }

    /// Get edge detection statistics
    pub fn edge_stats(&self) -> EdgeDetectionStats {
        let edges = self.edges.read();
        let mut stats = EdgeDetectionStats::default();
        
        let mut type_counts = HashMap::new();
        
        for chunk_edges in edges.values() {
            for edge in chunk_edges {
                stats.total_edges += 1;
                if edge.is_significant() {
                    stats.significant_edges += 1;
                }
                
                *type_counts.entry(edge.edge_type).or_insert(0) += 1;
            }
        }
        
        stats.chunks_processed = edges.len();
        stats.edges_by_type = type_counts;
        
        stats
    }

    /// Update configuration
    pub fn update_config(&mut self, config: EdgeDetectionConfig) {
        self.config = config;
        // Clear cache when config changes
        let _ = self.cache.clear();
    }
}

impl Default for TileEdgeDetector {
    fn default() -> Self {
        let tile_manager = Arc::new(TileComponentManager::new());
        let chunk_manager = Arc::new(ChunkManager::default());
        Self::new(tile_manager, chunk_manager)
    }
}

/// Statistics for edge detection monitoring
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EdgeDetectionStats {
    pub total_edges: usize,
    pub significant_edges: usize,
    pub chunks_processed: usize,
    pub edges_by_type: HashMap<EdgeType, usize>,
}

/// Edge detection errors
#[derive(Debug, thiserror::Error)]
pub enum EdgeDetectionError {
    #[error("Chunk data not available: {chunk:?}")]
    ChunkDataUnavailable { chunk: ChunkCoord },
    
    #[error("Image processing error: {message}")]
    ImageProcessingError { message: String },
    
    #[error("Invalid tile data")]
    InvalidTileData,
    
    #[error("Cache error: {message}")]
    CacheError { message: String },
}

/// System for updating edge detection when tiles change
pub fn update_edges_system(
    edge_detector: Res<TileEdgeDetector>,
    // Would include change detection queries
) {
    // Monitor tile changes and update edge detection for affected chunks
    // Implementation would depend on change tracking system
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_type_properties() {
        assert!(EdgeType::TerrainBoundary.strength_threshold() > 0.0);
        assert!(EdgeType::Coastline.strength_threshold() > EdgeType::PoliticalBorder.strength_threshold());
    }

    #[test]
    fn test_tile_edge_creation() {
        let edge = TileEdge::new(1, 2, HexDirection::East, EdgeType::TerrainBoundary, 0.5);
        
        assert_eq!(edge.from_tile, 1);
        assert_eq!(edge.to_tile, 2);
        assert_eq!(edge.direction, HexDirection::East);
        assert_eq!(edge.edge_type, EdgeType::TerrainBoundary);
        assert_eq!(edge.strength, 0.5);
        assert!(edge.is_significant());
    }

    #[test]
    fn test_edge_intensity() {
        let weak_edge = TileEdge::new(1, 2, HexDirection::East, EdgeType::TerrainBoundary, 0.1);
        let strong_edge = TileEdge::new(1, 2, HexDirection::East, EdgeType::TerrainBoundary, 0.9);
        
        assert_eq!(weak_edge.intensity(), EdgeIntensity::VeryWeak);
        assert_eq!(strong_edge.intensity(), EdgeIntensity::VeryStrong);
    }

    #[test]
    fn test_edge_detection_config() {
        let config = EdgeDetectionConfig::default();
        assert!(config.sobel_threshold > 0.0);
        assert!(config.canny_high_threshold > config.canny_low_threshold);
    }
}
