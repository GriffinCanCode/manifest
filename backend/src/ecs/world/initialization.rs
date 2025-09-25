//! Game initialization and terrain generation
//!
//! Contains methods for initializing new games and generating initial world content.

use bevy_ecs::prelude::*;
use glam::IVec2;
use tracing::info;

use crate::ecs::{
    components::{Name, Position, Health, Movement, Owner, MovementType, Renderable},
    components::entities::{TileBundle, UnitBundle},
    resources::{Players}
};

use super::core::GameWorld;

impl GameWorld {
    /// Initialize a new game with default entities
    pub fn initialize_game(&mut self, player_name: String, civilization: String) {
        // Clear existing world state (keep resources)
        self.world_mut().clear_entities();

        // Update player data
        if let Some(mut players) = self.world_mut().get_resource_mut::<Players>() {
            if let Some(player_data) = players.data.get_mut(&1) {
                player_data.name = player_name;
                player_data.civilization = civilization;
            }
        }

        // Create initial terrain
        self.generate_initial_terrain();
        
        // Create starting units
        self.create_starting_units();
        
        info!("🎮 New game initialized successfully");
    }

    /// Generate initial terrain for a new game
    fn generate_initial_terrain(&mut self) {
        for q in -5i32..=5i32 {
            for r in -5i32..=5i32 {
                let s = -q - r;
                if s.abs() <= 5 {
                    let hex_pos = IVec2::new(q, r);
                    
                    // Create basic tile with all required components
                    let tile_bundle = TileBundle {
                        position: Position::from_hex(hex_pos)
                            .expect("Valid tile position"),
                        name: Name::new(format!("Tile ({}, {})", q, r))
                            .expect("Valid tile name"),
                        renderable: Renderable::new("terrain_grass".to_string()),
                        owner: Owner::neutral(),
                    };
                    
                    self.spawn_entity(tile_bundle);
                }
            }
        }
        
        info!("🗺️ Generated initial terrain (121 tiles)");
    }

    /// Create starting units for the player
    fn create_starting_units(&mut self) {
        // Create a settler at the center
        let settler = UnitBundle {
            name: Name::new("Settler".to_string())
                .expect("Valid settler name"),
            position: Position::from_hex(IVec2::ZERO)
                .expect("Valid settler position"),
            health: Health::new(100.0)
                .expect("Valid health value"),
            movement: Movement::new(2.0, 2, MovementType::Land)
                .expect("Valid movement component"),
            owner: Owner::player(1, true)
                .expect("Valid owner component"),
            renderable: crate::ecs::components::Renderable::new("settler".to_string()),
        };
        
        let settler_entity = self.spawn_entity(settler);
        
        // Create a warrior nearby
        let warrior = UnitBundle {
            name: Name::new("Warrior".to_string())
                .expect("Valid warrior name"),
            position: Position::from_hex(IVec2::new(1, 0))
                .expect("Valid warrior position"),
            health: Health::new(100.0)
                .expect("Valid health value"),
            movement: Movement::new(2.0, 2, MovementType::Land)
                .expect("Valid movement component"),
            owner: Owner::player(1, true)
                .expect("Valid owner component"),
            renderable: crate::ecs::components::Renderable::new("warrior".to_string()),
        };
        
        let warrior_entity = self.spawn_entity(warrior);
        
        info!("⚔️ Created starting units: Settler({:?}), Warrior({:?})", settler_entity, warrior_entity);
    }

    /// Load a predefined map configuration
    pub fn load_map_template(&mut self, template_name: &str) -> Result<(), String> {
        match template_name {
            "continents" => self.generate_continents_map(),
            "archipelago" => self.generate_archipelago_map(),
            "pangaea" => self.generate_pangaea_map(),
            "small_islands" => self.generate_small_islands_map(),
            _ => return Err(format!("Unknown map template: {}", template_name)),
        }
        
        Ok(())
    }

    /// Generate a continents-style map
    fn generate_continents_map(&mut self) {
        // Clear existing terrain
        self.clear_terrain();
        
        // Generate larger landmasses with water between them
        for q in -10i32..=10i32 {
            for r in -10i32..=10i32 {
                let s = -q - r;
                if s.abs() <= 10 {
                    let hex_pos = IVec2::new(q, r);
                    
                    // Create varied terrain based on position
                    let tile_type = self.determine_tile_type_continents(q, r);
                    
                        let tile_bundle = TileBundle {
                            name: Name::new(format!("{} ({}, {})", tile_type, q, r))
                                .expect("Valid tile name"),
                            position: Position::from_hex(hex_pos)
                                .expect("Valid tile position"),
                        };
                    
                    self.spawn_entity(tile_bundle);
                }
            }
        }
        
        info!("🌍 Generated continents map");
    }

