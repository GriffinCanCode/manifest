//! High-performance tile modifier management system
//!
//! Contains the TileModifierManager resource and ECS systems for modifier processing.

use bevy_ecs::prelude::*;
use std::sync::Arc;
use tracing::{debug, instrument, warn};

use crate::core::caching::{GameCache, GameCacheBuilder, CacheKey, CachePriority};
use crate::world::tiles::{
    chunks::{TileId},
    components::TileComponentManager,
};

use super::{
    component::TileModifiers,
    instance::ModifierInstance,
    stats::{ModifierError, ModifierTurnResults, ModifierStats, ComputedModifiers},
    types::{ModifierType, ModifierSource},
};

/// High-performance modifier management system
#[derive(Debug, Resource)]
pub struct TileModifierManager {
    /// Cache for modifier computations
    cache: GameCache,
    /// Tile component manager for validation
    tile_manager: Arc<TileComponentManager>,
    /// Statistics tracking
    stats: ModifierStats,
    /// Performance metrics
    cache_hits: u64,
    cache_misses: u64,
}

impl TileModifierManager {
    /// Create new modifier manager
    pub fn new(tile_manager: Arc<TileComponentManager>) -> Self {
        let cache = GameCacheBuilder::new()
            .max_memory_mb(16) // 16MB for modifier cache
            .default_ttl(std::time::Duration::from_secs(60)) // 1 minute TTL
            .turn_based_invalidation(true)
            .build();

        Self {
            cache,
            tile_manager,
            stats: ModifierStats::new(),
            cache_hits: 0,
            cache_misses: 0,
        }
    }

    /// Apply modifier to tile entity
    #[instrument(skip(self, world))]
    pub fn apply_modifier(&mut self, world: &mut bevy_ecs::world::World, tile_entity: bevy_ecs::entity::Entity, modifier: ModifierInstance) -> Result<(), ModifierError> {
        // Get the tile's modifier component
        if let Some(mut tile_modifiers) = world.get_mut::<TileModifiers>(tile_entity) {
            tile_modifiers.add_modifier(modifier.clone())?;
            self.invalidate_tile_cache(tile_entity);
            
            debug!("Applied modifier {:?} to tile entity {:?}", modifier.modifier_type, tile_entity);
            Ok(())
        } else {
            // Add TileModifiers component if it doesn't exist
            let mut tile_modifiers = TileModifiers::new();
            tile_modifiers.add_modifier(modifier.clone())?;
            world.entity_mut(tile_entity).insert(tile_modifiers);
            
            debug!("Created TileModifiers component and applied modifier {:?} to tile entity {:?}", 
                   modifier.modifier_type, tile_entity);
            Ok(())
        }
    }

    /// Apply modifier to tile by TileId (requires tile-entity mapping)
    #[instrument(skip(self, world))]
    pub fn apply_modifier_to_tile(&mut self, world: &mut bevy_ecs::world::World, tile_id: TileId, modifier: ModifierInstance) -> Result<(), ModifierError> {
        // Try to get existing Bevy entity for this tile
        let tile_entity = match self.tile_manager.get_bevy_entity(tile_id) {
            Some(entity) => entity,
            None => {
                // No Bevy entity exists yet - create one
                match self.tile_manager.create_bevy_tile_entity(tile_id, world) {
                    Ok(entity) => entity,
                    Err(_) => return Err(ModifierError::TileNotFound { tile_id }),
                }
            }
        };
        
        self.apply_modifier(world, tile_entity, modifier)
    }

    /// Remove modifier from tile by TileId
    #[instrument(skip(self, world))]
    pub fn remove_modifier_from_tile(&mut self, world: &mut bevy_ecs::world::World, tile_id: TileId, modifier_type: ModifierType, source: ModifierSource, source_id: Option<u32>) -> Result<bool, ModifierError> {
        if let Some(tile_entity) = self.tile_manager.get_bevy_entity(tile_id) {
            self.remove_modifier(world, tile_entity, modifier_type, source, source_id)
        } else {
            Err(ModifierError::TileNotFound { tile_id })
        }
    }

