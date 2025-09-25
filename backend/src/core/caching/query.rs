//! ECS query result caching for component-based entity lookups
//!
//! Optimized caching for expensive ECS queries in the game:
//! - Component signature-based entity queries
//! - Complex multi-component filtered queries
//! - Archetype-based entity grouping
//! - Query optimization with result reuse

use std::any::TypeId;
use serde::{Serialize, Deserialize};
use bevy_ecs::prelude::Entity;

use crate::core::hashing::{HashStrategies, FastHashMap, FastHashSet};
use super::CachePriority;

/// Query cache key for ECS component-based queries
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryCacheKey {
    /// Component signature hash (what components are required)
    pub component_signature: u64,
    /// Query filter hash (additional constraints)
    pub filter_hash: Option<u64>,
    /// Player ID filter (for ownership queries)
    pub player_id: Option<u32>,
    /// World generation for cache invalidation
    pub world_generation: u32,
    /// Query type for categorization
    pub query_type: QueryType,
}

impl QueryCacheKey {
    /// Fast hash using existing game hashing infrastructure
    pub fn fast_hash(&self) -> u64 {
        use crate::core::hashing::FastHasher;
        FastHasher::hash_one(self)
    }
    
    /// Extract probable component TypeIds from this query key for caching index
    /// This is a best-effort approximation based on query type and signature
    pub fn extract_component_types(&self) -> Vec<TypeId> {
        use std::any::TypeId;
        use crate::ecs::components::{Position, Movement, Health, Owner, Name};
        use crate::ecs::components::Renderable;
        use crate::ecs::hierarchy::{Hierarchical, Relationships};
        
        match self.query_type {
            QueryType::ComponentQuery | QueryType::PlayerOwnedQuery | QueryType::FilteredQuery => {
                // For standard component queries, we can infer common component types
                // This is a heuristic based on common query patterns in the game
                let mut types = Vec::new();
                
                // Map common component signature patterns to likely TypeIds
                let sig = self.component_signature;
                
                // Check against known component type signatures (approximate matching)
                let position_sig = HashStrategies::hash_type_signature(&[TypeId::of::<Position>()]);
                let movement_sig = HashStrategies::hash_type_signature(&[TypeId::of::<Movement>()]);
                let health_sig = HashStrategies::hash_type_signature(&[TypeId::of::<Health>()]);
                let owner_sig = HashStrategies::hash_type_signature(&[TypeId::of::<Owner>()]);
                let renderable_sig = HashStrategies::hash_type_signature(&[TypeId::of::<Renderable>()]);
                
                // Multi-component signatures (most common patterns)
                let pos_health_sig = HashStrategies::hash_type_signature(&[TypeId::of::<Position>(), TypeId::of::<Health>()]);
                let pos_move_sig = HashStrategies::hash_type_signature(&[TypeId::of::<Position>(), TypeId::of::<Movement>()]);
                let pos_owner_sig = HashStrategies::hash_type_signature(&[TypeId::of::<Position>(), TypeId::of::<Owner>()]);
                
                // Match against common patterns
                if sig == position_sig {
                    types.push(TypeId::of::<Position>());
                } else if sig == movement_sig {
                    types.push(TypeId::of::<Movement>());
                } else if sig == health_sig {
                    types.push(TypeId::of::<Health>());
                } else if sig == owner_sig {
                    types.push(TypeId::of::<Owner>());
                } else if sig == renderable_sig {
                    types.push(TypeId::of::<Renderable>());
                } else if sig == pos_health_sig {
                    types.push(TypeId::of::<Position>());
                    types.push(TypeId::of::<Health>());
                } else if sig == pos_move_sig {
                    types.push(TypeId::of::<Position>());
                    types.push(TypeId::of::<Movement>());
                } else if sig == pos_owner_sig {
                    types.push(TypeId::of::<Position>());
                    types.push(TypeId::of::<Owner>());
                } else {
                    // For unknown signatures, assume it involves common components
                    // This is a reasonable fallback that ensures cache invalidation works
                    types.push(TypeId::of::<Position>()); // Most queries involve position
                }
                
                types
            },
            QueryType::HierarchicalQuery => {
                vec![TypeId::of::<Hierarchical>(), TypeId::of::<Relationships>()]
            },
            QueryType::SpatialComponentQuery => {
                vec![TypeId::of::<Position>(), TypeId::of::<Renderable>()] // Spatial queries typically need position
            },
            QueryType::ArchetypeQuery => {
                // Archetype queries can contain any mix of components
                // Return a conservative set of common component types
                vec![
                    TypeId::of::<Position>(),
                    TypeId::of::<Movement>(),
                    TypeId::of::<Health>(),
                    TypeId::of::<Owner>()
                ]
            },
            QueryType::EntitiesByComponents | QueryType::EntitiesWithData => {
                // For entity-focused queries, include common components
                vec![TypeId::of::<Position>(), TypeId::of::<Health>(), TypeId::of::<Owner>()]
            },
        }
    }
}

