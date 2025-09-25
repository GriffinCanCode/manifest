//! Command queue system for deterministic execution
//!
//! Provides ordered command processing with tick-based scheduling to ensure
//! deterministic game state changes across different execution environments.

use bevy_ecs::prelude::*;
use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::{
    collections::BinaryHeap,
    sync::Arc,
    cmp::{Ordering, Reverse},
};
use tracing::{debug, warn};
use crate::core::scheduler::Stage;

/// Command that can be executed deterministically
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "speedy", derive(speedy::Readable, speedy::Writable))]
pub enum SimulationCommand {
    /// Spawn entity with components
    SpawnEntity {
        components: DynamicBundle,
    },
    /// Despawn specific entity
    DespawnEntity {
        entity: Entity,
    },
    /// Update component on entity
    UpdateComponent {
        entity: Entity,
        component: ComponentData,
    },
    /// Execute system stage
    SystemExecution {
        stage: Stage,
    },
}

/// Dynamic component data for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentData {
    pub type_name: String,
    pub data: Vec<u8>,
}

/// Dynamic bundle for entity spawning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicBundle {
    pub components: Vec<ComponentData>,
}

impl DynamicBundle {
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
        }
    }

    pub fn with_component<T: Component + Serialize>(mut self, component: T) -> Self {
        let data = bincode::serialize(&component).unwrap_or_default();
        self.components.push(ComponentData {
            type_name: std::any::type_name::<T>().to_string(),
            data,
        });
        self
    }
}

impl SimulationCommand {
    /// Get delay in ticks before command should execute
    pub fn delay_ticks(&self) -> u64 {
        match self {
            SimulationCommand::SpawnEntity { .. } => 0,
            SimulationCommand::DespawnEntity { .. } => 0,
            SimulationCommand::UpdateComponent { .. } => 0,
            SimulationCommand::SystemExecution { .. } => 1, // Execute next tick
        }
    }

    /// Get command priority (higher = more important)
    pub fn priority(&self) -> u32 {
        match self {
            SimulationCommand::SystemExecution { .. } => 100,
            SimulationCommand::SpawnEntity { .. } => 75,
            SimulationCommand::UpdateComponent { .. } => 50,
            SimulationCommand::DespawnEntity { .. } => 25,
        }
    }
}

/// Scheduled command with execution tick
#[derive(Debug, Clone)]
pub struct ScheduledCommand {
    pub command: SimulationCommand,
    pub tick: u64,
    pub sequence: u64, // For deterministic ordering within same tick
}

impl PartialEq for ScheduledCommand {
    fn eq(&self, other: &Self) -> bool {
        self.tick == other.tick && self.sequence == other.sequence
    }
}

impl Eq for ScheduledCommand {}

impl PartialOrd for ScheduledCommand {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScheduledCommand {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse for min-heap (earliest tick first)
        match other.tick.cmp(&self.tick) {
            Ordering::Equal => {
                // Within same tick, order by priority then sequence
                match other.command.priority().cmp(&self.command.priority()) {
                    Ordering::Equal => other.sequence.cmp(&self.sequence),
                    ord => ord,
                }
            }
            ord => ord,
        }
    }
}

/// Result of command execution
#[derive(Debug, Clone)]
pub enum CommandResult {
    EntitySpawned(Entity),
    EntityDespawned(Entity),
    ComponentUpdated(Entity),
    SystemExecuted(Stage),
}

/// Thread-safe command queue with deterministic ordering
#[derive(Debug)]
pub struct CommandQueue {
    /// Priority queue for scheduled commands
    queue: BinaryHeap<ScheduledCommand>,
    /// Channel for receiving new commands
    receiver: Receiver<ScheduledCommand>,
    /// Channel sender for queuing commands
    sender: Sender<ScheduledCommand>,
    /// Sequence counter for deterministic ordering
    sequence_counter: u64,
}

impl CommandQueue {
    /// Create new command queue
    pub fn new() -> Self {
        let (sender, receiver) = unbounded();
        
        Self {
            queue: BinaryHeap::new(),
            receiver,
            sender,
            sequence_counter: 0,
        }
    }

    /// Enqueue command for execution at specific tick
    pub fn enqueue(&mut self, command: SimulationCommand, target_tick: u64) {
        let scheduled = ScheduledCommand {
            command,
            tick: target_tick,
            sequence: self.sequence_counter,
        };
        
        self.sequence_counter += 1;
        self.queue.push(scheduled);
        
        debug!("Enqueued command for tick {}", target_tick);
    }

