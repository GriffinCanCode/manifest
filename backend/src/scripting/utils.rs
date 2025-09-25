//! Lua scripting utilities and helper functions
//!
//! Provides common utilities for Lua scripts including math helpers,
//! data conversion, validation, and script debugging tools.

use mlua::{Lua, Table, Value, Error as LuaError};
use rand::Rng;
use tracing::{debug, info};
use super::{ScriptResult, ScriptError};

/// Lua utility functions registry
pub struct LuaUtilities;

impl LuaUtilities {
    /// Register all utility functions with Lua
    pub fn register_all(lua: &Lua) -> ScriptResult<()> {
        Self::register_math_utils(lua)?;
        Self::register_table_utils(lua)?;
        Self::register_string_utils(lua)?;
        Self::register_validation_utils(lua)?;
        Self::register_debug_utils(lua)?;
        Self::register_random_utils(lua)?;
        
        info!("🔧 Registered all Lua utility functions");
        Ok(())
    }

    /// Register enhanced math utilities
    fn register_math_utils(lua: &Lua) -> ScriptResult<()> {
        let globals = lua.globals();
        let math: Table = globals.get("math")
            .map_err(|e| ScriptError::BindingFailed {
                reason: format!("Failed to get math table: {}", e)
            })?;

        // Clamp function
        let clamp_fn = lua.create_function(|_, (value, min_val, max_val): (f64, f64, f64)| {
            Ok(value.max(min_val).min(max_val))
        }).map_err(|e| ScriptError::BindingFailed {
            reason: format!("Failed to create clamp function: {}", e)
        })?;

        math.set("clamp", clamp_fn)?;

        // Linear interpolation
        let lerp_fn = lua.create_function(|_, (a, b, t): (f64, f64, f64)| {
            let t_clamped = t.max(0.0).min(1.0);
            Ok(a + (b - a) * t_clamped)
        }).map_err(|e| ScriptError::BindingFailed {
            reason: format!("Failed to create lerp function: {}", e)
        })?;

        math.set("lerp", lerp_fn)?;

        // Smoothstep function
        let smoothstep_fn = lua.create_function(|_, (edge0, edge1, x): (f64, f64, f64)| {
            let t = ((x - edge0) / (edge1 - edge0)).max(0.0).min(1.0);
            Ok(t * t * (3.0 - 2.0 * t))
        }).map_err(|e| ScriptError::BindingFailed {
            reason: format!("Failed to create smoothstep function: {}", e)
        })?;

        math.set("smoothstep", smoothstep_fn)?;

        // Sign function
        let sign_fn = lua.create_function(|_, x: f64| {
            Ok(if x > 0.0 { 1.0 } else if x < 0.0 { -1.0 } else { 0.0 })
        }).map_err(|e| ScriptError::BindingFailed {
            reason: format!("Failed to create sign function: {}", e)
        })?;

        math.set("sign", sign_fn)?;

        // Round to decimal places
        let round_fn = lua.create_function(|_, (value, decimals): (f64, Option<i32>)| {
            let decimals = decimals.unwrap_or(0).max(0) as u32;
            let multiplier = 10f64.powi(decimals as i32);
            Ok((value * multiplier).round() / multiplier)
        }).map_err(|e| ScriptError::BindingFailed {
            reason: format!("Failed to create round function: {}", e)
        })?;

        math.set("round", round_fn)?;

        debug!("📐 Math utilities registered");
        Ok(())
    }

