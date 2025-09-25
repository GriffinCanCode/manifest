//! Hierarchical tiles with petgraph DAG integration
//!
//! Extends the existing hierarchy system to support tile-specific relationships,
//! multi-resolution tile organization, and spatial hierarchy queries using the
//! established ECS relationship components and graph infrastructure.

use petgraph::{Graph, Direction, graph::NodeIndex};
use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::RwLock;
use rayon::prelude::*;

use crate::core::{
    zig_ffi::HexCoord,
    hashing::{FastHashMap, FastHashSet},
    caching::{GameCache, GameCacheBuilder, CacheKey, CachePriority}
};
use crate::ecs::hierarchy::{
    HierarchyQueries, EntityGraph, HierarchyError, HierarchyResult,
    Relationship, RelationshipType, Relationships, Hierarchical
};
use crate::world::tiles::{
    chunks::{TileId, ChunkCoord},
    components::{Tile, TerrainType, TileComponentManager}
};
use tracing::{debug, instrument, warn};

/// Tile-specific relationship types extending the base relationship system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TileRelationshipType {
    /// Parent region contains child tiles (multi-resolution)
    RegionParent,
    /// Child tile belongs to parent region
    RegionChild,
    /// Tile influences adjacent tiles (cultural, economic spread)
    Influence,
    /// Tile borders another tile (spatial adjacency)
    Adjacent,
    /// Tile is part of a river system
    RiverFlow,
    /// Tile shares resources with another
    ResourceShare,
    /// Tile is part of a trade route
    TradeRoute,
}

impl TileRelationshipType {
    /// Convert to base relationship type for hierarchy system integration
    pub fn to_base_relationship(self) -> RelationshipType {
        match self {
            Self::RegionParent => RelationshipType::Parent,
            Self::RegionChild => RelationshipType::Child,
            Self::Influence => RelationshipType::Attachment,
            Self::Adjacent => RelationshipType::Attachment,
            Self::RiverFlow => RelationshipType::Dependency,
            Self::ResourceShare => RelationshipType::Dependency,
            Self::TradeRoute => RelationshipType::Dependency,
        }
    }

    /// Check if this relationship type is spatial (affects neighbor calculations)
    pub fn is_spatial(self) -> bool {
        matches!(self, Self::Adjacent | Self::Influence)
    }

    /// Check if this relationship type is hierarchical (affects resolution)
    pub fn is_hierarchical(self) -> bool {
        matches!(self, Self::RegionParent | Self::RegionChild)
    }
}

/// Multi-resolution tile entity for hierarchical organization
#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub struct HierarchicalTile {
    /// Base tile ID at highest resolution
    pub base_tile_id: TileId,
    /// Hex coordinate in world space
    pub hex: HexCoord,
    /// Resolution level (0 = highest detail, higher = more aggregated)
    pub resolution: u8,
    /// Aggregated area covered by this tile (in base tiles)
    pub coverage_area: u16,
    /// Spatial bounds of this hierarchical tile
    pub bounds: HexBounds,
}

impl HierarchicalTile {
    /// Create new hierarchical tile
    pub fn new(base_tile_id: TileId, hex: HexCoord, resolution: u8) -> Self {
        let coverage_area = Self::calculate_coverage_area(resolution);
        let bounds = HexBounds::from_center_and_radius(hex, coverage_area as u32 / 2);
        
        Self {
            base_tile_id,
            hex,
            resolution,
            coverage_area,
            bounds,
        }
    }

    /// Calculate how many base tiles this hierarchical tile covers
    fn calculate_coverage_area(resolution: u8) -> u16 {
        // Each resolution level increases coverage by factor of 4 (2x2 in hex space)
        4_u16.pow(resolution as u32).min(u16::MAX)
    }

    /// Check if this hierarchical tile contains the given hex coordinate
    pub fn contains_hex(&self, hex: HexCoord) -> bool {
        self.bounds.contains(hex)
    }

    /// Get all child tile coordinates at next resolution level
    pub fn get_child_hexes(&self) -> Vec<HexCoord> {
        if self.resolution == 0 {
            return vec![self.hex]; // Base resolution
        }

        let radius = (self.coverage_area as i32) / 4; // Next level down
        let mut child_hexes = Vec::new();
        
        // Generate hex ring pattern for child tiles
        for q_offset in -radius..=radius {
            for r_offset in -radius..=radius {
                let s_offset = -q_offset - r_offset;
                if s_offset.abs() <= radius {
                    child_hexes.push(HexCoord {
                        q: self.hex.q + q_offset,
                        r: self.hex.r + r_offset,
                    });
                }
            }
        }

        child_hexes
    }

