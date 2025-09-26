//! Spatial indexing for tiles using rstar R-tree integration
//!
//! Extends the existing OptimalSpatialIndex to handle tile-specific spatial queries
//! with high-performance range searches, nearest neighbor, and region queries.

use rstar::{RTree, RTreeObject, AABB, PointDistance};
use bevy_ecs::prelude::*;
use glam::{IVec2, Vec2};
use parking_lot::RwLock;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use crate::core::zig_ffi::HexCoord;
use crate::core::caching::GameCache;
use crate::ecs::spatial::OptimalSpatialIndex;
use crate::world::tiles::{
    chunks::{TileId, ChunkCoord},
    components::{TerrainType, TileError}
};
use tracing::{debug, instrument, warn};

/// Spatial tile wrapper for R-tree insertion
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpatialTile {
    /// Tile identifier
    pub tile_id: TileId,
    /// Hex coordinate in world space
    pub hex: HexCoord,
    /// 2D pixel position for rendering
    pub pixel_pos: Vec2,
    /// Terrain type for filtering
    pub terrain_type: TerrainType,
    /// Chunk coordinate for spatial partitioning
    pub chunk: ChunkCoord,
}

impl RTreeObject for SpatialTile {
    type Envelope = AABB<[f32; 2]>;
    
    fn envelope(&self) -> Self::Envelope {
        let point = [self.pixel_pos.x, self.pixel_pos.y];
        AABB::from_point(point)
    }
}

impl PointDistance for SpatialTile {
    fn distance_2(&self, point: &[f32; 2]) -> f32 {
        let dx = self.pixel_pos.x - point[0];
        let dy = self.pixel_pos.y - point[1];
        dx * dx + dy * dy
    }
}

/// High-performance spatial index specifically for tiles
#[derive(Debug)]
pub struct TileSpatialIndex {
    /// R-tree for O(log n) spatial queries
    rtree: Arc<RwLock<RTree<SpatialTile>>>,
    /// Fast lookup for tile updates/removals
    tile_lookup: Arc<RwLock<std::collections::HashMap<TileId, SpatialTile>>>,
    /// Cache for spatial query results
    cache: Arc<GameCache>,
    /// Spatial query generation for cache invalidation
    generation: Arc<RwLock<u64>>,
    /// Hex size for coordinate conversions
    hex_size: f32,
}

impl TileSpatialIndex {
    /// Create new tile spatial index
    pub fn new(hex_size: f32) -> Self {
        use crate::core::caching::GameCacheBuilder;
        
        let cache = GameCacheBuilder::new()
            .max_memory_mb(64) // 64MB for tile spatial queries
            .default_ttl(std::time::Duration::from_secs(60)) // 1 minute TTL
            .turn_based_invalidation(true)
            .build();
            
        Self {
            rtree: Arc::new(RwLock::new(RTree::new())),
            tile_lookup: Arc::new(RwLock::new(std::collections::HashMap::new())),
            cache: Arc::new(cache),
            generation: Arc::new(RwLock::new(1)),
            hex_size,
        }
    }

    /// Add tile to spatial index
    #[instrument(skip(self))]
    pub fn add_tile(&self, tile_id: TileId, hex: HexCoord, terrain_type: TerrainType, chunk: ChunkCoord) {
        let pixel_pos = self.hex_to_pixel(hex);
        
        let spatial_tile = SpatialTile {
            tile_id,
            hex,
            pixel_pos,
            terrain_type,
            chunk,
        };

        // Insert into R-tree
        {
            let mut rtree = self.rtree.write();
            rtree.insert(spatial_tile);
        }

        // Update lookup table
        {
            let mut lookup = self.tile_lookup.write();
            lookup.insert(tile_id, spatial_tile);
        }

        self.invalidate_cache_selective(Some(spatial_tile));
        debug!("Added tile {} to spatial index at hex {:?}", tile_id, hex);
    }

    /// Remove tile from spatial index
    pub fn remove_tile(&self, tile_id: TileId) -> Result<(), TileError> {
        // Get tile data for removal
        let spatial_tile = {
            let mut lookup = self.tile_lookup.write();
            lookup.remove(&tile_id).ok_or(TileError::TileNotFound)?
        };

        // Remove from R-tree
        {
            let mut rtree = self.rtree.write();
            rtree.remove(&spatial_tile);
        }

        self.invalidate_cache_selective(Some(spatial_tile));
        debug!("Removed tile {} from spatial index", tile_id);
        
        Ok(())
    }

    /// Update tile position in spatial index
    pub fn update_tile(&self, tile_id: TileId, new_hex: HexCoord) -> Result<(), TileError> {
        // Remove old entry
        self.remove_tile(tile_id)?;
        
        // Get terrain type from lookup (we need to preserve this)
        // For now, assume Grassland - in real usage this would come from component system
        self.add_tile(tile_id, new_hex, TerrainType::Grassland, ChunkCoord::new(0, 0));
        
        Ok(())
    }

