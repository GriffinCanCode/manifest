-- Resource Depletion and Exhaustion System
-- Uses deque queues for efficient event processing and serpent for state serialization
-- Implements sophisticated depletion mechanics with environmental and economic effects

local deque = require 'deque'
local serpent = require 'serpent'
local moses = require 'moses'

Game.log("info", "Loading resource depletion system...")

-- Depletion models for different resource types
local DepletionModels = {
    exponential = {
        name = "Exponential Depletion",
        description = "Extraction becomes increasingly difficult as reserves diminish",
        efficiency_curve = function(depletion_ratio)
            return math.exp(-depletion_ratio * 3.0) -- Exponential decay
        end,
        cost_curve = function(depletion_ratio)
            return 1.0 + depletion_ratio * 2.0 -- Exponential cost increase
        end
    },
    
    linear = {
        name = "Linear Depletion",
        description = "Steady decline in extraction efficiency",
        efficiency_curve = function(depletion_ratio)
            return 1.0 - depletion_ratio * 0.8 -- Linear decline to 20% efficiency
        end,
        cost_curve = function(depletion_ratio)
            return 1.0 + depletion_ratio * 1.5 -- Linear cost increase
        end
    },
    
    step = {
        name = "Step Function Depletion",
        description = "Sudden drops in efficiency at depletion thresholds",
        efficiency_curve = function(depletion_ratio)
            if depletion_ratio < 0.3 then
                return 1.0
            elseif depletion_ratio < 0.6 then
                return 0.7
            elseif depletion_ratio < 0.9 then
                return 0.4
            else
                return 0.1
            end
        end,
        cost_curve = function(depletion_ratio)
            if depletion_ratio < 0.5 then
                return 1.0
            elseif depletion_ratio < 0.8 then
                return 1.8
            else
                return 3.0
            end
        end
    },
    
    plateau = {
        name = "Plateau Depletion",
        description = "Stable extraction until sudden drop-off",
        efficiency_curve = function(depletion_ratio)
            if depletion_ratio < 0.7 then
                return 1.0 -- Stable plateau
            else
                return 1.0 - ((depletion_ratio - 0.7) / 0.3) * 0.9 -- Rapid decline
            end
        end,
        cost_curve = function(depletion_ratio)
            if depletion_ratio < 0.7 then
                return 1.0
            else
                return 1.0 + ((depletion_ratio - 0.7) / 0.3) * 4.0 -- Sharp cost increase
            end
        end
    }
}

-- Resource-specific depletion characteristics
local ResourceDepletionProfiles = {
    uranium = {
        model = "exponential",
        base_depletion_rate = 0.02,
        environmental_impact = 0.9,
        extraction_difficulty = 0.8,
        recycling_potential = 0.7,
        discovery_rate_decline = 0.05
    },
    
    oil = {
        model = "plateau", -- Hubbert peak theory
        base_depletion_rate = 0.05,
        environmental_impact = 0.8,
        extraction_difficulty = 0.7,
        recycling_potential = 0.1, -- Very limited recycling
        discovery_rate_decline = 0.08
    },
    
    coal = {
        model = "linear",
        base_depletion_rate = 0.04,
        environmental_impact = 0.9,
        extraction_difficulty = 0.5,
        recycling_potential = 0.0, -- No recycling
        discovery_rate_decline = 0.03
    },
    
    iron = {
        model = "linear",
        base_depletion_rate = 0.03,
        environmental_impact = 0.6,
        extraction_difficulty = 0.6,
        recycling_potential = 0.9, -- Highly recyclable
        discovery_rate_decline = 0.02
    },
    
    gold = {
        model = "exponential",
        base_depletion_rate = 0.01,
        environmental_impact = 0.7,
        extraction_difficulty = 0.9,
        recycling_potential = 0.95, -- Almost perfectly recyclable
        discovery_rate_decline = 0.04
    },
    
    -- Renewable resources have different depletion mechanics
    wheat = {
        model = "regenerative",
        regeneration_rate = 1.0, -- Fully regenerates each season
        overuse_threshold = 2.0, -- Can be overused leading to soil depletion
        soil_degradation_rate = 0.1,
        environmental_impact = 0.3
    },
    
    fish = {
        model = "biological",
        regeneration_rate = 0.3, -- Population recovery rate
        overuse_threshold = 1.5, -- Overfishing threshold
        population_collapse_risk = 0.05,
        environmental_impact = 0.2
    }
}

-- Depletion event processing queue
local DepletionEventQueue = deque.new()

-- Depletion system state
local DepletionSystemState = {
    global_depletion_events = {},
    resource_exhaustion_warnings = {},
    environmental_damage_accumulator = {},
    recycling_technology_levels = {},
    discovery_difficulty_modifiers = {}
}