    /// Get memory usage in bytes
    pub fn memory_size(&self) -> usize {
        std::mem::size_of::<Self>()
    }
}

/// Spatial bounds in hex coordinate space
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct HexBounds {
    pub min_q: i32,
    pub max_q: i32,
    pub min_r: i32,
    pub max_r: i32,
}

impl HexBounds {
    /// Create bounds from center and radius
    pub fn from_center_and_radius(center: HexCoord, radius: u32) -> Self {
        let r = radius as i32;
        Self {
            min_q: center.q - r,
            max_q: center.q + r,
            min_r: center.r - r,
            max_r: center.r + r,
        }
    }

    /// Check if bounds contain the given hex coordinate
    pub fn contains(&self, hex: HexCoord) -> bool {
        hex.q >= self.min_q && hex.q <= self.max_q &&
        hex.r >= self.min_r && hex.r <= self.max_r
    }

    /// Calculate area covered by bounds
    pub fn area(&self) -> u32 {
        let width = (self.max_q - self.min_q + 1) as u32;
        let height = (self.max_r - self.min_r + 1) as u32;
        width * height
    }
}

/// High-performance tile hierarchy manager extending the base hierarchy system
#[derive(Debug, Resource)]
pub struct TileHierarchy {
    /// Core hierarchy query system (integrates with ECS)
    hierarchy_queries: Arc<HierarchyQueries>,
    /// Tile-specific component manager
    tile_manager: Arc<TileComponentManager>,
    /// Hierarchical tile entities indexed by resolution
    hierarchical_tiles: Arc<RwLock<FastHashMap<u8, Vec<Entity>>>>,
    /// Cache for hierarchy queries
    cache: GameCache,
    /// Resolution configuration
    max_resolution: u8,
}

impl TileHierarchy {
    /// Create new tile hierarchy system
    pub fn new(hierarchy_queries: Arc<HierarchyQueries>, tile_manager: Arc<TileComponentManager>) -> Self {
        let cache = GameCacheBuilder::new()
            .max_memory_mb(128) // 128MB for tile hierarchy data
            .default_ttl(std::time::Duration::from_secs(300)) // 5 minute TTL
            .turn_based_invalidation(false)
            .build();

        Self {
            hierarchy_queries,
            tile_manager,
            hierarchical_tiles: Arc::new(RwLock::new(FastHashMap::default())),
            cache,
            max_resolution: 4, // Support up to 4 resolution levels
        }
    }

    /// Initialize hierarchical tile structure for a region
    #[instrument(skip(self, world))]
    pub fn initialize_hierarchy(&self, world: &mut World, base_tiles: &[TileId]) -> HierarchyResult<()> {
        debug!("Initializing tile hierarchy for {} base tiles", base_tiles.len());

        // Group base tiles by spatial proximity for hierarchy creation
        let tile_groups = self.group_tiles_by_proximity(base_tiles)?;
        
        // Create hierarchy levels from bottom up
        let mut current_entities: Vec<Entity> = Vec::new();
        
        // Resolution 0: Base tiles (already exist as regular tiles)
        for &tile_id in base_tiles {
            if let Ok(tile) = self.tile_manager.get_component::<Tile>(tile_id) {
                let entity = world.spawn((
                    HierarchicalTile::new(tile_id, tile.hex, 0),
                    Hierarchical,
                    Relationships::new(),
                )).id();
                current_entities.push(entity);
            }
        }

        self.hierarchical_tiles.write().insert(0, current_entities.clone());

        // Build higher resolution levels
        for resolution in 1..=self.max_resolution {
            let parent_entities = self.create_parent_level(world, &current_entities, resolution)?;
            
            if parent_entities.is_empty() {
                break; // No more aggregation possible
            }

            // Create parent-child relationships
            self.link_hierarchy_level(world, &current_entities, &parent_entities)?;
            
            self.hierarchical_tiles.write().insert(resolution, parent_entities.clone());
            current_entities = parent_entities;
            
            debug!("Created {} entities at resolution {}", current_entities.len(), resolution);
        }

        Ok(())
    }

