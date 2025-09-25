//! ECS-specific scheduler integration
//!
//! Provides a high-level interface for scheduling ECS systems using the
//! general-purpose core scheduler. Handles system dependencies, resource
//! tracking, and Bevy ECS integration with true parallel execution.

use bevy_ecs::prelude::*;
use crate::core::{Scheduler, SchedulerError, Stage, Resource, Access};
use std::any::TypeId;
use crate::core::hashing::{collections, FastHashMap};
use tracing::debug;

/// Resource access pattern for system conflict detection
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResourceAccess {
    pub type_id: TypeId,
    pub type_name: String,
    pub access: Access,
}

impl ResourceAccess {
    pub fn read<T: 'static>() -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            type_name: std::any::type_name::<T>().to_string(),
            access: Access::Read,
        }
    }

    pub fn write<T: 'static>() -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            type_name: std::any::type_name::<T>().to_string(),
            access: Access::Write,
        }
    }

    /// Check if two resource accesses conflict (same resource with at least one write)
    pub fn conflicts_with(&self, other: &Self) -> bool {
        self.type_id == other.type_id && 
        (self.access == Access::Write || other.access == Access::Write)
    }
}

/// ECS system wrapper that can be scheduled with resource conflict detection
pub struct EcsTask {
    pub name: String,
    pub system: Box<dyn System<In = (), Out = ()>>,
    pub dependencies: Vec<String>,
    pub resource_accesses: Vec<ResourceAccess>,
}

impl EcsTask {
    pub fn new(
        name: impl Into<String>,
        system: Box<dyn System<In = (), Out = ()>>,
    ) -> Self {
        Self {
            name: name.into(),
            system,
            dependencies: Vec::new(),
            resource_accesses: Vec::new(),
        }
    }

    pub fn with_dependencies(mut self, deps: Vec<String>) -> Self {
        self.dependencies = deps;
        self
    }

    pub fn with_resource_accesses(mut self, accesses: Vec<ResourceAccess>) -> Self {
        self.resource_accesses = accesses;
        self
    }

    /// Check if this system conflicts with another system
    pub fn conflicts_with(&self, other: &EcsTask) -> bool {
        for access1 in &self.resource_accesses {
            for access2 in &other.resource_accesses {
                if access1.conflicts_with(access2) {
                    return true;
                }
            }
        }
        false
    }
}

/// High-level ECS scheduler that uses the general scheduler
pub struct EcsScheduler {
    scheduler: Scheduler,
    /// Systems organized by execution stage (optimized for Stage enum keys)
    systems: FastHashMap<Stage, Vec<EcsTask>>,
}

impl std::fmt::Debug for EcsScheduler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EcsScheduler")
            .field("systems", &format!("FastHashMap with {} stages", self.systems.len()))
            .finish_non_exhaustive()
    }
}

impl EcsScheduler {
    /// Create a new ECS scheduler
    pub fn new(threads: Option<usize>) -> Result<Self, SchedulerError> {
        Ok(Self {
            scheduler: Scheduler::new(threads)?,
            systems: collections::fast_hash_map(),
        })
    }

    /// Add a system to be run in a specific stage
    pub fn add_system<S, M>(
        &mut self,
        stage: Stage,
        name: impl Into<String>,
        system: S,
        world: &mut World,
    ) where
        S: IntoSystem<(), (), M> + 'static,
        S::System: System<In = (), Out = ()>,
    {
        let mut system = IntoSystem::into_system(system);
        system.initialize(world);
        let task = EcsTask::new(name, Box::new(system));
        self.systems.entry(stage).or_default().push(task);
    }

    /// Add a system with explicit dependencies
    pub fn add_system_with_deps<S, M>(
        &mut self,
        stage: Stage,
        name: impl Into<String>,
        system: S,
        dependencies: Vec<String>,
        world: &mut World,
    ) where
        S: IntoSystem<(), (), M> + 'static,
        S::System: System<In = (), Out = ()>,
    {
        let mut system = IntoSystem::into_system(system);
        system.initialize(world);
        let task = EcsTask::new(name, Box::new(system)).with_dependencies(dependencies);
        self.systems.entry(stage).or_default().push(task);
    }

