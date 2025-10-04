//! ECS entity integration for Lua scripts
//!
//! Provides safe access to game entities and components from Lua,
//! with proper ownership and lifetime management.

use mlua::{Lua, Table, Value, UserData, UserDataMethods};
use bevy_ecs::prelude::*;
use parking_lot::RwLock;
use std::sync::Arc;
use crate::ecs::GameWorld;
use super::{ScriptResult, ScriptError};

/// Lua-safe entity wrapper
#[derive(Debug, Clone, Copy)]
pub struct LuaEntity {
    pub id: u64,
    generation: u32,
}

impl From<Entity> for LuaEntity {
    fn from(entity: Entity) -> Self {
        Self {
            id: entity.index() as u64,
            generation: entity.generation(),
        }
    }
}

impl Into<Entity> for LuaEntity {
    fn into(self) -> Entity {
        Entity::from_raw(self.id as u32)
    }
}

impl UserData for LuaEntity {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("id", |_, this, ()| {
            Ok(this.id)
        });

        methods.add_method("generation", |_, this, ()| {
            Ok(this.generation)
        });

        methods.add_method("is_valid", |_, this, ()| {
            // This would need world context to properly validate
            Ok(this.generation > 0)
        });
    }
}

/// ECS integration bridge for Lua scripts
pub struct LuaEcsIntegration {
    /// Shared reference to game world (read-only for safety)
    world_ref: Option<Arc<RwLock<GameWorld>>>,
}

impl LuaEcsIntegration {
    /// Create new ECS integration
    pub fn new() -> Self {
        Self {
            world_ref: None,
        }
    }

    /// Set world reference for ECS operations
    pub fn set_world_ref(&mut self, world: Arc<RwLock<GameWorld>>) {
        self.world_ref = Some(world);
    }

    /// Register ECS API functions with Lua
    pub fn register_ecs_api(&self, lua: &Lua) -> ScriptResult<()> {
        let globals = lua.globals();
        
        // Get or create Game API table
        let game_api: Table = match globals.get("Game") {
            Ok(table) => table,
            Err(_) => {
                let table = lua.create_table()
                    .map_err(|e| ScriptError::BindingFailed {
                        reason: format!("Failed to create Game table: {}", e)
                    })?;
                globals.set("Game", table.clone())
                    .map_err(|e| ScriptError::BindingFailed {
                        reason: format!("Failed to set Game table: {}", e)
                    })?;
                table
            }
        };

        // Create entities API
        let entities_api = lua.create_table()
            .map_err(|e| ScriptError::BindingFailed {
                reason: format!("Failed to create entities API: {}", e)
            })?;

        // Entity query functions (read-only for safety)
        let world_ref = self.world_ref.clone();
        let query_fn = lua.create_function(move |lua, query_type: String| {
            let results = lua.create_table()?;
            
            // If no world reference, return empty table
            let Some(world_ref) = world_ref.as_ref() else {
                return Ok(results);
            };
            
            // Query the world based on type (need write access for queries)
            match world_ref.try_write() {
                Some(mut world) => {
                    let entities = match query_type.as_str() {
                        "all" => {
                            // Get all entities with any components
                            let ecs_world = world.world_mut();
                            let mut query = ecs_world.query::<Entity>();
                            query.iter(ecs_world).take(1000).collect::<Vec<_>>() // Limit for safety
                        },
                        "hierarchical" => {
                            // Get entities with hierarchical components
                            use crate::ecs::hierarchy::components::Hierarchical;
                            let ecs_world = world.world_mut();
                            let mut query = ecs_world.query_filtered::<Entity, With<Hierarchical>>();
                            query.iter(ecs_world).take(1000).collect::<Vec<_>>()
                        },
                        "tiles" => {
                            // Get tile entities
                            use crate::world::tiles::components::Tile;
                            let ecs_world = world.world_mut();
                            let mut query = ecs_world.query_filtered::<Entity, With<Tile>>();
                            query.iter(ecs_world).take(1000).collect::<Vec<_>>()
                        },
                        _ => Vec::new(),
                    };
                    
                    // Convert entities to Lua table
                    for (index, entity) in entities.into_iter().enumerate() {
                        let lua_entity = LuaEntity::from(entity);
                        results.set(index + 1, lua_entity)?; // Lua arrays are 1-indexed
                    }
                },
                None => {
                    // World is locked, return empty result
                    tracing::warn!("Cannot access world from Lua - world is locked");
                }
            }
            
            Ok(results)
        }).map_err(|e| ScriptError::BindingFailed {
            reason: format!("Failed to create query function: {}", e)
        })?;

        entities_api.set("query", query_fn)
            .map_err(|e| ScriptError::BindingFailed {
                reason: format!("Failed to bind query function: {}", e)
            })?;

        game_api.set("Entities", entities_api)
            .map_err(|e| ScriptError::BindingFailed {
                reason: format!("Failed to bind entities API: {}", e)
            })?;

        Ok(())
    }

