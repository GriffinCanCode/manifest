//! ECS World management and initialization
//!
//! This module provides the main interface for managing the game world,
//! including initialization, updates, and serialization support.

use bevy_ecs::prelude::*;
use glam::IVec2;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use tracing::{info, warn, error, debug};
use slotmap::Key;

use crate::core::{Stage, SchedulerMetrics, reloader::*, logging::{LoggingSystem, LoggingConfig, game_logging}, caching::{GameCache, GameCacheBuilder, CacheKey, QueryCacheKey, QueryResult, CachePriority, broadcast_cache_invalidation, CacheInvalidationEvent}};
use crate::ecs::{resources::*, systems::*, changes::*, spatial::*, components::{Name, Position}, EcsScheduler, archetypes::{ArchetypeManager, BundleComponentExtractor}, hierarchy::{HierarchyQueries, Relationships, Hierarchical, StableEntityId}};

/// Main game world wrapper that manages the ECS world and systems
#[derive(Debug)]
pub struct GameWorld {
    /// The ECS world containing all entities, components, and resources
    pub world: World,
    /// Parallel scheduler for running systems efficiently
    scheduler: EcsScheduler,
    /// High-performance spatial indexing using R-tree
    spatial_index: OptimalSpatialIndex,
    /// Component signature organization for efficient ECS operations
    archetype_manager: ArchetypeManager,
    /// High-performance query result cache
    query_cache: GameCache,
    /// World generation for cache invalidation
    world_generation: u32,
    /// Hot reload manager for live development
    #[cfg(debug_assertions)]
    reload_manager: Option<ReloadManager>,
    /// Last update time for delta time calculation
    last_update: Instant,
}

impl GameWorld {
    /// Create a new game world with default resources and systems
    pub fn new() -> Self {
        let mut world = World::new();
        
        // Initialize default resources
        world.insert_resource(GameTime::default());
        world.insert_resource(Players::default());
        world.insert_resource(Camera::default());
        world.insert_resource(Selection::default());
        world.insert_resource(HierarchyQueries::new());

        // Create spatial index and insert as resource
        let spatial_index = OptimalSpatialIndex::new();
        world.insert_resource(spatial_index.clone());

        // Create parallel scheduler with optimal thread count
        let mut scheduler = EcsScheduler::new(None).expect("Failed to create ECS scheduler");
        configure_parallel_systems(&mut scheduler, &mut world);
        configure_change_detection(&mut scheduler, &mut world);
        
        // Add spatial sync systems to the scheduler with proper resource access specifications
        scheduler.add_system_with_accesses(
            Stage::PreUpdate,
            "incremental_spatial_sync".to_string(),
            incremental_spatial_sync,
            vec![
                crate::ecs::ResourceAccess::write::<OptimalSpatialIndex>(),
                // Component queries (Position, Owner, Movement) handled by Bevy's system
                // Commands handled by Bevy's system
            ],
            &mut world,
        );
        
        scheduler.add_system_with_accesses(
            Stage::PostUpdate,
            "spatial_cache_maintenance".to_string(),
            spatial_cache_maintenance,
            vec![
                crate::ecs::ResourceAccess::write::<OptimalSpatialIndex>(),
                // Commands handled by Bevy's system  
                // SpatialSyncNeeded resource access handled by Bevy's system
            ],
            &mut world,
        );

        // Configure query cache for ECS operations
        let query_cache = GameCacheBuilder::new()
            .max_memory_mb(256) // 256MB for query results  
            .default_ttl(std::time::Duration::from_secs(60)) // 1 minute TTL
            .turn_based_invalidation(true)
            .build();

        Self {
            world,
            scheduler,
            spatial_index,
            archetype_manager: ArchetypeManager::new(),
            query_cache,
            world_generation: 1,
            #[cfg(debug_assertions)]
            reload_manager: Self::setup_reloader(),
            last_update: Instant::now(),
        }
    }

    /// Update the world by one frame/tick
    pub fn update(&mut self) {
        let correlation_id = LoggingSystem::generate_correlation_id();
        let frame_start = Instant::now();
        
        // Calculate delta time
        let now = Instant::now();
        let delta_time = (now - self.last_update).as_secs_f32();
        self.last_update = now;
        
        debug!(
            target: "game::world",
            correlation_id = correlation_id,
            delta_time = delta_time,
            entity_count = self.world.entities().len(),
            "Starting world update cycle"
        );

        // Game time is now updated by the time_system, not directly here
        // This allows the time controller to manage pausing, stepping, speed control
        if let Some(game_time) = self.world.get_resource::<GameTime>() {
            debug!(
                target: "game::world",
                correlation_id = correlation_id,
                turn = game_time.turn,
                tick = game_time.tick,
                mode = ?game_time.playback_mode(),
                speed = game_time.speed(),
                interpolation_factor = game_time.interpolation_factor().into_inner(),
                "Game state"
            );
        }

        // Spatial indexing now handled automatically by incremental_spatial_sync system
        // No expensive full rebuilds needed!

        // Perform archetype maintenance (component organization)
        // Note: This is lightweight - only cleans up empty archetypes periodically
        if self.get_turn() % 10 == 0 {  // Every 10 turns
            let cleanup_start = Instant::now();
            let cleaned = self.archetype_manager.cleanup();
            let cleanup_duration = cleanup_start.elapsed().as_secs_f64() * 1000.0;
            
            if cleaned > 0 {
                info!(
                    target: "game::world",
                    correlation_id = correlation_id,
                    cleaned_archetypes = cleaned,
                    duration_ms = cleanup_duration,
                    "Cleaned up empty archetypes"
                );
                
                game_logging::log_archetype_operation(0, "cleanup", cleaned);
            }
            
            game_logging::log_performance_event("archetype_cleanup", cleanup_duration, cleaned);
        }

        // Validate hierarchy integrity periodically
        if self.get_turn() % 100 == 0 {  // Every 100 turns
            if let Some(hierarchy) = self.world.get_resource::<HierarchyQueries>() {
                let validation_start = Instant::now();
                
                match hierarchy.validate_hierarchy() {
                    Ok(validation) => {
                        let validation_duration = validation_start.elapsed().as_secs_f64() * 1000.0;
                        
                        if validation.has_cycles {
                            error!(
                                target: "game::world::hierarchy",
                                correlation_id = correlation_id,
                                entity_count = validation.entity_count,
                                relationship_count = validation.relationship_count,
                                "Hierarchy cycles detected!"
                            );
                        }
                        
                        if validation.orphaned_entities > 0 {
                            warn!(
                                target: "game::world::hierarchy",
                                correlation_id = correlation_id,
                                orphaned_entities = validation.orphaned_entities,
                                "Found orphaned entities in hierarchy"
                            );
                        }
                        
                        debug!(
                            target: "game::world::hierarchy",
                            correlation_id = correlation_id,
                            entity_count = validation.entity_count,
                            relationship_count = validation.relationship_count,
                            orphaned_entities = validation.orphaned_entities,
                            has_cycles = validation.has_cycles,
                            validation_duration_ms = validation_duration,
                            "Hierarchy validation completed"
                        );
                        
                        game_logging::log_performance_event("hierarchy_validation", validation_duration, validation.entity_count);
                    }
                    Err(e) => {
                        error!(
                            target: "game::world::hierarchy",
                            correlation_id = correlation_id,
                            error = %e,
                            "Hierarchy validation failed"
                        );
                    }
                }
            }
        }

        // Process hot reload events
        #[cfg(debug_assertions)]
        self.process_reload_events();

        // Run systems in parallel stages
        let stages = [Stage::PreUpdate, Stage::Update, Stage::PostUpdate, Stage::Cleanup];
        for stage in stages {
            if let Err(errors) = self.scheduler.run_stage(stage, &mut self.world) {
                tracing::error!("System execution errors: {:?}", errors);
            }
        }
    }

