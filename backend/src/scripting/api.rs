//! Game API bindings for Lua scripts
//!
//! Exposes game entities, components, and systems to Lua scripts
//! with type-safe bindings and proper error handling.

use mlua::{Lua, Value, UserData, MetaMethod, UserDataMethods, FromLua};
use crate::core::zig_ffi::HexCoord;
use crate::world::tiles::components::{Tile, TerrainType, TileResource, Climate};
use serde::{Serialize, Deserialize};
use super::{ScriptResult, ScriptError};

/// Lua-compatible hex coordinate wrapper
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LuaHexCoord {
    pub q: i32,
    pub r: i32,
}

impl std::fmt::Display for LuaHexCoord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.q, self.r)
    }
}

impl From<HexCoord> for LuaHexCoord {
    fn from(hex: HexCoord) -> Self {
        Self { q: hex.q, r: hex.r }
    }
}

impl Into<HexCoord> for LuaHexCoord {
    fn into(self) -> HexCoord {
        HexCoord { q: self.q, r: self.r }
    }
}

impl<'lua> FromLua<'lua> for LuaHexCoord {
    fn from_lua(lua_value: Value<'lua>, _lua: &'lua Lua) -> mlua::Result<Self> {
        match lua_value {
            Value::UserData(ud) => Ok(*ud.borrow::<LuaHexCoord>()?),
            Value::Table(table) => {
                let q: i32 = table.get("q")?;
                let r: i32 = table.get("r")?;
                Ok(LuaHexCoord { q, r })
            }
            _ => Err(mlua::Error::FromLuaConversionError {
                from: lua_value.type_name(),
                to: "LuaHexCoord",
                message: Some("expected userdata or table with q,r fields".to_string()),
            }),
        }
    }
}

impl UserData for LuaHexCoord {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("distance", |_, this, other: LuaHexCoord| {
            Ok(((this.q - other.q).abs() + (this.q + this.r - other.q - other.r).abs() + (this.r - other.r).abs()) / 2)
        });

        methods.add_method("to_string", |_, this, ()| {
            Ok(format!("({}, {})", this.q, this.r))
        });

        methods.add_meta_method(MetaMethod::ToString, |_, this, ()| {
            Ok(format!("Hex({}, {})", this.q, this.r))
        });

        methods.add_meta_method(MetaMethod::Eq, |_, this, other: LuaHexCoord| {
            Ok(this.q == other.q && this.r == other.r)
        });
    }
}

/// Lua-compatible tile data wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LuaTile {
    pub id: u64,
    pub hex: LuaHexCoord,
    pub terrain: String,
    pub elevation: f32,
}

impl From<&Tile> for LuaTile {
    fn from(tile: &Tile) -> Self {
        Self {
            id: tile.id.0 as u64,  // Convert u32 to u64 for Lua compatibility
            hex: tile.hex.into(),
            terrain: format!("{:?}", tile.terrain_type).to_lowercase(),
            elevation: tile.elevation,
        }
    }
}

impl UserData for LuaTile {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("get_hex", |_, this, ()| {
            Ok(this.hex)
        });

        methods.add_method("get_terrain", |_, this, ()| {
            Ok(this.terrain.clone())
        });

        methods.add_method("get_elevation", |_, this, ()| {
            Ok(this.elevation)
        });

        methods.add_method("is_water", |_, this, ()| {
            Ok(this.terrain == "ocean")
        });

        methods.add_method("is_land", |_, this, ()| {
            Ok(this.terrain != "ocean")
        });

        methods.add_meta_method(MetaMethod::ToString, |_, this, ()| {
            Ok(format!("Tile[{}] {} at {}", this.id, this.terrain, this.hex.to_string()))
        });
    }
}

/// Lua-compatible resource data  
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LuaResource {
    pub resource_type: String,
    pub quantity: u8,
    pub quality: f32,
}

impl From<&TileResource> for LuaResource {
    fn from(resource: &TileResource) -> Self {
        Self {
            resource_type: format!("{:?}", resource.resource_type).to_lowercase(),
            quantity: resource.quantity,
            quality: 1.0 - resource.depletion_rate.min(1.0), // Higher quality = lower depletion rate
        }
    }
}

impl UserData for LuaResource {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("get_type", |_, this, ()| {
            Ok(this.resource_type.clone())
        });

        methods.add_method("get_quantity", |_, this, ()| {
            Ok(this.quantity)
        });

        methods.add_method("get_quality", |_, this, ()| {
            Ok(this.quality)
        });

        methods.add_method("is_depleted", |_, this, ()| {
            Ok(this.quantity == 0)
        });

        methods.add_meta_method(MetaMethod::ToString, |_, this, ()| {
            Ok(format!("{} x{} ({}%)", this.resource_type, this.quantity, (this.quality * 100.0) as u8))
        });
    }
}

/// Lua-compatible climate data
#[derive(Debug, Clone, Serialize, Deserialize)]  
pub struct LuaClimate {
    pub temperature: i8,
    pub rainfall: u8,
    pub humidity: u8,
    pub wind_strength: u8,
}

