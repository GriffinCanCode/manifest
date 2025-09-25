-- Biome Determination System
-- Uses middleclass OOP and penlight utilities for biome assignment
-- Integrates climate, terrain, and elevation data for realistic biome placement

local class = require 'middleclass'
local pl = require 'pl'
local tablex = pl.tablex

Game.log("info", "Loading biome determination system...")

-- Biome classifier using object-oriented design
local BiomeClassifier = class('BiomeClassifier')

function BiomeClassifier:initialize()
    -- Biome classification rules based on Whittaker biome model
    self.biome_rules = {
        -- Tropical biomes
        {
            name = "tropical_rainforest",
            conditions = {
                temperature = {min = 20, max = 35},
                rainfall = {min = 200, max = 500},
                humidity = {min = 60, max = 100},
                elevation = {min = 0, max = 1000}
            },
            terrain_preferences = {"jungle", "forest", "hills"},
            suitability_multipliers = {jungle = 1.5, forest = 1.2, hills = 0.9},
            confidence_base = 0.9
        },
        
        {
            name = "tropical_grassland",
            conditions = {
                temperature = {min = 18, max = 32},
                rainfall = {min = 60, max = 200},
                humidity = {min = 40, max = 80},
                elevation = {min = 0, max = 800}
            },
            terrain_preferences = {"grassland", "plains", "savanna"},
            suitability_multipliers = {grassland = 1.3, plains = 1.2, savanna = 1.4},
            confidence_base = 0.8
        },
        
        -- Temperate biomes
        {
            name = "temperate_forest",
            conditions = {
                temperature = {min = 5, max = 25},
                rainfall = {min = 100, max = 300},
                humidity = {min = 50, max = 85},
                elevation = {min = 0, max = 1500}
            },
            terrain_preferences = {"forest", "hills", "mountains"},
            suitability_multipliers = {forest = 1.4, hills = 1.1, mountains = 0.8},
            confidence_base = 0.85
        },
        
        {
            name = "temperate_grassland",
            conditions = {
                temperature = {min = 0, max = 25},
                rainfall = {min = 50, max = 250},
                humidity = {min = 30, max = 70},
                elevation = {min = 0, max = 1000}
            },
            terrain_preferences = {"grassland", "plains"},
            suitability_multipliers = {grassland = 1.5, plains = 1.3},
            confidence_base = 0.9
        },
        
        -- Arid biomes
        {
            name = "desert",
            conditions = {
                temperature = {min = 15, max = 45},
                rainfall = {min = 0, max = 50},
                humidity = {min = 0, max = 30},
                elevation = {min = 0, max = 2000}
            },
            terrain_preferences = {"desert", "plains", "hills"},
            suitability_multipliers = {desert = 1.6, plains = 1.0, hills = 0.9},
            confidence_base = 0.95
        },
        
        {
            name = "steppe",
            conditions = {
                temperature = {min = 5, max = 30},
                rainfall = {min = 25, max = 100},
                humidity = {min = 20, max = 50},
                elevation = {min = 0, max = 1500}
            },
            terrain_preferences = {"grassland", "plains", "hills"},
            suitability_multipliers = {grassland = 1.2, plains = 1.3, hills = 1.0},
            confidence_base = 0.8
        },
        
        -- Cold biomes
        {
            name = "taiga",
            conditions = {
                temperature = {min = -10, max = 15},
                rainfall = {min = 100, max = 250},
                humidity = {min = 40, max = 80},
                elevation = {min = 0, max = 2000}
            },
            terrain_preferences = {"forest", "hills", "mountains"},
            suitability_multipliers = {forest = 1.3, hills = 1.1, mountains = 0.9},
            confidence_base = 0.85
        },
        
        {
            name = "tundra",
            conditions = {
                temperature = {min = -20, max = 5},
                rainfall = {min = 20, max = 150},
                humidity = {min = 30, max = 70},
                elevation = {min = 0, max = 1000}
            },
            terrain_preferences = {"tundra", "plains", "hills"},
            suitability_multipliers = {tundra = 1.4, plains = 1.1, hills = 1.0},
            confidence_base = 0.9
        },
        
        -- Mountain biomes
        {
            name = "alpine",
            conditions = {
                temperature = {min = -15, max = 10},
                rainfall = {min = 100, max = 300},
                humidity = {min = 40, max = 90},
                elevation = {min = 1500, max = 5000}
            },
            terrain_preferences = {"mountain", "hills"},
            suitability_multipliers = {mountain = 1.5, hills = 1.2},
            confidence_base = 0.95
        },
        
        -- Mediterranean 
        {
            name = "mediterranean",
            conditions = {
                temperature = {min = 10, max = 28},
                rainfall = {min = 80, max = 200},
                humidity = {min = 35, max = 65},
                elevation = {min = 0, max = 800}
            },
            terrain_preferences = {"hills", "coast", "plains"},
            suitability_multipliers = {hills = 1.3, coast = 1.4, plains = 1.1},
            confidence_base = 0.8
        }
    }
    
    -- Special biome conditions that override normal rules
    self.special_conditions = {
        polar_desert = function(climate_data)
            return climate_data.temperature < -15 and climate_data.rainfall < 25
        end,
        
        tropical_dry_forest = function(climate_data)
            return climate_data.temperature > 22 and 
                   climate_data.rainfall > 100 and climate_data.rainfall < 200 and
                   climate_data.climate_zone:find("tropical")
        end,
        
        cloud_forest = function(climate_data)
            return climate_data.elevation > 800 and climate_data.humidity > 80 and
                   climate_data.rainfall > 150 and
                   climate_data.terrain_type:find("mountain")
        end,
        
        mangrove = function(climate_data)
            return climate_data.terrain_type:find("coast") and
                   climate_data.temperature > 20 and
                   climate_data.humidity > 70
        end
    }