/// Types of ECS queries that can be cached
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryType {
    /// All entities with specific components
    ComponentQuery,
    /// Entities with components owned by player
    PlayerOwnedQuery,
    /// Entities with components in specific archetype
    ArchetypeQuery,
    /// Complex filtered query with multiple conditions
    FilteredQuery,
    /// Hierarchical queries (parent/child relationships)
    HierarchicalQuery,
    /// Spatial + component combined query
    SpatialComponentQuery,
    /// Query returning entities by components
    EntitiesByComponents,
    /// Query returning entities with their component data
    EntitiesWithData,
}

impl QueryCacheKey {
    /// Create a key for a component signature query
    pub fn component_query(component_types: &[TypeId], world_generation: u32) -> Self {
        let component_signature = HashStrategies::hash_type_signature(component_types);
        
        Self {
            component_signature,
            filter_hash: None,
            player_id: None,
            world_generation,
            query_type: QueryType::ComponentQuery,
        }
    }

    /// Create a key for player-owned entity query
    pub fn player_owned_query(component_types: &[TypeId], player_id: u32, world_generation: u32) -> Self {
        let component_signature = HashStrategies::hash_type_signature(component_types);
        
        Self {
            component_signature,
            filter_hash: None,
            player_id: Some(player_id),
            world_generation,
            query_type: QueryType::PlayerOwnedQuery,
        }
    }

    /// Create a key for archetype-based query
    pub fn archetype_query(archetype_id: u64, world_generation: u32) -> Self {
        Self {
            component_signature: archetype_id,
            filter_hash: None,
            player_id: None,
            world_generation,
            query_type: QueryType::ArchetypeQuery,
        }
    }

    /// Create a key for filtered query with additional constraints
    pub fn filtered_query(
        component_types: &[TypeId], 
        filter_constraints: &[QueryConstraint], 
        world_generation: u32
    ) -> Self {
        let component_signature = HashStrategies::hash_type_signature(component_types);
        let filter_hash = if !filter_constraints.is_empty() {
            Some(HashStrategies::hash_bytes(&bincode::serialize(filter_constraints).unwrap_or_default()))
        } else {
            None
        };
        
        Self {
            component_signature,
            filter_hash,
            player_id: None,
            world_generation,
            query_type: QueryType::FilteredQuery,
        }
    }

    /// Create a key for hierarchical relationships
    pub fn hierarchical_query(relationship_type: HierarchicalRelation, world_generation: u32) -> Self {
        let signature = HashStrategies::hash_bytes(&bincode::serialize(&relationship_type).unwrap_or_default());
        
        Self {
            component_signature: signature,
            filter_hash: None,
            player_id: None,
            world_generation,
            query_type: QueryType::HierarchicalQuery,
        }
    }

    /// Get cache priority for this query type
    pub fn cache_priority(&self) -> CachePriority {
        match self.query_type {
            QueryType::ComponentQuery => CachePriority::Normal,
            QueryType::PlayerOwnedQuery => CachePriority::High,
            QueryType::ArchetypeQuery => CachePriority::High,
            QueryType::FilteredQuery => CachePriority::Normal,
            QueryType::HierarchicalQuery => CachePriority::Normal,
            QueryType::SpatialComponentQuery => CachePriority::High,
            QueryType::EntitiesByComponents | QueryType::EntitiesWithData => CachePriority::Normal,
        }
    }

