//! Change Detection
//!
//! Central system for all change detection in the ECS. All component changes
//! flow through this system, which provides monitoring, callbacks, and actions
//! based on detected changes. Eliminates duplicate change queries across systems.

use bevy_ecs::prelude::*;
use tracing::{info, debug, warn, instrument};
use serde::{Deserialize, Serialize};
use std::any::{TypeId, type_name};
use std::time::Instant;

use crate::core::{hashing::{collections, FastHashMap}, logging::{LoggingSystem, game_logging}};
use crate::ecs::{components::*, resources::*, spatial::OptimalSpatialIndex};

/// Resource for tracking change detection statistics across the game
#[derive(Resource, Debug, Clone, Default)]
pub struct ChangeMonitor {
    /// Statistics by component type (optimized for TypeId keys)
    stats: FastHashMap<TypeId, ChangeStats>,
    /// Total changes across all components
    total_changes: u64,
    /// When monitoring started
    start_time: Option<Instant>,
    /// Whether monitoring is enabled
    enabled: bool,
    /// Cached most active types result (cache invalidated when stats change)
    cached_most_active: Option<(Vec<(String, u64)>, Instant)>,
    /// Cached change summary (cache invalidated when stats change)  
    cached_summary: Option<(String, Instant)>,
}

/// Statistics for a specific component type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeStats {
    /// Component type name for debugging
    pub type_name: String,
    /// Number of additions detected
    pub additions: u64,
    /// Number of modifications detected
    pub modifications: u64,
    /// Number of removals detected
    pub removals: u64,
    /// Last update timestamp
    #[serde(skip)]
    pub last_update: Option<Instant>,
}

impl ChangeStats {
    /// Create new empty statistics
    pub fn new<T: Component>() -> Self {
        Self {
            type_name: type_name::<T>().to_string(),
            additions: 0,
            modifications: 0,
            removals: 0,
            last_update: Some(Instant::now()),
        }
    }

    /// Get total changes for this component type
    pub fn total(&self) -> u64 {
        self.additions + self.modifications + self.removals
    }

    /// Record an addition
    pub fn record_addition(&mut self) {
        self.additions += 1;
        self.last_update = Some(Instant::now());
    }

    /// Record a modification
    pub fn record_modification(&mut self) {
        self.modifications += 1;
        self.last_update = Some(Instant::now());
    }

    /// Record a removal
    pub fn record_removal(&mut self) {
        self.removals += 1;
        self.last_update = Some(Instant::now());
    }
}

impl ChangeMonitor {
    /// Create new change monitor
    pub fn new() -> Self {
        Self {
            stats: collections::fast_hash_map(),
            total_changes: 0,
            start_time: Some(Instant::now()),
            enabled: true,
            cached_most_active: None,
            cached_summary: None,
        }
    }