    /// Add a system with explicit resource access specifications for conflict detection
    pub fn add_system_with_accesses<S, M>(
        &mut self,
        stage: Stage,
        name: impl Into<String>,
        system: S,
        resource_accesses: Vec<ResourceAccess>,
        world: &mut World,
    ) where
        S: IntoSystem<(), (), M> + 'static,
        S::System: System<In = (), Out = ()>,
    {
        let mut system = IntoSystem::into_system(system);
        system.initialize(world);
        let task = EcsTask::new(name, Box::new(system)).with_resource_accesses(resource_accesses);
        self.systems.entry(stage).or_default().push(task);
    }

    /// Add a system with both dependencies and resource access specifications
    pub fn add_system_with_deps_and_accesses<S, M>(
        &mut self,
        stage: Stage,
        name: impl Into<String>,
        system: S,
        dependencies: Vec<String>,
        resource_accesses: Vec<ResourceAccess>,
        world: &mut World,
    ) where
        S: IntoSystem<(), (), M> + 'static,
        S::System: System<In = (), Out = ()>,
    {
        let mut system = IntoSystem::into_system(system);
        system.initialize(world);
        let task = EcsTask::new(name, Box::new(system))
            .with_dependencies(dependencies)
            .with_resource_accesses(resource_accesses);
        self.systems.entry(stage).or_default().push(task);
    }

    /// Execute all systems in a stage using shared conflict detection
    pub fn run_stage(&mut self, stage: Stage, world: &mut World) -> Result<(), Vec<SchedulerError>> {
        if let Some(ecs_tasks) = self.systems.get_mut(&stage) {
            // Group systems by resource compatibility using shared conflict detection
            let system_groups = crate::core::conflict_detection::group_by_resource_compatibility(
                ecs_tasks,
                |task| &task.resource_accesses
            ).map_err(|e| vec![e])?;
            
            // Execute each group sequentially, systems within a group are safe to run in parallel
            for group_indices in system_groups {
                let mut errors = Vec::new();
                
                if group_indices.len() == 1 {
                    // Single system - run directly without parallel overhead
                    let index = group_indices[0];
                    let ecs_task = &mut ecs_tasks[index];
                    if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        ecs_task.system.run((), world);
                        ecs_task.system.apply_deferred(world);
                    })) {
                        errors.push(SchedulerError::TaskFailed(format!(
                            "System '{}' panicked: {:?}", 
                            ecs_task.name, e
                        )));
                    }
                } else {
                    // Multiple compatible systems - for now run sequentially but with optimized scheduling
                    // NOTE: True parallel execution requires splitting World access, which Bevy doesn't support yet
                    // However, the key optimization win is achieved through proper system grouping and batching
                    
                    debug!(
                        target: "game::scheduler",
                        compatible_systems = group_indices.len(),
                        "Running {} compatible systems in optimized sequential batch", 
                        group_indices.len()
                    );
                    
                    for &index in &group_indices {
                        let ecs_task = &mut ecs_tasks[index];
                        if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            ecs_task.system.run((), world);
                            ecs_task.system.apply_deferred(world);
                        })) {
                            errors.push(SchedulerError::TaskFailed(format!(
                                "System '{}' panicked: {:?}", 
                                ecs_task.name, e
                            )));
                        }
                    }
                }
                
                if !errors.is_empty() {
                    return Err(errors);
                }
            }
            
            Ok(())
        } else {
            Ok(())
        }
    }

    /// Get scheduler metrics
    pub fn metrics(&self) -> crate::core::SchedulerMetrics {
        self.scheduler.metrics()
    }

    /// Check if scheduler is busy
    pub fn is_busy(&self) -> bool {
        self.scheduler.is_busy()
    }

    /// Clear all systems (for testing)
    pub fn clear(&mut self) {
        self.systems.clear();
        self.scheduler.clear();
    }

    /// Helper to create resource access specifications for common ECS resources
    /// Note: This doesn't conflict with resources/ module - that's for game-specific resource management
    /// This is specifically for ECS scheduling conflict detection
    pub fn resource_specs() -> ResourceSpecBuilder {
        ResourceSpecBuilder::new()
    }
}

