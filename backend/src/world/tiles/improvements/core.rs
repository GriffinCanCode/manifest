//! Core Improvement struct and functionality
//!
//! Contains the main Improvement struct and its methods.

use serde::{Deserialize, Serialize};
use crate::world::tiles::ownership::PlayerId;

use super::types::{ImprovementType, ImprovementState, ImprovementKey};

/// Individual improvement instance with state tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Improvement {
    /// Unique identifier
    pub key: ImprovementKey,
    /// Type of improvement
    pub improvement_type: ImprovementType,
    /// Current state
    pub state: ImprovementState,
    /// Player who built this improvement
    pub owner: PlayerId,
    /// Turn when construction started
    pub construction_started: u32,
    /// Turn when construction was completed (if applicable)
    pub completion_turn: Option<u32>,
    /// Additional properties specific to improvement type
    pub properties: ImprovementProperties,
}

/// Additional properties that can vary by improvement type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovementProperties {
    /// Efficiency multiplier (can be affected by technology, resources, etc.)
    pub efficiency: f32,
    /// Custom name given by player (optional)
    pub custom_name: Option<String>,
    /// Upgrade level (0 = base level)
    pub upgrade_level: u8,
    /// Whether this improvement is worked by a citizen
    pub is_worked: bool,
    /// Maintenance cost per turn
    pub maintenance_cost: u32,
}

impl Improvement {
    /// Create a new improvement
    pub fn new(
        key: ImprovementKey,
        improvement_type: ImprovementType,
        owner: PlayerId,
        current_turn: u32,
    ) -> Self {
        Self {
            key,
            improvement_type,
            state: ImprovementState::Planned,
            owner,
            construction_started: current_turn,
            completion_turn: None,
            properties: ImprovementProperties::default_for_type(improvement_type),
        }
    }

    /// Start construction of this improvement
    pub fn start_construction(&mut self, current_turn: u32) {
        let construction_time = self.improvement_type.construction_time();
        self.state = ImprovementState::UnderConstruction { 
            turns_remaining: construction_time 
        };
        self.construction_started = current_turn;
    }

    /// Update construction progress (call each turn)
    pub fn update_construction(&mut self, current_turn: u32) -> bool {
        match &mut self.state {
            ImprovementState::UnderConstruction { turns_remaining } => {
                if *turns_remaining > 0 {
                    *turns_remaining -= 1;
                    if *turns_remaining == 0 {
                        self.state = ImprovementState::Completed;
                        self.completion_turn = Some(current_turn);
                        return true; // Construction completed
                    }
                }
            }
            ImprovementState::UnderRepair { turns_remaining } => {
                if *turns_remaining > 0 {
                    *turns_remaining -= 1;
                    if *turns_remaining == 0 {
                        self.state = ImprovementState::Completed;
                        return true; // Repair completed
                    }
                }
            }
            _ => {}
        }
        false
    }

    /// Take damage to the improvement
    pub fn take_damage(&mut self, severity: u8) {
        match self.state {
            ImprovementState::Completed => {
                if severity >= 100 {
                    self.state = ImprovementState::Destroyed;
                } else {
                    self.state = ImprovementState::Damaged { severity };
                }
            }
            ImprovementState::Damaged { severity: current } => {
                let new_severity = (current + severity).min(100);
                if new_severity >= 100 {
                    self.state = ImprovementState::Destroyed;
                } else {
                    self.state = ImprovementState::Damaged { severity: new_severity };
                }
            }
            _ => {}
        }
    }

    /// Start repair of damaged improvement
    pub fn start_repair(&mut self, repair_time: u32) -> bool {
        match self.state {
            ImprovementState::Damaged { .. } => {
                self.state = ImprovementState::UnderRepair { 
                    turns_remaining: repair_time 
                };
                true
            }
            _ => false
        }
    }

    /// Check if improvement can be worked by a citizen
    pub fn can_be_worked(&self) -> bool {
        self.state.can_be_worked() && !self.properties.is_worked
    }

    /// Set whether improvement is being worked
    pub fn set_worked(&mut self, worked: bool) {
        if self.state.can_be_worked() {
            self.properties.is_worked = worked;
        }
    }

    /// Get current effectiveness (0.0 to 1.0)
    pub fn effectiveness(&self) -> f32 {
        self.state.effectiveness_factor() * self.properties.efficiency
    }

    /// Get resource yields from this improvement
    pub fn get_yields(&self) -> ResourceYields {
        if !self.state.is_functional() {
            return ResourceYields::zero();
        }

        let base_yields = self.improvement_type.base_yields();
        let effectiveness = self.effectiveness();
        let upgrade_multiplier = 1.0 + (self.properties.upgrade_level as f32 * 0.1);

        ResourceYields {
            food: (base_yields.food as f32 * effectiveness * upgrade_multiplier) as u32,
            production: (base_yields.production as f32 * effectiveness * upgrade_multiplier) as u32,
            commerce: (base_yields.commerce as f32 * effectiveness * upgrade_multiplier) as u32,
            culture: (base_yields.culture as f32 * effectiveness * upgrade_multiplier) as u32,
            science: (base_yields.science as f32 * effectiveness * upgrade_multiplier) as u32,
        }
    }

    /// Upgrade the improvement to the next level
    pub fn upgrade(&mut self) -> bool {
        if self.state == ImprovementState::Completed && self.properties.upgrade_level < 3 {
            self.properties.upgrade_level += 1;
            self.properties.maintenance_cost = (self.properties.maintenance_cost as f32 * 1.2) as u32;
            true
        } else {
            false
        }
    }

