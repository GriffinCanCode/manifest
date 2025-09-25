//! River and water flow components
//!
//! Contains components for managing river systems, water flow directions,
//! and related water mechanics using efficient bitfield storage.

use bevy_ecs::prelude::Component;
use serde::{Deserialize, Serialize};
use modular_bitfield::prelude::*;

/// River flow directions bitfield for compact storage
#[bitfield(bits = 8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RiverFlowDirections {
    /// East direction flow
    east: bool,
    /// Northeast direction flow
    northeast: bool,
    /// Northwest direction flow
    northwest: bool,
    /// West direction flow
    west: bool,
    /// Southwest direction flow
    southwest: bool,
    /// Southeast direction flow
    southeast: bool,
    /// Reserved for future use
    #[bits = 2]
    reserved: B2,
}

impl Default for RiverFlowDirections {
    fn default() -> Self {
        Self::new()
    }
}

impl RiverFlowDirections {
    /// Set flow direction using HexDirection enum
    pub fn set_direction(&mut self, direction: crate::world::tiles::adjacency::HexDirection, flowing: bool) {
        use crate::world::tiles::adjacency::HexDirection;
        match direction {
            HexDirection::East => self.set_east(flowing),
            HexDirection::Northeast => self.set_northeast(flowing),
            HexDirection::Northwest => self.set_northwest(flowing),
            HexDirection::West => self.set_west(flowing),
            HexDirection::Southwest => self.set_southwest(flowing),
            HexDirection::Southeast => self.set_southeast(flowing),
        }
    }
    
    /// Check if flowing in specific direction
    pub fn is_flowing(&self, direction: crate::world::tiles::adjacency::HexDirection) -> bool {
        use crate::world::tiles::adjacency::HexDirection;
        match direction {
            HexDirection::East => self.east(),
            HexDirection::Northeast => self.northeast(),
            HexDirection::Northwest => self.northwest(),
            HexDirection::West => self.west(),
            HexDirection::Southwest => self.southwest(),
            HexDirection::Southeast => self.southeast(),
        }
    }
    
    /// Get all flowing directions
    pub fn get_flowing_directions(&self) -> Vec<crate::world::tiles::adjacency::HexDirection> {
        use crate::world::tiles::adjacency::HexDirection;
        let mut directions = Vec::new();
        if self.east() { directions.push(HexDirection::East); }
        if self.northeast() { directions.push(HexDirection::Northeast); }
        if self.northwest() { directions.push(HexDirection::Northwest); }
        if self.west() { directions.push(HexDirection::West); }
        if self.southwest() { directions.push(HexDirection::Southwest); }
        if self.southeast() { directions.push(HexDirection::Southeast); }
        directions
    }
}

/// River component for tiles with water flow
#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub struct River {
    /// River strength/flow rate (0-255)
    pub flow_rate: u8,
    /// Directions where river flows (using bitfield)
    pub flow_directions: RiverFlowDirections,
    /// Whether this is a river source
    pub is_source: bool,
    /// River system ID for connected waterways
    pub river_system_id: u32,
}
