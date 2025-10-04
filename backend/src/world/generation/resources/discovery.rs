//! Resource discovery and exploration system
//!
//! Implements realistic resource discovery mechanics using behavior trees,
//! technological progression, and exploration strategies.

use std::collections::{HashMap, VecDeque};
use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use tracing::{debug, info};

use crate::world::tiles::TileId;
use crate::core::zig_ffi::HexCoord;
use crate::scripting::{ComprehensiveLuaHandler, LuaEventData, LuaEventValue};

use super::types::*;
use super::{ResourceResult, ResourceDistributionError};

/// Resource discovery system with technology-based progression
pub struct ResourceDiscoverySystem {
    /// Lua handler for discovery rules
    lua_handler: ComprehensiveLuaHandler,
    /// Discovery state tracking
    discovery_states: HashMap<Entity, DiscoveryState>,
    /// Global discovery progress by civilization
    civilization_discovery: HashMap<u32, CivilizationDiscovery>,
    /// Discovery queue for processing
    discovery_queue: VecDeque<DiscoveryTask>,
    /// RNG for discovery events
    rng: ChaCha8Rng,
}

/// Discovery state for a specific resource deposit
#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub struct DiscoveryState {
    /// Base difficulty to discover this resource
    pub base_difficulty: f32,
    /// Current discovery progress (0.0 to 1.0)
    pub progress: f32,
    /// Technologies that aid discovery
    pub helpful_technologies: Vec<String>,
    /// Civilizations that have discovered this resource
    pub discovered_by: Vec<u32>,
    /// Discovery method used
    pub discovery_method: Option<DiscoveryMethod>,
    /// Discovery turn/timestamp
    pub discovery_turn: Option<u32>,
}

/// Methods of resource discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiscoveryMethod {
    /// Random exploration
    RandomExploration,
    /// Systematic survey
    SystematicSurvey,
    /// Following geological clues
    GeologicalAnalysis,
    /// Advanced remote sensing
    RemoteSensing,
    /// Following trade routes/rumors
    TradeIntelligence,
    /// Accidental discovery during construction
    AccidentalDiscovery,
}

/// Per-civilization discovery progress and capabilities
#[derive(Debug, Clone)]
pub struct CivilizationDiscovery {
    /// Civilization ID
    pub civ_id: u32,
    /// Available discovery technologies
    pub technologies: Vec<String>,
    /// Discovery efficiency modifiers
    pub efficiency_modifiers: HashMap<String, f32>,
    /// Active exploration efforts
    pub active_explorations: Vec<ExplorationEffort>,
    /// Discovered resources
    pub discovered_resources: HashMap<Entity, DiscoveryRecord>,
}

/// Active exploration effort
#[derive(Debug, Clone)]
pub struct ExplorationEffort {
    /// Target area for exploration
    pub target_area: ExplorationArea,
    /// Exploration method being used
    pub method: DiscoveryMethod,
    /// Investment/effort level (0.0 to 1.0)
    pub effort_level: f32,
    /// Duration of exploration
    pub duration_turns: u32,
    /// Progress made so far
    pub progress: f32,
}

/// Area being explored
#[derive(Debug, Clone)]
pub struct ExplorationArea {
    /// Center of exploration
    pub center: HexCoord,
    /// Exploration radius
    pub radius: u32,
    /// Priority tiles within area
    pub priority_tiles: Vec<TileId>,
}

/// Record of a discovered resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryRecord {
    /// Resource entity
    pub resource_entity: Entity,
    /// When it was discovered
    pub discovery_turn: u32,
    /// Method of discovery
    pub method: DiscoveryMethod,
    /// Quality of information (estimated vs. precise)
    pub information_quality: f32,
    /// Whether location is precisely known
    pub precise_location: bool,
}

/// Queued discovery task
#[derive(Debug, Clone)]
pub struct DiscoveryTask {
    /// Civilization performing discovery
    pub civ_id: u32,
    /// Target resource position
    pub target_position: TileId,
    /// Discovery method to use
    pub method: DiscoveryMethod,
    /// Task priority
    pub priority: f32,
}

