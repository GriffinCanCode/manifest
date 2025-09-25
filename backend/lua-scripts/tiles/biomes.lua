-- Biome System Configuration
-- Defines biome types based on climate data and terrain combinations

Game.log("info", "Loading biome system configuration...")

-- Biome definitions with climate requirements and effects
BiomeTypes = {
    arctic_tundra = {
        name = "Arctic Tundra",
        requirements = {
            temperature = {min = -50, max = -5},
            rainfall = {min = 0, max = 100},
        },
        preferred_terrains = {"tundra", "snow"},
        modifiers = {
            movement_cost_multiplier = 1.5,
            building_cost_multiplier = 2.0,
            population_growth_rate = -0.3,
            resource_yield_multiplier = 0.3,
        },
        special_resources = {"furs", "seal", "whale"},
        description = "Frozen wasteland with permafrost and sparse vegetation"
    },
    
    boreal_forest = {
        name = "Boreal Forest",
        requirements = {
            temperature = {min = -5, max = 10},
            rainfall = {min = 50, max = 200},
        },
        preferred_terrains = {"forest", "hills"},
        modifiers = {
            movement_cost_multiplier = 1.2,
            lumber_yield_multiplier = 2.0,
            defense_bonus = 0.1,
            hunting_yield = 1.5,
        },
        special_resources = {"lumber", "game", "furs"},
        description = "Cold coniferous forests with abundant timber"
    },
    
    temperate_grassland = {
        name = "Temperate Grassland",
        requirements = {
            temperature = {min = 5, max = 20},
            rainfall = {min = 30, max = 100},
        },
        preferred_terrains = {"grassland", "plains"},
        modifiers = {
            movement_cost_multiplier = 0.9,
            agriculture_yield_multiplier = 1.5,
            population_capacity_multiplier = 1.3,
            cavalry_bonus = 0.2,
        },
        special_resources = {"wheat", "cattle", "horses"},
        description = "Fertile plains ideal for agriculture and grazing"
    },
    
    temperate_forest = {
        name = "Temperate Forest",
        requirements = {
            temperature = {min = 0, max = 25},
            rainfall = {min = 80, max = 250},
        },
        preferred_terrains = {"forest", "hills"},
        modifiers = {
            movement_cost_multiplier = 1.3,
            lumber_yield_multiplier = 1.8,
            defense_bonus = 0.2,
            building_cost_multiplier = 0.8,
        },
        special_resources = {"lumber", "game", "berries"},
        description = "Dense deciduous forests with rich biodiversity"
    },
    
    mediterranean = {
        name = "Mediterranean",
        requirements = {
            temperature = {min = 10, max = 25},
            rainfall = {min = 40, max = 80},
        },
        preferred_terrains = {"hills", "plains"},
        modifiers = {
            agriculture_yield_multiplier = 1.2,
            trade_income_multiplier = 1.3,
            population_happiness = 0.1,
            naval_movement_bonus = 0.2,
        },
        special_resources = {"olives", "grapes", "fish"},
        description = "Warm, dry climate ideal for trade and specialized crops"
    },
    
    hot_desert = {
        name = "Hot Desert",
        requirements = {
            temperature = {min = 20, max = 50},
            rainfall = {min = 0, max = 25},
        },
        preferred_terrains = {"desert"},
        modifiers = {
            movement_cost_multiplier = 1.4,
            water_consumption = 2.0,
            population_capacity_multiplier = 0.2,
            mining_yield_multiplier = 1.5,
        },
        special_resources = {"oil", "gold", "precious_stones"},
        description = "Harsh, arid environment with valuable mineral deposits"
    },
    
    tropical_rainforest = {
        name = "Tropical Rainforest",
        requirements = {
            temperature = {min = 18, max = 35},
            rainfall = {min = 200, max = 500},
        },
        preferred_terrains = {"jungle", "forest"},
        modifiers = {
            movement_cost_multiplier = 2.0,
            disease_resistance = -0.3,
            biodiversity_bonus = 2.0,
            research_bonus = 0.2,
        },
        special_resources = {"exotic_woods", "spices", "medicinal_plants"},
        description = "Dense jungle with incredible biodiversity but difficult terrain"
    },
    
    tropical_savanna = {
        name = "Tropical Savanna",
        requirements = {
            temperature = {min = 18, max = 30},
            rainfall = {min = 50, max = 150},
        },
        preferred_terrains = {"grassland", "plains"},
        modifiers = {
            movement_cost_multiplier = 1.1,
            hunting_yield = 2.0,
            cattle_yield_multiplier = 1.3,
            seasonal_variation = 0.3,
        },
        special_resources = {"ivory", "exotic_animals", "cattle"},
        description = "Warm grasslands with seasonal rainfall and diverse wildlife"
    },
    
    mountain_alpine = {
        name = "Alpine",
        requirements = {
            elevation = {min = 1000},
            -- Temperature decreases with elevation
            temperature = {min = -10, max = 15},
        },
        preferred_terrains = {"mountain", "hills"},
        modifiers = {
            movement_cost_multiplier = 2.5,
            mining_yield_multiplier = 2.0,
            defense_bonus = 0.4,
            building_cost_multiplier = 1.8,
        },
        special_resources = {"precious_metals", "stone", "mineral_water"},
        description = "High altitude terrain with rich mineral deposits"
    }
}

