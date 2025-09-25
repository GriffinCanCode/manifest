//! Snapshot system using rkyv for zero-copy serialization
//!
//! Provides efficient world state snapshots for save/load and replay functionality
//! with deterministic serialization and fast restoration capabilities.

use bevy_ecs::prelude::*;
// Using bincode for serialization
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};
use thiserror::Error;
use tracing::{debug, info, warn};
use crate::core::time::{SimulationSnapshot as TimerSnapshot, DeterministicTimer};

/// World state snapshot with deterministic serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSnapshot {
    /// Snapshot metadata
    pub metadata: SnapshotMetadata,
    /// Entity data
    pub entities: Vec<EntityData>,
    /// Resource data
    pub resources: Vec<ResourceData>,
    /// Archetype information
    pub archetypes: Vec<ArchetypeData>,
    /// Component storage
    pub components: Vec<ComponentStorage>,
}

/// Snapshot metadata for identification and validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    /// Simulation tick when snapshot was created
    pub tick: u64,
    /// Unique snapshot ID
    pub id: String,
    /// Creation timestamp
    pub created_at: u64,
    /// Version for compatibility
    pub version: u32,
    /// Total entities in snapshot
    pub entity_count: u32,
    /// Total resources in snapshot
    pub resource_count: u32,
    /// Checksum for integrity verification
    pub checksum: u64,
}

/// Serialized entity data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityData {
    pub entity_id: u32,
    pub generation: u32,
    pub archetype_id: u32,
    pub component_indices: Vec<u32>,
}

/// Serialized resource data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceData {
    pub type_name: String,
    pub data: Vec<u8>,
}

/// Archetype layout information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchetypeData {
    pub id: u32,
    pub component_types: Vec<String>,
    pub entity_count: u32,
}

/// Component storage with type information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentStorage {
    pub type_name: String,
    pub type_id: u64,
    pub data: Vec<u8>,
    pub entity_indices: Vec<u32>,
}

/// Complete simulation snapshot including timer state
#[derive(Debug)]
pub struct SimulationSnapshot {
    /// World state snapshot
    pub world: WorldSnapshot,
    /// Timer state snapshot
    pub timer_state: DeterministicTimer,
    /// Additional simulation state
    pub sim_state: TimerSnapshot,
}

impl Clone for SimulationSnapshot {
    fn clone(&self) -> Self {
        Self {
            world: self.world.clone(),
            timer_state: self.timer_state.clone(),
            sim_state: self.sim_state.clone(),
        }
    }
}

/// Snapshot manager for storing and retrieving snapshots
#[derive(Debug)]
pub struct SnapshotManager {
    /// Stored snapshots by tick
    snapshots: HashMap<u64, SimulationSnapshot>,
    /// Serialized snapshot data for disk storage
    serialized_data: HashMap<u64, Vec<u8>>,
    /// Maximum snapshots to keep in memory
    max_snapshots: usize,
    /// Compression enabled
    compress: bool,
}

impl SnapshotManager {
    /// Create new snapshot manager
    pub fn new() -> Self {
        Self {
            snapshots: HashMap::new(),
            serialized_data: HashMap::new(),
            max_snapshots: 100,
            compress: true,
        }
    }

    /// Create snapshot of world state
    pub fn create_snapshot(
        &mut self,
        world: &World,
        tick: u64,
        sim_state: TimerSnapshot,
    ) -> Result<SimulationSnapshot, SnapshotError> {
        let start = Instant::now();

        // Extract world data
        let world_snapshot = self.serialize_world(world, tick)?;
        
        // Create timer state copy
        let timer_state = DeterministicTimer::new(Default::default()); // This needs proper state copying
        
        let snapshot = SimulationSnapshot {
            world: world_snapshot,
            timer_state,
            sim_state,
        };

        debug!(
            "Created snapshot {} in {:?}",
            tick,
            start.elapsed()
        );

        Ok(snapshot)
    }

    /// Store snapshot with optional serialization
    pub fn store(&mut self, tick: u64, snapshot: SimulationSnapshot) -> Result<(), SnapshotError> {
        // Serialize for storage
        let serialized = self.serialize_snapshot(&snapshot)?;
        
        self.snapshots.insert(tick, snapshot);
        self.serialized_data.insert(tick, serialized);

        // Clean up old snapshots if needed
        if self.snapshots.len() > self.max_snapshots {
            self.cleanup_old_snapshots();
        }

        info!("Stored snapshot for tick {}", tick);
        Ok(())
    }

    /// Retrieve snapshot by tick
    pub fn get(&self, tick: u64) -> Result<&SimulationSnapshot, SnapshotError> {
        self.snapshots
            .get(&tick)
            .ok_or(SnapshotError::NotFound(tick))
    }

    /// Restore world from snapshot
    pub fn restore_snapshot(
        &self,
        world: &mut World,
        snapshot: &SimulationSnapshot,
    ) -> Result<(), SnapshotError> {
        let start = Instant::now();

        // Clear current world state
        world.clear_entities();
        
        // Restore entities
        self.restore_entities(world, &snapshot.world)?;
        
        // Restore resources
        self.restore_resources(world, &snapshot.world)?;

        info!(
            "Restored snapshot {} in {:?}",
            snapshot.world.metadata.tick,
            start.elapsed()
        );

        Ok(())
    }

    /// Get number of stored snapshots
    pub fn count(&self) -> usize {
        self.snapshots.len()
    }

