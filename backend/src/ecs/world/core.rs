//! Core GameWorld struct and basic initialization
//!
//! Contains the main GameWorld struct, constructors, and basic world management.

use bevy_ecs::prelude::*;
use std::time::Instant;
use tracing::info;

use crate::core::{
    reloader::ReloadManager,
    caching::{GameCache, GameCacheBuilder}
};
use crate::ecs::{
    resources::*,
    systems::*,
    changes::*,
    spatial::*,
    EcsScheduler,
    archetypes::{ArchetypeManager, ArchetypeSpatialBridge},
    hierarchy::{HierarchyQueries}
};

/// Main game world wrapper that manages the ECS world and systems
#[derive(Debug)]
pub struct GameWorld {
    /// The ECS world containing all entities, components, and resources
    pub world: World,
    /// Parallel scheduler for running systems efficiently
    scheduler: EcsScheduler,
    /// High-performance spatial indexing using R-tree
    spatial_index: OptimalSpatialIndex,
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
        
        // Initialize tile properties system
        match crate::world::tiles::TilePropertiesSystem::new() {
            Ok(properties_system) => {
                world.insert_resource(properties_system);
                info!("🌍 Tile Properties System initialized successfully");
            }
            Err(e) => {
                tracing::warn!("Failed to initialize Tile Properties System: {}", e);
            }
        }

        // Create spatial index and insert as resource
        let spatial_index = OptimalSpatialIndex::new();
        world.insert_resource(spatial_index.clone());

        // Create archetype manager and spatial bridge
        let archetype_manager = ArchetypeManager::new();
        world.insert_resource(archetype_manager.clone());
        
        // Create bridge with shared access to resources
        let archetype_spatial_bridge = ArchetypeSpatialBridge::new(
            archetype_manager,
            spatial_index.clone()
        );
        world.insert_resource(archetype_spatial_bridge);
        info!("🏗️ Archetype management system initialized");

        // Create parallel scheduler with optimal thread count
        let mut scheduler = EcsScheduler::new(None).expect("Failed to create ECS scheduler");
        configure_parallel_systems(&mut scheduler, &mut world);
        configure_change_detection(&mut scheduler, &mut world);
        
        // Add spatial sync systems to the scheduler with proper resource access specifications
        Self::configure_spatial_systems(&mut scheduler, &mut world);
        
        // Create high-performance cache for query results
        let query_cache = GameCacheBuilder::new()
            .max_memory_mb(256)
            .default_ttl(std::time::Duration::from_secs(30))
            .turn_based_invalidation(true)
            .build();

        // Setup hot reload in debug builds
        #[cfg(debug_assertions)]
        let reload_manager = Self::setup_reloader();