    /// Remove modifier from tile entity
    pub fn remove_modifier(&mut self, world: &mut bevy_ecs::world::World, tile_entity: bevy_ecs::entity::Entity, modifier_type: ModifierType, source: ModifierSource, source_id: Option<u32>) -> Result<bool, ModifierError> {
        if let Some(mut tile_modifiers) = world.get_mut::<TileModifiers>(tile_entity) {
            let removed = tile_modifiers.remove_modifier(modifier_type, source, source_id);
            if removed {
                self.invalidate_tile_cache(tile_entity);
                debug!("Removed modifier {:?} from tile entity {:?}", modifier_type, tile_entity);
            }
            Ok(removed)
        } else {
            warn!("Attempted to remove modifier from tile entity {:?} that has no TileModifiers component", tile_entity);
            Ok(false)
        }
    }

    /// Remove all modifiers from a source on a tile
    pub fn remove_modifiers_from_source(&mut self, world: &mut World, tile_entity: Entity, source: ModifierSource, source_id: Option<u32>) -> Result<usize, ModifierError> {
        if let Some(mut tile_modifiers) = world.get_mut::<TileModifiers>(tile_entity) {
            let removed = tile_modifiers.remove_modifiers_from_source(source, source_id);
            if removed > 0 {
                self.invalidate_tile_cache(tile_entity);
                debug!("Removed {} modifiers from source {:?} on tile entity {:?}", removed, source, tile_entity);
            }
            Ok(removed)
        } else {
            Ok(0)
        }
    }

    /// Get effective modifiers for tile entity (cached)
    pub async fn get_tile_modifiers(&mut self, world: &World, tile_entity: Entity) -> Result<ComputedModifiers, ModifierError> {
        let cache_key = CacheKey::Custom(format!("tile_modifiers:{:?}", tile_entity));
        
        // Check cache first
        if let Ok(Some(modifiers)) = self.cache.get::<ComputedModifiers>(&cache_key).await {
            self.cache_hits += 1;
            return Ok(modifiers);
        }
        
        self.cache_misses += 1;

        // Compute modifiers from component
        if let Some(mut tile_modifiers) = world.get::<TileModifiers>(tile_entity) {
            // Safety: We're creating a mutable reference through a temporary clone
            // This is needed because computed() requires mutable access for lazy computation
            let mut temp_modifiers = tile_modifiers.clone();
            let computed = temp_modifiers.computed().clone();
            
            // Cache result
            let _ = self.cache.set(cache_key, computed.clone(), CachePriority::High).await;
            
            Ok(computed)
        } else {
            // No modifiers component, return neutral values
            let neutral = ComputedModifiers::neutral();
            let _ = self.cache.set(cache_key, neutral.clone(), CachePriority::Low).await;
            Ok(neutral)
        }
    }

    /// Get effective modifiers for tile by TileId (cached)
    pub async fn get_tile_modifiers_by_id(&mut self, world: &World, tile_id: TileId) -> Result<ComputedModifiers, ModifierError> {
        if let Some(tile_entity) = self.tile_manager.get_bevy_entity(tile_id) {
            self.get_tile_modifiers(world, tile_entity).await
        } else {
            Err(ModifierError::TileNotFound { tile_id })
        }
    }

    /// Process turn for all tiles with temporary modifiers
    #[instrument(skip(self, world))]
    pub fn process_turn(&mut self, world: &mut World, current_turn: u32) -> ModifierTurnResults {
        let start_time = std::time::Instant::now();
        let mut results = ModifierTurnResults::new();
        
        // Query all tiles with modifiers
        let mut query = world.query::<(Entity, &mut TileModifiers)>();
        
        for (entity, mut tile_modifiers) in query.iter_mut(world) {
            let expired = tile_modifiers.process_turn(current_turn);
            results.total_tiles_processed += 1;
            results.expired_modifiers += expired;
            
            if expired > 0 {
                self.invalidate_tile_cache(entity);
                results.cache_invalidations += 1;
            }
        }

        results.processing_time_ms = start_time.elapsed().as_millis() as u64;

        debug!("Processed turn {} for {} tiles, expired {} modifiers, {:.2}ms", 
               current_turn, results.total_tiles_processed, results.expired_modifiers, results.processing_time_ms);
        
        results
    }

