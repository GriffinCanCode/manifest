-- Resource Scarcity Management System
-- Uses lua-rbtree balanced trees for efficient scarcity calculations
-- Implements advanced scarcity metrics and strategic reserve management

local moses = require 'moses'
local inspect = require 'inspect'

Game.log("info", "Loading resource scarcity management system...")

-- Scarcity calculation models
local ScarcityModels = {
    reserve_based = {
        name = "Reserve-Based Scarcity",
        description = "Calculates scarcity based on proven reserves vs consumption",
        calculate = function(reserves, consumption_rate, discovery_rate)
            if consumption_rate <= 0 then return 0.0 end
            
            local static_index = reserves / consumption_rate -- Years until exhaustion
            local dynamic_index = static_index / (1.0 + discovery_rate)
            
            -- Convert to 0-1 scarcity scale (higher = more scarce)
            local years_threshold = 50 -- 50 years considered "abundant"
            return math.max(0.0, math.min(1.0, 1.0 - (dynamic_index / years_threshold)))
        end
    },
    
    price_based = {
        name = "Price-Based Scarcity", 
        description = "Calculates scarcity based on price trends and volatility",
        calculate = function(price_history, baseline_price, volatility)
            if #price_history < 2 then return 0.0 end
            
            local recent_price = price_history[#price_history]
            local price_trend = calculate_price_trend(price_history)
            
            -- Price elevation above baseline
            local price_elevation = (recent_price - baseline_price) / baseline_price
            
            -- Combine price level and trend
            local price_scarcity = math.max(0.0, price_elevation) * (1.0 + price_trend * 0.5)
            
            -- Volatility amplifies scarcity signal
            local volatility_factor = 1.0 + volatility * 0.3
            
            return math.max(0.0, math.min(1.0, price_scarcity * volatility_factor))
        end
    },
    
    accessibility_based = {
        name = "Accessibility-Based Scarcity",
        description = "Calculates scarcity based on extraction difficulty and accessibility",
        calculate = function(extraction_difficulty, infrastructure_quality, political_stability)
            -- Higher difficulty and lower quality/stability = higher scarcity
            local difficulty_factor = extraction_difficulty or 0.5
            local infrastructure_factor = 1.0 - (infrastructure_quality or 0.8)
            local political_factor = 1.0 - (political_stability or 0.8)
            
            local accessibility_scarcity = (difficulty_factor + infrastructure_factor + political_factor) / 3.0
            
            return math.max(0.0, math.min(1.0, accessibility_scarcity))
        end
    },
    
    substitution_based = {
        name = "Substitution-Based Scarcity",
        description = "Calculates scarcity considering available substitutes",
        calculate = function(base_scarcity, substitutes, substitute_adoption_rate)
            local substitute_relief = 0.0
            
            if substitutes and #substitutes > 0 then
                for _, substitute in ipairs(substitutes) do
                    local effectiveness = substitute.effectiveness or 0.5
                    local availability = substitute.availability or 0.5
                    local adoption = substitute_adoption_rate or 0.1
                    
                    substitute_relief = substitute_relief + (effectiveness * availability * adoption)
                end
                
                -- Diminishing returns on substitutes
                substitute_relief = 1.0 - math.exp(-substitute_relief)
            end
            
            -- Reduce base scarcity by substitute relief
            return base_scarcity * (1.0 - substitute_relief * 0.7) -- Max 70% relief
        end
    }
}

-- Scarcity thresholds and classifications
local ScarcityThresholds = {
    abundant = {min = 0.0, max = 0.2, description = "Abundant supply, stable prices"},
    adequate = {min = 0.2, max = 0.4, description = "Adequate supply, minor price pressure"},
    constrained = {min = 0.4, max = 0.6, description = "Constrained supply, noticeable price increases"},
    scarce = {min = 0.6, max = 0.8, description = "Scarce supply, significant price volatility"},
    critical = {min = 0.8, max = 1.0, description = "Critical shortage, extreme price volatility"}
}

-- Regional scarcity modifiers
local RegionalModifiers = {
    transportation_costs = function(distance_to_source, transportation_quality)
        local distance_penalty = distance_to_source * 0.001 -- 0.1% per distance unit
        local transport_bonus = (transportation_quality or 0.5) * 0.2 -- Up to 20% bonus
        return math.max(0.0, distance_penalty - transport_bonus)
    end,
    
    political_instability = function(stability_index, trade_relations)
        local instability_penalty = (1.0 - (stability_index or 0.8)) * 0.5 -- Up to 50% penalty
        local relations_bonus = (trade_relations or 0.5) * 0.3 -- Up to 30% bonus
        return math.max(0.0, instability_penalty - relations_bonus)
    end,
    
    economic_development = function(gdp_per_capita, infrastructure_index)
        -- Wealthier regions can better cope with scarcity
        local wealth_factor = math.min(1.0, (gdp_per_capita or 10000) / 50000) -- Normalize to $50k
        local infrastructure_factor = infrastructure_index or 0.5
        
        local resilience = (wealth_factor + infrastructure_factor) / 2.0
        return -resilience * 0.3 -- Up to 30% scarcity reduction
    end,
    
    strategic_reserves = function(reserve_level, consumption_rate)
        if consumption_rate <= 0 then return -0.5 end -- Perfect buffer
        
        local months_of_reserves = (reserve_level or 0) / consumption_rate * 12
        local buffer_factor = math.min(1.0, months_of_reserves / 6) -- 6 months ideal
        
        return -buffer_factor * 0.4 -- Up to 40% scarcity reduction
    end
}

-- Balanced tree implementation for efficient scarcity tracking
-- (Simplified version - in production would use proper red-black tree)
local ScarcityTree = {}

function ScarcityTree:new()
    local tree = {
        nodes = {},
        sorted_keys = {}
    }
    setmetatable(tree, {__index = self})
    return tree
end

function ScarcityTree:insert(resource_type, scarcity_value, metadata)
    self.nodes[resource_type] = {
        scarcity = scarcity_value,
        metadata = metadata or {},
        last_updated = os.time()
    }
    
    -- Rebuild sorted keys (simplified - real RB tree would be more efficient)
    self:rebuild_sorted_keys()
end

function ScarcityTree:get(resource_type)
    return self.nodes[resource_type]
end

function ScarcityTree:get_range(min_scarcity, max_scarcity)
    local results = {}
    
    for resource_type, node in pairs(self.nodes) do
        if node.scarcity >= min_scarcity and node.scarcity <= max_scarcity then
            table.insert(results, {
                resource_type = resource_type,
                scarcity = node.scarcity,
                metadata = node.metadata
            })
        end
    end
    
    -- Sort by scarcity level
    table.sort(results, function(a, b) return a.scarcity > b.scarcity end)
    return results
end

function ScarcityTree:get_most_scarce(count)
    local all_resources = {}
    
    for resource_type, node in pairs(self.nodes) do
        table.insert(all_resources, {
            resource_type = resource_type,
            scarcity = node.scarcity,
            metadata = node.metadata
        })
    end
    
    -- Sort by scarcity (descending)
    table.sort(all_resources, function(a, b) return a.scarcity > b.scarcity end)
    
    return moses.slice(all_resources, 1, count or 10)
end

function ScarcityTree:rebuild_sorted_keys()
    self.sorted_keys = {}
    for resource_type, _ in pairs(self.nodes) do
        table.insert(self.sorted_keys, resource_type)
    end
    
    table.sort(self.sorted_keys, function(a, b) 
        return self.nodes[a].scarcity > self.nodes[b].scarcity
    end)
end

-- Global scarcity tracking
local GlobalScarcityTracker = ScarcityTree:new()

-- Helper function to calculate price trend
function calculate_price_trend(price_history)
    if #price_history < 3 then return 0.0 end
    
    local trend_sum = 0.0
    local trend_count = 0
    
    for i = 2, #price_history do
        local change = (price_history[i] - price_history[i-1]) / price_history[i-1]
        trend_sum = trend_sum + change
        trend_count = trend_count + 1
    end
    
    return trend_count > 0 and (trend_sum / trend_count) or 0.0
end

-- Main scarcity calculation function
function calculate_scarcity_index(resource_type, reserves, extraction_rate, active_deposits)
    local base_scarcity = 0.0
    
    -- Reserve-based scarcity (primary model)
    if reserves and extraction_rate then
        local discovery_rate = calculate_discovery_rate_decline(resource_type, active_deposits)
        base_scarcity = ScarcityModels.reserve_based.calculate(reserves, extraction_rate, discovery_rate)
    end
    
    -- Get additional context for enhanced calculation
    local price_history = get_price_history(resource_type) or {}
    local baseline_price = get_baseline_price(resource_type) or 1.0
    local volatility = calculate_price_volatility(price_history) or 0.0
    
    -- Price-based scarcity modifier
    local price_scarcity = ScarcityModels.price_based.calculate(price_history, baseline_price, volatility)
    
    -- Extraction accessibility
    local extraction_difficulty = get_extraction_difficulty(resource_type) or 0.5
    local infrastructure_quality = get_global_infrastructure_quality() or 0.8
    local political_stability = get_global_political_stability() or 0.8
    
    local accessibility_scarcity = ScarcityModels.accessibility_based.calculate(
        extraction_difficulty, infrastructure_quality, political_stability
    )
    
    -- Combine scarcity metrics with weights
    local weighted_scarcity = (base_scarcity * 0.5) + (price_scarcity * 0.3) + (accessibility_scarcity * 0.2)
    
    -- Apply substitution relief
    local substitutes = get_resource_substitutes(resource_type) or {}
    local adoption_rate = get_substitute_adoption_rate(resource_type) or 0.1
    
    local final_scarcity = ScarcityModels.substitution_based.calculate(
        weighted_scarcity, substitutes, adoption_rate
    )
    
    -- Update global tracker
    GlobalScarcityTracker:insert(resource_type, final_scarcity, {
        reserves = reserves,
        extraction_rate = extraction_rate,
        active_deposits = active_deposits,
        price_trend = calculate_price_trend(price_history),
        accessibility_difficulty = extraction_difficulty
    })
    
    return final_scarcity
end

-- Calculate discovery rate decline based on resource maturity
function calculate_discovery_rate_decline(resource_type, active_deposits)
    local maturity_factors = {
        uranium = 0.05,
        oil = 0.08,
        coal = 0.03,
        iron = 0.02,
        gold = 0.04
    }
    
    local base_decline = maturity_factors[resource_type] or 0.03
    local deposit_saturation_factor = math.min(1.0, (active_deposits or 10) / 50) -- Saturated at 50 deposits
    
    return base_decline * (1.0 + deposit_saturation_factor)
end

-- Regional scarcity calculation
function calculate_regional_scarcity(resource_type, base_scarcity, region_data)
    local regional_scarcity = base_scarcity
    
    -- Apply regional modifiers
    for modifier_name, modifier_func in pairs(RegionalModifiers) do
        local modifier_data = region_data[modifier_name] or {}
        local modification = 0.0
        
        if modifier_name == "transportation_costs" then
            modification = modifier_func(modifier_data.distance, modifier_data.quality)
        elseif modifier_name == "political_instability" then
            modification = modifier_func(modifier_data.stability, modifier_data.relations)
        elseif modifier_name == "economic_development" then
            modification = modifier_func(modifier_data.gdp_per_capita, modifier_data.infrastructure)
        elseif modifier_name == "strategic_reserves" then
            modification = modifier_func(modifier_data.reserve_level, modifier_data.consumption)
        end
        
        regional_scarcity = regional_scarcity + modification
    end
    
    return math.max(0.0, math.min(1.0, regional_scarcity))
end

-- Scarcity classification
function classify_scarcity(scarcity_index)
    for classification, threshold in pairs(ScarcityThresholds) do
        if scarcity_index >= threshold.min and scarcity_index < threshold.max then
            return {
                level = classification,
                description = threshold.description,
                index = scarcity_index,
                severity = (scarcity_index - threshold.min) / (threshold.max - threshold.min)
            }
        end
    end
    
    return {
        level = "critical",
        description = ScarcityThresholds.critical.description,
        index = scarcity_index,
        severity = 1.0
    }
end

-- Strategic recommendations based on scarcity
function generate_scarcity_recommendations(resource_type, scarcity_data)
    local recommendations = {}
    local scarcity_level = classify_scarcity(scarcity_data.scarcity)
    
    if scarcity_level.index > 0.6 then -- Scarce or critical
        table.insert(recommendations, {
            type = "strategic_stockpiling",
            priority = "high",
            description = "Build strategic reserves of " .. resource_type,
            target_reserve_months = math.min(24, scarcity_level.index * 30)
        })
        
        table.insert(recommendations, {
            type = "substitute_development",
            priority = "high", 
            description = "Accelerate development of substitutes for " .. resource_type,
            research_focus = get_priority_substitutes(resource_type)
        })
        
        table.insert(recommendations, {
            type = "efficiency_improvement",
            priority = "medium",
            description = "Improve extraction and usage efficiency",
            efficiency_target = 1.0 + scarcity_level.index * 0.5 -- Up to 50% improvement
        })
    end
    
    if scarcity_level.index > 0.4 then -- Constrained or worse
        table.insert(recommendations, {
            type = "diversification",
            priority = "medium",
            description = "Diversify supply sources for " .. resource_type,
            target_suppliers = math.min(5, math.ceil(scarcity_level.index * 8))
        })
        
        table.insert(recommendations, {
            type = "recycling_enhancement",
            priority = "medium",
            description = "Enhance recycling capabilities",
            recycling_target = scarcity_level.index * 0.8 -- Up to 80% recycling
        })
    end
    
    return recommendations
end

-- Utility functions (these would integrate with actual game systems)
function get_price_history(resource_type)
    -- Would retrieve from market system
    return {} -- Placeholder
end

function get_baseline_price(resource_type)
    local baseline_prices = {
        uranium = 100,
        oil = 50,
        coal = 15,
        iron = 20,
        gold = 200
    }
    return baseline_prices[resource_type] or 10
end

function calculate_price_volatility(price_history)
    if #price_history < 2 then return 0.0 end
    
    local returns = {}
    for i = 2, #price_history do
        local return_val = (price_history[i] - price_history[i-1]) / price_history[i-1]
        table.insert(returns, return_val)
    end
    
    local mean_return = moses.reduce(returns, function(sum, ret) return sum + ret end, 0) / #returns
    local variance = moses.reduce(returns, function(sum, ret) 
        return sum + math.pow(ret - mean_return, 2) 
    end, 0) / #returns
    
    return math.sqrt(variance) -- Standard deviation
end

function get_extraction_difficulty(resource_type)
    local difficulties = {
        uranium = 0.8,
        oil = 0.6,
        coal = 0.4,
        iron = 0.5,
        gold = 0.9
    }
    return difficulties[resource_type] or 0.5
end

function get_global_infrastructure_quality()
    return 0.7 -- Global average placeholder
end

function get_global_political_stability()
    return 0.75 -- Global average placeholder
end

function get_substitute_adoption_rate(resource_type)
    local adoption_rates = {
        oil = 0.15, -- Moderate adoption of renewables
        coal = 0.20, -- Higher adoption of alternatives
        iron = 0.10, -- Lower adoption of substitutes
        uranium = 0.05 -- Very low adoption of alternatives
    }
    return adoption_rates[resource_type] or 0.08
end

function get_priority_substitutes(resource_type)
    local priority_substitutes = {
        oil = {"renewable_energy", "nuclear_power", "biofuels"},
        coal = {"natural_gas", "renewable_energy", "nuclear_power"},
        iron = {"aluminum", "composite_materials", "recycled_steel"},
        uranium = {"thorium", "fusion", "renewables_with_storage"}
    }
    return priority_substitutes[resource_type] or {}
end

-- Get current scarcity status for all resources
function get_global_scarcity_status()
    local status = {
        most_scarce = GlobalScarcityTracker:get_most_scarce(5),
        by_category = {},
        average_scarcity = 0.0,
        critical_resources = GlobalScarcityTracker:get_range(0.8, 1.0)
    }
    
    -- Calculate average scarcity
    local total_scarcity = 0.0
    local resource_count = 0
    
    for _, node in pairs(GlobalScarcityTracker.nodes) do
        total_scarcity = total_scarcity + node.scarcity
        resource_count = resource_count + 1
    end
    
    if resource_count > 0 then
        status.average_scarcity = total_scarcity / resource_count
    end
    
    return status
end

Game.log("info", "Resource scarcity management system initialized")

return {
    calculate_scarcity_index = calculate_scarcity_index,
    calculate_regional_scarcity = calculate_regional_scarcity,
    classify_scarcity = classify_scarcity,
    generate_scarcity_recommendations = generate_scarcity_recommendations,
    get_global_scarcity_status = get_global_scarcity_status,
    scarcity_models = ScarcityModels,
    scarcity_tracker = GlobalScarcityTracker
}
