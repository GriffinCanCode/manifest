-- Resource Distribution System - Main Configuration
-- Defines all resource types with comprehensive properties for geological simulation
-- Uses penlight utilities, lume game utilities, and advanced Lua patterns

local pl = require 'pl'
local lume = require 'lume'
local moses = require 'moses'

Game.log("info", "Loading main resource configuration...")

-- Resource categories and their characteristics
local ResourceCategories = {
    strategic = {
        name = "Strategic Resources",
        base_rarity = 0.8,
        strategic_value = 0.9,
        trade_importance = 0.8,
        description = "Critical for military and advanced technology"
    },
    industrial = {
        name = "Industrial Resources", 
        base_rarity = 0.4,
        strategic_value = 0.6,
        trade_importance = 0.7,
        description = "Essential for manufacturing and construction"
    },
    precious = {
        name = "Precious Resources",
        base_rarity = 0.9,
        strategic_value = 0.3,
        trade_importance = 0.9,
        description = "High value luxury and monetary resources"
    },
    agricultural = {
        name = "Agricultural Resources",
        base_rarity = 0.2,
        strategic_value = 0.8,
        trade_importance = 0.6,
        description = "Food production and biological materials"
    },
    energy = {
        name = "Energy Resources",
        base_rarity = 0.6,
        strategic_value = 0.9,
        trade_importance = 0.8,
        description = "Power generation and fuel resources"
    },
    construction = {
        name = "Construction Materials",
        base_rarity = 0.1,
        strategic_value = 0.4,
        trade_importance = 0.5,
        description = "Building and infrastructure materials"
    }
}

