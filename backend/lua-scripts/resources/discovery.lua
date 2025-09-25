-- Resource Discovery System
-- Uses behavior3 trees for discovery AI and advanced probability calculations
-- Implements realistic discovery mechanics based on technology and exploration

local behavior3 = require 'behavior3'
local deque = require 'deque'
local inspect = require 'inspect'

Game.log("info", "Loading resource discovery system...")

-- Discovery behavior tree nodes
local DiscoveryNodes = {}

-- Custom behavior tree node for technology evaluation
DiscoveryNodes.CheckTechnology = behavior3.Class("CheckTechnology", behavior3.Condition)
function DiscoveryNodes.CheckTechnology:initialize(params)
    behavior3.Condition.initialize(self, params)
    self.required_tech = params.required_tech or {}
    self.available_tech = params.available_tech or {}
end

function DiscoveryNodes.CheckTechnology:tick(tick)
    local civ_tech = tick.blackboard:get('civilization_technologies', tick.tree.id) or {}
    
    -- Check if civilization has required technologies
    for _, req_tech in ipairs(self.required_tech) do
        local has_tech = false
        for _, civ_tech_item in ipairs(civ_tech) do
            if civ_tech_item == req_tech then
                has_tech = true
                break
            end
        end
        
        if not has_tech then
            return behavior3.FAILURE
        end
    end
    
    return behavior3.SUCCESS
end

-- Discovery method evaluation node
DiscoveryNodes.EvaluateMethod = behavior3.Class("EvaluateMethod", behavior3.Action)
function DiscoveryNodes.EvaluateMethod:initialize(params)
    behavior3.Action.initialize(self, params)
    self.method = params.method
    self.base_effectiveness = params.base_effectiveness or 0.5
end

function DiscoveryNodes.EvaluateMethod:tick(tick)
    local resource_type = tick.blackboard:get('resource_type', tick.tree.id)
    local discovery_difficulty = tick.blackboard:get('discovery_difficulty', tick.tree.id)
    
    -- Calculate method effectiveness for this resource type
    local effectiveness = self:calculate_method_effectiveness(resource_type, self.method)
    local success_probability = effectiveness * (1.0 - discovery_difficulty)
    
    tick.blackboard:set('success_probability', success_probability, tick.tree.id)
    
    if success_probability > 0.3 then -- Minimum threshold for attempt
        return behavior3.SUCCESS
    else
        return behavior3.FAILURE
    end
end

function DiscoveryNodes.EvaluateMethod:calculate_method_effectiveness(resource_type, method)
    -- Method effectiveness by resource type
    local effectiveness_matrix = {
        RandomExploration = {
            uranium = 0.1,
            oil = 0.2,
            iron = 0.4,
            coal = 0.6,
            gold = 0.3,
            wheat = 0.9,
            fish = 0.8
        },
        SystematicSurvey = {
            uranium = 0.7,
            oil = 0.8,
            iron = 0.9,
            coal = 0.8,
            gold = 0.6,
            wheat = 0.5,
            fish = 0.6
        },
        GeologicalAnalysis = {
            uranium = 0.9,
            oil = 0.9,
            iron = 0.8,
            coal = 0.7,
            gold = 0.8,
            wheat = 0.2,
            fish = 0.3
        },
        RemoteSensing = {
            uranium = 0.6,
            oil = 0.9,
            iron = 0.7,
            coal = 0.6,
            gold = 0.5,
            wheat = 0.8,
            fish = 0.7
        },
        TradeIntelligence = {
            uranium = 0.8,
            oil = 0.7,
            iron = 0.6,
            coal = 0.5,
            gold = 0.9,
            wheat = 0.4,
            fish = 0.5
        },
        AccidentalDiscovery = {
            uranium = 0.05,
            oil = 0.1,
            iron = 0.3,
            coal = 0.4,
            gold = 0.2,
            wheat = 0.6,
            fish = 0.3
        }
    }
    
    local method_data = effectiveness_matrix[method]
    if method_data then
        return method_data[resource_type] or 0.1
    else
        return 0.1
    end
end

-- Discovery behavior trees for different approaches
local DiscoveryBehaviors = {}

