//! Comprehensive Lua scripting system for game logic
//!
//! Provides full mlua integration with sandbox environments, hot reloading,
//! event callbacks, and comprehensive API access to game systems.

pub mod api;
pub mod entities;  
pub mod events;
pub mod utils;

use std::{path::{Path, PathBuf}, sync::Arc, collections::HashMap, time::SystemTime};
use crossbeam::channel::Receiver;
use bevy_ecs::prelude::*;
use mlua::{Lua, LuaOptions, StdLib, Value, Function, Error as LuaError};
use tracing::{info, warn, debug, error};
use thiserror::Error;
use parking_lot::RwLock;

use crate::core::reloader::{ReloadManager, ReloadHandler, ReloadEvent, ReloadResult, ReloadError, FileType};
use crate::core::hashing::HashStrategies;

pub use api::*;
pub use entities::*;
pub use events::*;
pub use utils::*;

/// Errors that can occur in the scripting system
#[derive(Error, Debug)]
pub enum ScriptError {
    #[error("Script file not found: {path}")]
    FileNotFound { path: PathBuf },
    
    #[error("Script compilation failed: {reason}")]
    CompilationFailed { reason: String },
    
    #[error("Script execution failed: {reason}")]  
    ExecutionFailed { reason: String },
    
    #[error("API binding failed: {reason}")]
    BindingFailed { reason: String },
    
    #[error("Sandboxing failed: {reason}")]
    SandboxingFailed { reason: String },
}

impl From<LuaError> for ScriptError {
    fn from(error: LuaError) -> Self {
        match error {
            LuaError::SyntaxError { message, .. } => ScriptError::CompilationFailed {
                reason: message,
            },
            LuaError::RuntimeError(msg) => ScriptError::ExecutionFailed {
                reason: msg,
            },
            _ => ScriptError::ExecutionFailed {
                reason: error.to_string(),
            },
        }
    }
}

impl From<std::io::Error> for ScriptError {
    fn from(error: std::io::Error) -> Self {
        ScriptError::FileNotFound {
            path: PathBuf::from(error.to_string()),
        }
    }
}

pub type ScriptResult<T> = Result<T, ScriptError>;

/// Thread-safe Lua execution environment that creates VMs on demand
/// This avoids mlua thread safety issues by not storing the Lua instance
pub struct LuaEnvironment {
    /// Loaded scripts with their metadata  
    scripts: HashMap<PathBuf, ScriptInfo>,
    /// Event callbacks with priority
    event_callbacks: HashMap<String, Vec<LuaCallback>>,
    /// Sandbox restrictions enabled
    sandbox_enabled: bool,
}

unsafe impl Send for LuaEnvironment {}
unsafe impl Sync for LuaEnvironment {}

/// Information about a loaded script with hash-based change detection
#[derive(Debug, Clone)]
pub struct ScriptInfo {
    pub path: PathBuf,
    pub content: String,
    pub content_hash: u64, // Using FastHasher for efficient change detection
    pub last_modified: SystemTime,
    pub loaded_at: SystemTime,
    pub error_count: u32,
    pub execution_count: u64,
}

impl ScriptInfo {
    /// Create new script info with hash
    pub fn new(path: PathBuf, content: String) -> ScriptResult<Self> {
        let metadata = std::fs::metadata(&path)?;
        let last_modified = metadata.modified().unwrap_or_else(|_| SystemTime::now());
        let content_hash = HashStrategies::hash_string(&content);
        
        Ok(Self {
            path,
            content,
            content_hash,
            last_modified,
            loaded_at: SystemTime::now(),
            error_count: 0,
            execution_count: 0,
        })
    }

    /// Check if content has changed using hash comparison
    pub fn has_content_changed(&self, new_content: &str) -> bool {
        let new_hash = HashStrategies::hash_string(new_content);
        self.content_hash != new_hash
    }

    /// Update content and hash
    pub fn update_content(&mut self, new_content: String) -> ScriptResult<()> {
        self.content_hash = HashStrategies::hash_string(&new_content);
        self.content = new_content;
        self.last_modified = SystemTime::now();
        Ok(())
    }
}