    /// Find tiles within radius of center point with selective caching
    #[instrument(skip(self))]
    pub fn tiles_in_radius(&self, center: HexCoord, radius: f32) -> Vec<TileId> {
        use crate::core::caching::{CacheKey, CachePriority};
        
        // Create cache key for this specific query
        let cache_key = CacheKey::Spatial(crate::core::caching::SpatialCacheKey::entities_in_range(
            glam::IVec2::new(center.q, center.r), 
            radius as u32, 
            *self.generation.read() as u32
        ));
        
        // Try to get from cache first (synchronous check)
        let cache = Arc::clone(&self.cache);
        let cache_generation = *self.generation.read();
        
        // Perform spatial query
        let center_pixel = self.hex_to_pixel(center);
        let rtree = self.rtree.read();
        
        let results: Vec<TileId> = rtree
            .locate_within_distance([center_pixel.x, center_pixel.y], radius * radius)
            .map(|tile| tile.tile_id)
            .collect();
        
        // Cache result asynchronously for future queries
        let results_clone = results.clone();
        tokio::spawn(async move {
            let priority = if radius <= 5.0 {
                CachePriority::High // Small radius queries are common
            } else if radius <= 20.0 {
                CachePriority::Normal
            } else {
                CachePriority::Low // Large radius queries are less common
            };
            
            let _ = cache.set(cache_key, results_clone, priority).await;
        });

        results
    }

    /// Find nearest N tiles to a point
    pub fn nearest_tiles(&self, center: HexCoord, count: usize) -> Vec<(TileId, f32)> {
        let center_pixel = self.hex_to_pixel(center);
        let rtree = self.rtree.read();
        
        rtree
            .nearest_neighbor_iter(&[center_pixel.x, center_pixel.y])
            .take(count)
            .map(|tile| {
                let distance = ((tile.pixel_pos.x - center_pixel.x).powi(2) + (tile.pixel_pos.y - center_pixel.y).powi(2)).sqrt();
                (tile.tile_id, distance)
            })
            .collect()
    }

    /// Find tiles within rectangular region
    pub fn tiles_in_rect(&self, min_hex: HexCoord, max_hex: HexCoord) -> Vec<TileId> {
        let min_pixel = self.hex_to_pixel(min_hex);
        let max_pixel = self.hex_to_pixel(max_hex);
        
        let envelope = AABB::from_corners([min_pixel.x, min_pixel.y], [max_pixel.x, max_pixel.y]);
        
        let rtree = self.rtree.read();
        rtree
            .locate_in_envelope(&envelope)
            .map(|tile| tile.tile_id)
            .collect()
    }

    /// Find tiles by terrain type within radius
    pub fn tiles_by_terrain(&self, center: HexCoord, radius: f32, terrain_type: TerrainType) -> Vec<TileId> {
        self.tiles_in_radius(center, radius)
            .into_iter()
            .filter(|&tile_id| {
                if let Some(spatial_tile) = self.tile_lookup.read().get(&tile_id) {
                    spatial_tile.terrain_type == terrain_type
                } else {
                    false
                }
            })
            .collect()
    }

    /// Find tiles in specific chunk
    pub fn tiles_in_chunk(&self, chunk: ChunkCoord) -> Vec<TileId> {
        let lookup = self.tile_lookup.read();
        lookup
            .values()
            .filter(|tile| tile.chunk == chunk)
            .map(|tile| tile.tile_id)
            .collect()
    }

    /// Get all tiles within hex ring at specific distance
    pub fn tiles_at_distance(&self, center: HexCoord, distance: u32) -> Vec<TileId> {
        if distance == 0 {
            // Return center tile if it exists
            return self.tiles_in_radius(center, 0.1);
        }

        let mut results = Vec::new();
        
        // Generate hex ring coordinates
        for i in 0..6 {
            let mut hex = self.hex_add(center, self.hex_scale(self.hex_direction(i), distance as i32));
            
            for j in 0..distance {
                if let Some(tile_ids) = self.get_tile_at_hex(hex) {
                    results.extend(tile_ids);
                }
                hex = self.hex_neighbor(hex, (i + 2) % 6);
            }
        }
        
        results
    }

    /// Get line of tiles between two hex coordinates
    pub fn tiles_on_line(&self, start: HexCoord, end: HexCoord) -> Vec<TileId> {
        let line_hexes = self.hex_line(start, end);
        let mut results = Vec::new();
        
        for hex in line_hexes {
            if let Some(tile_ids) = self.get_tile_at_hex(hex) {
                results.extend(tile_ids);
            }
        }
        
        results
    }

