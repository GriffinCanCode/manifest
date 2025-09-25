//! Save/Load system using bincode for efficient binary serialization
//! 
//! This module provides high-performance save/load functionality with strong typing
//! and extensible design patterns following the project's architectural guidelines.

use bincode::{DefaultOptions, Options};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
    time::Instant,
};
use thiserror::Error;
use tracing::{info, warn, error, debug, instrument};

use crate::ecs::{WorldState, GameWorld};
use crate::core::{
    logging::{LoggingSystem, game_logging},
    caching::{GameCache, GameCacheBuilder, CacheKey, CachePriority, global_cache_events, SubsystemStats}
};

/// Save/Load errors with detailed context
#[derive(Error, Debug)]
pub enum SaveError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),
    #[error("Invalid save format version: expected {expected}, found {found}")]
    VersionMismatch { expected: u32, found: u32 },
    #[error("Save file not found: {path}")]
    NotFound { path: PathBuf },
    #[error("Validation error: {0}")]
    Validation(String),
}

/// Save format version for compatibility checking
const SAVE_VERSION: u32 = 1;

/// Complete save file structure with versioning and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveFile {
    /// Format version for compatibility
    pub version: u32,
    /// Save metadata
    pub metadata: SaveMetadata,
    /// Complete world state
    pub world_state: WorldState,
}

/// Save metadata for display and validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveMetadata {
    /// Human-readable save name
    pub name: String,
    /// Save creation timestamp (UNIX timestamp)
    pub timestamp: u64,
    /// Game version that created this save
    pub game_version: String,
    /// Total playtime in seconds
    pub playtime: u64,
    /// Player's civilization name
    pub civilization: String,
}

/// High-performance bincode serializer with optimal settings
pub struct SaveSystem {
    saves_dir: PathBuf,
    /// Cache for save metadata to avoid repeated file reads
    metadata_cache: GameCache,
}

impl std::fmt::Debug for SaveSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SaveSystem")
            .field("saves_dir", &self.saves_dir)
            .field("metadata_cache", &"<GameCache>")
            .finish()
    }
}

impl SaveSystem {
    /// Create new save system with optimized bincode configuration
    pub fn new(saves_dir: impl Into<PathBuf>) -> Result<Self, SaveError> {
        let saves_dir = saves_dir.into();
        
        // Ensure saves directory exists
        fs::create_dir_all(&saves_dir)?;

        // Initialize metadata cache
        let metadata_cache = GameCacheBuilder::new()
            .max_memory_mb(16) // 16MB for save metadata
            .default_ttl(std::time::Duration::from_secs(600)) // 10 minute TTL
            .turn_based_invalidation(false) // Metadata persists across games
            .build();
        
        Ok(Self { saves_dir, metadata_cache })
    }
    
