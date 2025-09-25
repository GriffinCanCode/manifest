//! Core tile components and terrain types
//!
//! Contains the fundamental Tile struct and TerrainType enum that form the basis
//! of the tile system.

use hecs::Component;
use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use glam::{Vec2, Vec3};
use crate::core::zig_ffi::HexCoord;
use crate::world::tiles::chunks::{TileId, ChunkCoord};

/// Core tile component representing a single hex tile
#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub struct Tile {
    /// Unique tile identifier
    pub id: TileId,
    /// Hex coordinate in world space
    pub hex: HexCoord,
    /// Chunk this tile belongs to
    pub chunk: ChunkCoord,
    /// Local coordinates within chunk
    pub local_x: u8,
    pub local_y: u8,
    /// Tile type/terrain identifier
    pub terrain_type: TerrainType,
    /// Base elevation in game units
    pub elevation: f32,
}

impl Tile {
    /// Create new tile with specified parameters
    pub fn new(id: TileId, hex: HexCoord, chunk: ChunkCoord, local_x: u8, local_y: u8, terrain_type: TerrainType) -> Self {
        Self {
            id,
            hex,
            chunk,
            local_x,
            local_y,
            terrain_type,
            elevation: 0.0,
        }
    }

    /// Get world position as Vec2 for rendering
    pub fn world_position(&self) -> Vec2 {
        // Convert hex to pixel coordinates
        let hex_size = 1.0; // Base hex size
        let x = hex_size * (3.0 / 2.0 * self.hex.q as f32);
        let y = hex_size * ((3.0_f32).sqrt() / 2.0 * self.hex.q as f32 + (3.0_f32).sqrt() * self.hex.r as f32);
        Vec2::new(x, y)
    }

    /// Get 3D world position including elevation
    pub fn world_position_3d(&self) -> Vec3 {
        let pos_2d = self.world_position();
        Vec3::new(pos_2d.x, self.elevation, pos_2d.y)
    }
}

/// Terrain type enumeration for tiles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum TerrainType {
    Ocean = 0,
    Grassland = 1,
    Plains = 2,
    Desert = 3,
    Tundra = 4,
    Snow = 5,
    Forest = 6,
    Jungle = 7,
    Hills = 8,
    Mountain = 9,
    Mountains = 10, // Alias for Mountain for backward compatibility
    River = 11,
    Coast = 12,
    // Add more terrain types as needed
}

impl Default for TerrainType {
    fn default() -> Self {
        Self::Ocean
    }
}