/// Lua callback with priority and metadata
#[derive(Debug, Clone)]
pub struct LuaCallback {
    pub function_name: String,
    pub priority: i32,
    pub script_path: PathBuf,
}

impl LuaEnvironment {
    /// Create new Lua environment with sandbox restrictions
    pub fn new_sandboxed() -> ScriptResult<Self> {
        let lua = Lua::new_with(
            StdLib::TABLE | StdLib::STRING | StdLib::MATH | StdLib::UTF8,
            LuaOptions::new().catch_rust_panics(true),
        )?;

        // Set up sandbox environment
        Self::setup_sandbox(&lua)?;

        Ok(Self {
            scripts: HashMap::new(),
            event_callbacks: HashMap::new(),
            sandbox_enabled: true,
        })
    }

    /// Create new unrestricted Lua environment
    pub fn new_unrestricted() -> ScriptResult<Self> {
        let lua = Lua::new_with(
            StdLib::ALL_SAFE,
            LuaOptions::new().catch_rust_panics(true),
        )?;

        Ok(Self {
            scripts: HashMap::new(),
            event_callbacks: HashMap::new(),
            sandbox_enabled: false,
        })
    }

    /// Set up sandbox restrictions
    fn setup_sandbox(lua: &Lua) -> ScriptResult<()> {
        let globals = lua.globals();

        // Remove dangerous functions
        globals.set("os", Value::Nil)?;
        globals.set("io", Value::Nil)?;
        globals.set("package", Value::Nil)?;
        globals.set("require", Value::Nil)?;
        globals.set("dofile", Value::Nil)?;
        globals.set("loadfile", Value::Nil)?;

        // Restrict debug
        globals.set("debug", Value::Nil)?;

        debug!("🔒 Lua sandbox environment configured");
        Ok(())
    }

    /// Create a new Lua VM on demand with appropriate settings
    pub fn create_lua_vm(&self) -> ScriptResult<Lua> {
        let lua = if self.sandbox_enabled {
            Lua::new_with(
                StdLib::TABLE | StdLib::STRING | StdLib::MATH | StdLib::UTF8,
                LuaOptions::new().catch_rust_panics(true),
            )?
        } else {
            Lua::new_with(
                StdLib::ALL_SAFE,
                LuaOptions::new().catch_rust_panics(true),
            )?
        };

        if self.sandbox_enabled {
            Self::setup_sandbox(&lua)?;
        }

        // Load all scripts into the VM
        for script_info in self.scripts.values() {
            lua.load(&script_info.content)
                .set_name(&script_info.path.to_string_lossy().to_string())
                .exec()?;
        }

        Ok(lua)
    }

    /// Load and compile a script with hash-based change detection
    pub fn load_script<P: AsRef<Path>>(&mut self, path: P) -> ScriptResult<()> {
        let path = path.as_ref().to_path_buf();
        let content = std::fs::read_to_string(&path)?;

        // Create script info with hash
        let script_info = ScriptInfo::new(path.clone(), content.clone())?;
        self.scripts.insert(path.clone(), script_info);

        info!("📜 Loaded script: {} (hash: {:016x})", path.display(), 
              HashStrategies::hash_string(&content));
        Ok(())
    }

    /// Reload script if content changed (using hash comparison)
    pub fn reload_script<P: AsRef<Path>>(&mut self, path: P) -> ScriptResult<bool> {
        let path = path.as_ref();
        let new_content = std::fs::read_to_string(path)?;

        // Check if script has changed using hash
        let needs_reload = if let Some(script_info) = self.scripts.get(path) {
            script_info.has_content_changed(&new_content)
        } else {
            true // New script
        };

        if needs_reload {
            // Update script info
            if let Some(script_info) = self.scripts.get_mut(path) {
                script_info.update_content(new_content.clone())?;
            } else {
                let script_info = ScriptInfo::new(path.to_path_buf(), new_content.clone())?;
                self.scripts.insert(path.to_path_buf(), script_info);
            }

            info!("🔄 Reloaded script: {} (new hash: {:016x})", path.display(), 
                  HashStrategies::hash_string(&new_content));
            Ok(true)
        } else {
            debug!("⏭️ Script unchanged: {}", path.display());
            Ok(false)
        }
    }