impl ResourceDiscoverySystem {
    /// Create new discovery system
    pub fn new(seed: u64) -> ResourceResult<Self> {
        info!("🔍 Initializing Resource Discovery System...");
        
        let lua_handler = ComprehensiveLuaHandler::new(true)?;
        
        // Load discovery behavior scripts
        lua_handler.load_script("lua-scripts/resources/discovery.lua")?;
        
        Ok(Self {
            lua_handler,
            discovery_states: HashMap::new(),
            civilization_discovery: HashMap::new(),
            discovery_queue: VecDeque::new(),
            rng: ChaCha8Rng::seed_from_u64(seed),
        })
    }
    
    /// Initialize discovery state for a resource deposit
    pub fn initialize_discovery_state(&mut self, entity: Entity, deposit: &ResourceDeposit) -> ResourceResult<()> {
        let discovery_state = DiscoveryState {
            base_difficulty: deposit.discovery_difficulty,
            progress: 0.0,
            helpful_technologies: self.get_helpful_technologies(&deposit.resource_type)?,
            discovered_by: Vec::new(),
            discovery_method: None,
            discovery_turn: None,
        };
        
        self.discovery_states.insert(entity, discovery_state);
        Ok(())
    }
    
    /// Add or update civilization discovery capabilities
    pub fn update_civilization_discovery(&mut self, civ_id: u32, technologies: Vec<String>) -> ResourceResult<()> {
        let civ_discovery = self.civilization_discovery.entry(civ_id).or_insert_with(|| {
            CivilizationDiscovery {
                civ_id,
                technologies: Vec::new(),
                efficiency_modifiers: HashMap::new(),
                active_explorations: Vec::new(),
                discovered_resources: HashMap::new(),
            }
        });
        
        civ_discovery.technologies = technologies.clone();
        
        // Release the mutable borrow, then calculate efficiency modifiers
        let efficiency_modifiers = self.calculate_efficiency_modifiers(&technologies)?;
        self.civilization_discovery.get_mut(&civ_id).unwrap().efficiency_modifiers = efficiency_modifiers;
        
        debug!("🧪 Updated discovery capabilities for civilization {}", civ_id);
        Ok(())
    }
    
    /// Start exploration effort in an area
    pub fn start_exploration(
        &mut self,
        civ_id: u32,
        area: ExplorationArea,
        method: DiscoveryMethod,
        effort_level: f32,
    ) -> ResourceResult<()> {
        let civ_discovery = self.civilization_discovery.entry(civ_id).or_insert_with(|| {
            CivilizationDiscovery {
                civ_id,
                technologies: Vec::new(),
                efficiency_modifiers: HashMap::new(),
                active_explorations: Vec::new(),
                discovered_resources: HashMap::new(),
            }
        });
        
        let exploration = ExplorationEffort {
            target_area: area,
            method: method.clone(),
            effort_level: effort_level.clamp(0.0, 1.0),
            duration_turns: 0,
            progress: 0.0,
        };
        
        civ_discovery.active_explorations.push(exploration);
        info!("🗺️ Started exploration for civilization {} with {:?}", civ_id, method);
        Ok(())
    }
    
    /// Process discovery attempts for all active explorations
    pub fn process_discovery_turn(&mut self, world: &mut World, current_turn: u32) -> ResourceResult<Vec<DiscoveryEvent>> {
        let mut discovery_events = Vec::new();
        
        // Collect civ IDs to avoid borrowing conflicts
        let civ_ids: Vec<u32> = self.civilization_discovery.keys().cloned().collect();
        
        // Process each civilization's explorations
        for civ_id in civ_ids {
            let (efficiency_modifiers, mut explorations) = {
                let civ_discovery = self.civilization_discovery.get(&civ_id).unwrap();
                (civ_discovery.efficiency_modifiers.clone(), civ_discovery.active_explorations.clone())
            };
            
            for exploration in &mut explorations {
                exploration.duration_turns += 1;
                
                // Process exploration progress
                let progress_this_turn = self.calculate_exploration_progress(exploration, &efficiency_modifiers)?;
                exploration.progress += progress_this_turn;
                
                // Check for discoveries in the exploration area
                let area_discoveries = self.check_area_discoveries(
                    world,
                    civ_id,
                    &exploration.target_area,
                    &exploration.method,
                    exploration.progress,
                    current_turn,
                )?;
                
                discovery_events.extend(area_discoveries);
            }
            
            // Update the civilization's explorations
            if let Some(civ_discovery) = self.civilization_discovery.get_mut(&civ_id) {
                civ_discovery.active_explorations = explorations;
            }
        }
        
        // Process discovery queue
        self.process_discovery_queue(world, current_turn)?;
        
        Ok(discovery_events)
    }
    
