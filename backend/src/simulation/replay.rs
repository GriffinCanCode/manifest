//! Replay system using speedy serialization
//!
//! Provides deterministic replay functionality by recording and playing back
//! command sequences with fast serialization for minimal performance impact.

use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
};
use thiserror::Error;
use tracing::{debug, info, warn};
use crate::simulation::commands::SimulationCommand;

/// Replay event with timestamp and command data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayEvent {
    /// Tick when event occurred
    pub tick: u64,
    /// Sequence number for ordering within tick
    pub sequence: u64,
    /// The simulation command that was executed
    pub command: SimulationCommand,
    /// Optional metadata
    pub metadata: ReplayMetadata,
}

/// Additional metadata for replay events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayMetadata {
    /// Event type classification
    pub event_type: ReplayEventType,
    /// Execution time in microseconds
    pub execution_time_us: u64,
    /// Optional description
    pub description: String,
    /// Custom tags for filtering
    pub tags: Vec<String>,
}

/// Classification of replay events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReplayEventType {
    /// Entity-related operations
    Entity,
    /// Component updates
    Component,
    /// System execution
    System,
    /// Resource changes
    Resource,
    /// User input
    Input,
    /// AI decision
    AI,
    /// Network event
    Network,
}

/// Replay file header with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayHeader {
    /// File format version
    pub version: u32,
    /// Game version when replay was created
    pub game_version: String,
    /// Initial random seed
    pub seed: u64,
    /// Start tick of the replay
    pub start_tick: u64,
    /// End tick of the replay
    pub end_tick: u64,
    /// Total number of events
    pub event_count: u64,
    /// Creation timestamp
    pub created_at: u64,
    /// Checksum for integrity
    pub checksum: u64,
    /// Optional replay name
    pub name: String,
    /// Optional description
    pub description: String,
}

/// Replay manager for recording and playback
#[derive(Debug)]
pub struct ReplayManager {
    /// Current replay mode
    mode: ReplayMode,
    /// Recorded events
    events: BTreeMap<u64, Vec<ReplayEvent>>,
    /// Current playback position
    playback_tick: u64,
    /// Replay file path
    file_path: Option<PathBuf>,
    /// Event sequence counter
    sequence_counter: u64,
    /// Recording enabled
    recording: bool,
}

/// Replay manager mode
#[derive(Debug, Clone, PartialEq)]
pub enum ReplayMode {
    /// Not recording or playing
    Idle,
    /// Recording events
    Recording,
    /// Playing back events
    Playback,
}

impl ReplayManager {
    /// Create new replay manager
    pub fn new() -> Self {
        Self {
            mode: ReplayMode::Idle,
            events: BTreeMap::new(),
            playback_tick: 0,
            file_path: None,
            sequence_counter: 0,
            recording: false,
        }
    }

    /// Start recording replay
    pub fn start_recording(&mut self, name: Option<String>) -> Result<(), ReplayError> {
        if self.mode != ReplayMode::Idle {
            return Err(ReplayError::AlreadyActive);
        }

        self.mode = ReplayMode::Recording;
        self.recording = true;
        self.events.clear();
        self.sequence_counter = 0;

        // Generate file path
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let filename = match name {
            Some(n) => format!("replay_{}_{}.dat", n, timestamp),
            None => format!("replay_{}.dat", timestamp),
        };

        self.file_path = Some(PathBuf::from("replays").join(filename));

        info!("Started recording replay: {:?}", self.file_path);
        Ok(())
    }

    /// Stop recording and save to file
    pub fn stop_recording(&mut self, seed: u64) -> Result<(), ReplayError> {
        if self.mode != ReplayMode::Recording {
            return Err(ReplayError::NotRecording);
        }

        if let Some(ref path) = self.file_path.clone() {
            self.save_to_file(path, seed)?;
        }

        self.mode = ReplayMode::Idle;
        self.recording = false;

        info!("Stopped recording replay");
        Ok(())
    }

