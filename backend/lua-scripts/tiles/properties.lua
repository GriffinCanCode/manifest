-- Tile Properties System - Main Configuration
-- Defines terrain types, resources, climate effects, and movement costs

-- Game API is available as global `Game`
-- Utilities are available as global utilities like `math.clamp`, `validation`, etc.

Game.log("info", "Loading tile properties configuration...")

-- Terrain type definitions with properties
TerrainTypes = {
    ocean = {
        name = "Ocean",
        movement_cost = 2.0,
        defense_bonus = 0.0,
        base_elevation = -10.0,
        can_build_land_improvements = false,
        provides_water = true,
        natural_resources = {"fish"},
        climate_modifiers = {
            temperature = -2,
            humidity = 15,
            rainfall = 5
        }
    },
    
    grassland = {
        name = "Grassland", 
        movement_cost = 1.0,
        defense_bonus = 0.0,
        base_elevation = 0.0,
        can_build_land_improvements = true,
        fertility = 0.8,
        natural_resources = {"wheat", "cattle"},
        climate_modifiers = {
            temperature = 0,
            humidity = 5,
            rainfall = 0
        }
    },
    
    plains = {
        name = "Plains",
        movement_cost = 1.0,
        defense_bonus = 0.0,
        base_elevation = 0.0,
        can_build_land_improvements = true,
        fertility = 0.6,
        natural_resources = {"wheat", "stone"},
        climate_modifiers = {
            temperature = 1,
            humidity = -5,
            rainfall = -10
        }
    },
    
    desert = {
        name = "Desert",
        movement_cost = 1.5,
        defense_bonus = 0.1,
        base_elevation = 0.0,
        can_build_land_improvements = true,
        fertility = 0.1,
        natural_resources = {"oil", "gold"},
        climate_modifiers = {
            temperature = 8,
            humidity = -20,
            rainfall = -40
        }
    },
    
    forest = {
        name = "Forest",
        movement_cost = 2.0,
        defense_bonus = 0.25,
        base_elevation = 0.0,
        can_build_land_improvements = true,
        fertility = 0.5,
        provides_lumber = true,
        natural_resources = {"lumber", "game"},
        climate_modifiers = {
            temperature = -1,
            humidity = 10,
            rainfall = 15
        }
    },
    
    hills = {
        name = "Hills",
        movement_cost = 2.0,
        defense_bonus = 0.25,
        base_elevation = 50.0,
        can_build_land_improvements = true,
        fertility = 0.4,
        natural_resources = {"stone", "iron", "copper"},
        climate_modifiers = {
            temperature = -2,
            humidity = 0,
            rainfall = 5
        }
    },
    
    mountain = {
        name = "Mountain",
        movement_cost = 3.0,
        defense_bonus = 0.5,
        base_elevation = 200.0,
        can_build_land_improvements = false,
        fertility = 0.1,
        impassable_for = {"naval"},
        natural_resources = {"stone", "iron", "gold", "silver"},
        climate_modifiers = {
            temperature = -5,
            humidity = -5,
            rainfall = 20
        }
    }
}

-- Calculate movement cost for a tile based on terrain and modifiers
function calculate_movement_cost(terrain_type, modifiers)
    modifiers = modifiers or {}
    
    local terrain = TerrainTypes[terrain_type]
    if not terrain then
        Game.log("warn", "Unknown terrain type: " .. tostring(terrain_type))
        return 1.0
    end
    
    local base_cost = terrain.movement_cost
    
    -- Apply weather modifiers
    local weather_modifier = modifiers.weather_modifier or 1.0
    local road_modifier = modifiers.road_modifier or 1.0
    local improvement_modifier = modifiers.improvement_modifier or 1.0
    
    local total_cost = base_cost * weather_modifier * road_modifier * improvement_modifier
    
    -- Clamp to reasonable range
    return math.clamp(total_cost, 0.1, 10.0)
end

-- Calculate defense bonus for a tile
function calculate_defense_bonus(terrain_type, improvements)
    improvements = improvements or {}
    
    local terrain = TerrainTypes[terrain_type]
    if not terrain then
        return 0.0
    end
    
    local base_bonus = terrain.defense_bonus
    
    -- Add improvement bonuses
    local improvement_bonus = 0.0
    for _, improvement in ipairs(improvements) do
        if improvement.type == "fort" then
            improvement_bonus = improvement_bonus + 0.5
        elseif improvement.type == "walls" then
            improvement_bonus = improvement_bonus + 0.3
        elseif improvement.type == "trench" then
            improvement_bonus = improvement_bonus + 0.2
        end
    end
    
    return math.clamp(base_bonus + improvement_bonus, 0.0, 0.9)
