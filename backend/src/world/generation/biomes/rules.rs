//! Biome Rules and Decision Trees
//!
//! Lua-based biome determination rules and decision tree processing.

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, collections::HashMap};
use tracing::{debug, instrument};

use crate::scripting::{ScriptManager, ScriptResult, LuaEventData, LuaEventValue};

/// Lua-based biome rules processor
#[derive(Component, Debug, Resource)]
pub struct LuaBiomeRules {
    script_manager: Arc<ScriptManager>,
    decision_trees: Vec<BiomeDecisionTree>,
}

impl LuaBiomeRules {
    /// Create new Lua biome rules processor
    pub fn new() -> ScriptResult<Self> {
        let script_manager = Arc::new(ScriptManager::new()?);
        let decision_trees = Self::create_default_decision_trees();
        
        let mut rules = Self { script_manager, decision_trees };
        rules.load_biome_scripts()?;
        
        Ok(rules)
    }
    
    /// Load biome determination scripts
    #[instrument(skip(self))]
    fn load_biome_scripts(&self) -> ScriptResult<()> {
        let scripts = [
            "biomes/biome_determination.lua",
            "biomes/special_biomes.lua",
            "biomes/validation.lua",
        ];
        
        for script in &scripts {
            if let Err(e) = self.script_manager.load_script(script) {
                debug!("Optional biome script not found: {} ({})", script, e);
            }
        }
        
        Ok(())
    }
    
    /// Create default decision trees for biome determination
    fn create_default_decision_trees() -> Vec<BiomeDecisionTree> {
        vec![
            // Temperature-based primary classification
            BiomeDecisionTree {
                name: "temperature_classification".to_string(),
                root: DecisionNode::Condition {
                    parameter: "temperature".to_string(),
                    operator: ConditionOperator::LessThan,
                    value: 0.0,
                    true_branch: Box::new(DecisionNode::Condition {
                        parameter: "temperature".to_string(),
                        operator: ConditionOperator::LessThan,
                        value: -10.0,
                        true_branch: Box::new(DecisionNode::Result("polar".to_string())),
                        false_branch: Box::new(DecisionNode::Result("tundra".to_string())),
                    }),
                    false_branch: Box::new(DecisionNode::Condition {
                        parameter: "temperature".to_string(),
                        operator: ConditionOperator::GreaterThan,
                        value: 25.0,
                        true_branch: Box::new(DecisionNode::Result("tropical".to_string())),
                        false_branch: Box::new(DecisionNode::Result("temperate".to_string())),
                    }),
                },
            },
            
            // Precipitation-based secondary classification
            BiomeDecisionTree {
                name: "precipitation_classification".to_string(),
                root: DecisionNode::Condition {
                    parameter: "rainfall".to_string(),
                    operator: ConditionOperator::LessThan,
                    value: 100.0,
                    true_branch: Box::new(DecisionNode::Result("arid".to_string())),
                    false_branch: Box::new(DecisionNode::Condition {
                        parameter: "rainfall".to_string(),
                        operator: ConditionOperator::GreaterThan,
                        value: 300.0,
                        true_branch: Box::new(DecisionNode::Result("wet".to_string())),
                        false_branch: Box::new(DecisionNode::Result("moderate".to_string())),
                    }),
                },
            },
        ]
    }
    
    /// Process biome determination using decision trees and Lua rules
    #[instrument(skip(self))]
    pub fn determine_biome(
        &self,
        climate_data: &BiomeClimateData,
    ) -> ScriptResult<BiomeDecision> {
        let mut decision = BiomeDecision {
            primary_biome: "temperate_grassland".to_string(),
            confidence: 0.5,
            modifiers: Vec::new(),
            reasoning: Vec::new(),
        };
        
        // Apply decision trees
        for tree in &self.decision_trees {
            if let Some(result) = tree.evaluate(climate_data) {
                decision.reasoning.push(format!("Tree '{}': {}", tree.name, result));
                
                match tree.name.as_str() {
                    "temperature_classification" => {
                        decision.primary_biome = self.refine_biome_by_temperature(&result, climate_data);
                    }
                    "precipitation_classification" => {
                        decision.primary_biome = self.refine_biome_by_precipitation(&decision.primary_biome, &result, climate_data);
                    }
                    _ => {}
                }
            }
        }
        
        // Apply Lua rules if available
        if self.script_manager.call_function::<(), bool>("biome_rules_available", ()).unwrap_or(false) {
            decision = self.apply_lua_biome_rules(decision, climate_data)?;
        }
        
        Ok(decision)
    }
    