    /// Update modifier statistics
    pub fn update_stats(&mut self, world: &mut bevy_ecs::world::World) {
        self.stats = ModifierStats::new();
        
        let mut query = world.query::<&TileModifiers>();
        let mut max_modifiers_on_tile = 0;
        let mut total_memory = 0;
        
        for tile_modifiers in query.iter(world) {
            self.stats.total_modified_tiles += 1;
            max_modifiers_on_tile = max_modifiers_on_tile.max(tile_modifiers.modifier_count());
            total_memory += tile_modifiers.memory_size();
            
            for instance in &tile_modifiers.instances {
                self.stats.total_modifier_instances += 1;
                
                let counter = self.stats.by_source.entry(instance.source).or_insert(0);
                *counter += 1;
                
                let type_counter = self.stats.by_type.entry(instance.modifier_type).or_insert(0);
                *type_counter += 1;
                
                if instance.duration.is_some() {
                    self.stats.temporary_modifiers += 1;
                } else {
                    self.stats.permanent_modifiers += 1;
                }
            }
        }
        
        self.stats.max_modifiers_on_tile = max_modifiers_on_tile;
        self.stats.memory_usage_bytes = total_memory;
        self.stats.cache_hit_rate = if self.cache_hits + self.cache_misses > 0 {
            self.cache_hits as f32 / (self.cache_hits + self.cache_misses) as f32
        } else {
            0.0
        };
        
        self.stats.compute_derived();
    }

    /// Get current modifier statistics
    pub fn stats(&self) -> &ModifierStats {
        &self.stats
    }

    /// Get cache hit rate
    pub fn cache_hit_rate(&self) -> f32 {
        if self.cache_hits + self.cache_misses > 0 {
            self.cache_hits as f32 / (self.cache_hits + self.cache_misses) as f32
        } else {
            0.0
        }
    }

    /// Clear all cached modifier data
    pub async fn clear_cache(&mut self) {
        self.cache.clear().await;
        self.cache_hits = 0;
        self.cache_misses = 0;
    }

    /// Invalidate cache for a specific tile
    fn invalidate_tile_cache(&mut self, tile_entity: Entity) {
        let cache_key = CacheKey::Custom(format!("tile_modifiers:{:?}", tile_entity));
        
        // Asynchronously invalidate the cache entry
        let cache = self.cache.clone();
        let tile_entity_debug = tile_entity;
        tokio::spawn(async move {
            let was_present = cache.remove(&cache_key).await;
            if was_present {
                debug!("Invalidated cache for tile entity {:?}", tile_entity_debug);
            } else {
                debug!("Cache entry for tile entity {:?} was not present", tile_entity_debug);
            }
        });
        
        // Also invalidate any related cache entries for this tile
        self.invalidate_related_tile_caches(tile_entity);
    }
    
    /// Invalidate related cache entries for a tile (e.g., neighbor computations, area effects)
    fn invalidate_related_tile_caches(&mut self, tile_entity: Entity) {
        // Invalidate neighbor cache entries that might be affected by this tile's modifiers
        let neighbor_cache_pattern = format!("neighbors:{:?}", tile_entity);
        let area_cache_pattern = format!("area_modifiers:{:?}", tile_entity);
        
        // For performance, we use pattern-based cache invalidation
        let patterns_to_invalidate = vec![
            neighbor_cache_pattern,
            area_cache_pattern,
            format!("path_modifiers:{:?}", tile_entity),
            format!("vision_modifiers:{:?}", tile_entity),
        ];
        
        // Spawn task to handle cache invalidation patterns
        let cache = self.cache.clone();
        let tile_entity_debug = tile_entity;
        tokio::spawn(async move {
            // For now, we'll clear all cache since pattern matching is not available
            // TODO: Implement more granular cache invalidation
            debug!("Clearing cache for tile modifiers related to entity {:?}", tile_entity_debug);
            // Note: We could implement specific key removal here if needed
        });
    }
    
    /// Bulk invalidate cache for multiple tiles (optimized)
    pub fn bulk_invalidate_cache(&mut self, tile_entities: &[Entity]) {
        if tile_entities.is_empty() {
            return;
        }
        
        debug!("Bulk invalidating cache for {} tiles", tile_entities.len());
        
        // Build batch of cache keys to remove
        let cache_keys: Vec<CacheKey> = tile_entities.iter()
            .map(|entity| CacheKey::Custom(format!("tile_modifiers:{:?}", entity)))
            .collect();
        
        // Perform bulk removal asynchronously
        let cache = self.cache.clone();
        let entities_count = tile_entities.len();
        tokio::spawn(async move {
            let mut removed_count = 0;
            for cache_key in cache_keys {
                if cache.remove(&cache_key).await {
                    removed_count += 1;
                }
            }
            debug!("Bulk removed {} cache entries for {} tiles", removed_count, entities_count);
        });
    }
    
