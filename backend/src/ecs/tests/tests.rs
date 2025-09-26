//! Comprehensive tests for the ECS architecture
//!
//! Tests are designed to be fast, isolated, and comprehensive to ensure
//! the ECS system works correctly with current APIs.

#[cfg(test)]
mod tests {
    use crate::ecs::{
        components::*,
        resources::*,
        world::*,
    };
    use crate::ecs::components::core::MovementType;
    use bevy_ecs::prelude::*;
    use glam::IVec2;

    /// Test component creation and basic functionality
    #[test]
    fn test_component_creation() {
        // Test Position component
        let pos = Position::new_unchecked(3, -2);
        assert_eq!(pos.hex(), IVec2::new(3, -2));
        assert!(pos.pixel().length() > 0.0); // Should have calculated pixel position

        // Test Movement component
        let mut movement = Movement::new(5.0, 5, MovementType::Land).unwrap();
        assert_eq!(movement.remaining_moves, 5);
        assert_eq!(movement.max_moves, 5);
        assert!(movement.can_move(3));
        assert!(!movement.can_move(6));

        let _ = movement.use_moves(2).unwrap();
        assert_eq!(movement.remaining_moves, 3);

        movement.reset_for_turn();
        assert_eq!(movement.remaining_moves, 5);

        // Test Health component
        let mut health = Health::new(100.0).unwrap();
        assert!(health.is_alive());
        assert_eq!(health.percentage(), 1.0);

        let damage_dealt = health.take_damage(30.0).unwrap();
        assert_eq!(damage_dealt, 30.0);
        assert_eq!(health.current, 70.0);
        assert_eq!(health.percentage(), 0.7);

        let healed = health.heal(20.0).unwrap();
        assert_eq!(healed, 20.0);
        assert_eq!(health.current, 90.0);

        health.take_damage(100.0).unwrap();
        assert!(!health.is_alive());
    }

    /// Test resource creation and management
    #[test]
    fn test_resources() {
        // Test GameTime resource
        let mut game_time = GameTime::default();
        assert_eq!(game_time.turn, 1);
        assert_eq!(game_time.tick, 0);
        assert!(!game_time.paused);

        // Test manual time advancement
        game_time.advance_turn();
        assert_eq!(game_time.turn, 2);
        assert_eq!(game_time.tick, 0);

        // Test pause/unpause using set_paused (current API)
        game_time.set_paused(true);
        assert!(game_time.paused);

        game_time.set_paused(false);
        assert!(!game_time.paused);

        // Test Players resource
        let mut players = Players::default();
        assert_eq!(players.current_player, 1);
        assert_eq!(players.data.len(), 1);

        let new_player_id = players.add_player(
            "AI Player".to_string(),
            "Robot Empire".to_string(),
            false,
        );
        assert_eq!(new_player_id, 2);
        assert_eq!(players.data.len(), 2);

        let ai_player = players.get_player(2).unwrap();
        assert_eq!(ai_player.name, "AI Player");
        assert!(!ai_player.is_human);

        // Test Camera resource
        let mut camera = Camera::default();
        camera.set_target(glam::Vec2::new(10.0, 20.0));
        assert_eq!(camera.target, glam::Vec2::new(10.0, 20.0));

        camera.set_zoom(2.5);
        assert_eq!(camera.zoom, 2.5);

        camera.set_zoom(15.0); // Should be clamped
        assert_eq!(camera.zoom, 10.0);

        // Test Selection resource
        let mut selection = Selection::default();
        let entity1 = Entity::from_raw(1);
        let entity2 = Entity::from_raw(2);

        selection.add(entity1);
        assert!(selection.contains(entity1));
        assert_eq!(selection.primary, Some(entity1));

        selection.add(entity2);
        assert!(selection.contains(entity2));
        assert_eq!(selection.primary, Some(entity1)); // Should stay the same

        selection.remove(entity1);
        assert!(!selection.contains(entity1));
        assert_eq!(selection.primary, Some(entity2)); // Should update

        selection.clear();
        assert_eq!(selection.entities.len(), 0);
        assert_eq!(selection.primary, None);
    }

