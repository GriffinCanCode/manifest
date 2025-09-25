//! Movement cost system with bitset optimizations
//!
//! Provides movement cost calculations with fixedbitset for efficient
//! unit type restrictions and road network connectivity.

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use ordered_float::OrderedFloat;
use fixedbitset::FixedBitSet;

/// Movement costs with fixedbitset for efficient calculations
#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub struct MovementCosts {
    /// Base movement cost
    pub base_cost: OrderedFloat<f32>,
    /// Current modified cost
    pub current_cost: OrderedFloat<f32>,
    /// Bitset for movement restrictions by unit type
    pub restrictions: FixedBitSet,
    /// Road network connectivity
    pub road_connections: FixedBitSet,
    /// Weather-affected cost
    pub weather_modified_cost: OrderedFloat<f32>,
}

impl Default for MovementCosts {
    fn default() -> Self {
        Self {
            base_cost: OrderedFloat(1.0),
            current_cost: OrderedFloat(1.0),
            restrictions: FixedBitSet::with_capacity(32), // Up to 32 unit types
            road_connections: FixedBitSet::with_capacity(6), // 6 hex directions
            weather_modified_cost: OrderedFloat(1.0),
        }
    }
}

impl MovementCosts {
    /// Create new movement costs with base cost
    pub fn new(base_cost: f32) -> Self {
        Self {
            base_cost: OrderedFloat(base_cost),
            current_cost: OrderedFloat(base_cost),
            weather_modified_cost: OrderedFloat(base_cost),
            ..Default::default()
        }
    }

    /// Set restriction for unit type
    pub fn set_unit_restriction(&mut self, unit_type: UnitType, restricted: bool) {
        self.restrictions.set(unit_type as usize, restricted);
    }

    /// Check if unit type is restricted
    pub fn is_unit_restricted(&self, unit_type: UnitType) -> bool {
        self.restrictions[unit_type as usize]
    }

    /// Set road connection in direction
    pub fn set_road_connection(&mut self, direction: HexDirection, connected: bool) {
        self.road_connections.set(direction as usize, connected);
    }

    /// Check if road exists in direction
    pub fn has_road_connection(&self, direction: HexDirection) -> bool {
        self.road_connections[direction as usize]
    }

    /// Get number of road connections
    pub fn road_connection_count(&self) -> usize {
        self.road_connections.count_ones(..)
    }

    /// Apply terrain modifier to movement cost
    pub fn apply_terrain_modifier(&mut self, modifier: f32) {
        let new_cost = self.base_cost.into_inner() * modifier;
        self.current_cost = OrderedFloat(new_cost);
    }

    /// Apply weather effects to movement cost
    pub fn apply_weather_modifier(&mut self, weather_effect: WeatherEffect) {
        let modifier = weather_effect.movement_modifier();
        let new_cost = self.current_cost.into_inner() * modifier;
        self.weather_modified_cost = OrderedFloat(new_cost);
    }

    /// Get effective movement cost for unit type
    pub fn get_effective_cost(&self, unit_type: UnitType) -> f32 {
        if self.is_unit_restricted(unit_type) {
            return f32::INFINITY; // Impassable for this unit type
        }

        let mut cost = self.weather_modified_cost.into_inner();

        // Apply road bonus if unit can use roads
        if unit_type.can_use_roads() && self.road_connection_count() > 0 {
            cost *= 0.5; // 50% movement cost on roads
        }

        // Apply unit-specific terrain modifier
        cost *= unit_type.terrain_modifier(self.base_cost.into_inner());

        cost.max(0.1) // Minimum movement cost
    }

    /// Reset to base values
    pub fn reset(&mut self) {
        self.current_cost = self.base_cost;
        self.weather_modified_cost = self.base_cost;
        self.restrictions.clear();
    }

    /// Update all costs (called by ECS system)
    pub fn update_costs(&mut self, terrain_modifier: f32, weather_effect: WeatherEffect) {
        self.apply_terrain_modifier(terrain_modifier);
        self.apply_weather_modifier(weather_effect);
    }

    /// Check if tile is impassable for any unit
    pub fn is_impassable(&self) -> bool {
        self.base_cost.into_inner() >= 100.0
    }

    /// Get movement cost category
    pub fn cost_category(&self) -> MovementCategory {
        let cost = self.base_cost.into_inner();
        match cost {
            c if c <= 1.0 => MovementCategory::Easy,
            c if c <= 2.0 => MovementCategory::Normal,
            c if c <= 4.0 => MovementCategory::Difficult,
            c if c < 100.0 => MovementCategory::VeryDifficult,
            _ => MovementCategory::Impassable,
        }
    }
}

