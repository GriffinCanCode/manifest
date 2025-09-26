//! Deterministic simulation pipeline for grand strategy game
//!
//! Provides command queuing, snapshot system, replay functionality, and checksum
//! verification for reproducible game simulations across platforms and runs.

pub mod commands;
pub mod snapshots;
pub mod replay;
pub mod verification;
pub mod sync;

// Re-export core types
pub use commands::*;
pub use snapshots::*;
pub use replay::*;
pub use verification::*;
pub use sync::*;

use crate::core::time::SimulationState;
use bevy_ecs::prelude::*;
use parking_lot::RwLock;
use std::sync::Arc;
use tracing::{debug, info};

/// Central simulation coordinator that manages all deterministic systems
#[derive(Debug)]
pub struct SimulationCore {
    /// Simulation timing and RNG state
    pub state: Arc<SimulationState>,
    /// Command queue for deterministic execution
    pub commands: Arc<RwLock<CommandQueue>>,
    /// Snapshot manager for save/load and replay
    pub snapshots: Arc<RwLock<SnapshotManager>>,
    /// Replay system for deterministic playback
    pub replay: Arc<RwLock<ReplayManager>>,
    /// Verification system for integrity checking
    pub verification: Arc<VerificationSystem>,
    /// Tick synchronization manager
    pub sync: Arc<RwLock<TickSynchronizer>>,
}

impl SimulationCore {
    /// Create new simulation core with given seed
    pub fn new(seed: u64) -> Self {
        let state = Arc::new(SimulationState::new(seed, None));
        let commands = Arc::new(RwLock::new(CommandQueue::new()));
        let snapshots = Arc::new(RwLock::new(SnapshotManager::new()));
        let replay = Arc::new(RwLock::new(ReplayManager::new()));
        let verification = Arc::new(VerificationSystem::new(seed));
        let sync = Arc::new(RwLock::new(TickSynchronizer::new()));

        info!("Initialized simulation core with seed: {}", seed);

        Self {
            state,
            commands,
            snapshots,
            replay,
            verification,
            sync,
        }
    }

    /// Execute one simulation step
    pub fn step(&self, world: &mut World) -> Result<SimulationStepResult, SimulationError> {
        let tick = self.state.tick();
        debug!("Starting simulation step {}", tick);

        // Process queued commands
        let commands = {
            let mut cmd_queue = self.commands.write();
            cmd_queue.drain_for_tick(tick)
        };

        // Execute commands deterministically
        let mut results = Vec::new();
        for command in commands {
            let result = self.execute_command(command, world)?;
            results.push(result);
        }

        // Create snapshot if needed
        if self.should_snapshot(tick) {
            let snapshot = self.create_snapshot(world)?;
            self.snapshots.write().store(tick, snapshot)?;
        }

        // Verify state integrity
        let checksum = self.verification.calculate_checksum(world, tick);
        self.verification.verify_checksum(tick, checksum)
            .map_err(SimulationError::Verification)?;

        // Update simulation state
        self.state.update();

        Ok(SimulationStepResult {
            tick,
            commands_executed: results.len() as u32,
            checksum,
            snapshot_created: self.should_snapshot(tick),
        })
    }

    /// Queue command for execution
    pub fn queue_command(&self, command: SimulationCommand) {
        let target_tick = self.state.tick() + command.delay_ticks();
        self.commands.write().enqueue(command, target_tick);
    }

    /// Create snapshot of current state
    pub fn create_snapshot(&self, world: &mut World) -> Result<SimulationSnapshot, SimulationError> {
        let tick = self.state.tick();
        let sim_state = self.state.state();
        
        self.snapshots.write().create_snapshot(world, tick, sim_state)
            .map_err(SimulationError::Snapshot)
    }

    /// Load from snapshot
    pub fn load_snapshot(&self, world: &mut World, tick: u64) -> Result<(), SimulationError> {
        let snapshot = {
            let snapshots = self.snapshots.read();
            snapshots.get(tick)?.clone()
        };
        self.snapshots.read().restore_snapshot(world, &snapshot)?;
        
        // Reset simulation state to snapshot tick
        *self.state.timer.lock() = snapshot.timer_state.clone();
        
        info!("Loaded simulation from snapshot at tick {}", tick);
        Ok(())
    }

    /// Start replay from specific tick
    pub fn start_replay(&self, from_tick: u64) -> Result<(), SimulationError> {
        self.replay.write().start(from_tick)?;
        info!("Started replay from tick {}", from_tick);
        Ok(())
    }