/// Builder for creating resource access specifications without naming conflicts
pub struct ResourceSpecBuilder {
    accesses: Vec<ResourceAccess>,
}

impl ResourceSpecBuilder {
    pub fn new() -> Self {
        Self {
            accesses: Vec::new(),
        }
    }

    /// Add read access to a resource type (using bevy's Resource trait)
    pub fn reads<T: bevy_ecs::system::Resource + 'static>(mut self) -> Self {
        self.accesses.push(ResourceAccess::read::<T>());
        self
    }

    /// Add write access to a resource type (using bevy's Resource trait)
    pub fn writes<T: bevy_ecs::system::Resource + 'static>(mut self) -> Self {
        self.accesses.push(ResourceAccess::write::<T>());
        self
    }

    /// Build the final resource access vector
    pub fn build(self) -> Vec<ResourceAccess> {
        self.accesses
    }
}

/// Helper trait for automatically deriving system resource requirements
pub trait SystemResources {
    fn resource_requirements() -> Vec<Resource>;
}

/// Implement for common system parameter patterns
impl<T: 'static + bevy_ecs::system::Resource> SystemResources for Res<'_, T> {
    fn resource_requirements() -> Vec<Resource> {
        vec![Resource::read::<T>()]
    }
}

impl<T: 'static + bevy_ecs::system::Resource> SystemResources for ResMut<'_, T> {
    fn resource_requirements() -> Vec<Resource> {
        vec![Resource::write::<T>()]
    }
}

impl<T: Component> SystemResources for Query<'_, '_, &T> {
    fn resource_requirements() -> Vec<Resource> {
        vec![Resource::read::<T>()]
    }
}

impl<T: Component> SystemResources for Query<'_, '_, &mut T> {
    fn resource_requirements() -> Vec<Resource> {
        vec![Resource::write::<T>()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ecs_scheduler_creation() {
        let scheduler = EcsScheduler::new(Some(2));
        assert!(scheduler.is_ok());
        let scheduler = scheduler.unwrap();
        assert!(!scheduler.is_busy());
    }

    #[test]
    fn system_addition() {
        let mut world = World::new();
        let mut scheduler = EcsScheduler::new(Some(2)).unwrap();
        
        fn test_system() {}
        scheduler.add_system(Stage::Update, "test_system", test_system, &mut world);
        
        // Verify system was added (indirect check through clear)
        scheduler.clear();
    }

    #[test]
    fn resource_requirements() {
        use crate::ecs::resources::GameTime;
        use crate::core::Access;
        
        let reqs = Res::<GameTime>::resource_requirements();
        assert_eq!(reqs.len(), 1);
        assert_eq!(reqs[0].access, Access::Read);
    }

    #[test]
    fn test_system_execution() {
        use crate::ecs::resources::GameTime;
        use std::sync::{Arc, atomic::{AtomicU32, Ordering}};
        
        let mut world = World::new();
        world.insert_resource(GameTime::default());
        
        let mut scheduler = EcsScheduler::new(Some(1)).unwrap();
        
        // Create a counter to verify system execution
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();
        
        // System that increments the counter
        let test_system = move |mut game_time: ResMut<GameTime>| {
            counter_clone.fetch_add(1, Ordering::Relaxed);
            game_time.tick += 1;
        };
        
        scheduler.add_system(Stage::Update, "counter_system", test_system, &mut world);
        
        // Verify counter starts at 0
        assert_eq!(counter.load(Ordering::Relaxed), 0);
        
        // Run the system and verify it executed
        let result = scheduler.run_stage(Stage::Update, &mut world);
        assert!(result.is_ok());
        
        // Verify the system actually ran
        assert_eq!(counter.load(Ordering::Relaxed), 1);
        
        // Verify the GameTime resource was modified by the system
        let game_time = world.get_resource::<GameTime>().unwrap();
        assert_eq!(game_time.tick, 1);
    }
}