    /// Register table manipulation utilities
    fn register_table_utils(lua: &Lua) -> ScriptResult<()> {
        let globals = lua.globals();
        
        let table_utils = lua.create_table()
            .map_err(|e| ScriptError::BindingFailed {
                reason: format!("Failed to create table utils: {}", e)
            })?;

        // Deep copy table
        let copy_fn = lua.create_function(|lua, table: Table| {
            fn copy_table_recursive<'lua>(lua: &'lua Lua, table: &Table<'lua>) -> Result<Table<'lua>, LuaError> {
                let new_table = lua.create_table()?;
                for pair in table.clone().pairs::<Value, Value>() {
                    let (key, value) = pair?;
                    let copied_value = match value {
                        Value::Table(ref t) => Value::Table(copy_table_recursive(lua, t)?),
                        other => other,
                    };
                    new_table.set(key, copied_value)?;
                }
                Ok(new_table)
            }

            copy_table_recursive(lua, &table)
        }).map_err(|e| ScriptError::BindingFailed {
            reason: format!("Failed to create copy function: {}", e)
        })?;

        table_utils.set("copy", copy_fn)?;

        // Merge tables
        let merge_fn = lua.create_function(|_, (table1, table2): (Table, Table)| {
            for pair in table2.pairs::<Value, Value>() {
                let (key, value) = pair?;
                table1.set(key, value)?;
            }
            Ok(table1)
        }).map_err(|e| ScriptError::BindingFailed {
            reason: format!("Failed to create merge function: {}", e)
        })?;

        table_utils.set("merge", merge_fn)?;

        // Get table size
        let size_fn = lua.create_function(|_, table: Table| {
            let mut count = 0;
            for _ in table.pairs::<Value, Value>() {
                count += 1;
            }
            Ok(count)
        }).map_err(|e| ScriptError::BindingFailed {
            reason: format!("Failed to create size function: {}", e)
        })?;

        table_utils.set("size", size_fn)?;

        // Check if table is empty
        let is_empty_fn = lua.create_function(|_, table: Table| {
            Ok(table.pairs::<Value, Value>().next().is_none())
        }).map_err(|e| ScriptError::BindingFailed {
            reason: format!("Failed to create is_empty function: {}", e)
        })?;

        table_utils.set("is_empty", is_empty_fn)?;

        globals.set("table_utils", table_utils)?;
        debug!("📋 Table utilities registered");
        Ok(())
    }

    /// Register string manipulation utilities
    fn register_string_utils(lua: &Lua) -> ScriptResult<()> {
        let globals = lua.globals();
        let string: Table = globals.get("string")
            .map_err(|e| ScriptError::BindingFailed {
                reason: format!("Failed to get string table: {}", e)
            })?;

        // Split string
        let split_fn = lua.create_function(|lua, (text, delimiter): (String, Option<String>)| {
            let delimiter = delimiter.unwrap_or_else(|| " ".to_string());
            let parts: Vec<String> = text.split(&delimiter).map(|s| s.to_string()).collect();
            
            let result = lua.create_table()?;
            for (i, part) in parts.iter().enumerate() {
                result.set(i + 1, part.clone())?;
            }
            Ok(result)
        }).map_err(|e| ScriptError::BindingFailed {
            reason: format!("Failed to create split function: {}", e)
        })?;

        string.set("split", split_fn)?;

        // Trim whitespace
        let trim_fn = lua.create_function(|_, text: String| {
            Ok(text.trim().to_string())
        }).map_err(|e| ScriptError::BindingFailed {
            reason: format!("Failed to create trim function: {}", e)
        })?;

        string.set("trim", trim_fn)?;

        // Title case
        let title_case_fn = lua.create_function(|_, text: String| {
            let result = text.split_whitespace()
                .map(|word| {
                    let mut chars: Vec<char> = word.chars().collect();
                    if let Some(first) = chars.get_mut(0) {
                        *first = first.to_uppercase().next().unwrap_or(*first);
                    }
                    chars.into_iter().collect::<String>()
                })
                .collect::<Vec<_>>()
                .join(" ");
            Ok(result)
        }).map_err(|e| ScriptError::BindingFailed {
            reason: format!("Failed to create title_case function: {}", e)
        })?;

        string.set("title_case", title_case_fn)?;

        debug!("📝 String utilities registered");
        Ok(())
    }