end

function BiomeClassifier:check_condition(value, condition)
    return value >= condition.min and value <= condition.max
end

function BiomeClassifier:calculate_suitability(climate_data, biome_rule)
    local suitability = 1.0
    local conditions_met = 0
    local total_conditions = 0
    
    -- Check temperature
    if biome_rule.conditions.temperature then
        total_conditions = total_conditions + 1
        if self:check_condition(climate_data.temperature, biome_rule.conditions.temperature) then
            conditions_met = conditions_met + 1
        else
            -- Apply penalty for temperature mismatch
            local temp_diff = math.min(
                math.abs(climate_data.temperature - biome_rule.conditions.temperature.min),
                math.abs(climate_data.temperature - biome_rule.conditions.temperature.max)
            )
            suitability = suitability * math.max(0.1, 1.0 - (temp_diff * 0.05))
        end
    end
    
    -- Check rainfall
    if biome_rule.conditions.rainfall then
        total_conditions = total_conditions + 1
        if self:check_condition(climate_data.rainfall, biome_rule.conditions.rainfall) then
            conditions_met = conditions_met + 1
        else
            local rain_diff = math.min(
                math.abs(climate_data.rainfall - biome_rule.conditions.rainfall.min),
                math.abs(climate_data.rainfall - biome_rule.conditions.rainfall.max)
            )
            suitability = suitability * math.max(0.1, 1.0 - (rain_diff * 0.002))
        end
    end
    
    -- Check humidity
    if biome_rule.conditions.humidity then
        total_conditions = total_conditions + 1
        if self:check_condition(climate_data.humidity, biome_rule.conditions.humidity) then
            conditions_met = conditions_met + 1
        else
            local humidity_diff = math.min(
                math.abs(climate_data.humidity - biome_rule.conditions.humidity.min),
                math.abs(climate_data.humidity - biome_rule.conditions.humidity.max)
            )
            suitability = suitability * math.max(0.2, 1.0 - (humidity_diff * 0.01))
        end
    end
    
    -- Check elevation
    if biome_rule.conditions.elevation then
        total_conditions = total_conditions + 1
        if self:check_condition(climate_data.elevation, biome_rule.conditions.elevation) then
            conditions_met = conditions_met + 1
        else
            local elev_diff = math.min(
                math.abs(climate_data.elevation - biome_rule.conditions.elevation.min),
                math.abs(climate_data.elevation - biome_rule.conditions.elevation.max)
            )
            suitability = suitability * math.max(0.1, 1.0 - (elev_diff * 0.0005))
        end
    end
    
    -- Apply terrain preference multiplier
    local terrain_multiplier = biome_rule.suitability_multipliers[climate_data.terrain_type] or 0.7
    suitability = suitability * terrain_multiplier
    
    -- Calculate confidence based on how many conditions were perfectly met
    local condition_ratio = total_conditions > 0 and (conditions_met / total_conditions) or 0
    local confidence = biome_rule.confidence_base * (0.5 + 0.5 * condition_ratio) * suitability
    
    return suitability, confidence
end