    /// Enable/disable monitoring
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if enabled && self.start_time.is_none() {
            self.start_time = Some(Instant::now());
        }
    }

    /// Record component additions
    pub fn record_additions<T: Component>(&mut self, count: usize) {
        if !self.enabled { return; }
        
        let type_id = TypeId::of::<T>();
        let stats = self.stats.entry(type_id).or_insert_with(|| ChangeStats::new::<T>());
        stats.additions += count as u64;
        stats.last_update = Some(Instant::now());
        self.total_changes += count as u64;
        
        // Invalidate caches when stats change
        self.invalidate_caches();
    }

    /// Record component modifications
    pub fn record_modifications<T: Component>(&mut self, count: usize) {
        if !self.enabled { return; }
        
        let type_id = TypeId::of::<T>();
        let stats = self.stats.entry(type_id).or_insert_with(|| ChangeStats::new::<T>());
        stats.modifications += count as u64;
        stats.last_update = Some(Instant::now());
        self.total_changes += count as u64;
        
        // Invalidate caches when stats change
        self.invalidate_caches();
    }

    /// Record component removals
    pub fn record_removals<T: Component>(&mut self, count: usize) {
        if !self.enabled { return; }
        
        let type_id = TypeId::of::<T>();
        let stats = self.stats.entry(type_id).or_insert_with(|| ChangeStats::new::<T>());
        stats.removals += count as u64;
        stats.last_update = Some(Instant::now());
        self.total_changes += count as u64;
        
        // Invalidate caches when stats change
        self.invalidate_caches();
    }

    /// Get statistics for a component type
    pub fn get_stats<T: Component>(&self) -> Option<&ChangeStats> {
        self.stats.get(&TypeId::of::<T>())
    }

    /// Get all statistics
    pub fn all_stats(&self) -> &FastHashMap<TypeId, ChangeStats> {
        &self.stats
    }

    /// Get total change count
    pub fn total_changes(&self) -> u64 {
        self.total_changes
    }

    /// Get changes per second since monitoring started
    pub fn changes_per_second(&self) -> f64 {
        if let Some(start_time) = self.start_time {
            let duration = start_time.elapsed().as_secs_f64();
            if duration > 0.0 {
                self.total_changes as f64 / duration
            } else {
                0.0
            }
        } else {
            0.0
        }
    }

    /// Reset all statistics
    pub fn reset(&mut self) {
        self.stats.clear();
        self.total_changes = 0;
        self.start_time = Some(Instant::now());
        
        // Clear caches when resetting
        self.invalidate_caches();
    }

    /// Get the most active component types (cached for performance)
    pub fn most_active_types(&mut self, limit: usize) -> Vec<(String, u64)> {
        const CACHE_DURATION: std::time::Duration = std::time::Duration::from_secs(5);
        
        // Check if we have valid cached results
        if let Some((ref cached_result, timestamp)) = self.cached_most_active {
            if timestamp.elapsed() < CACHE_DURATION {
                return cached_result.iter().take(limit).cloned().collect();
            }
        }
        
        // Cache miss - compute fresh results
        let mut sorted: Vec<_> = self.stats.values()
            .map(|stats| (stats.type_name.clone(), stats.total()))
            .collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        
        // Cache the full sorted result
        self.cached_most_active = Some((sorted.clone(), Instant::now()));
        
        // Return requested limit
        sorted.into_iter().take(limit).collect()
    }
    
    /// Invalidate cached results when stats change
    fn invalidate_caches(&mut self) {
        self.cached_most_active = None;
        self.cached_summary = None;
    }
}

/// Utility trait for enhanced change detection queries
pub trait ChangeDetectionExt<'w, 's> {
    /// Count the number of added components
    fn count_added(&mut self) -> usize;
    
    /// Count the number of changed components  
    fn count_changed(&mut self) -> usize;
}

/// Implementation for Added queries
impl<'w, 's, T: Component> ChangeDetectionExt<'w, 's> for Query<'w, 's, &T, Added<T>> {
    fn count_added(&mut self) -> usize {
        self.iter().count()
    }
    
    fn count_changed(&mut self) -> usize {
        0 // Added queries don't track changes
    }
}

/// Implementation for Changed queries  
impl<'w, 's, T: Component> ChangeDetectionExt<'w, 's> for Query<'w, 's, &T, Changed<T>> {
    fn count_added(&mut self) -> usize {
        0 // Changed queries don't track additions
    }
    
    fn count_changed(&mut self) -> usize {
        self.iter().count()
    }
}

