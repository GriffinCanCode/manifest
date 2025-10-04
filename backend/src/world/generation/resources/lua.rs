//! Lua API integration for resource distribution system
//!
//! Provides comprehensive Lua API for configurable resource rules,
//! geological analysis, and procedural distribution algorithms.

use mlua::{UserData, UserDataMethods};
use serde_json;
use tracing::{debug, warn};

use crate::scripting::ComprehensiveLuaHandler;
use super::types::*;
use super::ResourceResult;

/// Lua API for resource distribution system
pub struct ResourceLuaApi;

impl ResourceLuaApi {
    /// Register all resource-related Lua functions
    pub fn register(lua_handler: &ComprehensiveLuaHandler) -> ResourceResult<()> {
        debug!("🔧 Registering resource distribution Lua API...");
        
        // Register core resource functions
        Self::register_resource_functions(lua_handler)?;
        
        // Register geological analysis functions  
        Self::register_geological_functions(lua_handler)?;
        
        // Register distribution algorithms
        Self::register_distribution_functions(lua_handler)?;
        
        // Register utility functions
        Self::register_utility_functions(lua_handler)?;
        
        debug!("✅ Resource Lua API registered successfully");
        Ok(())
    }
    
    /// Register core resource management functions
    fn register_resource_functions(lua_handler: &ComprehensiveLuaHandler) -> ResourceResult<()> {
        let lua = lua_handler.environment().read().create_lua_vm()?;
        let globals = lua.globals();
        
        // Create Resources namespace
        let resources_table = lua.create_table()?;
        
        // Resource validation function
        let validate_resource = lua.create_function(|_, (resource_type, properties): (String, String)| {
            match serde_json::from_str::<ResourceType>(&properties) {
                Ok(resource) => Ok(true),
                Err(e) => {
                    warn!("Resource validation failed for {}: {}", resource_type, e);
                    Ok(false)
                }
            }
        })?;
        resources_table.set("validate", validate_resource)?;
        
        // Resource quality calculation
        let calculate_quality = lua.create_function(|_, (base_quality, noise_value, modifiers): (f32, f32, f32)| {
            let quality = (base_quality + noise_value * 0.3 + modifiers * 0.2).clamp(0.0, 1.0);
            Ok(quality)
        })?;
        resources_table.set("calculate_quality", calculate_quality)?;
        
        // Resource quantity calculation
        let calculate_quantity = lua.create_function(|_, (base_amount, quality, rarity): (f32, f32, f32)| {
            let quantity = ((base_amount * quality * (1.0 - rarity)) * 255.0) as u8;
            Ok(quantity)
        })?;
        resources_table.set("calculate_quantity", calculate_quantity)?;
        
        globals.set("Resources", resources_table)?;
        Ok(())
    }
    
    /// Register geological analysis functions
    fn register_geological_functions(lua_handler: &ComprehensiveLuaHandler) -> ResourceResult<()> {
        let lua = lua_handler.environment().read().create_lua_vm()?;
        let globals = lua.globals();
        
        let geology_table = lua.create_table()?;
        
        // Elevation-based resource probability
        let elevation_affinity = lua.create_function(|_, (elevation, min_elev, max_elev): (f32, f32, f32)| {
            if elevation >= min_elev && elevation <= max_elev {
                // Peak affinity at optimal elevation
                let optimal = (min_elev + max_elev) / 2.0;
                let distance = (elevation - optimal).abs();
                let range = (max_elev - min_elev) / 2.0;
                Ok(1.0 - (distance / range))
            } else {
                Ok(0.0)
            }
        })?;
        geology_table.set("elevation_affinity", elevation_affinity)?;
        
        // Tectonic feature distance analysis
        let tectonic_proximity = lua.create_function(|_, (tile_x, tile_y, feature_x, feature_y, optimal_distance): (f32, f32, f32, f32, f32)| {
            let distance = ((tile_x - feature_x).powi(2) + (tile_y - feature_y).powi(2)).sqrt();
            let proximity = (-((distance - optimal_distance) / optimal_distance).powi(2)).exp();
            Ok(proximity)
        })?;
        geology_table.set("tectonic_proximity", tectonic_proximity)?;
        
        // Plate age resource correlation
        let plate_age_affinity = lua.create_function(|_, (plate_age, min_age, max_age): (f32, f32, f32)| {
            if plate_age >= min_age && plate_age <= max_age {
                Ok(1.0)
            } else if plate_age < min_age {
                Ok((plate_age / min_age).powf(0.5))
            } else {
                Ok((max_age / plate_age).powf(0.5))
            }
        })?;
        geology_table.set("plate_age_affinity", plate_age_affinity)?;
        
        globals.set("Geology", geology_table)?;
        Ok(())
    }
    
