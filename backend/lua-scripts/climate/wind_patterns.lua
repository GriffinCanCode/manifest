-- Wind Patterns System  
-- Uses moses functional programming utilities for wind pattern calculations
-- Implements realistic atmospheric circulation with terrain effects

local moses = require 'moses'

Game.log("info", "Loading wind patterns system...")

-- Wind patterns configuration
local WindPatterns = {
    -- Global wind belts based on atmospheric circulation
    wind_belts = {},
    
    -- Terrain effects on wind
    terrain_effects = {
        mountain = { speed_multiplier = 1.8, turbulence = 0.4, direction_change = 0.3 },
        valley = { speed_multiplier = 0.6, turbulence = 0.1, direction_change = -0.2 },
        plains = { speed_multiplier = 1.0, turbulence = 0.0, direction_change = 0.0 },
        forest = { speed_multiplier = 0.4, turbulence = 0.2, direction_change = 0.1 },
        desert = { speed_multiplier = 1.2, turbulence = 0.3, direction_change = 0.0 },
        coast = { speed_multiplier = 1.3, turbulence = 0.1, direction_change = 0.1 },
        ocean = { speed_multiplier = 1.1, turbulence = 0.0, direction_change = 0.0 }
    },
    
    -- Seasonal variation strength
    seasonal_variation = {
        equatorial = 0.1,  -- Minimal seasonal change
        tropical = 0.2,    -- Moderate seasonal change  
        temperate = 0.4,   -- Strong seasonal change
        polar = 0.3        -- Moderate seasonal change
    }
}

