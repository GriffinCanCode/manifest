//! Spatial caching for hex-grid based queries
//!
//! Optimized caching for spatial operations in grand strategy games:
//! - Hex coordinate-based entity lookups
//! - Range queries with radius optimization
//! - Player territory and visibility caching
//! - Spatial relationship caching for pathfinding

use std::collections::HashSet;
use glam::IVec2;
use serde::{Serialize, Deserialize};
use bevy_ecs::prelude::Entity;

use crate::core::hashing::{CoordinateHasher, FastHashSet, FastHashMap};
use super::{CacheKey, CachePriority};

/// Spatial cache key for position-based queries
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpatialCacheKey {
    /// Query type
    pub query_type: SpatialQueryType,
    /// Primary position
    pub position: IVec2,
    /// Optional radius for range queries
    pub radius: Option<u32>,
    /// Optional player ID filter
    pub player_id: Option<u32>,
    /// Component filter hash (for type-specific queries)
    pub component_filter: Option<u64>,
    /// World generation/version for cache invalidation
    pub world_generation: u32,
}

impl SpatialCacheKey {
    /// Fast hash using existing game hashing infrastructure
    pub fn fast_hash(&self) -> u64 {
        use crate::core::hashing::FastHasher;
        FastHasher::hash_one(self)
    }
}

/// Types of spatial queries that can be cached
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpatialQueryType {
    /// Entities at exact position
    EntitiesAt,
    /// Entities within range
    EntitiesInRange,
    /// Entities in line of sight
    LineOfSight,
    /// Pathfinding accessibility
    PathAccessible,
    /// Player territory bounds
    TerritoryBounds,
    /// Visibility map for player
    VisibilityMap,
    /// Neighboring positions
    Neighbors,
    /// Hex ring at distance
    HexRing,
}

impl SpatialCacheKey {
    /// Create a key for entities at a specific position
    pub fn entities_at(position: IVec2, world_generation: u32) -> Self {
        Self {
            query_type: SpatialQueryType::EntitiesAt,
            position,
            radius: None,
            player_id: None,
            component_filter: None,
            world_generation,
        }
    }

    /// Create a key for entities within a range
    pub fn entities_in_range(position: IVec2, radius: u32, world_generation: u32) -> Self {
        Self {
            query_type: SpatialQueryType::EntitiesInRange,
            position,
            radius: Some(radius),
            player_id: None,
            component_filter: None,
            world_generation,
        }
    }

    /// Create a key for player-owned entities in range
    pub fn player_entities_in_range(position: IVec2, radius: u32, player_id: u32, world_generation: u32) -> Self {
        Self {
            query_type: SpatialQueryType::EntitiesInRange,
            position,
            radius: Some(radius),
            player_id: Some(player_id),
            component_filter: None,
            world_generation,
        }
    }

    /// Create a key for typed entities in range (with component filter)
    pub fn typed_entities_in_range(position: IVec2, radius: u32, component_hash: u64, world_generation: u32) -> Self {
        Self {
            query_type: SpatialQueryType::EntitiesInRange,
            position,
            radius: Some(radius),
            player_id: None,
            component_filter: Some(component_hash),
            world_generation,
        }
    }

    /// Create a key for line of sight query
    pub fn line_of_sight(from: IVec2, to: IVec2, player_id: u32, world_generation: u32) -> Self {
        Self {
            query_type: SpatialQueryType::LineOfSight,
            position: from,
            radius: Some(from.distance_squared(to) as u32), // Use distance as radius
            player_id: Some(player_id),
            component_filter: None,
            world_generation,
        }
    }

    /// Create a key for pathfinding accessibility
    pub fn path_accessible(from: IVec2, to: IVec2, world_generation: u32) -> Self {
        Self {
            query_type: SpatialQueryType::PathAccessible,
            position: from,
            radius: Some(from.distance_squared(to) as u32),
            player_id: None,
            component_filter: None,
            world_generation,
        }
    }

    /// Create a key for player territory bounds
    pub fn territory_bounds(player_id: u32, world_generation: u32) -> Self {
        Self {
            query_type: SpatialQueryType::TerritoryBounds,
            position: IVec2::ZERO, // Not relevant for territory queries
            radius: None,
            player_id: Some(player_id),
            component_filter: None,
            world_generation,
        }
    }

