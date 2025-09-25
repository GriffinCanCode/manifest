//! Resource configuration system with TOML loading
//!
//! Provides resource definitions, configurations loaded from TOML files,
//! and resource yield calculations for tiles.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Resource configuration loaded from TOML files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConfig {
    pub resources: HashMap<String, ResourceDefinition>,
}

impl Default for ResourceConfig {
    fn default() -> Self {
        Self {
            resources: HashMap::new(),
        }
    }
}

impl ResourceConfig {
    /// Get resource definition by name
    pub fn get_resource(&self, name: &str) -> Option<&ResourceDefinition> {
        self.resources.get(name)
    }

    /// Get all resources of a specific category
    pub fn get_resources_by_category(&self, category: ResourceCategory) -> Vec<&ResourceDefinition> {
        self.resources
            .values()
            .filter(|resource| resource.category() == category)
            .collect()
    }

    /// Get resources that can appear on specific terrain
    pub fn get_terrain_resources(&self, terrain_type: &str) -> Vec<&ResourceDefinition> {
        self.resources
            .values()
            .filter(|resource| resource.terrain_preferences.contains(&terrain_type.to_string()))
            .collect()
    }

    /// Get luxury resources
    pub fn get_luxury_resources(&self) -> Vec<&ResourceDefinition> {
        self.get_resources_by_category(ResourceCategory::Luxury)
    }

    /// Get strategic resources
    pub fn get_strategic_resources(&self) -> Vec<&ResourceDefinition> {
        self.get_resources_by_category(ResourceCategory::Strategic)
    }

    /// Get basic resources
    pub fn get_basic_resources(&self) -> Vec<&ResourceDefinition> {
        self.get_resources_by_category(ResourceCategory::Basic)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceDefinition {
    pub name: String,
    pub rarity: f32,
    pub base_yield: u8,
    pub required_tech: Option<String>,
    pub terrain_preferences: Vec<String>,
    pub biome_modifiers: HashMap<String, f32>,
}

impl ResourceDefinition {
    /// Get resource category based on properties
    pub fn category(&self) -> ResourceCategory {
        match self.rarity {
            r if r < 0.1 => ResourceCategory::Strategic,
            r if r < 0.3 => ResourceCategory::Luxury,
            _ => ResourceCategory::Basic,
        }
    }

    /// Check if resource is available with given technology level
    pub fn is_available_with_tech(&self, available_techs: &[String]) -> bool {
        match &self.required_tech {
            Some(required) => available_techs.contains(required),
            None => true,
        }
    }

    /// Calculate yield for specific biome
    pub fn calculate_biome_yield(&self, biome_type: &str) -> u8 {
        let modifier = self.biome_modifiers.get(biome_type).unwrap_or(&1.0);
        (self.base_yield as f32 * modifier).round() as u8
    }

    /// Get spawn probability for this terrain type
    pub fn spawn_probability(&self, terrain_type: &str) -> f32 {
        if self.terrain_preferences.contains(&terrain_type.to_string()) {
            self.rarity * 2.0 // Double chance on preferred terrain
        } else {
            self.rarity * 0.5 // Half chance on other terrain
        }
    }

    /// Check if resource provides strategic advantage
    pub fn is_strategic(&self) -> bool {
        self.category() == ResourceCategory::Strategic
    }

    /// Check if resource provides luxury benefits
    pub fn is_luxury(&self) -> bool {
        self.category() == ResourceCategory::Luxury
    }

    /// Get trade value of this resource
    pub fn trade_value(&self) -> u32 {
        match self.category() {
            ResourceCategory::Strategic => self.base_yield as u32 * 50,
            ResourceCategory::Luxury => self.base_yield as u32 * 25,
            ResourceCategory::Basic => self.base_yield as u32 * 10,
        }
    }
}

/// Resource categories for gameplay mechanics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceCategory {
    Basic,
    Luxury,
    Strategic,
}

impl ResourceCategory {
    /// Get maximum yield for this category
    pub fn max_yield(&self) -> u8 {
        match self {
            Self::Basic => 5,
            Self::Luxury => 3,
            Self::Strategic => 2,
        }
    }

