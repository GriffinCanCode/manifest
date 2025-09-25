-- Ocean Currents System
-- Uses lume game utilities for ocean current simulation and temperature/humidity effects
-- Implements realistic oceanic circulation patterns

local lume = require 'lume'

Game.log("info", "Loading ocean currents system...")

-- Ocean current configuration
local OceanCurrents = {
    -- Current strength multipliers by latitude band
    latitude_bands = {},
    
    -- Current direction patterns (in radians)
    circulation_patterns = {},
    
    -- Temperature effects of currents
    temperature_effects = {
        warm_current = 3.0,   -- °C warming from warm currents
        cold_current = -4.0,  -- °C cooling from cold currents
        neutral = 0.0
    },
    
    -- Humidity effects of currents
    humidity_effects = {
        warm_current = 15,    -- % increase in humidity
        cold_current = -8,    -- % decrease in humidity
        neutral = 0
    }
}

-- Initialize realistic ocean circulation patterns
function OceanCurrents:init()
    -- Setup latitude bands for different circulation patterns
    -- Based on real-world oceanic gyres and circulation
    
    -- Equatorial currents (0-10° N/S)
    for lat = 80, 100 do -- 0-10° N in our coordinate system
        self.latitude_bands[lat] = {
            strength = lume.randomchoice({0.6, 0.7, 0.8}),
            direction = 1.5, -- Westward (trade wind driven)
            type = "equatorial_westward"
        }
    end
    
    for lat = 180-100, 180-80 do -- 0-10° S
        self.latitude_bands[lat] = {
            strength = lume.randomchoice({0.6, 0.7, 0.8}),
            direction = 1.5, -- Westward
            type = "equatorial_westward"
        }
    end
    
    -- Subtropical gyres (10-40° N/S)
    for lat = 50, 80 do -- 10-40° N
        self.latitude_bands[lat] = {
            strength = lume.randomchoice({0.8, 0.9, 1.0}),
            direction = -1.0, -- Eastward (return flow)
            type = "subtropical_gyre"
        }
    end
    
    for lat = 180-80, 180-50 do -- 10-40° S
        self.latitude_bands[lat] = {
            strength = lume.randomchoice({0.8, 0.9, 1.0}),
            direction = -1.0, -- Eastward
            type = "subtropical_gyre"
        }
    end
    
    -- Western boundary currents (warm)
    for lat = 60, 90 do -- 20-50° N
        if lume.chance(0.1) then -- Only on western boundaries
            self.latitude_bands[lat] = {
                strength = 1.2,
                direction = 0.5, -- Northward
                type = "warm_western_boundary",
                temperature_effect = "warm_current"
            }
        end
    end
    
    -- Eastern boundary currents (cold)
    for lat = 60, 90 do -- 20-50° N
        if lume.chance(0.1) then -- Only on eastern boundaries
            self.latitude_bands[lat] = {
                strength = 0.8,
                direction = -2.6, -- Southward
                type = "cold_eastern_boundary",
                temperature_effect = "cold_current"
            }
        end
    end
    
    -- Polar currents
    for lat = 0, 30 do -- High northern latitudes
        self.latitude_bands[lat] = {
            strength = 0.4,
            direction = 1.2, -- Variable
            type = "polar"
        }
    end
    
    for lat = 150, 180 do -- High southern latitudes
        self.latitude_bands[lat] = {
            strength = 0.5,
            direction = -1.2, -- Circumpolar
            type = "circumpolar"
        }
    end
    
    Game.log("info", string.format("Initialized ocean currents for %d latitude bands", 
                                 lume.count(self.latitude_bands)))
end

function OceanCurrents:get_current_at_position(x, y)
    -- Calculate latitude band
    local world_height = 256
    local latitude_index = math.floor((y / world_height) * 180)
    latitude_index = lume.clamp(latitude_index, 0, 179)
    
    -- Get base current from latitude band
    local base_current = self.latitude_bands[latitude_index] or {
        strength = 0.3,
        direction = 0.0,
        type = "calm"
    }
    
    -- Add noise for realistic variation
    local noise_x = (x * 0.01) % 1.0
    local noise_y = (y * 0.01) % 1.0
    local noise = (math.sin(noise_x * math.pi * 2) + math.cos(noise_y * math.pi * 2)) * 0.1
    
    local current = lume.clone(base_current)
    current.strength = lume.clamp(current.strength + noise, 0.1, 1.5)
    current.direction = current.direction + (noise * 0.3)
    
    return current
end