    /// Update the world with a fixed time step (useful for deterministic simulation)
    pub fn update_fixed(&mut self, fixed_delta: f32) {
        // Game time is now updated by systems, but we can set a target speed
        if let Some(game_time) = self.world.get_resource::<GameTime>() {
            let target_speed = fixed_delta / (1.0 / 60.0); // Convert to speed multiplier
            let _ = game_time.set_speed(target_speed); // Ignore errors for now
            
            debug!(
                target: "game::world::fixed",
                fixed_delta = fixed_delta,
                target_speed = target_speed,
                "Fixed timestep update"
            );
        }

        // Spatial indexing now handled automatically by incremental_spatial_sync system
        // No expensive full rebuilds needed in fixed timestep mode!

        // Run systems in parallel stages
        let stages = [Stage::PreUpdate, Stage::Update, Stage::PostUpdate, Stage::Cleanup];
        for stage in stages {
            if let Err(errors) = self.scheduler.run_stage(stage, &mut self.world) {
                tracing::error!("System execution errors: {:?}", errors);
            }
        }
    }

    /// Get a reference to the ECS world for external access
    pub fn world(&self) -> &World {
        &self.world
    }

    /// Get a mutable reference to the ECS world for external modifications
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    /// Get scheduler performance metrics
    pub fn scheduler_metrics(&self) -> SchedulerMetrics {
        self.scheduler.metrics()
    }

    /// Check if the scheduler is currently busy executing systems
    pub fn is_updating(&self) -> bool {
        self.scheduler.is_busy()
    }

    /// Access the high-performance spatial index
    pub fn spatial_index(&self) -> &OptimalSpatialIndex {
        &self.spatial_index
    }

    /// Find all entities at a specific hex position using R-tree
    pub fn entities_at_position(&self, pos: IVec2) -> Vec<Entity> {
        self.spatial_index.entities_at_position(pos)
    }

    /// Find all entities within a hex range using optimized R-tree spatial queries
    pub fn entities_in_range(&self, center: IVec2, radius: u32) -> Vec<Entity> {
        self.spatial_index.entities_in_range(center, radius)
    }

    /// Find all entities owned by a player using high-performance spatial queries
    pub fn entities_owned_by_player(&self, player_id: u32) -> Vec<Entity> {
        self.spatial_index.entities_owned_by_player(player_id)
    }

    /// Find all hierarchical entities with relationships
    pub fn get_hierarchical_entities(&mut self) -> Vec<Entity> {
        use crate::core::caching::{CacheKey, QueryCacheKey, QueryResult, QueryType, CachePriority};
        use tokio::runtime::Handle;
        use std::any::TypeId;
        
        // Try cache first if we have a tokio runtime
        if let Ok(handle) = Handle::try_current() {
            let cache_key = QueryCacheKey {
                component_signature: crate::core::hashing::HashStrategies::hash_type_signature(&[TypeId::of::<Hierarchical>()]),  
                filter_hash: None,
                player_id: None,
                world_generation: self.world_generation,
                query_type: QueryType::ComponentQuery,
            };
            
            // Check cache
            if let Ok(Some(QueryResult::Entities(entities))) = handle.block_on(
                self.query_cache.get::<QueryResult>(&CacheKey::Query(cache_key.clone()))
            ) {
                return entities;
            }
            
            // Cache miss - perform query
            let mut query = self.world.query_filtered::<Entity, With<Hierarchical>>();
            let entities: Vec<Entity> = query.iter(&self.world).collect();
            
            // Cache result asynchronously
            let cache = self.query_cache.clone();
            let cache_key_clone = cache_key.clone();
            let entities_clone = entities.clone();
            handle.spawn(async move {
                let result = QueryResult::Entities(entities_clone);
                let _ = cache.set(CacheKey::Query(cache_key_clone), result, CachePriority::Normal).await;
            });
            
            entities
        } else {
            // No tokio runtime - fallback to uncached query
            let mut query = self.world.query_filtered::<Entity, With<Hierarchical>>();
            query.iter(&self.world).collect()
        }
    }