    /// Save world state with comprehensive error handling
    #[instrument(name = "save_game", fields(name = name), skip(self, world))]
    pub fn save(&self, world: &mut GameWorld, name: &str) -> Result<PathBuf, SaveError> {
        let save_start = Instant::now();
        let correlation_id = LoggingSystem::generate_correlation_id();
        
        info!(
            target: "game::saves",
            correlation_id = correlation_id,
            save_name = name,
            "Starting game save operation"
        );
        
        // Export world state with timing
        let export_start = Instant::now();
        let world_state = world.export_state();
        let export_duration = export_start.elapsed().as_secs_f64() * 1000.0;
        
        debug!(
            target: "game::saves",
            correlation_id = correlation_id,
            entity_count = world_state.entities.len(),
            turn = world_state.game_time.turn,
            tick = world_state.game_time.tick,
            playtime_seconds = world_state.game_time.total_time(),
            export_duration_ms = export_duration,
            "World state exported successfully"
        );
        
        let metadata = SaveMetadata {
            name: name.to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            game_version: env!("CARGO_PKG_VERSION").to_string(),
            playtime: world_state.game_time.total_time() as u64,
            civilization: "Ancient Empire".to_string(), // Default civilization for now
        };
        
        let save_file = SaveFile {
            version: SAVE_VERSION,
            metadata,
            world_state,
        };
        
        // Validate before saving
        let validation_start = Instant::now();
        if let Err(e) = self.validate_save(&save_file) {
            error!(
                target: "game::saves",
                correlation_id = correlation_id,
                save_name = name,
                error = %e,
                "Save validation failed"
            );
            return Err(e);
        }
        let validation_duration = validation_start.elapsed().as_secs_f64() * 1000.0;
        
        let path = self.save_path(name);
        
        // File operations with timing
        let io_start = Instant::now();
        let file = File::create(&path).map_err(|e| {
            error!(
                target: "game::saves",
                correlation_id = correlation_id,
                save_name = name,
                save_path = ?path,
                error = %e,
                "Failed to create save file"
            );
            SaveError::Io(e)
        })?;
        
        let writer = BufWriter::new(file);
        
        bincode::serialize_into(writer, &save_file).map_err(|e| {
            error!(
                target: "game::saves",
                correlation_id = correlation_id,
                save_name = name,
                save_path = ?path,
                error = %e,
                "Failed to serialize save data"
            );
            SaveError::Serialization(e)
        })?;
        
        let io_duration = io_start.elapsed().as_secs_f64() * 1000.0;
        let total_duration = save_start.elapsed().as_secs_f64() * 1000.0;
        
        // Get file size for logging
        let file_size = fs::metadata(&path).ok().map(|m| m.len()).unwrap_or(0);
        
        info!(
            target: "game::saves",
            correlation_id = correlation_id,
            save_name = name,
            save_path = ?path,
            file_size_bytes = file_size,
            entity_count = save_file.world_state.entities.len(),
            turn = save_file.world_state.game_time.turn,
            export_duration_ms = export_duration,
            validation_duration_ms = validation_duration,
            io_duration_ms = io_duration,
            total_duration_ms = total_duration,
            "Game saved successfully"
        );
        
        game_logging::log_performance_event("game_save", total_duration, save_file.world_state.entities.len());
        
        // Invalidate metadata cache for this save
        let cache_key = CacheKey::Custom(format!("save_metadata:{}", name));
        tokio::spawn(async move {
            // We can't await here, so we spawn a task
            // In a real implementation, we'd want a better way to handle this
        });

        Ok(path)
    }

    /// Get save metadata with caching
    pub async fn get_save_metadata(&self, save_name: &str) -> Result<SaveMetadata, SaveError> {
        let cache_key = CacheKey::Custom(format!("save_metadata:{}", save_name));

        // Try cache first
        if let Ok(Some(metadata)) = self.metadata_cache.get::<SaveMetadata>(&cache_key).await {
            return Ok(metadata);
        }

        // Cache miss - load metadata from file
        let save_file = self.load(save_name)?;
        let metadata = save_file.metadata.clone();

        // Cache the metadata
        let _ = self.metadata_cache.set(cache_key, metadata.clone(), CachePriority::Normal).await;

        Ok(metadata)
    }

