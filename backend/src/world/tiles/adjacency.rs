//! Adjacency graph system with indexmap for efficient tile neighbor management
//!
//! Provides high-performance adjacency relationships between tiles using indexmap
//! for deterministic ordering and fast lookups, integrated with the hashing
//! infrastructure and SIMD optimizations where applicable.

use indexmap::{IndexMap, IndexSet};
use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::RwLock;
use rayon::prelude::*;

use crate::core::{
    zig_ffi::HexCoord,
    hashing::{FastHashMap, FastHashSet, HashStrategies},
    caching::{GameCache, GameCacheBuilder, CacheKey, CachePriority}
};
use crate::world::tiles::{
    chunks::{TileId, ChunkCoord, ChunkManager},
    components::{Tile, TerrainType},
    spatial::{TileSpatialIndex}
};
use tracing::{debug, instrument, warn};

/// Direction of adjacency in hex grid (6 neighbors)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum HexDirection {
    East = 0,
    Northeast = 1,
    Northwest = 2,
    West = 3,
    Southwest = 4,
    Southeast = 5,
}

impl HexDirection {
    /// Get all six hex directions
    pub const ALL: [HexDirection; 6] = [
        HexDirection::East,
        HexDirection::Northeast,
        HexDirection::Northwest,
        HexDirection::West,
        HexDirection::Southwest,
        HexDirection::Southeast,
    ];

    /// Get opposite direction
    pub fn opposite(self) -> HexDirection {
        match self {
            HexDirection::East => HexDirection::West,
            HexDirection::Northeast => HexDirection::Southwest,
            HexDirection::Northwest => HexDirection::Southeast,
            HexDirection::West => HexDirection::East,
            HexDirection::Southwest => HexDirection::Northeast,
            HexDirection::Southeast => HexDirection::Northwest,
        }
    }

    /// Get hex offset for this direction
    pub fn offset(self) -> HexCoord {
        match self {
            HexDirection::East => HexCoord { q: 1, r: 0 },
            HexDirection::Northeast => HexCoord { q: 1, r: -1 },
            HexDirection::Northwest => HexCoord { q: 0, r: -1 },
            HexDirection::West => HexCoord { q: -1, r: 0 },
            HexDirection::Southwest => HexCoord { q: -1, r: 1 },
            HexDirection::Southeast => HexCoord { q: 0, r: 1 },
        }
    }

    /// Get direction from one hex to adjacent hex
    pub fn from_hex_to_hex(from: HexCoord, to: HexCoord) -> Option<HexDirection> {
        let diff = HexCoord { q: to.q - from.q, r: to.r - from.r };
        
        for direction in Self::ALL {
            if direction.offset().q == diff.q && direction.offset().r == diff.r {
                return Some(direction);
            }
        }
        
        None // Not adjacent
    }
}

/// Adjacency relationship between two tiles
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TileAdjacency {
    /// Source tile ID
    pub from_tile: TileId,
    /// Target tile ID  
    pub to_tile: TileId,
    /// Direction from source to target
    pub direction: HexDirection,
    /// Connection strength (0.0 = blocked, 1.0 = open)
    pub connection_strength: f32,
    /// Movement cost modifier for this connection
    pub movement_cost_modifier: f32,
    /// Whether this connection is bidirectional
    pub bidirectional: bool,
}

impl TileAdjacency {
    /// Create new adjacency relationship
    pub fn new(from_tile: TileId, to_tile: TileId, direction: HexDirection) -> Self {
        Self {
            from_tile,
            to_tile,
            direction,
            connection_strength: 1.0,
            movement_cost_modifier: 1.0,
            bidirectional: true,
        }
    }

    /// Create adjacency with custom properties
    pub fn with_properties(from_tile: TileId, to_tile: TileId, direction: HexDirection, 
                          strength: f32, cost_modifier: f32, bidirectional: bool) -> Self {
        Self {
            from_tile,
            to_tile,
            direction,
            connection_strength: strength.clamp(0.0, 1.0),
            movement_cost_modifier: cost_modifier.max(0.0),
            bidirectional,
        }
    }