-- Systematic survey behavior tree
function DiscoveryBehaviors.create_systematic_survey_tree()
    local tree = behavior3.BehaviorTree()
    
    local root = behavior3.Sequence({
        -- Check prerequisites
        DiscoveryNodes.CheckTechnology({
            required_tech = {"surveying", "geology_basics"}
        }),
        
        -- Evaluate systematic method
        DiscoveryNodes.EvaluateMethod({
            method = "SystematicSurvey",
            base_effectiveness = 0.8
        }),
        
        -- Execute survey (placeholder - would trigger actual survey logic)
        behavior3.Action({
            run = function(tick)
                local probability = tick.blackboard:get('success_probability', tick.tree.id)
                
                -- Systematic surveys take time but are thorough
                local survey_progress = tick.blackboard:get('survey_progress', tick.tree.id) or 0
                survey_progress = survey_progress + 0.1 -- 10% progress per turn
                tick.blackboard:set('survey_progress', survey_progress, tick.tree.id)
                
                if survey_progress >= 1.0 then
                    tick.blackboard:set('discovery_success', probability > 0.5, tick.tree.id)
                    return behavior3.SUCCESS
                else
                    return behavior3.RUNNING
                end
            end
        })
    })
    
    tree.root = root
    return tree
end

-- Geological analysis behavior tree
function DiscoveryBehaviors.create_geological_analysis_tree()
    local tree = behavior3.BehaviorTree()
    
    local root = behavior3.Sequence({
        -- Check advanced technology requirements
        DiscoveryNodes.CheckTechnology({
            required_tech = {"advanced_geology", "geophysics", "chemical_analysis"}
        }),
        
        -- Evaluate geological method
        DiscoveryNodes.EvaluateMethod({
            method = "GeologicalAnalysis",
            base_effectiveness = 0.9
        }),
        
        -- Analyze geological indicators
        behavior3.Action({
            run = function(tick)
                local geological_context = tick.blackboard:get('geological_context', tick.tree.id)
                local resource_type = tick.blackboard:get('resource_type', tick.tree.id)
                
                local indicator_score = calculate_geological_indicators(geological_context, resource_type)
                local base_probability = tick.blackboard:get('success_probability', tick.tree.id)
                
                -- Boost probability based on geological indicators
                local final_probability = base_probability * (0.5 + 0.5 * indicator_score)
                tick.blackboard:set('discovery_success', final_probability > 0.6, tick.tree.id)
                
                return behavior3.SUCCESS
            end
        })
    })
    
    tree.root = root
    return tree
end

-- Calculate geological indicators for resource presence
function calculate_geological_indicators(context, resource_type)
    local indicators = context.tectonic_features or {}
    local indicator_score = 0.0
    
    -- Resource-specific indicator patterns
    local indicator_patterns = {
        uranium = {"granite_intrusion", "pegmatite", "sandstone_host"},
        oil = {"sedimentary_basin", "anticline", "source_rock"},
        iron = {"banded_iron_formation", "metamorphic_core", "laterite"},
        coal = {"ancient_swamp", "plant_fossils", "carbonaceous_shale"},
        gold = {"quartz_vein", "hydrothermal_alteration", "placer_deposit"}
    }
    
    local patterns = indicator_patterns[resource_type] or {}
    local matches = 0
    
    for _, pattern in ipairs(patterns) do
        for _, feature in ipairs(indicators) do
            if string.find(string.lower(feature), string.lower(pattern)) then
                matches = matches + 1
                break
            end
        end
    end
    
    indicator_score = #patterns > 0 and (matches / #patterns) or 0.0
    
    -- Bonus for multiple indicators (geological convergence)
    if matches > 2 then
        indicator_score = indicator_score * 1.3
    end
    
    return math.min(1.0, indicator_score)
end