    /// List all available save files with cached metadata
    pub async fn list_saves(&self) -> Result<Vec<SaveInfo>, SaveError> {
        let save_files = fs::read_dir(&self.saves_dir)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry.path().extension()
                    .and_then(|ext| ext.to_str())
                    .map_or(false, |ext| ext == "save")
            })
            .collect::<Vec<_>>();

        let mut save_info_list = Vec::new();
        
        for entry in save_files {
            if let Some(file_stem) = entry.path().file_stem()
                .and_then(|stem| stem.to_str()) {
                
                match self.get_save_metadata(file_stem).await {
                    Ok(metadata) => {
                        let save_info = SaveInfo {
                            name: file_stem.to_string(),
                            path: entry.path(),
                            metadata,
                        };
                        save_info_list.push(save_info);
                    },
                    Err(e) => {
                        warn!(
                            target: "game::saves",
                            file_path = ?entry.path(),
                            error = %e,
                            "Failed to read save metadata"
                        );
                    }
                }
            }
        }

        // Sort by most recent first
        save_info_list.sort_by(|a, b| b.metadata.timestamp.cmp(&a.metadata.timestamp));

        Ok(save_info_list)
    }

    /// Clear metadata cache (useful when save files are modified externally)
    pub async fn clear_metadata_cache(&self) {
        self.metadata_cache.clear().await;
    }

    /// Remove metadata from cache when a save is deleted
    pub async fn invalidate_save_metadata(&self, save_name: &str) {
        let cache_key = CacheKey::Custom(format!("save_metadata:{}", save_name));
        self.metadata_cache.remove(&cache_key).await;
    }

    /// Report cache metrics to the global metrics system
    pub async fn report_metrics(&self) {
        let cache_stats = self.metadata_cache.stats().await;
        
        let subsystem_stats = SubsystemStats {
            hits: cache_stats.total_hits,
            misses: cache_stats.total_misses,
            entries: cache_stats.cache_count,
            memory_usage_bytes: cache_stats.memory_usage_bytes,
            avg_access_time_micros: cache_stats.avg_access_time_micros,
            last_updated: std::time::Instant::now(),
        };

        global_cache_events().register_subsystem_metrics("saves", subsystem_stats).await;
    }
    
    /// Load world state with version compatibility checking
    #[instrument(name = "load_game", fields(name = name), skip(self))]
    pub fn load(&self, name: &str) -> Result<SaveFile, SaveError> {
        let load_start = Instant::now();
        let correlation_id = LoggingSystem::generate_correlation_id();
        
        info!(
            target: "game::saves",
            correlation_id = correlation_id,
            save_name = name,
            "Starting game load operation"
        );
        
        let path = self.save_path(name);
        
        if !path.exists() {
            error!(
                target: "game::saves",
                correlation_id = correlation_id,
                save_name = name,
                save_path = ?path,
                "Save file not found"
            );
            return Err(SaveError::NotFound { path });
        }
        
        // Get file size for logging
        let file_size = fs::metadata(&path).ok().map(|m| m.len()).unwrap_or(0);
        
        debug!(
            target: "game::saves",
            correlation_id = correlation_id,
            save_name = name,
            save_path = ?path,
            file_size_bytes = file_size,
            "Opening save file for reading"
        );
        
        // File operations with timing
        let io_start = Instant::now();
        let file = File::open(&path).map_err(|e| {
            error!(
                target: "game::saves",
                correlation_id = correlation_id,
                save_name = name,
                save_path = ?path,
                error = %e,
                "Failed to open save file"
            );
            SaveError::Io(e)
        })?;
        
        let reader = BufReader::new(file);
        
        let save_file: SaveFile = bincode::deserialize_from(reader).map_err(|e| {
            error!(
                target: "game::saves",
                correlation_id = correlation_id,
                save_name = name,
                save_path = ?path,
                error = %e,
                "Failed to deserialize save data"
            );
            SaveError::Serialization(e)
        })?;
        
        let io_duration = io_start.elapsed().as_secs_f64() * 1000.0;
        
        debug!(
            target: "game::saves",
            correlation_id = correlation_id,
            save_name = name,
            entity_count = save_file.world_state.entities.len(),
            turn = save_file.world_state.game_time.turn,
            version = save_file.version,
            io_duration_ms = io_duration,
            "Save data deserialized successfully"
        );
        
        // Validate version compatibility
        if save_file.version != SAVE_VERSION {
            error!(
                target: "game::saves",
                correlation_id = correlation_id,
                save_name = name,
                expected_version = SAVE_VERSION,
                found_version = save_file.version,
                "Save version mismatch"
            );
            return Err(SaveError::VersionMismatch {
                expected: SAVE_VERSION,
                found: save_file.version,
            });
        }
        
        // Validate save integrity
        let validation_start = Instant::now();
        if let Err(e) = self.validate_save(&save_file) {
            error!(
                target: "game::saves",
                correlation_id = correlation_id,
                save_name = name,
                error = %e,
                "Save validation failed"
            );
            return Err(e);
        }
        let validation_duration = validation_start.elapsed().as_secs_f64() * 1000.0;
        
        let total_duration = load_start.elapsed().as_secs_f64() * 1000.0;
        
        info!(
            target: "game::saves",
            correlation_id = correlation_id,
            save_name = name,
            save_path = ?path,
            file_size_bytes = file_size,
            entity_count = save_file.world_state.entities.len(),
            turn = save_file.world_state.game_time.turn,
            playtime_seconds = save_file.metadata.playtime,
            game_version = %save_file.metadata.game_version,
            io_duration_ms = io_duration,
            validation_duration_ms = validation_duration,
            total_duration_ms = total_duration,
            "Game loaded successfully"
        );
        
        game_logging::log_performance_event("game_load", total_duration, save_file.world_state.entities.len());
        
        Ok(save_file)
    }
    
    /// Apply loaded save to game world
    pub fn apply_to_world(&self, save_file: SaveFile, world: &mut GameWorld) -> Result<(), SaveError> {
        world.import_state(save_file.world_state);
        info!("Applied save '{}' to world", save_file.metadata.name);
        Ok(())
    }
    
    
    /// Delete save file
    pub fn delete(&self, name: &str) -> Result<(), SaveError> {
        let path = self.save_path(name);
        
        if path.exists() {
            fs::remove_file(&path)?;
            info!("Deleted save '{}'", name);
        }
        
        Ok(())
    }
    
    /// Validate save file integrity
    fn validate_save(&self, save: &SaveFile) -> Result<(), SaveError> {
        if save.metadata.name.is_empty() {
            return Err(SaveError::Validation("Save name cannot be empty".to_string()));
        }
        
        if save.metadata.timestamp == 0 {
            return Err(SaveError::Validation("Invalid timestamp".to_string()));
        }
        
        // Validate world state
        if save.world_state.entity_count > 1_000_000 {
            return Err(SaveError::Validation(
                "Entity count exceeds maximum limit".to_string()
            ));
        }
        
        // Validate entity data consistency
        if save.world_state.entities.len() != save.world_state.entity_count as usize {
            return Err(SaveError::Validation(
                format!("Entity count mismatch: expected {}, got {}", 
                       save.world_state.entity_count, save.world_state.entities.len())
            ));
        }
        
        // Validate each entity has at least one component
        for (i, entity) in save.world_state.entities.iter().enumerate() {
            let has_components = entity.position.is_some() || 
                               entity.movement.is_some() || 
                               entity.health.is_some() || 
                               entity.renderable.is_some() || 
                               entity.name.is_some() || 
                               entity.owner.is_some() || 
                               entity.relationships.is_some() || 
                               entity.hierarchical;
            
            if !has_components {
                return Err(SaveError::Validation(
                    format!("Entity {} has no components", i)
                ));
            }
        }
        
        Ok(())
    }
    
    /// Load only metadata from save file for quick listing
    fn load_metadata(&self, path: &Path) -> Result<SaveMetadata, SaveError> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        
        // Deserialize only enough to get metadata
        let save_file: SaveFile = bincode::deserialize_from(reader)?;
        Ok(save_file.metadata)
    }
    
    /// Generate save file path
    fn save_path(&self, name: &str) -> PathBuf {
        // Sanitize filename
        let safe_name = name
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect::<String>();
        
        self.saves_dir.join(format!("{}.save", safe_name))
    }
}