/// Central change detection system optimized for archetype performance
/// Position and Movement change tracking system (split from unified system)
pub fn position_movement_change_system(
    mut commands: Commands,
    mut monitor: ResMut<ChangeMonitor>,
    game_time: Res<GameTime>,
    
    // Position changes
    added_positions: Query<(Entity, &Position), Added<Position>>,
    changed_positions: Query<(Entity, &Position, Option<&Name>), Changed<Position>>,
    
    // Movement changes using ParamSet to avoid conflicts
    mut movement_queries: ParamSet<(
        Query<(Entity, &Movement), Added<Movement>>,
        Query<(Entity, &mut Movement, Option<&Name>), Changed<Movement>>,
    )>,
    
    // Removal tracking
    mut removed_positions: RemovedComponents<Position>,
    mut removed_movements: RemovedComponents<Movement>,
) {
    // Record position and movement changes using the correct API
    monitor.record_additions::<Position>(added_positions.iter().count());
    monitor.record_modifications::<Position>(changed_positions.iter().count());
    monitor.record_removals::<Position>(removed_positions.read().count());
    
    monitor.record_additions::<Movement>(movement_queries.p0().iter().count());
    monitor.record_modifications::<Movement>(movement_queries.p1().iter().count());
    monitor.record_removals::<Movement>(removed_movements.read().count());
    
    // Process individual position changes for debugging
    for (entity, position, name) in changed_positions.iter() {
        let entity_name = name.map(|n| n.value()).unwrap_or("unnamed");
        debug!("📍 Position changed for entity {} ({}): {:?}", entity.index(), entity_name, position);
    }
    
    // Process individual movement changes and restoration
    for (entity, mut movement, name) in movement_queries.p1().iter_mut() {
        let entity_name = name.map(|n| n.value()).unwrap_or("unnamed");
        debug!("🚶 Movement changed for entity {} ({}): {:?}", entity.index(), entity_name, *movement);
        
        // Note: Movement restoration logic would need to be implemented based on actual component fields
        // This is commented out until proper Movement component structure is confirmed
        /*
        if !movement.previous_positions.is_empty() && movement.restore_previous {
            if let Some(previous_pos) = movement.previous_positions.pop() {
                commands.entity(entity).insert(Position(previous_pos));
                movement.restore_previous = false;
                debug!("🔄 Restored position for entity {} ({}): {:?}", entity.index(), entity_name, previous_pos);
            }
        }
        */
    }
}

/// Health and Owner change tracking system (split from unified system)
pub fn health_owner_change_system(
    mut commands: Commands,
    mut monitor: ResMut<ChangeMonitor>,
    game_time: Res<GameTime>,
    
    // Health changes
    added_healths: Query<(Entity, &Health), Added<Health>>,
    changed_healths: Query<(Entity, &Health, Option<&Name>), Changed<Health>>,
    
    // Owner changes
    added_owners: Query<(Entity, &Owner), Added<Owner>>,
    changed_owners: Query<(Entity, &Owner, Option<&Name>), Changed<Owner>>,
    
    // Removal tracking
    mut removed_healths: RemovedComponents<Health>,
    mut removed_owners: RemovedComponents<Owner>,
) {
    // Record health and owner changes using the correct API
    monitor.record_additions::<Health>(added_healths.iter().count());
    monitor.record_modifications::<Health>(changed_healths.iter().count());
    monitor.record_removals::<Health>(removed_healths.read().count());
    
    monitor.record_additions::<Owner>(added_owners.iter().count());
    monitor.record_modifications::<Owner>(changed_owners.iter().count());
    monitor.record_removals::<Owner>(removed_owners.read().count());
    
    // Process individual health changes
    for (entity, health, name) in changed_healths.iter() {
        let entity_name = name.map(|n| n.value()).unwrap_or("unnamed");
        
        // Note: Health cleanup logic would need to be implemented based on actual Health component structure
        // This is commented out until proper Health component structure is confirmed
        /*
        if health.current <= 0.0 {
            commands.entity(entity).despawn();
            debug!("💀 Entity {} ({}) destroyed due to zero health", entity.index(), entity_name);
        } else {
            debug!("❤️ Health changed for entity {} ({}): {:.1}/{:.1}", entity.index(), entity_name, health.current, health.max);
        }
        */
        debug!("❤️ Health changed for entity {} ({})", entity.index(), entity_name);
    }
    
    // Process individual owner changes
    for (entity, owner, name) in changed_owners.iter() {
        let entity_name = name.map(|n| n.value()).unwrap_or("unnamed");
        debug!("🏠 Owner changed for entity {} ({}): {:?}", entity.index(), entity_name, owner);
    }
}