-- Discovery probability calculation with advanced factors
function calculate_discovery_probability(resource_type, base_difficulty, method_json, exploration_progress, civ_id)
    local success, method_data = pcall(function()
        return Game.json_decode(method_json)
    end)
    
    if not success then
        Game.log("warning", "Failed to decode discovery method data")
        return 0.0
    end
    
    -- Base probability calculation
    local base_probability = (1.0 - base_difficulty) * 0.3 -- 30% base for perfect conditions
    
    -- Method effectiveness modifier
    local method_effectiveness = 0.5 -- Default
    
    if method_data == "SystematicSurvey" then
        method_effectiveness = 0.8
    elseif method_data == "GeologicalAnalysis" then
        method_effectiveness = 0.9
    elseif method_data == "RemoteSensing" then
        method_effectiveness = 0.7
    elseif method_data == "RandomExploration" then
        method_effectiveness = 0.3
    elseif method_data == "TradeIntelligence" then
        method_effectiveness = 0.6
    end
    
    -- Apply resource-specific modifiers
    local resource_modifiers = {
        uranium = 0.6, -- Very difficult to discover
        oil = 0.8,     -- Moderate difficulty with right methods
        iron = 1.2,    -- Relatively easy to find
        coal = 1.0,    -- Standard difficulty
        gold = 0.7,    -- Difficult but valuable
        wheat = 1.5,   -- Easy for agricultural resources
        fish = 1.3     -- Relatively straightforward
    }
    
    local resource_modifier = resource_modifiers[resource_type] or 1.0
    
    -- Exploration progress bonus
    local progress_bonus = exploration_progress * 0.4 -- Up to 40% bonus for sustained effort
    
    -- Final probability calculation
    local final_probability = base_probability * method_effectiveness * resource_modifier + progress_bonus
    
    -- Diminishing returns for very high probabilities
    if final_probability > 0.8 then
        final_probability = 0.8 + (final_probability - 0.8) * 0.3
    end
    
    return math.max(0.0, math.min(1.0, final_probability))
end

-- Get technologies that help discover specific resources
function get_helpful_technologies(resource_type)
    local tech_matrix = {
        uranium = {"geology", "chemistry", "nuclear_physics", "geiger_counter", "aerial_survey"},
        oil = {"geology", "drilling", "seismic_survey", "chemical_analysis", "well_logging"},
        iron = {"geology", "mining", "magnetic_survey", "chemical_testing"},
        coal = {"geology", "mining", "surveying", "core_drilling"},
        gold = {"geology", "mining", "metallurgy", "placer_mining", "chemical_assaying"},
        wheat = {"agriculture", "soil_science", "climate_knowledge"},
        fish = {"navigation", "marine_biology", "fishing_technology"}
    }
    
    return tech_matrix[resource_type] or {"basic_exploration"}
end

-- Calculate efficiency modifiers based on available technologies
function calculate_efficiency_modifiers(technologies_json)
    local success, technologies = pcall(function()
        return Game.json_decode(technologies_json)
    end)
    
    if not success then
        return "{}"
    end
    
    local modifiers = {}
    
    -- Base method modifiers
    modifiers["RandomExploration"] = 1.0
    modifiers["SystematicSurvey"] = 1.0
    modifiers["GeologicalAnalysis"] = 1.0
    modifiers["RemoteSensing"] = 1.0
    modifiers["TradeIntelligence"] = 1.0
    
    -- Technology bonuses
    local tech_bonuses = {
        -- Survey technologies
        surveying = {SystematicSurvey = 0.2},
        advanced_surveying = {SystematicSurvey = 0.4},
        
        -- Geological technologies
        geology = {GeologicalAnalysis = 0.3},
        advanced_geology = {GeologicalAnalysis = 0.5},
        geophysics = {GeologicalAnalysis = 0.4, RemoteSensing = 0.3},
        
        -- Remote sensing technologies
        aerial_photography = {RemoteSensing = 0.3},
        satellite_imagery = {RemoteSensing = 0.6},
        ground_penetrating_radar = {RemoteSensing = 0.4},
        
        -- Analysis technologies
        chemical_analysis = {GeologicalAnalysis = 0.2, SystematicSurvey = 0.1},
        spectroscopy = {RemoteSensing = 0.3, GeologicalAnalysis = 0.3},
        
        -- Intelligence gathering
        cartography = {TradeIntelligence = 0.2},
        linguistics = {TradeIntelligence = 0.3},
        diplomacy = {TradeIntelligence = 0.4}
    }
    
    -- Apply technology bonuses
    for _, tech in ipairs(technologies) do
        local tech_bonus = tech_bonuses[tech]
        if tech_bonus then
            for method, bonus in pairs(tech_bonus) do
                modifiers[method] = (modifiers[method] or 1.0) + bonus
            end
        end
    end
    
    return Game.json_encode(modifiers)