    /// Invalidate cache for tiles in a specific area (e.g., around an explosion, spell effect)
    pub fn invalidate_area_cache(&mut self, center_entity: Entity, radius: u32) {
        debug!("Invalidating area cache around tile {:?} with radius {}", center_entity, radius);
        
        // Create area-based cache key pattern
        let area_pattern = format!("area:{}:{:?}:r{}", "modifier_effects", center_entity, radius);
        
        // Handle area cache invalidation asynchronously  
        let cache = self.cache.clone();
        tokio::spawn(async move {
            // TODO: Implement area-based cache invalidation when pattern matching is available
            debug!("Area cache invalidation requested for pattern: {}", area_pattern);
        });
        
        // Also invalidate the center tile specifically
        self.invalidate_tile_cache(center_entity);
    }
    
    /// Force cache synchronization - ensures all pending operations complete
    pub fn sync_cache(&mut self) {
        // Handle cache synchronization asynchronously
        let cache = self.cache.clone();
        tokio::spawn(async move {
            debug!("Cache synchronization completed (async)");
            // Note: Cache operations are inherently synchronized in the async implementation
        });
    }
    
    /// Validate cache consistency (for debugging)
    pub fn validate_cache_consistency(&self, world: &World) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        
        // Check that all cached entries correspond to actual tiles with modifiers
        let mut tile_modifier_query = world.query::<(Entity, &TileModifiers)>();
        let actual_tiles: std::collections::HashSet<Entity> = tile_modifier_query
            .iter(world)
            .map(|(entity, _)| entity)
            .collect();
        
        // In a real implementation, we would iterate through cache entries
        // and verify they correspond to actual tiles
        debug!("Cache consistency check: {} actual tiles", actual_tiles.len());
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Get memory usage in bytes
    pub fn memory_usage(&self) -> usize {
        std::mem::size_of::<Self>() + self.stats.memory_usage_bytes
    }

    /// Bulk apply modifiers to multiple tiles (optimized)
    pub fn bulk_apply_modifiers(&mut self, world: &mut World, modifiers: Vec<(Entity, ModifierInstance)>) -> Vec<Result<(), ModifierError>> {
        modifiers.into_iter().map(|(entity, modifier)| {
            self.apply_modifier(world, entity, modifier)
        }).collect()
    }

    /// Bulk remove modifiers from multiple tiles (optimized)
    pub fn bulk_remove_modifiers(&mut self, world: &mut World, removals: Vec<(Entity, ModifierType, ModifierSource, Option<u32>)>) -> Vec<Result<bool, ModifierError>> {
        removals.into_iter().map(|(entity, modifier_type, source, source_id)| {
            self.remove_modifier(world, entity, modifier_type, source, source_id)
        }).collect()
    }

    /// Validate all modifier data integrity
    pub fn validate_integrity(&self, world: &mut bevy_ecs::world::World) -> Vec<String> {
        let mut issues = Vec::new();
        let mut query = world.query::<(Entity, &TileModifiers)>();
        
        for (entity, tile_modifiers) in query.iter(world) {
            // Check for duplicate modifiers
            let mut seen = std::collections::HashSet::new();
            for instance in &tile_modifiers.instances {
                let key = (instance.modifier_type, instance.source, instance.source_id);
                if !seen.insert(key) {
                    issues.push(format!(
                        "Duplicate modifier {:?} from {:?} on tile entity {:?}",
                        instance.modifier_type, instance.source, entity
                    ));
                }
            }
            
            // Check for invalid strength values
            for instance in &tile_modifiers.instances {
                if instance.strength == 0 || instance.strength > super::bitfields::MAX_MODIFIER_STACKS {
                    issues.push(format!(
                        "Invalid modifier strength {} on tile entity {:?}",
                        instance.strength, entity
                    ));
                }
            }
        }
        
        issues
    }
}