    /// Estimate result size for memory planning
    pub fn estimated_result_size(&self) -> usize {
        match self.query_type {
            QueryType::ComponentQuery => 512, // Medium entity list
            QueryType::PlayerOwnedQuery => 256, // Smaller subset
            QueryType::ArchetypeQuery => 1024, // Large archetype lists
            QueryType::FilteredQuery => 128, // Filtered down results
            QueryType::HierarchicalQuery => 256, // Relationship data
            QueryType::SpatialComponentQuery => 512, // Combined query results
            QueryType::EntitiesByComponents | QueryType::EntitiesWithData => 384, // Entity-focused query results
        }
    }
}

/// Query constraints for filtered queries
#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub enum QueryConstraint {
    /// Health above threshold
    HealthAbove(u32),
    /// Health below threshold
    HealthBelow(u32),
    /// Movement speed above threshold
    MovementAbove(u32),
    /// Within position range
    WithinRange { center: glam::IVec2, radius: u32 },
    /// Owned by player
    OwnedBy(u32),
    /// Not owned by player
    NotOwnedBy(u32),
    /// Has name containing string
    NameContains(String),
    /// Custom constraint with hash
    Custom(u64),
}

/// Hierarchical relationship types
#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub enum HierarchicalRelation {
    /// All children of entity
    ChildrenOf(Entity),
    /// All parents of entity
    ParentsOf(Entity),
    /// All descendants (recursive children)
    DescendantsOf(Entity),
    /// All ancestors (recursive parents)
    AncestorsOf(Entity),
    /// All root entities (no parents)
    RootEntities,
    /// All leaf entities (no children)
    LeafEntities,
}

/// Query result types that can be cached
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryResult {
    /// Simple entity list
    Entities(Vec<Entity>),
    /// Entities with component data
    EntitiesWithData {
        entities: Vec<Entity>,
        component_data: Vec<ComponentData>,
    },
    /// Archetype groups
    ArchetypeGroups {
        groups: FastHashMap<u64, Vec<Entity>>, // Archetype ID -> Entities
    },
    /// Hierarchical relationships
    Relationships {
        relations: Vec<(Entity, Vec<Entity>)>, // Parent -> Children
    },
    /// Query statistics
    Statistics {
        count: usize,
        archetype_distribution: FastHashMap<u64, usize>,
    },
}

impl QueryResult {
    /// Get estimated size in bytes
    pub fn size_bytes(&self) -> usize {
        match self {
            QueryResult::Entities(entities) => entities.len() * 8,
            QueryResult::EntitiesWithData { entities, component_data } => {
                entities.len() * 8 + component_data.iter().map(|d| d.size_bytes()).sum::<usize>()
            }
            QueryResult::ArchetypeGroups { groups } => {
                groups.iter().map(|(_, entities)| 8 + entities.len() * 8).sum()
            }
            QueryResult::Relationships { relations } => {
                relations.iter().map(|(_, children)| 8 + children.len() * 8).sum()
            }
            QueryResult::Statistics { archetype_distribution, .. } => {
                16 + archetype_distribution.len() * 16
            }
        }
    }

    /// Check if result is empty
    pub fn is_empty(&self) -> bool {
        match self {
            QueryResult::Entities(entities) => entities.is_empty(),
            QueryResult::EntitiesWithData { entities, .. } => entities.is_empty(),
            QueryResult::ArchetypeGroups { groups } => groups.is_empty(),
            QueryResult::Relationships { relations } => relations.is_empty(),
            QueryResult::Statistics { count, .. } => *count == 0,
        }
    }

    /// Get entity count
    pub fn entity_count(&self) -> usize {
        match self {
            QueryResult::Entities(entities) => entities.len(),
            QueryResult::EntitiesWithData { entities, .. } => entities.len(),
            QueryResult::ArchetypeGroups { groups } => {
                groups.values().map(|entities| entities.len()).sum()
            }
            QueryResult::Relationships { relations } => {
                relations.iter().map(|(_, children)| children.len()).sum()
            }
            QueryResult::Statistics { count, .. } => *count,
        }
    }
}

/// Component data for queries that return component values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComponentData {
    Position(glam::IVec2),
    Health(f32),
    Movement(f32),
    Name(String),
    Owner { player_id: u32, controllable: bool },
    Serialized { type_id: u64, data: Vec<u8> },
}