    /// Execute a loaded script and return a serialized result
    pub fn execute_script<P: AsRef<Path>>(&self, path: P) -> ScriptResult<String> {
        let path = path.as_ref();
        let lua = self.create_lua_vm()?;
        
        // Find and execute the specific script
        if let Some(script_info) = self.scripts.get(path) {
            let result: mlua::Value = lua
                .load(&script_info.content)
                .set_name(&script_info.path.to_string_lossy().to_string())
                .call(())?;
            
            // Convert to String to avoid lifetime issues
            let result_str = match result {
                mlua::Value::String(s) => s.to_str()?.to_string(),
                mlua::Value::Integer(n) => n.to_string(),
                mlua::Value::Number(n) => n.to_string(),
                mlua::Value::Boolean(b) => b.to_string(),
                mlua::Value::Nil => "nil".to_string(),
                _ => "complex_value".to_string(),
            };
            
            debug!("🚀 Executed script: {}", path.display());
            Ok(result_str)
        } else {
            Err(ScriptError::FileNotFound { 
                path: path.to_path_buf() 
            })
        }
    }

    /// Call a global Lua function
    pub fn call_function<A, R>(&self, name: &str, args: A) -> ScriptResult<R>
    where
        A: for<'a> mlua::IntoLuaMulti<'a>,
        R: for<'a> mlua::FromLuaMulti<'a>,
    {
        let lua = self.create_lua_vm()?;
        let globals = lua.globals();
        let function: Function = globals.get(name)
            .map_err(|_| ScriptError::ExecutionFailed {
                reason: format!("Function '{}' not found", name)
            })?;

        let result = function.call(args)?;
        Ok(result)
    }

    /// Register an event callback
    pub fn register_event_callback(&mut self, event_name: &str, callback: LuaCallback) -> ScriptResult<()> {
        self.event_callbacks
            .entry(event_name.to_string())
            .or_default()
            .push(callback.clone());

        // Sort by priority (higher priority first)
        if let Some(callbacks) = self.event_callbacks.get_mut(event_name) {
            callbacks.sort_by(|a, b| b.priority.cmp(&a.priority));
        }

        debug!("📋 Registered event callback: {} -> {}", event_name, callback.function_name);
        Ok(())
    }

    /// Trigger event callbacks and return serialized results
    pub fn trigger_event(&self, event_name: &str, event_data: &LuaEventData) -> ScriptResult<Vec<String>> {
        let empty_vec = vec![];
        let callbacks = self.event_callbacks.get(event_name).unwrap_or(&empty_vec);
        let mut results = Vec::new();

        for callback in callbacks {
            match self.call_function::<_, String>(&callback.function_name, event_data.clone()) {
                Ok(result) => {
                    results.push(result);
                    debug!("✅ Event callback '{}' executed successfully", callback.function_name);
                }
                Err(e) => {
                    warn!("❌ Event callback '{}' failed: {}", callback.function_name, e);
                }
            }
        }

        Ok(results)
    }

    /// Get script count for diagnostics
    pub fn script_count(&self) -> usize {
        self.scripts.len()
    }
}

/// Comprehensive Lua handler that integrates with the existing ReloadHandler system
#[derive(Clone)]
pub struct ComprehensiveLuaHandler {
    /// Lua environment (wrapped for thread safety)
    environment: Arc<RwLock<LuaEnvironment>>,
    /// Handler name
    name: &'static str,
}

impl ComprehensiveLuaHandler {
    /// Create new comprehensive Lua handler
    pub fn new(sandboxed: bool) -> ScriptResult<Self> {
        let environment = if sandboxed {
            LuaEnvironment::new_sandboxed()?
        } else {
            LuaEnvironment::new_unrestricted()?
        };

        Ok(Self {
            environment: Arc::new(RwLock::new(environment)),
            name: "comprehensive_lua",
        })
    }

    /// Get access to the Lua environment
    pub fn environment(&self) -> Arc<RwLock<LuaEnvironment>> {
        self.environment.clone()
    }