-- Comprehensive resource definitions using moonscript-style DSL patterns
local ResourceTypes = {
    -- Strategic Resources
    uranium = {
        id = "uranium",
        name = "Uranium",
        category = "strategic",
        properties = {
            rarity = 0.9,
            base_value = 100.0,
            quality_range = {0.3, 1.0},
            renewable = false,
            regen_rate = 0.0,
            depletion_rate = 0.02,
            discovery_difficulty = 0.8,
            required_tech = {"nuclear_physics", "advanced_mining"}
        },
        distribution = {
            terrain_affinity = {
                mountains = 0.8,
                hills = 0.6,
                desert = 0.4,
                plains = 0.2
            },
            geological_rules = {
                elevation_range = {200, 3000},
                plate_age_range = {50, 500}, -- Precambrian shields
                tectonic_features = {"ancient_crust", "granite_intrusions"},
                boundary_distance = {20, 100},
                volcanic_requirements = {
                    active_volcanism = false,
                    ancient_volcanism = true,
                    distance_range = {10, 50}
                }
            },
            climate_rules = {
                temperature_range = {-20, 40},
                seasonal_tolerance = 0.7
            },
            clustering = {
                cluster_tendency = 0.7,
                cluster_size = 3,
                cluster_radius = 5,
                secondary_cluster_chance = 0.3
            }
        },
        economics = {
            base_demand = 20.0,
            volatility = 0.6,
            strategic_value = 0.95,
            trade_value = 1.2,
            stockpile_priority = 0.9
        }
    },
    
    oil = {
        id = "oil",
        name = "Oil",
        category = "energy", 
        properties = {
            rarity = 0.6,
            base_value = 50.0,
            quality_range = {0.4, 1.0},
            renewable = false,
            regen_rate = 0.0,
            depletion_rate = 0.05,
            discovery_difficulty = 0.6,
            required_tech = {"drilling", "geological_survey"}
        },
        distribution = {
            terrain_affinity = {
                ocean = 0.9,
                desert = 0.8,
                plains = 0.6,
                swamp = 0.7
            },
            geological_rules = {
                elevation_range = {-2000, 500},
                plate_age_range = {100, 300}, -- Mesozoic-Cenozoic
                tectonic_features = {"sedimentary_basin", "rift_valley"},
                boundary_distance = {5, 200}
            },
            climate_rules = {
                seasonal_tolerance = 0.9
            },
            clustering = {
                cluster_tendency = 0.9,
                cluster_size = 8,
                cluster_radius = 12,
                secondary_cluster_chance = 0.6
            }
        },
        economics = {
            base_demand = 80.0,
            volatility = 0.8,
            strategic_value = 0.8,
            trade_value = 1.5,
            stockpile_priority = 0.8
        }
    },
    
    -- Industrial Resources
    iron = {
        id = "iron",
        name = "Iron",
        category = "industrial",
        properties = {
            rarity = 0.3,
            base_value = 20.0,
            quality_range = {0.5, 1.0},
            renewable = false,
            regen_rate = 0.0,
            depletion_rate = 0.03,
            discovery_difficulty = 0.3,
            required_tech = {"mining"}
        },
        distribution = {
            terrain_affinity = {
                mountains = 0.8,
                hills = 0.9,
                forest = 0.6,
                plains = 0.4
            },
            geological_rules = {
                elevation_range = {0, 2500},
                plate_age_range = {200, 2000}, -- Precambrian iron formations
                tectonic_features = {"banded_iron_formation", "metamorphic_core"},
                boundary_distance = {0, 300}
            },
            clustering = {
                cluster_tendency = 0.6,
                cluster_size = 6,
                cluster_radius = 8,
                secondary_cluster_chance = 0.4
            }
        },
        economics = {
            base_demand = 100.0,
            volatility = 0.4,
            strategic_value = 0.7,
            trade_value = 1.0,
            stockpile_priority = 0.6
        }
    },
    
    coal = {
        id = "coal",
        name = "Coal", 
        category = "energy",
        properties = {
            rarity = 0.4,
            base_value = 15.0,
            quality_range = {0.3, 1.0},
            renewable = false,
            regen_rate = 0.0,
            depletion_rate = 0.04,
            discovery_difficulty = 0.2,
            required_tech = {"mining"}
        },
        distribution = {
            terrain_affinity = {
                forest = 0.9,
                hills = 0.7,
                plains = 0.6,
                swamp = 0.8
            },
            geological_rules = {
                elevation_range = {0, 1500},
                plate_age_range = {250, 350}, -- Carboniferous period
                tectonic_features = {"sedimentary_basin", "ancient_forest"},
                boundary_distance = {10, 500}
            },
            climate_rules = {
                temperature_range = {-10, 35},
                humidity_range = {30, 90},
                seasonal_tolerance = 0.8
            },
            clustering = {
                cluster_tendency = 0.8,
                cluster_size = 12,
                cluster_radius = 15,
                secondary_cluster_chance = 0.7
            }
        },
        economics = {
            base_demand = 70.0,
            volatility = 0.3,
            strategic_value = 0.6,
            trade_value = 0.8,
            stockpile_priority = 0.5
        }
    },
    
    -- Precious Resources
    gold = {
        id = "gold",
        name = "Gold",
        category = "precious",
        properties = {
            rarity = 0.95,
            base_value = 200.0,
            quality_range = {0.6, 1.0},
            renewable = false,
            regen_rate = 0.0,
            depletion_rate = 0.01,
            discovery_difficulty = 0.7,
            required_tech = {"mining", "metallurgy"}
        },
        distribution = {
            terrain_affinity = {
                mountains = 0.9,
                hills = 0.7,
                desert = 0.5,
                river = 0.6 -- Placer deposits
            },
            geological_rules = {
                elevation_range = {0, 4000},
                plate_age_range = {100, 3000},
                tectonic_features = {"hydrothermal_veins", "quartz_veins", "fault_zones"},
                boundary_distance = {0, 50},
                volcanic_requirements = {
                    active_volcanism = false,
                    ancient_volcanism = true,
                    distance_range = {1, 30}
                }
            },
            clustering = {
                cluster_tendency = 0.5,
                cluster_size = 2,
                cluster_radius = 3,
                secondary_cluster_chance = 0.2
            }
        },
        economics = {
            base_demand = 30.0,
            volatility = 0.5,
            strategic_value = 0.4,
            trade_value = 2.0,
            stockpile_priority = 0.7
        }
    },
    
    -- Agricultural Resources
    wheat = {
        id = "wheat",
        name = "Wheat",
        category = "agricultural",
        properties = {
            rarity = 0.1,
            base_value = 5.0,
            quality_range = {0.4, 1.0},
            renewable = true,
            regen_rate = 10.0, -- Regrows each season
            depletion_rate = 0.0,
            discovery_difficulty = 0.1,
            required_tech = {}
        },
        distribution = {
            terrain_affinity = {
                grassland = 0.9,
                plains = 0.8,
                hills = 0.4,
                river = 0.7 -- Fertile river valleys
            },
            geological_rules = {
                elevation_range = {0, 1200}
            },
            climate_rules = {
                temperature_range = {5, 30},
                rainfall_range = {300, 800},
                humidity_range = {40, 70},
                seasonal_tolerance = 0.6
            },
            clustering = {
                cluster_tendency = 0.4,
                cluster_size = 20,
                cluster_radius = 25,
                secondary_cluster_chance = 0.8
            }
        },
        economics = {
            base_demand = 200.0,
            volatility = 0.3,
            strategic_value = 0.8,
            trade_value = 0.5,
            stockpile_priority = 0.9
        }
    },
    
    fish = {
        id = "fish",
        name = "Fish",
        category = "agricultural",
        properties = {
            rarity = 0.2,
            base_value = 8.0,
            quality_range = {0.3, 1.0},
            renewable = true,
            regen_rate = 15.0,
            depletion_rate = 0.1, -- Overfishing can deplete
            discovery_difficulty = 0.1,
            required_tech = {}
        },
        distribution = {
            terrain_affinity = {
                ocean = 0.9,
                lake = 0.8,
                river = 0.6,
                coastal = 0.7
            },
            climate_rules = {
                temperature_range = {-2, 25}, -- Ocean temperatures
                seasonal_tolerance = 0.7
            },
            clustering = {
                cluster_tendency = 0.6,
                cluster_size = 15,
                cluster_radius = 10,
                secondary_cluster_chance = 0.5
            }
        },
        economics = {
            base_demand = 120.0,
            volatility = 0.4,
            strategic_value = 0.6,
            trade_value = 0.6,
            stockpile_priority = 0.3
        }
    }
}

