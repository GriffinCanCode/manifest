//! Modifiers with modular-bitfield for compact tile property storage
//!
//! This module provides memory-efficient storage for tile modifiers using bitfield structures
//! with modular-bitfield for optimal packing and fast access.
//!
//! The large modifiers.rs file has been refactored into focused submodules:
//! - `bitfields`: Core and extended bitfield structures for efficient storage
//! - `types`: Type definitions, enums, and constants
//! - `instance`: Individual modifier instances with tracking
//! - `component`: TileModifiers ECS component for storing modifiers on tiles
//! - `stats`: Statistics, computed results, and error types
//! - `manager`: High-performance modifier management system and ECS systems

pub mod bitfields;
pub mod types;
pub mod instance;
pub mod component;
pub mod stats;
pub mod manager;

// Re-export commonly used types
pub use bitfields::{CoreModifiers, ExtendedModifiers, MAX_MODIFIER_STACKS};
pub use types::{
    ModifierType, ModifierSource, ModifierCategory, StackingMethod,
    SpecialFlag, EnvironmentalStatus, MAX_MODIFIER_TYPES,
};
pub use instance::{ModifierInstance, ModifierDisplayColor};
pub use component::{TileModifiers, ModifierSummary};
pub use stats::{
    ComputedModifiers, ModifierTurnResults, ModifierStats, 
    ModifierError, ErrorSeverity,
};
pub use manager::{
    TileModifierManager,
    process_modifiers_system,
    update_modifier_stats_system,
};

// Re-export debug-only systems
#[cfg(debug_assertions)]
pub use manager::validate_modifier_integrity_system;

// Re-export for compatibility with existing code
pub use bitfields::CoreModifiers as CoreTileModifiers;
pub use component::TileModifiers as TileModifierComponent;

#[cfg(test)]
mod integration_tests {
    use super::*;
    use bevy_ecs::prelude::*;

    #[test]
    fn test_complete_modifier_system() {
        let mut world = World::new();
        
        // Create a tile entity
        let tile_entity = world.spawn_empty().id();
        
        // Create modifier manager
        let tile_manager = std::sync::Arc::new(
            crate::world::tiles::components::TileComponentManager::new()
        );
        let mut modifier_manager = TileModifierManager::new(tile_manager);
        
        // Create a food modifier
        let food_modifier = ModifierInstance::new(
            ModifierType::Food,
            ModifierSource::Improvement,
            10 // +25% food
        );
        
        // Apply the modifier
        assert!(modifier_manager.apply_modifier(&mut world, tile_entity, food_modifier).is_ok());
        
        // Verify the modifier was applied
        let tile_modifiers = world.get::<TileModifiers>(tile_entity).expect("TileModifiers component should exist after applying modifier");
        assert_eq!(tile_modifiers.modifier_count(), 1);
        
        // Test computed values
        let mut temp_modifiers = tile_modifiers.clone();
        let computed = temp_modifiers.computed();
        assert!(computed.food_multiplier > 1.0);
        
        println!("✓ Complete modifier system integration test passed");
    }

    #[test]
    fn test_bitfield_efficiency() {
        // Test memory efficiency
        assert_eq!(std::mem::size_of::<CoreModifiers>(), 8); // 64 bits
        assert_eq!(std::mem::size_of::<ExtendedModifiers>(), 4); // 32 bits
        
        // Test that TileModifiers is reasonably sized
        assert!(std::mem::size_of::<TileModifiers>() < 1024); // Less than 1KB
        
        println!("✓ Bitfield efficiency test passed");
    }

    #[test]
    fn test_modifier_stacking_scenarios() {
        let mut tile_modifiers = TileModifiers::new();
        
        // Test additive stacking (food modifiers)
        let terrain_food = ModifierInstance::new(ModifierType::Food, ModifierSource::Terrain, 10);
        let improvement_food = ModifierInstance::new(ModifierType::Food, ModifierSource::Improvement, 12);
        
        tile_modifiers.add_modifier(terrain_food).expect("Should be able to add terrain food modifier");
        tile_modifiers.add_modifier(improvement_food).expect("Should be able to add improvement food modifier");
        
        let computed = tile_modifiers.computed();
        assert!(computed.food_multiplier > 1.5); // Should be significantly enhanced
        
        // Test maximum stacking (defense modifiers)
        let mut defense_modifiers = TileModifiers::new();
        let fort_defense = ModifierInstance::new(ModifierType::Defense, ModifierSource::Building, 12);
        let terrain_defense = ModifierInstance::new(ModifierType::Defense, ModifierSource::Terrain, 8);
        
        defense_modifiers.add_modifier(fort_defense).expect("Should be able to add fort defense modifier");
        defense_modifiers.add_modifier(terrain_defense).expect("Should be able to add terrain defense modifier");
        
        let defense_computed = defense_modifiers.computed();
        // Should use maximum value, not stack additively
        assert!(defense_computed.defense_bonus > 0.0);
        
        println!("✓ Modifier stacking scenarios test passed");
    }

    #[test]
    fn test_temporary_modifier_expiration() {
        let mut tile_modifiers = TileModifiers::new();
        
        // Add temporary modifier
        let temp_modifier = ModifierInstance::temporary(
            ModifierType::Gold,
            ModifierSource::Event,
            12,
            5 // 5 turns
        );
        
        tile_modifiers.add_modifier(temp_modifier).unwrap();
        assert_eq!(tile_modifiers.modifier_count(), 1);
        
        // Process 3 turns (should not expire)
        let expired = tile_modifiers.process_turn(3);
        assert_eq!(expired, 0);
        assert_eq!(tile_modifiers.modifier_count(), 1);
        
        // Process 6 turns (should expire)
        let expired = tile_modifiers.process_turn(6);
        assert_eq!(expired, 1);
        assert_eq!(tile_modifiers.modifier_count(), 0);
        
        println!("✓ Temporary modifier expiration test passed");
    }

    #[test]
    fn test_error_handling() {
        let mut tile_modifiers = TileModifiers::new();
        
        // Fill to capacity
        for i in 0..MAX_MODIFIER_TYPES {
            let modifier = ModifierInstance::new(
                ModifierType::Food,
                ModifierSource::Event,
                5
            );
            // Each needs unique source_id to avoid conflicts
            let mut unique_modifier = modifier;
            unique_modifier.source_id = Some(i as u32);
            
            assert!(tile_modifiers.add_modifier(unique_modifier).is_ok());
        }
        
        // Adding one more should fail
        let extra_modifier = ModifierInstance::new(ModifierType::Gold, ModifierSource::Technology, 8);
        assert!(matches!(
            tile_modifiers.add_modifier(extra_modifier),
            Err(ModifierError::TooManyModifiers)
        ));
        
        println!("✓ Error handling test passed");
    }
}
