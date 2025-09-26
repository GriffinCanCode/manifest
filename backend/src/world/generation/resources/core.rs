//! Core resource distribution system
//!
//! Main system coordinator that integrates Lua scripting, geological analysis,
//! and parallel processing for realistic resource distribution.

use std::sync::Arc;
use std::collections::HashMap;
use parking_lot::RwLock;
use rayon::prelude::*;
use bevy_ecs::prelude::*;
use tracing::{info, debug, warn, error};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::scripting::{ComprehensiveLuaHandler, ScriptResult, LuaEventData};
use crate::world::tiles::TileId;
use crate::world::generation::noise::NoiseGenerator;
use crate::world::generation::tectonics::TectonicPlate;
use crate::core::scheduler::Scheduler;

use super::types::*;
use super::lua::ResourceLuaApi;
use super::{ResourceDistributionError, ResourceResult};

/// Main resource distribution system coordinator
pub struct ResourceDistributionSystem {
    /// Lua scripting handler for rule evaluation
    lua_handler: Arc<ComprehensiveLuaHandler>,
    /// Resource type definitions loaded from Lua
    resource_types: Arc<RwLock<HashMap<String, ResourceType>>>,
    /// Distribution rules cache
    distribution_cache: Arc<RwLock<HashMap<String, Vec<DistributionRule>>>>,
    /// Noise generator for procedural placement
    noise_generator: Arc<NoiseGenerator>,
    /// Deterministic RNG for consistent results
    rng: ChaCha8Rng,
    /// Parallel scheduler for performance
    scheduler: Arc<Scheduler>,
}

/// Compiled distribution rule from Lua
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DistributionRule {
    pub resource_type: String,
    pub weight: f32,
    pub conditions: Vec<PlacementCondition>,
    pub placement_algorithm: PlacementAlgorithm,
}

/// Placement condition for resource distribution
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PlacementCondition {
    /// Terrain type requirement
    TerrainType { terrain: String, affinity: f32 },
    /// Elevation requirement
    Elevation { min: f32, max: f32 },
    /// Tectonic feature requirement
    TectonicFeature { feature: String, distance: f32 },
    /// Climate requirement
    Climate { temperature: Option<(f32, f32)>, rainfall: Option<(f32, f32)> },
    /// Noise-based probability
    NoiseThreshold { noise_type: String, min: f32, max: f32 },
}

/// Algorithm for placing resources
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PlacementAlgorithm {
    /// Scattered individual deposits
    Scattered { density: f32 },
    /// Clustered deposits
    Clustered { cluster_size: u32, density: f32 },
    /// Linear veins
    LinearVein { length: u32, width: u32 },
    /// Circular/oval deposits
    Circular { radius: u32 },
    /// Following geological features
    Geological { feature_type: String },
}

impl ResourceDistributionSystem {
    /// Create new resource distribution system
    pub fn new(seed: u64) -> ResourceResult<Self> {
        info!("🏭 Initializing Resource Distribution System...");
        
        // Create Lua handler for rule evaluation
        let lua_handler = Arc::new(ComprehensiveLuaHandler::new(true)?);
        
        // Initialize API bindings
        ResourceLuaApi::register(&lua_handler)?;
        
        // Create noise generator for procedural placement
        let mut noise_config = crate::world::generation::noise::NoiseConfig::default();
        noise_config.seed = seed;
        let noise_generator = Arc::new(NoiseGenerator::new(&noise_config));
        
        // Create deterministic RNG
        let rng = ChaCha8Rng::seed_from_u64(seed);
        
        // Create parallel scheduler
        let scheduler = Arc::new(crate::core::scheduler::Scheduler::new(None)
            .map_err(|e| ResourceDistributionError::ConfigError(format!("Failed to create scheduler: {}", e)))?);
        
        Ok(Self {
            lua_handler,
            resource_types: Arc::new(RwLock::new(HashMap::new())),
            distribution_cache: Arc::new(RwLock::new(HashMap::new())),
            noise_generator,
            rng,
            scheduler,
        })
    }
    
    /// Load resource configurations from Lua scripts
    pub fn load_resource_configs(&mut self, script_dir: &str) -> ResourceResult<()> {
        info!("📋 Loading resource configurations from {}", script_dir);
        
        // Load main resource configuration
        let main_config = format!("{}/main.lua", script_dir);
        self.lua_handler.load_script(&main_config)?;
        
        // Load distribution rules
        let rules_config = format!("{}/distribution.lua", script_dir);
        self.lua_handler.load_script(&rules_config)?;
        
        // Load economic data
        let economics_config = format!("{}/economics.lua", script_dir);
        self.lua_handler.load_script(&economics_config)?;
        
        // Execute configuration scripts and cache results
        self.cache_resource_types()?;
        self.cache_distribution_rules()?;
        
        info!("✅ Resource configurations loaded successfully");
        Ok(())
    }
    
