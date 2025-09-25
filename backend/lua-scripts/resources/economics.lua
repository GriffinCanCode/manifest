-- Resource Economics and Market Dynamics
-- Uses lua-protobuf for configuration and serpent for data serialization
-- Implements sophisticated market simulation with supply/demand modeling

local serpent = require 'serpent'
local moses = require 'moses'
local inspect = require 'inspect'

Game.log("info", "Loading resource economics configuration...")

-- Market structure definitions
local MarketStructures = {
    perfect_competition = {
        name = "Perfect Competition",
        price_elasticity = 0.8,
        volatility_factor = 0.3,
        barriers_to_entry = 0.1,
        information_transparency = 0.9
    },
    
    oligopoly = {
        name = "Oligopoly",
        price_elasticity = 0.5,
        volatility_factor = 0.6,
        barriers_to_entry = 0.7,
        information_transparency = 0.6
    },
    
    monopoly = {
        name = "Monopoly",
        price_elasticity = 0.2,
        volatility_factor = 0.8,
        barriers_to_entry = 0.9,
        information_transparency = 0.3
    },
    
    cartel = {
        name = "Cartel",
        price_elasticity = 0.3,
        volatility_factor = 0.9,
        barriers_to_entry = 0.8,
        information_transparency = 0.2
    }
}

-- Economic properties by resource category
local CategoryEconomics = {
    strategic = {
        market_structure = "oligopoly",
        base_elasticity = 0.3,
        speculation_factor = 0.8,
        government_intervention = 0.9,
        strategic_stockpiling = 0.9
    },
    
    industrial = {
        market_structure = "perfect_competition",
        base_elasticity = 0.6,
        speculation_factor = 0.4,
        government_intervention = 0.3,
        strategic_stockpiling = 0.5
    },
    
    precious = {
        market_structure = "oligopoly",
        base_elasticity = 0.2,
        speculation_factor = 0.9,
        government_intervention = 0.6,
        strategic_stockpiling = 0.8
    },
    
    agricultural = {
        market_structure = "perfect_competition",
        base_elasticity = 0.8,
        speculation_factor = 0.5,
        government_intervention = 0.7,
        strategic_stockpiling = 0.4
    },
    
    energy = {
        market_structure = "cartel",
        base_elasticity = 0.4,
        speculation_factor = 0.9,
        government_intervention = 0.8,
        strategic_stockpiling = 0.9
    }
}

-- Price calculation models
local PriceModels = {}

function PriceModels.supply_demand_model(supply, demand, base_price, elasticity)
    if supply <= 0 then
        return base_price * 10 -- Extreme shortage
    end
    
    local supply_demand_ratio = supply / demand
    
    -- Apply price elasticity using exponential model
    local price_multiplier = math.pow(supply_demand_ratio, -elasticity)
    
    -- Clamp to reasonable bounds (10x up or down)
    price_multiplier = math.max(0.1, math.min(10.0, price_multiplier))
    
    return base_price * price_multiplier
end

function PriceModels.volatility_model(current_price, base_price, volatility_factor, market_news)
    -- Random walk with mean reversion
    local mean_reversion_strength = 0.1
    local random_shock = (math.random() - 0.5) * 2 -- -1 to 1
    
    -- Mean reversion component
    local mean_reversion = (base_price - current_price) * mean_reversion_strength
    
    -- Volatility shock
    local volatility_shock = current_price * volatility_factor * random_shock * 0.1
    
    -- News impact
    local news_impact = 0
    if market_news then
        for _, news in ipairs(market_news) do
            news_impact = news_impact + news.price_impact
        end
    end
    
    local new_price = current_price + mean_reversion + volatility_shock + news_impact
    
    -- Prevent negative prices
    return math.max(0.01, new_price)
end

function PriceModels.speculation_model(price, speculation_factor, market_sentiment)
    -- Speculation amplifies price movements
    local sentiment_multiplier = 1.0
    
    if market_sentiment > 0.6 then
        sentiment_multiplier = 1.0 + (market_sentiment - 0.6) * speculation_factor
    elseif market_sentiment < 0.4 then
        sentiment_multiplier = 1.0 - (0.4 - market_sentiment) * speculation_factor
    end
    
    return price * sentiment_multiplier
end

