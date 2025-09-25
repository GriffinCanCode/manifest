//! Hot reload functionality for development builds
//!
//! Contains hot reload setup and management for live development.

#[cfg(debug_assertions)]
use crate::core::reloader::*;

use super::core::GameWorld;

impl GameWorld {
    /// Setup hot reload system for development builds
    #[cfg(debug_assertions)]
    pub(super) fn setup_reloader() -> Option<ReloadManager> {
        use std::path::Path;
        
        match ReloadManager::new() {
            Ok(mut manager) => {
                // Add default handlers
                manager.add_handler(Box::new(LuaHandler::new().expect("Failed to initialize Lua handler for hot reload")));
                manager.add_handler(Box::new(ConfigHandler::new()));
                manager.add_handler(Box::new(AssetHandler::new()));

                // Watch common script/config directories
                let watch_dirs = [
                    "lua-scripts",
                    "configs", 
                    "assets",
                    "backend/src",  // For system files (informational only)
                ];

                for dir in &watch_dirs {
                    let path = Path::new(dir);
                    if path.exists() {
                        Self::watch_directory_recursive(&mut manager, path);
                    }
                }

                // Start the reloader
                if manager.start().is_ok() {
                    tracing::info!("🔥 Hot reload system activated");
                    Some(manager)
                } else {
                    tracing::warn!("Failed to start hot reload system");
                    None
                }
            }
            Err(e) => {
                tracing::warn!("Hot reload system disabled: {}", e);
                None
            }
        }
    }

    /// Watch directory recursively for file changes
    #[cfg(debug_assertions)]
    fn watch_directory_recursive(manager: &mut ReloadManager, path: &std::path::Path) {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if entry_path.is_file() {
                    let file_type = match entry_path.extension().and_then(|ext| ext.to_str()) {
                        Some("lua") => Some(FileType::Lua),
                        Some("toml" | "json" | "yaml" | "yml") => Some(FileType::Config),
                        Some("png" | "jpg" | "wav" | "glb") => Some(FileType::Asset),
                        _ => None,
                    };
                    
                    if let Some(ft) = file_type {
                        let _ = manager.watch_file(entry_path, ft);
                    }
                } else if entry_path.is_dir() {
                    Self::watch_directory_recursive(manager, &entry_path);
                }
            }
        }
    }

    /// Get hot reload statistics (debug builds only)
    #[cfg(debug_assertions)]
    pub fn reload_stats(&self) -> Option<ReloadStats> {
        self.reload_manager().as_ref().map(|m| m.stats())
    }

    /// Access Lua handler for direct script execution (debug builds only)
    #[cfg(debug_assertions)]
    pub fn lua_handler(&self) -> Option<std::sync::Arc<parking_lot::Mutex<mlua::Lua>>> {
        self.reload_manager().as_ref().and_then(|manager| {
            // Get the handlers from the manager and find the Lua handler
            manager.with_handlers(|handlers| {
                handlers.iter().find_map(|handler| {
                    if handler.name() == "lua" {
                        // Downcast to LuaHandler to access the lua() method
                        handler.as_any().downcast_ref::<crate::core::reloader::LuaHandler>()
                            .map(|lua_handler| lua_handler.lua())
                    } else {
                        None
                    }
                })
            })
        })
    }

    /// Execute a Lua script directly (debug builds only)
    #[cfg(debug_assertions)]
    pub fn execute_lua_script(&self, script: &str) -> Result<String, String> {
        if let Some(lua_arc) = self.lua_handler() {
            let lua = lua_arc.lock();
            match lua.load(script).eval::<String>() {
                Ok(result) => Ok(result),
                Err(e) => Err(format!("Lua execution error: {}", e)),
            }
        } else {
            Err("Lua handler not available".to_string())
        }
    }

    /// Reload a specific file manually (debug builds only)
    #[cfg(debug_assertions)]
    pub fn reload_file(&mut self, path: &std::path::Path) -> Result<(), String> {
        if let Some(ref mut manager) = self.reload_manager_mut() {
            manager.reload_file_manually(path)
                .map_err(|e| format!("Failed to reload file: {}", e))
        } else {
            Err("Hot reload manager not available".to_string())
        }
    }

    /// Get list of currently watched files (debug builds only)
    #[cfg(debug_assertions)]
    pub fn watched_files(&self) -> Vec<std::path::PathBuf> {
        if let Some(ref manager) = self.reload_manager() {
            manager.watched_files()
        } else {
            Vec::new()
        }
    }
}

// Stub implementations for release builds
#[cfg(not(debug_assertions))]
impl GameWorld {
    pub(super) fn setup_reloader() -> Option<()> {
        None
    }
}