-- Resource validation function
function validate_resource_type(resource_type, properties_json)
    local success, properties = pcall(function()
        return Game.json_decode(properties_json)
    end)
    
    if not success then
        Game.log("warning", "Failed to decode resource properties for " .. resource_type)
        return false
    end
    
    -- Validate required fields
    local required_fields = {"rarity", "base_value", "quality_range", "renewable"}
    for _, field in ipairs(required_fields) do
        if not properties.properties or properties.properties[field] == nil then
            Game.log("warning", "Missing required field '" .. field .. "' for resource " .. resource_type)
            return false
        end
    end
    
    return true
end

-- Get all resource types as JSON for Rust consumption
function get_resource_types()
    local resource_data = {}
    
    for resource_id, resource in pairs(ResourceTypes) do
        resource_data[resource_id] = resource
    end
    
    return Game.json_encode(resource_data)
end

-- Calculate resource properties with noise and modifiers
function calculate_resource_properties(q, r, resource_type, noise_value)
    local resource = ResourceTypes[resource_type]
    if not resource then
        Game.log("warning", "Unknown resource type: " .. resource_type)
        return 0, 0.0
    end
    
    local props = resource.properties
    
    -- Calculate quality with noise variation
    local base_quality = (props.quality_range[1] + props.quality_range[2]) / 2
    local quality_variation = (props.quality_range[2] - props.quality_range[1]) / 2
    local quality = Utils.lerp(
        props.quality_range[1],
        props.quality_range[2],
        (noise_value + 1) / 2 -- Normalize noise to 0-1
    )
    quality = math.max(props.quality_range[1], math.min(props.quality_range[2], quality))
    
    -- Calculate quantity based on rarity and quality
    local base_amount = 255 * (1.0 - props.rarity)
    local quantity = math.floor(base_amount * quality * (0.8 + 0.4 * ((noise_value + 1) / 2)))
    quantity = math.max(1, math.min(255, quantity))
    
    return quantity, quality