        Self {
            world,
            scheduler,
            spatial_index,
            query_cache,
            world_generation: 0,
            #[cfg(debug_assertions)]
            reload_manager,
            last_update: Instant::now(),
        }
    }

    /// Configure spatial indexing systems
    fn configure_spatial_systems(scheduler: &mut EcsScheduler, world: &mut World) {
        use crate::core::Stage;
        
        scheduler.add_system_with_accesses(
            Stage::PreUpdate,
            "incremental_spatial_sync".to_string(),
            incremental_spatial_sync,
            vec![
                crate::ecs::ResourceAccess::write::<OptimalSpatialIndex>(),
                // Component queries (Position, Owner, Movement) handled by Bevy's system
                // Commands handled by Bevy's system
            ],
            world,
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
            world,
        );
        
        // Spatial rebuild check system for performance optimization
        scheduler.add_system_with_accesses(
            Stage::Cleanup,
            "full_spatial_rebuild_check".to_string(),
            full_spatial_rebuild_check,
            vec![
                crate::ecs::ResourceAccess::write::<OptimalSpatialIndex>(),
                crate::ecs::ResourceAccess::read::<GameTime>(),
            ],
            world,
        );
        
        info!("🗺️ Spatial indexing systems configured");
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
    pub fn scheduler_metrics(&self) -> crate::core::SchedulerMetrics {
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

    /// Get current world generation
    pub fn world_generation(&self) -> u32 {
        self.world_generation
    }

    /// Get reference to the query cache
    pub fn query_cache(&self) -> &GameCache {
        &self.query_cache
    }

    /// Get reference to the reload manager (debug builds only)
    #[cfg(debug_assertions)]
    pub fn reload_manager(&self) -> &Option<ReloadManager> {
        &self.reload_manager
    }

    /// Get mutable reference to the reload manager (debug builds only)
    #[cfg(debug_assertions)]
    pub fn reload_manager_mut(&mut self) -> &mut Option<ReloadManager> {
        &mut self.reload_manager
    }

    /// Increment world generation and invalidate caches
    pub(super) fn increment_world_generation(&mut self) {
        self.world_generation += 1;
    }

    /// Get the last update time
    pub fn last_update(&self) -> Instant {
        self.last_update
    }

    /// Update the last update time
    pub fn set_last_update(&mut self, time: Instant) {
        self.last_update = time;
    }

    /// Get mutable reference to the scheduler
    pub fn scheduler_mut(&mut self) -> &mut EcsScheduler {
        &mut self.scheduler
    }

    /// Get reference to the scheduler
    pub fn scheduler(&self) -> &EcsScheduler {
        &self.scheduler
    }

    /// Get current turn number
    pub fn get_turn(&self) -> u32 {
        self.world.get_resource::<GameTime>()
            .map(|game_time| game_time.turn)
            .unwrap_or(1)
    }

    /// Check if the game is paused
    pub fn is_paused(&self) -> bool {
        self.world.get_resource::<GameTime>()
            .map(|game_time| game_time.paused)
            .unwrap_or(false)
    }

    /// Set paused state
    pub fn set_paused(&mut self, paused: bool) {
        if let Some(mut game_time) = self.world.get_resource_mut::<GameTime>() {
            game_time.paused = paused;
        }
    }

    /// Export world state for saving
    pub fn export_state(&mut self) -> crate::ecs::WorldState {
        use crate::ecs::world_state::WorldState;
        let mut state = WorldState::default();
        
        // Get game time
        if let Some(game_time) = self.world.get_resource::<GameTime>() {
            state.game_time = game_time.clone();
        }
        
        // Get players
        if let Some(players) = self.world.get_resource::<crate::ecs::resources::Players>() {
            state.players = players.clone();
        }

        // Get camera state (default for now)
        state.camera_position = (0.0, 0.0);
        state.camera_zoom = 1.0;

        // Serialize all entities with their components
        let mut entity_query = self.world.query::<Entity>();
        let entities: Vec<Entity> = entity_query.iter(&self.world).collect();
        
        for entity in entities {
            if let Some(serialized_entity) = crate::ecs::entity_serialization::serialize_entity(&self.world, entity) {
                state.entities.push(serialized_entity);
                state.entity_count += 1;
            }
        }
        
        // Export hierarchical relationships for compatibility
        use crate::ecs::hierarchy::{StableEntityId, Relationships, Hierarchical};
        use std::collections::HashMap;
        
        let hierarchical_entities: Vec<StableEntityId> = {
            let mut hierarchical_query = self.world.query_filtered::<Entity, With<Hierarchical>>();
            hierarchical_query.iter(&self.world)
                .filter_map(|entity| StableEntityId::from_entity_id(entity.index(), entity.generation()).into())
                .collect()
        };
        
        let entity_relationships: HashMap<StableEntityId, Relationships> = {
            let mut relationships_query = self.world.query::<(Entity, &Relationships)>();
            relationships_query.iter(&self.world)
                .filter_map(|(entity, relationships)| {
                    let stable_id = StableEntityId::from_entity_id(entity.index(), entity.generation());
                    Some((stable_id, relationships.clone()))
                })
                .collect()
        };
        
        state.hierarchical_entities = hierarchical_entities;
        state.entity_relationships = entity_relationships;

        state
    }

    /// Import world state from save
    pub fn import_state(&mut self, state: crate::ecs::WorldState) {
        // Update game time
        self.world.insert_resource(state.game_time);
        
        // Update players
        self.world.insert_resource(state.players);
        
        // Clear existing entities before importing new ones
        self.world.clear_entities();
        
        // Import all entities with their components
        use crate::ecs::entity_serialization::deserialize_entity;
        use crate::ecs::hierarchy::StableEntityId;
        use std::collections::HashMap;
        
        let mut stable_id_mapping: HashMap<StableEntityId, Entity> = HashMap::new();
        
        // First pass: Create all entities and store mapping
        for serialized_entity in &state.entities {
            let entity = deserialize_entity(&mut self.world, serialized_entity);
            stable_id_mapping.insert(serialized_entity.stable_id, entity);
        }
        
        // Second pass: Restore hierarchical relationships using the mapping
        // Note: This ensures all entity references in relationships are valid
        for (stable_id, relationships) in &state.entity_relationships {
            if let Some(&entity) = stable_id_mapping.get(stable_id) {
                // Update relationships to use current entity IDs
                let mut updated_relationships = relationships.clone();
                // The relationships component handles internal ID mapping during insertion
                if let Some(mut entity_ref) = self.world.get_entity_mut(entity) {
                    entity_ref.insert(updated_relationships);
                }
            }
        }
        
        // Set camera state (for UI restoration)
        // Note: In a full implementation, this would be handled by a camera system
        // For now, we just store the values for potential UI use
    }
}

impl Default for GameWorld {
    fn default() -> Self {
        Self::new()
    }
}
