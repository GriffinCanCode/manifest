-- Temperature Zones System
-- Uses penlight utilities for comprehensive temperature zone calculation
-- Handles latitude effects, elevation cooling, and continental/oceanic differences

local pl = require 'pl'
local utils = pl.utils
local tablex = pl.tablex
local seq = pl.seq

Game.log("info", "Loading temperature zones configuration...")

-- Temperature zone definitions
local TemperatureZones = pl.class()

function TemperatureZones:_init()
    -- Zone definitions with temperature ranges (Celsius)
    self.zones = {
        polar = { min = -50, max = -10, name = "Polar", continental_effect = -5 },
        arctic = { min = -10, max = 0, name = "Arctic", continental_effect = -8 },
        subarctic = { min = 0, max = 10, name = "Subarctic", continental_effect = -12 },
        temperate_cold = { min = 10, max = 15, name = "Cold Temperate", continental_effect = -15 },
        temperate = { min = 15, max = 25, name = "Temperate", continental_effect = -12 },
        subtropical = { min = 25, max = 30, name = "Subtropical", continental_effect = -8 },
        tropical = { min = 30, max = 40, name = "Tropical", continental_effect = -3 },
        equatorial = { min = 35, max = 45, name = "Equatorial", continental_effect = 0 }
    }
    
    -- Elevation lapse rate (°C per 100m)
    self.lapse_rate = 0.65
    
    -- Ocean moderating effects
    self.ocean_moderation = {
        coastal = 3.0,    -- Strong oceanic influence
        near_coast = 1.5, -- Moderate oceanic influence  
        inland = 0.0,     -- No oceanic influence
        continental = -2.0 -- Continental extreme effect
    }
end

function TemperatureZones:calculate_base_temperature(latitude, elevation, ocean_proximity)
    -- Calculate base temperature from latitude (simplified cosine distribution)
    local lat_rad = math.rad(math.abs(latitude))
    local equatorial_temp = 35
    local polar_temp = -30
    
    local base_temp = equatorial_temp - (equatorial_temp - polar_temp) * (lat_rad / (math.pi/2))
    
    -- Apply elevation cooling
    local elevation_cooling = (elevation / 100.0) * self.lapse_rate
    base_temp = base_temp - elevation_cooling
    
    -- Apply ocean moderation
    local ocean_effect = self:get_ocean_effect(ocean_proximity)
    base_temp = base_temp + ocean_effect
    
    return math.max(-50, math.min(45, base_temp))
end

function TemperatureZones:get_ocean_effect(proximity)
    if proximity > 0.8 then
        return self.ocean_moderation.coastal
    elseif proximity > 0.5 then
        return self.ocean_moderation.near_coast
    elseif proximity > 0.2 then
        return self.ocean_moderation.inland
    else
        return self.ocean_moderation.continental
    end
end

function TemperatureZones:determine_zone(temperature)
    -- Find the zone that contains this temperature
    for zone_name, zone_data in pairs(self.zones) do
        if temperature >= zone_data.min and temperature <= zone_data.max then
            return zone_name, zone_data
        end
    end
    
    -- Fallback to closest zone
    if temperature < -10 then
        return "polar", self.zones.polar
    else
        return "tropical", self.zones.tropical
    end
end

function TemperatureZones:apply_continental_effect(temperature, continentality)
    -- Continentality: 0.0 = oceanic, 1.0 = extremely continental
    local zone_name, zone_data = self:determine_zone(temperature)
    local continental_adjustment = zone_data.continental_effect * continentality
    
    return math.max(-50, math.min(45, temperature + continental_adjustment))
end

-- Create global temperature zones instance
local temp_zones = TemperatureZones()

-- Event handler for climate temperature zone calculation
Game.register_event_callback("climate_temperature_zones", function(event_data)
    Game.log("debug", "Processing temperature zones for tile: " .. tostring(event_data.tile_id))
    
    -- Extract parameters
    local x = tonumber(event_data.x) or 0
    local y = tonumber(event_data.y) or 0
    local elevation = tonumber(event_data.elevation) or 0
    local base_temperature = tonumber(event_data.base_temperature) or 15
    
    -- Calculate latitude from y coordinate (assuming world height of 256)
    local world_height = 256
    local latitude = ((y / world_height) - 0.5) * 180 -- -90 to +90 degrees
    
    -- Calculate ocean proximity (simplified - distance from edge)
    local world_width = 256
    local edge_distance_x = math.min(x / world_width, (world_width - x) / world_width)
    local edge_distance_y = math.min(y / world_height, (world_height - y) / world_height)
    local ocean_proximity = 1.0 - math.min(edge_distance_x, edge_distance_y)
    
    -- Calculate continentality (inverse of ocean proximity)
    local continentality = math.max(0, 1.0 - ocean_proximity * 2.0)
    
    -- Calculate refined temperature
    local refined_temp = temp_zones:calculate_base_temperature(latitude, elevation, ocean_proximity)
    
    -- Apply continental effects
    refined_temp = temp_zones:apply_continental_effect(refined_temp, continentality)
    
    -- Blend with base temperature from noise (70% calculated, 30% noise)
    local final_temp = refined_temp * 0.7 + base_temperature * 0.3
    
    -- Determine temperature zone
    local zone_name, zone_data = temp_zones:determine_zone(final_temp)
    
    Game.log("debug", string.format("Temperature zone: %s, Final temp: %.1f°C, Latitude: %.1f°, Ocean proximity: %.2f", 
                                   zone_name, final_temp, latitude, ocean_proximity))
    
    return tostring(math.floor(final_temp + 0.5))
end)

-- Utility functions for other climate scripts
function get_temperature_zone_info(temperature)
    return temp_zones:determine_zone(temperature)
end

function calculate_latitude_temperature(latitude)
    return temp_zones:calculate_base_temperature(latitude, 0, 0.5)
end

Game.log("info", "Temperature zones system loaded with " .. tablex.size(temp_zones.zones) .. " zones")