    /// Query entities with specific components (read-only)
    pub fn query_entities_with_components(&self, component_names: Vec<String>) -> ScriptResult<Vec<LuaEntity>> {
        let Some(world_ref) = self.world_ref.as_ref() else {
            return Ok(Vec::new());
        };
        
        match world_ref.try_write() {
            Some(mut world) => {
                let mut matching_entities = Vec::new();
                
                // For safety and simplicity, handle common component combinations
                match component_names.iter().map(|s| s.as_str()).collect::<Vec<_>>().as_slice() {
                    ["Tile"] => {
                        use crate::world::tiles::components::Tile;
                        let mut query = world.world_mut().query_filtered::<Entity, With<Tile>>();
                        matching_entities = query.iter(world.world())
                            .take(1000) // Limit for safety
                            .map(LuaEntity::from)
                            .collect();
                    },
                    ["Hierarchical"] => {
                        use crate::ecs::hierarchy::components::Hierarchical;
                        let mut query = world.world_mut().query_filtered::<Entity, With<Hierarchical>>();
                        matching_entities = query.iter(world.world())
                            .take(1000)
                            .map(LuaEntity::from)
                            .collect();
                    },
                    ["GameSelection"] => {
                        use crate::ecs::components::GameSelection;
                        let mut query = world.world_mut().query_filtered::<Entity, With<GameSelection>>();
                        matching_entities = query.iter(world.world())
                            .take(1000)
                            .map(LuaEntity::from)
                            .collect();
                    },
                    _ => {
                        // For complex or unknown combinations, return empty for safety
                        tracing::debug!("Unsupported component combination in Lua query: {:?}", component_names);
                    }
                }
                
                Ok(matching_entities)
            },
            None => {
                tracing::warn!("Cannot query entities from Lua - world is locked");
                Ok(Vec::new())
            }
        }
    }

    /// Get component data for an entity (read-only)
    pub fn get_entity_component(&self, entity: LuaEntity, component_name: String) -> ScriptResult<Value> {
        let Some(world_ref) = self.world_ref.as_ref() else {
            return Ok(Value::Nil);
        };
        
        match world_ref.try_write() {
            Some(world) => {
                let bevy_entity: Entity = entity.into();
                
                // For safety, only expose basic component data that's safe to access from Lua
                match component_name.as_str() {
                    "Tile" => {
                        use crate::world::tiles::components::Tile;
                        if let Some(tile) = world.world().get::<Tile>(bevy_entity) {
                            // Return basic tile info as a Lua-friendly format
                            let lua_value = format!("{{id={}, hex={{q={}, r={}}}, terrain={:?}}}", 
                                tile.id.0, tile.hex.q, tile.hex.r, tile.terrain_type);
                            // Return as a table with the tile data instead of a string
                            Ok(Value::Nil) // TODO: Fix this to return proper tile data
                        } else {
                            Ok(Value::Nil)
                        }
                    },
                    "id" => {
                        // Return entity ID
                        Ok(Value::Integer(entity.id as i64))
                    },
                    "generation" => {
                        // Return entity generation
                        Ok(Value::Integer(entity.generation as i64))
                    },
                    _ => {
                        tracing::debug!("Unsupported component access from Lua: {}", component_name);
                        Ok(Value::Nil)
                    }
                }
            },
            None => {
                tracing::warn!("Cannot access entity component from Lua - world is locked");
                Ok(Value::Nil)
            }
        }
    }