    /// Get rarity range for this category
    pub fn rarity_range(&self) -> (f32, f32) {
        match self {
            Self::Basic => (0.3, 1.0),
            Self::Luxury => (0.1, 0.3),
            Self::Strategic => (0.01, 0.1),
        }
    }

    /// Get happiness bonus for luxury resources
    pub fn happiness_bonus(&self) -> f32 {
        match self {
            Self::Basic => 0.0,
            Self::Luxury => 2.0,
            Self::Strategic => 0.0,
        }
    }
}

/// Resource spawner for world generation
pub struct ResourceSpawner;

impl ResourceSpawner {
    /// Generate resources for a tile based on terrain, biome, and config
    pub fn generate_tile_resources(
        terrain_type: &str,
        biome_type: &str,
        resource_config: &ResourceConfig,
        rng_seed: u64,
    ) -> Vec<String> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        rng_seed.hash(&mut hasher);
        terrain_type.hash(&mut hasher);
        biome_type.hash(&mut hasher);
        let hash_value = hasher.finish();

        // Simple deterministic "random" based on hash
        let random_value = (hash_value % 1000) as f32 / 1000.0;

        let mut resources = Vec::new();
        let mut probability_used = 0.0;

        // Sort resources by rarity (rarest first)
        let mut sorted_resources: Vec<_> = resource_config.resources.iter().collect();
        sorted_resources.sort_by(|a, b| a.1.rarity.partial_cmp(&b.1.rarity).unwrap());

        for (resource_name, resource_def) in sorted_resources {
            let spawn_prob = resource_def.spawn_probability(terrain_type);
            
            // Adjust probability based on already spawned resources
            let adjusted_prob = spawn_prob * (1.0 - probability_used);
            
            if random_value < adjusted_prob {
                resources.push(resource_name.clone());
                probability_used += adjusted_prob;
                
                // Strategic resources are exclusive (only one per tile)
                if resource_def.is_strategic() {
                    break;
                }
                
                // Limit total resources per tile
                if resources.len() >= 3 {
                    break;
                }
            }
        }

        resources
    }

    /// Calculate total resource yield for a tile
    pub fn calculate_total_yield(
        resources: &[String],
        biome_type: &str,
        resource_config: &ResourceConfig,
    ) -> ResourceYields {
        let mut yields = ResourceYields::default();

        for resource_name in resources {
            if let Some(resource_def) = resource_config.get_resource(resource_name) {
                let yield_amount = resource_def.calculate_biome_yield(biome_type);
                
                match resource_def.category() {
                    ResourceCategory::Basic => yields.food += yield_amount,
                    ResourceCategory::Luxury => yields.commerce += yield_amount,
                    ResourceCategory::Strategic => yields.production += yield_amount,
                }
            }
        }

        yields
    }
}

/// Resource yields structure
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceYields {
    pub food: u8,
    pub production: u8,
    pub commerce: u8,
    pub science: u8,
    pub culture: u8,
}

impl ResourceYields {
    /// Add two yield structures together
    pub fn add(&self, other: ResourceYields) -> ResourceYields {
        ResourceYields {
            food: self.food + other.food,
            production: self.production + other.production,
            commerce: self.commerce + other.commerce,
            science: self.science + other.science,
            culture: self.culture + other.culture,
        }
    }

    /// Scale yields by a multiplier
    pub fn scale(&self, multiplier: f32) -> ResourceYields {
        ResourceYields {
            food: (self.food as f32 * multiplier).round() as u8,
            production: (self.production as f32 * multiplier).round() as u8,
            commerce: (self.commerce as f32 * multiplier).round() as u8,
            science: (self.science as f32 * multiplier).round() as u8,
            culture: (self.culture as f32 * multiplier).round() as u8,
        }
    }

    /// Get total yield value
    pub fn total(&self) -> u16 {
        self.food as u16 + self.production as u16 + self.commerce as u16 + 
        self.science as u16 + self.culture as u16
    }