    /// Create a key for player visibility map
    pub fn visibility_map(player_id: u32, world_generation: u32) -> Self {
        Self {
            query_type: SpatialQueryType::VisibilityMap,
            position: IVec2::ZERO,
            radius: None,
            player_id: Some(player_id),
            component_filter: None,
            world_generation,
        }
    }

    /// Create a key for hex neighbors
    pub fn hex_neighbors(position: IVec2, world_generation: u32) -> Self {
        Self {
            query_type: SpatialQueryType::Neighbors,
            position,
            radius: Some(1),
            player_id: None,
            component_filter: None,
            world_generation,
        }
    }

    /// Create a key for hex ring at specific distance
    pub fn hex_ring(center: IVec2, distance: u32, world_generation: u32) -> Self {
        Self {
            query_type: SpatialQueryType::HexRing,
            position: center,
            radius: Some(distance),
            player_id: None,
            component_filter: None,
            world_generation,
        }
    }

    /// Get cache priority for this spatial query type
    pub fn cache_priority(&self) -> CachePriority {
        match self.query_type {
            SpatialQueryType::EntitiesAt | SpatialQueryType::EntitiesInRange => CachePriority::High,
            SpatialQueryType::LineOfSight | SpatialQueryType::PathAccessible => CachePriority::High,
            SpatialQueryType::TerritoryBounds | SpatialQueryType::VisibilityMap => CachePriority::Normal,
            SpatialQueryType::Neighbors | SpatialQueryType::HexRing => CachePriority::High,
        }
    }

    /// Estimate cache value size for memory management
    pub fn estimated_value_size(&self) -> usize {
        match self.query_type {
            SpatialQueryType::EntitiesAt => 64, // Small entity list
            SpatialQueryType::EntitiesInRange => {
                let radius = self.radius.unwrap_or(1);
                // Estimate: roughly 8 bytes per entity * hex area
                (radius * radius * 6 * 8) as usize // Hex area approximation
            }
            SpatialQueryType::LineOfSight => 32, // Boolean result + path
            SpatialQueryType::PathAccessible => 32, // Boolean result
            SpatialQueryType::TerritoryBounds => 256, // Set of positions
            SpatialQueryType::VisibilityMap => 1024, // Large set of positions
            SpatialQueryType::Neighbors => 48, // ~6 neighbors
            SpatialQueryType::HexRing => {
                let radius = self.radius.unwrap_or(1);
                (radius * 6 * 8) as usize // Ring circumference
            }
        }
    }
}

/// Spatial query results that can be cached
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpatialQueryResult {
    /// List of entities
    Entities(Vec<Entity>),
    /// Set of positions
    Positions(Vec<IVec2>),
    /// Boolean result
    Boolean(bool),
    /// Line of sight with blocked positions
    LineOfSight {
        visible: bool,
        blocked_positions: Vec<IVec2>,
    },
    /// Territory bounds with perimeter
    Territory {
        bounds: Vec<IVec2>,
        perimeter: Vec<IVec2>,
        area: u32,
    },
    /// Visibility map with ranges
    VisibilityMap {
        visible_positions: Vec<IVec2>,
        sight_ranges: FastHashMap<IVec2, u32>,
    },
}

impl SpatialQueryResult {
    /// Get the size of this result in bytes (approximation)
    pub fn size_bytes(&self) -> usize {
        match self {
            SpatialQueryResult::Entities(entities) => entities.len() * 8,
            SpatialQueryResult::Positions(positions) => positions.len() * 8,
            SpatialQueryResult::Boolean(_) => 1,
            SpatialQueryResult::LineOfSight { blocked_positions, .. } => {
                1 + blocked_positions.len() * 8
            }
            SpatialQueryResult::Territory { bounds, perimeter, .. } => {
                bounds.len() * 8 + perimeter.len() * 8 + 4
            }
            SpatialQueryResult::VisibilityMap { visible_positions, sight_ranges } => {
                visible_positions.len() * 8 + sight_ranges.len() * 12
            }
        }
    }

    /// Check if this result is empty or trivial
    pub fn is_empty(&self) -> bool {
        match self {
            SpatialQueryResult::Entities(entities) => entities.is_empty(),
            SpatialQueryResult::Positions(positions) => positions.is_empty(),
            SpatialQueryResult::Boolean(_) => false,
            SpatialQueryResult::LineOfSight { .. } => false,
            SpatialQueryResult::Territory { bounds, .. } => bounds.is_empty(),
            SpatialQueryResult::VisibilityMap { visible_positions, .. } => visible_positions.is_empty(),
        }
    }
}