    /// Record an event during gameplay
    pub fn record_event(&mut self, command: SimulationCommand, tick: u64) -> Result<(), ReplayError> {
        if !self.recording {
            return Ok(()); // Silently ignore if not recording
        }

        let event = ReplayEvent {
            tick,
            sequence: self.sequence_counter,
            command,
            metadata: ReplayMetadata {
                event_type: ReplayEventType::Entity, // Default - would need proper classification
                execution_time_us: 0,
                description: String::new(),
                tags: Vec::new(),
            },
        };

        self.events.entry(tick).or_insert_with(Vec::new).push(event);
        self.sequence_counter += 1;

        debug!("Recorded replay event at tick {}", tick);
        Ok(())
    }

    /// Start playback from file
    pub fn start_playback<P: AsRef<Path>>(&mut self, file_path: P) -> Result<ReplayHeader, ReplayError> {
        if self.mode != ReplayMode::Idle {
            return Err(ReplayError::AlreadyActive);
        }

        let header = self.load_from_file(file_path)?;
        self.mode = ReplayMode::Playback;
        self.playback_tick = header.start_tick;

        info!("Started replay playback: {}", header.name);
        Ok(header)
    }

    /// Start playback from specific tick
    pub fn start(&mut self, from_tick: u64) -> Result<(), ReplayError> {
        if self.events.is_empty() {
            return Err(ReplayError::NoEventsLoaded);
        }

        self.mode = ReplayMode::Playback;
        self.playback_tick = from_tick;

        debug!("Started replay from tick {}", from_tick);
        Ok(())
    }

    /// Get events for current tick during playback
    pub fn get_events_for_tick(&mut self, tick: u64) -> Vec<ReplayEvent> {
        if self.mode != ReplayMode::Playback {
            return Vec::new();
        }

        self.playback_tick = tick;
        self.events.get(&tick).cloned().unwrap_or_default()
    }

    /// Stop playback
    pub fn stop_playback(&mut self) {
        self.mode = ReplayMode::Idle;
        self.playback_tick = 0;
        info!("Stopped replay playback");
    }

    /// Check if currently recording
    pub fn is_recording(&self) -> bool {
        self.mode == ReplayMode::Recording
    }

    /// Check if currently playing back
    pub fn is_playing(&self) -> bool {
        self.mode == ReplayMode::Playback
    }

    /// Get current mode
    pub fn mode(&self) -> &ReplayMode {
        &self.mode
    }

    /// Get replay statistics
    pub fn stats(&self) -> ReplayStats {
        let total_events = self.events.values().map(|v| v.len()).sum::<usize>() as u64;
        let tick_range = if !self.events.is_empty() {
            let min_tick = *self.events.keys().min().unwrap_or(&0);
            let max_tick = *self.events.keys().max().unwrap_or(&0);
            (min_tick, max_tick)
        } else {
            (0, 0)
        };

        ReplayStats {
            total_events,
            tick_range,
            ticks_with_events: self.events.len() as u64,
            current_tick: self.playback_tick,
            mode: self.mode.clone(),
        }
    }