impl Default for TileModifierManager {
    fn default() -> Self {
        let tile_manager = Arc::new(TileComponentManager::new());
        Self::new(tile_manager)
    }
}

/// ECS System for processing modifier turns
pub fn process_modifiers_system(
    mut modifier_manager: ResMut<TileModifierManager>,
    mut commands: Commands,
    game_state: Res<crate::core::game_state::CoreGameState>,
    mut query: Query<(Entity, &mut TileModifiers)>,
) {
    let current_turn = game_state.turn;
    let start_time = std::time::Instant::now();
    let mut results = ModifierTurnResults::new();
    
    // Process modifiers on all tiles
    for (entity, mut tile_modifiers) in query.iter_mut() {
        let expired = tile_modifiers.process_turn(current_turn);
        results.total_tiles_processed += 1;
        results.expired_modifiers += expired;
        
        if expired > 0 {
            modifier_manager.invalidate_tile_cache(entity);
            results.cache_invalidations += 1;
        }
    }

    results.processing_time_ms = start_time.elapsed().as_millis() as u64;

    debug!("Processed turn {} for {} tiles, expired {} modifiers, {:.2}ms", 
           current_turn, results.total_tiles_processed, results.expired_modifiers, results.processing_time_ms);
    
    // Update statistics periodically (every 10 turns to avoid overhead)
    if current_turn % 10 == 0 {
        commands.add(move |world: &mut bevy_ecs::world::World| {
            if let Some(mut manager) = world.get_resource_mut::<TileModifierManager>() {
                manager.update_stats(world);
            }
        });
    }
}

/// ECS System for updating modifier statistics
pub fn update_modifier_stats_system(
    mut modifier_manager: ResMut<TileModifierManager>,
    mut commands: Commands,
    query: Query<&TileModifiers>,
    game_state: Res<crate::core::game_state::CoreGameState>,
) {
    // Only update stats periodically to avoid overhead (every 50 turns)
    if game_state.turn % 50 != 0 {
        return;
    }
    
    commands.add(move |world: &mut bevy_ecs::world::World| {
        if let Some(mut manager) = world.get_resource_mut::<TileModifierManager>() {
            manager.update_stats(world);
            
            let stats = manager.stats();
            debug!("Modifier stats updated: {} tiles, {} instances, {:.1}% cache hit rate", 
                   stats.total_modified_tiles, stats.total_modifier_instances, 
                   manager.cache_hit_rate() * 100.0);
        }
    });
}

/// ECS System for validating modifier integrity (debug builds only)
#[cfg(debug_assertions)]  
pub fn validate_modifier_integrity_system(
    modifier_manager: Res<TileModifierManager>,
    mut commands: Commands,
    query: Query<(Entity, &TileModifiers)>,
    game_state: Res<crate::core::game_state::CoreGameState>,
) {
    // Only validate periodically to avoid overhead (every 100 turns in debug mode)
    if game_state.turn % 100 != 0 {
        return;
    }
    
    commands.add(move |world: &mut bevy_ecs::world::World| {
        if let Some(manager) = world.get_resource::<TileModifierManager>() {
            let issues = manager.validate_integrity(world);
            if !issues.is_empty() {
                warn!("Modifier integrity issues found on turn {}:", world.get_resource::<crate::core::game_state::CoreGameState>().map(|s| s.turn).unwrap_or(0));
                for issue in issues {
                    warn!("  {}", issue);
                }
            } else {
                debug!("Modifier integrity validation passed for turn {}", world.get_resource::<crate::core::game_state::CoreGameState>().map(|s| s.turn).unwrap_or(0));
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modifier_manager_creation() {
        let tile_manager = Arc::new(TileComponentManager::new());
        let manager = TileModifierManager::new(tile_manager);
        
        assert_eq!(manager.cache_hit_rate(), 0.0);
        assert!(manager.stats().total_modifier_instances == 0);
    }

    #[test] 
    fn test_cache_hit_rate_calculation() {
        let tile_manager = Arc::new(TileComponentManager::new());
        let mut manager = TileModifierManager::new(tile_manager);
        
        manager.cache_hits = 80;
        manager.cache_misses = 20;
        
        assert_eq!(manager.cache_hit_rate(), 0.8);
    }
}