/// Unit types for movement calculations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(usize)]
pub enum UnitType {
    Infantry = 0,
    Cavalry = 1,
    Artillery = 2,
    Naval = 3,
    Air = 4,
    Mechanical = 5,
    // Add more unit types as needed up to 31 (0-31 for 32-bit restriction)
}

impl UnitType {
    /// Check if unit type can use roads
    pub fn can_use_roads(&self) -> bool {
        match self {
            Self::Infantry | Self::Cavalry | Self::Artillery | Self::Mechanical => true,
            Self::Naval | Self::Air => false,
        }
    }

    /// Get terrain movement modifier for this unit type
    pub fn terrain_modifier(&self, base_terrain_cost: f32) -> f32 {
        match self {
            Self::Infantry => {
                // Infantry is versatile but slower on difficult terrain
                if base_terrain_cost > 2.0 { 1.2 } else { 1.0 }
            },
            Self::Cavalry => {
                // Cavalry is fast on open terrain, slower on rough terrain
                if base_terrain_cost <= 1.5 { 0.8 } else { 1.5 }
            },
            Self::Artillery => {
                // Artillery is slow and struggles on rough terrain
                1.0 + (base_terrain_cost - 1.0) * 0.5
            },
            Self::Naval => {
                // Naval units can only move on water
                if base_terrain_cost <= 1.0 { 1.0 } else { f32::INFINITY }
            },
            Self::Air => {
                // Air units ignore most terrain
                0.5
            },
            Self::Mechanical => {
                // Mechanical units are fast on roads, struggle off-road
                if base_terrain_cost > 1.5 { 1.8 } else { 0.9 }
            },
        }
    }

    /// Get all unit types
    pub fn all_types() -> Vec<UnitType> {
        vec![
            UnitType::Infantry,
            UnitType::Cavalry,
            UnitType::Artillery,
            UnitType::Naval,
            UnitType::Air,
            UnitType::Mechanical,
        ]
    }
}

/// Hex directions for road connections
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(usize)]
pub enum HexDirection {
    North = 0,
    NorthEast = 1,
    SouthEast = 2,
    South = 3,
    SouthWest = 4,
    NorthWest = 5,
}

impl HexDirection {
    /// Get all hex directions
    pub fn all_directions() -> Vec<HexDirection> {
        vec![
            HexDirection::North,
            HexDirection::NorthEast,
            HexDirection::SouthEast,
            HexDirection::South,
            HexDirection::SouthWest,
            HexDirection::NorthWest,
        ]
    }

    /// Get opposite direction
    pub fn opposite(&self) -> HexDirection {
        match self {
            HexDirection::North => HexDirection::South,
            HexDirection::NorthEast => HexDirection::SouthWest,
            HexDirection::SouthEast => HexDirection::NorthWest,
            HexDirection::South => HexDirection::North,
            HexDirection::SouthWest => HexDirection::NorthEast,
            HexDirection::NorthWest => HexDirection::SouthEast,
        }
    }
}

/// Weather effects on movement
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WeatherEffect {
    Clear,
    Rain,
    Snow,
    Storm,
    Fog,
    Drought,
}

impl WeatherEffect {
    /// Get movement cost modifier for weather
    pub fn movement_modifier(&self) -> f32 {
        match self {
            Self::Clear => 1.0,
            Self::Rain => 1.3,
            Self::Snow => 1.5,
            Self::Storm => 2.0,
            Self::Fog => 1.2,
            Self::Drought => 1.1,
        }
    }

    /// Get visibility modifier for weather
    pub fn visibility_modifier(&self) -> f32 {
        match self {
            Self::Clear => 1.0,
            Self::Rain => 0.8,
            Self::Snow => 0.7,
            Self::Storm => 0.5,
            Self::Fog => 0.6,
            Self::Drought => 1.0,
        }
    }
}

/// Movement cost categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MovementCategory {
    Easy,
    Normal,
    Difficult,
    VeryDifficult,
    Impassable,
}

impl MovementCategory {
    /// Get category color for UI display
    pub fn color(&self) -> (u8, u8, u8) {
        match self {
            Self::Easy => (0, 255, 0),        // Green
            Self::Normal => (255, 255, 0),    // Yellow
            Self::Difficult => (255, 128, 0), // Orange
            Self::VeryDifficult => (255, 0, 0), // Red
            Self::Impassable => (128, 0, 128), // Purple
        }
    }