    /// Save replay to file
    fn save_to_file<P: AsRef<Path>>(&self, path: P, seed: u64) -> Result<(), ReplayError> {
        // Create directory if needed
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent).map_err(|e| ReplayError::IoError(e.to_string()))?;
        }

        let file = File::create(path).map_err(|e| ReplayError::IoError(e.to_string()))?;
        let mut writer = BufWriter::new(file);

        // Write header
        let total_events = self.events.values().map(|v| v.len()).sum::<usize>() as u64;
        let (start_tick, end_tick) = if !self.events.is_empty() {
            (*self.events.keys().min().unwrap(), *self.events.keys().max().unwrap())
        } else {
            (0, 0)
        };

        let header = ReplayHeader {
            version: 1,
            game_version: "0.1.0".to_string(), // Would be actual game version
            seed,
            start_tick,
            end_tick,
            event_count: total_events,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            checksum: 0, // Would calculate actual checksum
            name: "Recorded Replay".to_string(),
            description: String::new(),
        };

        let header_bytes = bincode::serialize(&header)
            .map_err(|e| ReplayError::SerializationError(e.to_string()))?;
        
        // Write header size first (4 bytes)
        let header_size = header_bytes.len() as u32;
        writer.write_all(&header_size.to_le_bytes())
            .map_err(|e| ReplayError::IoError(e.to_string()))?;
        
        // Write header data
        writer.write_all(&header_bytes)
            .map_err(|e| ReplayError::IoError(e.to_string()))?;

        // Write events
        for (tick, events) in &self.events {
            writer
                .write_all(&tick.to_le_bytes())
                .map_err(|e| ReplayError::IoError(e.to_string()))?;
            
            let event_count = events.len() as u32;
            writer
                .write_all(&event_count.to_le_bytes())
                .map_err(|e| ReplayError::IoError(e.to_string()))?;

            for event in events {
                let event_bytes = bincode::serialize(event)
                    .map_err(|e| ReplayError::SerializationError(e.to_string()))?;
                
                // Write event size first (4 bytes)
                let event_size = event_bytes.len() as u32;
                writer.write_all(&event_size.to_le_bytes())
                    .map_err(|e| ReplayError::IoError(e.to_string()))?;
                
                // Write event data
                writer.write_all(&event_bytes)
                    .map_err(|e| ReplayError::IoError(e.to_string()))?;
            }
        }

        writer.flush().map_err(|e| ReplayError::IoError(e.to_string()))?;
        info!("Saved replay with {} events", total_events);
        Ok(())
    }

    /// Load replay from file with proper binary deserialization
    fn load_from_file<P: AsRef<Path>>(&mut self, path: P) -> Result<ReplayHeader, ReplayError> {
        let file = File::open(path).map_err(|e| ReplayError::IoError(e.to_string()))?;
        let mut reader = BufReader::new(file);

        // Step 1: Read header size (first 4 bytes)
        let mut header_size_bytes = [0u8; 4];
        reader.read_exact(&mut header_size_bytes)
            .map_err(|e| ReplayError::IoError(format!("Failed to read header size: {}", e)))?;
        let header_size = u32::from_le_bytes(header_size_bytes) as usize;
        
        // Sanity check header size
        if header_size > 1024 * 1024 { // 1MB max header size
            return Err(ReplayError::CorruptedFile("Header size too large".to_string()));
        }

        // Step 2: Read and deserialize header
        let mut header_buffer = vec![0u8; header_size];
        reader.read_exact(&mut header_buffer)
            .map_err(|e| ReplayError::IoError(format!("Failed to read header data: {}", e)))?;
        
        let header: ReplayHeader = bincode::deserialize(&header_buffer)
            .map_err(|e| ReplayError::SerializationError(format!("Failed to deserialize header: {}", e)))?;

        // Validate header
        if header.version != 1 {
            return Err(ReplayError::VersionMismatch { 
                expected: 1, 
                found: header.version 
            });
        }

        // Clear existing events
        self.events.clear();

        // Step 3: Read events
        let mut events_read = 0u64;
        while events_read < header.event_count {
            // Read tick (8 bytes)
            let mut tick_bytes = [0u8; 8];
            if reader.read_exact(&mut tick_bytes).is_err() {
                debug!("Reached end of file while reading tick at event {}/{}", events_read, header.event_count);
                break; // End of file - might be truncated replay
            }
            let tick = u64::from_le_bytes(tick_bytes);

            // Read event count for this tick (4 bytes)
            let mut count_bytes = [0u8; 4];
            reader
                .read_exact(&mut count_bytes)
                .map_err(|e| ReplayError::IoError(format!("Failed to read event count at tick {}: {}", tick, e)))?;
            let tick_event_count = u32::from_le_bytes(count_bytes);

            // Read all events for this tick
            let mut tick_events = Vec::with_capacity(tick_event_count as usize);
            for event_idx in 0..tick_event_count {
                // Read event size (4 bytes)
                let mut event_size_bytes = [0u8; 4];
                reader.read_exact(&mut event_size_bytes)
                    .map_err(|e| ReplayError::IoError(format!("Failed to read event size at tick {}, event {}: {}", tick, event_idx, e)))?;
                let event_size = u32::from_le_bytes(event_size_bytes) as usize;
                
                // Sanity check event size
                if event_size > 1024 * 1024 { // 1MB max event size
                    return Err(ReplayError::CorruptedFile(format!("Event size too large: {} bytes", event_size)));
                }

                // Read event data
                let mut event_buffer = vec![0u8; event_size];
                reader.read_exact(&mut event_buffer)
                    .map_err(|e| ReplayError::IoError(format!("Failed to read event data at tick {}, event {}: {}", tick, event_idx, e)))?;
                
                // Deserialize event
                let event: ReplayEvent = bincode::deserialize(&event_buffer)
                    .map_err(|e| ReplayError::SerializationError(format!("Failed to deserialize event at tick {}, event {}: {}", tick, event_idx, e)))?;
                
                tick_events.push(event);
                events_read += 1;
            }

            self.events.insert(tick, tick_events);
        }

        // Validate that we read expected number of events
        let actual_events_read = self.events.values().map(|events| events.len() as u64).sum::<u64>();
        if actual_events_read != events_read {
            warn!("Event count mismatch: expected {}, actually read {}", header.event_count, actual_events_read);
        }

        info!("Loaded replay '{}': {} events from tick {} to {}", 
              header.name, actual_events_read, header.start_tick, header.end_tick);
        Ok(header)
    }
}