/// Spatial cache specialized for hex-grid operations
pub struct SpatialCache {
    /// Cache storage
    cache: FastHashMap<u64, CachedSpatialResult>,
    /// Spatial index for quick invalidation
    spatial_index: SpatialIndex,
    /// Current world generation for cache invalidation
    world_generation: u32,
}

/// Cached spatial result with metadata
#[derive(Debug, Clone)]
pub struct CachedSpatialResult {
    pub result: SpatialQueryResult,
    pub key: SpatialCacheKey,
    pub created_at: std::time::Instant,
    pub access_count: u32,
    pub last_accessed: std::time::Instant,
}

impl CachedSpatialResult {
    pub fn new(result: SpatialQueryResult, key: SpatialCacheKey) -> Self {
        let now = std::time::Instant::now();
        Self {
            result,
            key,
            created_at: now,
            access_count: 0,
            last_accessed: now,
        }
    }

    pub fn access(&mut self) {
        self.last_accessed = std::time::Instant::now();
        self.access_count = self.access_count.saturating_add(1);
    }

    pub fn age(&self) -> std::time::Duration {
        std::time::Instant::now() - self.created_at
    }
}

/// Spatial index for efficient cache invalidation
#[derive(Debug, Default)]
pub struct SpatialIndex {
    /// Map positions to cache keys that depend on them
    position_to_keys: FastHashMap<IVec2, FastHashSet<u64>>,
    /// Map regions to cache keys for range queries
    region_to_keys: FastHashMap<u64, FastHashSet<u64>>, // Region hash -> key hashes
}

impl SpatialIndex {
    /// Add a cache key to the spatial index
    pub fn add_key(&mut self, key: &SpatialCacheKey, key_hash: u64) {
        // Add to position index
        self.position_to_keys.entry(key.position)
            .or_default()
            .insert(key_hash);

        // Add to region index for range queries
        if let Some(radius) = key.radius {
            let region_hash = self.compute_region_hash(key.position, radius);
            self.region_to_keys.entry(region_hash)
                .or_default()
                .insert(key_hash);
        }
    }

    /// Remove a cache key from the spatial index
    pub fn remove_key(&mut self, key: &SpatialCacheKey, key_hash: u64) {
        // Remove from position index
        if let Some(keys) = self.position_to_keys.get_mut(&key.position) {
            keys.remove(&key_hash);
            if keys.is_empty() {
                self.position_to_keys.remove(&key.position);
            }
        }

        // Remove from region index
        if let Some(radius) = key.radius {
            let region_hash = self.compute_region_hash(key.position, radius);
            if let Some(keys) = self.region_to_keys.get_mut(&region_hash) {
                keys.remove(&key_hash);
                if keys.is_empty() {
                    self.region_to_keys.remove(&region_hash);
                }
            }
        }
    }

    /// Get cache keys affected by a position change
    pub fn get_affected_keys(&self, position: IVec2) -> Vec<u64> {
        let mut affected = Vec::new();

        // Direct position matches
        if let Some(keys) = self.position_to_keys.get(&position) {
            affected.extend(keys.iter().copied());
        }

        // Range query matches - check regions that might contain this position
        for radius in 1..=10u32 { // Check reasonable range radii
            for center in self.hex_neighbors_at_distance(position, radius as i32) {
                let region_hash = self.compute_region_hash(center, radius);
                if let Some(keys) = self.region_to_keys.get(&region_hash) {
                    affected.extend(keys.iter().copied());
                }
            }
        }

        affected.sort_unstable();
        affected.dedup();
        affected
    }

    /// Compute hash for a region (center + radius)
    fn compute_region_hash(&self, center: IVec2, radius: u32) -> u64 {
        CoordinateHasher::hash_with_seed(center, radius as u64)
    }

    /// Get hex neighbors at a specific distance (for region checking)
    fn hex_neighbors_at_distance(&self, center: IVec2, distance: i32) -> Vec<IVec2> {
        let mut neighbors = Vec::new();
        
        // Simple hex ring algorithm
        for dx in -distance..=distance {
            let dy_min = (-distance - dx).max(-distance);
            let dy_max = (-distance - dx).min(distance);
            
            for dy in dy_min..=dy_max {
                if dx != 0 || dy != 0 {
                    neighbors.push(IVec2::new(center.x + dx, center.y + dy));
                }
            }
        }
        
        neighbors
    }
}