-- Core depletion calculation function
function calculate_extraction_efficiency(resource_type, current_quantity, efficiency_penalty, tech_level, infrastructure, environment)
    local profile = ResourceDepletionProfiles[resource_type]
    if not profile then
        Game.log("warning", "No depletion profile found for resource: " .. resource_type)
        return 1.0
    end
    
    local depletion_ratio = efficiency_penalty -- Passed from Rust system
    local model = DepletionModels[profile.model]
    
    if not model then
        Game.log("warning", "Unknown depletion model: " .. profile.model)
        return 1.0
    end
    
    -- Base efficiency from depletion model
    local base_efficiency = model.efficiency_curve(depletion_ratio)
    
    -- Technology improvements
    local tech_bonus = 1.0 + (tech_level - 1.0) * 0.3 -- 30% efficiency gain per tech level above 1.0
    
    -- Infrastructure quality effects
    local infrastructure_bonus = 0.7 + infrastructure * 0.3 -- 70-100% efficiency based on infrastructure
    
    -- Environmental conditions
    local environment_modifier = 0.8 + environment * 0.2 -- 80-100% efficiency based on conditions
    
    local final_efficiency = base_efficiency * tech_bonus * infrastructure_bonus * environment_modifier
    
    -- Apply recycling benefits for applicable resources
    if profile.recycling_potential and profile.recycling_potential > 0 then
        local recycling_level = DepletionSystemState.recycling_technology_levels[resource_type] or 0.1
        local recycling_bonus = 1.0 + (recycling_level * profile.recycling_potential * 0.5)
        final_efficiency = final_efficiency * recycling_bonus
    end
    
    return math.max(0.01, math.min(2.0, final_efficiency)) -- Clamp to 1%-200%
end

-- Efficiency penalty calculation based on depletion
function calculate_efficiency_penalty(resource_type, depletion_ratio)
    local profile = ResourceDepletionProfiles[resource_type]
    if not profile or not profile.model then
        return 0.0
    end
    
    local model = DepletionModels[profile.model]
    if not model then
        return 0.0
    end
    
    -- Calculate penalty as inverse of efficiency
    local efficiency = model.efficiency_curve(depletion_ratio)
    local penalty = 1.0 - efficiency
    
    -- Add environmental damage accumulation
    local env_damage = DepletionSystemState.environmental_damage_accumulator[resource_type] or 0.0
    penalty = penalty + (env_damage * 0.1) -- Up to 10% additional penalty from environmental damage
    
    return math.max(0.0, math.min(0.9, penalty)) -- Cap at 90% penalty
end

-- Advanced depletion event detection
function should_trigger_quality_degradation_event(resource_type, current_quality, original_quality)
    local degradation_threshold = 0.5 -- 50% quality loss triggers event
    local quality_ratio = current_quality / original_quality
    
    if quality_ratio <= degradation_threshold then
        -- Check if we haven't already triggered this event recently
        local recent_events = moses.filter(DepletionSystemState.global_depletion_events, function(event)
            return event.resource_type == resource_type and 
                   event.event_type == "quality_degradation" and
                   (Game.current_turn - event.turn) < 10 -- Within last 10 turns
        end)
        
        return #recent_events == 0 -- Only trigger if no recent similar event
    end
    
    return false
end

-- Environmental impact calculation
function calculate_environmental_impact(resource_type, extraction_amount, extraction_method)
    local profile = ResourceDepletionProfiles[resource_type]
    if not profile then
        return 0.0
    end
    
    local base_impact = profile.environmental_impact
    local method_modifiers = {
        surface_mining = 1.5,
        underground_mining = 1.0,
        hydraulic_fracturing = 2.0,
        offshore_drilling = 1.8,
        sustainable_extraction = 0.3
    }
    
    local method_modifier = method_modifiers[extraction_method] or 1.0
    local impact = base_impact * extraction_amount * method_modifier * 0.01 -- Scale factor
    
    -- Accumulate environmental damage
    local current_damage = DepletionSystemState.environmental_damage_accumulator[resource_type] or 0.0
    DepletionSystemState.environmental_damage_accumulator[resource_type] = current_damage + impact
    
    return impact
end

