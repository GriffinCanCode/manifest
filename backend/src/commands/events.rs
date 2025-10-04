//! Event system for frontend notifications
//! 
//! Handles emitting events to the frontend for reactive updates

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tracing::{debug, error, warn};
use std::time::{SystemTime, UNIX_EPOCH};

/// Game state change event
#[derive(Debug, Clone, Serialize)]
pub struct GameStateChangedEvent {
    pub state: crate::commands::GameState,
    pub timestamp: u64,
}

/// Tile update event
#[derive(Debug, Clone, Serialize)]
pub struct TileUpdatedEvent {
    pub tile_ids: Vec<u32>,
    pub timestamp: u64,
}

/// Error occurred event
#[derive(Debug, Clone, Serialize)]
pub struct ErrorOccurredEvent {
    pub command: String,
    pub error: String,
    pub correlation_id: Option<String>,
    pub timestamp: u64,
}

/// Performance warning event
#[derive(Debug, Clone, Serialize)]
pub struct PerformanceWarningEvent {
    pub metric: String,
    pub value: f64,
    pub threshold: f64,
    pub timestamp: u64,
}

/// Notification event
#[derive(Debug, Clone, Serialize)]
pub struct NotificationEvent {
    pub id: String,
    #[serde(rename = "type")]
    pub notification_type: String, // info, success, warning, error
    pub title: String,
    pub message: String,
    pub duration: Option<u64>,
    pub timestamp: u64,
}

/// Event emitter for the IPC system
pub struct IPCEventEmitter {
    app_handle: AppHandle,
}

impl IPCEventEmitter {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }

    /// Emit a game state changed event
    pub fn game_state_changed(&self, state: crate::commands::GameState) {
        let event = GameStateChangedEvent {
            state,
            timestamp: current_timestamp(),
        };

        if let Err(e) = self.app_handle.emit("game_state_changed", &event) {
            error!("Failed to emit game_state_changed event: {}", e);
        } else {
            debug!("Emitted game_state_changed event");
        }
    }

    /// Emit a tile updated event
    pub fn tile_updated(&self, tile_ids: Vec<u32>) {
        let event = TileUpdatedEvent {
            tile_ids: tile_ids.clone(),
            timestamp: current_timestamp(),
        };

        if let Err(e) = self.app_handle.emit("tile_updated", &event) {
            error!("Failed to emit tile_updated event: {}", e);
        } else {
            debug!("Emitted tile_updated event for {} tiles", tile_ids.len());
        }
    }

    /// Emit an error occurred event
    pub fn error_occurred(&self, command: String, error: String, correlation_id: Option<String>) {
        let event = ErrorOccurredEvent {
            command: command.clone(),
            error: error.clone(),
            correlation_id,
            timestamp: current_timestamp(),
        };

        if let Err(e) = self.app_handle.emit("error_occurred", &event) {
            error!("Failed to emit error_occurred event: {}", e);
        } else {
            debug!("Emitted error_occurred event for command: {}", command);
        }
    }

    /// Emit a performance warning event
    pub fn performance_warning(&self, metric: String, value: f64, threshold: f64) {
        let event = PerformanceWarningEvent {
            metric: metric.clone(),
            value,
            threshold,
            timestamp: current_timestamp(),
        };

        if let Err(e) = self.app_handle.emit("performance_warning", &event) {
            error!("Failed to emit performance_warning event: {}", e);
        } else {
            warn!("Emitted performance warning: {} = {} (threshold: {})", metric, value, threshold);
        }
    }

    /// Emit a notification event
    pub fn notification(&self, notification_type: &str, title: String, message: String, duration: Option<u64>) {
        let event = NotificationEvent {
            id: generate_id(),
            notification_type: notification_type.to_string(),
            title,
            message,
            duration,
            timestamp: current_timestamp(),
        };

        if let Err(e) = self.app_handle.emit("notification", &event) {
            error!("Failed to emit notification event: {}", e);
        } else {
            debug!("Emitted {} notification: {}", notification_type, event.title);
        }
    }

    /// Emit a success notification
    pub fn success(&self, title: String, message: String) {
        self.notification("success", title, message, Some(3000));
    }

    /// Emit an error notification
    pub fn error(&self, title: String, message: String) {
        self.notification("error", title, message, Some(8000));
    }

    /// Emit a warning notification
    pub fn warning(&self, title: String, message: String) {
        self.notification("warning", title, message, Some(6000));
    }

    /// Emit an info notification
    pub fn info(&self, title: String, message: String) {
        self.notification("info", title, message, Some(4000));
    }

    /// Emit command started event (for debugging)
    pub fn command_started(&self, command_id: String, command_name: String) {
        let event = serde_json::json!({
            "commandId": command_id,
            "name": command_name,
            "timestamp": current_timestamp()
        });

        if let Err(e) = self.app_handle.emit("command_started", &event) {
            error!("Failed to emit command_started event: {}", e);
        }
    }

    /// Emit command completed event (for debugging)
    pub fn command_completed(&self, command_id: String, command_name: String, duration: u64) {
        let event = serde_json::json!({
            "commandId": command_id,
            "name": command_name,
            "duration": duration,
            "timestamp": current_timestamp()
        });

        if let Err(e) = self.app_handle.emit("command_completed", &event) {
            error!("Failed to emit command_completed event: {}", e);
        }
    }

    /// Emit command failed event (for debugging)
    pub fn command_failed(&self, command_id: String, command_name: String, error: String) {
        let event = serde_json::json!({
            "commandId": command_id,
            "name": command_name,
            "error": error,
            "timestamp": current_timestamp()
        });

        if let Err(e) = self.app_handle.emit("command_failed", &event) {
            error!("Failed to emit command_failed event: {}", e);
        }
    }

    /// Emit batch completed event
    pub fn batch_completed(&self, batch_id: String, command_count: usize, duration: u64, success_count: usize) {
        let event = serde_json::json!({
            "batchId": batch_id,
            "commandCount": command_count,
            "duration": duration,
            "successCount": success_count,
            "timestamp": current_timestamp()
        });

        if let Err(e) = self.app_handle.emit("batch_completed", &event) {
            error!("Failed to emit batch_completed event: {}", e);
        }
    }
}

/// Get current timestamp in milliseconds
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Generate a unique ID
fn generate_id() -> String {
    format!("{}", current_timestamp())
}
