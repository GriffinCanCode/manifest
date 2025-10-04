//! Input validation and error handling for IPC commands
//! 
//! Provides consistent validation and error responses that match frontend schemas

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use tracing::error;

/// Standardized error response for IPC commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IPCError {
    pub code: String,
    pub message: String,
    pub details: Option<HashMap<String, serde_json::Value>>,
    pub correlation_id: Option<String>,
}

/// Error types for IPC operations
#[derive(Error, Debug)]
pub enum IPCErrorType {
    #[error("Validation error: {message}")]
    Validation { message: String, field: Option<String> },
    
    #[error("Resource not found: {resource}")]
    NotFound { resource: String },
    
    #[error("Permission denied: {operation}")]
    PermissionDenied { operation: String },
    
    #[error("Internal server error: {message}")]
    Internal { message: String },
    
    #[error("Database error: {message}")]
    Database { message: String },
    
    #[error("File system error: {message}")]
    FileSystem { message: String },
    
    #[error("Serialization error: {message}")]
    Serialization { message: String },
    
    #[error("Timeout error: {message}")]
    Timeout { message: String },
    
    #[error("Rate limit exceeded: {message}")]
    RateLimit { message: String },
}

impl IPCErrorType {
    /// Convert to standardized IPC error response
    pub fn to_ipc_error(&self, correlation_id: Option<String>) -> IPCError {
        let (code, message, details) = match self {
            IPCErrorType::Validation { message, field } => {
                let mut details = HashMap::new();
                if let Some(field) = field {
                    details.insert("field".to_string(), serde_json::Value::String(field.clone()));
                }
                ("VALIDATION_ERROR".to_string(), message.clone(), if details.is_empty() { None } else { Some(details) })
            },
            IPCErrorType::NotFound { resource } => {
                let mut details = HashMap::new();
                details.insert("resource".to_string(), serde_json::Value::String(resource.clone()));
                ("NOT_FOUND".to_string(), format!("Resource '{}' not found", resource), Some(details))
            },
            IPCErrorType::PermissionDenied { operation } => {
                let mut details = HashMap::new();
                details.insert("operation".to_string(), serde_json::Value::String(operation.clone()));
                ("PERMISSION_DENIED".to_string(), format!("Permission denied for operation: {}", operation), Some(details))
            },
            IPCErrorType::Internal { message } => {
                ("INTERNAL_ERROR".to_string(), message.clone(), None)
            },
            IPCErrorType::Database { message } => {
                ("DATABASE_ERROR".to_string(), format!("Database error: {}", message), None)
            },
            IPCErrorType::FileSystem { message } => {
                ("FILESYSTEM_ERROR".to_string(), format!("File system error: {}", message), None)
            },
            IPCErrorType::Serialization { message } => {
                ("SERIALIZATION_ERROR".to_string(), format!("Serialization error: {}", message), None)
            },
            IPCErrorType::Timeout { message } => {
                ("TIMEOUT_ERROR".to_string(), format!("Operation timed out: {}", message), None)
            },
            IPCErrorType::RateLimit { message } => {
                ("RATE_LIMIT_ERROR".to_string(), format!("Rate limit exceeded: {}", message), None)
            },
        };

        IPCError {
            code,
            message,
            details,
            correlation_id,
        }
    }

    /// Convert to JSON string for Tauri command errors
    pub fn to_error_string(&self, correlation_id: Option<String>) -> String {
        let ipc_error = self.to_ipc_error(correlation_id);
        serde_json::to_string(&ipc_error).unwrap_or_else(|_| self.to_string())
    }
}

/// Input validation utilities
pub struct Validator;

impl Validator {
    /// Validate player name
    pub fn validate_player_name(name: &str) -> Result<(), IPCErrorType> {
        if name.is_empty() {
            return Err(IPCErrorType::Validation {
                message: "Player name cannot be empty".to_string(),
                field: Some("player_name".to_string()),
            });
        }

        if name.len() > 50 {
            return Err(IPCErrorType::Validation {
                message: "Player name cannot be longer than 50 characters".to_string(),
                field: Some("player_name".to_string()),
            });
        }

        // Check for invalid characters
        if name.chars().any(|c| !c.is_alphanumeric() && c != ' ' && c != '-' && c != '_') {
            return Err(IPCErrorType::Validation {
                message: "Player name can only contain alphanumeric characters, spaces, hyphens, and underscores".to_string(),
                field: Some("player_name".to_string()),
            });
        }

        Ok(())
    }

    /// Validate civilization name
    pub fn validate_civilization_name(name: &str) -> Result<(), IPCErrorType> {
        if name.is_empty() {
            return Err(IPCErrorType::Validation {
                message: "Civilization name cannot be empty".to_string(),
                field: Some("civilization".to_string()),
            });
        }

        if name.len() > 50 {
            return Err(IPCErrorType::Validation {
                message: "Civilization name cannot be longer than 50 characters".to_string(),
                field: Some("civilization".to_string()),
            });
        }

        Ok(())
    }

