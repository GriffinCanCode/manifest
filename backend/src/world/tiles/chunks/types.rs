//! Core types for the chunk system
//!
//! Defines fundamental data types used throughout the chunk storage system.

use serde::{Deserialize, Serialize};
use bevy_ecs::prelude::Component;
use glam::IVec2;

/// Chunk size for tile storage (power of 2 for optimal memory alignment)
pub const CHUNK_SIZE: usize = 64;

/// World coordinate type for chunk addressing
pub type ChunkCoord = IVec2;

/// Tile identifier within a chunk
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component, Serialize, Deserialize)]
pub struct TileId(pub u32);

impl TileId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

impl std::fmt::Display for TileId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl num_traits::Zero for TileId {
    fn zero() -> Self {
        TileId(0)
    }
    
    fn is_zero(&self) -> bool {
        self.0 == 0
    }
}

impl std::ops::AddAssign<u32> for TileId {
    fn add_assign(&mut self, rhs: u32) {
        self.0 += rhs;
    }
}

impl std::ops::Add<u32> for TileId {
    type Output = TileId;

    fn add(self, rhs: u32) -> Self::Output {
        TileId(self.0 + rhs)
    }
}

impl std::ops::Add<TileId> for TileId {
    type Output = TileId;

    fn add(self, rhs: TileId) -> Self::Output {
        TileId(self.0 + rhs.0)
    }
}

/// Invalid tile constant
pub const INVALID_TILE: TileId = TileId(0);
