//! Resource depletion and scarcity management system
//!
//! Implements sophisticated resource depletion mechanics with market dynamics,
//! scarcity calculations, and economic balance using Lua configuration.

use std::collections::HashMap;
use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use tracing::{debug, info, warn};

use crate::scripting::{ComprehensiveLuaHandler, LuaEventData};
use crate::world::tiles::TileId;

use super::types::*;
use super::{ResourceResult, ResourceDistributionError};

/// Resource depletion and scarcity management system
pub struct ResourceDepletionSystem {
    /// Lua handler for depletion rules
    lua_handler: ComprehensiveLuaHandler,
    /// Global resource statistics
    global_stats: GlobalResourceStats,
    /// Scarcity calculations by resource type
    scarcity_index: HashMap<String, ScarcityData>,
    /// Market dynamics tracker
    market_dynamics: MarketDynamicsTracker,
    /// Depletion events history
    depletion_events: Vec<DepletionEvent>,
    /// RNG for random depletion events
    rng: ChaCha8Rng,
}

/// Global resource statistics and tracking
#[derive(Debug, Clone, Default)]
pub struct GlobalResourceStats {
    /// Total known reserves by resource type
    pub total_reserves: HashMap<String, u64>,
    /// Total extracted to date by resource type
    pub total_extracted: HashMap<String, u64>,
    /// Current extraction rates by resource type
    pub extraction_rates: HashMap<String, f32>,
    /// Number of active deposits by resource type
    pub active_deposits: HashMap<String, u32>,
    /// Average quality by resource type
    pub average_quality: HashMap<String, f32>,
}

/// Scarcity tracking and calculations
#[derive(Debug, Clone)]
pub struct ScarcityData {
    /// Resource type
    pub resource_type: String,
    /// Current scarcity index (0.0 = abundant, 1.0 = extremely scarce)
    pub scarcity_index: f32,
    /// Historical scarcity trend
    pub scarcity_trend: Vec<f32>,
    /// Estimated years until depletion at current rate
    pub years_until_depletion: f32,
    /// Regional scarcity variations
    pub regional_scarcity: HashMap<String, f32>,
    /// Alternative resource substitutes
    pub substitutes: Vec<SubstituteResource>,
}

/// Substitute resource information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubstituteResource {
    /// Substitute resource type
    pub resource_type: String,
    /// Effectiveness as substitute (0.0 to 1.0)
    pub effectiveness: f32,
    /// Technology required for substitution
    pub required_tech: Option<String>,
    /// Cost multiplier compared to original
    pub cost_multiplier: f32,
}

/// Market dynamics tracking
#[derive(Debug, Clone, Default)]
pub struct MarketDynamicsTracker {
    /// Price history by resource type
    pub price_history: HashMap<String, Vec<f32>>,
    /// Current market prices
    pub current_prices: HashMap<String, f32>,
    /// Supply/demand ratios
    pub supply_demand: HashMap<String, f32>,
    /// Price volatility indices
    pub volatility: HashMap<String, f32>,
    /// Strategic reserves by civilization
    pub strategic_reserves: HashMap<u32, HashMap<String, u64>>,
}

/// Depletion event record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepletionEvent {
    /// Resource entity affected
    pub resource_entity: Entity,
    /// Resource type
    pub resource_type: String,
    /// Position of depletion
    pub position: TileId,
    /// Event type
    pub event_type: DepletionEventType,
    /// Turn when event occurred
    pub turn: u32,
    /// Severity (0.0 to 1.0)
    pub severity: f32,
    /// Affected civilizations
    pub affected_civilizations: Vec<u32>,
}

/// Types of depletion events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DepletionEventType {
    /// Deposit fully exhausted
    Exhaustion,
    /// Quality degradation due to over-extraction
    QualityDegradation,
    /// Extraction efficiency loss
    EfficiencyLoss,
    /// Environmental damage affecting extraction
    EnvironmentalDamage,
    /// Market crash due to oversupply
    MarketCrash,
    /// Resource boom discovery
    ResourceBoom,
    /// New extraction technology
    TechnologyAdvancement,
}

impl ResourceDepletionSystem {
    /// Create new depletion system
    pub fn new(seed: u64) -> ResourceResult<Self> {
        info!("⚖️ Initializing Resource Depletion System...");
        
        let lua_handler = ComprehensiveLuaHandler::new(true)?;
        
        // Load depletion behavior scripts
        lua_handler.load_script("lua-scripts/resources/depletion.lua")?;
        lua_handler.load_script("lua-scripts/resources/scarcity.lua")?;
        lua_handler.load_script("lua-scripts/resources/market.lua")?;
        
        Ok(Self {
            lua_handler,
            global_stats: GlobalResourceStats::default(),
            scarcity_index: HashMap::new(),
            market_dynamics: MarketDynamicsTracker::default(),
            depletion_events: Vec::new(),
            rng: ChaCha8Rng::seed_from_u64(seed),
        })
    }
    