impl SpatialCache {
    /// Create a new spatial cache
    pub fn new() -> Self {
        Self {
            cache: FastHashMap::default(),
            spatial_index: SpatialIndex::default(),
            world_generation: 1,
        }
    }

    /// Get a cached spatial query result
    pub fn get(&mut self, key: &SpatialCacheKey) -> Option<SpatialQueryResult> {
        // Check if cache is valid for current world generation
        if key.world_generation < self.world_generation {
            return None;
        }

        let key_hash = key.fast_hash();
        if let Some(cached) = self.cache.get_mut(&key_hash) {
            cached.access();
            Some(cached.result.clone())
        } else {
            None
        }
    }

    /// Store a spatial query result
    pub fn set(&mut self, key: SpatialCacheKey, result: SpatialQueryResult) {
        let key_hash = key.fast_hash();
        let cached_result = CachedSpatialResult::new(result, key.clone());
        
        // Add to spatial index
        self.spatial_index.add_key(&key, key_hash);
        
        // Store in cache
        self.cache.insert(key_hash, cached_result);
    }

    /// Remove a cached result
    pub fn remove(&mut self, key: &SpatialCacheKey) -> bool {
        let key_hash = key.fast_hash();
        
        if let Some(cached) = self.cache.remove(&key_hash) {
            self.spatial_index.remove_key(&cached.key, key_hash);
            true
        } else {
            false
        }
    }

    /// Invalidate cache entries affected by a position change
    pub fn invalidate_position(&mut self, position: IVec2) {
        let affected_keys = self.spatial_index.get_affected_keys(position);
        
        for key_hash in affected_keys {
            if let Some(cached) = self.cache.remove(&key_hash) {
                self.spatial_index.remove_key(&cached.key, key_hash);
            }
        }
    }

    /// Invalidate cache entries for a specific player
    pub fn invalidate_player(&mut self, player_id: u32) {
        let keys_to_remove: Vec<u64> = self.cache.iter()
            .filter_map(|(key_hash, cached)| {
                if cached.key.player_id == Some(player_id) {
                    Some(*key_hash)
                } else {
                    None
                }
            })
            .collect();

        for key_hash in keys_to_remove {
            if let Some(cached) = self.cache.remove(&key_hash) {
                self.spatial_index.remove_key(&cached.key, key_hash);
            }
        }
    }

    /// Advance world generation (invalidates all cache)
    pub fn advance_world_generation(&mut self) {
        self.world_generation += 1;
        self.cache.clear();
        self.spatial_index = SpatialIndex::default();
    }

    /// Clean up old cache entries
    pub fn cleanup(&mut self, max_age: std::time::Duration) {
        let keys_to_remove: Vec<u64> = self.cache.iter()
            .filter_map(|(key_hash, cached)| {
                if cached.age() > max_age {
                    Some(*key_hash)
                } else {
                    None
                }
            })
            .collect();

        for key_hash in keys_to_remove {
            if let Some(cached) = self.cache.remove(&key_hash) {
                self.spatial_index.remove_key(&cached.key, key_hash);
            }
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> SpatialCacheStats {
        let total_size = self.cache.values()
            .map(|cached| cached.result.size_bytes())
            .sum();

        let avg_access_count = if !self.cache.is_empty() {
            self.cache.values().map(|c| c.access_count as f64).sum::<f64>() / self.cache.len() as f64
        } else {
            0.0
        };

        SpatialCacheStats {
            entry_count: self.cache.len(),
            total_size_bytes: total_size,
            world_generation: self.world_generation,
            avg_access_count,
            position_index_size: self.spatial_index.position_to_keys.len(),
            region_index_size: self.spatial_index.region_to_keys.len(),
        }
    }

    /// Get current world generation
    pub fn world_generation(&self) -> u32 {
        self.world_generation
    }
}

/// Statistics for spatial cache
#[derive(Debug, Clone)]
pub struct SpatialCacheStats {
    pub entry_count: usize,
    pub total_size_bytes: usize,
    pub world_generation: u32,
    pub avg_access_count: f64,
    pub position_index_size: usize,
    pub region_index_size: usize,
}

impl Default for SpatialCache {
    fn default() -> Self {
        Self::new()
    }
}
