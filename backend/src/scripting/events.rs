//! Event system integration for Lua scripts
//!
//! Provides event-driven scripting with callbacks, triggers,
//! and proper event data serialization.

use mlua::{Lua, Table, Function, Value, UserData, UserDataMethods, IntoLua};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use parking_lot::RwLock;
use tracing::{debug, warn};
use super::{ScriptResult, ScriptError};

/// Event data that can be passed to Lua callbacks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LuaEventData {
    pub event_type: String,
    pub data: HashMap<String, LuaEventValue>,
    pub timestamp: u64,
    pub source: Option<String>,
}

/// Values that can be stored in event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LuaEventValue {
    Nil,
    Boolean(bool),
    Integer(i64),
    Number(f64),
    String(String),
    Table(HashMap<String, LuaEventValue>),
}

impl From<Value<'_>> for LuaEventValue {
    fn from(value: Value<'_>) -> Self {
        match value {
            Value::Nil => LuaEventValue::Nil,
            Value::Boolean(b) => LuaEventValue::Boolean(b),
            Value::Integer(i) => LuaEventValue::Integer(i),
            Value::Number(n) => LuaEventValue::Number(n),
            Value::String(s) => LuaEventValue::String(s.to_str().unwrap_or("").to_string()),
            Value::Table(t) => {
                let mut map = HashMap::new();
                for pair in t.pairs::<String, Value>() {
                    if let Ok((key, val)) = pair {
                        map.insert(key, val.into());
                    }
                }
                LuaEventValue::Table(map)
            }
            _ => LuaEventValue::Nil,
        }
    }
}

impl<'lua> IntoLua<'lua> for LuaEventValue {
    fn into_lua(self, lua: &'lua Lua) -> mlua::Result<Value<'lua>> {
        let value = match self {
            LuaEventValue::Nil => Value::Nil,
            LuaEventValue::Boolean(b) => Value::Boolean(b),
            LuaEventValue::Integer(i) => Value::Integer(i),
            LuaEventValue::Number(n) => Value::Number(n),
            LuaEventValue::String(s) => Value::String(lua.create_string(&s)?),
            LuaEventValue::Table(map) => {
                let table = lua.create_table()?;
                for (key, value) in map {
                    table.set(key, value.into_lua(lua)?)?;
                }
                Value::Table(table)
            },
        };
        Ok(value)
    }
}

impl UserData for LuaEventData {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("get_type", |_, this, ()| {
            Ok(this.event_type.clone())
        });

        methods.add_method("get_timestamp", |_, this, ()| {
            Ok(this.timestamp)
        });

        methods.add_method("get_source", |_, this, ()| {
            Ok(this.source.clone())
        });

        methods.add_method("get", |_, this, key: String| {
            match this.data.get(&key) {
                Some(value) => Ok(value.clone()),
                None => Ok(LuaEventValue::Nil),
            }
        });

        methods.add_method("has", |_, this, key: String| {
            Ok(this.data.contains_key(&key))
        });

        methods.add_method("keys", |_, this, ()| {
            Ok(this.data.keys().cloned().collect::<Vec<_>>())
        });
    }
}

/// Event callback information
#[derive(Debug, Clone)]
pub struct EventCallback {
    pub name: String,
    pub event_type: String,
    pub priority: i32,
    pub once: bool,
    pub called: bool,
}

/// Event system manager for Lua integration
pub struct LuaEventSystem {
    callbacks: RwLock<HashMap<String, Vec<EventCallback>>>,
    event_history: RwLock<Vec<LuaEventData>>,
    max_history: usize,
}

impl LuaEventSystem {
    /// Create new event system
    pub fn new() -> Self {
        Self {
            callbacks: RwLock::new(HashMap::new()),
            event_history: RwLock::new(Vec::new()),
            max_history: 1000,
        }
    }