    /// Test entity creation through bundles
    #[test]
    fn test_entity_creation() {
        let mut world = World::new();
        
        // Test basic entity creation with corrected Renderable API
        let entity = world.spawn(UnitBundle {
            position: Position::new_unchecked(0, 0),
            movement: Movement::new(2.0, 2, MovementType::Land).unwrap(),
            health: Health::new(40.0).unwrap(),
            renderable: Renderable::new("unit_infantry".to_string()), // Fixed: only one parameter
            name: Name::new("infantry".to_string()).unwrap(),
            owner: Owner::new(1), // Fixed: use Owner::new() instead of Owner::player()
        }).id();

        // Verify entity was created with correct components
        assert!(world.get::<Position>(entity).is_some());
        assert!(world.get::<Movement>(entity).is_some());
        assert!(world.get::<Health>(entity).is_some());
        assert!(world.get::<Renderable>(entity).is_some());
        assert!(world.get::<Name>(entity).is_some());
        assert!(world.get::<Owner>(entity).is_some());

        let position = world.get::<Position>(entity).unwrap();
        assert_eq!(position.hex(), IVec2::new(0, 0));

        let owner = world.get::<Owner>(entity).unwrap();
        assert_eq!(owner.player_id, 1); // Fixed: direct field access instead of method

        // Test terrain creation with simplified bundle
        let terrain_entity = world.spawn(TileBundle {
            position: Position::new_unchecked(1, 1),
            renderable: Renderable::new("terrain_forest".to_string()),
            name: Name::new("Forest Tile".to_string()).unwrap(),
            owner: Owner::new(0), // Neutral owner
        }).id();

        let terrain_renderable = world.get::<Renderable>(terrain_entity).unwrap();
        assert_eq!(terrain_renderable.sprite, "terrain_forest"); // Fixed: direct field access
        assert_eq!(terrain_renderable.layer, 0); // Fixed: direct field access
    }

    /// Test game world initialization and basic operations
    #[test]
    fn test_game_world() {
        let mut game_world = GameWorld::new();

        // Test initial state
        assert_eq!(game_world.get_turn(), 1);
        assert!(!game_world.is_paused());

        // Test game initialization
        game_world.initialize_game(
            "Test Player".to_string(),
            "Test Civilization".to_string(),
        );

        // Test pause/unpause
        game_world.set_paused(true);
        assert!(game_world.is_paused());

        game_world.set_paused(false);
        assert!(!game_world.is_paused());

        // Basic functionality test - verify world has been initialized
        assert!(game_world.world().entities().len() > 0);
    }

    /// Test system execution and world updates
    #[test]
    fn test_systems() {
        let mut game_world = GameWorld::new();
        game_world.initialize_game(
            "Test Player".to_string(), 
            "Test Civilization".to_string()
        );

        let initial_turn = game_world.get_turn();
        
        // Update world and verify it doesn't crash
        game_world.update();
        
        // Test fixed timestep update
        game_world.update_fixed(1.0 / 60.0);
        
        // Verify world is still in a valid state
        assert!(game_world.get_turn() >= initial_turn);
        assert!(!game_world.is_paused());
    }

    /// Test change detection and system reactivity
    #[test]
    fn test_change_detection() {
        let mut world = World::new();

        // Create an entity with health
        let entity = world.spawn((
            Health::new(100.0).unwrap(),
            Name::new("Test Entity".to_string()).unwrap(), // Fixed: pass String instead of &str
        )).id();

        // Create a simple system that tracks health changes using Changed filter
        let mut changed_entities = Vec::new();
        
        {
            let mut query = world.query_filtered::<(Entity, &Health), Changed<Health>>();
            for (entity, _) in query.iter(&world) {
                changed_entities.push(entity);
            }
        }

        // Initially, the entity should be detected as changed (just created)
        assert_eq!(changed_entities.len(), 1);

        // Modify health using correct method name
        if let Some(mut health) = world.get_mut::<Health>(entity) {
            let _ = health.take_damage(10.0); // Fixed: use take_damage() instead of damage()
        }

        // After clearing the world's change tracking and making a new change
        world.clear_trackers(); // Clear the change tracking
        
        // Modify health again
        if let Some(mut health) = world.get_mut::<Health>(entity) {
            let _ = health.take_damage(5.0); // Fixed: use take_damage() instead of damage()
        }
        
        // Now changes should be detected again
        changed_entities.clear();
        {
            let mut query = world.query_filtered::<(Entity, &Health), Changed<Health>>();
            for (entity, _) in query.iter(&world) {
                changed_entities.push(entity);
            }
        }

        assert_eq!(changed_entities.len(), 1);
        assert_eq!(changed_entities[0], entity);
    }

    /// Performance test for large numbers of entities
    #[test]
    fn test_performance() {
        let mut world = World::new();

        // Create 100 entities (reduced for faster test)
        for i in 0..100 {
            world.spawn(UnitBundle {
                position: Position::new_unchecked(i % 10, i / 10),
                movement: Movement::new(2.0, 2, MovementType::Land).unwrap(),
                health: Health::new(40.0).unwrap(),
                renderable: Renderable::new("unit_infantry".to_string()), // Fixed: one parameter
                name: Name::new("infantry".to_string()).unwrap(),
                owner: Owner::new(1), // Fixed: use Owner::new()
            });
        }

        // Measure query performance
        let start = std::time::Instant::now();
        
        let count = world.query::<(&Position, &Movement, &Health)>()
            .iter(&world)
            .count();
            
        let duration = start.elapsed();

        assert_eq!(count, 100);
        assert!(duration.as_millis() < 100, "Query took too long: {:?}", duration);
    }
}