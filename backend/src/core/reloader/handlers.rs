//! File type handlers for hot reloading
//!
//! Simple handlers for specific file types, each with single responsibility.

use super::types::{ReloadHandler, ReloadResult, ReloadError};
use mlua::Lua;
use parking_lot::Mutex;
use std::{path::PathBuf, sync::Arc, collections::HashMap};
use tracing::{debug, warn};
use crossbeam::channel::{unbounded, Sender, Receiver};

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

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Configuration change notification
#[derive(Debug, Clone)]
pub struct ConfigChange {
    pub path: PathBuf,
    pub config_type: ConfigType,
    pub data: HashMap<String, serde_json::Value>,
}

/// Types of configuration files supported
#[derive(Debug, Clone)]
pub enum ConfigType {
    Json,
    Toml,
    Yaml,
    Ron,
}

/// Handler for configuration files (TOML, JSON, etc.)
pub struct ConfigHandler {
    /// Channel for notifying subscribers of config changes
    change_sender: Sender<ConfigChange>,
    change_receiver: Arc<Mutex<Receiver<ConfigChange>>>,
}

impl ConfigHandler {
    pub fn new() -> Self {
        let (sender, receiver) = unbounded();
        Self {
            change_sender: sender,
            change_receiver: Arc::new(Mutex::new(receiver)),
        }
    }

    /// Get receiver for config change notifications
    pub fn change_receiver(&self) -> Arc<Mutex<Receiver<ConfigChange>>> {
        self.change_receiver.clone()
    }

    /// Parse configuration based on file extension
    fn parse_config(&self, path: &PathBuf, content: &str) -> ReloadResult<(ConfigType, HashMap<String, serde_json::Value>)> {
        let ext = path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");

        match ext {
            "json" => {
                let json_value: serde_json::Value = serde_json::from_str(content)
                    .map_err(|e| ReloadError::Failed {
                        reason: format!("Invalid JSON in {}: {}", path.display(), e),
                    })?;
                
                let data = match json_value {
                    serde_json::Value::Object(map) => map.into_iter().collect(),
                    _ => return Err(ReloadError::Failed {
                        reason: format!("Config file {} must contain a JSON object", path.display()),
                    }),
                };
                
                Ok((ConfigType::Json, data))
            }
            "ron" => {
                let ron_value: ron::Value = ron::from_str(content)
                    .map_err(|e| ReloadError::Failed {
                        reason: format!("Invalid RON in {}: {}", path.display(), e),
                    })?;
                
                // Convert RON to JSON for uniform handling
                let json_value: serde_json::Value = serde_json::to_value(ron_value)
                    .map_err(|e| ReloadError::Failed {
                        reason: format!("Failed to convert RON to JSON for {}: {}", path.display(), e),
                    })?;
                
                let data = match json_value {
                    serde_json::Value::Object(map) => map.into_iter().collect(),
                    _ => return Err(ReloadError::Failed {
                        reason: format!("Config file {} must contain a RON mapping", path.display()),
                    }),
                };
                
                Ok((ConfigType::Ron, data))
            }
            "toml" => {
                let toml_value: toml::Value = toml::from_str(content)
                    .map_err(|e| ReloadError::Failed {
                        reason: format!("Invalid TOML in {}: {}", path.display(), e),
                    })?;
                
                // Convert TOML to JSON for uniform handling
                let json_str = serde_json::to_string(&toml_value)
                    .map_err(|e| ReloadError::Failed {
                        reason: format!("Failed to convert TOML to JSON for {}: {}", path.display(), e),
                    })?;
                
                let json_value: serde_json::Value = serde_json::from_str(&json_str)
                    .map_err(|e| ReloadError::Failed {
                        reason: format!("Failed to parse converted TOML as JSON for {}: {}", path.display(), e),
                    })?;
                
                let data = match json_value {
                    serde_json::Value::Object(map) => map.into_iter().collect(),
                    _ => return Err(ReloadError::Failed {
                        reason: format!("Config file {} must contain a TOML table", path.display()),
                    }),
                };
                
                Ok((ConfigType::Toml, data))
            }
            "yaml" | "yml" => {
                let yaml_value: serde_yaml::Value = serde_yaml::from_str(content)
                    .map_err(|e| ReloadError::Failed {
                        reason: format!("Invalid YAML in {}: {}", path.display(), e),
                    })?;
                
                // Convert YAML to JSON for uniform handling
                let json_value: serde_json::Value = serde_json::to_value(yaml_value)
                    .map_err(|e| ReloadError::Failed {
                        reason: format!("Failed to convert YAML to JSON for {}: {}", path.display(), e),
                    })?;
                
                let data = match json_value {
                    serde_json::Value::Object(map) => map.into_iter().collect(),
                    _ => return Err(ReloadError::Failed {
                        reason: format!("Config file {} must contain a YAML mapping", path.display()),
                    }),
                };
                
                Ok((ConfigType::Yaml, data))
            }
            _ => Err(ReloadError::Failed {
                reason: format!("Unsupported config format for file: {}", path.display()),
            }),
        }
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
            .map(|ext| matches!(ext, "toml" | "json" | "yaml" | "yml" | "ron"))
            .unwrap_or(false)
    }