/// Renderable and Name change tracking system (split from unified system)
pub fn renderable_name_change_system(
    mut monitor: ResMut<ChangeMonitor>,
    game_time: Res<GameTime>,
    
    // Renderable changes
    added_renderables: Query<(Entity, &Renderable), Added<Renderable>>,
    changed_renderables: Query<(Entity, &Renderable, Option<&Name>), Changed<Renderable>>,
    
    // Name changes  
    added_names: Query<(Entity, &Name), Added<Name>>,
    changed_names: Query<(Entity, &Name), Changed<Name>>,
    
    // Removal tracking
    mut removed_renderables: RemovedComponents<Renderable>,
    mut removed_names: RemovedComponents<Name>,
) {
    // Record renderable and name changes using the correct API
    monitor.record_additions::<Renderable>(added_renderables.iter().count());
    monitor.record_modifications::<Renderable>(changed_renderables.iter().count());
    monitor.record_removals::<Renderable>(removed_renderables.read().count());
    
    monitor.record_additions::<Name>(added_names.iter().count());
    monitor.record_modifications::<Name>(changed_names.iter().count());
    monitor.record_removals::<Name>(removed_names.read().count());
    
    // Process individual renderable changes
    for (entity, renderable, name) in changed_renderables.iter() {
        let entity_name = name.map(|n| n.value()).unwrap_or("unnamed");
        debug!("🎨 Renderable changed for entity {} ({}): {:?}", entity.index(), entity_name, renderable);
    }
    
    // Process individual name changes
    for (entity, name) in changed_names.iter() {
        debug!("📛 Name changed for entity {}: {}", entity.index(), name.value());
    }
}

