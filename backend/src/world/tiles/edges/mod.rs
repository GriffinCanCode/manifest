//! Edge detection with image crate algorithms for tile boundaries
//!
//! Provides sophisticated edge detection for tile-based systems using image
//! processing algorithms to identify terrain boundaries, political borders,
//! and other significant transitions between tiles.

pub mod types;
pub mod config;
pub mod detector;
pub mod stats;
pub mod systems;

#[cfg(test)]
mod tests;

// Re-export main types and functionality
pub use types::{
    EdgeType, TileEdge, EdgeIntensity, EdgeProperties, EdgeVisualStyle
};
pub use config::EdgeDetectionConfig;
pub use detector::TileEdgeDetector;
pub use stats::{EdgeDetectionStats, EdgeDetectionError};
pub use systems::update_edges_system;