    /// Check for resource discoveries in an exploration area
    fn check_area_discoveries(
        &mut self,
        world: &mut World,
        civ_id: u32,
        area: &ExplorationArea,
        method: &DiscoveryMethod,
        exploration_progress: f32,
        current_turn: u32,
    ) -> ResourceResult<Vec<DiscoveryEvent>> {
        let mut discoveries = Vec::new();
        
        // Find resource deposits in the exploration area
        let mut resource_query = world.query::<(Entity, &TileId, &ResourceDeposit)>();
        
        for (entity, tile_pos, deposit) in resource_query.iter(world) {
            // Check if position is within exploration area
            // Convert TileId to HexCoord for distance calculation
            let id = tile_pos.0;
            let hex_coord = HexCoord::new((id % 1000) as i32, (id / 1000) as i32);
            let hex_center = crate::core::zig_ffi::HexCoord::new(area.center.q as i32, area.center.r as i32);
            let distance = self.hex_distance(&hex_center, &hex_coord);
            if distance <= area.radius as f32 {
                // Check if already discovered by this civilization
                if let Some(discovery_state) = self.discovery_states.get(&entity) {
                    if !discovery_state.discovered_by.contains(&civ_id) {
                        // Calculate discovery probability
                        let discovery_probability = self.calculate_discovery_probability(
                            deposit,
                            discovery_state,
                            &method,
                            exploration_progress,
                            civ_id,
                        )?;
                        
                        if self.rng.gen::<f32>() < discovery_probability {
                            // Resource discovered!
                            let discovery_event = self.process_resource_discovery(
                                entity,
                                deposit,
                                civ_id,
                                method.clone(),
                                current_turn,
                            )?;
                            
                            discoveries.push(discovery_event);
                        }
                    }
                }
            }
        }
        
        Ok(discoveries)
    }
    
    /// Calculate discovery probability for a resource
    fn calculate_discovery_probability(
        &self,
        deposit: &ResourceDeposit,
        discovery_state: &DiscoveryState,
        method: &DiscoveryMethod,
        exploration_progress: f32,
        civ_id: u32,
    ) -> ResourceResult<f32> {
        // Use Lua to calculate complex discovery probability
        let probability: f32 = self.lua_handler.call_function(
            "calculate_discovery_probability",
            (
                deposit.resource_type.clone(),
                discovery_state.base_difficulty,
                serde_json::to_string(method)?,
                exploration_progress,
                civ_id,
            ),
        )?;
        
        Ok(probability.clamp(0.0, 1.0))
    }
    
    /// Process a successful resource discovery
    fn process_resource_discovery(
        &mut self,
        resource_entity: Entity,
        deposit: &ResourceDeposit,
        civ_id: u32,
        method: DiscoveryMethod,
        current_turn: u32,
    ) -> ResourceResult<DiscoveryEvent> {
        // Update discovery state
        if let Some(discovery_state) = self.discovery_states.get_mut(&resource_entity) {
            discovery_state.discovered_by.push(civ_id);
            discovery_state.discovery_method = Some(method.clone());
            discovery_state.discovery_turn = Some(current_turn);
        }
        
        // Calculate information quality before mutable borrow
        let information_quality = self.calculate_information_quality(&method, &deposit)?;
        
        // Add to civilization's discovered resources
        if let Some(civ_discovery) = self.civilization_discovery.get_mut(&civ_id) {
            let discovery_record = DiscoveryRecord {
                resource_entity,
                discovery_turn: current_turn,
                method: method.clone(),
                information_quality,
                precise_location: matches!(method, DiscoveryMethod::SystematicSurvey | DiscoveryMethod::RemoteSensing),
            };
            
            civ_discovery.discovered_resources.insert(resource_entity, discovery_record);
        }
        
        // Create discovery event
        let event = DiscoveryEvent {
            civ_id,
            resource_entity,
            resource_type: deposit.resource_type.clone(),
            discovery_method: method.clone(),
            discovery_turn: current_turn,
            estimated_quantity: deposit.quantity,
            estimated_quality: deposit.quality,
        };
        
        info!("💎 Civilization {} discovered {} via {:?}", civ_id, deposit.resource_type, event.discovery_method);
        
        // Trigger Lua event callback
          let event_data = LuaEventData::from_map(HashMap::from([
            ("civ_id".to_string(), LuaEventValue::String(civ_id.to_string())),
            ("resource_type".to_string(), LuaEventValue::String(deposit.resource_type.clone())),
            ("method".to_string(), LuaEventValue::String(format!("{:?}", event.discovery_method))),
        ]));
        
        let _: Vec<String> = self.lua_handler.trigger_event("resource_discovered", &event_data)?;
        
        Ok(event)
    }
    