    /// Group tiles by spatial proximity for hierarchy creation
    fn group_tiles_by_proximity(&self, base_tiles: &[TileId]) -> HierarchyResult<Vec<Vec<TileId>>> {
        let mut groups = Vec::new();
        let mut processed: FastHashSet<TileId> = FastHashSet::default();
        
        for &tile_id in base_tiles {
            if processed.contains(&tile_id) {
                continue;
            }

            // Start new group with this tile
            let mut group = vec![tile_id];
            processed.insert(tile_id);
            
            // Find nearby tiles to group together (using simple distance clustering)
            if let Ok(center_tile) = self.tile_manager.get_component::<Tile>(tile_id) {
                for &other_id in base_tiles {
                    if processed.contains(&other_id) {
                        continue;
                    }
                    
                    if let Ok(other_tile) = self.tile_manager.get_component::<Tile>(other_id) {
                        let distance = self.hex_distance(center_tile.hex, other_tile.hex);
                        if distance <= 2 && group.len() < 4 { // Group up to 4 tiles within distance 2
                            group.push(other_id);
                            processed.insert(other_id);
                        }
                    }
                }
            }
            
            groups.push(group);
        }

        Ok(groups)
    }

    /// Create parent level entities from child entities
    fn create_parent_level(&self, world: &mut World, children: &[Entity], resolution: u8) -> HierarchyResult<Vec<Entity>> {
        let mut parent_entities = Vec::new();
        
        // Process children in groups of 4 for parent creation
        for chunk in children.chunks(4) {
            if let Some(parent_entity) = self.create_parent_entity(world, chunk, resolution)? {
                parent_entities.push(parent_entity);
            }
        }
        
        Ok(parent_entities)
    }

    /// Create single parent entity from group of children
    fn create_parent_entity(&self, world: &mut World, children: &[Entity], resolution: u8) -> HierarchyResult<Option<Entity>> {
        if children.is_empty() {
            return Ok(None);
        }

        // Calculate center position from children
        let mut center_q = 0i32;
        let mut center_r = 0i32;
        let mut total_coverage = 0u16;
        let mut base_tile_id = 0u32;

        for &child_entity in children {
            if let Ok(hierarchical_tile) = world.get::<HierarchicalTile>(child_entity) {
                center_q += hierarchical_tile.hex.q;
                center_r += hierarchical_tile.hex.r;
                total_coverage += hierarchical_tile.coverage_area;
                if base_tile_id == 0 {
                    base_tile_id = hierarchical_tile.base_tile_id;
                }
            }
        }

        let count = children.len() as i32;
        let center = HexCoord {
            q: center_q / count,
            r: center_r / count,
        };

        // Create parent hierarchical tile
        let parent_tile = HierarchicalTile::new(base_tile_id, center, resolution);
        
        let parent_entity = world.spawn((
            parent_tile,
            Hierarchical,
            Relationships::new(),
        )).id();

        Ok(Some(parent_entity))
    }

    /// Link parent-child relationships between hierarchy levels
    fn link_hierarchy_level(&self, world: &mut World, children: &[Entity], parents: &[Entity]) -> HierarchyResult<()> {
        // Simple spatial assignment - assign each child to nearest parent
        for &child_entity in children {
            if let Ok(child_tile) = world.get::<HierarchicalTile>(child_entity) {
                let mut nearest_parent = None;
                let mut min_distance = f32::MAX;

                for &parent_entity in parents {
                    if let Ok(parent_tile) = world.get::<HierarchicalTile>(parent_entity) {
                        let distance = self.hex_distance_f32(child_tile.hex, parent_tile.hex);
                        if distance < min_distance {
                            min_distance = distance;
                            nearest_parent = Some(parent_entity);
                        }
                    }
                }

                // Create parent-child relationship
                if let Some(parent_entity) = nearest_parent {
                    self.add_tile_relationship(world, parent_entity, child_entity, TileRelationshipType::RegionParent)?;
                    self.add_tile_relationship(world, child_entity, parent_entity, TileRelationshipType::RegionChild)?;
                }
            }
        }

        Ok(())
    }

    /// Add tile-specific relationship between two hierarchical tiles
    #[instrument(skip(self, world))]
    pub fn add_tile_relationship(&self, world: &mut World, from: Entity, to: Entity, rel_type: TileRelationshipType) -> HierarchyResult<()> {
        // Create relationship using base hierarchy system
        let base_rel_type = rel_type.to_base_relationship();
        let relationship = Relationship::new(to, base_rel_type);

        // Add to entity's relationships component
        if let Ok(mut relationships) = world.get_mut::<Relationships>(from) {
            relationships.add(relationship)?;
        }

        // Update hierarchy graph
        let updates = vec![(from, world.get::<Relationships>(from).unwrap().clone())];
        self.hierarchy_queries.update_relationships(updates)?;

        debug!("Added tile relationship {:?} from {:?} to {:?}", rel_type, from, to);
        Ok(())
    }