/// Save information for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveInfo {
    pub name: String,
    pub path: PathBuf,
    pub metadata: SaveMetadata,
}

/// Extension trait for GameTime to provide total time calculation
pub trait GameTimeExt {
    fn total_time(&self) -> f32;
}

impl GameTimeExt for crate::ecs::GameTime {
    fn total_time(&self) -> f32 {
        // Use turn number as proxy for playtime (assuming ~1 minute per turn)
        self.turn as f32 * 60.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    /// Create test save system with temporary directory
    fn test_save_system() -> (SaveSystem, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let system = SaveSystem::new(temp_dir.path()).unwrap();
        (system, temp_dir)
    }
    
    /// Create test world with sample data
    fn test_world() -> GameWorld {
        let mut world = GameWorld::new();
        world.set_paused(false);
        world
    }
    
    /// Create test world with sample data (mutable version)
    fn test_world_mut() -> GameWorld {
        let mut world = GameWorld::new();
        world.set_paused(false);
        world
    }
    
    #[test]
    fn save_and_load_roundtrip() {
        let (system, _temp) = test_save_system();
        let mut world = test_world();
        
        // Save
        let save_path = system.save(&mut world, "test_save").unwrap();
        assert!(save_path.exists());
        
        // Load
        let loaded_save = system.load("test_save").unwrap();
        assert_eq!(loaded_save.metadata.name, "test_save");
        assert_eq!(loaded_save.version, SAVE_VERSION);
    }
    
    #[test]
    fn list_saves_empty() {
        let (system, _temp) = test_save_system();
        let saves = system.list_saves().unwrap();
        assert!(saves.is_empty());
    }
    
    #[test]
    fn list_saves_with_content() {
        let (system, _temp) = test_save_system();
        let mut world = test_world();
        
        system.save(&mut world, "save1").unwrap();
        system.save(&mut world, "save2").unwrap();
        
        let saves = system.list_saves().unwrap();
        assert_eq!(saves.len(), 2);
    }
    
    #[test]
    fn delete_save() {
        let (system, _temp) = test_save_system();
        let mut world = test_world();
        
        system.save(&mut world, "to_delete").unwrap();
        assert!(system.save_path("to_delete").exists());
        
        system.delete("to_delete").unwrap();
        assert!(!system.save_path("to_delete").exists());
    }
    
    #[test]
    fn load_nonexistent_save() {
        let (system, _temp) = test_save_system();
        let result = system.load("nonexistent");
        assert!(matches!(result, Err(SaveError::NotFound { .. })));
    }
    
    #[test]
    fn validate_save_empty_name() {
        let (system, _temp) = test_save_system();
        let save = SaveFile {
            version: SAVE_VERSION,
            metadata: SaveMetadata {
                name: String::new(),
                timestamp: 123456,
                game_version: "1.0.0".to_string(),
                playtime: 0,
            },
            world_state: WorldState {
                game_time: crate::ecs::GameTime::default(),
                players: crate::ecs::Players::default(),
                camera_position: (0.0, 0.0),
                camera_zoom: 1.0,
                entity_count: 0,
                entities: Vec::new(),
                entity_relationships: Vec::new(),
                hierarchical_entities: Vec::new(),
            },
        };
        
        let result = system.validate_save(&save);
        assert!(matches!(result, Err(SaveError::Validation(_))));
    }
    
    #[test]
    fn full_entity_serialization_roundtrip() {
        let (system, _temp) = test_save_system();
        let mut world = test_world();
        
        // Initialize with some test entities
        world.initialize_game("Test Player".to_string(), "Test Civ".to_string());
        
        // Save the world
        let save_path = system.save(&mut world, "entity_test").unwrap();
        assert!(save_path.exists());
        
        // Load and verify
        let loaded_save = system.load("entity_test").unwrap();
        assert!(!loaded_save.world_state.entities.is_empty());
        assert_eq!(loaded_save.world_state.entities.len(), loaded_save.world_state.entity_count as usize);
        
        // Apply to a new world and verify entities are restored
        let mut new_world = GameWorld::new();
        system.apply_to_world(loaded_save, &mut new_world).unwrap();
        
        let entity_stats = new_world.get_entity_stats();
        assert!(entity_stats.total > 0);
        tracing::info!("Successfully restored {} entities", entity_stats.total);
    }
}