function BiomeClassifier:determine_biome(climate_data)
    -- First check for special biomes
    for special_name, condition_func in pairs(self.special_conditions) do
        if condition_func(climate_data) then
            return {
                biome_type = special_name,
                confidence = 0.95,
                suitability = 0.9,
                reasoning = "Special biome condition met: " .. special_name
            }
        end
    end
    
    -- Evaluate all standard biome rules
    local best_biome = nil
    local best_score = 0
    local candidates = {}
    
    for _, biome_rule in ipairs(self.biome_rules) do
        local suitability, confidence = self:calculate_suitability(climate_data, biome_rule)
        local combined_score = suitability * confidence
        
        if combined_score > 0.1 then -- Only consider viable biomes
            table.insert(candidates, {
                name = biome_rule.name,
                suitability = suitability,
                confidence = confidence,
                score = combined_score
            })
            
            if combined_score > best_score then
                best_score = combined_score
                best_biome = {
                    biome_type = biome_rule.name,
                    confidence = confidence,
                    suitability = suitability,
                    reasoning = string.format("Best match with score %.3f", combined_score)
                }
            end
        end
    end
    
    -- Fallback if no good match
    if not best_biome or best_biome.confidence < 0.3 then
        local fallback_biome = self:determine_fallback_biome(climate_data)
        return {
            biome_type = fallback_biome,
            confidence = 0.4,
            suitability = 0.5,
            reasoning = "Fallback biome assignment"
        }
    end
    
    return best_biome
end

function BiomeClassifier:determine_fallback_biome(climate_data)
    -- Simple fallback logic based on temperature and rainfall
    if climate_data.temperature < -10 then
        return climate_data.rainfall < 50 and "polar_desert" or "tundra"
    elseif climate_data.temperature < 5 then
        return climate_data.rainfall > 100 and "taiga" or "steppe"
    elseif climate_data.temperature < 20 then
        return climate_data.rainfall > 150 and "temperate_forest" or "temperate_grassland"
    else
        return climate_data.rainfall < 100 and "desert" or "tropical_grassland"
    end
end

-- Create global biome classifier
local biome_classifier = BiomeClassifier:new()

-- Event handler for biome determination
Game.register_event_callback("biome_determination", function(event_data)
    Game.log("debug", "Determining biome for tile: " .. tostring(event_data.tile_id))
    
    -- Extract climate data
    local climate_data = {
        temperature = tonumber(event_data.temperature) or 15,
        rainfall = tonumber(event_data.rainfall) or 100,
        humidity = tonumber(event_data.humidity) or 50,
        elevation = tonumber(event_data.elevation) or 0,
        terrain_type = event_data.terrain_type or "plains",
        climate_zone = event_data.climate_zone or "temperate"
    }
    
    -- Determine best biome
    local result = biome_classifier:determine_biome(climate_data)
    
    Game.log("debug", string.format("Biome determination: %s (confidence: %.2f, suitability: %.2f) - %s",
                                   result.biome_type, result.confidence, result.suitability, result.reasoning))
    
    -- Return results in expected format
    return {
        "biome_type:" .. result.biome_type,
        "confidence:" .. string.format("%.3f", result.confidence),
        "suitability:" .. string.format("%.3f", result.suitability)
    }
end)

-- Event handler for special biome checks
Game.register_event_callback("special_biomes", function(event_data)
    local climate_data = {
        temperature = tonumber(event_data.temperature) or 15,
        rainfall = tonumber(event_data.rainfall) or 100,
        humidity = tonumber(event_data.humidity) or 50,
        elevation = tonumber(event_data.elevation) or 0,
        terrain_type = event_data.terrain_type or "plains",
        climate_zone = event_data.climate_zone or "temperate"
    }
    
    -- Check for special biomes
    for special_name, condition_func in pairs(biome_classifier.special_conditions) do
        if condition_func(climate_data) then
            Game.log("debug", "Special biome detected: " .. special_name)
            return {"special:" .. special_name}
        end
    end
    
    return {}
end)

-- Utility function for other scripts
function get_biome_suitability(climate_data, biome_name)
    for _, rule in ipairs(biome_classifier.biome_rules) do
        if rule.name == biome_name then
            local suitability, confidence = biome_classifier:calculate_suitability(climate_data, rule)
            return suitability, confidence
        end
    end
    return 0, 0
end

-- Helper function to check if biome rules are available
function biome_rules_available()
    return tablex.size(biome_classifier.biome_rules) > 0
end

Game.log("info", "Biome determination system loaded with " .. #biome_classifier.biome_rules .. " biome rules and " .. 
         tablex.size(biome_classifier.special_conditions) .. " special conditions")