    /// Update global resource statistics
    pub fn update_global_stats(&mut self, world: &mut World) -> ResourceResult<()> {
        debug!("📊 Updating global resource statistics...");
        
        // Reset counters
        self.global_stats.total_reserves.clear();
        self.global_stats.active_deposits.clear();
        let mut quality_sums: HashMap<String, f32> = HashMap::new();
        let mut quality_counts: HashMap<String, u32> = HashMap::new();
        
        // Query all resource deposits
        let mut resource_query = world.query::<&ResourceDeposit>();
        
        for deposit in resource_query.iter(world) {
            let resource_type = &deposit.resource_type;
            
            // Update reserves
            *self.global_stats.total_reserves.entry(resource_type.clone()).or_insert(0) += deposit.quantity as u64;
            
            // Update active deposits
            if deposit.quantity > 0 {
                *self.global_stats.active_deposits.entry(resource_type.clone()).or_insert(0) += 1;
            }
            
            // Update quality tracking
            *quality_sums.entry(resource_type.clone()).or_insert(0.0) += deposit.quality;
            *quality_counts.entry(resource_type.clone()).or_insert(0) += 1;
        }
        
        // Calculate average quality
        for (resource_type, sum) in quality_sums {
            if let Some(count) = quality_counts.get(&resource_type) {
                if *count > 0 {
                    self.global_stats.average_quality.insert(resource_type, sum / *count as f32);
                }
            }
        }
        
        info!("📈 Updated statistics for {} resource types", self.global_stats.total_reserves.len());
        Ok(())
    }
    
    /// Process resource extraction for a turn
    pub fn process_extraction_turn(
        &mut self,
        world: &mut World,
        extraction_data: &HashMap<Entity, ExtractionOperation>,
        current_turn: u32,
    ) -> ResourceResult<Vec<DepletionEvent>> {
        debug!("⛏️ Processing resource extraction for turn {}", current_turn);
        
        let mut turn_events = Vec::new();
        
        for (entity, extraction) in extraction_data {
            // First, get immutable reference to read deposit data
            let deposit_data = if let Some(deposit) = world.get::<ResourceDeposit>(*entity) {
                deposit.clone()
            } else {
                continue;
            };
            
            // Calculate actual extraction amount
            let extraction_amount = self.calculate_extraction_amount(&deposit_data, extraction)?;
            
            if extraction_amount > 0.0 {
                // Now get mutable reference to apply changes
                if let Some(mut deposit_mut) = world.get_mut::<ResourceDeposit>(*entity) {
                    // Apply extraction to deposit
                    let extracted_quantity = (extraction_amount.min(deposit_mut.quantity as f32)) as u8;
                    deposit_mut.quantity = deposit_mut.quantity.saturating_sub(extracted_quantity);
                    
                    // Update depletion state
                    deposit_mut.depletion_state.current_extraction = extraction_amount;
                    deposit_mut.depletion_state.efficiency_penalty = self.calculate_efficiency_penalty(&deposit_data)?;
                    deposit_mut.depletion_state.turns_remaining = self.calculate_turns_remaining(&deposit_mut)?;
                    
                    // Create updated deposit data for event checking
                    let updated_deposit = deposit_mut.clone();
                    
                    // Check for depletion events
                    if let Some(event) = self.check_depletion_events(
                        *entity,
                        &updated_deposit,
                        extracted_quantity,
                        current_turn,
                        world,
                    )? {
                        turn_events.push(event);
                    }
                    
                    // Update global extraction rates
                    let resource_type = deposit_data.resource_type.clone();
                    *self.global_stats.extraction_rates.entry(resource_type).or_insert(0.0) += extraction_amount;
                    *self.global_stats.total_extracted.entry(deposit_data.resource_type.clone()).or_insert(0) += extracted_quantity as u64;
                }
            }
        }
        
        // Update scarcity calculations
        self.update_scarcity_indices()?;
        
        // Update market dynamics
        self.update_market_dynamics(&turn_events, current_turn)?;
        
        Ok(turn_events)
    }
    
    /// Calculate actual extraction amount considering efficiency and technology
    fn calculate_extraction_amount(&self, deposit: &ResourceDeposit, extraction: &ExtractionOperation) -> ResourceResult<f32> {
        let base_extraction = extraction.planned_extraction;
        let efficiency = self.calculate_extraction_efficiency(deposit, extraction)?;
        
        Ok(base_extraction * efficiency)
    }
    