    /// Register distribution algorithm functions
    fn register_distribution_functions(lua_handler: &ComprehensiveLuaHandler) -> ResourceResult<()> {
        let lua = lua_handler.environment().read().create_lua_vm()?;
        let globals = lua.globals();
        
        let distribution_table = lua.create_table()?;
        
        // Clustering algorithm
        let calculate_clustering = lua.create_function(|lua, args: mlua::Value| -> mlua::Result<mlua::Table> {
            // Parse arguments manually from Lua table
            if let mlua::Value::Table(table) = args {
                let positions_val = table.get::<_, mlua::Value>(1)?;
                let cluster_radius: f32 = table.get(2).unwrap_or(10.0);
                let cluster_tendency: f32 = table.get(3).unwrap_or(0.5);
                
                // Parse positions from Lua array
                let mut positions = Vec::new();
                if let mlua::Value::Table(pos_table) = positions_val {
                    for i in 1..=pos_table.len()? {
                        if let Ok(mlua::Value::Table(pos)) = pos_table.get::<_, mlua::Value>(i) {
                            let x: i32 = pos.get(1).unwrap_or(0);
                            let y: i32 = pos.get(2).unwrap_or(0);
                            positions.push((x, y));
                        }
                    }
                }
                
                // Clustering algorithm
                let mut clusters = Vec::new();
            let mut processed = std::collections::HashSet::new();
            
            for (i, &pos) in positions.iter().enumerate() {
                if processed.contains(&i) {
                    continue;
                }
                
                let mut cluster = vec![pos];
                processed.insert(i);
                
                // Find nearby positions
                for (j, &other_pos) in positions.iter().enumerate() {
                    if i != j && !processed.contains(&j) {
                        let distance = ((pos.0 - other_pos.0).pow(2) + (pos.1 - other_pos.1).pow(2)) as f32;
                        if distance <= cluster_radius * cluster_radius {
                            cluster.push(other_pos);
                            processed.insert(j);
                        }
                    }
                }
                
                if cluster.len() > 1 || rand::random::<f32>() < cluster_tendency {
                    clusters.push(cluster);
                }
            }
            
                // Convert clusters to Lua table
                let result_table = lua.create_table()?;
                for (i, cluster) in clusters.into_iter().enumerate() {
                    let cluster_table = lua.create_table()?;
                    for (j, pos) in cluster.into_iter().enumerate() {
                        let pos_table = lua.create_table()?;
                        pos_table.set(1, pos.0)?;
                        pos_table.set(2, pos.1)?;
                        cluster_table.set(j + 1, pos_table)?;
                    }
                    result_table.set(i + 1, cluster_table)?;
                }
                Ok(result_table)
            } else {
                Ok(lua.create_table()?)
            }
        })?;
        distribution_table.set("calculate_clustering", calculate_clustering)?;
        
        // Linear vein generation
        let generate_vein = lua.create_function(|lua, args: mlua::Value| -> mlua::Result<mlua::Table> {
            // Parse arguments manually from Lua
            if let mlua::Value::Table(table) = args {
                let start_pos_val = table.get::<_, mlua::Value>(1)?;
                let direction: f32 = table.get(2).unwrap_or(0.0);
                let length: u32 = table.get(3).unwrap_or(10);
                let width: u32 = table.get(4).unwrap_or(1);
                
                let (start_x, start_y) = if let mlua::Value::Table(pos_table) = start_pos_val {
                    let x: i32 = pos_table.get(1).unwrap_or(0);
                    let y: i32 = pos_table.get(2).unwrap_or(0);
                    (x, y)
                } else {
                    (0, 0)
                };
                
                let mut vein_positions = Vec::new();
            
            for i in 0..length {
                let progress = i as f32 / length as f32;
                let base_x = start_x as f32 + direction.cos() * progress * length as f32;
                let base_y = start_y as f32 + direction.sin() * progress * length as f32;
                
                // Add width variation
                for w in 0..width {
                    let offset = (w as f32 - width as f32 / 2.0) / width as f32;
                    let vein_x = (base_x + direction.sin() * offset * width as f32) as i32;
                    let vein_y = (base_y - direction.cos() * offset * width as f32) as i32;
                    vein_positions.push((vein_x, vein_y));
                }
            }
            
                // Convert to Lua table
                let result_table = lua.create_table()?;
                for (i, pos) in vein_positions.into_iter().enumerate() {
                    let pos_table = lua.create_table()?;
                    pos_table.set(1, pos.0)?;
                    pos_table.set(2, pos.1)?;
                    result_table.set(i + 1, pos_table)?;
                }
                Ok(result_table)
            } else {
                Ok(lua.create_table()?)
            }
        })?;
        distribution_table.set("generate_vein", generate_vein)?;
        
        globals.set("Distribution", distribution_table)?;
        Ok(())
    }
    