    fn reload(&mut self, path: &PathBuf) -> ReloadResult<()> {
        // Read and parse the configuration file
        let content = std::fs::read_to_string(path).map_err(|e| ReloadError::Failed {
            reason: format!("Failed to read config {}: {}", path.display(), e),
        })?;

        // Parse and validate specific config formats
        let (config_type, data) = self.parse_config(path, &content)?;
        
        // Create config change notification
        let change = ConfigChange {
            path: path.clone(),
            config_type,
            data,
        };

        // Notify systems that config changed
        if let Err(_) = self.change_sender.send(change) {
            warn!("No subscribers for config change notifications");
        }
        
        debug!("⚙️ Reloaded and parsed config: {}", path.display());
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Asset change notification
#[derive(Debug, Clone)]
pub struct AssetChange {
    pub path: PathBuf,
    pub asset_type: AssetType,
    pub size_bytes: u64,
}

/// Types of assets supported
#[derive(Debug, Clone)]
pub enum AssetType {
    Image,
    Audio,
    Model,
    Unknown,
}

/// Handler for asset files (images, audio, etc.)
pub struct AssetHandler {
    /// Channel for notifying subscribers of asset changes
    change_sender: Sender<AssetChange>,
    change_receiver: Arc<Mutex<Receiver<AssetChange>>>,
}

impl AssetHandler {
    pub fn new() -> Self {
        let (sender, receiver) = unbounded();
        Self {
            change_sender: sender,
            change_receiver: Arc::new(Mutex::new(receiver)),
        }
    }

    /// Get receiver for asset change notifications
    pub fn change_receiver(&self) -> Arc<Mutex<Receiver<AssetChange>>> {
        self.change_receiver.clone()
    }

    /// Determine asset type from file extension
    fn determine_asset_type(&self, path: &PathBuf) -> AssetType {
        let ext = path.extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.to_lowercase());

        match ext.as_deref() {
            Some("png" | "jpg" | "jpeg" | "gif" | "bmp" | "tga" | "dds" | "ktx2") => AssetType::Image,
            Some("wav" | "ogg" | "mp3" | "flac" | "opus") => AssetType::Audio,
            Some("glb" | "gltf" | "obj" | "fbx" | "blend") => AssetType::Model,
            _ => AssetType::Unknown,
        }
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
        // Validate the file exists
        if !path.exists() {
            return Err(ReloadError::FileNotFound {
                path: path.to_string_lossy().to_string(),
            });
        }

        // Get file metadata to include in notification
        let metadata = std::fs::metadata(path).map_err(|e| ReloadError::Failed {
            reason: format!("Failed to read asset metadata for {}: {}", path.display(), e),
        })?;

        // Determine asset type from file extension
        let asset_type = self.determine_asset_type(path);

        // Create asset change notification
        let change = AssetChange {
            path: path.clone(),
            asset_type: asset_type.clone(),
            size_bytes: metadata.len(),
        };

        // Notify systems that asset changed
        // The rendering system or asset management system would listen to these notifications
        if let Err(_) = self.change_sender.send(change) {
            warn!("No subscribers for asset change notifications");
        }
        
        debug!("🎨 Reloaded asset: {} (type: {:?}, size: {} bytes)", 
               path.display(), asset_type, metadata.len());
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
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
        
        let handler = handler.expect("Failed to create LuaHandler for test");
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
        let temp_dir = TempDir::new().expect("Failed to create temporary directory for Lua handler test");
        let script_path = temp_dir.path().join("test.lua");
        std::fs::write(&script_path, "x = 42\nfunction get_x() return x end").expect("Failed to write test Lua script");

        let mut handler = LuaHandler::new().expect("Failed to create LuaHandler for reload test");
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