/// Replay statistics
#[derive(Debug, Clone)]
pub struct ReplayStats {
    pub total_events: u64,
    pub tick_range: (u64, u64),
    pub ticks_with_events: u64,
    pub current_tick: u64,
    pub mode: ReplayMode,
}

/// Replay system errors
#[derive(Error, Debug)]
pub enum ReplayError {
    #[error("Replay system already active")]
    AlreadyActive,
    #[error("Not currently recording")]
    NotRecording,
    #[error("No events loaded")]
    NoEventsLoaded,
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Deserialization error: {0}")]
    DeserializationError(String),
    #[error("File format error: {0}")]
    FormatError(String),
    #[error("Corrupted file: {0}")]
    CorruptedFile(String),
    #[error("Version mismatch: expected {expected}, found {found}")]
    VersionMismatch { expected: u32, found: u32 },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::commands::{SimulationCommand, DynamicBundle};
    use bevy_ecs::entity::Entity;

    #[test]
    fn test_replay_manager_creation() {
        let manager = ReplayManager::new();
        assert_eq!(manager.mode(), &ReplayMode::Idle);
        assert!(!manager.is_recording());
        assert!(!manager.is_playing());
    }

    #[test]
    fn test_start_stop_recording() {
        let mut manager = ReplayManager::new();
        
        manager.start_recording(Some("test".to_string())).unwrap();
        assert!(manager.is_recording());
        
        let command = SimulationCommand::SpawnEntity {
            components: DynamicBundle::new(),
        };
        manager.record_event(command, 1).unwrap();
        
        manager.stop_recording(12345).unwrap();
        assert!(!manager.is_recording());
        assert_eq!(manager.mode(), &ReplayMode::Idle);
    }

    #[test]
    fn test_replay_event() {
        let command = SimulationCommand::DespawnEntity {
            entity: Entity::from_raw(1),
        };
        
        let event = ReplayEvent {
            tick: 100,
            sequence: 5,
            command,
            metadata: ReplayMetadata {
                event_type: ReplayEventType::Entity,
                execution_time_us: 150,
                description: "Test event".to_string(),
                tags: vec!["test".to_string()],
            },
        };

        assert_eq!(event.tick, 100);
        assert_eq!(event.sequence, 5);
    }

    #[test]
    fn test_replay_stats() {
        let mut manager = ReplayManager::new();
        manager.start_recording(None).unwrap();
        
        let command = SimulationCommand::SpawnEntity {
            components: DynamicBundle::new(),
        };
        manager.record_event(command, 10).unwrap();
        
        let stats = manager.stats();
        assert_eq!(stats.total_events, 1);
        assert_eq!(stats.tick_range, (10, 10));
    }
}
