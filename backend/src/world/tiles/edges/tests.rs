//! Tests for edge detection system

use super::{
    types::{EdgeType, TileEdge, EdgeIntensity},
    config::EdgeDetectionConfig,
};
use crate::world::tiles::adjacency::HexDirection;

#[test]
fn test_edge_type_properties() {
    assert!(EdgeType::TerrainBoundary.strength_threshold() > 0.0);
    assert!(EdgeType::Coastline.strength_threshold() > EdgeType::PoliticalBorder.strength_threshold());
}

#[test]
fn test_tile_edge_creation() {
    let edge = TileEdge::new(1, 2, HexDirection::East, EdgeType::TerrainBoundary, 0.5);
    
    assert_eq!(edge.from_tile, 1);
    assert_eq!(edge.to_tile, 2);
    assert_eq!(edge.direction, HexDirection::East);
    assert_eq!(edge.edge_type, EdgeType::TerrainBoundary);
    assert_eq!(edge.strength, 0.5);
    assert!(edge.is_significant());
}

#[test]
fn test_edge_intensity() {
    let weak_edge = TileEdge::new(1, 2, HexDirection::East, EdgeType::TerrainBoundary, 0.1);
    let strong_edge = TileEdge::new(1, 2, HexDirection::East, EdgeType::TerrainBoundary, 0.9);
    
    assert_eq!(weak_edge.intensity(), EdgeIntensity::VeryWeak);
    assert_eq!(strong_edge.intensity(), EdgeIntensity::VeryStrong);
}

#[test]
fn test_edge_detection_config() {
    let config = EdgeDetectionConfig::default();
    assert!(config.sobel_threshold > 0.0);
    assert!(config.canny_high_threshold > config.canny_low_threshold);
}