    /// Load a script into the Lua environment
    pub fn load_script<P: AsRef<Path>>(&self, path: P) -> ScriptResult<()> {
        let mut env = self.environment.write();
        env.load_script(path)
    }

    /// Execute a loaded script
    pub fn execute_script<P: AsRef<Path>>(&self, path: P) -> ScriptResult<String> {
        let env = self.environment.read();
        env.execute_script(path)
    }

    /// Call a Lua function
    pub fn call_function<A, R>(&self, name: &str, args: A) -> ScriptResult<R>
    where
        A: for<'a> mlua::IntoLuaMulti<'a>,
        R: for<'a> mlua::FromLuaMulti<'a>,
    {
        let env = self.environment.read();
        env.call_function(name, args)
    }

    /// Register an event callback
    pub fn register_event_callback(&self, event_name: &str, callback: LuaCallback) -> ScriptResult<()> {
        let mut env = self.environment.write();
        env.register_event_callback(event_name, callback)
    }

    /// Trigger event callbacks
    pub fn trigger_event(&self, event_name: &str, event_data: &LuaEventData) -> ScriptResult<Vec<String>> {
        let env = self.environment.read();
        env.trigger_event(event_name, event_data)
    }
}

impl ReloadHandler for ComprehensiveLuaHandler {
    fn name(&self) -> &'static str {
        self.name
    }

    fn handles(&self, path: &PathBuf) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext == "lua")
            .unwrap_or(false)
    }

    fn reload(&mut self, path: &PathBuf) -> ReloadResult<()> {
        let mut env = self.environment.write();
        
        match env.reload_script(path) {
            Ok(was_reloaded) => {
                if was_reloaded {
                    info!("🔄 Successfully reloaded Lua script: {}", path.display());
                } else {
                    debug!("⏭️ Lua script unchanged: {}", path.display());
                }
                Ok(())
            }
            Err(e) => Err(ReloadError::Failed {
                reason: format!("Failed to reload Lua script {}: {}", path.display(), e),
            }),
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Comprehensive script manager integrated with existing ReloadManager and hashing
#[derive(Resource)]
pub struct ScriptManager {
    /// Lua handler integrated with reload system
    lua_handler: Arc<RwLock<ComprehensiveLuaHandler>>,
    /// Existing reload manager
    reload_manager: Arc<RwLock<ReloadManager>>,
    /// Script search directories
    script_dirs: Vec<PathBuf>,
    /// Reload event receiver (not debug-able)
    reload_receiver: Option<Receiver<ReloadEvent>>,
    /// Whether hot reload is enabled
    hot_reload: bool,
}

// Manual Debug implementation to avoid issues with non-Debug types
impl std::fmt::Debug for ScriptManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScriptManager")
            .field("lua_handler", &"Arc<RwLock<ComprehensiveLuaHandler>>")
            .field("reload_manager", &"Arc<RwLock<ReloadManager>>")
            .field("script_dirs", &self.script_dirs)
            .field("hot_reload", &self.hot_reload)
            .finish()
    }
}

impl Default for ScriptManager {
    fn default() -> Self {
        Self::new().expect("Failed to create default ScriptManager")
    }
}

impl ScriptManager {
    /// Create new comprehensive script manager with hot reloading
    pub fn new() -> ScriptResult<Self> {
        Self::new_with_options(true, true) // sandboxed, hot reload enabled
    }

    /// Create script manager with custom options
    pub fn new_with_options(sandboxed: bool, hot_reload: bool) -> ScriptResult<Self> {
        // Create the comprehensive Lua handler
        let lua_handler = ComprehensiveLuaHandler::new(sandboxed)?;
        let lua_handler_arc = Arc::new(RwLock::new(lua_handler));

        // Create the reload manager
        let reload_manager = ReloadManager::new()
            .map_err(|e| ScriptError::SandboxingFailed {
                reason: format!("Failed to create ReloadManager: {}", e),
            })?;

        // Start the reload manager if hot reload is enabled
        if hot_reload {
            reload_manager.start()
                .map_err(|e| ScriptError::SandboxingFailed {
                    reason: format!("Failed to start ReloadManager: {}", e),
                })?;
        }

        let reload_manager = Arc::new(RwLock::new(reload_manager));

        let manager = Self {
            lua_handler: lua_handler_arc,
            reload_manager,
            script_dirs: vec![
                PathBuf::from("lua-scripts"),
                PathBuf::from("backend/lua-scripts"),
            ],
            reload_receiver: None, // We'll poll events instead
            hot_reload,
        };

        info!("🔧 Initialized comprehensive Lua scripting system (sandboxed: {}, hot_reload: {})", 
              sandboxed, hot_reload);
        Ok(manager)
    }