    /// Get hierarchy queries resource for advanced relationship operations
    pub fn hierarchy_queries(&self) -> Option<&HierarchyQueries> {
        self.world.get_resource::<HierarchyQueries>()
    }

    /// Find all entities with relationships
    pub fn entities_with_relationships(&mut self) -> Vec<(Entity, Relationships)> {
        use crate::core::caching::{CacheKey, QueryCacheKey, QueryResult, QueryType, CachePriority};
        use tokio::runtime::Handle;
        use std::any::TypeId;
        
        // Try cache first if we have a tokio runtime
        if let Ok(handle) = Handle::try_current() {
            let cache_key = QueryCacheKey {
                component_signature: crate::core::hashing::HashStrategies::hash_type_signature(&[TypeId::of::<Relationships>()]),  
                filter_hash: None,
                player_id: None,
                world_generation: self.world_generation,
                query_type: QueryType::ComponentQuery,
            };
            
            // Check cache - we'll store as serialized component data
            if let Ok(Some(QueryResult::EntitiesWithData { entities, component_data })) = handle.block_on(
                self.query_cache.get::<QueryResult>(&CacheKey::Query(cache_key.clone()))
            ) {
                // Deserialize relationships from component data
                let mut result = Vec::new();
                for (i, entity) in entities.iter().enumerate() {
                    if let Some(data) = component_data.get(i) {
                        if let crate::core::caching::ComponentData::Serialized { data, .. } = data {
                            if let Ok(relationships) = bincode::deserialize::<Relationships>(data) {
                                result.push((*entity, relationships));
                            }
                        }
                    }
                }
                return result;
            }
            
            // Cache miss - perform query
            let mut query = self.world.query::<(Entity, &Relationships)>();
            let entities_with_relationships: Vec<(Entity, Relationships)> = query.iter(&self.world).map(|(e, r)| (e, r.clone())).collect();
            
            // Cache result asynchronously
            let cache = self.query_cache.clone();
            let cache_key_clone = cache_key.clone();
            let entities: Vec<Entity> = entities_with_relationships.iter().map(|(e, _)| *e).collect();
            let component_data: Vec<crate::core::caching::ComponentData> = entities_with_relationships.iter().map(|(_, r)| {
                let data = bincode::serialize(r).unwrap_or_default();
                crate::core::caching::ComponentData::Serialized {
                    type_id: {
                        use std::hash::{Hash, Hasher};
                        let mut hasher = std::collections::hash_map::DefaultHasher::new();
                        TypeId::of::<Relationships>().hash(&mut hasher);
                        hasher.finish()
                    },
                    data,
                }
            }).collect();
            
            handle.spawn(async move {
                let result = QueryResult::EntitiesWithData { entities, component_data };
                let _ = cache.set(CacheKey::Query(cache_key_clone), result, CachePriority::Normal).await;
            });
            
            entities_with_relationships
        } else {
            // No tokio runtime - fallback to uncached query
            let mut query = self.world.query::<(Entity, &Relationships)>();
            query.iter(&self.world).map(|(e, r)| (e, r.clone())).collect()
        }
    }