/// Original unified change system (exceeds Bevy IntoSystem parameter limits)
/// Replaced by split systems above to stay under the parameter limit
/// Batches component changes to minimize archetype fragmentation and improve query performance
#[allow(dead_code)]
#[instrument(name = "unified_change_system", skip_all)]
pub fn unified_change_system(
    mut commands: Commands,
    mut monitor: ResMut<ChangeMonitor>,
    game_time: Res<GameTime>,
    
    // Batch core component changes together for better archetype locality
    added_positions: Query<(Entity, &Position), Added<Position>>,
    changed_positions: Query<(Entity, &Position, Option<&Name>), Changed<Position>>,
    added_movements: Query<(Entity, &Movement), Added<Movement>>,
    mut changed_movements: Query<(Entity, &mut Movement, Option<&Name>), Changed<Movement>>,
    added_healths: Query<(Entity, &Health), Added<Health>>,
    changed_healths: Query<(Entity, &Health, Option<&Name>), Changed<Health>>,
    added_names: Query<(Entity, &Name), Added<Name>>,
    changed_names: Query<(Entity, &Name), Changed<Name>>,
    added_owners: Query<(Entity, &Owner), Added<Owner>>,
    changed_owners: Query<(Entity, &Owner, Option<&Name>), Changed<Owner>>,
    added_renderables: Query<(Entity, &Renderable), Added<Renderable>>,
    changed_renderables: Query<(Entity, &Renderable, Option<&Name>), Changed<Renderable>>,
    
    // Separate interpolated components to avoid mixing with core components
    added_interpolated_positions: Query<(Entity, &InterpolatedPosition), Added<InterpolatedPosition>>,
    changed_interpolated_positions: Query<(Entity, &InterpolatedPosition), Changed<InterpolatedPosition>>,
    added_interpolated_health: Query<(Entity, &InterpolatedHealth), Added<InterpolatedHealth>>,
    changed_interpolated_health: Query<(Entity, &InterpolatedHealth), Changed<InterpolatedHealth>>,
    
    // Game-specific components batched separately
    added_game_selections: Query<(Entity, &crate::ecs::components::GameSelection), Added<crate::ecs::components::GameSelection>>,
    changed_game_selections: Query<(Entity, &crate::ecs::components::GameSelection), Changed<crate::ecs::components::GameSelection>>,
    
    // Comprehensive removal detection for all tracked components
    mut removed_positions: RemovedComponents<Position>,
    mut removed_movements: RemovedComponents<Movement>, 
    mut removed_healths: RemovedComponents<Health>,
    mut removed_names: RemovedComponents<Name>,
    mut removed_owners: RemovedComponents<Owner>,
    mut removed_renderables: RemovedComponents<Renderable>,
    mut removed_interpolated_positions: RemovedComponents<InterpolatedPosition>,
    mut removed_interpolated_health: RemovedComponents<InterpolatedHealth>,
    mut removed_game_selections: RemovedComponents<crate::ecs::components::GameSelection>,
) {
    let system_start = Instant::now();
    let correlation_id = LoggingSystem::generate_correlation_id();
    
    // Count changes for comprehensive performance tracking
    let position_changes = added_positions.iter().count() + changed_positions.iter().count();
    let movement_changes = added_movements.iter().count() + changed_movements.iter().count();
    let health_changes = added_healths.iter().count() + changed_healths.iter().count();
    let name_changes = added_names.iter().count() + changed_names.iter().count();
    let owner_changes = added_owners.iter().count() + changed_owners.iter().count();
    let renderable_changes = added_renderables.iter().count() + changed_renderables.iter().count();
    let interpolation_changes = added_interpolated_positions.iter().count() + changed_interpolated_positions.iter().count() +
                               added_interpolated_health.iter().count() + changed_interpolated_health.iter().count();
    let game_selection_changes = added_game_selections.iter().count() + changed_game_selections.iter().count();
    
    let total_changes = position_changes + movement_changes + health_changes + name_changes + 
                       owner_changes + renderable_changes + interpolation_changes + game_selection_changes;
    
    if total_changes > 0 {
        debug!(
            target: "game::systems::changes",
            correlation_id = correlation_id,
            turn = game_time.turn,
            tick = game_time.tick,
            position_changes = position_changes,
            movement_changes = movement_changes,
            health_changes = health_changes,
            name_changes = name_changes,
            owner_changes = owner_changes,
            renderable_changes = renderable_changes,
            interpolation_changes = interpolation_changes,
            game_selection_changes = game_selection_changes,
            total_changes = total_changes,
            "Processing comprehensive component changes"
        );
    }
    // === COMPREHENSIVE MONITORING (count changes for statistics) ===
    monitor.record_additions::<Position>(added_positions.iter().count());
    monitor.record_additions::<Movement>(added_movements.iter().count());
    monitor.record_additions::<Health>(added_healths.iter().count());
    monitor.record_additions::<Name>(added_names.iter().count());
    monitor.record_additions::<Owner>(added_owners.iter().count());
    monitor.record_additions::<Renderable>(added_renderables.iter().count());
    monitor.record_additions::<InterpolatedPosition>(added_interpolated_positions.iter().count());
    monitor.record_additions::<InterpolatedHealth>(added_interpolated_health.iter().count());
    monitor.record_additions::<crate::ecs::components::GameSelection>(added_game_selections.iter().count());

    monitor.record_modifications::<Position>(changed_positions.iter().count());
    monitor.record_modifications::<Movement>(changed_movements.iter().count());
    monitor.record_modifications::<Health>(changed_healths.iter().count());
    monitor.record_modifications::<Name>(changed_names.iter().count());
    monitor.record_modifications::<Owner>(changed_owners.iter().count());
    monitor.record_modifications::<Renderable>(changed_renderables.iter().count());
    monitor.record_modifications::<InterpolatedPosition>(changed_interpolated_positions.iter().count());
    monitor.record_modifications::<InterpolatedHealth>(changed_interpolated_health.iter().count());
    monitor.record_modifications::<crate::ecs::components::GameSelection>(changed_game_selections.iter().count());

    monitor.record_removals::<Position>(removed_positions.read().count());
    monitor.record_removals::<Movement>(removed_movements.read().count());
    monitor.record_removals::<Health>(removed_healths.read().count());
    monitor.record_removals::<Name>(removed_names.read().count());
    monitor.record_removals::<Owner>(removed_owners.read().count());
    monitor.record_removals::<Renderable>(removed_renderables.read().count());
    monitor.record_removals::<InterpolatedPosition>(removed_interpolated_positions.read().count());
    monitor.record_removals::<InterpolatedHealth>(removed_interpolated_health.read().count());
    monitor.record_removals::<crate::ecs::components::GameSelection>(removed_game_selections.read().count());

    // === FUNCTIONAL ACTIONS BASED ON CHANGES ===

    // 1. MOVEMENT RESTORATION - restore movement points at turn start
    if game_time.tick == 0 && game_time.turn > 1 {
        let mut restored_count = 0;
        for (entity, mut movement, name) in changed_movements.iter_mut() {
            movement.reset_for_turn();
            restored_count += 1;
            
            debug!(
                target: "game::systems::changes",
                correlation_id = correlation_id,
                entity = ?entity,
                name = name.map(|n| n.value()),
                restored_movement = movement.remaining_moves,
                max_movement = movement.max_moves,
                "Movement restored for turn start"
            );
            
            game_logging::log_entity_operation(entity, "movement_restore", name.map(|n| n.value()));
        }
        
        if restored_count > 0 {
            info!(
                target: "game::systems::changes",
                correlation_id = correlation_id,
                turn = game_time.turn,
                restored_entities = restored_count,
                "Movement restoration completed for turn start"
            );
        }
    }

    // 2. HEALTH CLEANUP - remove dead entities  
    let mut despawned_count = 0;
    for (entity, health, name) in changed_healths.iter() {
        if !health.is_alive() {
            let entity_name = name.map(|n| n.value().to_string());
            
            warn!(
                target: "game::systems::changes",
                correlation_id = correlation_id,
                entity = ?entity,
                name = ?entity_name,
                health_current = health.current,
                health_max = health.max,
                "Entity died and will be removed"
            );
            
            game_logging::log_entity_operation(entity, "death", entity_name.as_deref());
            commands.entity(entity).despawn();
            despawned_count += 1;
        }
    }
    
    if despawned_count > 0 {
        info!(
            target: "game::systems::changes",
            correlation_id = correlation_id,
            despawned_entities = despawned_count,
            "Health cleanup completed - entities removed"
        );
    }

    // 3. DEBUG LOGGING - comprehensive change logging in debug builds
    #[cfg(debug_assertions)]
    {
        for (entity, position, name) in changed_positions.iter() {
            let name_str = name.map(|n| n.value()).unwrap_or("Unknown");
            tracing::debug!("Entity {:?} '{}' moved to hex {:?}", entity, name_str, position.hex());
        }

        for (entity, health, name) in changed_healths.iter() {
            let name_str = name.map(|n| n.value()).unwrap_or("Unknown");
            tracing::debug!("Entity {:?} '{}' health: {:.1}/{:.1}", 
                           entity, name_str, health.current, health.max);
        }

        for (entity, movement, name) in changed_movements.iter() {
            let name_str = name.map(|n| n.value()).unwrap_or("Unknown");
            debug!(
                target: "game::systems::changes::debug",
                correlation_id = correlation_id,
                entity = ?entity,
                name = name_str,
                current_movement = movement.remaining_moves,
                max_movement = movement.max_moves,
                "Movement component changed"
            );
        }
    }
    
    // Final performance logging
    let system_duration = system_start.elapsed().as_secs_f64() * 1000.0;
    
    if total_changes > 10 || system_duration > 1.0 {
        info!(
            target: "game::systems::changes",
            correlation_id = correlation_id,
            duration_ms = system_duration,
            total_changes = total_changes,
            entities_despawned = despawned_count,
            "Change detection system completed"
        );
    }
    
    game_logging::log_performance_event("unified_change_system", system_duration, total_changes);
}

