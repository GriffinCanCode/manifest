-- Resource Distribution Rules
-- Advanced geological rules using lrexlib-pcre2 regex and perlin noise
-- Implements sophisticated distribution algorithms and geological analysis

local lrex = require 'rex_pcre2'
local perlin = require 'perlin'
local heap = require 'heap'
local moses = require 'moses'

Game.log("info", "Loading resource distribution rules...")

-- Initialize perlin noise with different seeds for each resource type
local noise_generators = {}

-- Geological analysis patterns using advanced regex
local GeologicalPatterns = {
    ore_vein_indicators = lrex.new([[
        (?x)  # Extended regex for readability
        (
            # Primary indicators
            (quartz|granite|metamorphic)_veins? |
            fault_zone |
            hydrothermal_activity |
            # Secondary indicators  
            mineral_staining |
            altered_rock |
            gossans
        )
    ]], "i"),
    
    oil_indicators = lrex.new([[
        (?x)
        (
            # Structural indicators
            sedimentary_basin |
            anticline |
            fault_trap |
            # Lithological indicators
            source_rocks? |
            reservoir_rocks? |
            seal_rocks?
        )
    ]], "i"),
    
    coal_seam_patterns = lrex.new([[
        (?x)
        (
            # Coal formation indicators
            ancient_swamp |
            carboniferous_deposits |
            plant_fossils |
            # Stratigraphic indicators
            cyclothems |
            coal_measures
        )
    ]], "i")
}

-- Distribution rule factory using behavior trees
local DistributionRules = {}

function DistributionRules:new(resource_type)
    local rule = {
        resource_type = resource_type,
        conditions = {},
        weights = {},
        noise_params = self:get_noise_parameters(resource_type)
    }
    setmetatable(rule, {__index = self})
    return rule
end

function DistributionRules:get_noise_parameters(resource_type)
    -- Different noise parameters for different resource types
    local params = {
        uranium = {scale = 0.01, octaves = 4, persistence = 0.5, lacunarity = 2.0},
        oil = {scale = 0.005, octaves = 6, persistence = 0.6, lacunarity = 1.8},
        iron = {scale = 0.02, octaves = 3, persistence = 0.4, lacunarity = 2.2},
        coal = {scale = 0.015, octaves = 5, persistence = 0.5, lacunarity = 2.0},
        gold = {scale = 0.03, octaves = 2, persistence = 0.3, lacunarity = 2.5}
    }
    
    return params[resource_type] or {scale = 0.01, octaves = 3, persistence = 0.5, lacunarity = 2.0}
end

function DistributionRules:add_geological_condition(condition_type, parameters)
    table.insert(self.conditions, {
        type = condition_type,
        params = parameters,
        weight = parameters.weight or 1.0
    })
end

function DistributionRules:evaluate_position(q, r, geological_context)
    local total_score = 0.0
    local total_weight = 0.0
    
    -- Evaluate each condition
    for _, condition in ipairs(self.conditions) do
        local score = self:evaluate_condition(condition, q, r, geological_context)
        total_score = total_score + (score * condition.weight)
        total_weight = total_weight + condition.weight
    end
    
    local base_score = total_weight > 0 and (total_score / total_weight) or 0.0
    
    -- Apply noise-based variation
    local noise_value = self:get_noise_value(q, r)
    local final_score = base_score * (0.7 + 0.3 * noise_value) -- 30% noise influence
    
    return math.max(0.0, math.min(1.0, final_score))
end

function DistributionRules:get_noise_value(q, r)
    if not noise_generators[self.resource_type] then
        -- Initialize noise generator for this resource type
        local seed = string.byte(self.resource_type, 1) or 42
        noise_generators[self.resource_type] = perlin:new(seed, 
            self.noise_params.octaves, 
            self.noise_params.persistence, 
            self.noise_params.lacunarity)
    end
    
    local x = q * self.noise_params.scale
    local y = r * self.noise_params.scale
    return (noise_generators[self.resource_type]:noise(x, y, 0) + 1) / 2 -- Normalize to 0-1
end

function DistributionRules:evaluate_condition(condition, q, r, context)
    local condition_type = condition.type
    local params = condition.params
    
    if condition_type == "elevation_range" then
        return self:evaluate_elevation_range(context.elevation, params.min, params.max)
    elseif condition_type == "tectonic_proximity" then
        return self:evaluate_tectonic_proximity(q, r, params)
    elseif condition_type == "geological_feature" then
        return self:evaluate_geological_feature(context, params)
    elseif condition_type == "climate_suitability" then
        return self:evaluate_climate_suitability(context, params)
    elseif condition_type == "ore_vein_potential" then
        return self:evaluate_ore_vein_potential(context, params)
    else
        Game.log("warning", "Unknown condition type: " .. condition_type)
        return 0.0
    end
end

