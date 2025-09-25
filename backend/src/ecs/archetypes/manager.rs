//! Archetype manager that integrates with existing query system
//!
//! Focuses purely on entity storage organization by component signature,
//! designed to work WITH the OptimalSpatialIndex system for efficient queries.

use super::types::{Archetype, ArchetypeId, ComponentSignature, ArchetypeError, ArchetypeResult};
use super::storage::ArchetypeStorage;
use bevy_ecs::prelude::*;
use tracing::{info, debug, warn, error, instrument, Span};
use slotmap::Key;
use std::any::TypeId;
use std::collections::HashSet;
use std::time::Instant;
use crate::ecs::components::*;
use crate::ecs::entities::*;
use crate::core::{
    caching::{
        GameCache, GameCacheBuilder, CacheKey, QueryCacheKey, QueryType, CachePriority, 
        CacheInvalidationEvent, global_cache_events, SubsystemStats
    }, 
    logging::{LoggingSystem, game_logging}
};

/// Trait for extracting component types from Bundle types at runtime
/// 
/// # Safety
/// Implementations must return the exact set of TypeIds that correspond to the
/// components in the Bundle. Incorrect implementations will break archetype tracking.
pub trait BundleComponentExtractor: Bundle {
    /// Extract component TypeIds from this bundle type
    /// 
    /// This must return the same TypeIds as the actual Bundle components.
    /// Used for archetype organization and query optimization.
    fn extract_component_types() -> HashSet<TypeId>;
}

/// Implement component extraction for known bundle types
impl BundleComponentExtractor for UnitBundle {
    fn extract_component_types() -> HashSet<TypeId> {
        // Using std::collections::HashSet as expected by ComponentSignature::new()
        // The ComponentSignature will convert to FastHashSet internally for optimal storage
        let mut types = HashSet::new();
        types.insert(TypeId::of::<Position>());
        types.insert(TypeId::of::<Movement>());
        types.insert(TypeId::of::<Health>());
        types.insert(TypeId::of::<Renderable>());
        types.insert(TypeId::of::<Name>());
        types.insert(TypeId::of::<Owner>());
        types
    }
}

impl BundleComponentExtractor for LivingEntityBundle {
    fn extract_component_types() -> HashSet<TypeId> {
        let mut types = HashSet::new();
        types.insert(TypeId::of::<Health>());
        types.insert(TypeId::of::<Position>());
        types.insert(TypeId::of::<Renderable>());
        types.insert(TypeId::of::<Name>());
        types.insert(TypeId::of::<Owner>());
        types
    }
}

impl BundleComponentExtractor for MovableEntityBundle {
    fn extract_component_types() -> HashSet<TypeId> {
        let mut types = HashSet::new();
        types.insert(TypeId::of::<Position>());
        types.insert(TypeId::of::<Movement>());
        types.insert(TypeId::of::<Renderable>());
        types.insert(TypeId::of::<Name>());
        types.insert(TypeId::of::<Owner>());
        types
    }
}

/// Implement for common tuple bundles used in the game
impl BundleComponentExtractor for (Position, Renderable, Name, Owner) {
    fn extract_component_types() -> HashSet<TypeId> {
        let mut types = HashSet::new();
        types.insert(TypeId::of::<Position>());
        types.insert(TypeId::of::<Renderable>());
        types.insert(TypeId::of::<Name>());
        types.insert(TypeId::of::<Owner>());
        types
    }
}

/// High-level archetype manager that integrates with existing ECS infrastructure
#[derive(Debug)]
pub struct ArchetypeManager {
    /// Core archetype storage
    storage: ArchetypeStorage,
    /// High-performance cache for archetype queries
    cache: GameCache,
    /// World generation for cache invalidation
    world_generation: u32,
}