    /// Distribute resources across the world using parallel processing
    pub fn distribute_resources(
        &mut self,
        world: &mut World,
        tectonic_data: &TectonicPlate,
        chunk_size: usize,
    ) -> ResourceResult<ResourceDistributionStats> {
        info!("🌍 Beginning world resource distribution...");
        
        let start_time = std::time::Instant::now();
        let mut stats = ResourceDistributionStats::new();
        
        // Get all tile positions that need resource evaluation
        let tile_positions: Vec<(TileId, crate::core::zig_ffi::HexCoord)> = world
            .query::<(&TileId, &crate::world::tiles::components::core::Tile)>()
            .iter(world)
            .map(|(_, tile)| (tile.id, tile.hex))
            .collect();
        
        info!("🔍 Evaluating {} tile positions for resources", tile_positions.len());
        
        // Process tiles in parallel chunks
        let distribution_results: Vec<_> = tile_positions
            .par_chunks(chunk_size)
            .map(|chunk| {
                self.process_tile_chunk(chunk, tectonic_data)
            })
            .collect();
        
        // Aggregate results and apply to world
        for chunk_result in distribution_results {
            let chunk_stats = chunk_result?;
            stats.merge(chunk_stats);
        }
        
        // Apply resource deposits to world entities
        self.apply_resource_deposits(world, stats.deposits.clone())?;
        
        // Generate resource veins connecting related deposits
        self.generate_resource_veins(world, &stats.deposits)?;
        
        let distribution_time = start_time.elapsed();
        stats.generation_time_ms = distribution_time.as_millis() as u64;
        
        info!(
            "✅ Resource distribution completed in {:.2}s. Placed {} deposits across {} types",
            distribution_time.as_secs_f64(),
            stats.total_deposits,
            stats.resource_type_counts.len()
        );
        
        Ok(stats)
    }
    
    /// Process a chunk of tiles for resource placement
    fn process_tile_chunk(
        &self,
        chunk: &[(TileId, crate::core::zig_ffi::HexCoord)],
        tectonic_data: &TectonicPlate,
    ) -> ResourceResult<ResourceDistributionStats> {
        let mut chunk_stats = ResourceDistributionStats::new();
        let resource_types = self.resource_types.read();
        let distribution_rules = self.distribution_cache.read();
        
        for (tile_id, hex_coord) in chunk {
            // Evaluate each resource type for this tile
            for (resource_id, resource_type) in resource_types.iter() {
                if let Some(rules) = distribution_rules.get(resource_id) {
                    if let Some(deposit) = self.evaluate_resource_placement(
                        hex_coord,
                        resource_type,
                        rules,
                        tectonic_data,
                    )? {
                        chunk_stats.add_deposit(*tile_id, deposit);
                    }
                }
            }
        }
        
        Ok(chunk_stats)
    }
    
    /// Evaluate whether to place a resource at a specific tile
    fn evaluate_resource_placement(
        &self,
        tile_pos: &crate::core::zig_ffi::HexCoord,
        resource_type: &ResourceType,
        rules: &[DistributionRule],
        tectonic_data: &TectonicPlate,
    ) -> ResourceResult<Option<ResourceDeposit>> {
        // Use Lua to evaluate placement rules
        let lua_result: bool = self.lua_handler.call_function(
            "evaluate_resource_placement",
            (
                tile_pos.q,
                tile_pos.r,
                resource_type.id.clone(),
                serde_json::to_string(&resource_type.distribution)?,
            ),
        )?;
        
        if !lua_result {
            return Ok(None);
        }
        
        // Calculate resource properties using Lua
        let (quantity, quality): (u8, f32) = self.lua_handler.call_function(
            "calculate_resource_properties",
            (
                tile_pos.q,
                tile_pos.r,
                resource_type.id.clone(),
                self.get_noise_value(tile_pos, &resource_type.id),
            ),
        )?;
        
        let deposit = ResourceDeposit {
            resource_type: resource_type.id.clone(),
            quantity,
            quality,
            discovered: false,
            discovery_difficulty: resource_type.properties.discovery_difficulty,
            depletion_state: DepletionState {
                original_quantity: quantity,
                ..Default::default()
            },
            extraction_modifiers: ExtractionModifiers::default(),
        };
        
        Ok(Some(deposit))
    }
    
    /// Get noise value for resource placement
    fn get_noise_value(&self, hex_pos: &crate::core::zig_ffi::HexCoord, resource_type: &str) -> f32 {
        // Use hex coordinates directly for better spatial distribution
        let x = hex_pos.q as f64;
        let y = hex_pos.r as f64;
        
        // Use different noise octaves for different resource types
        let hash = crate::core::hashing::HashStrategies::hash_string(resource_type);
        let seed_offset = (hash % 1000) as f64;
        
        self.noise_generator.sample_2d(x + seed_offset, y + seed_offset)
    }
    