    /// Get technologies that help discover a specific resource type
    fn get_helpful_technologies(&self, resource_type: &str) -> ResourceResult<Vec<String>> {
        let technologies: Vec<String> = self.lua_handler.call_function(
            "get_helpful_technologies",
            (resource_type,),
        )?;
        
        Ok(technologies)
    }
    
    /// Calculate efficiency modifiers based on available technologies
    fn calculate_efficiency_modifiers(&self, technologies: &[String]) -> ResourceResult<HashMap<String, f32>> {
        let modifiers_json: String = self.lua_handler.call_function(
            "calculate_efficiency_modifiers",
            (technologies,),
        )?;
        
        let modifiers: HashMap<String, f32> = serde_json::from_str(&modifiers_json)
            .map_err(|e| ResourceDistributionError::ConfigError(e.to_string()))?;
        
        Ok(modifiers)
    }
    
    /// Calculate exploration progress for a turn
    fn calculate_exploration_progress(&self, exploration: &ExplorationEffort, modifiers: &HashMap<String, f32>) -> ResourceResult<f32> {
        let base_progress = 0.1 * exploration.effort_level; // 10% base progress per turn at full effort
        let method_modifier = modifiers.get(&format!("{:?}", exploration.method)).copied().unwrap_or(1.0);
        
        Ok(base_progress * method_modifier)
    }
    
    /// Calculate information quality based on discovery method
    fn calculate_information_quality(&self, method: &DiscoveryMethod, deposit: &ResourceDeposit) -> ResourceResult<f32> {
        let quality: f32 = self.lua_handler.call_function(
            "calculate_information_quality",
            (serde_json::to_string(method)?, deposit.resource_type.clone()),
        )?;
        
        Ok(quality.clamp(0.0, 1.0))
    }
    
    /// Process queued discovery tasks
    fn process_discovery_queue(&mut self, world: &World, current_turn: u32) -> ResourceResult<()> {
        while let Some(task) = self.discovery_queue.pop_front() {
            // Process discovery task
            debug!("🔬 Processing discovery task for civilization {}", task.civ_id);
            
            // Implementation would process the specific discovery task
            // This is a simplified version
        }
        
        Ok(())
    }
    
    /// Calculate hex distance between two positions
    fn hex_distance(&self, pos1: &HexCoord, pos2: &HexCoord) -> f32 {
        let dq = pos1.q - pos2.q;
        let dr = pos1.r - pos2.r;
        ((dq.abs() + (dq + dr).abs() + dr.abs()) / 2) as f32
    }
}

/// Discovery event that occurred
#[derive(Debug, Clone)]
pub struct DiscoveryEvent {
    /// Civilization that made the discovery
    pub civ_id: u32,
    /// Resource entity discovered
    pub resource_entity: Entity,
    /// Type of resource discovered
    pub resource_type: String,
    /// Method used for discovery
    pub discovery_method: DiscoveryMethod,
    /// Turn when discovered
    pub discovery_turn: u32,
    /// Estimated quantity (may be inaccurate)
    pub estimated_quantity: u8,
    /// Estimated quality (may be inaccurate)
    pub estimated_quality: f32,
}