    /// Validate save name
    pub fn validate_save_name(name: &str) -> Result<(), IPCErrorType> {
        if name.is_empty() {
            return Err(IPCErrorType::Validation {
                message: "Save name cannot be empty".to_string(),
                field: Some("save_name".to_string()),
            });
        }

        if name.len() > 100 {
            return Err(IPCErrorType::Validation {
                message: "Save name cannot be longer than 100 characters".to_string(),
                field: Some("save_name".to_string()),
            });
        }

        // Check for invalid filename characters
        let invalid_chars = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
        if name.chars().any(|c| invalid_chars.contains(&c)) {
            return Err(IPCErrorType::Validation {
                message: "Save name contains invalid characters".to_string(),
                field: Some("save_name".to_string()),
            });
        }

        Ok(())
    }

    /// Validate tile streaming request
    pub fn validate_tile_streaming_request(request: &crate::commands::tile_streaming::TileStreamingRequest) -> Result<(), IPCErrorType> {
        if request.view_radius <= 0.0 {
            return Err(IPCErrorType::Validation {
                message: "View radius must be positive".to_string(),
                field: Some("view_radius".to_string()),
            });
        }

        if request.max_tiles == 0 {
            return Err(IPCErrorType::Validation {
                message: "Max tiles must be greater than 0".to_string(),
                field: Some("max_tiles".to_string()),
            });
        }

        if request.max_tiles > 25000 {
            return Err(IPCErrorType::Validation {
                message: "Max tiles cannot exceed 25000 for performance reasons".to_string(),
                field: Some("max_tiles".to_string()),
            });
        }

        // Validate LOD levels
        for level in &request.lod_levels {
            if *level > 5 {
                return Err(IPCErrorType::Validation {
                    message: "LOD levels cannot exceed 5".to_string(),
                    field: Some("lod_levels".to_string()),
                });
            }
        }

        Ok(())
    }

    /// Validate tile ID
    pub fn validate_tile_id(tile_id: u32) -> Result<(), IPCErrorType> {
        if tile_id == 0 {
            return Err(IPCErrorType::Validation {
                message: "Tile ID cannot be 0".to_string(),
                field: Some("tile_id".to_string()),
            });
        }

        Ok(())
    }
}

/// Result type for IPC operations
pub type IPCResult<T> = Result<T, IPCErrorType>;

/// Trait for converting various error types to IPC errors
pub trait ToIPCError {
    fn to_ipc_error(&self, correlation_id: Option<String>) -> IPCErrorType;
}

impl ToIPCError for std::io::Error {
    fn to_ipc_error(&self, _correlation_id: Option<String>) -> IPCErrorType {
        match self.kind() {
            std::io::ErrorKind::NotFound => IPCErrorType::NotFound {
                resource: "file".to_string(),
            },
            std::io::ErrorKind::PermissionDenied => IPCErrorType::PermissionDenied {
                operation: "file operation".to_string(),
            },
            std::io::ErrorKind::TimedOut => IPCErrorType::Timeout {
                message: self.to_string(),
            },
            _ => IPCErrorType::FileSystem {
                message: self.to_string(),
            },
        }
    }
}

impl ToIPCError for serde_json::Error {
    fn to_ipc_error(&self, _correlation_id: Option<String>) -> IPCErrorType {
        IPCErrorType::Serialization {
            message: self.to_string(),
        }
    }
}

impl ToIPCError for bincode::Error {
    fn to_ipc_error(&self, _correlation_id: Option<String>) -> IPCErrorType {
        IPCErrorType::Serialization {
            message: self.to_string(),
        }
    }
}

/// Macro for easy error handling in commands
#[macro_export]
macro_rules! ipc_try {
    ($expr:expr, $correlation_id:expr) => {
        match $expr {
            Ok(val) => val,
            Err(err) => {
                let ipc_err = crate::commands::validation::ToIPCError::to_ipc_error(&err, $correlation_id.clone());
                error!("IPC command error: {}", ipc_err);
                return Err(ipc_err.to_error_string($correlation_id));
            }
        }
    };
}

/// Macro for creating validation errors quickly
#[macro_export]
macro_rules! validation_error {
    ($message:expr) => {
        crate::commands::validation::IPCErrorType::Validation {
            message: $message.to_string(),
            field: None,
        }
    };
    ($message:expr, $field:expr) => {
        crate::commands::validation::IPCErrorType::Validation {
            message: $message.to_string(),
            field: Some($field.to_string()),
        }
    };
}

/// Macro for creating not found errors quickly
#[macro_export]
macro_rules! not_found_error {
    ($resource:expr) => {
        crate::commands::validation::IPCErrorType::NotFound {
            resource: $resource.to_string(),
        }
    };
}