function OceanCurrents:calculate_temperature_effect(current, base_temperature)
    local temp_effect = self.temperature_effects.neutral
    
    if current.temperature_effect == "warm_current" then
        temp_effect = self.temperature_effects.warm_current
    elseif current.temperature_effect == "cold_current" then
        temp_effect = self.temperature_effects.cold_current
    else
        -- Determine effect from current type and strength
        if current.type == "warm_western_boundary" then
            temp_effect = self.temperature_effects.warm_current * current.strength
        elseif current.type == "cold_eastern_boundary" then
            temp_effect = self.temperature_effects.cold_current * current.strength
        elseif current.strength > 0.8 then
            -- Strong currents have moderate warming effect
            temp_effect = 1.0 * current.strength
        end
    end
    
    return math.floor(base_temperature + temp_effect + 0.5)
end

function OceanCurrents:calculate_humidity_effect(current, base_humidity)
    local humidity_effect = self.humidity_effects.neutral
    
    if current.temperature_effect == "warm_current" then
        humidity_effect = self.humidity_effects.warm_current
    elseif current.temperature_effect == "cold_current" then
        humidity_effect = self.humidity_effects.cold_current
    else
        -- Calculate based on current type
        if current.type == "warm_western_boundary" then
            humidity_effect = self.humidity_effects.warm_current * current.strength
        elseif current.type == "cold_eastern_boundary" then
            humidity_effect = self.humidity_effects.cold_current * current.strength
        elseif current.strength > 0.7 then
            -- Strong ocean currents increase evaporation
            humidity_effect = 8 * current.strength
        end
    end
    
    return lume.clamp(base_humidity + humidity_effect, 0, 100)
end

-- Initialize the ocean currents system
OceanCurrents:init()

-- Event handler for ocean current effects on climate
Game.register_event_callback("climate_ocean_currents", function(event_data)
    Game.log("debug", "Processing ocean currents for tile: " .. tostring(event_data.tile_id))
    
    local x = tonumber(event_data.x) or 0
    local y = tonumber(event_data.y) or 0
    local base_temperature = tonumber(event_data.base_temperature) or 15
    local base_humidity = tonumber(event_data.base_humidity) or 50
    local terrain_type = event_data.terrain_type or "unknown"
    
    -- Only apply ocean currents to water tiles or coastal areas
    local is_water = lume.find({"ocean", "sea", "coast", "coastal"}, terrain_type) ~= nil
    local is_coastal = lume.find({"coastal", "beach", "shore"}, terrain_type) ~= nil
    
    if not (is_water or is_coastal) then
        -- No ocean current effects on inland tiles
        return {}
    end
    
    local current = OceanCurrents:get_current_at_position(x, y)
    
    local results = {}
    
    -- Calculate temperature modification
    local new_temperature = OceanCurrents:calculate_temperature_effect(current, base_temperature)
    if new_temperature ~= base_temperature then
        table.insert(results, "temperature_mod:" .. tostring(new_temperature - base_temperature))
    end
    
    -- Calculate humidity modification  
    local new_humidity = OceanCurrents:calculate_humidity_effect(current, base_humidity)
    if new_humidity ~= base_humidity then
        table.insert(results, "humidity_mod:" .. tostring(new_humidity - base_humidity))
    end
    
    -- Store current data for other systems
    table.insert(results, "current_strength:" .. string.format("%.2f", current.strength))
    table.insert(results, "current_direction:" .. string.format("%.2f", current.direction))
    table.insert(results, "current_type:" .. current.type)
    
    Game.log("debug", string.format("Ocean current: %s, Strength: %.2f, Temp effect: %+d°C, Humidity effect: %+d%%",
                                   current.type, current.strength,
                                   new_temperature - base_temperature,
                                   new_humidity - base_humidity))
    
    return results
end)

-- Event handler for calculating ocean current properties
Game.register_event_callback("ocean_current_calculation", function(event_data)
    local x = tonumber(event_data.x) or 0
    local y = tonumber(event_data.y) or 0
    
    local current = OceanCurrents:get_current_at_position(x, y)
    
    return {
        "strength:" .. string.format("%.3f", current.strength),
        "direction:" .. string.format("%.3f", current.direction)
    }
end)

-- Utility functions for other climate scripts
function get_ocean_current_at(x, y)
    return OceanCurrents:get_current_at_position(x, y)
end

function get_ocean_temperature_effect(current, base_temp)
    return OceanCurrents:calculate_temperature_effect(current, base_temp)
end

Game.log("info", "Ocean currents system loaded with " .. lume.count(OceanCurrents.latitude_bands) .. " circulation patterns")