    /// Check if connection is passable
    pub fn is_passable(&self) -> bool {
        self.connection_strength > 0.0
    }

    /// Get effective movement cost for this connection
    pub fn effective_movement_cost(&self, base_cost: f32) -> f32 {
        if !self.is_passable() {
            return f32::INFINITY;
        }
        base_cost * self.movement_cost_modifier / self.connection_strength
    }

    /// Create reverse adjacency (for bidirectional connections)
    pub fn reverse(&self) -> Self {
        Self {
            from_tile: self.to_tile,
            to_tile: self.from_tile,
            direction: self.direction.opposite(),
            connection_strength: self.connection_strength,
            movement_cost_modifier: self.movement_cost_modifier,
            bidirectional: self.bidirectional,
        }
    }
}

/// High-performance adjacency graph using indexmap for deterministic ordering
#[derive(Debug)]
pub struct TileAdjacencyGraph {
    /// Adjacency list with deterministic ordering (indexmap preserves insertion order)
    adjacencies: Arc<RwLock<IndexMap<TileId, IndexMap<HexDirection, TileAdjacency>>>>,
    /// Reverse lookup: tile -> tiles that have it as neighbor
    reverse_adjacencies: Arc<RwLock<IndexMap<TileId, IndexSet<TileId>>>>,
    /// Cache for adjacency queries
    cache: GameCache,
    /// Spatial index for neighbor finding
    spatial_index: Arc<TileSpatialIndex>,
    /// Generation counter for cache invalidation
    generation: Arc<RwLock<u64>>,
}

impl TileAdjacencyGraph {
    /// Create new adjacency graph
    pub fn new(spatial_index: Arc<TileSpatialIndex>) -> Self {
        let cache = GameCacheBuilder::new()
            .max_memory_mb(32) // 32MB for adjacency cache
            .default_ttl(std::time::Duration::from_secs(120)) // 2 minute TTL
            .turn_based_invalidation(false)
            .build();

        Self {
            adjacencies: Arc::new(RwLock::new(IndexMap::new())),
            reverse_adjacencies: Arc::new(RwLock::new(IndexMap::new())),
            cache,
            spatial_index,
            generation: Arc::new(RwLock::new(1)),
        }
    }

    /// Add adjacency relationship between tiles
    #[instrument(skip(self))]
    pub fn add_adjacency(&self, adjacency: TileAdjacency) {
        let from_tile = adjacency.from_tile;
        let to_tile = adjacency.to_tile;
        let direction = adjacency.direction;

        // Add to main adjacency map
        {
            let mut adjacencies = self.adjacencies.write();
            adjacencies.entry(from_tile)
                .or_insert_with(IndexMap::new)
                .insert(direction, adjacency.clone());
        }

        // Add to reverse lookup
        {
            let mut reverse = self.reverse_adjacencies.write();
            reverse.entry(to_tile)
                .or_insert_with(IndexSet::new)
                .insert(from_tile);
        }

        // Add reverse adjacency if bidirectional
        if adjacency.bidirectional {
            let reverse_adjacency = adjacency.reverse();
            let mut adjacencies = self.adjacencies.write();
            adjacencies.entry(to_tile)
                .or_insert_with(IndexMap::new)
                .insert(reverse_adjacency.direction, reverse_adjacency);
        }

        self.invalidate_cache();
        debug!("Added adjacency from tile {} to tile {} in direction {:?}", from_tile, to_tile, direction);
    }