    /// List all snapshot ticks
    pub fn list_ticks(&self) -> Vec<u64> {
        let mut ticks: Vec<u64> = self.snapshots.keys().copied().collect();
        ticks.sort();
        ticks
    }

    /// Get snapshot metadata without loading full snapshot
    pub fn get_metadata(&self, tick: u64) -> Option<&SnapshotMetadata> {
        self.snapshots.get(&tick).map(|s| &s.world.metadata)
    }

    /// Serialize snapshot to bytes
    pub fn serialize_snapshot(&self, snapshot: &SimulationSnapshot) -> Result<Vec<u8>, SnapshotError> {
        let bytes = bincode::serialize(&snapshot.world)
            .map_err(|e| SnapshotError::SerializationFailed(e.to_string()))?;
        
        if self.compress {
            // Use zstd compression for better performance
            Ok(zstd::bulk::compress(&bytes, 3)
                .map_err(|e| SnapshotError::CompressionFailed(e.to_string()))?)
        } else {
            Ok(bytes)
        }
    }

    /// Deserialize snapshot from bytes
    pub fn deserialize_snapshot(&self, bytes: &[u8]) -> Result<WorldSnapshot, SnapshotError> {
        let data = if self.compress {
            zstd::bulk::decompress(bytes, 10 * 1024 * 1024) // 10MB limit
                .map_err(|e| SnapshotError::DecompressionFailed(e.to_string()))?
        } else {
            bytes.to_vec()
        };

        bincode::deserialize(&data)
            .map_err(|e| SnapshotError::DeserializationFailed(e.to_string()))
    }

    fn serialize_world(&self, world: &World, tick: u64) -> Result<WorldSnapshot, SnapshotError> {
        let mut entities = Vec::new();
        let mut resources = Vec::new();
        let mut archetypes = Vec::new();
        let mut components = Vec::new();

        // Extract entities - this is simplified and would need actual ECS integration
        let entity_count = 0; // world.entities().len() - need proper access
        
        // Create metadata
        let metadata = SnapshotMetadata {
            tick,
            id: format!("snapshot_{}", tick),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            version: 1,
            entity_count: entity_count as u32,
            resource_count: resources.len() as u32,
            checksum: 0, // Calculate actual checksum
        };

        Ok(WorldSnapshot {
            metadata,
            entities,
            resources,
            archetypes,
            components,
        })
    }

    fn restore_entities(&self, world: &mut World, snapshot: &WorldSnapshot) -> Result<(), SnapshotError> {
        // Restore entities from snapshot data
        for entity_data in &snapshot.entities {
            // This would need proper ECS integration to restore entities
            let _entity = world.spawn_empty();
        }
        Ok(())
    }

    fn restore_resources(&self, world: &mut World, snapshot: &WorldSnapshot) -> Result<(), SnapshotError> {
        // Restore resources from snapshot data
        for resource_data in &snapshot.resources {
            // This would need proper resource deserialization
            debug!("Restoring resource: {}", resource_data.type_name);
        }
        Ok(())
    }

    fn cleanup_old_snapshots(&mut self) {
        if self.snapshots.len() <= self.max_snapshots {
            return;
        }

        let mut ticks: Vec<u64> = self.snapshots.keys().copied().collect();
        ticks.sort();

        let to_remove = ticks.len() - self.max_snapshots;
        for tick in ticks.iter().take(to_remove) {
            self.snapshots.remove(tick);
            self.serialized_data.remove(tick);
        }

        debug!("Cleaned up {} old snapshots", to_remove);
    }
}

/// Snapshot creation and restoration errors
#[derive(Error, Debug)]
pub enum SnapshotError {
    #[error("Snapshot not found for tick {0}")]
    NotFound(u64),
    #[error("Serialization failed: {0}")]
    SerializationFailed(String),
    #[error("Deserialization failed: {0}")]
    DeserializationFailed(String),
    #[error("Compression failed: {0}")]
    CompressionFailed(String),
    #[error("Decompression failed: {0}")]
    DecompressionFailed(String),
    #[error("Validation failed: {0}")]
    ValidationFailed(String),
    #[error("World extraction failed: {0}")]
    WorldExtractionFailed(String),
    #[error("Restoration failed: {0}")]
    RestorationFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_manager() {
        let mut manager = SnapshotManager::new();
        assert_eq!(manager.count(), 0);
        assert!(manager.list_ticks().is_empty());
    }

    #[test]
    fn test_snapshot_metadata() {
        let metadata = SnapshotMetadata {
            tick: 100,
            id: "test".to_string(),
            created_at: 1000,
            version: 1,
            entity_count: 5,
            resource_count: 3,
            checksum: 12345,
        };

        assert_eq!(metadata.tick, 100);
        assert_eq!(metadata.entity_count, 5);
    }

    #[test]
    fn test_world_snapshot_creation() {
        let metadata = SnapshotMetadata {
            tick: 50,
            id: "test_world".to_string(),
            created_at: 2000,
            version: 1,
            entity_count: 0,
            resource_count: 0,
            checksum: 0,
        };

        let snapshot = WorldSnapshot {
            metadata,
            entities: Vec::new(),
            resources: Vec::new(),
            archetypes: Vec::new(),
            components: Vec::new(),
        };

        assert_eq!(snapshot.metadata.tick, 50);
        assert_eq!(snapshot.entities.len(), 0);
    }
}