    /// Apply Lua biome determination rules
    #[instrument(skip(self, decision, climate_data))]
    fn apply_lua_biome_rules(
        &self,
        mut decision: BiomeDecision,
        climate_data: &BiomeClimateData,
    ) -> ScriptResult<BiomeDecision> {
        let mut event_data = LuaEventData {
            event_type: "biome_rule".to_string(),
            data: HashMap::new(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            source: Some("biome_rules".to_string()),
        };
        event_data.data.insert("temperature".to_string(), LuaEventValue::Number(climate_data.temperature as f64));
        event_data.data.insert("rainfall".to_string(), LuaEventValue::Number(climate_data.rainfall as f64));
        event_data.data.insert("humidity".to_string(), LuaEventValue::Number(climate_data.humidity as f64));
        event_data.data.insert("elevation".to_string(), LuaEventValue::Number(climate_data.elevation as f64));
        event_data.data.insert("terrain_type".to_string(), LuaEventValue::String(climate_data.terrain_type.clone()));
        event_data.data.insert("climate_zone".to_string(), LuaEventValue::String(climate_data.climate_zone.clone()));
        event_data.data.insert("current_biome".to_string(), LuaEventValue::String(decision.primary_biome.clone()));
        
        // Get Lua biome determination
        if let Ok(results) = self.script_manager.trigger_event("biome_determination", &event_data) {
            for result in results {
                if let Some((key, value)) = result.split_once(':') {
                    match key {
                        "biome_type" => {
                            decision.primary_biome = value.to_string();
                            decision.reasoning.push(format!("Lua rule: biome_type = {}", value));
                        }
                        "confidence" => {
                            if let Ok(conf) = value.parse::<f32>() {
                                decision.confidence = conf.clamp(0.0, 1.0);
                                decision.reasoning.push(format!("Lua rule: confidence = {}", conf));
                            }
                        }
                        "modifier" => {
                            decision.modifiers.push(value.to_string());
                            decision.reasoning.push(format!("Lua rule: modifier = {}", value));
                        }
                        _ => {}
                    }
                }
            }
        }
        
        // Check for special biomes
        if let Ok(results) = self.script_manager.trigger_event("special_biomes", &event_data) {
            for result in results {
                if result.starts_with("special:") {
                    decision.primary_biome = result[8..].to_string();
                    decision.confidence = 0.9;
                    decision.reasoning.push(format!("Special biome: {}", decision.primary_biome));
                }
            }
        }
        
        Ok(decision)
    }
    
    /// Refine biome based on temperature classification
    fn refine_biome_by_temperature(&self, temp_class: &str, climate: &BiomeClimateData) -> String {
        match temp_class {
            "polar" => "polar_desert".to_string(),
            "tundra" => if climate.rainfall > 100 { "tundra".to_string() } else { "polar_desert".to_string() },
            "temperate" => if climate.rainfall > 200 { "temperate_forest".to_string() } else { "temperate_grassland".to_string() },
            "tropical" => if climate.rainfall > 250 { "tropical_rainforest".to_string() } else { "tropical_grassland".to_string() },
            _ => "temperate_grassland".to_string(),
        }
    }
    
    /// Refine biome based on precipitation classification
    fn refine_biome_by_precipitation(&self, base_biome: &str, precip_class: &str, climate: &BiomeClimateData) -> String {
        match precip_class {
            "arid" => {
                if climate.temperature > 25 {
                    "desert".to_string()
                } else if climate.temperature < 5 {
                    "polar_desert".to_string()
                } else {
                    "steppe".to_string()
                }
            }
            "wet" => {
                if climate.temperature > 20 {
                    "tropical_rainforest".to_string()
                } else {
                    "temperate_rainforest".to_string()
                }
            }
            "moderate" => base_biome.to_string(),
            _ => base_biome.to_string(),
        }
    }
    
    /// Get decision tree by name
    pub fn get_decision_tree(&self, name: &str) -> Option<&BiomeDecisionTree> {
        self.decision_trees.iter().find(|tree| tree.name == name)
    }
    
    /// Add custom decision tree
    pub fn add_decision_tree(&mut self, tree: BiomeDecisionTree) {
        self.decision_trees.push(tree);
    }
}

/// Climate data for biome determination
#[derive(Debug, Clone)]
pub struct BiomeClimateData {
    pub temperature: i8,
    pub rainfall: u16,
    pub humidity: u8,
    pub elevation: f32,
    pub terrain_type: String,
    pub climate_zone: String,
}

/// Biome determination result
#[derive(Debug, Clone)]
pub struct BiomeDecision {
    pub primary_biome: String,
    pub confidence: f32,
    pub modifiers: Vec<String>,
    pub reasoning: Vec<String>,
}

/// Decision tree for biome classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeDecisionTree {
    pub name: String,
    pub root: DecisionNode,
}

