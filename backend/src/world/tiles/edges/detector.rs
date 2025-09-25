//! Main edge detection implementation

use image::{Luma, GrayImage};
use ndarray::Array2;
use bevy_ecs::prelude::*;
use std::collections::HashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use tracing::{debug, instrument};

use crate::core::{
    zig_ffi::HexCoord,
    caching::{GameCache, GameCacheBuilder, CacheKey, CachePriority}
};
use crate::world::tiles::{
    chunks::{TileId, ChunkCoord, ChunkManager, CHUNK_SIZE},
    components::{Tile, TerrainType, TileComponentManager},
    adjacency::HexDirection
};

use super::{
    types::{EdgeType, TileEdge},
    config::EdgeDetectionConfig,
    stats::{EdgeDetectionStats, EdgeDetectionError}
};

/// High-performance edge detection system using image processing algorithms
#[derive(Debug, Resource)]
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
        let gray_image = self.array_to_gray_image(&terrain_data.mapv(|v| v as f32));
        
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

    /// Apply complete Canny edge detection filter with all stages
    fn apply_canny_filter(&self, image: &GrayImage) -> Array2<f32> {
        let (height, width) = (image.height() as usize, image.width() as usize);
        
        // Step 1: Convert to f32 array
        let mut data = Array2::zeros((height, width));
        for y in 0..height {
            for x in 0..width {
                data[[y, x]] = image.get_pixel(x as u32, y as u32)[0] as f32 / 255.0;
            }
        }
        
        // Step 2: Apply Gaussian blur to reduce noise
        let blurred = self.apply_gaussian_blur(&data);
        
        // Step 3: Compute gradients using Sobel operators
        let (grad_x, grad_y, magnitude) = self.compute_gradients(&blurred);
        
        // Step 4: Apply non-maxima suppression
        let thin_edges = self.non_maxima_suppression(&magnitude, &grad_x, &grad_y);
        
        // Step 5: Apply hysteresis thresholding
        let final_edges = self.hysteresis_thresholding(&thin_edges);
        
        final_edges
    }
    
    /// Compute gradients using Sobel operators
    fn compute_gradients(&self, data: &Array2<f32>) -> (Array2<f32>, Array2<f32>, Array2<f32>) {
        let (height, width) = data.dim();
        let mut grad_x = Array2::zeros((height, width));
        let mut grad_y = Array2::zeros((height, width));
        let mut magnitude = Array2::zeros((height, width));
        
        // Sobel kernels
        let sobel_x = [[-1.0, 0.0, 1.0], [-2.0, 0.0, 2.0], [-1.0, 0.0, 1.0]];
        let sobel_y = [[-1.0, -2.0, -1.0], [0.0, 0.0, 0.0], [1.0, 2.0, 1.0]];
        
        for y in 1..(height - 1) {
            for x in 1..(width - 1) {
                let mut gx = 0.0;
                let mut gy = 0.0;
                
                // Apply Sobel kernels
                for ky in -1i32..=1 {
                    for kx in -1i32..=1 {
                        let pixel = data[[(y as i32 + ky) as usize, (x as i32 + kx) as usize]];
                        gx += pixel * sobel_x[(ky + 1) as usize][(kx + 1) as usize];
                        gy += pixel * sobel_y[(ky + 1) as usize][(kx + 1) as usize];
                    }
                }
                
                grad_x[[y, x]] = gx;
                grad_y[[y, x]] = gy;
                magnitude[[y, x]] = (gx * gx + gy * gy).sqrt();
            }
        }
        
        (grad_x, grad_y, magnitude)
    }
    
    /// Apply non-maxima suppression to thin edges
    fn non_maxima_suppression(&self, magnitude: &Array2<f32>, grad_x: &Array2<f32>, grad_y: &Array2<f32>) -> Array2<f32> {
        let (height, width) = magnitude.dim();
        let mut result = Array2::zeros((height, width));
        
        for y in 1..(height - 1) {
            for x in 1..(width - 1) {
                let mag = magnitude[[y, x]];
                if mag == 0.0 { continue; }
                
                // Calculate gradient direction
                let gx = grad_x[[y, x]];
                let gy = grad_y[[y, x]];
                let angle = gy.atan2(gx);
                
                // Quantize angle to one of 4 directions (0, 45, 90, 135 degrees)
                let angle_deg = angle.to_degrees();
                let quantized_angle = ((angle_deg + 180.0 + 22.5) / 45.0).floor() as i32 % 4;
                
                // Get neighbor pixels based on gradient direction
                let (dy1, dx1, dy2, dx2) = match quantized_angle {
                    0 => (0, -1, 0, 1),   // Horizontal
                    1 => (-1, 1, 1, -1),  // Diagonal /
                    2 => (-1, 0, 1, 0),   // Vertical
                    3 => (-1, -1, 1, 1),  // Diagonal \
                    _ => (0, -1, 0, 1),   // Default to horizontal
                };
                
                let mag1 = magnitude[[(y as i32 + dy1) as usize, (x as i32 + dx1) as usize]];
                let mag2 = magnitude[[(y as i32 + dy2) as usize, (x as i32 + dx2) as usize]];
                
                // Keep pixel only if it's a local maximum along the gradient direction
                if mag >= mag1 && mag >= mag2 {
                    result[[y, x]] = mag;
                }
            }
        }
        
        result
    }
    
    /// Apply hysteresis thresholding with edge following
    fn hysteresis_thresholding(&self, edges: &Array2<f32>) -> Array2<f32> {
        let (height, width) = edges.dim();
        let mut result = Array2::zeros((height, width));
        let mut visited = Array2::from_elem((height, width), false);
        
        let high_threshold = self.config.canny_high_threshold;
        let low_threshold = self.config.canny_low_threshold;
        
        // First pass: mark strong edges
        for y in 0..height {
            for x in 0..width {
                if edges[[y, x]] >= high_threshold {
                    result[[y, x]] = 1.0;
                    visited[[y, x]] = true;
                }
            }
        }
        
        // Second pass: follow weak edges connected to strong edges
        let mut stack = Vec::new();
        
        // Find all strong edge pixels
        for y in 0..height {
            for x in 0..width {
                if result[[y, x]] == 1.0 {
                    stack.push((y as i32, x as i32));
                }
            }
        }
        
        // Follow connected weak edges
        while let Some((y, x)) = stack.pop() {
            // Check 8-connected neighbors
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    if dy == 0 && dx == 0 { continue; }
                    
                    let ny = y + dy;
                    let nx = x + dx;
                    
                    if ny >= 0 && ny < height as i32 && nx >= 0 && nx < width as i32 {
                        let ny = ny as usize;
                        let nx = nx as usize;
                        
                        if !visited[[ny, nx]] && edges[[ny, nx]] >= low_threshold {
                            result[[ny, nx]] = 0.8; // Mark as weak edge
                            visited[[ny, nx]] = true;
                            stack.push((ny as i32, nx as i32));
                        }
                    }
                }
            }
        }
        
        result
    }

    /// Apply optimized separable Gaussian blur to reduce noise
    fn apply_gaussian_blur(&self, data: &Array2<f32>) -> Array2<f32> {
        let sigma = self.config.gaussian_blur_sigma;
        let kernel_radius = (3.0 * sigma).ceil() as usize;
        let kernel_size = 2 * kernel_radius + 1;
        
        // Generate 1D Gaussian kernel
        let mut kernel = vec![0.0; kernel_size];
        let mut kernel_sum = 0.0;
        
        for i in 0..kernel_size {
            let x = (i as i32 - kernel_radius as i32) as f32;
            let value = (-0.5 * x * x / (sigma * sigma)).exp();
            kernel[i] = value;
            kernel_sum += value;
        }
        
        // Normalize kernel
        for value in kernel.iter_mut() {
            *value /= kernel_sum;
        }
        
        let (height, width) = data.dim();
        
        // First pass: horizontal blur
        let mut temp = Array2::zeros((height, width));
        for y in 0..height {
            for x in 0..width {
                let mut sum = 0.0;
                let mut weight_sum = 0.0;
                
                for k in 0..kernel_size {
                    let sample_x = x as i32 + k as i32 - kernel_radius as i32;
                    if sample_x >= 0 && sample_x < width as i32 {
                        let weight = kernel[k];
                        sum += data[[y, sample_x as usize]] * weight;
                        weight_sum += weight;
                    }
                }
                
                temp[[y, x]] = if weight_sum > 0.0 { sum / weight_sum } else { data[[y, x]] };
            }
        }
        
        // Second pass: vertical blur
        let mut result = Array2::zeros((height, width));
        for y in 0..height {
            for x in 0..width {
                let mut sum = 0.0;
                let mut weight_sum = 0.0;
                
                for k in 0..kernel_size {
                    let sample_y = y as i32 + k as i32 - kernel_radius as i32;
                    if sample_y >= 0 && sample_y < height as i32 {
                        let weight = kernel[k];
                        sum += temp[[sample_y as usize, x]] * weight;
                        weight_sum += weight;
                    }
                }
                
                result[[y, x]] = if weight_sum > 0.0 { sum / weight_sum } else { temp[[y, x]] };
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