-- Economic indicators and metrics
local EconomicIndicators = {}

function EconomicIndicators.calculate_price_index(prices, weights)
    local weighted_sum = 0
    local total_weight = 0
    
    for resource_type, price in pairs(prices) do
        local weight = weights[resource_type] or 1.0
        weighted_sum = weighted_sum + (price * weight)
        total_weight = total_weight + weight
    end
    
    return total_weight > 0 and (weighted_sum / total_weight) or 1.0
end

function EconomicIndicators.calculate_inflation_rate(current_index, previous_index)
    if previous_index <= 0 then
        return 0.0
    end
    
    return ((current_index - previous_index) / previous_index) * 100
end

function EconomicIndicators.calculate_market_concentration(market_shares)
    -- Herfindahl-Hirschman Index
    local hhi = 0
    for _, share in pairs(market_shares) do
        hhi = hhi + (share * share)
    end
    return hhi
end

function EconomicIndicators.calculate_consumer_surplus(demand_curve, equilibrium_price, equilibrium_quantity)
    -- Simplified consumer surplus calculation
    -- Assumes linear demand curve for simplicity
    local max_price = demand_curve.intercept or (equilibrium_price * 2)
    return 0.5 * (max_price - equilibrium_price) * equilibrium_quantity
end

-- Market dynamics simulation
local MarketSimulation = {}

function MarketSimulation:new(resource_type, initial_conditions)
    local market = {
        resource_type = resource_type,
        supply = initial_conditions.supply or 1000,
        demand = initial_conditions.demand or 1000,
        price = initial_conditions.price or 1.0,
        price_history = {initial_conditions.price or 1.0},
        participants = initial_conditions.participants or {},
        news_events = {},
        market_structure = initial_conditions.market_structure or "perfect_competition"
    }
    
    setmetatable(market, {__index = self})
    return market
end

function MarketSimulation:update_market(turn)
    -- Update supply based on production
    self:update_supply()
    
    -- Update demand based on economic activity
    self:update_demand()
    
    -- Calculate new equilibrium price
    self:calculate_equilibrium_price()
    
    -- Apply market dynamics
    self:apply_market_dynamics()
    
    -- Record price history
    table.insert(self.price_history, self.price)
    
    -- Keep only last 100 price points for memory efficiency
    if #self.price_history > 100 then
        table.remove(self.price_history, 1)
    end
    
    -- Generate market events
    self:generate_market_events(turn)
end

function MarketSimulation:update_supply()
    -- Supply updates based on production capacity and extraction rates
    local production_efficiency = self:calculate_production_efficiency()
    local new_production = self:calculate_new_production()
    
    self.supply = self.supply + new_production * production_efficiency
    
    -- Account for resource depletion
    local depletion_rate = self:get_resource_depletion_rate()
    if depletion_rate > 0 then
        self.supply = self.supply * (1.0 - depletion_rate)
    end
end