function DistributionRules:evaluate_elevation_range(elevation, min_elev, max_elev)
    if elevation >= min_elev and elevation <= max_elev then
        -- Calculate optimal elevation curve
        local optimal = (min_elev + max_elev) / 2
        local distance = math.abs(elevation - optimal)
        local range = (max_elev - min_elev) / 2
        return 1.0 - (distance / range)
    else
        -- Penalty for being outside range
        local distance = math.min(math.abs(elevation - min_elev), math.abs(elevation - max_elev))
        return math.max(0.0, 1.0 - distance / 1000.0) -- 1km falloff
    end
end

function DistributionRules:evaluate_tectonic_proximity(q, r, params)
    local feature_distance = params.distance_to_feature or 10.0
    local optimal_distance = params.optimal_distance or 5.0
    
    -- Gaussian distribution around optimal distance
    local distance_score = math.exp(-0.5 * math.pow((feature_distance - optimal_distance) / (optimal_distance * 0.5), 2))
    return distance_score
end

function DistributionRules:evaluate_geological_feature(context, params)
    local required_features = params.required_features or {}
    local feature_score = 0.0
    
    -- Check for required geological features using regex patterns
    for _, feature in ipairs(required_features) do
        for _, tectonic_feature in ipairs(context.tectonic_features or {}) do
            if self:matches_geological_pattern(feature, tectonic_feature) then
                feature_score = feature_score + 1.0
                break
            end
        end
    end
    
    return math.min(1.0, feature_score / math.max(1, #required_features))
end

function DistributionRules:matches_geological_pattern(feature_pattern, tectonic_feature)
    -- Use appropriate regex pattern for different resource types
    local pattern = nil
    
    if string.find(feature_pattern, "vein") or string.find(feature_pattern, "ore") then
        pattern = GeologicalPatterns.ore_vein_indicators
    elseif string.find(feature_pattern, "oil") or string.find(feature_pattern, "hydrocarbon") then
        pattern = GeologicalPatterns.oil_indicators  
    elseif string.find(feature_pattern, "coal") then
        pattern = GeologicalPatterns.coal_seam_patterns
    end
    
    if pattern then
        return pattern:find(tectonic_feature) ~= nil
    else
        -- Fallback to simple string matching
        return string.find(string.lower(tectonic_feature), string.lower(feature_pattern)) ~= nil
    end
end

function DistributionRules:evaluate_climate_suitability(context, params)
    local score = 1.0
    
    -- Temperature suitability
    if params.temperature_range then
        local temp_score = self:range_suitability(
            context.temperature or 15, 
            params.temperature_range[1], 
            params.temperature_range[2]
        )
        score = score * temp_score
    end
    
    -- Rainfall suitability  
    if params.rainfall_range then
        local rain_score = self:range_suitability(
            context.rainfall or 500,
            params.rainfall_range[1],
            params.rainfall_range[2] 
        )
        score = score * rain_score
    end
    
    return score
end

function DistributionRules:range_suitability(value, min_val, max_val)
    if value >= min_val and value <= max_val then
        return 1.0
    elseif value < min_val then
        return math.max(0.0, 1.0 - (min_val - value) / min_val)
    else
        return math.max(0.0, 1.0 - (value - max_val) / max_val)
    end
end

function DistributionRules:evaluate_ore_vein_potential(context, params)
    -- Advanced ore vein evaluation using multiple indicators
    local vein_indicators = {
        "hydrothermal_activity",
        "fault_zones", 
        "igneous_intrusions",
        "metamorphic_aureole",
        "mineral_alteration"
    }
    
    local indicator_score = 0.0
    local indicator_count = 0
    
    for _, indicator in ipairs(vein_indicators) do
        for _, feature in ipairs(context.tectonic_features or {}) do
            if string.find(string.lower(feature), string.lower(indicator)) then
                indicator_score = indicator_score + 1.0
                indicator_count = indicator_count + 1
                break
            end
        end
    end
    
    -- Bonus for multiple indicators
    local base_score = indicator_count > 0 and (indicator_score / #vein_indicators) or 0.0
    local synergy_bonus = indicator_count > 2 and 0.3 or 0.0
    
    return math.min(1.0, base_score + synergy_bonus)
end

-- Priority queue for resource placement using heap
local ResourcePlacementQueue = {}

function ResourcePlacementQueue:new()
    local queue = {
        heap = heap.new(function(a, b) return a.priority > b.priority end),
        processed_positions = {}
    }
    setmetatable(queue, {__index = self})
    return queue
end

function ResourcePlacementQueue:add_candidate(q, r, resource_type, priority, context)
    local key = q .. "," .. r .. "," .. resource_type
    if not self.processed_positions[key] then
        self.heap:push({
            q = q,
            r = r, 
            resource_type = resource_type,
            priority = priority,
            context = context
        })
        self.processed_positions[key] = true
    end
end

function ResourcePlacementQueue:get_next_candidate()
    return self.heap:pop()
end

function ResourcePlacementQueue:is_empty()
    return self.heap:empty()
end

-- Main distribution rule evaluation function
function evaluate_resource_placement(q, r, resource_type, distribution_rules_json)
    local success, distribution_rules = pcall(function()
        return Game.json_decode(distribution_rules_json)
    end)
    
    if not success then
        Game.log("warning", "Failed to decode distribution rules for " .. resource_type)
        return false
    end
    
    -- Create rule evaluator
    local rule = DistributionRules:new(resource_type)
    
    -- Add conditions from rules
    local geological_rules = distribution_rules.geological_rules or {}
    
    if geological_rules.elevation_range then
        rule:add_geological_condition("elevation_range", {
            min = geological_rules.elevation_range[1],
            max = geological_rules.elevation_range[2],
            weight = 1.0
        })
    end
    
    if geological_rules.tectonic_features and #geological_rules.tectonic_features > 0 then
        rule:add_geological_condition("geological_feature", {
            required_features = geological_rules.tectonic_features,
            weight = 0.8
        })
    end
    
    -- Mock geological context (in real implementation, this would come from tectonic system)
    local geological_context = {
        elevation = q * 10 + r * 5, -- Placeholder elevation
        temperature = 15 + (q + r) * 0.1, -- Placeholder temperature
        rainfall = 500 + math.sin(q * 0.1) * 200, -- Placeholder rainfall
        tectonic_features = {"continental_crust", "stable_platform"}
    }
    
    -- Evaluate placement probability
    local placement_score = rule:evaluate_position(q, r, geological_context)
    
    -- Apply base rarity threshold
    local rarity_threshold = 0.3 -- Most resources need at least 30% score
    if resource_type == "uranium" or resource_type == "gold" then
        rarity_threshold = 0.7 -- Rare resources need higher scores
    end
    
    return placement_score >= rarity_threshold
end

-- Get compiled distribution rules for all resource types
function get_distribution_rules()
    local rules = {}
    
    -- This would be populated with actual compiled rules
    -- For now, returning empty structure that matches expected format
    for resource_type, _ in pairs(Game.Resources.resource_types or {}) do
        rules[resource_type] = {
            {
                resource_type = resource_type,
                weight = 1.0,
                conditions = {},
                placement_algorithm = "geological"
            }
        }
    end
    
    return Game.json_encode(rules)
end

-- Advanced vein generation using mathematical models
function generate_linear_vein(start_pos, direction_radians, length, width, resource_type)
    local positions = {}
    local noise_gen = noise_generators[resource_type]
    
    if not noise_gen then
        local seed = string.byte(resource_type, 1) or 42
        noise_gen = perlin:new(seed, 3, 0.5, 2.0)
        noise_generators[resource_type] = noise_gen
    end
    
    for i = 0, length - 1 do
        local progress = i / length
        
        -- Base position along vein
        local base_x = start_pos[1] + math.cos(direction_radians) * progress * length
        local base_y = start_pos[2] + math.sin(direction_radians) * progress * length
        
        -- Add noise-based wandering
        local noise_x = noise_gen:noise(progress * 5, 0, 0) * 2
        local noise_y = noise_gen:noise(0, progress * 5, 0) * 2
        
        -- Calculate width positions
        for w = 0, width - 1 do
            local width_offset = (w - width/2) / width
            local perp_x = -math.sin(direction_radians) * width_offset * width
            local perp_y = math.cos(direction_radians) * width_offset * width
            
            local final_x = math.floor(base_x + noise_x + perp_x)
            local final_y = math.floor(base_y + noise_y + perp_y)
            
            table.insert(positions, {final_x, final_y})
        end
    end
    
    return positions
end

-- Clustering analysis using balanced trees (lua-rbtree equivalent)
function analyze_resource_clustering(positions, cluster_radius)
    -- Simple clustering analysis - in production would use proper rbtree
    local clusters = {}
    local processed = {}
    
    for i, pos in ipairs(positions) do
        if not processed[i] then
            local cluster = {pos}
            processed[i] = true
            
            -- Find nearby positions
            for j = i + 1, #positions do
                if not processed[j] then
                    local distance = math.sqrt(
                        math.pow(pos[1] - positions[j][1], 2) + 
                        math.pow(pos[2] - positions[j][2], 2)
                    )
                    
                    if distance <= cluster_radius then
                        table.insert(cluster, positions[j])
                        processed[j] = true
                    end
                end
            end
            
            table.insert(clusters, cluster)
        end
    end
    
    return clusters
end

Game.log("info", "Resource distribution rules loaded with advanced geological analysis")

return {
    evaluate_resource_placement = evaluate_resource_placement,
    get_distribution_rules = get_distribution_rules,
    generate_linear_vein = generate_linear_vein,
    analyze_resource_clustering = analyze_resource_clustering,
    geological_patterns = GeologicalPatterns
}