    /// Register validation utilities
    fn register_validation_utils(lua: &Lua) -> ScriptResult<()> {
        let globals = lua.globals();
        
        let validation = lua.create_table()
            .map_err(|e| ScriptError::BindingFailed {
                reason: format!("Failed to create validation table: {}", e)
            })?;

        // Type checking functions
        let is_number_fn = lua.create_function(|_, value: Value| {
            Ok(matches!(value, Value::Integer(_) | Value::Number(_)))
        })?;

        let is_string_fn = lua.create_function(|_, value: Value| {
            Ok(matches!(value, Value::String(_)))
        })?;

        let is_table_fn = lua.create_function(|_, value: Value| {
            Ok(matches!(value, Value::Table(_)))
        })?;

        let is_function_fn = lua.create_function(|_, value: Value| {
            Ok(matches!(value, Value::Function(_)))
        })?;

        validation.set("is_number", is_number_fn)?;
        validation.set("is_string", is_string_fn)?;
        validation.set("is_table", is_table_fn)?;
        validation.set("is_function", is_function_fn)?;

        // Range validation
        let in_range_fn = lua.create_function(|_, (value, min_val, max_val): (f64, f64, f64)| {
            Ok(value >= min_val && value <= max_val)
        })?;

        validation.set("in_range", in_range_fn)?;

        globals.set("validation", validation)?;
        debug!("✅ Validation utilities registered");
        Ok(())
    }

    /// Register debug utilities
    fn register_debug_utils(lua: &Lua) -> ScriptResult<()> {
        let globals = lua.globals();
        
        let debug_utils = lua.create_table()
            .map_err(|e| ScriptError::BindingFailed {
                reason: format!("Failed to create debug table: {}", e)
            })?;

        // Pretty print function
        let print_fn = lua.create_function(|_, value: Value| {
            fn value_to_string(value: &Value, depth: usize) -> String {
                if depth > 10 { return "...".to_string(); }
                
                match value {
                    Value::Nil => "nil".to_string(),
                    Value::Boolean(b) => b.to_string(),
                    Value::Integer(i) => i.to_string(),
                    Value::Number(n) => n.to_string(),
                    Value::String(s) => format!("\"{}\"", s.to_str().unwrap_or("")),
                    Value::Table(t) => {
                        let mut parts = Vec::new();
                        for pair in t.clone().pairs::<Value, Value>() {
                            if let Ok((k, v)) = pair {
                                let key_str = value_to_string(&k, depth + 1);
                                let val_str = value_to_string(&v, depth + 1);
                                parts.push(format!("{} = {}", key_str, val_str));
                                if parts.len() > 20 { // Limit output
                                    parts.push("...".to_string());
                                    break;
                                }
                            }
                        }
                        format!("{{ {} }}", parts.join(", "))
                    },
                    _ => "<complex_value>".to_string(),
                }
            }

            let output = value_to_string(&value, 0);
            info!("[Lua Debug] {}", output);
            Ok(())
        })?;

        debug_utils.set("print", print_fn)?;

        // Type inspection
        let typeof_fn = lua.create_function(|_, value: Value| {
            let type_name = match value {
                Value::Nil => "nil",
                Value::Boolean(_) => "boolean",
                Value::Integer(_) => "integer",
                Value::Number(_) => "number",
                Value::String(_) => "string",
                Value::Table(_) => "table",
                Value::Function(_) => "function",
                Value::Thread(_) => "thread",
                Value::UserData(_) => "userdata",
                _ => "unknown",
            };
            Ok(type_name.to_string())
        })?;

        debug_utils.set("typeof", typeof_fn)?;

        globals.set("debug_utils", debug_utils)?;
        debug!("🔍 Debug utilities registered");
        Ok(())
    }