    /// Register a callback for an event type
    pub fn register_callback(&self, event_type: String, callback_name: String, priority: Option<i32>, once: Option<bool>) {
        let callback = EventCallback {
            name: callback_name,
            event_type: event_type.clone(),
            priority: priority.unwrap_or(0),
            once: once.unwrap_or(false),
            called: false,
        };

        let mut callbacks = self.callbacks.write();
        let event_callbacks = callbacks.entry(event_type).or_insert_with(Vec::new);
        event_callbacks.push(callback);
        
        // Sort by priority (higher priority first)
        event_callbacks.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Trigger callbacks for an event
    pub fn trigger_event(&self, lua: &Lua, event_data: LuaEventData) -> ScriptResult<()> {
        let event_type = event_data.event_type.clone();
        
        // Add to history
        {
            let mut history = self.event_history.write();
            history.push(event_data.clone());
            if history.len() > self.max_history {
                let excess = history.len() - self.max_history;
                history.drain(..excess);
            }
        }

        // Get callbacks for this event type
        let callbacks_to_call = {
            let callbacks = self.callbacks.read();
            callbacks.get(&event_type).cloned().unwrap_or_default()
        };

        if callbacks_to_call.is_empty() {
            return Ok(());
        }

        // Execute callbacks
        let globals = lua.globals();
        let mut executed_callbacks = Vec::new();
        
        for callback in callbacks_to_call {
            if callback.called && callback.once {
                continue;
            }

            if let Ok(func) = globals.get::<_, Function>(callback.name.as_str()) {
                match func.call::<_, ()>(event_data.clone()) {
                    Ok(()) => {
                        debug!("Event callback {} executed successfully", callback.name);
                        if callback.once {
                            executed_callbacks.push(callback.name.clone());
                        }
                    }
                    Err(e) => {
                        warn!("Event callback {} failed: {}", callback.name, e);
                    }
                }
            } else {
                warn!("Event callback function {} not found", callback.name);
            }
        }

        // Mark one-time callbacks as called
        if !executed_callbacks.is_empty() {
            let mut callbacks = self.callbacks.write();
            if let Some(event_callbacks) = callbacks.get_mut(&event_type) {
                for callback in event_callbacks.iter_mut() {
                    if executed_callbacks.contains(&callback.name) {
                        callback.called = true;
                    }
                }
            }
        }

        Ok(())
    }

    /// Get recent events of a specific type
    pub fn get_recent_events(&self, event_type: Option<String>, limit: Option<usize>) -> Vec<LuaEventData> {
        let history = self.event_history.read();
        let limit = limit.unwrap_or(10).min(history.len());
        
        let filtered: Vec<_> = match event_type {
            Some(event_type) => {
                history.iter()
                    .rev()
                    .filter(|event| event.event_type == event_type)
                    .take(limit)
                    .cloned()
                    .collect()
            }
            None => {
                history.iter()
                    .rev()
                    .take(limit)
                    .cloned()
                    .collect()
            }
        };

        filtered
    }

    /// Clear event history
    pub fn clear_history(&self) {
        let mut history = self.event_history.write();
        history.clear();
    }

    /// Remove callbacks for an event type
    pub fn remove_callbacks(&self, event_type: &str) {
        let mut callbacks = self.callbacks.write();
        callbacks.remove(event_type);
    }
}

impl Default for LuaEventSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// Register event system API with Lua
pub fn register_event_api(lua: &Lua, event_system: &LuaEventSystem) -> ScriptResult<()> {
    let globals = lua.globals();
    
    // Get or create Game API table
    let game_api: Table = match globals.get("Game") {
        Ok(table) => table,
        Err(_) => {
            let table = lua.create_table()
                .map_err(|e| ScriptError::BindingFailed {
                    reason: format!("Failed to create Game table: {}", e)
                })?;
            globals.set("Game", table.clone())
                .map_err(|e| ScriptError::BindingFailed {
                    reason: format!("Failed to set Game table: {}", e)
                })?;
            table
        }
    };

    // Create events API table
    let events_api = lua.create_table()
        .map_err(|e| ScriptError::BindingFailed {
            reason: format!("Failed to create events API: {}", e)
        })?;

    // Event registration function
    let register_fn = lua.create_function(|_, (event_type, callback_name, options): (String, String, Option<Table>)| {
        let priority = options.as_ref().and_then(|opts| opts.get::<_, Option<i32>>("priority").ok().flatten());
        let once = options.as_ref().and_then(|opts| opts.get::<_, Option<bool>>("once").ok().flatten());
        
        // Note: In actual implementation, we'd need access to event_system here
        // This would be done through Lua userdata or similar mechanism
        
        debug!("Registered event callback: {} -> {}", event_type, callback_name);
        Ok(())
    }).map_err(|e| ScriptError::BindingFailed {
        reason: format!("Failed to create register function: {}", e)
    })?;

    events_api.set("register", register_fn)
        .map_err(|e| ScriptError::BindingFailed {
            reason: format!("Failed to bind register function: {}", e)
        })?;

    // Event creation helper
    let create_event_fn = lua.create_function(|_, (event_type, data): (String, Option<Table>)| {
        let mut event_data = LuaEventData {
            event_type,
            data: HashMap::new(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            source: Some("lua_script".to_string()),
        };

        if let Some(data_table) = data {
            for pair in data_table.pairs::<String, Value>() {
                if let Ok((key, value)) = pair {
                    event_data.data.insert(key, value.into());
                }
            }
        }

        Ok(event_data)
    }).map_err(|e| ScriptError::BindingFailed {
        reason: format!("Failed to create event creation function: {}", e)
    })?;

    events_api.set("create", create_event_fn)
        .map_err(|e| ScriptError::BindingFailed {
            reason: format!("Failed to bind create function: {}", e)
        })?;

    game_api.set("Events", events_api)
        .map_err(|e| ScriptError::BindingFailed {
            reason: format!("Failed to bind events API: {}", e)
        })?;

    Ok(())
}

/// Common game events that scripts can listen to
pub mod events {
    pub const TILE_CHANGED: &str = "tile_changed";
    pub const UNIT_MOVED: &str = "unit_moved";
    pub const BUILDING_CONSTRUCTED: &str = "building_constructed";
    pub const RESOURCE_DISCOVERED: &str = "resource_discovered";
    pub const TURN_STARTED: &str = "turn_started";
    pub const TURN_ENDED: &str = "turn_ended";
    pub const PLAYER_ACTION: &str = "player_action";
    pub const AI_DECISION: &str = "ai_decision";
    pub const DIPLOMATIC_EVENT: &str = "diplomatic_event";
    pub const COMBAT_OCCURRED: &str = "combat_occurred";
}