    /// Get total construction cost including upgrades
    pub fn total_cost(&self) -> u32 {
        let base_cost = self.improvement_type.construction_cost();
        let upgrade_cost = (0..self.properties.upgrade_level)
            .map(|level| base_cost * (level as u32 + 1))
            .sum::<u32>();
        base_cost + upgrade_cost
    }

    /// Check if improvement is owned by player
    pub fn is_owned_by(&self, player_id: PlayerId) -> bool {
        self.owner == player_id
    }

    /// Get age in turns (if completed)
    pub fn age_in_turns(&self, current_turn: u32) -> Option<u32> {
        self.completion_turn.map(|completion| current_turn - completion)
    }
}

impl ImprovementProperties {
    /// Create default properties for an improvement type
    pub fn default_for_type(improvement_type: ImprovementType) -> Self {
        let maintenance_cost = match improvement_type {
            ImprovementType::Road => 1,
            ImprovementType::Railroad => 3,
            ImprovementType::Bridge => 2,
            ImprovementType::Tunnel => 4,
            ImprovementType::Fort => 2,
            ImprovementType::Factory => 3,
            ImprovementType::University => 4,
            ImprovementType::Monument => 2,
            ImprovementType::Fortress => 5,
            ImprovementType::Observatory => 3,
            ImprovementType::Lighthouse => 1,
            ImprovementType::Aqueduct => 2,
            _ => 1,
        };

        Self {
            efficiency: 1.0,
            custom_name: None,
            upgrade_level: 0,
            is_worked: false,
            maintenance_cost,
        }
    }
}

/// Resource yields from improvements
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceYields {
    pub food: u32,
    pub production: u32,
    pub commerce: u32,
    pub culture: u32,
    pub science: u32,
}

impl ResourceYields {
    /// Zero yields
    pub fn zero() -> Self {
        Self { food: 0, production: 0, commerce: 0, culture: 0, science: 0 }
    }

    /// Add two yields together
    pub fn add(self, other: Self) -> Self {
        Self {
            food: self.food + other.food,
            production: self.production + other.production,
            commerce: self.commerce + other.commerce,
            culture: self.culture + other.culture,
            science: self.science + other.science,
        }
    }
}

impl ImprovementType {
    /// Get base resource yields for this improvement type
    pub fn base_yields(self) -> ResourceYields {
        match self {
            Self::Farm => ResourceYields { food: 3, production: 0, commerce: 0, culture: 0, science: 0 },
            Self::Mine => ResourceYields { food: 0, production: 4, commerce: 0, culture: 0, science: 0 },
            Self::Lumbermill => ResourceYields { food: 0, production: 2, commerce: 1, culture: 0, science: 0 },
            Self::Quarry => ResourceYields { food: 0, production: 3, commerce: 0, culture: 0, science: 0 },
            Self::Pasture => ResourceYields { food: 2, production: 0, commerce: 1, culture: 0, science: 0 },
            Self::Road => ResourceYields { food: 0, production: 0, commerce: 1, culture: 0, science: 0 },
            Self::Railroad => ResourceYields { food: 0, production: 1, commerce: 2, culture: 0, science: 0 },
            Self::Bridge => ResourceYields { food: 0, production: 0, commerce: 1, culture: 0, science: 0 },
            Self::Tunnel => ResourceYields { food: 0, production: 1, commerce: 1, culture: 0, science: 0 },
            Self::Fort => ResourceYields { food: 0, production: 0, commerce: 0, culture: 1, science: 0 },
            Self::TradingPost => ResourceYields { food: 0, production: 0, commerce: 3, culture: 0, science: 0 },
            Self::Market => ResourceYields { food: 0, production: 0, commerce: 2, culture: 1, science: 0 },
            Self::Bank => ResourceYields { food: 0, production: 0, commerce: 4, culture: 0, science: 0 },
            Self::Factory => ResourceYields { food: 0, production: 5, commerce: 0, culture: 0, science: 0 },
            Self::Port => ResourceYields { food: 1, production: 0, commerce: 3, culture: 0, science: 0 },
            Self::Temple => ResourceYields { food: 0, production: 0, commerce: 0, culture: 3, science: 0 },
            Self::University => ResourceYields { food: 0, production: 0, commerce: 0, culture: 1, science: 4 },
            Self::Library => ResourceYields { food: 0, production: 0, commerce: 0, culture: 2, science: 2 },
            Self::Monument => ResourceYields { food: 0, production: 0, commerce: 0, culture: 4, science: 0 },
            Self::Theater => ResourceYields { food: 0, production: 0, commerce: 1, culture: 3, science: 0 },
            Self::Barracks => ResourceYields { food: 0, production: 1, commerce: 0, culture: 0, science: 0 },
            Self::Arsenal => ResourceYields { food: 0, production: 3, commerce: 0, culture: 0, science: 0 },
            Self::Fortress => ResourceYields { food: 0, production: 1, commerce: 0, culture: 2, science: 0 },
            Self::Watchtower => ResourceYields { food: 0, production: 0, commerce: 0, culture: 1, science: 1 },
            Self::Bunker => ResourceYields { food: 0, production: 2, commerce: 0, culture: 0, science: 0 },
            Self::Observatory => ResourceYields { food: 0, production: 0, commerce: 0, culture: 1, science: 3 },
            Self::Lighthouse => ResourceYields { food: 0, production: 0, commerce: 2, culture: 1, science: 0 },
            Self::Aqueduct => ResourceYields { food: 2, production: 0, commerce: 0, culture: 1, science: 0 },
            Self::Windmill => ResourceYields { food: 1, production: 2, commerce: 0, culture: 0, science: 0 },
            Self::Irrigation => ResourceYields { food: 4, production: 0, commerce: 0, culture: 0, science: 0 },
        }
    }
}