    /// Get current simulation metrics
    pub fn metrics(&self) -> SimulationMetrics {
        SimulationMetrics {
            current_tick: self.state.tick(),
            commands_queued: self.commands.read().len(),
            snapshots_stored: self.snapshots.read().count(),
            checksums_verified: self.verification.verified_count(),
            deterministic_seed: self.state.initial_seed,
        }
    }

    fn execute_command(&self, command: SimulationCommand, world: &mut World) -> Result<CommandResult, SimulationError> {
        // Commands will be executed through the ECS scheduler
        match command {
            SimulationCommand::SpawnEntity { components } => {
                // Spawn empty entity first, then add components dynamically
                let mut entity_commands = world.spawn_empty();
                
                // Deserialize and add each component
                for component_data in components.components {
                    self.deserialize_and_add_component(&mut entity_commands, &component_data)?;
                }
                
                let entity = entity_commands.id();
                Ok(CommandResult::EntitySpawned(entity))
            }
            SimulationCommand::DespawnEntity { entity } => {
                if world.despawn(entity) {
                    Ok(CommandResult::EntityDespawned(entity))
                } else {
                    Err(SimulationError::EntityNotFound(entity))
                }
            }
            SimulationCommand::UpdateComponent { entity, component } => {
                // This would need component-specific handling
                Ok(CommandResult::ComponentUpdated(entity))
            }
            SimulationCommand::SystemExecution { stage } => {
                // Execute specific system stage
                Ok(CommandResult::SystemExecuted(stage))
            }
        }
    }

    fn should_snapshot(&self, tick: u64) -> bool {
        // Snapshot every 100 ticks or at specific intervals
        tick % 100 == 0
    }

    /// Deserialize component data and add it to an entity
    fn deserialize_and_add_component(
        &self,
        entity_commands: &mut bevy_ecs::world::EntityWorldMut,
        component_data: &crate::simulation::commands::ComponentData,
    ) -> Result<(), SimulationError> {
        use crate::ecs::components::*;
        
        match component_data.type_name.as_str() {
            "manifest_rust_ts::ecs::components::Position" => {
                let component: Position = bincode::deserialize(&component_data.data)
                    .map_err(|e| SimulationError::DeserializationError(format!("Position: {}", e)))?;
                entity_commands.insert(component);
            }
            "manifest_rust_ts::ecs::components::Movement" => {
                let component: Movement = bincode::deserialize(&component_data.data)
                    .map_err(|e| SimulationError::DeserializationError(format!("Movement: {}", e)))?;
                entity_commands.insert(component);
            }
            "manifest_rust_ts::ecs::components::Health" => {
                let component: Health = bincode::deserialize(&component_data.data)
                    .map_err(|e| SimulationError::DeserializationError(format!("Health: {}", e)))?;
                entity_commands.insert(component);
            }
            "manifest_rust_ts::ecs::components::Renderable" => {
                let component: Renderable = bincode::deserialize(&component_data.data)
                    .map_err(|e| SimulationError::DeserializationError(format!("Renderable: {}", e)))?;
                entity_commands.insert(component);
            }
            "manifest_rust_ts::ecs::components::Owner" => {
                let component: Owner = bincode::deserialize(&component_data.data)
                    .map_err(|e| SimulationError::DeserializationError(format!("Owner: {}", e)))?;
                entity_commands.insert(component);
            }
            "manifest_rust_ts::ecs::components::Name" => {
                let component: Name = bincode::deserialize(&component_data.data)
                    .map_err(|e| SimulationError::DeserializationError(format!("Name: {}", e)))?;
                entity_commands.insert(component);
            }
            "manifest_rust_ts::ecs::components::InterpolatedPosition" => {
                let component: InterpolatedPosition = bincode::deserialize(&component_data.data)
                    .map_err(|e| SimulationError::DeserializationError(format!("InterpolatedPosition: {}", e)))?;
                entity_commands.insert(component);
            }
            "manifest_rust_ts::ecs::components::InterpolatedHealth" => {
                let component: InterpolatedHealth = bincode::deserialize(&component_data.data)
                    .map_err(|e| SimulationError::DeserializationError(format!("InterpolatedHealth: {}", e)))?;
                entity_commands.insert(component);
            }
            "manifest_rust_ts::ecs::components::InterpolatedRenderable" => {
                let component: InterpolatedRenderable = bincode::deserialize(&component_data.data)
                    .map_err(|e| SimulationError::DeserializationError(format!("InterpolatedRenderable: {}", e)))?;
                entity_commands.insert(component);
            }
            // Handle short type names as well (when using std::any::type_name)
            "Position" => {
                let component: Position = bincode::deserialize(&component_data.data)
                    .map_err(|e| SimulationError::DeserializationError(format!("Position: {}", e)))?;
                entity_commands.insert(component);
            }
            "Movement" => {
                let component: Movement = bincode::deserialize(&component_data.data)
                    .map_err(|e| SimulationError::DeserializationError(format!("Movement: {}", e)))?;
                entity_commands.insert(component);
            }
            "Health" => {
                let component: Health = bincode::deserialize(&component_data.data)
                    .map_err(|e| SimulationError::DeserializationError(format!("Health: {}", e)))?;
                entity_commands.insert(component);
            }
            "Renderable" => {
                let component: Renderable = bincode::deserialize(&component_data.data)
                    .map_err(|e| SimulationError::DeserializationError(format!("Renderable: {}", e)))?;
                entity_commands.insert(component);
            }
            "Owner" => {
                let component: Owner = bincode::deserialize(&component_data.data)
                    .map_err(|e| SimulationError::DeserializationError(format!("Owner: {}", e)))?;
                entity_commands.insert(component);
            }
            "Name" => {
                let component: Name = bincode::deserialize(&component_data.data)
                    .map_err(|e| SimulationError::DeserializationError(format!("Name: {}", e)))?;
                entity_commands.insert(component);
            }
            "InterpolatedPosition" => {
                let component: InterpolatedPosition = bincode::deserialize(&component_data.data)
                    .map_err(|e| SimulationError::DeserializationError(format!("InterpolatedPosition: {}", e)))?;
                entity_commands.insert(component);
            }
            "InterpolatedHealth" => {
                let component: InterpolatedHealth = bincode::deserialize(&component_data.data)
                    .map_err(|e| SimulationError::DeserializationError(format!("InterpolatedHealth: {}", e)))?;
                entity_commands.insert(component);
            }
            "InterpolatedRenderable" => {
                let component: InterpolatedRenderable = bincode::deserialize(&component_data.data)
                    .map_err(|e| SimulationError::DeserializationError(format!("InterpolatedRenderable: {}", e)))?;
                entity_commands.insert(component);
            }
            _ => {
                return Err(SimulationError::UnknownComponent(component_data.type_name.clone()));
            }
        }
        
        Ok(())
    }
}