-- Determine biome type based on climate and terrain data
function determine_biome(climate_data, terrain_type, elevation)
    elevation = elevation or 0
    
    if not climate_data.temperature or not climate_data.rainfall then
        Game.log("warn", "Insufficient climate data for biome determination")
        return nil
    end
    
    local candidates = {}
    
    -- Check each biome type against requirements
    for biome_id, biome in pairs(BiomeTypes) do
        local matches = true
        
        -- Check temperature requirement
        if biome.requirements.temperature then
            local temp = climate_data.temperature
            if temp < biome.requirements.temperature.min or temp > biome.requirements.temperature.max then
                matches = false
            end
        end
        
        -- Check rainfall requirement
        if matches and biome.requirements.rainfall then
            local rainfall = climate_data.rainfall
            if rainfall < biome.requirements.rainfall.min or rainfall > biome.requirements.rainfall.max then
                matches = false
            end
        end
        
        -- Check elevation requirement
        if matches and biome.requirements.elevation then
            if elevation < (biome.requirements.elevation.min or 0) or 
               elevation > (biome.requirements.elevation.max or 10000) then
                matches = false
            end
        end
        
        -- Check terrain preference
        if matches and biome.preferred_terrains then
            local terrain_match = false
            for _, preferred in ipairs(biome.preferred_terrains) do
                if preferred == terrain_type then
                    terrain_match = true
                    break
                end
            end
            if not terrain_match then
                matches = false
            end
        end
        
        if matches then
            -- Calculate suitability score based on how well conditions match
            local score = calculate_biome_suitability(biome, climate_data, terrain_type, elevation)
            table.insert(candidates, {id = biome_id, biome = biome, score = score})
        end
    end
    
    -- Sort by suitability score and return the best match
    if #candidates > 0 then
        table.sort(candidates, function(a, b) return a.score > b.score end)
        return candidates[1].id, candidates[1].biome
    end
    
    return nil
end

-- Calculate how suitable a biome is for given conditions (0.0 to 1.0)
function calculate_biome_suitability(biome, climate_data, terrain_type, elevation)
    local score = 0.5 -- Base score
    
    -- Temperature suitability
    if biome.requirements.temperature then
        local temp = climate_data.temperature
        local temp_range = biome.requirements.temperature.max - biome.requirements.temperature.min
        local temp_center = (biome.requirements.temperature.max + biome.requirements.temperature.min) / 2
        local temp_deviation = math.abs(temp - temp_center) / (temp_range / 2)
        score = score + (1.0 - math.clamp(temp_deviation, 0, 1)) * 0.3
    end
    
    -- Rainfall suitability
    if biome.requirements.rainfall then
        local rainfall = climate_data.rainfall
        local rain_range = biome.requirements.rainfall.max - biome.requirements.rainfall.min
        local rain_center = (biome.requirements.rainfall.max + biome.requirements.rainfall.min) / 2
        local rain_deviation = math.abs(rainfall - rain_center) / (rain_range / 2)
        score = score + (1.0 - math.clamp(rain_deviation, 0, 1)) * 0.2
    end
    
    -- Terrain preference bonus
    if biome.preferred_terrains then
        for _, preferred in ipairs(biome.preferred_terrains) do
            if preferred == terrain_type then
                score = score + 0.2
                break
            end
        end
    end
    
    return math.clamp(score, 0.0, 1.0)