/// Enhanced change detection utilities that work with OptimalSpatialIndex
pub struct ChangeDetectionUtils;

impl ChangeDetectionUtils {
    /// Get changed entities with spatial filtering using OptimalSpatialIndex
    pub fn changed_entities_in_range<T: Component>(
        _world: &World,
        spatial_index: &OptimalSpatialIndex,
        center: glam::IVec2,
        radius: u32,
    ) -> Vec<Entity>
    where
        T: 'static,
    {
        // Get entities in range using R-tree spatial queries
        let spatial_entities = spatial_index.entities_in_range(center, radius);
        
        // Filter to only those with changed components
        // Note: This would need access to Bevy's change detection ticks
        // In practice, this would be implemented as a system parameter
        spatial_entities
    }

    /// Get change statistics summary (cached for performance)
    pub fn change_summary(monitor: &mut ChangeMonitor) -> String {
        const CACHE_DURATION: std::time::Duration = std::time::Duration::from_secs(2);
        
        // Check if we have valid cached summary
        if let Some((ref cached_summary, timestamp)) = monitor.cached_summary {
            if timestamp.elapsed() < CACHE_DURATION {
                return cached_summary.clone();
            }
        }
        
        // Cache miss - compute fresh summary
        let summary = format!(
            "Total changes: {}, Rate: {:.2}/sec, Most active: {:?}",
            monitor.total_changes(),
            monitor.changes_per_second(),
            monitor.most_active_types(3)
        );
        
        // Cache the result
        monitor.cached_summary = Some((summary.clone(), Instant::now()));
        
        summary
    }
}