    /// Find all parent entities (entities that have children)
    pub fn find_parent_entities(&mut self) -> Vec<Entity> {
        // Work around borrow checker by doing this in two phases
        let hierarchical_entities: Vec<Entity> = {
            let mut query = self.world.query_filtered::<Entity, With<Hierarchical>>();
            query.iter(&self.world).collect()
        };
        
        // Now filter using hierarchy in a separate phase  
        if let Some(hierarchy) = self.world.get_resource::<HierarchyQueries>() {
            hierarchical_entities.into_iter()
                .filter(|&entity| !hierarchy.children(entity).is_empty())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Find all root entities (entities with no parents)
    pub fn find_root_entities(&mut self) -> Vec<Entity> {
        // Use a two-phase approach to avoid borrow conflicts
        let has_hierarchy = self.world.get_resource::<HierarchyQueries>().is_some();
        if has_hierarchy {
            // Get a mutable reference to the hierarchy queries resource
            if let Some(hierarchy) = self.world.remove_resource::<HierarchyQueries>() {
                let result = hierarchy.find_roots(&mut self.world);
                self.world.insert_resource(hierarchy);
                result
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    }

    /// Find all entities at a specific hierarchy depth from a root
    pub fn entities_at_depth(&self, root: Entity, depth: u32) -> Vec<Entity> {
        if let Some(hierarchy) = self.hierarchy_queries() {
            hierarchy.entities_at_depth(root, depth)
        } else {
            Vec::new()
        }
    }

    /// Validate hierarchy integrity and return validation results
    pub fn validate_hierarchy(&self) -> Result<crate::ecs::hierarchy::HierarchyValidation, crate::ecs::hierarchy::HierarchyError> {
        if let Some(hierarchy) = self.hierarchy_queries() {
            hierarchy.validate_hierarchy()
        } else {
            Err(crate::ecs::hierarchy::HierarchyError::GraphError("HierarchyQueries resource not found".to_string()))
        }
    }

    /// Find all entities owned by player using R-tree spatial queries
    pub fn entities_owned_by(&self, player_id: u32) -> Vec<Entity> {
        self.spatial_index.entities_owned_by_player(player_id)
    }

    /// Find entities owned by player within a specific range using R-tree optimization
    pub fn entities_owned_by_in_range(&self, player_id: u32, center: IVec2, radius: u32) -> Vec<Entity> {
        self.spatial_index.owned_entities_in_range(player_id, center, radius)
    }

    /// Find all movable entities (units) using high-performance R-tree queries
    pub fn movable_entities(&self) -> Vec<Entity> {
        self.spatial_index.movable_entities()
    }

    /// Find owned units at specific position using R-tree multi-component query
    pub fn owned_units_at_position(&self, pos: IVec2, player_id: u32) -> Vec<Entity> {
        self.spatial_index.owned_units_at_position(pos, player_id)
    }

    /// Get spatial performance metrics
    pub fn spatial_metrics(&self) -> SpatialStats {
        self.spatial_index.stats()
    }

    /// Get archetype organization statistics
    pub fn archetype_stats(&self) -> crate::ecs::archetypes::ArchetypeStats {
        self.archetype_manager.stats()
    }

    /// Access archetype manager for component-based queries (read-only)
    pub fn archetype_manager(&self) -> &ArchetypeManager {
        &self.archetype_manager
    }

    // === CLEAN INTEGRATION METHODS (SoC-Compliant) ===
    //
    // Design Pattern: Complementary Responsibilities
    // 
    // ArchetypeManager: Answers "WHAT components does an entity have?"
    // - Organizes entities by component signature
    // - Provides efficient component-based entity grouping
    // - Tracks entity archetype membership
    //
    // OptimalSpatialIndex: Answers "WHERE are entities and HOW to query efficiently?"
    // - Maintains spatial indexing for position-based queries
    // - Uses R-tree for O(log n) spatial operations
    // - Handles query optimization and incremental updates
    //
    // Integration Philosophy:
    // 1. No duplicate responsibilities
    // 2. Each system owns its domain completely
    // 3. Coordination happens through GameWorld methods
    // 4. Performance gains from combining both approaches
    
    /// Find entities by component signature, then apply spatial filtering
    /// Combines archetype pre-filtering with spatial optimization  
    pub fn entities_with_components_in_range<T: BundleComponentExtractor>(&self, center: IVec2, radius: u32) -> Vec<Entity> {
        // Step 1: ArchetypeManager finds entities with specific components (WHAT)
        let signature = T::extract_component_types();
        let candidate_entities = self.archetype_manager.find_archetypes_with_components(&signature)
            .into_iter()
            .flat_map(|arch_id| self.archetype_manager.get_archetype_entities(arch_id))
            .collect::<Vec<_>>();
        
        // Step 2: SpatialIndex applies spatial filtering (WHERE)
        let spatial_entities = self.spatial_index.entities_in_range(center, radius);
        
        // Step 3: Intersection - entities that match both component AND spatial criteria
        let spatial_set: std::collections::HashSet<Entity> = spatial_entities.into_iter().collect();
        candidate_entities.into_iter()
            .filter(|entity| spatial_set.contains(entity))
            .collect()
    }

    /// Efficient query: Find entities of specific archetype owned by player
    /// Demonstrates clean separation: archetype organization + ownership filtering
    pub fn archetype_entities_owned_by(&self, player_id: u32) -> std::collections::HashMap<crate::ecs::archetypes::ArchetypeId, Vec<Entity>> {
        use std::collections::HashMap;
        let mut result = HashMap::new();
        
        // Get all owned entities (SpatialIndex responsibility)
        let owned_entities = self.spatial_index.entities_owned_by_player(player_id);
        
        // Group by archetype (ArchetypeManager responsibility)
        for entity in owned_entities {
            if let Some(archetype_id) = self.archetype_manager.storage().get_entity_archetype(entity) {
                result.entry(archetype_id).or_insert_with(Vec::new).push(entity);
            }
        }
        
        result
    }

    // === ENTITY LIFECYCLE INTEGRATION ===
    
    /// Spawn entity and register with both archetype and spatial systems
    /// Maintains clean separation: GameWorld coordinates, systems handle their domains
    pub fn spawn_entity_registered<T: BundleComponentExtractor>(&mut self, bundle: T) -> Entity {
        let spawn_start = Instant::now();
        let correlation_id = LoggingSystem::generate_correlation_id();
        
        // Step 1: Spawn in bevy_ecs world (core ECS responsibility)
        let entity = self.world.spawn(bundle).id();
        
        // Step 2: Register with archetype system (component signature tracking)
        let archetype_id = self.archetype_manager.register_entity::<T>(entity);
        let spawn_duration = spawn_start.elapsed().as_secs_f64() * 1000.0;
        
        // Step 3: SpatialIndex will pick it up automatically via incremental sync system
        // No need to manually sync here - happens automatically via Added<Position> queries
        
        // Log the entity creation using game-specific logging
        if let Some(name_component) = self.world.get::<Name>(entity) {
            info!(
                target: "game::world::entities",
                correlation_id = correlation_id,
                entity = ?entity,
                archetype_id = ?archetype_id,
                name = %name_component.value(),
                spawn_duration_ms = spawn_duration,
                "Entity spawned with name"
            );
            
            game_logging::log_entity_operation(entity, "spawn", Some(name_component.value()));
        } else {
            info!(
                target: "game::world::entities",
                correlation_id = correlation_id,
                entity = ?entity,
                archetype_id = ?archetype_id,
                spawn_duration_ms = spawn_duration,
                "Entity spawned"
            );
            
            game_logging::log_entity_operation(entity, "spawn", None);
        }
        
        // Log position if available
        if let Some(position) = self.world.get::<Position>(entity) {
            game_logging::log_spatial_operation(position.hex(), "entity_spawn", None);
        }
        
        game_logging::log_archetype_operation(archetype_id.data().as_ffi(), "entity_added", 1);
        game_logging::log_performance_event("entity_spawn", spawn_duration, 1);
        
        // Increment world generation to invalidate caches
        self.increment_world_generation();
        
        entity
    }

    /// Remove entity from all tracking systems
    /// Maintains clean separation during entity destruction
    pub fn despawn_entity_registered(&mut self, entity: Entity) -> bool {
        let despawn_start = Instant::now();
        let correlation_id = LoggingSystem::generate_correlation_id();
        
        // Get entity info before despawning for logging
        let name = self.world.get::<Name>(entity).map(|n| n.value().to_string());
        let position = self.world.get::<Position>(entity).map(|p| p.hex());
        
        // Step 1: Remove from archetype tracking (component organization)
        let archetype_result = self.archetype_manager.unregister_entity(entity);
        
        // Step 2: Remove from ECS world (spatial index automatically updated via RemovedComponents<Position>)
        let success = if let Some(entity_mut) = self.world.get_entity_mut(entity) {
            entity_mut.despawn();
            true
        } else {
            false
        };
        
        let despawn_duration = despawn_start.elapsed().as_secs_f64() * 1000.0;
        
        if success {
            info!(
                target: "game::world::entities",
                correlation_id = correlation_id,
                entity = ?entity,
                name = ?name,
                position = ?position,
                despawn_duration_ms = despawn_duration,
                "Entity despawned successfully"
            );
            
            game_logging::log_entity_operation(entity, "despawn", name.as_deref());
            
            if let Some(pos) = position {
                game_logging::log_spatial_operation(pos, "entity_despawn", None);
            }
            
            if archetype_result.is_ok() {
                game_logging::log_archetype_operation(0, "entity_removed", 1);
            }
        } else {
            warn!(
                target: "game::world::entities",
                correlation_id = correlation_id,
                entity = ?entity,
                "Failed to despawn entity - entity not found"
            );
        }
        
        game_logging::log_performance_event("entity_despawn", despawn_duration, if success { 1 } else { 0 });
        
        if success {
            // Increment world generation to invalidate caches
            self.increment_world_generation();
        }
        
        success
    }

    /// Update entity archetype when components change
    /// Maintains archetype organization without interfering with spatial indexing
    pub fn update_entity_archetype<T: BundleComponentExtractor>(&mut self, entity: Entity) -> Result<(), String> {
        self.archetype_manager.update_entity_archetype::<T>(entity)
            .map_err(|e| format!("Failed to update entity archetype: {}", e))
    }

    /// Cleanup empty archetypes (maintenance operation)
    /// Pure archetype manager responsibility
    pub fn cleanup_archetypes(&mut self) -> usize {
        self.archetype_manager.cleanup()
    }

    /// Advance world generation and invalidate all caches
    pub async fn advance_world_generation(&mut self) {
        self.world_generation += 1;
        self.query_cache.clear().await;
        
        // Notify subsystems
        tokio::spawn(async move {
            broadcast_cache_invalidation(crate::core::caching::events::CacheInvalidationEvent::TileUpdated { tile_id: 0, batch_size: 1 }).await;
        });
    }

    /// Invalidate caches when entity is modified
    pub async fn invalidate_entity_caches(&self, entity: Entity, archetype_changed: bool, position_changed: Option<IVec2>) {
        tokio::spawn(async move {
            broadcast_cache_invalidation(crate::core::caching::events::CacheInvalidationEvent::TileUpdated { tile_id: 0, batch_size: 1 }).await;
        });
    }

    /// Report all cache metrics to unified system
    pub async fn report_all_cache_metrics(&self) {
        // Report archetype cache metrics
        self.archetype_manager.report_metrics().await;
        
        // Report main query cache metrics
        let stats = self.query_cache.stats().await;
        use crate::core::caching::{global_cache_events, SubsystemStats};
        let subsystem_stats = SubsystemStats {
            hits: stats.total_hits,
            misses: stats.total_misses,
            entries: stats.cache_count,
            memory_usage_bytes: stats.memory_usage_bytes,
            avg_access_time_micros: stats.avg_access_time_micros,
            last_updated: std::time::Instant::now(),
        };
        global_cache_events().register_subsystem_metrics("world_query", subsystem_stats).await;
    }

    /// Initialize a new game with default entities
    pub fn initialize_game(&mut self, player_name: String, civilization: String) {

        // Clear existing world state (keep resources)
        self.world.clear_entities();

        // Update player data
        if let Some(mut players) = self.world.get_resource_mut::<Players>() {
            if let Some(player_data) = players.data.get_mut(&1) {
                player_data.name = player_name;
                player_data.civilization = civilization;
            }
        }

        // Create initial terrain
        for q in -5i32..=5i32 {
            for r in -5i32..=5i32 {
                let terrain_type = if q.abs() + r.abs() <= 2 {
                    "plains"
                } else if q.abs() + r.abs() <= 4 {
                    "grassland"  
                } else {
                    "forest"
                };
                
                // Create terrain entity with archetype tracking
                self.spawn_entity_registered((
                    crate::ecs::components::Position::new_unchecked(q, r),
                    crate::ecs::components::Renderable::new(format!("terrain_{}", terrain_type), 0).unwrap(),
                    crate::ecs::components::Name::new(format!("{} Tile", terrain_type)).unwrap(),
                    crate::ecs::components::Owner::neutral(),
                ));
            }
        }

        // Create a starting city for the player with archetype tracking
        self.spawn_entity_registered(crate::ecs::entities::LivingEntityBundle {
            position: crate::ecs::components::Position::new_unchecked(0, 0),
            health: crate::ecs::components::Health::new(100.0).unwrap(),
            renderable: crate::ecs::components::Renderable::new("city".to_string(), 1).unwrap(),
            name: crate::ecs::components::Name::new("Capital".to_string()).unwrap(),
            owner: crate::ecs::components::Owner::player(1, true).unwrap(),
        });

        // Create a starting unit with archetype tracking
        self.spawn_entity_registered(crate::ecs::entities::UnitBundle {
            position: crate::ecs::components::Position::new_unchecked(1, 0),
            movement: crate::ecs::components::Movement::new(3.0).unwrap(),
            health: crate::ecs::components::Health::new(20.0).unwrap(),
            renderable: crate::ecs::components::Renderable::new("unit_scout".to_string(), 2).unwrap(),
            name: crate::ecs::components::Name::new("scout".to_string()).unwrap(),
            owner: crate::ecs::components::Owner::player(1, true).unwrap(),
        });

        // Add some resources with archetype tracking
        self.spawn_entity_registered((
            crate::ecs::components::Position::new_unchecked(2, -1),
            crate::ecs::components::Renderable::new("resource_gold".to_string(), 1).unwrap(),
            crate::ecs::components::Name::new("Gold Deposit".to_string()).unwrap(),
            crate::ecs::components::Owner::neutral(),
        ));
        
        self.spawn_entity_registered((
            crate::ecs::components::Position::new_unchecked(-2, 1),
            crate::ecs::components::Renderable::new("resource_iron".to_string(), 1).unwrap(),
            crate::ecs::components::Name::new("Iron Deposit".to_string()).unwrap(),
            crate::ecs::components::Owner::neutral(),
        ));

        // Validate archetype integration is working
        let archetype_stats = self.archetype_stats();
        tracing::info!("Game world initialized with {} entities across {} archetypes", 
                      self.world.entities().len(),
                      archetype_stats.total_archetypes);
        
        // Log archetype distribution for debugging
        tracing::debug!("Archetype distribution: {} archetypes, {} total entities, avg {:.1} entities per archetype",
                       archetype_stats.total_archetypes,
                       archetype_stats.total_entities, 
                       archetype_stats.avg_entities_per_archetype);
    }

    /// Demonstration method: Find all units in range using archetype pre-filtering
    /// Shows the integration working: component filtering + spatial optimization
    pub fn find_units_in_range_demo(&self, center: glam::IVec2, radius: u32) -> Vec<Entity> {
        // This uses the integrated archetype + spatial system
        self.entities_with_components_in_range::<crate::ecs::entities::UnitBundle>(center, radius)
    }

    /// Demonstration method: Find all cities/buildings in range  
    pub fn find_buildings_in_range_demo(&self, center: glam::IVec2, radius: u32) -> Vec<Entity> {
        self.entities_with_components_in_range::<crate::ecs::entities::LivingEntityBundle>(center, radius)
    }

    /// Get detailed archetype analysis for debugging/monitoring
    pub fn archetype_analysis(&self) -> std::collections::HashMap<crate::ecs::archetypes::ArchetypeId, Vec<Entity>> {
        // Example: Get all entities owned by player 1, grouped by archetype
        self.archetype_entities_owned_by(1)
    }

    /// Get the current turn number
    pub fn get_turn(&self) -> u32 {
        self.world
            .get_resource::<GameTime>()
            .map(|time| time.turn)
            .unwrap_or(1)
    }
    
    /// Get current world generation for cache invalidation
    pub fn world_generation(&self) -> u32 {
        self.world_generation
    }
    
    /// Increment world generation and invalidate caches
    pub fn increment_world_generation(&mut self) {
        self.world_generation += 1;
        
        // Invalidate query cache when world generation changes
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let cache = self.query_cache.clone();
            handle.spawn(async move {
                cache.clear().await;
            });
        }
        
        debug!(
            target: "game::world::cache",
            world_generation = self.world_generation,
            "World generation incremented, caches invalidated"
        );
    }
    
    /// Export entities with caching for expensive serialization operations
    fn export_entities_cached(&mut self) -> Vec<SerializableEntity> {
        use crate::core::caching::{CacheKey, CachePriority};
        use tokio::runtime::Handle;
        
        // Try to get cached entity data if available
        if let Ok(handle) = Handle::try_current() {
            let cache_key = CacheKey::Custom(format!("serialized_entities_{}", self.world_generation));
            
            // Check cache for complete entity serialization
            if let Ok(Some(cached_entities)) = handle.block_on(
                self.query_cache.get::<Vec<SerializableEntity>>(&cache_key)
            ) {
                debug!(
                    target: "game::world::cache",
                    entity_count = cached_entities.len(),
                    world_generation = self.world_generation,
                    "Using cached entity serialization data"
                );
                return cached_entities;
            }
        }
        
        // Cache miss - perform expensive serialization
        let export_start = std::time::Instant::now();
        let mut entities = Vec::new();
        
        // Query all entities and serialize their components
        let mut entity_query = self.world.query::<Entity>();
        for entity in entity_query.iter(&self.world) {
            let stable_id = StableEntityId::from(entity);
            
            // Extract all possible components for this entity
            let position = self.world.get::<crate::ecs::components::Position>(entity).cloned();
            let movement = self.world.get::<crate::ecs::components::Movement>(entity).cloned();
            let health = self.world.get::<crate::ecs::components::Health>(entity).cloned();
            let renderable = self.world.get::<crate::ecs::components::Renderable>(entity).cloned();
            let name = self.world.get::<crate::ecs::components::Name>(entity).cloned();
            let owner = self.world.get::<crate::ecs::components::Owner>(entity).cloned();
            let relationships = self.world.get::<Relationships>(entity).cloned();
            let hierarchical = self.world.get::<Hierarchical>(entity).is_some();
            
            // Only serialize entities that have at least one component
            if position.is_some() || movement.is_some() || health.is_some() || 
               renderable.is_some() || name.is_some() || owner.is_some() || 
               relationships.is_some() || hierarchical {
                entities.push(SerializableEntity {
                    stable_id,
                    position,
                    movement,
                    health,
                    renderable,
                    name,
                    owner,
                    relationships,
                    hierarchical,
                });
            }
        }
        
        let export_duration = export_start.elapsed().as_secs_f64() * 1000.0;
        
        debug!(
            target: "game::world::cache",
            entity_count = entities.len(),
            world_generation = self.world_generation,
            export_duration_ms = export_duration,
            "Serialized entities for caching"
        );
        
        // Cache the result asynchronously for future use
        if let Ok(handle) = Handle::try_current() {
            let cache = self.query_cache.clone();
            let cache_key = CacheKey::Custom(format!("serialized_entities_{}", self.world_generation));
            let entities_clone = entities.clone();
            
            handle.spawn(async move {
                let _ = cache.set(cache_key, entities_clone, CachePriority::Normal).await;
            });
        }
        
        entities
    }

    /// Check if the game is paused
    pub fn is_paused(&self) -> bool {
        self.world
            .get_resource::<GameTime>()
            .map(|time| time.paused)
            .unwrap_or(false)
    }

    /// Pause or unpause the game
    pub fn set_paused(&mut self, paused: bool) {
        if let Some(mut game_time) = self.world.get_resource_mut::<GameTime>() {
            game_time.paused = paused;
        }
    }

    /// Get the current player ID
    pub fn get_current_player(&self) -> u32 {
        self.world
            .get_resource::<Players>()
            .map(|players| players.current_player)
            .unwrap_or(1)
    }

    /// Get entity count for different types
    pub fn get_entity_stats(&mut self) -> EntityStats {
        let total = self.world.entities().len() as u32;
        
        // We need to create temporary query state for immutable access
        let mut movement_query_state = self.world.query_filtered::<&crate::ecs::components::Movement, ()>();
        let units = movement_query_state.iter(&self.world).count() as u32;
            
        let mut city_query_state = self.world.query::<(&crate::ecs::components::Name, &crate::ecs::components::Health)>();
        let cities = city_query_state.iter(&self.world)
            .filter(|(name, _)| name.value().contains("Capital") || name.value().contains("City"))
            .count() as u32;

        // Count hierarchical entities for better statistics
        let mut hierarchical_query_state = self.world.query_filtered::<Entity, With<Hierarchical>>();
        let hierarchical = hierarchical_query_state.iter(&self.world).count() as u32;

        EntityStats {
            total,
            units,
            cities,
            hierarchical,
        }
    }
}

impl Default for GameWorld {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about entities in the world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityStats {
    pub total: u32,
    pub units: u32,
    pub cities: u32,
    pub hierarchical: u32,
}

/// Serializable entity data containing all components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableEntity {
    pub stable_id: StableEntityId,
    pub position: Option<crate::ecs::components::Position>,
    pub movement: Option<crate::ecs::components::Movement>,
    pub health: Option<crate::ecs::components::Health>,
    pub renderable: Option<crate::ecs::components::Renderable>,
    pub name: Option<crate::ecs::components::Name>,
    pub owner: Option<crate::ecs::components::Owner>,
    pub relationships: Option<Relationships>,
    pub hierarchical: bool,
}

/// World state that can be serialized for saving/loading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldState {
    pub game_time: GameTime,
    pub players: Players,
    pub camera_position: (f32, f32), // x, y position
    pub camera_zoom: f32,
    pub entity_count: u32,
    // Complete entity serialization with all components
    pub entities: Vec<SerializableEntity>,
    // Hierarchical entity data for save/load (legacy - now included in entities)
    pub entity_relationships: Vec<(StableEntityId, Relationships)>,
    pub hierarchical_entities: Vec<StableEntityId>,
}

impl GameWorld {
    /// Export the current world state for serialization
    pub fn export_state(&mut self) -> WorldState {
        let game_time = self.world.get_resource::<GameTime>()
            .cloned()
            .unwrap_or_default();
            
        let players = self.world.get_resource::<Players>()
            .cloned()
            .unwrap_or_default();
            
        let camera = self.world.get_resource::<Camera>();
        let (camera_position, camera_zoom) = camera
            .map(|c| ((c.target.x, c.target.y), c.zoom))
            .unwrap_or(((0.0, 0.0), 1.0));

        // Export all entities with their components (with caching for frequently accessed data)
        let entities = self.export_entities_cached();

        // Export hierarchical entity data (legacy - for backwards compatibility)
        let mut entity_relationships = Vec::new();
        let mut hierarchical_entities = Vec::new();
        
        let mut relationship_query = self.world.query::<(Entity, &Relationships)>();
        for (entity, relationships) in relationship_query.iter(&self.world) {
            entity_relationships.push((entity.into(), relationships.clone()));
        }
        
        let mut hierarchical_query = self.world.query_filtered::<Entity, With<Hierarchical>>();
        for entity in hierarchical_query.iter(&self.world) {
            hierarchical_entities.push(entity.into());
        }

        tracing::info!("Exported {} entities for serialization", entities.len());

        WorldState {
            game_time,
            players,
            camera_position,
            camera_zoom,
            entity_count: entities.len() as u32,
            entities,
            entity_relationships,
            hierarchical_entities,
        }
    }

    /// Import world state from serialization (complete implementation)
    pub fn import_state(&mut self, state: WorldState) {
        // Clear existing entities while preserving resources
        self.world.clear_entities();
        
        // Update resources
        self.world.insert_resource(state.game_time);
        self.world.insert_resource(state.players);
        
        if let Some(mut camera) = self.world.get_resource_mut::<Camera>() {
            camera.target = glam::Vec2::new(state.camera_position.0, state.camera_position.1);
            camera.zoom = state.camera_zoom;
        }

        // Create entity ID mapping for stable restoration
        let mut entity_mapping = std::collections::HashMap::new();

        // First pass: Create all entities and map IDs
        for serialized_entity in &state.entities {
            let entity = self.world.spawn_empty().id();
            entity_mapping.insert(serialized_entity.stable_id, entity);
        }

        // Second pass: Add components to entities and register with archetype manager
        let mut restored_count = 0;
        for serialized_entity in &state.entities {
            if let Some(&entity) = entity_mapping.get(&serialized_entity.stable_id) {
                let mut entity_mut = self.world.entity_mut(entity);
                
                // Add components based on what was serialized
                if let Some(position) = &serialized_entity.position {
                    entity_mut.insert(position.clone());
                }
                if let Some(movement) = &serialized_entity.movement {
                    entity_mut.insert(movement.clone());
                }
                if let Some(health) = &serialized_entity.health {
                    entity_mut.insert(health.clone());
                }
                if let Some(renderable) = &serialized_entity.renderable {
                    entity_mut.insert(renderable.clone());
                }
                if let Some(name) = &serialized_entity.name {
                    entity_mut.insert(name.clone());
                }
                if let Some(owner) = &serialized_entity.owner {
                    entity_mut.insert(owner.clone());
                }
                if let Some(relationships) = &serialized_entity.relationships {
                    // Update relationship targets to use new entity IDs
                    let mut updated_relationships = relationships.clone();
                    updated_relationships.remap_entities(&entity_mapping);
                    entity_mut.insert(updated_relationships);
                }
                if serialized_entity.hierarchical {
                    entity_mut.insert(Hierarchical);
                }

                // Register with archetype manager based on components
                // This is a bit tricky since we need to determine the bundle type dynamically
                if serialized_entity.position.is_some() && 
                   serialized_entity.movement.is_some() && 
                   serialized_entity.health.is_some() {
                    // This is a UnitBundle entity
                    let _ = self.archetype_manager.register_entity::<crate::ecs::entities::UnitBundle>(entity);
                } else if serialized_entity.position.is_some() && 
                         serialized_entity.health.is_some() {
                    // This is a LivingEntityBundle entity
                    let _ = self.archetype_manager.register_entity::<crate::ecs::entities::LivingEntityBundle>(entity);
                } else if serialized_entity.position.is_some() && 
                         serialized_entity.movement.is_some() {
                    // This is a MovableEntityBundle entity
                    let _ = self.archetype_manager.register_entity::<crate::ecs::entities::MovableEntityBundle>(entity);
                }

                restored_count += 1;
            }
        }

        // Legacy: Restore hierarchical entities (for backwards compatibility)
        for stable_id in state.hierarchical_entities {
            if let Some(&entity) = entity_mapping.get(&stable_id) {
                if self.world.get_entity(entity).is_some() {
                    self.world.entity_mut(entity).insert(Hierarchical);
                }
            }
        }

        // Legacy: Restore entity relationships (for backwards compatibility)
        for (stable_id, relationships) in state.entity_relationships {
            if let Some(&entity) = entity_mapping.get(&stable_id) {
                if self.world.get_entity(entity).is_some() {
                    let mut updated_relationships = relationships.clone();
                    updated_relationships.remap_entities(&entity_mapping);
                    self.world.entity_mut(entity).insert(updated_relationships);
                }
            }
        }

        // Sync hierarchy system after import
        if let Some(hierarchy_queries) = self.world.remove_resource::<HierarchyQueries>() {
            // Manually sync the hierarchy using our direct approach
            let mut relationships_query = self.world.query::<(Entity, &Relationships)>();
            let updates: Vec<_> = relationships_query.iter(&self.world)
                .map(|(entity, relationships)| (entity, relationships.clone()))
                .collect();
            
            // Apply updates to hierarchy graph using public interface
            if let Err(e) = hierarchy_queries.update_relationships(updates) {
                tracing::warn!("Failed to sync hierarchy after import: {}", e);
            }
            
            // Put the resource back
            self.world.insert_resource(hierarchy_queries);
        }

        tracing::info!("Imported world state with {} entities restored ({} entities in save file)", 
                      restored_count, state.entity_count);
    }

    /// Setup hot reload system for development builds
    #[cfg(debug_assertions)]
    fn setup_reloader() -> Option<ReloadManager> {
        use std::path::Path;
        
        match ReloadManager::new() {
            Ok(mut manager) => {
                // Add default handlers
                manager.add_handler(Box::new(LuaHandler::new().unwrap()));
                manager.add_handler(Box::new(ConfigHandler::new()));
                manager.add_handler(Box::new(AssetHandler::new()));

                // Watch common script/config directories
                let watch_dirs = [
                    "lua-scripts",
                    "configs", 
                    "assets",
                    "backend/src",  // For system files (informational only)
                ];

                for dir in &watch_dirs {
                    let path = Path::new(dir);
                    if path.exists() {
                        Self::watch_directory_recursive(&mut manager, path);
                    }
                }

                // Start the reloader
                if manager.start().is_ok() {
                    tracing::info!("🔥 Hot reload system activated");
                    Some(manager)
                } else {
                    tracing::warn!("Failed to start hot reload system");
                    None
                }
            }
            Err(e) => {
                tracing::warn!("Hot reload system disabled: {}", e);
                None
            }
        }
    }

    /// Watch directory recursively for file changes
    #[cfg(debug_assertions)]
    fn watch_directory_recursive(manager: &mut ReloadManager, path: &std::path::Path) {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if entry_path.is_file() {
                    let file_type = match entry_path.extension().and_then(|ext| ext.to_str()) {
                        Some("lua") => Some(FileType::Lua),
                        Some("toml" | "json" | "yaml" | "yml") => Some(FileType::Config),
                        Some("png" | "jpg" | "wav" | "glb") => Some(FileType::Asset),
                        _ => None,
                    };
                    
                    if let Some(ft) = file_type {
                        let _ = manager.watch_file(entry_path, ft);
                    }
                } else if entry_path.is_dir() {
                    Self::watch_directory_recursive(manager, &entry_path);
                }
            }
        }
    }