impl Default for ArchetypeManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ArchetypeManager {
    /// Create new archetype manager
    pub fn new() -> Self {
        // Configure cache for archetype queries
        let cache = GameCacheBuilder::new()
            .max_memory_mb(64) // 64MB for archetype data
            .default_ttl(std::time::Duration::from_secs(120)) // 2 minute TTL
            .turn_based_invalidation(true)
            .build();
            
        Self {
            storage: ArchetypeStorage::new(),
            cache,
            world_generation: 1,
        }
    }
    
    /// Register entity with its component signature
    /// This organizes entities by their component layout for optimal storage
    #[instrument(name = "archetype_register", skip(self), fields(entity = ?entity))]
    pub fn register_entity<T: BundleComponentExtractor>(&self, entity: Entity) -> ArchetypeId {
        let register_start = Instant::now();
        let correlation_id = LoggingSystem::generate_correlation_id();
        
        let signature = self.create_signature_for_bundle::<T>();
        let archetype_id = self.storage.get_or_create_archetype(signature.clone());
        
        // Add entity to archetype (ignoring errors for now - entity might already be there)
        let add_result = self.storage.add_entity_to_archetype(entity, archetype_id);
        let register_duration = register_start.elapsed().as_secs_f64() * 1000.0;
        
        debug!(
            target: "game::archetypes",
            correlation_id = correlation_id,
            entity = ?entity,
            archetype_id = ?archetype_id,
            component_count = signature.components().len(),
            register_duration_ms = register_duration,
            success = add_result.is_ok(),
            "Entity registered to archetype"
        );
        
        // Convert archetype_id to u64 for logging - use the key's underlying data
        let archetype_id_u64 = u64::from(archetype_id.data().as_ffi());
        game_logging::log_archetype_operation(archetype_id_u64, "entity_registered", 1);
        game_logging::log_performance_event("archetype_register", register_duration, 1);
        
        archetype_id
    }
    
    /// Move entity to new archetype when components change
    pub fn update_entity_archetype<T: BundleComponentExtractor>(&self, entity: Entity) -> ArchetypeResult<()> {
        let new_signature = self.create_signature_for_bundle::<T>();
        let new_archetype_id = self.storage.get_or_create_archetype(new_signature);
        
        // Move entity (handles removal from old archetype)
        self.storage.move_entity(entity, new_archetype_id)?;
        
        Ok(())
    }
    
    /// Remove entity from archetype tracking
    pub fn unregister_entity(&self, entity: Entity) -> ArchetypeResult<()> {
        self.storage.remove_entity_from_archetype(entity)?;
        Ok(())
    }
    
    /// Get entities in same archetype (same component layout)
    pub fn get_entities_with_same_layout(&self, entity: Entity) -> Vec<Entity> {
        if let Some(archetype_id) = self.storage.get_entity_archetype(entity) {
            if let Some(archetype) = self.storage.get(archetype_id) {
                return archetype.entities.clone();
            }
        }
        Vec::new()
    }
    
    /// Get all entities in archetype by ID
    pub fn get_archetype_entities(&self, archetype_id: ArchetypeId) -> Vec<Entity> {
        if let Some(archetype) = self.storage.get(archetype_id) {
            archetype.entities.clone()
        } else {
            Vec::new()
        }
    }
    
    /// Find archetypes that contain all required components
    /// This enables the spatial system to focus on specific archetypes
    pub fn find_archetypes_with_components(&self, component_types: &HashSet<TypeId>) -> Vec<ArchetypeId> {
        let all_ids = self.storage.all_archetype_ids();
        let mut matching = Vec::new();
        
        for archetype_id in all_ids {
            if let Some(archetype) = self.storage.get(archetype_id) {
                if archetype.contains_all(component_types) {
                    matching.push(archetype_id);
                }
            }
        }
        
        matching
    }