end

-- Apply biome modifiers to tile properties
function apply_biome_modifiers(tile_properties, biome_id)
    if not biome_id or not BiomeTypes[biome_id] then
        return tile_properties
    end
    
    local biome = BiomeTypes[biome_id]
    local modifiers = biome.modifiers or {}
    
    -- Apply movement cost modifier
    if modifiers.movement_cost_multiplier and tile_properties.movement_cost then
        tile_properties.movement_cost = tile_properties.movement_cost * modifiers.movement_cost_multiplier
    end
    
    -- Apply defense bonus
    if modifiers.defense_bonus and tile_properties.defense_bonus then
        tile_properties.defense_bonus = tile_properties.defense_bonus + modifiers.defense_bonus
    end
    
    -- Apply resource yield modifiers
    if modifiers.agriculture_yield_multiplier and tile_properties.agriculture_yield then
        tile_properties.agriculture_yield = tile_properties.agriculture_yield * modifiers.agriculture_yield_multiplier
    end
    
    if modifiers.mining_yield_multiplier and tile_properties.mining_yield then
        tile_properties.mining_yield = tile_properties.mining_yield * modifiers.mining_yield_multiplier
    end
    
    if modifiers.lumber_yield_multiplier and tile_properties.lumber_yield then
        tile_properties.lumber_yield = tile_properties.lumber_yield * modifiers.lumber_yield_multiplier
    end
    
    -- Store biome-specific properties
    tile_properties.biome = biome_id
    tile_properties.biome_name = biome.name
    tile_properties.biome_description = biome.description
    
    return tile_properties
end

-- Get special resources available in a biome
function get_biome_special_resources(biome_id)
    if not biome_id or not BiomeTypes[biome_id] then
        return {}
    end
    
    return BiomeTypes[biome_id].special_resources or {}
end

-- Check if a biome has seasonal variations
function has_seasonal_effects(biome_id)
    if not biome_id or not BiomeTypes[biome_id] then
        return false
    end
    
    local modifiers = BiomeTypes[biome_id].modifiers or {}
    return modifiers.seasonal_variation and modifiers.seasonal_variation > 0
end

-- Apply seasonal effects to biome properties
function apply_seasonal_effects(tile_properties, biome_id, season)
    if not has_seasonal_effects(biome_id) then
        return tile_properties
    end
    
    local biome = BiomeTypes[biome_id]
    local seasonal_intensity = biome.modifiers.seasonal_variation or 0
    
    -- Season-specific effects (spring=0, summer=1, autumn=2, winter=3)
    local season_effects = {
        [0] = {movement = 1.2, growth = 1.5}, -- Spring: muddy but growing
        [1] = {movement = 1.0, growth = 1.0}, -- Summer: normal
        [2] = {movement = 1.1, growth = 0.8}, -- Autumn: harvest time
        [3] = {movement = 1.5, growth = 0.3}, -- Winter: difficult movement
    }
    
    local effects = season_effects[season] or season_effects[1]
    
    if tile_properties.movement_cost then
        tile_properties.movement_cost = tile_properties.movement_cost * 
            (1.0 + (effects.movement - 1.0) * seasonal_intensity)
    end
    
    if tile_properties.agriculture_yield then
        tile_properties.agriculture_yield = tile_properties.agriculture_yield * 
            (1.0 + (effects.growth - 1.0) * seasonal_intensity)
    end
    
    return tile_properties
end

Game.log("info", "Biome system initialized with " .. table_utils.size(BiomeTypes) .. " biome types")

-- Export functions for use by Rust
return {
    determine_biome = determine_biome,
    calculate_biome_suitability = calculate_biome_suitability,
    apply_biome_modifiers = apply_biome_modifiers,
    get_biome_special_resources = get_biome_special_resources,
    has_seasonal_effects = has_seasonal_effects,
    apply_seasonal_effects = apply_seasonal_effects,
    biome_types = BiomeTypes
}