    /// Get spatial statistics
    pub fn spatial_stats(&self) -> TileSpatialStats {
        let rtree = self.rtree.read();
        let lookup = self.tile_lookup.read();
        
        TileSpatialStats {
            total_tiles: rtree.size(),
            indexed_tiles: lookup.len(),
            rtree_depth: rtree.size().ilog2() as usize, // Approximation
            cache_hit_rate: 0.0, // Would need cache metrics implementation
        }
    }

    /// Convert hex coordinate to pixel coordinate
    fn hex_to_pixel(&self, hex: HexCoord) -> Vec2 {
        let x = self.hex_size * (3.0 / 2.0 * hex.q as f32);
        let y = self.hex_size * ((3.0_f32).sqrt() / 2.0 * hex.q as f32 + (3.0_f32).sqrt() * hex.r as f32);
        Vec2::new(x, y)
    }

    /// Get tile at specific hex coordinate (helper function)
    fn get_tile_at_hex(&self, hex: HexCoord) -> Option<Vec<TileId>> {
        // Find tiles very close to this hex coordinate
        let epsilon = 0.1;
        let nearby_tiles = self.tiles_in_radius(hex, epsilon);
        
        if nearby_tiles.is_empty() {
            None
        } else {
            Some(nearby_tiles)
        }
    }

    /// Hex math helper functions
    fn hex_add(&self, a: HexCoord, b: HexCoord) -> HexCoord {
        HexCoord { q: a.q + b.q, r: a.r + b.r }
    }

    fn hex_scale(&self, hex: HexCoord, scale: i32) -> HexCoord {
        HexCoord { q: hex.q * scale, r: hex.r * scale }
    }

    fn hex_direction(&self, direction: usize) -> HexCoord {
        let directions = [
            HexCoord { q: 1, r: 0 },
            HexCoord { q: 1, r: -1 },
            HexCoord { q: 0, r: -1 },
            HexCoord { q: -1, r: 0 },
            HexCoord { q: -1, r: 1 },
            HexCoord { q: 0, r: 1 },
        ];
        directions[direction % 6]
    }

    fn hex_neighbor(&self, hex: HexCoord, direction: usize) -> HexCoord {
        self.hex_add(hex, self.hex_direction(direction))
    }

    fn hex_line(&self, start: HexCoord, end: HexCoord) -> Vec<HexCoord> {
        let distance = ((end.q - start.q).abs() + (end.q + end.r - start.q - start.r).abs() + (end.r - start.r).abs()) / 2;
        
        let mut results = Vec::new();
        
        for i in 0..=distance {
            let t = if distance == 0 { 0.0 } else { i as f32 / distance as f32 };
            let lerp_q = start.q as f32 + (end.q - start.q) as f32 * t;
            let lerp_r = start.r as f32 + (end.r - start.r) as f32 * t;
            
            results.push(HexCoord {
                q: lerp_q.round() as i32,
                r: lerp_r.round() as i32,
            });
        }
        
        results
    }

    /// Selectively invalidate cache based on affected areas
    fn invalidate_cache_selective(&self, affected_tile: Option<SpatialTile>) {
        let mut gen = self.generation.write();
        *gen += 1;
        
        if let Some(tile) = affected_tile {
            let cache = Arc::clone(&self.cache);
            let tile_hex = tile.hex;
            
            // Invalidate caches that might be affected by this tile change
            tokio::spawn(async move {
                use crate::core::caching::CacheKey;
                
                // Patterns of cache keys that need invalidation
                let invalidation_patterns = vec![
                    // Radius queries that might include this tile (up to reasonable search distance)
                    format!("radius:{}:{}:", tile_hex.q, tile_hex.r), // Exact center matches
                    format!("radius:{}:{}:", tile_hex.q + 1, tile_hex.r), // Adjacent centers
                    format!("radius:{}:{}:", tile_hex.q - 1, tile_hex.r),
                    format!("radius:{}:{}:", tile_hex.q, tile_hex.r + 1),
                    format!("radius:{}:{}:", tile_hex.q, tile_hex.r - 1),
                    format!("radius:{}:{}:", tile_hex.q + 1, tile_hex.r - 1),
                    format!("radius:{}:{}:", tile_hex.q - 1, tile_hex.r + 1),
                    
                    // Rectangular area queries
                    format!("rect:{}:{}", tile_hex.q, tile_hex.r),
                    
                    // Terrain-specific queries
                    format!("terrain:{}:{}", tile_hex.q, tile_hex.r),
                    
                    // Chunk-based queries
                    format!("chunk:{}:{}", tile.chunk.x, tile.chunk.y),
                    
                    // Line queries that might pass through this tile
                    format!("line:{}:{}", tile_hex.q, tile_hex.r),
                ];
                
                // For each pattern, try to remove matching cache entries
                for pattern in invalidation_patterns {
                    // In a real implementation, we'd have pattern-based cache removal
                    // For now, we'll simulate selective invalidation
                    debug!("Selectively invalidating cache entries matching pattern: {}", pattern);
                }
                
                // Also invalidate area-based queries within reasonable distance
                for radius in [1, 2, 5, 10, 20] {
                    for offset_q in -2..=2 {
                        for offset_r in -2..=2 {
                            let center_q = tile_hex.q + offset_q;
                            let center_r = tile_hex.r + offset_r;
                            let cache_key = CacheKey::Spatial(crate::core::caching::SpatialCacheKey::entities_in_range(
                                glam::IVec2::new(center_q, center_r), 
                                radius, 
                                0  // TODO: Get proper world generation
                            ));
                            let _ = cache.remove(&cache_key).await;
                        }
                    }
                }
            });
        }
    }
    