    /// Get entities with specific components using QueryCacheKey system
    #[instrument(name = "archetype_query_cached", skip(self))]
    pub async fn get_entities_with_components_cached(&self, component_types: &[TypeId]) -> Vec<Entity> {
        use crate::core::caching::{QueryResult, QueryType};
        use crate::core::hashing::HashStrategies;

        // Create cache key
        let cache_key = QueryCacheKey {
            component_signature: HashStrategies::hash_type_signature(component_types),
            filter_hash: None,
            player_id: None,
            world_generation: self.world_generation,
            query_type: QueryType::ArchetypeQuery,
        };

        // Try cache first
        if let Ok(Some(QueryResult::Entities(entities))) = self.cache.get(&CacheKey::Query(cache_key.clone())).await {
            return entities;
        }

        // Cache miss - compute result
        let component_set: HashSet<TypeId> = component_types.iter().copied().collect();
        let matching_archetypes = self.find_archetypes_with_components(&component_set);
        
        let mut entities = Vec::new();
        for archetype_id in matching_archetypes {
            entities.extend(self.get_archetype_entities(archetype_id));
        }

        // Cache the result
        let result = QueryResult::Entities(entities.clone());
        let _ = self.cache.set(
            CacheKey::Query(cache_key), 
            result, 
            CachePriority::High
        ).await;

        entities
    }

    /// Get entities for player with specific components using QueryCacheKey system
    #[instrument(name = "archetype_player_query_cached", skip(self))]
    pub async fn get_player_entities_cached(&self, player_id: u32, component_types: &[TypeId]) -> Vec<Entity> {
        use crate::core::caching::{QueryResult, QueryType};
        use crate::core::hashing::HashStrategies;

        // Create cache key with player filter
        let cache_key = QueryCacheKey {
            component_signature: HashStrategies::hash_type_signature(component_types),
            filter_hash: None,
            player_id: Some(player_id),
            world_generation: self.world_generation,
            query_type: QueryType::PlayerOwnedQuery,
        };

        // Try cache first
        if let Ok(Some(QueryResult::Entities(entities))) = self.cache.get(&CacheKey::Query(cache_key.clone())).await {
            return entities;
        }

        // Cache miss - compute result
        let component_set: HashSet<TypeId> = component_types.iter().copied().collect();
        let matching_archetypes = self.find_archetypes_with_components(&component_set);
        
        let mut entities = Vec::new();
        for archetype_id in matching_archetypes {
            // Filter by player ownership - would need access to world for full implementation
            // For now, return all entities from matching archetypes
            entities.extend(self.get_archetype_entities(archetype_id));
        }

        // Cache the result
        let result = QueryResult::Entities(entities.clone());
        let _ = self.cache.set(
            CacheKey::Query(cache_key), 
            result, 
            CachePriority::High
        ).await;

        entities
    }

    /// Invalidate cache when world generation changes
    pub async fn advance_world_generation(&mut self) {
        self.world_generation += 1;
        self.cache.clear().await;
    }

    /// Invalidate cache for specific archetype changes
    pub async fn invalidate_archetype_cache(&self, archetype_id: ArchetypeId) {
        // For a more sophisticated implementation, we'd track which cache entries
        // depend on specific archetypes and invalidate only those
        let invalidation_event = CacheInvalidationEvent::Manual(
            Box::new(move |key| {
                matches!(key, CacheKey::Query(query_key) if query_key.query_type == QueryType::ArchetypeQuery)
            })
        );
        self.cache.handle_invalidation(&invalidation_event).await;
    }

    /// Report cache metrics to the global metrics system
    pub async fn report_metrics(&self) {
        let cache_stats = self.cache.stats().await;
        let archetype_stats = self.stats();
        
        let subsystem_stats = SubsystemStats {
            hits: cache_stats.total_hits,
            misses: cache_stats.total_misses,
            entries: archetype_stats.total_archetypes,
            memory_usage_bytes: cache_stats.memory_usage_bytes,
            avg_access_time_micros: cache_stats.avg_access_time_micros,
            last_updated: std::time::Instant::now(),
        };

        global_cache_events().register_subsystem_metrics("archetypes", subsystem_stats).await;
    }
    