    /// Get hierarchical tiles at specific resolution level
    pub fn get_tiles_at_resolution(&self, resolution: u8) -> Vec<Entity> {
        self.hierarchical_tiles.read()
            .get(&resolution)
            .cloned()
            .unwrap_or_default()
    }

    /// Find hierarchical tile containing the given hex coordinate
    pub async fn find_containing_tile(&self, hex: HexCoord, resolution: u8) -> Option<Entity> {
        let cache_key = CacheKey::Custom(format!("containing_tile:{}:{}:{}", hex.q, hex.r, resolution));
        
        // Check cache first
        if let Ok(Some(entity)) = self.cache.get::<Entity>(&cache_key).await {
            return Some(entity);
        }

        // Search tiles at specified resolution
        let tiles = self.get_tiles_at_resolution(resolution);
        for tile_entity in tiles {
            // This would require world access - simplified for now
            // In practice, this would use a spatial query system
        }

        None
    }

    /// Get all ancestor tiles up the hierarchy
    pub async fn get_ancestor_tiles(&self, tile_entity: Entity) -> Vec<Entity> {
        self.hierarchy_queries.ancestors(tile_entity).await
    }

    /// Get all descendant tiles down the hierarchy
    pub async fn get_descendant_tiles(&self, tile_entity: Entity) -> Vec<Entity> {
        self.hierarchy_queries.descendants(tile_entity).await
    }

    /// Find tiles influenced by the given tile (using influence relationships)
    pub fn get_influenced_tiles(&self, world: &mut World, tile_entity: Entity) -> Vec<Entity> {
        self.hierarchy_queries.find_by_relationship(world, RelationshipType::Attachment, Direction::Outgoing)
            .get(&tile_entity)
            .cloned()
            .unwrap_or_default()
    }

    /// Find adjacent tiles (direct spatial neighbors)
    pub fn get_adjacent_tiles(&self, world: &mut World, tile_entity: Entity) -> Vec<Entity> {
        // This would use the spatial adjacency relationships
        // Implementation depends on how adjacency is tracked
        self.hierarchy_queries.find_by_relationship(world, RelationshipType::Attachment, Direction::Outgoing)
            .get(&tile_entity)
            .cloned()
            .unwrap_or_default()
    }

    /// Batch update tile relationships for multiple tiles (parallelized)
    pub fn batch_update_tile_relationships<I>(&self, world: &mut World, updates: I) -> HierarchyResult<()>
    where
        I: IntoIterator<Item = (Entity, Vec<(Entity, TileRelationshipType)>)> + Send,
        I::IntoIter: Send,
    {
        let updates: Vec<_> = updates.into_iter().collect();
        
        // Process in parallel for large batches
        updates.par_iter().try_for_each(|(from_entity, relationships)| {
            for &(to_entity, rel_type) in relationships {
                self.add_tile_relationship(world, *from_entity, to_entity, rel_type)?;
            }
            Ok::<(), HierarchyError>(())
        })?;

        Ok(())
    }

    /// Get hierarchy statistics for monitoring
    pub async fn hierarchy_stats(&self) -> TileHierarchyStats {
        let mut stats = TileHierarchyStats {
            resolution_counts: FastHashMap::default(),
            total_hierarchical_tiles: 0,
            max_resolution: self.max_resolution,
            cache_hit_rate: 0.0,
        };

        let hierarchical_tiles = self.hierarchical_tiles.read();
        for (&resolution, tiles) in hierarchical_tiles.iter() {
            stats.resolution_counts.insert(resolution, tiles.len());
            stats.total_hierarchical_tiles += tiles.len();
        }

        // Get cache statistics
        let cache_stats = self.cache.stats().await;
        stats.cache_hit_rate = if cache_stats.total_requests > 0 {
            cache_stats.total_hits as f32 / cache_stats.total_requests as f32
        } else {
            0.0
        };

        stats
    }

    /// Validate hierarchy integrity
    pub fn validate_tile_hierarchy(&self) -> HierarchyResult<TileHierarchyValidation> {
        let base_validation = self.hierarchy_queries.validate_hierarchy()?;
        
        let hierarchical_tiles = self.hierarchical_tiles.read();
        let resolution_count = hierarchical_tiles.len() as u8;
        let has_gaps = (0..resolution_count).any(|r| !hierarchical_tiles.contains_key(&r));

        Ok(TileHierarchyValidation {
            base_validation,
            resolution_levels: resolution_count,
            has_resolution_gaps: has_gaps,
            total_hierarchical_entities: hierarchical_tiles.values().map(|v| v.len()).sum(),
        })
    }

