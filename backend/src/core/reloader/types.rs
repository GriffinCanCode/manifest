//! Core types for the hot reload system
//!
//! Simple, focused types following the existing codebase patterns.

use std::{path::PathBuf, time::SystemTime};
use thiserror::Error;

/// Hot reload errors
#[derive(Error, Debug, Clone)]
pub enum ReloadError {
    #[error("File watch failed: {reason}")]
    WatchFailed { reason: String },
    #[error("File not found: {path}")]
    FileNotFound { path: String },
    #[error("Reload failed: {reason}")]
    Failed { reason: String },
}

/// Result type for reload operations
pub type ReloadResult<T = ()> = Result<T, ReloadError>;

/// Events emitted during hot reloading
#[derive(Debug, Clone)]
pub enum ReloadEvent {
    /// File changed on disk
    FileChanged { path: PathBuf },
    /// Successfully reloaded
    Reloaded { path: PathBuf, handler: String },
    /// Reload failed
    Failed { path: PathBuf, error: String },
}

/// File categories we can hot reload
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    Lua,
    Config,
    Asset,
}

/// Metadata for tracked files
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: PathBuf,
    pub file_type: FileType,
    pub last_modified: SystemTime,
}

impl FileInfo {
    pub fn new(path: PathBuf, file_type: FileType) -> ReloadResult<Self> {
        let metadata = std::fs::metadata(&path).map_err(|_| ReloadError::FileNotFound {
            path: path.to_string_lossy().to_string(),
        })?;

        Ok(Self {
            path,
            file_type,
            last_modified: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
        })
    }

    pub fn has_changed(&mut self) -> bool {
        if let Ok(metadata) = std::fs::metadata(&self.path) {
            if let Ok(modified) = metadata.modified() {
                if modified > self.last_modified {
                    self.last_modified = modified;
                    return true;
                }
            }
        }
        false
    }
}

/// Trait for handling specific file types
pub trait ReloadHandler: Send + Sync {
    /// Name of this handler
    fn name(&self) -> &'static str;
    
    /// Check if this handler can process the file
    fn handles(&self, path: &PathBuf) -> bool;
    
    /// Reload the file
    fn reload(&mut self, path: &PathBuf) -> ReloadResult<()>;
}