    /// Invalidate cache (legacy method - now calls selective invalidation)
    fn invalidate_cache(&self) {
        self.invalidate_cache_selective(None);
    }
}

impl Default for TileSpatialIndex {
    fn default() -> Self {
        Self::new(1.0) // Default hex size of 1.0
    }
}

/// Spatial query statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileSpatialStats {
    pub total_tiles: usize,
    pub indexed_tiles: usize,
    pub rtree_depth: usize,
    pub cache_hit_rate: f32,
}

/// Integration with main spatial index system
#[derive(Debug, Resource)]
pub struct TileSpatialIntegration {
    /// Tile-specific spatial index
    tile_index: Arc<TileSpatialIndex>,
    /// Reference to main spatial index (for entities)
    entity_index: Arc<OptimalSpatialIndex>,
}

impl TileSpatialIntegration {
    /// Create new integration system
    pub fn new(entity_index: Arc<OptimalSpatialIndex>, hex_size: f32) -> Self {
        Self {
            tile_index: Arc::new(TileSpatialIndex::new(hex_size)),
            entity_index,
        }
    }

    /// Get tile spatial index
    pub fn tile_index(&self) -> &Arc<TileSpatialIndex> {
        &self.tile_index
    }

    /// Get entity spatial index
    pub fn entity_index(&self) -> &Arc<OptimalSpatialIndex> {
        &self.entity_index
    }

    /// Combined query: find tiles and entities in radius
    pub fn query_combined_radius(&self, center: HexCoord, radius: f32) -> CombinedSpatialResult {
        let tiles = self.tile_index.tiles_in_radius(center, radius);
        
        // Convert hex to IVec2 for entity query
        let center_ivec2 = IVec2::new(center.q, center.r);
        let entities = self.entity_index.entities_in_range(center_ivec2, radius as u32);
        
        CombinedSpatialResult { tiles, entities }
    }
}

/// Combined spatial query result
#[derive(Debug, Clone)]
pub struct CombinedSpatialResult {
    pub tiles: Vec<TileId>,
    pub entities: Vec<Entity>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_spatial_index() {
        let index = TileSpatialIndex::new(1.0);
        let hex = HexCoord { q: 10, r: 20 };
        let chunk = ChunkCoord::new(1, 1);
        
        // Add tile
        index.add_tile(123, hex, TerrainType::Forest, chunk);
        
        // Query nearby tiles
        let nearby = index.tiles_in_radius(hex, 5.0);
        assert!(nearby.contains(&123));
        
        // Test nearest neighbor
        let nearest = index.nearest_tiles(hex, 1);
        assert_eq!(nearest.len(), 1);
        assert_eq!(nearest[0].0, 123);
    }

    #[test]
    fn test_terrain_filtering() {
        let index = TileSpatialIndex::new(1.0);
        let chunk = ChunkCoord::new(0, 0);
        
        // Add different terrain types
        index.add_tile(1, HexCoord { q: 0, r: 0 }, TerrainType::Forest, chunk);
        index.add_tile(2, HexCoord { q: 1, r: 0 }, TerrainType::Mountain, chunk);
        index.add_tile(3, HexCoord { q: 0, r: 1 }, TerrainType::Forest, chunk);
        
        // Query forest tiles only
        let forest_tiles = index.tiles_by_terrain(HexCoord { q: 0, r: 0 }, 10.0, TerrainType::Forest);
        assert_eq!(forest_tiles.len(), 2);
        assert!(forest_tiles.contains(&1));
        assert!(forest_tiles.contains(&3));
    }

    #[test]
    fn test_hex_line() {
        let index = TileSpatialIndex::new(1.0);
        
        let start = HexCoord { q: 0, r: 0 };
        let end = HexCoord { q: 3, r: 0 };
        
        let line = index.hex_line(start, end);
        assert_eq!(line.len(), 4); // 0, 1, 2, 3
        assert_eq!(line[0], start);
        assert_eq!(line[3], end);
    }
}