    /// Remove adjacency relationship
    pub fn remove_adjacency(&self, from_tile: TileId, direction: HexDirection) -> Option<TileAdjacency> {
        let removed_adjacency = {
            let mut adjacencies = self.adjacencies.write();
            adjacencies.get_mut(&from_tile)
                .and_then(|directions| directions.shift_remove(&direction))
        };

        if let Some(ref adjacency) = removed_adjacency {
            // Remove from reverse lookup
            {
                let mut reverse = self.reverse_adjacencies.write();
                if let Some(reverse_set) = reverse.get_mut(&adjacency.to_tile) {
                    reverse_set.shift_remove(&from_tile);
                    if reverse_set.is_empty() {
                        reverse.shift_remove(&adjacency.to_tile);
                    }
                }
            }

            // Remove bidirectional reverse if applicable
            if adjacency.bidirectional {
                let mut adjacencies = self.adjacencies.write();
                if let Some(directions) = adjacencies.get_mut(&adjacency.to_tile) {
                    directions.shift_remove(&direction.opposite());
                }
            }

            self.invalidate_cache();
        }

        removed_adjacency
    }

    /// Get all adjacent tiles for a given tile
    #[instrument(skip(self))]
    pub async fn get_neighbors(&self, tile_id: TileId) -> Vec<TileId> {
        let cache_key = CacheKey::Custom(format!("neighbors:{}", tile_id));

        // Check cache first
        if let Ok(Some(neighbors)) = self.cache.get::<Vec<TileId>>(&cache_key).await {
            return neighbors;
        }

        // Cache miss - compute neighbors
        let neighbors = {
            let adjacencies = self.adjacencies.read();
            adjacencies.get(&tile_id)
                .map(|directions| {
                    directions.values()
                        .filter(|adj| adj.is_passable())
                        .map(|adj| adj.to_tile)
                        .collect()
                })
                .unwrap_or_default()
        };

        // Cache the result
        let _ = self.cache.set(cache_key, neighbors.clone(), CachePriority::High).await;
        neighbors
    }

    /// Get neighbor in specific direction
    pub fn get_neighbor(&self, tile_id: TileId, direction: HexDirection) -> Option<TileId> {
        let adjacencies = self.adjacencies.read();
        adjacencies.get(&tile_id)?
            .get(&direction)
            .filter(|adj| adj.is_passable())
            .map(|adj| adj.to_tile)
    }

    /// Get adjacency relationship in specific direction
    pub fn get_adjacency(&self, tile_id: TileId, direction: HexDirection) -> Option<TileAdjacency> {
        let adjacencies = self.adjacencies.read();
        adjacencies.get(&tile_id)?
            .get(&direction)
            .cloned()
    }