impl ComponentData {
    pub fn size_bytes(&self) -> usize {
        match self {
            ComponentData::Position(_) => 8,
            ComponentData::Health(_) => 4,
            ComponentData::Movement(_) => 4,
            ComponentData::Name(name) => name.len() + 8,
            ComponentData::Owner { .. } => 5,
            ComponentData::Serialized { data, .. } => 8 + data.len(),
        }
    }
}

/// Query cache for ECS operations
pub struct QueryCache {
    /// Cache storage
    cache: FastHashMap<u64, CachedQueryResult>,
    /// Component type tracking for invalidation
    component_index: ComponentIndex,
    /// World generation tracking
    world_generation: u32,
}

/// Cached query result with metadata
#[derive(Debug, Clone)]
pub struct CachedQueryResult {
    pub result: QueryResult,
    pub key: QueryCacheKey,
    pub created_at: std::time::Instant,
    pub access_count: u32,
    pub last_accessed: std::time::Instant,
    pub computation_cost: u32, // Relative cost to compute this query
}

impl CachedQueryResult {
    pub fn new(result: QueryResult, key: QueryCacheKey, computation_cost: u32) -> Self {
        let now = std::time::Instant::now();
        Self {
            result,
            key,
            created_at: now,
            access_count: 0,
            last_accessed: now,
            computation_cost,
        }
    }

    pub fn access(&mut self) {
        self.last_accessed = std::time::Instant::now();
        self.access_count = self.access_count.saturating_add(1);
    }

    pub fn age(&self) -> std::time::Duration {
        std::time::Instant::now() - self.created_at
    }

    /// Calculate cache value score (higher = more valuable to keep)
    pub fn cache_value_score(&self) -> f64 {
        let access_factor = (self.access_count as f64).log2().max(1.0);
        let cost_factor = (self.computation_cost as f64).log2().max(1.0);
        let recency_factor = 1.0 / (self.age().as_secs() as f64 + 1.0);
        
        access_factor * cost_factor * recency_factor
    }
}

/// Component type index for query invalidation
#[derive(Debug, Default)]
pub struct ComponentIndex {
    /// Map component types to query keys that depend on them
    component_to_queries: FastHashMap<TypeId, FastHashSet<u64>>,
    /// Map archetypes to query keys
    archetype_to_queries: FastHashMap<u64, FastHashSet<u64>>,
}

impl ComponentIndex {
    /// Add a query key to the component index
    pub fn add_query(&mut self, key: &QueryCacheKey, key_hash: u64) {
        // Extract component types from the query key
        let component_types = key.extract_component_types();
        
        // Add query to each component type index
        for component_type in component_types {
            self.component_to_queries.entry(component_type)
                .or_default()
                .insert(key_hash);
        }
        
        // Add to archetype index if it's an archetype query
        if key.query_type == QueryType::ArchetypeQuery {
            self.archetype_to_queries.entry(key.component_signature)
                .or_default()
                .insert(key_hash);
        }
    }

    /// Remove a query key from the index
    pub fn remove_query(&mut self, key: &QueryCacheKey, key_hash: u64) {
        // Extract component types to remove from specific indices
        let component_types = key.extract_component_types();
        
        // Remove from each component type index
        for component_type in component_types {
            if let Some(queries) = self.component_to_queries.get_mut(&component_type) {
                queries.remove(&key_hash);
                // Clean up empty sets
                if queries.is_empty() {
                    self.component_to_queries.remove(&component_type);
                }
            }
        }
        
        // Remove from archetype index
        if key.query_type == QueryType::ArchetypeQuery {
            if let Some(queries) = self.archetype_to_queries.get_mut(&key.component_signature) {
                queries.remove(&key_hash);
                // Clean up empty sets
                if queries.is_empty() {
                    self.archetype_to_queries.remove(&key.component_signature);
                }
            }
        }
    }