impl BiomeDecisionTree {
    /// Evaluate decision tree against climate data
    pub fn evaluate(&self, climate_data: &BiomeClimateData) -> Option<String> {
        self.root.evaluate(climate_data)
    }
}

/// Decision tree node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DecisionNode {
    Condition {
        parameter: String,
        operator: ConditionOperator,
        value: f64,
        true_branch: Box<DecisionNode>,
        false_branch: Box<DecisionNode>,
    },
    Result(String),
}

impl DecisionNode {
    /// Evaluate node against climate data
    pub fn evaluate(&self, climate_data: &BiomeClimateData) -> Option<String> {
        match self {
            DecisionNode::Condition { parameter, operator, value, true_branch, false_branch } => {
                let param_value = self.get_parameter_value(parameter, climate_data)?;
                
                let condition_result = match operator {
                    ConditionOperator::LessThan => param_value < *value,
                    ConditionOperator::LessThanOrEqual => param_value <= *value,
                    ConditionOperator::GreaterThan => param_value > *value,
                    ConditionOperator::GreaterThanOrEqual => param_value >= *value,
                    ConditionOperator::Equal => (param_value - *value).abs() < 0.001,
                };
                
                let next_node = if condition_result { true_branch } else { false_branch };
                next_node.evaluate(climate_data)
            }
            DecisionNode::Result(result) => Some(result.clone()),
        }
    }
    
    /// Extract parameter value from climate data
    fn get_parameter_value(&self, parameter: &str, climate_data: &BiomeClimateData) -> Option<f64> {
        match parameter {
            "temperature" => Some(climate_data.temperature as f64),
            "rainfall" => Some(climate_data.rainfall as f64),
            "humidity" => Some(climate_data.humidity as f64),
            "elevation" => Some(climate_data.elevation as f64),
            _ => None,
        }
    }
}

/// Condition operators for decision trees
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConditionOperator {
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    Equal,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_lua_biome_rules_creation() {
        let rules = LuaBiomeRules::new();
        assert!(rules.is_ok());
    }
    
    #[test]
    fn test_decision_tree_evaluation() {
        let climate_data = BiomeClimateData {
            temperature: -15,
            rainfall: 50,
            humidity: 30,
            elevation: 100.0,
            terrain_type: "plains".to_string(),
            climate_zone: "polar".to_string(),
        };
        
        let rules = LuaBiomeRules::new().unwrap();
        let temp_tree = rules.get_decision_tree("temperature_classification").unwrap();
        let result = temp_tree.evaluate(&climate_data);
        
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "polar");
    }
    
    #[test]
    fn test_biome_determination() {
        let climate_data = BiomeClimateData {
            temperature: 22,
            rainfall: 350,
            humidity: 75,
            elevation: 200.0,
            terrain_type: "jungle".to_string(),
            climate_zone: "tropical".to_string(),
        };
        
        let rules = LuaBiomeRules::new().unwrap();
        let decision = rules.determine_biome(&climate_data);
        
        assert!(decision.is_ok());
        let decision = decision.unwrap();
        assert!(decision.primary_biome.contains("tropical"));
        assert!(decision.confidence > 0.0);
    }
}
