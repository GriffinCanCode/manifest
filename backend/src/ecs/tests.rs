//! Comprehensive tests for the ECS architecture
//!
//! Tests are designed to be fast, isolated, and comprehensive to ensure
//! the ECS system works correctly and maintains backward compatibility.

#[cfg(test)]
mod tests {
    use crate::ecs::{
        components::*,
        entities::*,
        resources::*,
        world::*,
    };
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
        let mut movement = Movement::new(5.0).unwrap();
        assert_eq!(movement.points(), 5.0);
        assert_eq!(movement.max_points(), 5.0);
        assert!(movement.can_move(3.0));
        assert!(!movement.can_move(6.0));

        let consumed = movement.consume(2.0).unwrap();
        assert_eq!(consumed, 2.0);
        assert_eq!(movement.points(), 3.0);

        movement.restore();
        assert_eq!(movement.points(), 5.0);

        // Test Health component
        let mut health = Health::new(100.0).unwrap();
        assert!(health.is_alive());
        assert_eq!(health.percentage(), 1.0);

        let damage_dealt = health.damage(30.0).unwrap();
        assert_eq!(damage_dealt, 30.0);
        assert_eq!(health.current(), 70.0);
        assert_eq!(health.percentage(), 0.7);

        let healed = health.heal(20.0).unwrap();
        assert_eq!(healed, 20.0);
        assert_eq!(health.current(), 90.0);

        health.damage(100.0).unwrap();
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

        game_time.update(1.0/60.0);
        assert_eq!(game_time.tick, 1);

        game_time.advance_turn();
        assert_eq!(game_time.turn, 2);
        assert_eq!(game_time.tick, 0);

        game_time.toggle_pause();
        assert!(game_time.paused);

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

