-- Rainfall Shadows System
-- Uses middleclass OOP for modeling orographic precipitation and rain shadow effects
-- Simulates windward/leeward precipitation patterns around mountains

local class = require 'middleclass'

Game.log("info", "Loading rainfall shadows system...")

-- Base terrain elevation analyzer
local TerrainAnalyzer = class('TerrainAnalyzer')

function TerrainAnalyzer:initialize()
    self.elevation_cache = {}
    self.wind_cache = {}
    self.precipitation_patterns = {}
end

function TerrainAnalyzer:get_elevation_profile(x, y, direction, distance)
    -- Get elevation profile in a given direction from a point
    -- Used to determine if there are mountains blocking moisture
    local profile = {}
    local dx = math.cos(direction) * distance / 10  -- 10 sample points
    local dy = math.sin(direction) * distance / 10
    
    for i = 0, 10 do
        local sample_x = x + (dx * i)
        local sample_y = y + (dy * i)
        
        -- Simple elevation model (would use real terrain data in full implementation)
        local elevation = self:estimate_elevation(sample_x, sample_y)
        table.insert(profile, {
            x = sample_x,
            y = sample_y,
            elevation = elevation,
            distance = distance * i / 10
        })
    end
    
    return profile
end

function TerrainAnalyzer:estimate_elevation(x, y)
    -- Cache elevation calculations
    local key = string.format("%.1f,%.1f", x, y)
    if self.elevation_cache[key] then
        return self.elevation_cache[key]
    end
    
    -- Simple fractal elevation (replace with actual terrain data)
    local elevation = 0
    local amplitude = 1000
    local frequency = 0.01
    
    for octave = 1, 4 do
        elevation = elevation + amplitude * math.sin(x * frequency) * math.cos(y * frequency)
        amplitude = amplitude * 0.5
        frequency = frequency * 2.1
    end
    
    elevation = math.max(0, elevation)
    self.elevation_cache[key] = elevation
    return elevation
end

-- Mountain range class for complex orographic effects
local MountainRange = class('MountainRange')

function MountainRange:initialize(center_x, center_y, width, height, orientation)
    self.center_x = center_x
    self.center_y = center_y
    self.width = width
    self.height = height
    self.orientation = orientation or 0  -- radians
    self.peak_elevation = height
    self.effective_height = height * 0.8  -- Height for precipitation effects
end

function MountainRange:get_blocking_effect(x, y, wind_direction)
    -- Calculate how much this mountain blocks wind/moisture
    local dx = x - self.center_x
    local dy = y - self.center_y
    local distance = math.sqrt(dx*dx + dy*dy)
    
    -- Check if point is within mountain influence
    if distance > self.width then
        return 0.0
    end
    
    -- Calculate position relative to mountain orientation
    local relative_angle = math.atan2(dy, dx) - self.orientation
    local cross_wind_distance = distance * math.abs(math.sin(relative_angle))
    local along_wind_distance = distance * math.cos(relative_angle)
    
    -- Determine windward vs leeward side
    local wind_relative_angle = wind_direction - self.orientation
    local is_windward = math.cos(wind_relative_angle) > 0
    
    -- Calculate blocking strength based on mountain geometry
    local height_factor = (self.width - distance) / self.width
    local elevation_at_point = self.peak_elevation * height_factor * height_factor
    
    return {
        blocking_strength = elevation_at_point / 2000.0,  -- Normalize to 0-1 range
        is_windward = is_windward,
        elevation = elevation_at_point,
        relative_distance = distance / self.width
    }
end

-- Precipitation calculator class
local PrecipitationCalculator = class('PrecipitationCalculator')

function PrecipitationCalculator:initialize()
    self.terrain_analyzer = TerrainAnalyzer:new()
    self.mountain_ranges = {}
    
    -- Create some example mountain ranges
    self:initialize_mountain_ranges()
    
    -- Orographic precipitation parameters
    self.orographic_params = {
        moisture_depletion_rate = 0.15,  -- How quickly air loses moisture upslope
        condensation_height = 800,       -- Elevation where condensation starts (m)
        max_orographic_bonus = 200,      -- Maximum additional rainfall (mm)
        rain_shadow_factor = 0.6,        -- How much rainfall is reduced in shadows
        distance_decay = 0.02            -- How shadow effects decay with distance
    }
end