/// Result of a simulation step
#[derive(Debug, Clone)]
pub struct SimulationStepResult {
    pub tick: u64,
    pub commands_executed: u32,
    pub checksum: u64,
    pub snapshot_created: bool,
}

/// Simulation performance and state metrics
#[derive(Debug, Clone)]
pub struct SimulationMetrics {
    pub current_tick: u64,
    pub commands_queued: usize,
    pub snapshots_stored: usize,
    pub checksums_verified: u64,
    pub deterministic_seed: u64,
}

/// Simulation-specific errors
#[derive(Debug, thiserror::Error)]
pub enum SimulationError {
    #[error("Entity not found: {0:?}")]
    EntityNotFound(Entity),
    #[error("Snapshot error: {0}")]
    Snapshot(#[from] SnapshotError),
    #[error("Command execution error: {0}")]
    CommandExecution(String),
    #[error("Verification failed: {0}")]
    VerificationFailed(String),
    #[error("Verification error: {0}")]
    Verification(#[from] VerificationError),
    #[error("Replay error: {0}")]
    Replay(#[from] ReplayError),
    #[error("Sync error: {0}")]
    Sync(#[from] SyncError),
    #[error("Deserialization error: {0}")]
    DeserializationError(String),
    #[error("Unknown component type: {0}")]
    UnknownComponent(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simulation_core_creation() {
        let core = SimulationCore::new(42);
        assert_eq!(core.state.initial_seed, 42);
        assert_eq!(core.state.tick(), 0);
    }

    #[test]
    fn test_command_queuing() {
        let core = SimulationCore::new(123);
        let command = SimulationCommand::SpawnEntity {
            components: DynamicBundle::new(),
        };
        
        core.queue_command(command);
        assert_eq!(core.commands.read().len(), 1);
    }

    #[test]
    fn test_deterministic_behavior() {
        let core1 = SimulationCore::new(999);
        let core2 = SimulationCore::new(999);
        
        // Same seed should produce same random values
        let val1 = core1.state.gen_u32();
        let val2 = core2.state.gen_u32();
        assert_eq!(val1, val2);
    }
}