-- Resource substitution analysis
function analyze_substitution_opportunities(depleted_resource_type, current_reserves, extraction_rate)
    local substitution_matrix = {
        oil = {
            {resource = "coal", effectiveness = 0.7, tech_requirement = "coal_liquefaction"},
            {resource = "uranium", effectiveness = 2.0, tech_requirement = "nuclear_power"},
            {resource = "renewable_energy", effectiveness = 1.5, tech_requirement = "advanced_renewables"}
        },
        
        coal = {
            {resource = "oil", effectiveness = 0.8, tech_requirement = "oil_power_plants"},
            {resource = "uranium", effectiveness = 3.0, tech_requirement = "nuclear_power"},
            {resource = "renewable_energy", effectiveness = 1.0, tech_requirement = "wind_solar_power"}
        },
        
        iron = {
            {resource = "aluminum", effectiveness = 0.6, tech_requirement = "aluminum_alloys"},
            {resource = "steel_recycling", effectiveness = 0.9, tech_requirement = "advanced_recycling"},
            {resource = "composite_materials", effectiveness = 1.2, tech_requirement = "materials_science"}
        },
        
        uranium = {
            {resource = "thorium", effectiveness = 1.1, tech_requirement = "thorium_reactors"},
            {resource = "fusion_fuel", effectiveness = 10.0, tech_requirement = "fusion_power"},
            {resource = "renewable_energy", effectiveness = 0.8, tech_requirement = "energy_storage"}
        },
        
        wheat = {
            {resource = "rice", effectiveness = 0.9, tech_requirement = "irrigation"},
            {resource = "corn", effectiveness = 0.8, tech_requirement = "crop_rotation"},
            {resource = "synthetic_food", effectiveness = 1.5, tech_requirement = "food_synthesis"}
        }
    }
    
    local substitutes = substitution_matrix[depleted_resource_type] or {}
    local viable_substitutes = {}
    
    -- Calculate urgency based on reserves vs consumption
    local urgency = extraction_rate / math.max(current_reserves, 1)
    
    for _, substitute in ipairs(substitutes) do
        local viability_score = substitute.effectiveness * (1.0 + urgency * 0.5)
        
        table.insert(viable_substitutes, {
            resource = substitute.resource,
            effectiveness = substitute.effectiveness,
            technology_requirement = substitute.tech_requirement,
            viability_score = viability_score,
            urgency_factor = urgency
        })
    end
    
    -- Sort by viability score
    table.sort(viable_substitutes, function(a, b) 
        return a.viability_score > b.viability_score 
    end)
    
    return viable_substitutes
end

-- Depletion event creation and queuing
function queue_depletion_event(event_type, resource_type, severity, affected_positions, turn)
    local event = {
        event_type = event_type,
        resource_type = resource_type,
        severity = severity or 1.0,
        affected_positions = affected_positions or {},
        turn = turn or (Game.current_turn or 0),
        timestamp = os.time(),
        processed = false
    }
    
    DepletionEventQueue:push_right(event)
    table.insert(DepletionSystemState.global_depletion_events, event)
    
    Game.log("info", string.format("Queued depletion event: %s for %s (severity: %.2f)", 
        event_type, resource_type, severity))
end

-- Process depletion events from queue
function process_depletion_events(max_events_per_turn)
    local processed_events = {}
    local events_processed = 0
    
    while not DepletionEventQueue:empty() and events_processed < (max_events_per_turn or 5) do
        local event = DepletionEventQueue:pop_left()
        
        if not event.processed then
            local result = process_single_depletion_event(event)
            if result then
                event.processed = true
                event.processing_result = result
                table.insert(processed_events, event)
            end
        end
        
        events_processed = events_processed + 1
    end
    
    return processed_events
end

-- Process individual depletion event
function process_single_depletion_event(event)
    local event_processors = {
        exhaustion = process_exhaustion_event,
        quality_degradation = process_quality_degradation_event,
        efficiency_loss = process_efficiency_loss_event,
        environmental_damage = process_environmental_damage_event,
        market_crash = process_market_crash_event,
        resource_boom = process_resource_boom_event
    }
    
    local processor = event_processors[event.event_type]
    if processor then
        return processor(event)
    else
        Game.log("warning", "No processor for depletion event type: " .. event.event_type)
        return nil
    end
end

-- Specific event processors
function process_exhaustion_event(event)
    -- Resource exhaustion has major economic and strategic implications
    local economic_impact = calculate_exhaustion_economic_impact(event)
    local strategic_impact = calculate_exhaustion_strategic_impact(event)
    
    -- Trigger substitute resource searches
    local substitutes = analyze_substitution_opportunities(event.resource_type, 0, 0)
    
    return {
        event_type = "exhaustion_processed",
        resource_type = event.resource_type,
        economic_impact = economic_impact,
        strategic_impact = strategic_impact,
        recommended_substitutes = substitutes,
        global_market_effect = true
    }
end

function process_quality_degradation_event(event)
    -- Quality degradation affects extraction efficiency and market value
    local efficiency_impact = event.severity * 0.2 -- Up to 20% efficiency loss
    local value_impact = event.severity * 0.3 -- Up to 30% value loss
    
    return {
        event_type = "quality_degradation_processed",
        resource_type = event.resource_type,
        efficiency_penalty = efficiency_impact,
        market_value_penalty = value_impact,
        requires_technology_upgrade = efficiency_impact > 0.15
    }
end

