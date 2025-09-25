//! System functions for maintaining adjacency graph
//!
//! Contains Bevy ECS systems for adjacency graph management.

use bevy_ecs::prelude::*;
use tracing::warn;

use super::core::TileAdjacencyGraph;

/// System for maintaining adjacency graph consistency
pub fn maintain_adjacency_system(
    adjacency_graph: Res<TileAdjacencyGraph>,
) {
    // Validate adjacency graph periodically
    if let Err(e) = adjacency_graph.validate() {
        warn!("Adjacency graph validation failed: {}", e);
    }
}

/// System for updating adjacency based on tile changes
pub fn update_adjacency_system(
    adjacency_graph: Res<TileAdjacencyGraph>,
    // This would include queries for changed tiles
) {
    // Monitor tile changes and update adjacencies accordingly
    // Implementation would depend on change detection system
}