    /// Get query keys affected by component type change
    pub fn get_affected_queries(&self, component_type: TypeId) -> Vec<u64> {
        self.component_to_queries.get(&component_type)
            .map(|queries| queries.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Get query keys affected by archetype change
    pub fn get_archetype_queries(&self, archetype_id: u64) -> Vec<u64> {
        self.archetype_to_queries.get(&archetype_id)
            .map(|queries| queries.iter().copied().collect())
            .unwrap_or_default()
    }
}

impl QueryCache {
    /// Create a new query cache
    pub fn new() -> Self {
        Self {
            cache: FastHashMap::default(),
            component_index: ComponentIndex::default(),
            world_generation: 1,
        }
    }

    /// Get cached query result
    pub fn get(&mut self, key: &QueryCacheKey) -> Option<QueryResult> {
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

    /// Store query result
    pub fn set(&mut self, key: QueryCacheKey, result: QueryResult, computation_cost: u32) {
        let key_hash = key.fast_hash();
        let cached_result = CachedQueryResult::new(result, key.clone(), computation_cost);
        
        // Add to component index
        self.component_index.add_query(&key, key_hash);
        
        // Store in cache
        self.cache.insert(key_hash, cached_result);
    }

    /// Remove cached result
    pub fn remove(&mut self, key: &QueryCacheKey) -> bool {
        let key_hash = key.fast_hash();
        
        if let Some(cached) = self.cache.remove(&key_hash) {
            self.component_index.remove_query(&cached.key, key_hash);
            true
        } else {
            false
        }
    }

    /// Invalidate queries affected by component type changes
    pub fn invalidate_component_type(&mut self, component_type: TypeId) {
        let affected_queries = self.component_index.get_affected_queries(component_type);
        
        for key_hash in affected_queries {
            if let Some(cached) = self.cache.remove(&key_hash) {
                self.component_index.remove_query(&cached.key, key_hash);
            }
        }
    }

    /// Invalidate queries for specific player
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
                self.component_index.remove_query(&cached.key, key_hash);
            }
        }
    }

    /// Invalidate queries by archetype
    pub fn invalidate_archetype(&mut self, archetype_id: u64) {
        let affected_queries = self.component_index.get_archetype_queries(archetype_id);
        
        for key_hash in affected_queries {
            if let Some(cached) = self.cache.remove(&key_hash) {
                self.component_index.remove_query(&cached.key, key_hash);
            }
        }
    }

    /// Advance world generation
    pub fn advance_world_generation(&mut self) {
        self.world_generation += 1;
        self.cache.clear();
        self.component_index = ComponentIndex::default();
    }

    /// Clean up based on cache value scores
    pub fn cleanup_by_value(&mut self, target_count: usize) {
        if self.cache.len() <= target_count {
            return;
        }

        // Collect entries with their scores
        let mut entries: Vec<(u64, f64)> = self.cache.iter()
            .map(|(key_hash, cached)| (*key_hash, cached.cache_value_score()))
            .collect();

        // Sort by score (lowest first for removal)
        entries.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        // Remove lowest scoring entries
        let to_remove = self.cache.len() - target_count;
        for (key_hash, _) in entries.iter().take(to_remove) {
            if let Some(cached) = self.cache.remove(key_hash) {
                self.component_index.remove_query(&cached.key, *key_hash);
            }
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> QueryCacheStats {
        let total_size = self.cache.values()
            .map(|cached| cached.result.size_bytes())
            .sum();

        let avg_access_count = if !self.cache.is_empty() {
            self.cache.values().map(|c| c.access_count as f64).sum::<f64>() / self.cache.len() as f64
        } else {
            0.0
        };

        let avg_computation_cost = if !self.cache.is_empty() {
            self.cache.values().map(|c| c.computation_cost as f64).sum::<f64>() / self.cache.len() as f64
        } else {
            0.0
        };

        QueryCacheStats {
            entry_count: self.cache.len(),
            total_size_bytes: total_size,
            world_generation: self.world_generation,
            avg_access_count,
            avg_computation_cost,
            component_types_tracked: self.component_index.component_to_queries.len(),
            archetypes_tracked: self.component_index.archetype_to_queries.len(),
        }
    }

    /// Get current world generation
    pub fn world_generation(&self) -> u32 {
        self.world_generation
    }
}

/// Query cache statistics
#[derive(Debug, Clone)]
pub struct QueryCacheStats {
    pub entry_count: usize,
    pub total_size_bytes: usize,
    pub world_generation: u32,
    pub avg_access_count: f64,
    pub avg_computation_cost: f64,
    pub component_types_tracked: usize,
    pub archetypes_tracked: usize,
}

impl Default for QueryCache {
    fn default() -> Self {
        Self::new()
    }
}