    /// Find archetypes that contain any of the specified components
    pub fn find_archetypes_with_any_components(&self, component_types: &HashSet<TypeId>) -> Vec<ArchetypeId> {
        let all_ids = self.storage.all_archetype_ids();
        let mut matching = Vec::new();
        
        for archetype_id in all_ids {
            if let Some(archetype) = self.storage.get(archetype_id) {
                if archetype.contains_any(component_types) {
                    matching.push(archetype_id);
                }
            }
        }
        
        matching
    }
    
    /// Get archetype statistics for monitoring
    pub fn stats(&self) -> super::types::ArchetypeStats {
        self.storage.stats()
    }
    
    /// Cleanup empty archetypes
    pub fn cleanup(&self) -> usize {
        self.storage.cleanup_empty_archetypes()
    }
    
    /// Validate internal consistency
    pub fn validate(&self) -> Result<(), String> {
        self.storage.validate()
    }
    
    /// Get reference to underlying storage (for integration)
    pub fn storage(&self) -> &ArchetypeStorage {
        &self.storage
    }
    
    // Private helper to create component signature from bundle type
    fn create_signature_for_bundle<T: BundleComponentExtractor>(&self) -> ComponentSignature {
        // Extract actual component types from the bundle
        let component_types = T::extract_component_types();
        ComponentSignature::new(component_types)
    }
}

/// Integration traits for working with existing query system

/// Trait for archetype-aware querying (integrates with OptimalSpatialIndex)
pub trait ArchetypeAware {
    /// Get entities from specific archetypes only
    fn from_archetypes(&self, archetype_ids: Vec<ArchetypeId>) -> Vec<Entity>;
    
    /// Get archetype distribution of query results
    fn archetype_distribution(&self) -> std::collections::HashMap<ArchetypeId, usize>;
}

// Example integration would be with OptimalSpatialIndex:
// impl ArchetypeAware for crate::ecs::spatial::OptimalSpatialIndex {
//     fn from_archetypes(&self, archetype_ids: Vec<ArchetypeId>) -> Vec<Entity> {
//         // Use R-tree spatial index but limit to entities from specific archetypes
//         todo!()
//     }
//     
//     fn archetype_distribution(&self) -> std::collections::HashMap<ArchetypeId, usize> {
//         todo!()
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::entities::UnitBundle;

    #[test]
    fn test_manager_creation() {
        let manager = ArchetypeManager::new();
        let stats = manager.stats();
        assert_eq!(stats.total_archetypes, 0);
        assert_eq!(stats.total_entities, 0);
    }
    
    #[test]
    fn test_entity_registration() {
        let manager = ArchetypeManager::new();
        let entity = Entity::from_raw(42);
        
        let archetype_id = manager.register_entity::<UnitBundle>(entity);
        let entities = manager.get_archetype_entities(archetype_id);
        assert_eq!(entities.len(), 1);
        assert!(entities.contains(&entity));
    }
    
    #[test]
    fn test_entity_unregistration() {
        let manager = ArchetypeManager::new();
        let entity = Entity::from_raw(123);
        
        manager.register_entity::<UnitBundle>(entity);
        assert!(manager.unregister_entity(entity).is_ok());
        
        let stats = manager.stats();
        assert_eq!(stats.total_entities, 0);
    }
    
    #[test]
    fn test_cleanup() {
        let manager = ArchetypeManager::new();
        let entity = Entity::from_raw(456);
        
        manager.register_entity::<UnitBundle>(entity);
        manager.unregister_entity(entity).unwrap();
        
        let removed = manager.cleanup();
        assert_eq!(removed, 1); // Should remove empty archetype
    }
    
    #[test]
    fn test_validation() {
        let manager = ArchetypeManager::new();
        assert!(manager.validate().is_ok());
        
        let entity = Entity::from_raw(789);
        manager.register_entity::<UnitBundle>(entity);
        assert!(manager.validate().is_ok());
    }
}