    /// Register random number utilities
    fn register_random_utils(lua: &Lua) -> ScriptResult<()> {
        let globals = lua.globals();
        
        let random_utils = lua.create_table()
            .map_err(|e| ScriptError::BindingFailed {
                reason: format!("Failed to create random table: {}", e)
            })?;

        // Random float in range
        let random_float_fn = lua.create_function(|_, (min_val, max_val): (Option<f64>, Option<f64>)| {
            let min_val = min_val.unwrap_or(0.0);
            let max_val = max_val.unwrap_or(1.0);
            let mut rng = rand::thread_rng();
            Ok(rng.gen_range(min_val..=max_val))
        })?;

        random_utils.set("float", random_float_fn)?;

        // Random integer in range
        let random_int_fn = lua.create_function(|_, (min_val, max_val): (Option<i64>, Option<i64>)| {
            let min_val = min_val.unwrap_or(0);
            let max_val = max_val.unwrap_or(100);
            let mut rng = rand::thread_rng();
            Ok(rng.gen_range(min_val..=max_val))
        })?;

        random_utils.set("int", random_int_fn)?;

        // Random boolean
        let random_bool_fn = lua.create_function(|_, probability: Option<f64>| {
            let probability = probability.unwrap_or(0.5).max(0.0).min(1.0);
            let mut rng = rand::thread_rng();
            Ok(rng.gen::<f64>() < probability)
        })?;

        random_utils.set("bool", random_bool_fn)?;

        // Choose random item from table
        let choice_fn = lua.create_function(|_, table: Table| {
            let items: Vec<Value> = table.pairs::<Value, Value>()
                .filter_map(|pair| pair.ok())
                .map(|(_, value)| value)
                .collect();

            if items.is_empty() {
                return Ok(Value::Nil);
            }

            let mut rng = rand::thread_rng();
            let index = rng.gen_range(0..items.len());
            Ok(items[index].clone())
        })?;

        random_utils.set("choice", choice_fn)?;

        globals.set("random", random_utils)?;
        debug!("🎲 Random utilities registered");
        Ok(())
    }
}

/// Lua script validation and safety checks
pub struct ScriptValidator;

impl ScriptValidator {
    /// Validate Lua script syntax without executing
    pub fn validate_syntax(lua: &Lua, script: &str, name: &str) -> ScriptResult<()> {
        // Check syntax by attempting to load the chunk
        // This will fail if there are syntax errors without executing the code
        let chunk = lua.load(script).set_name(name);
        
        // Try to get the function from the chunk, which validates syntax
        let _ = chunk.into_function().map_err(|e| ScriptError::CompilationFailed {
            reason: format!("Syntax error in {}: {}", name, e)
        })?;

        debug!("✅ Script {} passed syntax validation", name);
        Ok(())
    }

    /// Check for potentially unsafe patterns in script
    pub fn check_safety(script: &str) -> Vec<String> {
        let mut warnings = Vec::new();
        
        let unsafe_patterns = [
            ("while true", "Potential infinite loop detected"),
            ("for i=1,math.huge", "Potential infinite loop detected"),
            ("repeat", "Unbounded repeat loop detected"),
            ("goto", "Unsafe goto statement detected"),
            ("_G", "Global environment access detected"),
            ("getmetatable", "Metatable manipulation detected"),
            ("setmetatable", "Metatable manipulation detected"),
            ("coroutine", "Coroutine usage detected"),
        ];

        for (pattern, warning) in &unsafe_patterns {
            if script.contains(pattern) {
                warnings.push(warning.to_string());
            }
        }

        // Check for excessive nesting
        let max_nesting = script.matches("function").count() + 
                         script.matches("if").count() + 
                         script.matches("for").count() + 
                         script.matches("while").count();
        
        if max_nesting > 20 {
            warnings.push("Excessive control structure nesting detected".to_string());
        }

        // Check script length
        if script.len() > 100_000 {
            warnings.push("Script is very large and may impact performance".to_string());
        }

        warnings
    }
}

/// Convert Rust values to Lua values safely
pub fn rust_to_lua_value<'lua>(lua: &'lua Lua, value: &serde_json::Value) -> ScriptResult<Value<'lua>> {
    let lua_value = match value {
        serde_json::Value::Null => Value::Nil,
        serde_json::Value::Bool(b) => Value::Boolean(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Integer(i)
            } else if let Some(f) = n.as_f64() {
                Value::Number(f)
            } else {
                Value::Nil
            }
        },
        serde_json::Value::String(s) => Value::String(lua.create_string(s)?),
        serde_json::Value::Array(arr) => {
            let table = lua.create_table()?;
            for (i, item) in arr.iter().enumerate() {
                let lua_item = rust_to_lua_value(lua, item)?;
                table.set(i + 1, lua_item)?;
            }
            Value::Table(table)
        },
        serde_json::Value::Object(obj) => {
            let table = lua.create_table()?;
            for (key, val) in obj {
                let lua_val = rust_to_lua_value(lua, val)?;
                table.set(key.as_str(), lua_val)?;
            }
            Value::Table(table)
        },
    };

    Ok(lua_value)
}