    /// Load and watch a script file
    pub fn load_script<P: AsRef<Path>>(&self, path: P) -> ScriptResult<()> {
        let path = path.as_ref();
        let full_path = self.resolve_script_path(path)?;
        
        // Load the script into the Lua environment
        {
            let handler = self.lua_handler.read();
            handler.load_script(&full_path)?;
        }

        // Watch the file for changes if hot reload is enabled
        if self.hot_reload {
            let mut reload_manager = self.reload_manager.write();
            reload_manager.watch_file(full_path.clone(), FileType::Lua)
                .map_err(|e| ScriptError::BindingFailed {
                    reason: format!("Failed to watch file {}: {}", full_path.display(), e),
                })?;
        }

        info!("📜 Loaded and watching script: {}", full_path.display());
        Ok(())
    }

    /// Call a Lua function
    pub fn call_function<A, R>(&self, name: &str, args: A) -> ScriptResult<R> 
    where
        A: for<'a> mlua::IntoLuaMulti<'a>,
        R: for<'a> mlua::FromLuaMulti<'a>,
    {
        let handler = self.lua_handler.read();
        handler.call_function(name, args)
    }

    /// Register an event callback
    pub fn register_event_callback(&self, event_name: &str, callback: LuaCallback) -> ScriptResult<()> {
        let handler = self.lua_handler.read();
        handler.register_event_callback(event_name, callback)
    }

    /// Resolve script path from script directories
    fn resolve_script_path(&self, path: &Path) -> ScriptResult<PathBuf> {
        // Try as absolute path first
        if path.is_absolute() && path.exists() {
            return Ok(path.to_path_buf());
        }

        // Search in script directories
        for dir in &self.script_dirs {
            let full_path = dir.join(path);
            if full_path.exists() {
                return Ok(full_path);
            }
        }

        Err(ScriptError::FileNotFound { path: path.to_path_buf() })
    }

    /// Check if hot reload is enabled
    pub fn hot_reload_enabled(&self) -> bool {
        self.hot_reload
    }

    /// Execute a loaded script
    pub fn execute_script<P: AsRef<Path>>(&self, path: P) -> ScriptResult<String> {
        let path = path.as_ref();
        let full_path = self.resolve_script_path(path)?;
        
        let handler = self.lua_handler.read();
        handler.execute_script(&full_path)
    }

    /// Trigger event callbacks
    pub fn trigger_event(&self, event_name: &str, event_data: &LuaEventData) -> ScriptResult<Vec<String>> {
        let handler = self.lua_handler.read();
        handler.trigger_event(event_name, event_data)
    }

    /// Process pending reload events
    pub fn process_reload_events(&self) -> Vec<ReloadEvent> {
        if !self.hot_reload {
            return vec![];
        }

        let reload_manager = self.reload_manager.read();
        reload_manager.poll_events()
    }

    /// Get Lua environment for advanced usage
    pub fn environment(&self) -> Arc<RwLock<LuaEnvironment>> {
        let handler = self.lua_handler.read();
        handler.environment()
    }

    /// Get the number of loaded scripts
    pub fn loaded_script_count(&self) -> usize {
        let handler = self.lua_handler.read();
        let env = handler.environment();
        let env = env.read();
        env.scripts.len()
    }

    /// Get reload manager statistics
    pub fn reload_stats(&self) -> crate::core::reloader::ReloadStats {
        let reload_manager = self.reload_manager.read();
        reload_manager.stats()
    }
}