    /// Check if yields are empty
    pub fn is_empty(&self) -> bool {
        self.total() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_resource_config() -> ResourceConfig {
        let mut resources = HashMap::new();
        
        resources.insert("wheat".to_string(), ResourceDefinition {
            name: "Wheat".to_string(),
            rarity: 0.8,
            base_yield: 3,
            required_tech: None,
            terrain_preferences: vec!["grassland".to_string(), "plains".to_string()],
            biome_modifiers: {
                let mut modifiers = HashMap::new();
                modifiers.insert("temperate_grassland".to_string(), 1.2);
                modifiers.insert("arid_desert".to_string(), 0.5);
                modifiers
            },
        });

        resources.insert("iron".to_string(), ResourceDefinition {
            name: "Iron".to_string(),
            rarity: 0.05,
            base_yield: 2,
            required_tech: Some("metallurgy".to_string()),
            terrain_preferences: vec!["hills".to_string(), "mountain".to_string()],
            biome_modifiers: HashMap::new(),
        });

        ResourceConfig { resources }
    }

    #[test]
    fn test_resource_categories() {
        let config = create_test_resource_config();
        
        let wheat = config.get_resource("wheat").unwrap();
        assert_eq!(wheat.category(), ResourceCategory::Basic);
        
        let iron = config.get_resource("iron").unwrap();
        assert_eq!(iron.category(), ResourceCategory::Strategic);
    }

    #[test]
    fn test_biome_yield_calculation() {
        let config = create_test_resource_config();
        let wheat = config.get_resource("wheat").unwrap();
        
        assert_eq!(wheat.calculate_biome_yield("temperate_grassland"), 4); // 3 * 1.2 = 3.6 -> 4
        assert_eq!(wheat.calculate_biome_yield("arid_desert"), 2); // 3 * 0.5 = 1.5 -> 2
        assert_eq!(wheat.calculate_biome_yield("unknown_biome"), 3); // 3 * 1.0 = 3
    }

    #[test]
    fn test_tech_requirements() {
        let config = create_test_resource_config();
        let iron = config.get_resource("iron").unwrap();
        
        assert!(!iron.is_available_with_tech(&[]));
        assert!(!iron.is_available_with_tech(&["agriculture".to_string()]));
        assert!(iron.is_available_with_tech(&["metallurgy".to_string()]));
    }

    #[test]
    fn test_spawn_probability() {
        let config = create_test_resource_config();
        let wheat = config.get_resource("wheat").unwrap();
        
        // Double chance on preferred terrain
        assert_eq!(wheat.spawn_probability("grassland"), 1.6); // 0.8 * 2.0
        
        // Half chance on other terrain
        assert_eq!(wheat.spawn_probability("ocean"), 0.4); // 0.8 * 0.5
    }

    #[test]
    fn test_resource_yields_operations() {
        let yields1 = ResourceYields {
            food: 2,
            production: 1,
            commerce: 0,
            science: 0,
            culture: 0,
        };
        
        let yields2 = ResourceYields {
            food: 1,
            production: 0,
            commerce: 3,
            science: 1,
            culture: 0,
        };
        
        let combined = yields1.add(yields2);
        assert_eq!(combined.food, 3);
        assert_eq!(combined.production, 1);
        assert_eq!(combined.commerce, 3);
        assert_eq!(combined.total(), 8);
        
        let scaled = yields1.scale(2.0);
        assert_eq!(scaled.food, 4);
        assert_eq!(scaled.production, 2);
    }

    #[test]
    fn test_resource_generation() {
        let config = create_test_resource_config();
        
        // Test deterministic generation
        let resources1 = ResourceSpawner::generate_tile_resources(
            "grassland",
            "temperate_grassland",
            &config,
            12345
        );
        
        let resources2 = ResourceSpawner::generate_tile_resources(
            "grassland", 
            "temperate_grassland",
            &config,
            12345
        );
        
        // Should be deterministic with same seed
        assert_eq!(resources1, resources2);
        
        // Different seed should potentially give different results
        let resources3 = ResourceSpawner::generate_tile_resources(
            "grassland",
            "temperate_grassland", 
            &config,
            54321
        );
        
        // May or may not be different, but at least we're testing the function works
        assert!(resources3.len() <= 3); // Should respect max resources limit
    }

    #[test]
    fn test_total_yield_calculation() {
        let config = create_test_resource_config();
        let resources = vec!["wheat".to_string()];
        
        let yields = ResourceSpawner::calculate_total_yield(
            &resources,
            "temperate_grassland",
            &config
        );
        
        assert_eq!(yields.food, 4); // Wheat with temperate grassland modifier
        assert!(yields.total() > 0);
    }
}