    /// Check if entity has component
    pub fn entity_has_component(&self, entity: LuaEntity, component_name: String) -> ScriptResult<bool> {
        let Some(world_ref) = self.world_ref.as_ref() else {
            return Ok(false);
        };
        
        match world_ref.try_write() {
            Some(world) => {
                let bevy_entity: Entity = entity.into();
                
                // Check for specific component types that are safe to expose
                let has_component = match component_name.as_str() {
                    "Tile" => {
                        use crate::world::tiles::components::Tile;
                        world.world().get::<Tile>(bevy_entity).is_some()
                    },
                    "Hierarchical" => {
                        use crate::ecs::hierarchy::components::Hierarchical;
                        world.world().get::<Hierarchical>(bevy_entity).is_some()
                    },
                    "GameSelection" => {
                        use crate::ecs::components::GameSelection;
                        world.world().get::<GameSelection>(bevy_entity).is_some()
                    },
                    "Relationships" => {
                        use crate::ecs::hierarchy::components::Relationships;
                        world.world().get::<Relationships>(bevy_entity).is_some()
                    },
                    _ => {
                        tracing::debug!("Unsupported component check from Lua: {}", component_name);
                        false
                    }
                };
                
                Ok(has_component)
            },
            None => {
                tracing::warn!("Cannot check entity component from Lua - world is locked");
                Ok(false)
            }
        }
    }
}

impl Default for LuaEcsIntegration {
    fn default() -> Self {
        Self::new()
    }
}

/// Component data wrapper for Lua
#[derive(Debug, Clone)]
pub struct LuaComponentData {
    pub component_type: String,
    pub data: Value<'static>,
}

impl UserData for LuaComponentData {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("get_type", |_, this, ()| {
            Ok(this.component_type.clone())
        });

        methods.add_method("get_data", |_, this, ()| {
            Ok(this.data.clone())
        });
    }
}

/// Safe query builder for Lua scripts
pub struct LuaQueryBuilder {
    with_components: Vec<String>,
    without_components: Vec<String>,
    filters: Vec<String>,
}

impl LuaQueryBuilder {
    pub fn new() -> Self {
        Self {
            with_components: Vec::new(),
            without_components: Vec::new(),
            filters: Vec::new(),
        }
    }

    pub fn with_component(mut self, component: String) -> Self {
        self.with_components.push(component);
        self
    }

    pub fn without_component(mut self, component: String) -> Self {
        self.without_components.push(component);
        self
    }

    pub fn with_filter(mut self, filter: String) -> Self {
        self.filters.push(filter);
        self
    }