-- Initialize global wind belt system
function WindPatterns:init()
    -- Create wind belts based on real atmospheric circulation
    -- Using functional programming approach with moses
    
    local latitude_bands = moses.range(0, 179)
    
    self.wind_belts = moses.map(latitude_bands, function(lat)
        -- Convert to actual latitude (-90 to +90)
        local actual_lat = (lat / 180.0 - 0.5) * 180
        local abs_lat = math.abs(actual_lat)
        
        local wind_data = {
            latitude = actual_lat,
            index = lat
        }
        
        -- Determine wind belt based on latitude
        if abs_lat >= 0 and abs_lat < 30 then
            -- Trade winds (0-30°)
            wind_data.belt_name = "trade_winds"
            wind_data.direction = actual_lat >= 0 and 1.2 or -1.2  -- NE trades (N), SE trades (S)
            wind_data.base_speed = 45 + math.random(-5, 5)
            wind_data.consistency = 0.8  -- Very consistent
            
        elseif abs_lat >= 30 and abs_lat < 60 then
            -- Westerlies (30-60°)
            wind_data.belt_name = "westerlies"  
            wind_data.direction = actual_lat >= 0 and -0.8 or 0.8  -- SW winds (N), NW winds (S)
            wind_data.base_speed = 65 + math.random(-10, 10)
            wind_data.consistency = 0.6  -- Moderately consistent
            
        elseif abs_lat >= 60 and abs_lat <= 90 then
            -- Polar easterlies (60-90°)
            wind_data.belt_name = "polar_easterlies"
            wind_data.direction = actual_lat >= 0 and 0.4 or -0.4   -- NE winds (N), SE winds (S)  
            wind_data.base_speed = 35 + math.random(-8, 8)
            wind_data.consistency = 0.5  -- Variable
            
        else
            -- Fallback
            wind_data.belt_name = "variable"
            wind_data.direction = 0.0
            wind_data.base_speed = 25
            wind_data.consistency = 0.3
        end
        
        return wind_data
    end)
    
    Game.log("info", string.format("Initialized %d wind belt zones", #self.wind_belts))
end

function WindPatterns:get_base_wind(latitude_index)
    -- Use functional approach to find wind belt
    local wind_belt = moses.detect(self.wind_belts, function(belt)
        return belt.index == latitude_index
    end)
    
    return wind_belt or {
        belt_name = "calm",
        direction = 0.0,
        base_speed = 20,
        consistency = 0.2
    }
end

function WindPatterns:apply_terrain_effects(wind_data, terrain_type, elevation)
    local terrain_key = string.lower(terrain_type or "plains")
    local effects = self.terrain_effects[terrain_key] or self.terrain_effects.plains
    
    -- Create modified wind data using functional transformation
    local modified_wind = moses.clone(wind_data)
    
    -- Apply speed multiplier
    modified_wind.speed = math.floor((wind_data.base_speed * effects.speed_multiplier) + 0.5)
    
    -- Apply direction change
    modified_wind.direction = wind_data.direction + effects.direction_change
    
    -- Apply turbulence
    local turbulence_factor = effects.turbulence
    local direction_noise = (math.random() - 0.5) * turbulence_factor
    local speed_noise = (math.random() - 0.5) * turbulence_factor * 10
    
    modified_wind.direction = modified_wind.direction + direction_noise
    modified_wind.speed = math.max(5, modified_wind.speed + speed_noise)
    
    -- Elevation effects (wind increases with altitude)
    local elevation_factor = 1.0 + (elevation / 1000.0) * 0.1
    modified_wind.speed = math.floor(modified_wind.speed * elevation_factor + 0.5)
    
    -- Apply consistency factor for final variability
    local consistency_noise = (1.0 - wind_data.consistency) * (math.random() - 0.5) * 20
    modified_wind.speed = math.max(5, math.min(150, modified_wind.speed + consistency_noise))
    
    return modified_wind
end

function WindPatterns:apply_seasonal_effects(wind_data, season, climate_zone)
    -- Season: 0.0 = spring, 0.25 = summer, 0.5 = autumn, 0.75 = winter
    local seasonal_strength = self.seasonal_variation[climate_zone] or self.seasonal_variation.temperate
    
    -- Calculate seasonal wind shift
    local seasonal_cycle = math.sin(season * 2 * math.pi) -- Full yearly cycle
    
    -- Modify wind based on season using functional approach
    local seasonal_wind = moses.extend(wind_data, {
        seasonal_direction_shift = seasonal_cycle * seasonal_strength * 0.5,
        seasonal_speed_change = seasonal_cycle * seasonal_strength * 15
    })
    
    seasonal_wind.direction = wind_data.direction + seasonal_wind.seasonal_direction_shift
    seasonal_wind.speed = math.max(5, wind_data.speed + seasonal_wind.seasonal_speed_change)
    
    return seasonal_wind
end

function WindPatterns:calculate_wind_strength_byte(wind_speed)
    -- Convert wind speed (km/h) to 0-255 range for storage
    return math.min(255, math.max(0, math.floor((wind_speed / 150.0) * 255 + 0.5)))
end

-- Initialize the wind patterns system
WindPatterns:init()

-- Event handler for wind pattern calculation
Game.register_event_callback("climate_wind_patterns", function(event_data)
    Game.log("debug", "Processing wind patterns for tile: " .. tostring(event_data.tile_id))
    
    local x = tonumber(event_data.x) or 0
    local y = tonumber(event_data.y) or 0
    local elevation = tonumber(event_data.elevation) or 0
    local terrain_type = event_data.terrain_type or "plains"
    local base_wind_strength = tonumber(event_data.wind_strength) or 50
    
    -- Calculate latitude index
    local world_height = 256
    local latitude_index = math.floor((y / world_height) * 180)
    latitude_index = moses.clamp(latitude_index, 0, 179)
    
    -- Get base wind for this latitude
    local base_wind = WindPatterns:get_base_wind(latitude_index)
    
    -- Apply terrain effects
    local terrain_wind = WindPatterns:apply_terrain_effects(base_wind, terrain_type, elevation)
    
    -- Convert to byte value for storage
    local final_wind_strength = WindPatterns:calculate_wind_strength_byte(terrain_wind.speed)
    
    -- Add some noise variation
    local noise_variation = math.random(-10, 10)
    final_wind_strength = moses.clamp(final_wind_strength + noise_variation, 0, 255)
    
    Game.log("debug", string.format("Wind pattern: %s, Speed: %d km/h (%d/255), Direction: %.2f rad, Terrain: %s",
                                   base_wind.belt_name, terrain_wind.speed, final_wind_strength, 
                                   terrain_wind.direction, terrain_type))
    
    return tostring(final_wind_strength)
end)

-- Event handler for detailed wind calculation (used by other systems)
Game.register_event_callback("detailed_wind_calculation", function(event_data)
    local x = tonumber(event_data.x) or 0
    local y = tonumber(event_data.y) or 0
    local elevation = tonumber(event_data.elevation) or 0
    local terrain_type = event_data.terrain_type or "plains"
    local season = tonumber(event_data.season) or 0.0
    local climate_zone = event_data.climate_zone or "temperate"
    
    local world_height = 256
    local latitude_index = math.floor((y / world_height) * 180)
    latitude_index = moses.clamp(latitude_index, 0, 179)
    
    -- Get and process wind data
    local base_wind = WindPatterns:get_base_wind(latitude_index)
    local terrain_wind = WindPatterns:apply_terrain_effects(base_wind, terrain_type, elevation)
    local seasonal_wind = WindPatterns:apply_seasonal_effects(terrain_wind, season, climate_zone)
    
    return {
        "wind_speed:" .. tostring(seasonal_wind.speed),
        "wind_direction:" .. string.format("%.3f", seasonal_wind.direction),
        "wind_belt:" .. base_wind.belt_name,
        "consistency:" .. string.format("%.2f", base_wind.consistency)
    }
end)

-- Utility functions for other climate systems
function get_wind_at_position(x, y, terrain_type, elevation)
    local world_height = 256
    local latitude_index = math.floor((y / world_height) * 180)
    latitude_index = moses.clamp(latitude_index, 0, 179)
    
    local base_wind = WindPatterns:get_base_wind(latitude_index)
    return WindPatterns:apply_terrain_effects(base_wind, terrain_type or "plains", elevation or 0)
end

function get_seasonal_wind(wind_data, season, climate_zone)
    return WindPatterns:apply_seasonal_effects(wind_data, season or 0.0, climate_zone or "temperate")
end

-- Export wind belt information for debugging
function get_wind_belt_info()
    return moses.map(WindPatterns.wind_belts, function(belt)
        return {
            latitude = belt.latitude,
            belt_name = belt.belt_name,
            base_speed = belt.base_speed,
            direction = belt.direction
        }
    end)
end

Game.log("info", "Wind patterns system loaded with " .. 
         moses.size(WindPatterns.terrain_effects) .. " terrain effects and " ..
         #WindPatterns.wind_belts .. " wind belts")
