//! Core tile hierarchy manager
//!
//! Contains the main TileHierarchy struct and its management logic
//! for creating, initializing, and maintaining hierarchical tile structures.

use std::sync::Arc;
use parking_lot::RwLock;
use bevy_ecs::prelude::*;
use tracing::{debug, instrument, warn};

use crate::core::{
    zig_ffi::HexCoord,
    hashing::{FastHashMap, FastHashSet},
    caching::{GameCache, GameCacheBuilder}
};
use crate::ecs::hierarchy::{
    HierarchyQueries, HierarchyResult, HierarchyError,
    Relationship, Relationships, Hierarchical
};
use crate::world::tiles::{
    chunks::TileId,
    components::{Tile, TileComponentManager}
};

use super::types::{
    TileRelationshipType, HierarchicalTile, TileHierarchyStats,
    TileHierarchyValidation
};

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

    /// Get hierarchy queries reference
    pub fn hierarchy_queries(&self) -> &Arc<HierarchyQueries> {
        &self.hierarchy_queries
    }

    /// Get cache reference
    pub fn cache(&self) -> &GameCache {
        &self.cache
    }

    /// Get maximum resolution level
    pub fn max_resolution(&self) -> u8 {
        self.max_resolution
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
        let mut base_tile_id = TileId(0);

        for &child_entity in children {
            if let Some(hierarchical_tile) = world.get::<HierarchicalTile>(child_entity) {
                center_q += hierarchical_tile.hex.q;
                center_r += hierarchical_tile.hex.r;
                total_coverage += hierarchical_tile.coverage_area;
                if base_tile_id.0 == 0 {
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
            if let Some(child_tile) = world.get::<HierarchicalTile>(child_entity) {
                let mut nearest_parent = None;
                let mut min_distance = f32::MAX;

                for &parent_entity in parents {
                    if let Some(parent_tile) = world.get::<HierarchicalTile>(parent_entity) {
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
        if let Some(mut relationships) = world.get_mut::<Relationships>(from) {
            relationships.add(relationship).map_err(|e| HierarchyError::InvalidRelationship(e.to_string()))?;
        }

        // Update hierarchy graph
        let updates = vec![(from, world.get::<Relationships>(from).unwrap().clone())];
        self.hierarchy_queries.update_relationships_sync(updates)?;

        debug!("Added tile relationship {:?} from {:?} to {:?}", rel_type, from, to);
        Ok(())
    }

    /// Batch update tile relationships for multiple tiles (parallelized)
    pub fn batch_update_tile_relationships<I>(&self, world: &mut World, updates: I) -> HierarchyResult<()>
    where
        I: IntoIterator<Item = (Entity, Vec<(Entity, TileRelationshipType)>)> + Send,
        I::IntoIter: Send,
    {
        let updates: Vec<_> = updates.into_iter().collect();
        
        // Process sequentially to avoid borrow checker issues with mutable world access
        for (from_entity, relationships) in updates {
            for &(to_entity, rel_type) in &relationships {
                self.add_tile_relationship(world, from_entity, to_entity, rel_type)?;
            }
        }

        Ok(())
    }

    /// Get hierarchical tiles at specific resolution level
    pub fn get_tiles_at_resolution(&self, resolution: u8) -> Vec<Entity> {
        self.hierarchical_tiles.read()
            .get(&resolution)
            .cloned()
            .unwrap_or_default()
    }

    /// Get reference to hierarchical tiles (for external access)
    pub fn hierarchical_tiles(&self) -> &Arc<RwLock<FastHashMap<u8, Vec<Entity>>>> {
        &self.hierarchical_tiles
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
        let total_requests = cache_stats.total_hits + cache_stats.total_misses;
        stats.cache_hit_rate = if total_requests > 0 {
            cache_stats.total_hits as f32 / total_requests as f32
        } else {
            0.0
        };

        stats
    }

    /// Validate hierarchy integrity
    pub fn validate_tile_hierarchy(&self, world: &mut World) -> HierarchyResult<TileHierarchyValidation> {
        let base_validation = self.hierarchy_queries.validate_hierarchy(world)?;
        
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
    pub fn hex_distance(&self, hex1: HexCoord, hex2: HexCoord) -> u32 {
        let dx = (hex1.q - hex2.q).abs();
        let dy = (hex1.q + hex1.r - hex2.q - hex2.r).abs();
        let dz = (hex1.r - hex2.r).abs();
        ((dx + dy + dz) / 2) as u32
    }

    /// Helper: Calculate hex distance as float for precise calculations
    #[inline]
    pub fn hex_distance_f32(&self, hex1: HexCoord, hex2: HexCoord) -> f32 {
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