    /// Execute the query against the ECS world
    pub fn execute(&self, world: &mut GameWorld) -> ScriptResult<Vec<LuaEntity>> {
        let mut matching_entities = Vec::new();
        
        // Handle different query combinations based on with_components
        match self.with_components.as_slice() {
            // Single component queries
            components if components == ["Tile"] => {
                use crate::world::tiles::components::Tile;
                let mut query = world.world_mut().query_filtered::<Entity, With<Tile>>();
                for entity in query.iter(world.world()) {
                    if self.entity_matches_without_components(world, entity)
                        && self.entity_matches_filters(world, entity) {
                        matching_entities.push(LuaEntity::from(entity));
                    }
                }
            },
            components if components == ["Hierarchical"] => {
                use crate::ecs::hierarchy::components::Hierarchical;
                let mut query = world.world_mut().query_filtered::<Entity, With<Hierarchical>>();
                for entity in query.iter(world.world()) {
                    if self.entity_matches_without_components(world, entity)
                        && self.entity_matches_filters(world, entity) {
                        matching_entities.push(LuaEntity::from(entity));
                    }
                }
            },
            components if components == ["Position"] => {
                use crate::ecs::components::Position;
                let mut query = world.world_mut().query_filtered::<Entity, With<Position>>();
                for entity in query.iter(world.world()) {
                    if self.entity_matches_without_components(world, entity)
                        && self.entity_matches_filters(world, entity) {
                        matching_entities.push(LuaEntity::from(entity));
                    }
                }
            },
            components if components == ["Health"] => {
                use crate::ecs::components::Health;
                let mut query = world.world_mut().query_filtered::<Entity, With<Health>>();
                for entity in query.iter(world.world()) {
                    if self.entity_matches_without_components(world, entity)
                        && self.entity_matches_filters(world, entity) {
                        matching_entities.push(LuaEntity::from(entity));
                    }
                }
            },
            // Empty query - return all entities (with safety limit)
            components if components.is_empty() => {
                let mut query = world.world_mut().query::<Entity>();
                for entity in query.iter(world.world()).take(500) { // Safety limit
                    if self.entity_matches_without_components(world, entity)
                        && self.entity_matches_filters(world, entity) {
                        matching_entities.push(LuaEntity::from(entity));
                    }
                }
            },
            // Fallback for unsupported component combinations
            _ => {
                tracing::debug!(
                    "Unsupported component combination in Lua query: {:?}", 
                    self.with_components
                );
            }
        }
        
        // Apply safety limit
        matching_entities.truncate(1000);
        Ok(matching_entities)
    }
    
    /// Check if entity matches without_components constraints
    fn entity_matches_without_components(&self, world: &GameWorld, entity: Entity) -> bool {
        for component_name in &self.without_components {
            match component_name.as_str() {
                "Tile" => {
                    use crate::world::tiles::components::Tile;
                    if world.world().get::<Tile>(entity).is_some() {
                        return false;
                    }
                },
                "Hierarchical" => {
                    use crate::ecs::hierarchy::components::Hierarchical;
                    if world.world().get::<Hierarchical>(entity).is_some() {
                        return false;
                    }
                },
                "Position" => {
                    use crate::ecs::components::Position;
                    if world.world().get::<Position>(entity).is_some() {
                        return false;
                    }
                },
                "Health" => {
                    use crate::ecs::components::Health;
                    if world.world().get::<Health>(entity).is_some() {
                        return false;
                    }
                },
                _ => {
                    tracing::debug!("Unknown component in without constraint: {}", component_name);
                }
            }
        }
        true
    }
    
    /// Check if entity matches filter constraints
    fn entity_matches_filters(&self, world: &GameWorld, entity: Entity) -> bool {
        for filter in &self.filters {
            if !self.evaluate_filter(world, entity, filter) {
                return false;
            }
        }
        true
    }
    
