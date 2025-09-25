//! Entity creation and management utilities with strong validation
//!
//! This module provides factory functions for creating common entity types
//! with appropriate component bundles. All factories use validated components
//! and return Results for proper error handling.

use bevy_ecs::prelude::*;
use glam::IVec2;

use crate::ecs::components::*;

/// Bundle for basic movable game entities
#[derive(Bundle, Debug, Clone)]
pub struct MovableEntityBundle {
    pub position: Position,
    pub movement: Movement,
    pub renderable: Renderable,
    pub name: Name,
    pub owner: Owner,
}

/// Bundle for entities that can take damage
#[derive(Bundle, Debug, Clone)]
pub struct LivingEntityBundle {
    pub health: Health,
    pub position: Position,
    pub renderable: Renderable,
    pub name: Name,
    pub owner: Owner,
}

/// Bundle for complete game units (most common entity type)
#[derive(Bundle, Debug, Clone)]
pub struct UnitBundle {
    pub position: Position,
    pub movement: Movement,
    pub health: Health,
    pub renderable: Renderable,
    pub name: Name,
    pub owner: Owner,
}

/// Entity factory for creating common game entities with validation
#[derive(Debug, Clone, Copy, Default)]
pub struct EntityFactory;

impl EntityFactory {
    /// Create a basic terrain tile with validation
    pub fn create_terrain_tile(
        commands: &mut Commands,
        hex_pos: IVec2,
        terrain_type: &str,
    ) -> Result<Entity, ComponentError> {
        let entity = commands.spawn((
            Position::new_unchecked(hex_pos.x, hex_pos.y),
            Renderable::new(format!("terrain_{}", terrain_type), 0)?,
            Name::new(format!("{} Tile", terrain_type))?,
            Owner::neutral(),
        )).id();
        Ok(entity)
    }

    /// Create a military unit with validation
    pub fn create_unit(
        commands: &mut Commands,
        hex_pos: IVec2,
        unit_type: &str,
        player_id: u32,
        is_human: bool,
    ) -> Result<Entity, ComponentError> {
        let entity = commands.spawn(UnitBundle {
            position: Position::new_unchecked(hex_pos.x, hex_pos.y),
            movement: Movement::new(get_unit_movement(unit_type))?,
            health: Health::new(get_unit_health(unit_type))?,
            renderable: Renderable::new(format!("unit_{}", unit_type), 2)?,
            name: Name::new(unit_type.to_string())?,
            owner: Owner::player(player_id, is_human)?,
        }).id();
        Ok(entity)
    }

    /// Create a city with validation
    pub fn create_city(
        commands: &mut Commands,
        hex_pos: IVec2,
        city_name: &str,
        player_id: u32,
        is_human: bool,
    ) -> Result<Entity, ComponentError> {
        let entity = commands.spawn(LivingEntityBundle {
            position: Position::new_unchecked(hex_pos.x, hex_pos.y),
            health: Health::new(100.0)?, // Cities have more health
            renderable: Renderable::new("city".to_string(), 1)?,
            name: Name::new(city_name.to_string())?,
            owner: Owner::player(player_id, is_human)?,
        }).id();
        Ok(entity)
    }

    /// Create an improvement (roads, farms, mines, etc.) with validation
    pub fn create_improvement(
        commands: &mut Commands,
        hex_pos: IVec2,
        improvement_type: &str,
        player_id: u32,
        is_human: bool,
    ) -> Result<Entity, ComponentError> {
        let entity = commands.spawn((
            Position::new_unchecked(hex_pos.x, hex_pos.y),
            Renderable::new(format!("improvement_{}", improvement_type), 1)?,
            Name::new(format!("{} Improvement", improvement_type))?,
            Owner::player(player_id, is_human)?,
        )).id();
        Ok(entity)
    }

    /// Create a resource node (oil, gold, stone, etc.) with validation
    pub fn create_resource(
        commands: &mut Commands,
        hex_pos: IVec2,
        resource_type: &str,
    ) -> Result<Entity, ComponentError> {
        let entity = commands.spawn((
            Position::new_unchecked(hex_pos.x, hex_pos.y),
            Renderable::new(format!("resource_{}", resource_type), 1)?,
            Name::new(format!("{} Deposit", resource_type))?,
            Owner::neutral(),
        )).id();
        Ok(entity)
    }
}

/// Entity query utilities for common operations
#[derive(Debug, Clone, Copy, Default)]
pub struct EntityQueries;

impl EntityQueries {
    /// Find all entities at a specific hex position
    pub fn at_position(
        world: &mut World,
        hex_pos: IVec2,
    ) -> Vec<Entity> {
        let mut query_state = world.query::<(Entity, &Position)>();
        query_state
            .iter(world)
            .filter(|(_, position)| position.hex() == hex_pos)
            .map(|(entity, _)| entity)
            .collect()
    }

    /// Find all entities owned by a specific player
    pub fn owned_by_player(
        world: &mut World,
        player_id: u32,
    ) -> Vec<Entity> {
        let mut query_state = world.query::<(Entity, &Owner)>();
        query_state
            .iter(world)
            .filter(|(_, owner)| owner.player_id() == player_id)
            .map(|(entity, _)| entity)
            .collect()
    }

    /// Find all units (entities with movement component)
    pub fn all_units(world: &mut World) -> Vec<Entity> {
        let mut query_state = world.query::<(Entity, &Movement)>();
        query_state
            .iter(world)
            .map(|(entity, _)| entity)
            .collect()
    }

    /// Find all living entities (entities with health component)
    pub fn all_living(world: &mut World) -> Vec<Entity> {
        let mut query_state = world.query::<(Entity, &Health)>();
        query_state
            .iter(world)
            .map(|(entity, _)| entity)
            .collect()
    }

    /// Check if a hex position is occupied by any entity
    pub fn is_position_occupied(
        world: &mut World,
        hex_pos: IVec2,
        exclude_layer: Option<u8>,
    ) -> bool {
        let mut query_state = world.query::<(&Position, &Renderable)>();
        query_state.iter(world).any(|(position, renderable)| {
            position.hex() == hex_pos && 
            exclude_layer.map_or(true, |layer| renderable.layer() != layer)
        })
    }
}

/// Get movement points for different unit types
fn get_unit_movement(unit_type: &str) -> f32 {
    match unit_type.to_lowercase().as_str() {
        "scout" | "cavalry" => 3.0,
        "infantry" | "archer" => 2.0,
        "siege" => 1.0,
        "naval" => 4.0,
        _ => 2.0, // Default movement
    }
}

/// Get health points for different unit types
fn get_unit_health(unit_type: &str) -> f32 {
    match unit_type.to_lowercase().as_str() {
        "scout" => 20.0,
        "infantry" => 40.0,
        "archer" => 30.0,
        "cavalry" => 35.0,
        "siege" => 60.0,
        "naval" => 50.0,
        _ => 30.0, // Default health
    }
}