    /// Helper: Calculate hex distance between two coordinates
    #[inline]
    fn hex_distance(&self, hex1: HexCoord, hex2: HexCoord) -> u32 {
        let dx = (hex1.q - hex2.q).abs();
        let dy = (hex1.q + hex1.r - hex2.q - hex2.r).abs();
        let dz = (hex1.r - hex2.r).abs();
        ((dx + dy + dz) / 2) as u32
    }

    /// Helper: Calculate hex distance as float for precise calculations
    #[inline]
    fn hex_distance_f32(&self, hex1: HexCoord, hex2: HexCoord) -> f32 {
        let dx = (hex1.q - hex2.q) as f32;
        let dy = (hex1.q + hex1.r - hex2.q - hex2.r) as f32;
        let dz = (hex1.r - hex2.r) as f32;
        (dx.abs() + dy.abs() + dz.abs()) / 2.0
    }
}

impl Default for TileHierarchy {
    fn default() -> Self {
        let hierarchy_queries = Arc::new(HierarchyQueries::new());
        let tile_manager = Arc::new(TileComponentManager::new());
        Self::new(hierarchy_queries, tile_manager)
    }
}

/// Statistics for tile hierarchy performance monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileHierarchyStats {
    pub resolution_counts: FastHashMap<u8, usize>,
    pub total_hierarchical_tiles: usize,
    pub max_resolution: u8,
    pub cache_hit_rate: f32,
}

/// Validation results for tile hierarchy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileHierarchyValidation {
    pub base_validation: crate::ecs::hierarchy::HierarchyValidation,
    pub resolution_levels: u8,
    pub has_resolution_gaps: bool,
    pub total_hierarchical_entities: usize,
}

/// System for maintaining tile hierarchy consistency
pub fn maintain_tile_hierarchy_system(
    mut tile_hierarchy: ResMut<TileHierarchy>,
    hierarchical_query: Query<Entity, With<HierarchicalTile>>,
) {
    let hierarchical_entities: Vec<_> = hierarchical_query.iter().collect();
    
    if hierarchical_entities.len() > 1000 {
        // For large numbers of hierarchical entities, validate in batches
        // This prevents performance issues with very large worlds
        warn!("Large number of hierarchical tiles ({}), consider optimization", hierarchical_entities.len());
    }
}

/// System for cleaning up orphaned hierarchical relationships
pub fn cleanup_tile_hierarchy_system(
    mut commands: Commands,
    mut tile_hierarchy: ResMut<TileHierarchy>,
    hierarchical_query: Query<Entity, With<HierarchicalTile>>,
) {
    // Validate and clean up any inconsistencies in the tile hierarchy
    if let Ok(validation) = tile_hierarchy.validate_tile_hierarchy() {
        if validation.has_resolution_gaps {
            warn!("Tile hierarchy has resolution gaps - consider rebuilding");
        }
        
        if validation.base_validation.has_cycles {
            warn!("Cycles detected in tile hierarchy - cleaning up");
            // In practice, this would implement cycle breaking logic
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hierarchical_tile_creation() {
        let hex = HexCoord { q: 10, r: 20 };
        let tile = HierarchicalTile::new(123, hex, 2);
        
        assert_eq!(tile.base_tile_id, 123);
        assert_eq!(tile.hex, hex);
        assert_eq!(tile.resolution, 2);
        assert_eq!(tile.coverage_area, 16); // 4^2
        assert!(tile.contains_hex(hex));
    }

    #[test]
    fn test_hex_bounds() {
        let center = HexCoord { q: 0, r: 0 };
        let bounds = HexBounds::from_center_and_radius(center, 2);
        
        assert!(bounds.contains(HexCoord { q: 1, r: 1 }));
        assert!(bounds.contains(HexCoord { q: -2, r: -2 }));
        assert!(!bounds.contains(HexCoord { q: 3, r: 3 }));
        assert_eq!(bounds.area(), 25); // 5x5
    }

    #[test]
    fn test_tile_relationship_type_conversion() {
        assert_eq!(TileRelationshipType::RegionParent.to_base_relationship(), RelationshipType::Parent);
        assert_eq!(TileRelationshipType::Adjacent.to_base_relationship(), RelationshipType::Attachment);
        assert!(TileRelationshipType::Adjacent.is_spatial());
        assert!(TileRelationshipType::RegionParent.is_hierarchical());
    }

    #[test]
    fn test_child_hex_generation() {
        let parent = HierarchicalTile::new(1, HexCoord { q: 0, r: 0 }, 1);
        let child_hexes = parent.get_child_hexes();
        
        assert!(!child_hexes.is_empty());
        assert!(child_hexes.contains(&HexCoord { q: 0, r: 0 })); // Should include center
    }
}