    /// Evaluate a specific filter constraint
    fn evaluate_filter(&self, world: &GameWorld, entity: Entity, filter: &str) -> bool {
        // Parse filter format: "component.field operator value"
        // Examples: "health.current > 50", "owner.player_id = 1", "position.q < 100"
        
        let parts: Vec<&str> = filter.split_whitespace().collect();
        if parts.len() != 3 {
            tracing::debug!("Invalid filter format: {}", filter);
            return true; // Don't fail query for malformed filters
        }
        
        let field_path = parts[0];
        let operator = parts[1];
        let value_str = parts[2];
        
        let field_parts: Vec<&str> = field_path.split('.').collect();
        if field_parts.len() != 2 {
            tracing::debug!("Invalid field path in filter: {}", field_path);
            return true;
        }
        
        let component_name = field_parts[0];
        let field_name = field_parts[1];
        
        match component_name {
            "health" => {
                use crate::ecs::components::Health;
                if let Some(health) = world.world().get::<Health>(entity) {
                    match field_name {
                        "current" => {
                            if let Ok(value) = value_str.parse::<f32>() {
                                return self.compare_f32(health.current, operator, value);
                            }
                        },
                        "max" => {
                            if let Ok(value) = value_str.parse::<f32>() {
                                return self.compare_f32(health.max, operator, value);
                            }
                        },
                        _ => {
                            tracing::debug!("Unknown health field: {}", field_name);
                        }
                    }
                }
            },
            "position" => {
                use crate::ecs::components::Position;
                if let Some(position) = world.world().get::<Position>(entity) {
                    match field_name {
                        "q" => {
                            if let Ok(value) = value_str.parse::<i32>() {
                                return self.compare_i32(position.q(), operator, value);
                            }
                        },
                        "r" => {
                            if let Ok(value) = value_str.parse::<i32>() {
                                return self.compare_i32(position.r(), operator, value);
                            }
                        },
                        _ => {
                            tracing::debug!("Unknown position field: {}", field_name);
                        }
                    }
                }
            },
            _ => {
                tracing::debug!("Unknown component in filter: {}", component_name);
            }
        }
        
        // Default to true if filter can't be evaluated
        true
    }
    
    /// Compare f32 values using string operator
    fn compare_f32(&self, left: f32, operator: &str, right: f32) -> bool {
        match operator {
            "=" | "==" => left == right,
            "!=" => left != right,
            ">" => left > right,
            ">=" => left >= right,
            "<" => left < right,
            "<=" => left <= right,
            _ => {
                tracing::debug!("Unknown operator: {}", operator);
                true
            }
        }
    }
    
    /// Compare i32 values using string operator
    fn compare_i32(&self, left: i32, operator: &str, right: i32) -> bool {
        match operator {
            "=" | "==" => left == right,
            "!=" => left != right,
            ">" => left > right,
            ">=" => left >= right,
            "<" => left < right,
            "<=" => left <= right,
            _ => {
                tracing::debug!("Unknown operator: {}", operator);
                true
            }
        }
    }
}

impl UserData for LuaQueryBuilder {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut("with", |_, this, component: String| {
            this.with_components.push(component);
            Ok(())
        });

        methods.add_method_mut("without", |_, this, component: String| {
            this.without_components.push(component);
            Ok(())
        });

        methods.add_method_mut("filter", |_, this, filter: String| {
            this.filters.push(filter);
            Ok(())
        });

        methods.add_method("execute", |_, this, ()| {
            // Note: For safety, we return empty results from Lua binding
            // The actual execute method with world parameter should be called from Rust code
            tracing::warn!("Query.execute() called from Lua without world context - returning empty results");
            Ok(Vec::<LuaEntity>::new())
        });
    }
}

/// Register query builder constructor with Lua
pub fn register_query_builder(lua: &Lua) -> ScriptResult<()> {
    let globals = lua.globals();
    
    let query_constructor = lua.create_function(|_, ()| {
        Ok(LuaQueryBuilder::new())
    }).map_err(|e| ScriptError::BindingFailed {
        reason: format!("Failed to create query constructor: {}", e)
    })?;

    // Add to Game API if it exists
    if let Ok(game_api) = globals.get::<_, Table>("Game") {
        game_api.set("Query", query_constructor)
            .map_err(|e| ScriptError::BindingFailed {
                reason: format!("Failed to bind Query constructor: {}", e)
            })?;
    } else {
        globals.set("Query", query_constructor)
            .map_err(|e| ScriptError::BindingFailed {
                reason: format!("Failed to bind Query constructor: {}", e)
            })?;
    }

    Ok(())
}