end

-- Advanced resource clustering using lume utilities
function generate_resource_veins(resource_type, positions)
    local resource = ResourceTypes[resource_type]
    if not resource then
        return {}
    end
    
    local clustering = resource.distribution.clustering
    local veins = {}
    
    -- Use lume to group nearby positions
    local processed = {}
    local vein_id = 1
    
    for i, pos in ipairs(positions) do
        if not processed[i] then
            local vein_positions = {pos}
            processed[i] = true
            
            -- Find connected positions within cluster radius
            for j = i + 1, #positions do
                if not processed[j] then
                    local distance = Utils.hex_distance(pos[1], pos[2], positions[j][1], positions[j][2])
                    if distance <= clustering.cluster_radius then
                        table.insert(vein_positions, positions[j])
                        processed[j] = true
                    end
                end
            end
            
            -- Create vein if we have multiple connected positions
            if #vein_positions > 1 then
                local vein_type = "clustered"
                if #vein_positions >= 5 then
                    vein_type = "massive"
                elseif clustering.cluster_tendency > 0.7 then
                    vein_type = "linear"
                end
                
                local vein = {
                    vein_id = vein_id,
                    vein_type = vein_type,
                    total_reserves = #vein_positions * 50, -- Estimate
                    connected_tiles = vein_positions
                }
                
                table.insert(veins, vein)
                vein_id = vein_id + 1
            end
        end
    end
    
    Game.log("info", "Generated " .. #veins .. " veins for " .. resource_type)
    return veins
end

-- Resource affinity calculation using moses functional programming
function calculate_terrain_affinity(resource_type, terrain_type, modifiers)
    local resource = ResourceTypes[resource_type]
    if not resource then return 0.0 end
    
    local base_affinity = resource.distribution.terrain_affinity[terrain_type] or 0.1
    
    -- Apply modifiers using moses chains
    local final_affinity = moses.chain(modifiers or {})
        :reduce(function(acc, modifier)
            return acc * modifier
        end, base_affinity)
        :value()
    
    return math.max(0.0, math.min(1.0, final_affinity))
end

-- Get substitute resources for scarcity management
function get_resource_substitutes(resource_type)
    local substitutes = {
        coal = {
            {resource_type = "oil", effectiveness = 0.8, cost_multiplier = 1.2},
            {resource_type = "uranium", effectiveness = 2.0, cost_multiplier = 5.0, required_tech = "nuclear_power"}
        },
        iron = {
            {resource_type = "copper", effectiveness = 0.6, cost_multiplier = 0.8},
            {resource_type = "aluminum", effectiveness = 0.9, cost_multiplier = 1.5, required_tech = "advanced_metallurgy"}
        },
        wheat = {
            {resource_type = "fish", effectiveness = 0.7, cost_multiplier = 1.1},
            {resource_type = "cattle", effectiveness = 0.8, cost_multiplier = 1.3}
        }
    }
    
    return substitutes[resource_type] or {}
end

-- Export main functions
Game.log("info", "Resource types system initialized with " .. moses.size(ResourceTypes) .. " resources")

return {
    validate_resource_type = validate_resource_type,
    get_resource_types = get_resource_types,
    calculate_resource_properties = calculate_resource_properties,
    generate_resource_veins = generate_resource_veins,
    calculate_terrain_affinity = calculate_terrain_affinity,
    get_resource_substitutes = get_resource_substitutes,
    resource_categories = ResourceCategories,
    resource_types = ResourceTypes
}
