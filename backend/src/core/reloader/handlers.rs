//! File type handlers for hot reloading
//!
//! Simple handlers for specific file types, each with single responsibility.

use super::types::{ReloadHandler, ReloadResult, ReloadError};
use mlua::Lua;
use parking_lot::Mutex;
use std::{path::PathBuf, sync::Arc};
use tracing::debug;

/// Handler for Lua script files
pub struct LuaHandler {
    lua: Arc<Mutex<Lua>>,
}

impl LuaHandler {
    /// Create a new Lua handler with sandboxed environment
    pub fn new() -> ReloadResult<Self> {
        let lua = Lua::new();
        
        // Basic sandboxing - remove dangerous functions
        {
            let globals = lua.globals();
            for dangerous in &["dofile", "loadfile", "load", "loadstring", "io", "debug"] {
                globals.set(*dangerous, mlua::Value::Nil).map_err(|e| {
                    ReloadError::Failed {
                        reason: format!("Failed to sandbox Lua: {}", e),
                    }
                })?;
            }
        } // Drop globals here

        debug!("🔒 Created sandboxed Lua environment");
        Ok(Self {
            lua: Arc::new(Mutex::new(lua)),
        })
    }

    /// Get the Lua instance for advanced usage
    pub fn lua(&self) -> Arc<Mutex<Lua>> {
        self.lua.clone()
    }

    /// Execute a Lua function by name (returns string representation)
    pub fn call_function_simple(&self, func_name: &str) -> mlua::Result<String> {
        let lua = self.lua.lock();
        let func: mlua::Function = lua.globals().get(func_name)?;
        let result: mlua::Value = func.call(())?;
        
        // Convert to string to avoid lifetime issues
        let result_str = match result {
            mlua::Value::String(s) => s.to_str()?.to_string(),
            mlua::Value::Integer(n) => n.to_string(),
            mlua::Value::Number(n) => n.to_string(),
            mlua::Value::Boolean(b) => b.to_string(),
            mlua::Value::Nil => "nil".to_string(),
            _ => "complex_value".to_string(),
        };
        
        Ok(result_str)
    }
}

impl Default for LuaHandler {
    fn default() -> Self {
        Self::new().expect("Failed to create default LuaHandler")
    }
}

impl ReloadHandler for LuaHandler {
    fn name(&self) -> &'static str {
        "lua"
    }

    fn handles(&self, path: &PathBuf) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext == "lua")
            .unwrap_or(false)
    }

    fn reload(&mut self, path: &PathBuf) -> ReloadResult<()> {
        let content = std::fs::read_to_string(path).map_err(|e| ReloadError::Failed {
            reason: format!("Failed to read {}: {}", path.display(), e),
        })?;

        let script_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        let lua = self.lua.lock();
        lua.load(&content)
            .set_name(script_name)
            .exec()
            .map_err(|e| ReloadError::Failed {
                reason: format!("Failed to execute Lua script {}: {}", script_name, e),
            })?;

        debug!("📜 Reloaded Lua script: {}", script_name);
        Ok(())
    }
}

/// Handler for configuration files (TOML, JSON, etc.)
pub struct ConfigHandler;

impl ConfigHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ConfigHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl ReloadHandler for ConfigHandler {
    fn name(&self) -> &'static str {
        "config"
    }

    fn handles(&self, path: &PathBuf) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| matches!(ext, "toml" | "json" | "yaml" | "yml"))
            .unwrap_or(false)
    }

    fn reload(&mut self, path: &PathBuf) -> ReloadResult<()> {
        // For now, just validate the file can be read
        let _content = std::fs::read_to_string(path).map_err(|e| ReloadError::Failed {
            reason: format!("Failed to read config {}: {}", path.display(), e),
        })?;

        // TODO: Parse and validate specific config formats
        // TODO: Notify systems that config changed
        
        debug!("⚙️ Reloaded config: {}", path.display());
        Ok(())
    }
}

/// Handler for asset files (images, audio, etc.)
pub struct AssetHandler;

impl AssetHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AssetHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl ReloadHandler for AssetHandler {
    fn name(&self) -> &'static str {
        "asset"
    }

    fn handles(&self, path: &PathBuf) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| {
                matches!(
                    ext,
                    "png" | "jpg" | "jpeg" | "gif" | "bmp" | "tga" | "dds" | "ktx2" |
                    "wav" | "ogg" | "mp3" | "flac" | "opus" |
                    "glb" | "gltf" | "obj" | "fbx" | "blend"
                )
            })
            .unwrap_or(false)
    }

    fn reload(&mut self, path: &PathBuf) -> ReloadResult<()> {
        // For now, just validate the file exists
        if !path.exists() {
            return Err(ReloadError::FileNotFound {
                path: path.to_string_lossy().to_string(),
            });
        }

        // TODO: Reload asset in the rendering system
        // TODO: Notify systems that asset changed
        
        debug!("🎨 Reloaded asset: {}", path.display());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tempfile::TempDir;

    #[test]
    fn lua_handler_creation() {
        let handler = LuaHandler::new();
        assert!(handler.is_ok());
        
        let handler = handler.unwrap();
        assert_eq!(handler.name(), "lua");
    }

    #[test]
    fn lua_handler_file_detection() {
        let handler = LuaHandler::default();
        
        assert!(handler.handles(&PathBuf::from("test.lua")));
        assert!(!handler.handles(&PathBuf::from("test.rs")));
        assert!(!handler.handles(&PathBuf::from("test")));
    }

    #[test]
    fn lua_handler_reload() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("test.lua");
        std::fs::write(&script_path, "x = 42\nfunction get_x() return x end").unwrap();

        let mut handler = LuaHandler::new().unwrap();
        let result = handler.reload(&script_path);
        assert!(result.is_ok());

        // Test that the script was executed
        let result = handler.call_function_simple("get_x");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "42");
    }

    #[test]
    fn config_handler_file_detection() {
        let handler = ConfigHandler::new();
        
        assert!(handler.handles(&PathBuf::from("config.toml")));
        assert!(handler.handles(&PathBuf::from("settings.json")));
        assert!(handler.handles(&PathBuf::from("data.yaml")));
        assert!(!handler.handles(&PathBuf::from("test.lua")));
    }

    #[test]
    fn asset_handler_file_detection() {
        let handler = AssetHandler::new();
        
        assert!(handler.handles(&PathBuf::from("texture.png")));
        assert!(handler.handles(&PathBuf::from("sound.wav")));
        assert!(handler.handles(&PathBuf::from("model.glb")));
        assert!(!handler.handles(&PathBuf::from("script.lua")));
    }

    #[test]
    fn config_handler_reload() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test.toml");
        std::fs::write(&config_path, "[settings]\nvalue = 123").unwrap();

        let mut handler = ConfigHandler::new();
        let result = handler.reload(&config_path);
        assert!(result.is_ok());
    }
}