impl From<&Climate> for LuaClimate {
    fn from(climate: &Climate) -> Self {
        Self {
            temperature: climate.temperature,
            rainfall: climate.rainfall,
            humidity: climate.humidity,
            wind_strength: climate.wind_strength,
        }
    }
}

impl UserData for LuaClimate {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("get_temperature", |_, this, ()| {
            Ok(this.temperature)
        });

        methods.add_method("get_rainfall", |_, this, ()| {
            Ok(this.rainfall)
        });

        methods.add_method("is_tropical", |_, this, ()| {
            Ok(this.temperature > 20 && this.rainfall > 150)
        });

        methods.add_method("is_arid", |_, this, ()| {
            Ok(this.rainfall < 50)
        });

        methods.add_method("is_temperate", |_, this, ()| {
            Ok(this.temperature >= 0 && this.temperature <= 20)
        });

        methods.add_meta_method(MetaMethod::ToString, |_, this, ()| {
            Ok(format!("{}°C, {}mm rain", this.temperature, this.rainfall))
        });
    }
}

/// Register all game API types with Lua
pub fn register_game_types(lua: &Lua) -> ScriptResult<()> {
    let globals = lua.globals();

    // Create constructors for game types
    let hex_constructor = lua.create_function(|_, (q, r): (i32, i32)| {
        Ok(LuaHexCoord { q, r })
    }).map_err(|e| ScriptError::BindingFailed {
        reason: format!("Failed to create Hex constructor: {}", e)
    })?;

    globals.set("Hex", hex_constructor)
        .map_err(|e| ScriptError::BindingFailed {
            reason: format!("Failed to bind Hex constructor: {}", e)
        })?;

    Ok(())
}

/// Utility functions for common game calculations
pub fn register_game_utils(lua: &Lua) -> ScriptResult<()> {
    let globals = lua.globals();
    
    let utils_table = lua.create_table()
        .map_err(|e| ScriptError::BindingFailed {
            reason: format!("Failed to create utils table: {}", e)
        })?;

    // Distance calculation
    let distance_fn = lua.create_function(|_, (hex1, hex2): (LuaHexCoord, LuaHexCoord)| {
        Ok(((hex1.q - hex2.q).abs() + (hex1.q + hex1.r - hex2.q - hex2.r).abs() + (hex1.r - hex2.r).abs()) / 2)
    }).map_err(|e| ScriptError::BindingFailed {
        reason: format!("Failed to create distance function: {}", e)
    })?;

    utils_table.set("distance", distance_fn)
        .map_err(|e| ScriptError::BindingFailed {
            reason: format!("Failed to bind distance function: {}", e)
        })?;

    // Hex neighbors
    let neighbors_fn = lua.create_function(|_, hex: LuaHexCoord| {
        let directions = [
            (1, 0), (1, -1), (0, -1),
            (-1, 0), (-1, 1), (0, 1)
        ];
        
        let neighbors: Vec<LuaHexCoord> = directions.iter()
            .map(|(dq, dr)| LuaHexCoord { q: hex.q + dq, r: hex.r + dr })
            .collect();
            
        Ok(neighbors)
    }).map_err(|e| ScriptError::BindingFailed {
        reason: format!("Failed to create neighbors function: {}", e)
    })?;

    utils_table.set("neighbors", neighbors_fn)
        .map_err(|e| ScriptError::BindingFailed {
            reason: format!("Failed to bind neighbors function: {}", e)
        })?;

    globals.set("Utils", utils_table)
        .map_err(|e| ScriptError::BindingFailed {
            reason: format!("Failed to bind Utils table: {}", e)
        })?;

    Ok(())
}

/// Terrain type conversion utilities
pub fn terrain_type_to_string(terrain: TerrainType) -> String {
    match terrain {
        TerrainType::Ocean => "ocean".to_string(),
        TerrainType::Grassland => "grassland".to_string(),
        TerrainType::Plains => "plains".to_string(),
        TerrainType::Desert => "desert".to_string(),
        TerrainType::Tundra => "tundra".to_string(),
        TerrainType::Snow => "snow".to_string(),
        TerrainType::Forest => "forest".to_string(),
        TerrainType::Jungle => "jungle".to_string(),
        TerrainType::Hills => "hills".to_string(),
        TerrainType::Mountain => "mountain".to_string(),
        TerrainType::Mountains => "mountains".to_string(), // Alias for Mountain
        TerrainType::River => "river".to_string(),
        TerrainType::Coast => "coast".to_string(),
    }
}

pub fn string_to_terrain_type(s: &str) -> Option<TerrainType> {
    match s.to_lowercase().as_str() {
        "ocean" => Some(TerrainType::Ocean),
        "grassland" => Some(TerrainType::Grassland),
        "plains" => Some(TerrainType::Plains),
        "desert" => Some(TerrainType::Desert),
        "tundra" => Some(TerrainType::Tundra),
        "snow" => Some(TerrainType::Snow),
        "forest" => Some(TerrainType::Forest),
        "jungle" => Some(TerrainType::Jungle),
        "hills" => Some(TerrainType::Hills),
        "mountain" => Some(TerrainType::Mountain),
        _ => None,
    }
}