function process_efficiency_loss_event(event)
    -- Efficiency loss due to aging infrastructure or depleted reserves
    local infrastructure_degradation = event.severity * 0.25
    local maintenance_cost_increase = event.severity * 0.4
    
    return {
        event_type = "efficiency_loss_processed",
        resource_type = event.resource_type,
        infrastructure_penalty = infrastructure_degradation,
        cost_increase = maintenance_cost_increase,
        requires_infrastructure_investment = infrastructure_degradation > 0.2
    }
end

function process_environmental_damage_event(event)
    -- Environmental damage affects long-term sustainability and public opinion
    local cleanup_cost = calculate_environmental_cleanup_cost(event)
    local reputation_impact = event.severity * 0.6
    
    return {
        event_type = "environmental_damage_processed",
        resource_type = event.resource_type,
        cleanup_cost = cleanup_cost,
        reputation_penalty = reputation_impact,
        requires_environmental_remediation = event.severity > 0.7
    }
end

function process_market_crash_event(event)
    -- Market crashes due to oversupply or demand collapse
    local price_impact = -event.severity * 0.6 -- Up to 60% price drop
    local recovery_time = math.ceil(event.severity * 20) -- 1-20 turns to recover
    
    return {
        event_type = "market_crash_processed",
        resource_type = event.resource_type,
        price_multiplier = 1.0 + price_impact,
        recovery_duration = recovery_time,
        affects_related_resources = true
    }
end

function process_resource_boom_event(event)
    -- New discoveries or technology breakthroughs
    local supply_increase = event.severity * 0.8 -- Up to 80% supply increase
    local price_impact = -event.severity * 0.4 -- Up to 40% price drop
    
    return {
        event_type = "resource_boom_processed",
        resource_type = event.resource_type,
        supply_multiplier = 1.0 + supply_increase,
        price_multiplier = 1.0 + price_impact,
        investment_opportunities = true
    }
end

-- Calculate economic impact of resource exhaustion
function calculate_exhaustion_economic_impact(event)
    local resource_importance = {
        uranium = 0.95,
        oil = 0.9,
        coal = 0.7,
        iron = 0.8,
        gold = 0.4
    }
    
    local importance = resource_importance[event.resource_type] or 0.5
    local base_impact = importance * event.severity
    
    return {
        gdp_impact = -base_impact * 0.1, -- Up to 10% GDP impact for critical resources
        inflation_impact = base_impact * 0.15, -- Up to 15% inflation
        trade_balance_impact = -base_impact * 0.2, -- Negative trade balance impact
        employment_impact = -base_impact * 0.08 -- Up to 8% employment impact
    }
end

-- Calculate strategic impact of resource exhaustion  
function calculate_exhaustion_strategic_impact(event)
    local strategic_resources = {"uranium", "oil", "iron", "coal"}
    local is_strategic = moses.find(strategic_resources, event.resource_type) ~= nil
    
    if not is_strategic then
        return {strategic_vulnerability = 0.0}
    end
    
    return {
        strategic_vulnerability = event.severity * 0.8,
        military_capability_impact = -event.severity * 0.3,
        diplomatic_leverage_change = -event.severity * 0.4,
        national_security_risk = event.severity > 0.7
    }
end

-- Calculate environmental cleanup costs
function calculate_environmental_cleanup_cost(event)
    local base_cleanup_costs = {
        uranium = 1000000, -- Very expensive cleanup
        oil = 500000,     -- Expensive spill cleanup
        coal = 200000,    -- Mine restoration
        iron = 100000,    -- Moderate cleanup
        gold = 150000     -- Mercury/cyanide cleanup
    }
    
    local base_cost = base_cleanup_costs[event.resource_type] or 50000
    return base_cost * event.severity * (0.8 + math.random() * 0.4) -- 80-120% of base cost
end

-- Save/load depletion system state
function serialize_depletion_state()
    return serpent.dump(DepletionSystemState)
end

function deserialize_depletion_state(serialized_data)
    local success, state = serpent.load(serialized_data)
    if success then
        DepletionSystemState = state
        return true
    else
        Game.log("error", "Failed to deserialize depletion state")
        return false
    end
end

Game.log("info", "Resource depletion system initialized with event processing queue")

return {
    calculate_extraction_efficiency = calculate_extraction_efficiency,
    calculate_efficiency_penalty = calculate_efficiency_penalty,
    should_trigger_quality_degradation_event = should_trigger_quality_degradation_event,
    calculate_environmental_impact = calculate_environmental_impact,
    analyze_substitution_opportunities = analyze_substitution_opportunities,
    queue_depletion_event = queue_depletion_event,
    process_depletion_events = process_depletion_events,
    serialize_depletion_state = serialize_depletion_state,
    deserialize_depletion_state = deserialize_depletion_state,
    depletion_models = DepletionModels,
    resource_profiles = ResourceDepletionProfiles
}