end

-- Determine if terrain can support specific improvements
function can_build_improvement(terrain_type, improvement_type)
    local terrain = TerrainTypes[terrain_type]
    if not terrain then
        return false
    end
    
    -- Check basic land improvement capability
    if improvement_type == "farm" or improvement_type == "mine" or improvement_type == "quarry" then
        return terrain.can_build_land_improvements
    end
    
    -- Special cases
    if improvement_type == "fishery" then
        return terrain.provides_water
    end
    
    if improvement_type == "lumber_mill" then
        return terrain.provides_lumber
    end
    
    -- Roads and basic improvements can usually be built anywhere on land
    if improvement_type == "road" or improvement_type == "fort" then
        return terrain.can_build_land_improvements
    end
    
    return true
end

-- Calculate resource yield for a tile
function calculate_resource_yield(terrain_type, resource_type, base_quantity)
    local terrain = TerrainTypes[terrain_type]
    if not terrain or not terrain.natural_resources then
        return 0
    end
    
    -- Check if terrain naturally provides this resource
    local provides_resource = false
    for _, natural_resource in ipairs(terrain.natural_resources) do
        if natural_resource == resource_type then
            provides_resource = true
            break
        end
    end
    
    if not provides_resource then
        return 0
    end
    
    -- Apply terrain-specific modifiers
    local modifier = 1.0
    if terrain.fertility and (resource_type == "wheat" or resource_type == "cattle") then
        modifier = terrain.fertility
    end
    
    return math.round(base_quantity * modifier)
end

-- Climate impact on tile properties
function apply_climate_effects(tile_data, climate)
    if not tile_data.terrain_type or not TerrainTypes[tile_data.terrain_type] then
        return tile_data
    end
    
    local terrain = TerrainTypes[tile_data.terrain_type]
    local modifiers = terrain.climate_modifiers or {}
    
    -- Apply climate modifiers
    if climate.temperature then
        tile_data.effective_temperature = climate.temperature + (modifiers.temperature or 0)
    end
    
    if climate.rainfall then
        tile_data.effective_rainfall = climate.rainfall + (modifiers.rainfall or 0)
    end
    
    if climate.humidity then
        tile_data.effective_humidity = climate.humidity + (modifiers.humidity or 0)
    end
    
    -- Calculate derived properties
    tile_data.is_frozen = (tile_data.effective_temperature or 0) < -10
    tile_data.is_tropical = (tile_data.effective_temperature or 0) > 25 and (tile_data.effective_rainfall or 0) > 150
    tile_data.is_arid = (tile_data.effective_rainfall or 0) < 25
    
    return tile_data
end

-- Event callbacks for tile changes
function on_tile_improvement_built(event_data)
    local tile_id = event_data:get("tile_id")
    local improvement_type = event_data:get("improvement_type")
    
    Game.log("info", string.format("Improvement %s built on tile %s", improvement_type, tile_id))
    
    -- Could trigger additional effects here
    -- For example, update nearby tiles, affect trade routes, etc.
end

function on_tile_terrain_changed(event_data)
    local tile_id = event_data:get("tile_id")
    local old_terrain = event_data:get("old_terrain")
    local new_terrain = event_data:get("new_terrain")
    
    Game.log("info", string.format("Tile %s terrain changed from %s to %s", tile_id, old_terrain, new_terrain))
    
    -- Recalculate tile properties
    -- This would be called by the Rust system
end

-- Register event callbacks
if Game.Events then
    Game.Events.register("improvement_built", "on_tile_improvement_built")
    Game.Events.register("terrain_changed", "on_tile_terrain_changed")
end

Game.log("info", "Tile properties system initialized successfully")

-- Export functions for use by Rust
return {
    calculate_movement_cost = calculate_movement_cost,
    calculate_defense_bonus = calculate_defense_bonus,
    can_build_improvement = can_build_improvement,
    calculate_resource_yield = calculate_resource_yield,
    apply_climate_effects = apply_climate_effects,
    terrain_types = TerrainTypes
}