function PrecipitationCalculator:initialize_mountain_ranges()
    -- Create realistic mountain distributions
    local world_width, world_height = 256, 256
    
    -- Add some mountain ranges in realistic patterns
    table.insert(self.mountain_ranges, MountainRange:new(64, 128, 40, 2500, math.pi/4))   -- NW-SE range
    table.insert(self.mountain_ranges, MountainRange:new(192, 64, 30, 1800, math.pi/2))   -- N-S range  
    table.insert(self.mountain_ranges, MountainRange:new(128, 200, 50, 3000, 0))          -- E-W range
    table.insert(self.mountain_ranges, MountainRange:new(200, 150, 25, 1200, -math.pi/3)) -- Smaller range
    
    Game.log("debug", string.format("Initialized %d mountain ranges for precipitation modeling", 
                                   #self.mountain_ranges))
end

function PrecipitationCalculator:calculate_orographic_precipitation(x, y, base_rainfall, wind_direction, wind_speed)
    local total_orographic_effect = 0
    local total_shadow_effect = 1.0
    local max_elevation_effect = 0
    
    -- Check effects from all mountain ranges
    for _, mountain in ipairs(self.mountain_ranges) do
        local effect = mountain:get_blocking_effect(x, y, wind_direction)
        
        if effect.blocking_strength > 0.1 then  -- Only consider significant effects
            
            if effect.is_windward then
                -- Windward side: Enhanced precipitation
                local elevation_factor = math.min(1.0, effect.elevation / 2000.0)
                local orographic_bonus = self.orographic_params.max_orographic_bonus * 
                                       elevation_factor * effect.blocking_strength
                
                -- Wind speed affects orographic lifting
                local wind_factor = math.min(1.5, wind_speed / 50.0)  -- Normalize wind speed
                orographic_bonus = orographic_bonus * wind_factor
                
                total_orographic_effect = total_orographic_effect + orographic_bonus
                max_elevation_effect = math.max(max_elevation_effect, elevation_factor)
                
            else
                -- Leeward side: Rain shadow effect
                local shadow_strength = effect.blocking_strength * self.orographic_params.rain_shadow_factor
                shadow_strength = shadow_strength * math.exp(-effect.relative_distance * 
                                                              self.orographic_params.distance_decay)
                
                total_shadow_effect = total_shadow_effect * (1.0 - shadow_strength)
            end
        end
    end
    
    -- Apply combined effects
    local enhanced_rainfall = base_rainfall + total_orographic_effect
    local final_rainfall = enhanced_rainfall * total_shadow_effect
    
    -- Ensure reasonable bounds
    final_rainfall = math.max(0, math.min(500, final_rainfall))
    
    return {
        final_rainfall = final_rainfall,
        orographic_bonus = total_orographic_effect,
        shadow_reduction = (1.0 - total_shadow_effect) * enhanced_rainfall,
        elevation_effect = max_elevation_effect
    }
end

function PrecipitationCalculator:get_detailed_precipitation_info(x, y, base_rainfall, wind_direction, wind_speed)
    local result = self:calculate_orographic_precipitation(x, y, base_rainfall, wind_direction, wind_speed)
    
    -- Add diagnostic information
    local nearby_mountains = {}
    for i, mountain in ipairs(self.mountain_ranges) do
        local effect = mountain:get_blocking_effect(x, y, wind_direction)
        if effect.blocking_strength > 0.01 then
            table.insert(nearby_mountains, {
                range_id = i,
                blocking_strength = effect.blocking_strength,
                is_windward = effect.is_windward,
                distance = effect.relative_distance
            })
        end
    end
    
    result.nearby_mountains = nearby_mountains
    return result
end

-- Create global precipitation calculator
local precipitation_calc = PrecipitationCalculator:new()

-- Event handler for rainfall shadow calculation
Game.register_event_callback("climate_rainfall_shadows", function(event_data)
    Game.log("debug", "Processing rainfall shadows for tile: " .. tostring(event_data.tile_id))
    
    local x = tonumber(event_data.x) or 0
    local y = tonumber(event_data.y) or 0
    local base_rainfall = tonumber(event_data.base_rainfall) or 100
    local wind_strength = tonumber(event_data.wind_strength) or 50
    local elevation = tonumber(event_data.elevation) or 0
    
    -- Get wind direction (simplified - would use actual wind data)
    local wind_direction = math.rad(270)  -- Default westerly winds
    local wind_speed = (wind_strength / 255.0) * 100  -- Convert to km/h
    
    -- Calculate orographic effects
    local precip_result = precipitation_calc:calculate_orographic_precipitation(
        x, y, base_rainfall, wind_direction, wind_speed)
    
    local final_rainfall = math.floor(precip_result.final_rainfall + 0.5)
    
    Game.log("debug", string.format("Rainfall: %d mm (base: %d, orographic: %+.1f, shadow: %.1f), Elevation effect: %.2f",
                                   final_rainfall, base_rainfall,
                                   precip_result.orographic_bonus, precip_result.shadow_reduction,
                                   precip_result.elevation_effect))
    
    return tostring(final_rainfall)
end)

-- Event handler for detailed orographic analysis
Game.register_event_callback("orographic_analysis", function(event_data)
    local x = tonumber(event_data.x) or 0
    local y = tonumber(event_data.y) or 0
    local base_rainfall = tonumber(event_data.base_rainfall) or 100
    local wind_direction = tonumber(event_data.wind_direction) or math.rad(270)
    local wind_speed = tonumber(event_data.wind_speed) or 50
    
    local detailed_info = precipitation_calc:get_detailed_precipitation_info(
        x, y, base_rainfall, wind_direction, wind_speed)
    
    local results = {
        "final_rainfall:" .. string.format("%.1f", detailed_info.final_rainfall),
        "orographic_bonus:" .. string.format("%.1f", detailed_info.orographic_bonus),
        "shadow_reduction:" .. string.format("%.1f", detailed_info.shadow_reduction),
        "nearby_mountains:" .. tostring(#detailed_info.nearby_mountains)
    }
    
    return results
end)

-- Utility functions for other climate systems
function get_orographic_rainfall(x, y, base_rainfall, wind_dir, wind_speed)
    local result = precipitation_calc:calculate_orographic_precipitation(x, y, base_rainfall, wind_dir, wind_speed)
    return result.final_rainfall
end

function get_mountain_shadow_effect(x, y, wind_direction)
    local shadow_effect = 1.0
    for _, mountain in ipairs(precipitation_calc.mountain_ranges) do
        local effect = mountain:get_blocking_effect(x, y, wind_direction)
        if not effect.is_windward and effect.blocking_strength > 0.1 then
            shadow_effect = shadow_effect * (1.0 - effect.blocking_strength * 0.6)
        end
    end
    return shadow_effect
end

function get_elevation_at_position(x, y)
    return precipitation_calc.terrain_analyzer:estimate_elevation(x, y)
end

Game.log("info", "Rainfall shadows system loaded with " .. 
         #precipitation_calc.mountain_ranges .. " mountain ranges and orographic modeling")