    /// Drain all commands ready for execution at current tick
    pub fn drain_for_tick(&mut self, current_tick: u64) -> Vec<SimulationCommand> {
        // Process any commands from channel first
        self.process_channel_commands();
        
        let mut commands = Vec::new();
        
        // Extract all commands for current tick
        while let Some(scheduled) = self.queue.peek() {
            if scheduled.tick <= current_tick {
                if let Some(scheduled) = self.queue.pop() {
                    commands.push(scheduled.command);
                }
            } else {
                break;
            }
        }
        
        debug!("Drained {} commands for tick {}", commands.len(), current_tick);
        commands
    }

    /// Get number of queued commands
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Get sender for async command queuing
    pub fn sender(&self) -> Sender<ScheduledCommand> {
        self.sender.clone()
    }

    /// Clear all queued commands
    pub fn clear(&mut self) {
        self.queue.clear();
        // Drain channel
        while let Ok(_) = self.receiver.try_recv() {}
        debug!("Cleared command queue");
    }

    /// Get commands scheduled for future ticks
    pub fn peek_future(&self, from_tick: u64, limit: usize) -> Vec<&ScheduledCommand> {
        self.queue
            .iter()
            .filter(|cmd| cmd.tick >= from_tick)
            .take(limit)
            .collect()
    }

    fn process_channel_commands(&mut self) {
        loop {
            match self.receiver.try_recv() {
                Ok(scheduled) => {
                    self.queue.push(scheduled);
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    warn!("Command queue channel disconnected");
                    break;
                }
            }
        }
    }
}

/// Thread-safe command queue wrapper
#[derive(Debug)]
pub struct ConcurrentCommandQueue {
    inner: Arc<Mutex<CommandQueue>>,
}

impl ConcurrentCommandQueue {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(CommandQueue::new())),
        }
    }

    pub fn enqueue(&self, command: SimulationCommand, target_tick: u64) {
        self.inner.lock().enqueue(command, target_tick);
    }

    pub fn drain_for_tick(&self, current_tick: u64) -> Vec<SimulationCommand> {
        self.inner.lock().drain_for_tick(current_tick)
    }

    pub fn len(&self) -> usize {
        self.inner.lock().len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.lock().is_empty()
    }

    pub fn sender(&self) -> Sender<ScheduledCommand> {
        self.inner.lock().sender()
    }

    pub fn clear(&self) {
        self.inner.lock().clear();
    }
}

/// Command batch for efficient processing
#[derive(Debug)]
pub struct CommandBatch {
    pub commands: Vec<SimulationCommand>,
    pub tick: u64,
}

impl CommandBatch {
    pub fn new(tick: u64) -> Self {
        Self {
            commands: Vec::new(),
            tick,
        }
    }

    pub fn add(&mut self, command: SimulationCommand) {
        self.commands.push(command);
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_queue_ordering() {
        let mut queue = CommandQueue::new();
        
        // Add commands in reverse order
        queue.enqueue(
            SimulationCommand::SpawnEntity { components: DynamicBundle::new() },
            3
        );
        queue.enqueue(
            SimulationCommand::DespawnEntity { entity: Entity::from_raw(1) },
            1
        );
        queue.enqueue(
            SimulationCommand::SystemExecution { stage: Stage::Update },
            2
        );
        
        // Should drain in tick order
        let commands_1 = queue.drain_for_tick(1);
        assert_eq!(commands_1.len(), 1);
        
        let commands_2 = queue.drain_for_tick(2);
        assert_eq!(commands_2.len(), 1);
        
        let commands_3 = queue.drain_for_tick(3);
        assert_eq!(commands_3.len(), 1);
    }

    #[test]
    fn test_priority_ordering() {
        let mut queue = CommandQueue::new();
        
        // Add multiple commands for same tick with different priorities
        queue.enqueue(
            SimulationCommand::DespawnEntity { entity: Entity::from_raw(1) },
            1
        );
        queue.enqueue(
            SimulationCommand::SystemExecution { stage: Stage::Update },
            1
        );
        queue.enqueue(
            SimulationCommand::SpawnEntity { components: DynamicBundle::new() },
            1
        );
        
        let commands = queue.drain_for_tick(1);
        assert_eq!(commands.len(), 3);
        
        // SystemExecution should be first (highest priority)
        if let SimulationCommand::SystemExecution { .. } = commands[0] {
            // Expected
        } else {
            panic!("Expected SystemExecution first");
        }
    }

    #[test]
    fn test_deterministic_bundle() {
        let bundle = DynamicBundle::new()
            .with_component(42u32)
            .with_component("test".to_string());
        
        assert_eq!(bundle.components.len(), 2);
    }
}