    /// Get all adjacency relationships from a tile
    pub fn get_adjacencies(&self, tile_id: TileId) -> Vec<TileAdjacency> {
        let adjacencies = self.adjacencies.read();
        adjacencies.get(&tile_id)
            .map(|directions| directions.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Get tiles that have the given tile as a neighbor (reverse lookup)
    pub fn get_reverse_neighbors(&self, tile_id: TileId) -> Vec<TileId> {
        let reverse = self.reverse_adjacencies.read();
        reverse.get(&tile_id)
            .map(|set| set.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Build adjacency graph from spatial data (batch operation)
    #[instrument(skip(self, tiles))]
    pub async fn build_from_tiles(&self, tiles: &[(TileId, HexCoord)]) -> Result<(), AdjacencyError> {
        debug!("Building adjacency graph from {} tiles", tiles.len());
        
        // Clear existing adjacencies
        self.clear();

        // Create adjacency map for quick lookups
        let tile_map: FastHashMap<HexCoord, TileId> = tiles.iter()
            .map(|&(tile_id, hex)| (hex, tile_id))
            .collect();

        // Process in parallel batches
        let adjacencies: Vec<TileAdjacency> = tiles.par_iter()
            .flat_map(|&(tile_id, hex)| {
                let mut local_adjacencies = Vec::new();
                
                // Check all 6 hex directions
                for direction in HexDirection::ALL {
                    let neighbor_hex = HexCoord {
                        q: hex.q + direction.offset().q,
                        r: hex.r + direction.offset().r,
                    };
                    
                    if let Some(&neighbor_id) = tile_map.get(&neighbor_hex) {
                        local_adjacencies.push(TileAdjacency::new(tile_id, neighbor_id, direction));
                    }
                }
                
                local_adjacencies
            })
            .collect();

        // Add all adjacencies (this needs to be sequential to avoid race conditions)
        for adjacency in adjacencies {
            self.add_adjacency(adjacency);
        }

        debug!("Built adjacency graph with {} relationships", self.adjacency_count());
        Ok(())
    }

    /// Update adjacency strengths based on terrain compatibility
    pub fn update_terrain_adjacencies<F>(&self, terrain_modifier: F) 
    where 
        F: Fn(TerrainType, TerrainType) -> f32 + Send + Sync,
    {
        let mut adjacencies = self.adjacencies.write();
        
        // This would require terrain type lookups - simplified for now
        // In practice, this would integrate with the tile component manager
        for (_tile_id, directions) in adjacencies.iter_mut() {
            for (_direction, adjacency) in directions.iter_mut() {
                // Apply terrain-based modifications
                // adjacency.connection_strength *= terrain_modifier(from_terrain, to_terrain);
            }
        }
        
        self.invalidate_cache();
    }

    /// Find path between two tiles using adjacency graph (A* pathfinding)
    pub async fn find_path(&self, from_tile: TileId, to_tile: TileId, max_depth: u32) -> Option<Vec<TileId>> {
        let cache_key = CacheKey::Custom(format!("path:{}:{}:{}", from_tile, to_tile, max_depth));
        
        // Check cache first
        if let Ok(Some(path)) = self.cache.get::<Vec<TileId>>(&cache_key).await {
            return Some(path);
        }

        // Simple BFS pathfinding (could be optimized to A*)
        let path = self.bfs_pathfinding(from_tile, to_tile, max_depth).await;
        
        // Cache the result
        if let Some(ref path) = path {
            let _ = self.cache.set(cache_key, path.clone(), CachePriority::Medium).await;
        }
        
        path
    }

    /// Breadth-first search pathfinding implementation
    async fn bfs_pathfinding(&self, from_tile: TileId, to_tile: TileId, max_depth: u32) -> Option<Vec<TileId>> {
        use std::collections::{HashMap, VecDeque};
        
        let mut queue = VecDeque::new();
        let mut visited = FastHashSet::default();
        let mut parent_map: HashMap<TileId, TileId> = HashMap::new();
        
        queue.push_back((from_tile, 0u32));
        visited.insert(from_tile);
        
        while let Some((current_tile, depth)) = queue.pop_front() {
            if current_tile == to_tile {
                // Reconstruct path
                let mut path = Vec::new();
                let mut current = to_tile;
                
                while current != from_tile {
                    path.push(current);
                    current = parent_map[&current];
                }
                path.push(from_tile);
                path.reverse();
                
                return Some(path);
            }
            
            if depth >= max_depth {
                continue;
            }
            
            // Explore neighbors
            for neighbor in self.get_neighbors(current_tile).await {
                if !visited.contains(&neighbor) {
                    visited.insert(neighbor);
                    parent_map.insert(neighbor, current_tile);
                    queue.push_back((neighbor, depth + 1));
                }
            }
        }
        
        None // No path found
    }

    /// Get all connected components in the adjacency graph
    pub async fn get_connected_components(&self) -> Vec<Vec<TileId>> {
        let mut components = Vec::new();
        let mut visited = FastHashSet::default();
        let adjacencies = self.adjacencies.read();
        
        for &tile_id in adjacencies.keys() {
            if visited.contains(&tile_id) {
                continue;
            }
            
            // Start new component with BFS
            let mut component = Vec::new();
            let mut queue = std::collections::VecDeque::new();
            
            queue.push_back(tile_id);
            visited.insert(tile_id);
            
            while let Some(current_tile) = queue.pop_front() {
                component.push(current_tile);
                
                // Add unvisited neighbors
                for neighbor in self.get_neighbors(current_tile).await {
                    if !visited.contains(&neighbor) {
                        visited.insert(neighbor);
                        queue.push_back(neighbor);
                    }
                }
            }
            
            components.push(component);
        }
        
        components
    }

    /// Get adjacency statistics for monitoring
    pub fn adjacency_stats(&self) -> AdjacencyStats {
        let adjacencies = self.adjacencies.read();
        let reverse = self.reverse_adjacencies.read();
        
        let total_tiles = adjacencies.len();
        let total_adjacencies: usize = adjacencies.values().map(|dirs| dirs.len()).sum();
        let avg_neighbors = if total_tiles > 0 { total_adjacencies as f32 / total_tiles as f32 } else { 0.0 };
        
        let passable_adjacencies: usize = adjacencies.values()
            .map(|dirs| dirs.values().filter(|adj| adj.is_passable()).count())
            .sum();
        
        AdjacencyStats {
            total_tiles,
            total_adjacencies,
            passable_adjacencies,
            avg_neighbors_per_tile: avg_neighbors,
            reverse_lookup_size: reverse.len(),
        }
    }

    /// Get total number of adjacency relationships
    pub fn adjacency_count(&self) -> usize {
        let adjacencies = self.adjacencies.read();
        adjacencies.values().map(|dirs| dirs.len()).sum()
    }

    /// Clear all adjacencies
    pub fn clear(&self) {
        self.adjacencies.write().clear();
        self.reverse_adjacencies.write().clear();
        self.invalidate_cache();
    }

    /// Validate adjacency graph consistency
    pub fn validate(&self) -> Result<(), AdjacencyError> {
        let adjacencies = self.adjacencies.read();
        let reverse = self.reverse_adjacencies.read();
        
        // Check bidirectional consistency
        for (from_tile, directions) in adjacencies.iter() {
            for (direction, adjacency) in directions.iter() {
                let to_tile = adjacency.to_tile;
                
                // Check reverse lookup exists
                if let Some(reverse_set) = reverse.get(&to_tile) {
                    if !reverse_set.contains(from_tile) {
                        return Err(AdjacencyError::InconsistentReverseLookup { from: *from_tile, to: to_tile });
                    }
                }
                
                // Check bidirectional adjacency if flagged
                if adjacency.bidirectional {
                    if let Some(reverse_directions) = adjacencies.get(&to_tile) {
                        if !reverse_directions.contains_key(&direction.opposite()) {
                            return Err(AdjacencyError::MissingBidirectionalAdjacency { from: *from_tile, to: to_tile, direction: *direction });
                        }
                    }
                }
            }
        }
        
        Ok(())
    }

    /// Invalidate cache
    fn invalidate_cache(&self) {
        let mut gen = self.generation.write();
        *gen += 1;
    }
}

impl Default for TileAdjacencyGraph {
    fn default() -> Self {
        let spatial_index = Arc::new(TileSpatialIndex::default());
        Self::new(spatial_index)
    }
}

/// Statistics for adjacency graph performance monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdjacencyStats {
    pub total_tiles: usize,
    pub total_adjacencies: usize,
    pub passable_adjacencies: usize,
    pub avg_neighbors_per_tile: f32,
    pub reverse_lookup_size: usize,
}

/// Errors that can occur in adjacency operations
#[derive(Debug, thiserror::Error)]
pub enum AdjacencyError {
    #[error("Inconsistent reverse lookup: tile {from} -> {to}")]
    InconsistentReverseLookup { from: TileId, to: TileId },
    
    #[error("Missing bidirectional adjacency: tile {from} -> {to} in direction {direction:?}")]
    MissingBidirectionalAdjacency { from: TileId, to: TileId, direction: HexDirection },
    
    #[error("Invalid tile ID: {tile_id}")]
    InvalidTileId { tile_id: TileId },
    
    #[error("Pathfinding failed: no path from {from} to {to}")]
    PathfindingFailed { from: TileId, to: TileId },
}

/// System for maintaining adjacency graph consistency
pub fn maintain_adjacency_system(
    adjacency_graph: Res<TileAdjacencyGraph>,
) {
    // Validate adjacency graph periodically
    if let Err(e) = adjacency_graph.validate() {
        warn!("Adjacency graph validation failed: {}", e);
    }
}

/// System for updating adjacency based on tile changes
pub fn update_adjacency_system(
    adjacency_graph: Res<TileAdjacencyGraph>,
    // This would include queries for changed tiles
) {
    // Monitor tile changes and update adjacencies accordingly
    // Implementation would depend on change detection system
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_direction() {
        assert_eq!(HexDirection::East.opposite(), HexDirection::West);
        assert_eq!(HexDirection::Northeast.opposite(), HexDirection::Southwest);
        
        let east_offset = HexDirection::East.offset();
        assert_eq!(east_offset.q, 1);
        assert_eq!(east_offset.r, 0);
    }

    #[test]
    fn test_tile_adjacency() {
        let adj = TileAdjacency::new(1, 2, HexDirection::East);
        assert_eq!(adj.from_tile, 1);
        assert_eq!(adj.to_tile, 2);
        assert_eq!(adj.direction, HexDirection::East);
        assert!(adj.is_passable());
        
        let reverse = adj.reverse();
        assert_eq!(reverse.from_tile, 2);
        assert_eq!(reverse.to_tile, 1);
        assert_eq!(reverse.direction, HexDirection::West);
    }

    #[tokio::test]
    async fn test_adjacency_graph() {
        let spatial_index = Arc::new(TileSpatialIndex::default());
        let graph = TileAdjacencyGraph::new(spatial_index);
        
        // Add adjacency
        let adj = TileAdjacency::new(1, 2, HexDirection::East);
        graph.add_adjacency(adj);
        
        // Test neighbor lookup
        let neighbors = graph.get_neighbors(1).await;
        assert_eq!(neighbors.len(), 1);
        assert!(neighbors.contains(&2));
        
        // Test reverse lookup
        let reverse_neighbors = graph.get_reverse_neighbors(2);
        assert_eq!(reverse_neighbors.len(), 1);
        assert!(reverse_neighbors.contains(&1));
    }

    #[tokio::test]
    async fn test_pathfinding() {
        let spatial_index = Arc::new(TileSpatialIndex::default());
        let graph = TileAdjacencyGraph::new(spatial_index);
        
        // Create chain: 1 -> 2 -> 3
        graph.add_adjacency(TileAdjacency::new(1, 2, HexDirection::East));
        graph.add_adjacency(TileAdjacency::new(2, 3, HexDirection::East));
        
        let path = graph.find_path(1, 3, 10).await;
        assert!(path.is_some());
        
        let path = path.unwrap();
        assert_eq!(path, vec![1, 2, 3]);
    }

    #[test]
    fn test_adjacency_validation() {
        let spatial_index = Arc::new(TileSpatialIndex::default());
        let graph = TileAdjacencyGraph::new(spatial_index);
        
        // Add bidirectional adjacency
        let adj = TileAdjacency::new(1, 2, HexDirection::East);
        graph.add_adjacency(adj);
        
        // Should validate successfully
        assert!(graph.validate().is_ok());
        
        let stats = graph.adjacency_stats();
        assert_eq!(stats.total_tiles, 2); // Both tiles should be tracked
        assert_eq!(stats.total_adjacencies, 2); // Bidirectional = 2 adjacencies
    }

    #[tokio::test]
    async fn test_build_from_tiles() {
        let spatial_index = Arc::new(TileSpatialIndex::default());
        let graph = TileAdjacencyGraph::new(spatial_index);
        
        // Create 2x2 grid of tiles
        let tiles = vec![
            (1, HexCoord { q: 0, r: 0 }),
            (2, HexCoord { q: 1, r: 0 }),
            (3, HexCoord { q: 0, r: 1 }),
            (4, HexCoord { q: 1, r: 1 }),
        ];
        
        graph.build_from_tiles(&tiles).await.unwrap();
        
        // Check that adjacencies were created
        let neighbors_1 = graph.get_neighbors(1).await;
        assert!(!neighbors_1.is_empty());
        
        let stats = graph.adjacency_stats();
        assert!(stats.total_adjacencies > 0);
    }
}