    /// Register utility functions
    fn register_utility_functions(lua_handler: &ComprehensiveLuaHandler) -> ResourceResult<()> {
        let lua = lua_handler.environment().read().create_lua_vm()?;
        let globals = lua.globals();
        
        let utils_table = lua.create_table()?;
        
        // Noise sampling helper
        let sample_noise = lua.create_function(|_, (x, y, scale, octaves): (f32, f32, f32, u32)| {
            // Simple noise implementation for Lua
            let mut value = 0.0;
            let mut amplitude = 1.0;
            let mut frequency = scale;
            
            for _ in 0..octaves {
                let nx = x * frequency;
                let ny = y * frequency;
                
                // Simple hash-based noise
                let hash = ((nx as i32).wrapping_mul(374761393)
                    .wrapping_add((ny as i32).wrapping_mul(668265263))) as u32;
                let noise_val = (hash as f32 / u32::MAX as f32) * 2.0 - 1.0;
                
                value += noise_val * amplitude;
                amplitude *= 0.5;
                frequency *= 2.0;
            }
            
            Ok(value.clamp(-1.0, 1.0))
        })?;
        utils_table.set("sample_noise", sample_noise)?;
        
        // Distance calculation
        let hex_distance = lua.create_function(|_, (q1, r1, q2, r2): (i32, i32, i32, i32)| {
            let dq = q1 - q2;
            let dr = r1 - r2;
            let distance = ((dq.abs() + (dq + dr).abs() + dr.abs()) / 2) as f32;
            Ok(distance)
        })?;
        utils_table.set("hex_distance", hex_distance)?;
        
        // Interpolation helpers
        let lerp = lua.create_function(|_, (a, b, t): (f32, f32, f32)| {
            Ok(a + (b - a) * t.clamp(0.0, 1.0))
        })?;
        utils_table.set("lerp", lerp)?;
        
        let smoothstep = lua.create_function(|_, (x, min, max): (f32, f32, f32)| {
            let t = ((x - min) / (max - min)).clamp(0.0, 1.0);
            Ok(t * t * (3.0 - 2.0 * t))
        })?;
        utils_table.set("smoothstep", smoothstep)?;
        
        globals.set("Utils", utils_table)?;
        Ok(())
    }
}

/// Lua wrapper for tile position
#[derive(Debug, Clone)]
pub struct LuaTilePosition {
    pub q: i32,
    pub r: i32,
}

impl<'lua> mlua::FromLua<'lua> for LuaTilePosition {
    fn from_lua(lua_value: mlua::Value<'lua>, _lua: &'lua mlua::Lua) -> mlua::Result<Self> {
        match lua_value {
            mlua::Value::UserData(ud) => {
                Ok(ud.borrow::<LuaTilePosition>()?.clone())
            }
            mlua::Value::Table(table) => {
                let q = table.get("q").or_else(|_| table.get(1)).unwrap_or(0);
                let r = table.get("r").or_else(|_| table.get(2)).unwrap_or(0);
                Ok(LuaTilePosition { q, r })
            }
            _ => Err(mlua::Error::FromLuaConversionError {
                from: lua_value.type_name(),
                to: "LuaTilePosition",
                message: Some("expected LuaTilePosition or table".to_string()),
            }),
        }
    }
}

impl UserData for LuaTilePosition {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("distance_to", |_, this, other: LuaTilePosition| {
            let dq = this.q - other.q;
            let dr = this.r - other.r;
            Ok(((dq.abs() + (dq + dr).abs() + dr.abs()) / 2) as f32)
        });
        
        methods.add_method("neighbors", |lua, this, ()| {
            let neighbors = vec![
                LuaTilePosition { q: this.q + 1, r: this.r },
                LuaTilePosition { q: this.q - 1, r: this.r },
                LuaTilePosition { q: this.q, r: this.r + 1 },
                LuaTilePosition { q: this.q, r: this.r - 1 },
                LuaTilePosition { q: this.q + 1, r: this.r - 1 },
                LuaTilePosition { q: this.q - 1, r: this.r + 1 },
            ];
            Ok(neighbors)
        });
    }
}

/// Lua wrapper for resource deposit data
#[derive(Debug, Clone)]
pub struct LuaResourceDeposit {
    pub resource_type: String,
    pub quantity: u8,
    pub quality: f32,
    pub discovered: bool,
}

impl UserData for LuaResourceDeposit {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("get_value", |_, this, market_price: f32| {
            Ok(this.quantity as f32 * this.quality * market_price)
        });
        
        methods.add_method("extraction_time", |_, this, extraction_rate: f32| {
            if extraction_rate > 0.0 {
                Ok((this.quantity as f32 / extraction_rate) as u32)
            } else {
                Ok(0)
            }
        });
    }
}