    /// Calculate extraction efficiency based on technology, infrastructure, and depletion
    fn calculate_extraction_efficiency(&self, deposit: &ResourceDeposit, extraction: &ExtractionOperation) -> ResourceResult<f32> {
        let efficiency: f32 = self.lua_handler.call_function(
            "calculate_extraction_efficiency",
            (
                deposit.resource_type.clone(),
                deposit.quantity,
                deposit.depletion_state.efficiency_penalty,
                extraction.technology_level,
                extraction.infrastructure_quality,
                extraction.environmental_conditions,
            ),
        )?;
        
        Ok(efficiency.clamp(0.01, 2.0)) // 1% minimum, 200% maximum efficiency
    }
    
    /// Calculate efficiency penalty due to depletion
    fn calculate_efficiency_penalty(&self, deposit: &ResourceDeposit) -> ResourceResult<f32> {
        if deposit.depletion_state.original_quantity == 0 {
            return Ok(0.0);
        }
        
        let depletion_ratio = 1.0 - (deposit.quantity as f32 / deposit.depletion_state.original_quantity as f32);
        let penalty: f32 = self.lua_handler.call_function(
            "calculate_efficiency_penalty",
            (deposit.resource_type.clone(), depletion_ratio),
        )?;
        
        Ok(penalty.clamp(0.0, 0.9)) // Maximum 90% efficiency penalty
    }
    
    /// Calculate estimated turns until resource exhaustion
    fn calculate_turns_remaining(&self, deposit: &ResourceDeposit) -> ResourceResult<u32> {
        if deposit.depletion_state.current_extraction <= 0.0 {
            return Ok(u32::MAX); // Never depletes if not being extracted
        }
        
        let remaining_turns = deposit.quantity as f32 / deposit.depletion_state.current_extraction;
        Ok(remaining_turns.ceil() as u32)
    }
    
    /// Check for depletion events that should trigger
    fn check_depletion_events(
        &mut self,
        entity: Entity,
        deposit: &ResourceDeposit,
        extracted_this_turn: u8,
        current_turn: u32,
        world: &World,
    ) -> ResourceResult<Option<DepletionEvent>> {
        // Check for exhaustion
        if deposit.quantity == 0 && extracted_this_turn > 0 {
            let position = world.get::<TileId>(entity)
                .copied()
                .unwrap_or(TileId(0));
            
            return Ok(Some(DepletionEvent {
                resource_entity: entity,
                resource_type: deposit.resource_type.clone(),
                position,
                event_type: DepletionEventType::Exhaustion,
                turn: current_turn,
                severity: 1.0,
                affected_civilizations: Vec::new(), // Would be populated based on who was extracting
            }));
        }
        
        // Check for quality degradation
        let original_quality = 0.7; // Would come from original deposit data
        if deposit.quality < original_quality * 0.5 {
            // Quality has degraded significantly
            let should_trigger: bool = self.lua_handler.call_function(
                "should_trigger_quality_degradation_event",
                (deposit.resource_type.clone(), deposit.quality, original_quality),
            )?;
            
            if should_trigger && self.rng.gen::<f32>() < 0.1 { // 10% chance per turn when conditions are met
                let position = world.get::<TileId>(entity)
                    .copied()
                    .unwrap_or(TileId(0));
                
                return Ok(Some(DepletionEvent {
                    resource_entity: entity,
                    resource_type: deposit.resource_type.clone(),
                    position,
                    event_type: DepletionEventType::QualityDegradation,
                    turn: current_turn,
                    severity: 1.0 - (deposit.quality / original_quality),
                    affected_civilizations: Vec::new(),
                }));
            }
        }
        
        Ok(None)
    }
    
    /// Update scarcity indices for all resource types
    fn update_scarcity_indices(&mut self) -> ResourceResult<()> {
        for (resource_type, reserves) in &self.global_stats.total_reserves {
            let extraction_rate = self.global_stats.extraction_rates.get(resource_type).copied().unwrap_or(0.0);
            
            let scarcity: f32 = self.lua_handler.call_function(
                "calculate_scarcity_index",
                (
                    resource_type.clone(),
                    *reserves,
                    extraction_rate,
                    self.global_stats.active_deposits.get(resource_type).copied().unwrap_or(0),
                ),
            )?;
            
            let scarcity_data = self.scarcity_index.entry(resource_type.clone()).or_insert_with(|| {
                ScarcityData {
                    resource_type: resource_type.clone(),
                    scarcity_index: 0.0,
                    scarcity_trend: Vec::new(),
                    years_until_depletion: f32::INFINITY,
                    regional_scarcity: HashMap::new(),
                    substitutes: Vec::new(),
                }
            });
            
            scarcity_data.scarcity_trend.push(scarcity);
            scarcity_data.scarcity_index = scarcity;
            
            // Calculate years until depletion
            if extraction_rate > 0.0 {
                scarcity_data.years_until_depletion = *reserves as f32 / (extraction_rate * 365.0); // Assuming daily extraction rates
            }
            
            // Keep only last 100 data points for trend analysis
            if scarcity_data.scarcity_trend.len() > 100 {
                scarcity_data.scarcity_trend.remove(0);
            }
        }
        
        Ok(())
    }
    