    /// Apply resource deposits to world entities
    fn apply_resource_deposits(
        &self,
        world: &mut World,
        deposits: HashMap<TileId, Vec<ResourceDeposit>>,
    ) -> ResourceResult<()> {
        debug!("📍 Applying {} deposit locations to world", deposits.len());
        
        for (tile_pos, tile_deposits) in deposits {
            // Find the entity for this tile position
            let tile_entity = world
                .query::<(Entity, &TileId)>()
                .iter(world)
                .find(|(_, pos)| **pos == tile_pos)
                .map(|(entity, _)| entity);
            
            if let Some(entity) = tile_entity {
                // Add resource deposits as components
                for deposit in tile_deposits {
                    let mut entity_mut = world.entity_mut(entity);
                    entity_mut.insert(deposit);
                }
            }
        }
        
        Ok(())
    }
    
    /// Generate resource veins connecting related deposits
    fn generate_resource_veins(
        &self,
        world: &mut World,
        deposits: &HashMap<TileId, Vec<ResourceDeposit>>,
    ) -> ResourceResult<()> {
        debug!("🔗 Generating resource veins...");
        
        // Group deposits by resource type
        let mut resource_groups: HashMap<String, Vec<TileId>> = HashMap::new();
        
        for (tile_pos, tile_deposits) in deposits {
            for deposit in tile_deposits {
                resource_groups
                    .entry(deposit.resource_type.clone())
                    .or_default()
                    .push(tile_pos.clone());
            }
        }
        
        // Generate veins for each resource type using Lua rules
        for (resource_type, positions) in resource_groups {
            if positions.len() > 1 {
                // Convert positions to Lua-compatible format
                let lua_positions = serde_json::json!(positions.iter().map(|p| {
                    let id = p.0;
                    vec![(id % 1000) as i32, (id / 1000) as i32]
                }).collect::<Vec<_>>());
                
                // Convert to Lua-compatible types - use owned String instead of reference
                let veins: Vec<ResourceVein> = self.lua_handler.call_function(
                    "generate_resource_veins",
                    (resource_type.clone(), lua_positions.to_string()),
                )?;
                
                // Apply veins to world entities
                for vein in veins {
                    // Create a new entity for the vein
                    let vein_entity = world.spawn(vein).id();
                    debug!("Created resource vein entity: {:?}", vein_entity);
                }
            }
        }
        
        Ok(())
    }
    
    /// Cache resource types from Lua configuration
    fn cache_resource_types(&self) -> ResourceResult<()> {
        let resource_data: String = self.lua_handler.call_function("get_resource_types", ())?;
        let types: HashMap<String, ResourceType> = serde_json::from_str(&resource_data)
            .map_err(|e| ResourceDistributionError::ConfigError(e.to_string()))?;
        
        let mut resource_types = self.resource_types.write();
        *resource_types = types;
        
        info!("💾 Cached {} resource types", resource_types.len());
        Ok(())
    }
    
    /// Cache distribution rules from Lua
    fn cache_distribution_rules(&self) -> ResourceResult<()> {
        let rules_data: String = self.lua_handler.call_function("get_distribution_rules", ())?;
        let rules: HashMap<String, Vec<DistributionRule>> = serde_json::from_str(&rules_data)
            .map_err(|e| ResourceDistributionError::ConfigError(e.to_string()))?;
        
        let mut distribution_cache = self.distribution_cache.write();
        *distribution_cache = rules;
        
        info!("📐 Cached distribution rules for {} resource types", distribution_cache.len());
        Ok(())
    }
}

/// Statistics from resource distribution process
#[derive(Debug, Clone, Default)]
pub struct ResourceDistributionStats {
    /// Total number of deposits placed
    pub total_deposits: u64,
    /// Deposits by resource type
    pub resource_type_counts: HashMap<String, u32>,
    /// All placed deposits by position
    pub deposits: HashMap<TileId, Vec<ResourceDeposit>>,
    /// Generation time in milliseconds
    pub generation_time_ms: u64,
}

impl ResourceDistributionStats {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn add_deposit(&mut self, position: TileId, deposit: ResourceDeposit) {
        *self.resource_type_counts.entry(deposit.resource_type.clone()).or_insert(0) += 1;
        self.total_deposits += 1;
        self.deposits.entry(position).or_default().push(deposit);
    }
    
    pub fn merge(&mut self, other: ResourceDistributionStats) {
        self.total_deposits += other.total_deposits;
        
        for (resource_type, count) in other.resource_type_counts {
            *self.resource_type_counts.entry(resource_type).or_insert(0) += count;
        }
        
        for (position, deposits) in other.deposits {
            self.deposits.entry(position).or_default().extend(deposits);
        }
    }
}