    /// Generate an archipelago-style map
    fn generate_archipelago_map(&mut self) {
        self.clear_terrain();
        
        // Generate many small islands
        for q in -8i32..=8i32 {
            for r in -8i32..=8i32 {
                let s = -q - r;
                if s.abs() <= 8 {
                    let hex_pos = IVec2::new(q, r);
                    let tile_type = self.determine_tile_type_archipelago(q, r);
                    
                        let tile_bundle = TileBundle {
                            name: Name::new(format!("{} ({}, {})", tile_type, q, r))
                                .expect("Valid tile name"),
                            position: Position::from_hex(hex_pos)
                                .expect("Valid tile position"),
                        };
                    
                    self.spawn_entity(tile_bundle);
                }
            }
        }
        
        info!("🏝️ Generated archipelago map");
    }

    /// Generate a pangaea-style map (single large landmass)
    fn generate_pangaea_map(&mut self) {
        self.clear_terrain();
        
        for q in -12i32..=12i32 {
            for r in -12i32..=12i32 {
                let s = -q - r;
                if s.abs() <= 12 {
                    let hex_pos = IVec2::new(q, r);
                    let tile_type = self.determine_tile_type_pangaea(q, r);
                    
                        let tile_bundle = TileBundle {
                            name: Name::new(format!("{} ({}, {})", tile_type, q, r))
                                .expect("Valid tile name"),
                            position: Position::from_hex(hex_pos)
                                .expect("Valid tile position"),
                        };
                    
                    self.spawn_entity(tile_bundle);
                }
            }
        }
        
        info!("🗻 Generated pangaea map");
    }

    /// Generate small islands map
    fn generate_small_islands_map(&mut self) {
        self.clear_terrain();
        
        for q in -6i32..=6i32 {
            for r in -6i32..=6i32 {
                let s = -q - r;
                if s.abs() <= 6 {
                    let hex_pos = IVec2::new(q, r);
                    let tile_type = self.determine_tile_type_small_islands(q, r);
                    
                        let tile_bundle = TileBundle {
                            name: Name::new(format!("{} ({}, {})", tile_type, q, r))
                                .expect("Valid tile name"),
                            position: Position::from_hex(hex_pos)
                                .expect("Valid tile position"),
                        };
                    
                    self.spawn_entity(tile_bundle);
                }
            }
        }
        
        info!("🏖️ Generated small islands map");
    }

    /// Clear all terrain tiles
    fn clear_terrain(&mut self) {
        // Find all tile entities and despawn them
        let mut tile_query = self.world_mut().query_filtered::<Entity, With<Position>>();
        let tile_entities: Vec<Entity> = tile_query.iter(self.world()).collect();
        
        for entity in tile_entities {
            self.despawn_entity(entity);
        }
    }

    /// Determine tile type for continents map
    fn determine_tile_type_continents(&self, q: i32, r: i32) -> &'static str {
        let distance_from_center = ((q * q + r * r + (q + r) * (q + r)) as f32).sqrt();
        
        match distance_from_center as i32 {
            0..=3 => "Plains",
            4..=6 => "Forest", 
            7..=8 => "Hills",
            _ => "Ocean",
        }
    }

    /// Determine tile type for archipelago map
    fn determine_tile_type_archipelago(&self, q: i32, r: i32) -> &'static str {
        // Create clustered islands
        let island_centers = [(0, 0), (4, -2), (-3, 3), (2, -5), (-4, -1)];
        
        for (center_q, center_r) in island_centers.iter() {
            let dist = ((q - center_q).pow(2) + (r - center_r).pow(2)) as f32;
            if dist <= 4.0 {
                return match dist as i32 {
                    0..=1 => "Plains",
                    2..=3 => "Forest",
                    _ => "Coast",
                };
            }
        }
        
        "Ocean"
    }

    /// Determine tile type for pangaea map
    fn determine_tile_type_pangaea(&self, q: i32, r: i32) -> &'static str {
        let distance_from_center = ((q * q + r * r + (q + r) * (q + r)) as f32).sqrt();
        
        match distance_from_center as i32 {
            0..=2 => "Plains",
            3..=5 => "Forest",
            6..=8 => "Hills",
            9..=10 => "Mountains",
            11 => "Coast",
            _ => "Ocean",
        }
    }

    /// Determine tile type for small islands map
    fn determine_tile_type_small_islands(&self, q: i32, r: i32) -> &'static str {
        // Very small scattered islands
        let island_centers = [(0, 0), (3, -1), (-2, 2), (1, -3), (-3, 0), (2, 1)];
        
        for (center_q, center_r) in island_centers.iter() {
            let dist = ((q - center_q).pow(2) + (r - center_r).pow(2)) as f32;
            if dist <= 1.0 {
                return "Plains";
            } else if dist <= 2.0 {
                return "Coast";
            }
        }
        
        "Ocean"
    }
}