    /// Update market dynamics based on supply, demand, and events
    fn update_market_dynamics(&mut self, events: &[DepletionEvent], current_turn: u32) -> ResourceResult<()> {
        // Update prices based on scarcity
        for (resource_type, scarcity_data) in &self.scarcity_index {
            let current_price = self.market_dynamics.current_prices.get(resource_type).copied().unwrap_or(1.0);
            
            let new_price: f32 = self.lua_handler.call_function(
                "calculate_market_price",
                (
                    resource_type.clone(),
                    current_price,
                    scarcity_data.scarcity_index,
                    self.global_stats.extraction_rates.get(resource_type).copied().unwrap_or(0.0),
                ),
            )?;
            
            self.market_dynamics.current_prices.insert(resource_type.clone(), new_price);
            
            // Update price history
            let history = self.market_dynamics.price_history.entry(resource_type.clone()).or_default();
            history.push(new_price);
            
            // Keep only last 200 price points
            if history.len() > 200 {
                history.remove(0);
            }
            
            // Calculate volatility
            if history.len() > 10 {
                let recent_history: Vec<f32> = history[history.len()-10..].to_vec();
                drop(history); // Explicitly drop the mutable borrow
                let volatility = self.calculate_price_volatility(&recent_history);
                self.market_dynamics.volatility.insert(resource_type.clone(), volatility);
            }
        }
        
        // Process event impacts on market
        for event in events {
            self.process_event_market_impact(event, current_turn)?;
        }
        
        Ok(())
    }
    
    /// Calculate price volatility from recent price history
    fn calculate_price_volatility(&self, recent_prices: &[f32]) -> f32 {
        if recent_prices.len() < 2 {
            return 0.0;
        }
        
        let mean = recent_prices.iter().sum::<f32>() / recent_prices.len() as f32;
        let variance = recent_prices.iter()
            .map(|p| (p - mean).powi(2))
            .sum::<f32>() / recent_prices.len() as f32;
        
        variance.sqrt() / mean // Coefficient of variation
    }
    
    /// Process market impact of depletion events
    fn process_event_market_impact(&mut self, event: &DepletionEvent, _current_turn: u32) -> ResourceResult<()> {
        let current_price = self.market_dynamics.current_prices.get(&event.resource_type).copied().unwrap_or(1.0);
        
        let price_impact = match event.event_type {
            DepletionEventType::Exhaustion => current_price * (1.0 + event.severity * 0.5), // Up to 50% price increase
            DepletionEventType::QualityDegradation => current_price * (1.0 + event.severity * 0.2), // Up to 20% increase
            DepletionEventType::ResourceBoom => current_price * (1.0 - event.severity * 0.3), // Up to 30% decrease
            _ => current_price, // Other events have minimal immediate impact
        };
        
        self.market_dynamics.current_prices.insert(event.resource_type.clone(), price_impact);
        
        debug!("💰 Market impact for {}: {:.2} -> {:.2}", event.resource_type, current_price, price_impact);
        Ok(())
    }
    
    /// Get current scarcity data for a resource type
    pub fn get_scarcity_data(&self, resource_type: &str) -> Option<&ScarcityData> {
        self.scarcity_index.get(resource_type)
    }
    
    /// Get current market price for a resource
    pub fn get_market_price(&self, resource_type: &str) -> f32 {
        self.market_dynamics.current_prices.get(resource_type).copied().unwrap_or(1.0)
    }
    
    /// Get global statistics for all resources
    pub fn get_global_stats(&self) -> &GlobalResourceStats {
        &self.global_stats
    }
}

/// Resource extraction operation data
#[derive(Debug, Clone)]
pub struct ExtractionOperation {
    /// Planned extraction amount for this turn
    pub planned_extraction: f32,
    /// Technology level of extraction (affects efficiency)
    pub technology_level: f32,
    /// Infrastructure quality (roads, processing facilities, etc.)
    pub infrastructure_quality: f32,
    /// Environmental conditions (weather, terrain difficulty, etc.)
    pub environmental_conditions: f32,
    /// Civilization performing extraction
    pub extracting_civilization: u32,
}