    /// Process pending reload events
    #[cfg(debug_assertions)]
    fn process_reload_events(&mut self) {
        if let Some(ref manager) = self.reload_manager {
            for event in manager.poll_events() {
                match event {
                    ReloadEvent::Reloaded { path, handler } => {
                        tracing::debug!("🔄 Reloaded {} with {}", path.display(), handler);
                    }
                    ReloadEvent::Failed { path, error } => {
                        tracing::warn!("❌ Reload failed for {}: {}", path.display(), error);
                    }
                    ReloadEvent::FileChanged { path } => {
                        tracing::debug!("📝 File changed: {}", path.display());
                    }
                }
            }
        }
    }

    /// Get hot reload statistics (debug builds only)
    #[cfg(debug_assertions)]
    pub fn reload_stats(&self) -> Option<ReloadStats> {
        self.reload_manager.as_ref().map(|m| m.stats())
    }

    /// Access Lua handler for direct script execution (debug builds only)
    #[cfg(debug_assertions)]
    pub fn lua_handler(&self) -> Option<std::sync::Arc<parking_lot::Mutex<mlua::Lua>>> {
        self.reload_manager.as_ref().and_then(|_manager| {
            // This is a bit hacky - in a real implementation we'd want a cleaner API
            // For now, users can access Lua directly through the manager if needed
            None // TODO: Implement proper Lua access
        })
    }

}