    /// Test entity creation through bundles and factory
    #[test]
    fn test_entity_creation() {
        let mut world = World::new();
        
        // Test basic entity creation - using world.spawn directly for simplicity
        let entity = world.spawn(UnitBundle {
            position: Position::new_unchecked(0, 0),
            movement: Movement::new(2.0).unwrap(), // infantry movement
            health: Health::new(40.0).unwrap(), // infantry health
            renderable: Renderable::new("unit_infantry".to_string(), 2).unwrap(),
            name: Name::new("infantry".to_string()).unwrap(),
            owner: Owner::player(1, true).unwrap(),
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
        assert_eq!(owner.player_id(), 1);
        assert!(owner.is_human());

        // Test terrain creation
        let terrain_entity = world.spawn((
            Position::new_unchecked(1, 1),
            Renderable::new("terrain_forest".to_string(), 0).unwrap(),
            Name::new("Forest Tile".to_string()).unwrap(),
            Owner::neutral(),
        )).id();

        let terrain_renderable = world.get::<Renderable>(terrain_entity).unwrap();
        assert_eq!(terrain_renderable.sprite(), "terrain_forest");
        assert_eq!(terrain_renderable.layer(), 0);
    }

    /// Test entity queries and utility functions
    #[test]
    fn test_entity_queries() {
        let mut world = World::new();

        // Create some test entities directly
        let _unit1 = world.spawn(UnitBundle {
            position: Position::new_unchecked(0, 0),
            movement: Movement::new(2.0).unwrap(),
            health: Health::new(40.0).unwrap(),
            renderable: Renderable::new("unit_infantry".to_string(), 2).unwrap(),
            name: Name::new("infantry".to_string()).unwrap(),
            owner: Owner::player(1, true).unwrap(),
        }).id();

        let _unit2 = world.spawn(UnitBundle {
            position: Position::new_unchecked(1, 0),
            movement: Movement::new(3.0).unwrap(),
            health: Health::new(35.0).unwrap(),
            renderable: Renderable::new("unit_cavalry".to_string(), 2).unwrap(),
            name: Name::new("cavalry".to_string()).unwrap(),
            owner: Owner::player(2, false).unwrap(),
        }).id();

        let _city = world.spawn(LivingEntityBundle {
            position: Position::new_unchecked(0, 0),
            health: Health::new(100.0).unwrap(),
            renderable: Renderable::new("city".to_string(), 1).unwrap(),
            name: Name::new("Test City".to_string()).unwrap(),
            owner: Owner::player(1, true).unwrap(),
        }).id();

        // Test position queries
        let entities_at_origin = EntityQueries::at_position(&mut world, IVec2::new(0, 0));
        assert_eq!(entities_at_origin.len(), 2); // unit1 and city

        let entities_at_1_0 = EntityQueries::at_position(&mut world, IVec2::new(1, 0));
        assert_eq!(entities_at_1_0.len(), 1); // unit2

        // Test owner queries
        let player1_entities = EntityQueries::owned_by_player(&mut world, 1);
        assert_eq!(player1_entities.len(), 2); // unit1 and city

        let player2_entities = EntityQueries::owned_by_player(&mut world, 2);
        assert_eq!(player2_entities.len(), 1); // unit2

        // Test unit queries
        let all_units = EntityQueries::all_units(&mut world);
        assert_eq!(all_units.len(), 2); // unit1 and unit2

        // Test living entity queries
        let all_living = EntityQueries::all_living(&mut world);
        assert_eq!(all_living.len(), 3); // unit1, unit2, and city

        // Test position occupation
        assert!(EntityQueries::is_position_occupied(&mut world, IVec2::new(0, 0), None));
        assert!(!EntityQueries::is_position_occupied(&mut world, IVec2::new(5, 5), None));
    }

    /// Test game world initialization and basic operations
    #[test]
    fn test_game_world() {
        let mut game_world = GameWorld::new();

        // Test initial state
        assert_eq!(game_world.get_turn(), 1);
        assert!(!game_world.is_paused());
        assert_eq!(game_world.get_current_player(), 1);

        // Test game initialization
        game_world.initialize_game(
            "Test Player".to_string(),
            "Test Civilization".to_string(),
        );

        let stats = game_world.get_entity_stats();
        assert!(stats.total > 0);
        assert_eq!(stats.cities, 1);
        assert_eq!(stats.units, 1);

        // Test pause/unpause
        game_world.set_paused(true);
        assert!(game_world.is_paused());

        game_world.set_paused(false);
        assert!(!game_world.is_paused());

        // Test world state export/import
        let exported_state = game_world.export_state();
        assert_eq!(exported_state.game_time.turn, 1);
        assert_eq!(exported_state.entity_count, stats.total);

        // Modify world state and import
        let mut modified_state = exported_state.clone();
        modified_state.game_time.turn = 5;
        
        game_world.import_state(modified_state);
        assert_eq!(game_world.get_turn(), 5);
    }

    /// Test system execution and world updates
    #[test]
    fn test_systems() {
        let mut game_world = GameWorld::new();
        game_world.initialize_game(
            "Test Player".to_string(), 
            "Test Civilization".to_string()
        );

        let _initial_turn = game_world.get_turn();
        let initial_tick = game_world.world()
            .get_resource::<GameTime>()
            .unwrap()
            .tick;

        // Update world and verify time advancement
        game_world.update();

        let updated_tick = {
            let updated_time = game_world.world()
                .get_resource::<GameTime>()
                .unwrap();
            updated_time.tick
        };

        assert!(updated_tick > initial_tick);

        // Test fixed timestep update
        game_world.update_fixed(1.0 / 60.0);
        
        let final_tick = {
            let final_time = game_world.world()
                .get_resource::<GameTime>()
                .unwrap();
            final_time.tick
        };

        assert!(final_tick > updated_tick);
    }

    /// Test change detection and system reactivity
    #[test]
    fn test_change_detection() {
        let mut world = World::new();

        // Create an entity with health
        let entity = world.spawn((
            Health::new(100.0).unwrap(),
            Name::new("Test Entity").unwrap(),
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

        // Modify health
        if let Some(mut health) = world.get_mut::<Health>(entity) {
            let _ = health.damage(10.0);
        }

        // After clearing the world's change tracking and making a new change
        world.clear_trackers(); // Clear the change tracking
        
        // Modify health again
        if let Some(mut health) = world.get_mut::<Health>(entity) {
            let _ = health.damage(5.0);
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

        // Create 1000 entities
        for i in 0..1000 {
            world.spawn(UnitBundle {
                position: Position::new_unchecked(i % 50, i / 50),
                movement: Movement::new(2.0).unwrap(),
                health: Health::new(40.0).unwrap(),
                renderable: Renderable::new("unit_infantry".to_string(), 2).unwrap(),
                name: Name::new("infantry".to_string()).unwrap(),
                owner: Owner::player(1, true).unwrap(),
            });
        }

        // Measure query performance
        let start = std::time::Instant::now();
        
        let count = world.query::<(&Position, &Movement, &Health)>()
            .iter(&world)
            .count();
            
        let duration = start.elapsed();

        assert_eq!(count, 1000);
        assert!(duration.as_millis() < 100, "Query took too long: {:?}", duration);
    }
}
