//! Core types for edge detection system

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::world::tiles::{
    chunks::TileId,
    adjacency::HexDirection
};

/// Types of edges that can be detected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum EdgeType {
    /// Terrain boundary (forest to grassland, etc.)
    TerrainBoundary = 0,
    /// Elevation change (cliff, slope)
    ElevationChange = 1,
    /// Political border (nation, province)
    PoliticalBorder = 2,
    /// Cultural boundary (different cultures)
    CulturalBoundary = 3,
    /// Climate zone transition
    ClimateTransition = 4,
    /// Resource deposit edge
    ResourceBoundary = 5,
    /// River bank
    Riverbank = 6,
    /// Coastline (land to water)
    Coastline = 7,
}

impl EdgeType {
    /// Get all edge types
    pub const ALL: [EdgeType; 8] = [
        EdgeType::TerrainBoundary,
        EdgeType::ElevationChange,
        EdgeType::PoliticalBorder,
        EdgeType::CulturalBoundary,
        EdgeType::ClimateTransition,
        EdgeType::ResourceBoundary,
        EdgeType::Riverbank,
        EdgeType::Coastline,
    ];

    /// Get edge strength threshold (0.0 to 1.0)
    pub fn strength_threshold(self) -> f32 {
        match self {
            EdgeType::TerrainBoundary => 0.3,
            EdgeType::ElevationChange => 0.4,
            EdgeType::PoliticalBorder => 0.2,
            EdgeType::CulturalBoundary => 0.25,
            EdgeType::ClimateTransition => 0.35,
            EdgeType::ResourceBoundary => 0.3,
            EdgeType::Riverbank => 0.5,
            EdgeType::Coastline => 0.6,
        }
    }
}

/// Detected edge between two tiles
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TileEdge {
    /// Source tile ID
    pub from_tile: TileId,
    /// Target tile ID
    pub to_tile: TileId,
    /// Direction of edge from source
    pub direction: HexDirection,
    /// Type of edge detected
    pub edge_type: EdgeType,
    /// Strength of edge (0.0 = no edge, 1.0 = strong edge)
    pub strength: f32,
    /// Additional properties of the edge
    pub properties: EdgeProperties,
}

impl TileEdge {
    /// Create new tile edge
    pub fn new(from_tile: TileId, to_tile: TileId, direction: HexDirection, edge_type: EdgeType, strength: f32) -> Self {
        Self {
            from_tile,
            to_tile,
            direction,
            edge_type,
            strength: strength.clamp(0.0, 1.0),
            properties: EdgeProperties::default(),
        }
    }

    /// Check if edge is significant (above threshold)
    pub fn is_significant(&self) -> bool {
        self.strength >= self.edge_type.strength_threshold()
    }

    /// Get edge intensity category
    pub fn intensity(&self) -> EdgeIntensity {
        if self.strength >= 0.8 { EdgeIntensity::VeryStrong }
        else if self.strength >= 0.6 { EdgeIntensity::Strong }
        else if self.strength >= 0.4 { EdgeIntensity::Moderate }
        else if self.strength >= 0.2 { EdgeIntensity::Weak }
        else { EdgeIntensity::VeryWeak }
    }
}

/// Edge intensity categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeIntensity {
    VeryWeak,
    Weak,
    Moderate,
    Strong,
    VeryStrong,
}

/// Additional properties for edges
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EdgeProperties {
    /// Whether edge blocks movement
    pub blocks_movement: bool,
    /// Movement cost multiplier
    pub movement_cost_modifier: f32,
    /// Visual representation data
    pub visual_style: EdgeVisualStyle,
    /// Custom metadata
    pub metadata: HashMap<String, String>,
}

/// Visual styling for edge rendering
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeVisualStyle {
    /// Line width for rendering
    pub line_width: f32,
    /// Color components (RGBA)
    pub color: [f32; 4],
    /// Whether edge should be dashed
    pub dashed: bool,
    /// Animation speed (if animated)
    pub animation_speed: f32,
}

impl Default for EdgeVisualStyle {
    fn default() -> Self {
        Self {
            line_width: 1.0,
            color: [1.0, 1.0, 1.0, 1.0], // White
            dashed: false,
            animation_speed: 0.0,
        }
    }
}