end

-- Calculate information quality based on discovery method
function calculate_information_quality(method_json, resource_type)
    local success, method = pcall(function()
        return Game.json_decode(method_json)
    end)
    
    if not success then
        return 0.3 -- Low quality fallback
    end
    
    local quality_matrix = {
        SystematicSurvey = 0.9,      -- Very accurate information
        GeologicalAnalysis = 0.95,    -- Extremely accurate
        RemoteSensing = 0.8,         -- Good accuracy
        TradeIntelligence = 0.6,     -- Moderate accuracy (rumors, estimates)
        RandomExploration = 0.4,      -- Poor accuracy
        AccidentalDiscovery = 0.5     -- Variable accuracy
    }
    
    local base_quality = quality_matrix[method] or 0.3
    
    -- Some resources are inherently harder to assess accurately
    local resource_assessment_difficulty = {
        uranium = 0.8, -- Requires specialized equipment
        oil = 0.9,     -- Well understood assessment techniques
        iron = 0.95,   -- Easy to assess visually and chemically
        coal = 0.9,    -- Relatively straightforward
        gold = 0.7,    -- Can be hard to estimate true extent
        wheat = 0.95,  -- Very obvious quality assessment
        fish = 0.8     -- Seasonal and population variations
    }
    
    local assessment_modifier = resource_assessment_difficulty[resource_type] or 0.8
    
    return math.max(0.1, math.min(1.0, base_quality * assessment_modifier))
end

-- Discovery queue management using deque for efficient processing
local DiscoveryQueue = {}

function DiscoveryQueue:new()
    local queue = {
        tasks = deque.new(),
        priorities = {}
    }
    setmetatable(queue, {__index = self})
    return queue
end

function DiscoveryQueue:add_task(task)
    -- Insert task based on priority
    local priority = task.priority or 0.5
    
    if priority > 0.8 then
        self.tasks:push_left(task) -- High priority - front of queue
    elseif priority < 0.3 then
        self.tasks:push_right(task) -- Low priority - back of queue  
    else
        -- Medium priority - insert in middle (simplified)
        self.tasks:push_right(task)
    end
end

function DiscoveryQueue:get_next_task()
    return self.tasks:pop_left()
end

function DiscoveryQueue:is_empty()
    return self.tasks:length() == 0
end

-- Global discovery queue instance
local global_discovery_queue = DiscoveryQueue:new()

-- Add discovery task to global queue
function queue_discovery_task(civ_id, target_q, target_r, method, priority)
    global_discovery_queue:add_task({
        civ_id = civ_id,
        target_q = target_q,
        target_r = target_r,
        method = method,
        priority = priority or 0.5,
        queued_turn = Game.current_turn or 0
    })
end

-- Process discovery queue (called each turn)
function process_discovery_queue()
    local processed_tasks = {}
    local max_tasks_per_turn = 10 -- Limit processing for performance
    
    local tasks_processed = 0
    while not global_discovery_queue:is_empty() and tasks_processed < max_tasks_per_turn do
        local task = global_discovery_queue:get_next_task()
        
        -- Process task (simplified)
        Game.log("debug", string.format("Processing discovery task for civ %d at (%d,%d)", 
            task.civ_id, task.target_q, task.target_r))
        
        table.insert(processed_tasks, task)
        tasks_processed = tasks_processed + 1
    end
    
    return processed_tasks
end

Game.log("info", "Resource discovery system initialized with behavior trees")

return {
    calculate_discovery_probability = calculate_discovery_probability,
    get_helpful_technologies = get_helpful_technologies, 
    calculate_efficiency_modifiers = calculate_efficiency_modifiers,
    calculate_information_quality = calculate_information_quality,
    queue_discovery_task = queue_discovery_task,
    process_discovery_queue = process_discovery_queue,
    discovery_behaviors = DiscoveryBehaviors
}