function MarketSimulation:update_demand()
    -- Demand updates based on economic growth and population
    local economic_growth = self:get_economic_growth_rate()
    local population_growth = self:get_population_growth_rate()
    
    -- Base demand growth
    local demand_growth = (economic_growth + population_growth) / 2
    
    -- Price elasticity effect on demand
    local price_elasticity = self:get_price_elasticity()
    local price_change = self.price_history[#self.price_history] / self.price_history[math.max(1, #self.price_history - 1)]
    local price_effect = math.pow(price_change, -price_elasticity)
    
    self.demand = self.demand * (1 + demand_growth) * price_effect
end

function MarketSimulation:calculate_equilibrium_price()
    local market_structure = MarketStructures[self.market_structure]
    local elasticity = market_structure.price_elasticity
    
    self.price = PriceModels.supply_demand_model(
        self.supply,
        self.demand,
        self.price,
        elasticity
    )
end

function MarketSimulation:apply_market_dynamics()
    local market_structure = MarketStructures[self.market_structure]
    
    -- Apply volatility
    self.price = PriceModels.volatility_model(
        self.price,
        1.0, -- base price
        market_structure.volatility_factor,
        self.news_events
    )
    
    -- Apply speculation if applicable
    local market_sentiment = self:calculate_market_sentiment()
    if market_sentiment then
        local category_econ = CategoryEconomics[self:get_resource_category()]
        if category_econ then
            self.price = PriceModels.speculation_model(
                self.price,
                category_econ.speculation_factor,
                market_sentiment
            )
        end
    end
end

function MarketSimulation:calculate_market_sentiment()
    -- Simplified sentiment based on recent price trends
    if #self.price_history < 5 then
        return 0.5 -- Neutral
    end
    
    local recent_prices = moses.slice(self.price_history, -5, -1)
    local trend = 0
    
    for i = 2, #recent_prices do
        if recent_prices[i] > recent_prices[i-1] then
            trend = trend + 1
        else
            trend = trend - 1
        end
    end
    
    -- Normalize to 0-1
    return 0.5 + (trend / (#recent_prices - 1)) * 0.5
end

function MarketSimulation:generate_market_events(turn)
    -- Clear old news events
    self.news_events = {}
    
    -- Random market events
    if math.random() < 0.1 then -- 10% chance per turn
        local event_types = {
            "supply_shock",
            "demand_surge", 
            "technological_breakthrough",
            "trade_disruption",
            "regulatory_change"
        }
        
        local event_type = event_types[math.random(#event_types)]
        local event = self:create_market_event(event_type, turn)
        
        if event then
            table.insert(self.news_events, event)
            Game.log("info", "Market event: " .. event.description)
        end
    end
end

function MarketSimulation:create_market_event(event_type, turn)
    local events = {
        supply_shock = {
            description = "Major supply disruption affects " .. self.resource_type .. " market",
            price_impact = self.price * (0.1 + math.random() * 0.3), -- 10-40% price increase
            duration = 3 + math.random(5), -- 3-8 turns
            supply_multiplier = 0.7 + math.random() * 0.2 -- 70-90% of normal supply
        },
        
        demand_surge = {
            description = "Unexpected demand increase for " .. self.resource_type,
            price_impact = self.price * (0.05 + math.random() * 0.15), -- 5-20% price increase
            duration = 2 + math.random(4), -- 2-6 turns
            demand_multiplier = 1.2 + math.random() * 0.3 -- 120-150% of normal demand
        },
        
        technological_breakthrough = {
            description = "New technology reduces demand for " .. self.resource_type,
            price_impact = -self.price * (0.15 + math.random() * 0.25), -- 15-40% price decrease
            duration = 10 + math.random(20), -- Long-term effect
            demand_multiplier = 0.6 + math.random() * 0.3 -- 60-90% of normal demand
        },
        
        trade_disruption = {
            description = "Trade route disruption affects " .. self.resource_type .. " availability",
            price_impact = self.price * (0.08 + math.random() * 0.12), -- 8-20% price increase
            duration = 1 + math.random(3), -- 1-4 turns
            supply_multiplier = 0.8 + math.random() * 0.15 -- 80-95% of normal supply
        },
        
        regulatory_change = {
            description = "New regulations affect " .. self.resource_type .. " market",
            price_impact = (math.random() > 0.5 and 1 or -1) * self.price * (0.05 + math.random() * 0.1), -- +/- 5-15%
            duration = 5 + math.random(10), -- Medium-term effect
            supply_multiplier = 0.9 + math.random() * 0.2 -- 90-110% of normal supply
        }
    }
    
    return events[event_type]
end

-- Helper methods for market simulation
function MarketSimulation:calculate_production_efficiency()
    -- Placeholder - would integrate with actual production systems
    return 0.9 + math.random() * 0.2 -- 90-110% efficiency
end

function MarketSimulation:calculate_new_production()
    -- Placeholder - would calculate based on investment and capacity
    return self.supply * (0.01 + math.random() * 0.05) -- 1-6% growth per turn
end

function MarketSimulation:get_resource_depletion_rate()
    -- Would integrate with actual depletion system
    local depletion_rates = {
        uranium = 0.001,
        oil = 0.002,
        coal = 0.0015,
        iron = 0.0005,
        gold = 0.0001
    }
    return depletion_rates[self.resource_type] or 0.0
end

function MarketSimulation:get_economic_growth_rate()
    -- Placeholder - would get from economic system
    return 0.02 + math.random() * 0.04 -- 2-6% annual growth
end

function MarketSimulation:get_population_growth_rate()
    -- Placeholder - would get from demographics system
    return 0.01 + math.random() * 0.02 -- 1-3% annual growth
end

function MarketSimulation:get_price_elasticity()
    local category = self:get_resource_category()
    local category_econ = CategoryEconomics[category]
    return category_econ and category_econ.base_elasticity or 0.5
end

function MarketSimulation:get_resource_category()
    -- Would get from resource definitions
    local categories = {
        uranium = "strategic",
        oil = "energy",
        coal = "energy",
        iron = "industrial",
        gold = "precious",
        wheat = "agricultural",
        fish = "agricultural"
    }
    return categories[self.resource_type] or "industrial"
end

-- Market price calculation function for Rust integration
function calculate_market_price(resource_type, current_price, scarcity_index, extraction_rate)
    -- Create temporary market simulation for price calculation
    local market = MarketSimulation:new(resource_type, {
        price = current_price,
        supply = 1000 * (1.0 - scarcity_index),
        demand = 1000,
        market_structure = "perfect_competition"
    })
    
    -- Apply scarcity effects
    local scarcity_multiplier = 1.0 + (scarcity_index * 2.0) -- Up to 3x price increase for extreme scarcity
    
    -- Apply extraction rate effects (higher extraction = lower prices due to increased supply)
    local extraction_multiplier = 1.0 / (1.0 + extraction_rate * 0.1)
    
    local new_price = current_price * scarcity_multiplier * extraction_multiplier
    
    -- Apply category-specific constraints
    local category = market:get_resource_category()
    local category_econ = CategoryEconomics[category]
    
    if category_econ then
        -- Government intervention can stabilize prices
        if category_econ.government_intervention > 0.7 then
            -- Reduce extreme price movements
            local change_ratio = new_price / current_price
            if change_ratio > 1.5 then
                new_price = current_price * (1.5 + (change_ratio - 1.5) * 0.3)
            elseif change_ratio < 0.5 then
                new_price = current_price * (0.5 + (0.5 - change_ratio) * 0.3)
            end
        end
    end
    
    return math.max(0.01, new_price) -- Minimum price floor
end

-- Trade value calculation
function calculate_trade_value(resource_type, quantity, quality, distance, trade_route_efficiency)
    -- Base value calculation
    local base_prices = {
        uranium = 100,
        oil = 50,
        coal = 15,
        iron = 20,
        gold = 200,
        wheat = 5,
        fish = 8
    }
    
    local base_price = base_prices[resource_type] or 10
    local base_value = quantity * quality * base_price
    
    -- Distance decay
    local distance_multiplier = 1.0 / (1.0 + distance * 0.01) -- 1% reduction per distance unit
    
    -- Trade route efficiency
    local efficiency_multiplier = 0.5 + trade_route_efficiency * 0.5 -- 50-100% efficiency
    
    -- Market demand bonus (simplified)
    local demand_multiplier = 0.8 + math.random() * 0.4 -- 80-120% base demand
    
    return base_value * distance_multiplier * efficiency_multiplier * demand_multiplier
end

-- Strategic value assessment
function calculate_strategic_value(resource_type, civilization_id, current_reserves, projected_needs)
    local strategic_importance = {
        uranium = 0.95,
        oil = 0.9,
        iron = 0.8,
        coal = 0.7,
        gold = 0.6,
        wheat = 0.85,
        fish = 0.4
    }
    
    local base_importance = strategic_importance[resource_type] or 0.5
    
    -- Scarcity multiplier
    local reserve_ratio = current_reserves / math.max(projected_needs, 1)
    local scarcity_multiplier = 1.0
    
    if reserve_ratio < 0.5 then
        scarcity_multiplier = 2.0 -- Double importance if reserves < 50% of needs
    elseif reserve_ratio < 1.0 then
        scarcity_multiplier = 1.5 -- 1.5x importance if reserves < 100% of needs
    end
    
    return base_importance * scarcity_multiplier
end

Game.log("info", "Resource economics system initialized with market simulation")

return {
    calculate_market_price = calculate_market_price,
    calculate_trade_value = calculate_trade_value,
    calculate_strategic_value = calculate_strategic_value,
    market_simulation = MarketSimulation,
    economic_indicators = EconomicIndicators,
    category_economics = CategoryEconomics
}