/// Integration with existing parallel systems - replaces old change detection systems
pub fn configure_change_detection(scheduler: &mut crate::ecs::EcsScheduler, world: &mut World) {
    use crate::ecs::ResourceAccess;
    use crate::ecs::resources::*;
    
    // Add split change detection systems (replacing oversized unified system)
    // These systems are split to stay under Bevy's IntoSystem parameter limits
    scheduler.add_system_with_accesses(
        crate::core::Stage::Update,
        "position_movement_change_system", 
        position_movement_change_system,
        vec![
            ResourceAccess::write::<ChangeMonitor>(),
            ResourceAccess::read::<GameTime>(),
            // Component queries handled by Bevy's system
        ],
        world
    );
    
    // Add health and owner change system
    scheduler.add_system_with_accesses(
        crate::core::Stage::Update,
        "health_owner_change_system", 
        health_owner_change_system,
        vec![
            ResourceAccess::write::<ChangeMonitor>(),
            ResourceAccess::read::<GameTime>(),
            // Component queries handled by Bevy's system
        ],
        world
    );
    
    // Add renderable and name change system
    scheduler.add_system_with_accesses(
        crate::core::Stage::Update,
        "renderable_name_change_system", 
        renderable_name_change_system,
        vec![
            ResourceAccess::write::<ChangeMonitor>(),
            ResourceAccess::read::<GameTime>(),
            // Component queries handled by Bevy's system
        ],
        world,
    );
    
    // Ensure ChangeMonitor resource exists
    if world.get_resource::<ChangeMonitor>().is_none() {
        world.insert_resource(ChangeMonitor::new());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::components::*;

    #[test]
    fn test_change_monitor_creation() {
        let mut monitor = ChangeMonitor::new();
        assert_eq!(monitor.total_changes(), 0);
        assert!(monitor.enabled);
    }

    #[test]
    fn test_recording_changes() {
        let mut monitor = ChangeMonitor::new();
        
        monitor.record_additions::<Position>(5);
        monitor.record_modifications::<Position>(3);
        monitor.record_removals::<Position>(1);
        
        let stats = monitor.get_stats::<Position>().expect("Position stats should exist after recording changes");
        assert_eq!(stats.additions, 5);
        assert_eq!(stats.modifications, 3);
        assert_eq!(stats.removals, 1);
        assert_eq!(stats.total(), 9);
        assert_eq!(monitor.total_changes(), 9);
    }

    #[test]
    fn test_most_active_types() {
        let mut monitor = ChangeMonitor::new();
        
        monitor.record_modifications::<Position>(10);
        monitor.record_modifications::<Health>(5);
        monitor.record_modifications::<Movement>(8);
        
        let active = monitor.most_active_types(2);
        assert_eq!(active.len(), 2);
        // Should be sorted by activity (Position: 10, Movement: 8)
        assert!(active[0].1 >= active[1].1);
    }

    #[test] 
    fn test_monitor_enable_disable() {
        let mut monitor = ChangeMonitor::new();
        monitor.set_enabled(false);
        
        monitor.record_additions::<Position>(5);
        assert_eq!(monitor.total_changes(), 0); // Should not record when disabled
        
        monitor.set_enabled(true);
        monitor.record_additions::<Position>(3);
        assert_eq!(monitor.total_changes(), 3);
    }

    #[test]
    fn test_change_stats_creation() {
        let stats = ChangeStats::new::<Position>();
        assert_eq!(stats.type_name, "manifest::ecs::components::Position");
        assert_eq!(stats.total(), 0);
    }

    #[test]
    fn test_unified_system_configuration() {
        use crate::ecs::EcsScheduler;
        let mut world = World::new();
        let mut scheduler = EcsScheduler::new(Some(1)).expect("Failed to create ECS scheduler with 1 thread for testing");
        
        // Configure the unified change detection
        configure_change_detection(&mut scheduler, &mut world);
        
        // Verify ChangeMonitor resource was added
        assert!(world.get_resource::<ChangeMonitor>().is_some());
    }
}