    /// Get category description
    pub fn description(&self) -> &'static str {
        match self {
            Self::Easy => "Easy movement",
            Self::Normal => "Normal movement",
            Self::Difficult => "Difficult movement",
            Self::VeryDifficult => "Very difficult movement",
            Self::Impassable => "Impassable terrain",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_movement_cost_creation() {
        let costs = MovementCosts::new(2.0);
        assert_eq!(costs.base_cost.into_inner(), 2.0);
        assert_eq!(costs.current_cost.into_inner(), 2.0);
        assert_eq!(costs.cost_category(), MovementCategory::Normal);
    }

    #[test]
    fn test_unit_restrictions() {
        let mut costs = MovementCosts::new(1.0);
        
        assert!(!costs.is_unit_restricted(UnitType::Infantry));
        costs.set_unit_restriction(UnitType::Infantry, true);
        assert!(costs.is_unit_restricted(UnitType::Infantry));
        assert!(!costs.is_unit_restricted(UnitType::Cavalry));
    }

    #[test]
    fn test_road_connections() {
        let mut costs = MovementCosts::new(2.0);
        
        assert_eq!(costs.road_connection_count(), 0);
        costs.set_road_connection(HexDirection::North, true);
        costs.set_road_connection(HexDirection::South, true);
        assert_eq!(costs.road_connection_count(), 2);
        assert!(costs.has_road_connection(HexDirection::North));
    }

    #[test]
    fn test_effective_cost_calculation() {
        let mut costs = MovementCosts::new(2.0);
        
        // Normal cost for infantry
        assert_eq!(costs.get_effective_cost(UnitType::Infantry), 2.0);
        
        // Restricted unit gets infinite cost
        costs.set_unit_restriction(UnitType::Naval, true);
        assert_eq!(costs.get_effective_cost(UnitType::Naval), f32::INFINITY);
        
        // Road reduces cost for ground units
        costs.set_road_connection(HexDirection::North, true);
        assert!(costs.get_effective_cost(UnitType::Infantry) < 2.0);
    }

    #[test]
    fn test_unit_type_modifiers() {
        assert!(UnitType::Infantry.can_use_roads());
        assert!(!UnitType::Naval.can_use_roads());
        assert!(!UnitType::Air.can_use_roads());
        
        // Cavalry should be faster on easy terrain
        assert!(UnitType::Cavalry.terrain_modifier(1.0) < 1.0);
        
        // Cavalry should be slower on difficult terrain  
        assert!(UnitType::Cavalry.terrain_modifier(3.0) > 1.0);
        
        // Air units ignore terrain
        assert_eq!(UnitType::Air.terrain_modifier(10.0), 0.5);
    }

    #[test]
    fn test_weather_effects() {
        let mut costs = MovementCosts::new(1.0);
        
        costs.apply_weather_modifier(WeatherEffect::Clear);
        assert_eq!(costs.weather_modified_cost.into_inner(), 1.0);
        
        costs.apply_weather_modifier(WeatherEffect::Snow);
        assert_eq!(costs.weather_modified_cost.into_inner(), 1.5);
        
        costs.apply_weather_modifier(WeatherEffect::Storm);
        assert_eq!(costs.weather_modified_cost.into_inner(), 2.0);
    }

    #[test]
    fn test_terrain_modifiers() {
        let mut costs = MovementCosts::new(1.0);
        costs.apply_terrain_modifier(2.0);
        assert_eq!(costs.current_cost.into_inner(), 2.0);
    }

    #[test]
    fn test_hex_directions() {
        assert_eq!(HexDirection::North.opposite(), HexDirection::South);
        assert_eq!(HexDirection::NorthEast.opposite(), HexDirection::SouthWest);
        assert_eq!(HexDirection::all_directions().len(), 6);
    }

    #[test]
    fn test_movement_categories() {
        assert_eq!(MovementCosts::new(0.5).cost_category(), MovementCategory::Easy);
        assert_eq!(MovementCosts::new(1.5).cost_category(), MovementCategory::Normal);
        assert_eq!(MovementCosts::new(3.0).cost_category(), MovementCategory::Difficult);
        assert_eq!(MovementCosts::new(100.0).cost_category(), MovementCategory::Impassable);
    }

    #[test]
    fn test_cost_reset() {
        let mut costs = MovementCosts::new(1.0);
        costs.apply_terrain_modifier(2.0);
        costs.apply_weather_modifier(WeatherEffect::Storm);
        costs.set_unit_restriction(UnitType::Infantry, true);
        
        costs.reset();
        
        assert_eq!(costs.current_cost, costs.base_cost);
        assert_eq!(costs.weather_modified_cost, costs.base_cost);
        assert!(!costs.is_unit_restricted(UnitType::Infantry));
    }
}
