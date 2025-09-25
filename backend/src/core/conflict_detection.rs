//! Shared resource conflict detection utilities
//!
//! This module provides algorithms for detecting resource conflicts and grouping
//! tasks/systems that can run safely in parallel. Used by both the core scheduler
//! and ECS scheduler to avoid code duplication.

use std::any::TypeId;
use crate::core::{Access, SchedulerError};
use crate::core::hashing::{collections, FastHashMap};

/// Generic resource access pattern for conflict detection
pub trait ResourceAccess {
    fn type_id(&self) -> TypeId;
    fn access(&self) -> Access;
}

/// Implementation for core scheduler resources
impl ResourceAccess for crate::core::Resource {
    fn type_id(&self) -> TypeId {
        self.type_id
    }
    
    fn access(&self) -> Access {
        self.access.clone()
    }
}

/// Implementation for ECS resource accesses
impl ResourceAccess for crate::ecs::ResourceAccess {
    fn type_id(&self) -> TypeId {
        self.type_id
    }
    
    fn access(&self) -> Access {
        self.access.clone()
    }
}

/// Group items by resource compatibility - items in the same group can run in parallel
pub fn group_by_resource_compatibility<T, R>(
    items: &[T],
    get_resources: impl Fn(&T) -> &[R]
) -> Result<Vec<Vec<usize>>, SchedulerError>
where
    R: ResourceAccess,
{
    let mut groups = Vec::new();
    let mut remaining: Vec<usize> = (0..items.len()).collect();
    
    while !remaining.is_empty() {
        let mut current_group = Vec::new();
        let mut used_resources = collections::fast_hash_map();
        
        remaining.retain(|&i| {
            let item_resources = get_resources(&items[i]);
            let conflicts = item_resources.iter().any(|resource| {
                match used_resources.get(&resource.type_id()) {
                    Some(Access::Write) => true, // Write conflicts with everything
                    Some(Access::Read) => resource.access() == Access::Write, // Read conflicts with write
                    None => false, // No conflict
                }
            });
            
            if !conflicts {
                // Track resource usage for this group
                for resource in item_resources {
                    match resource.access() {
                        Access::Write => {
                            used_resources.insert(resource.type_id(), Access::Write);
                        }
                        Access::Read => {
                            used_resources.entry(resource.type_id()).or_insert(Access::Read);
                        }
                    }
                }
                current_group.push(i);
                false // Remove from remaining
            } else {
                true // Keep for next group
            }
        });
        
        if current_group.is_empty() && !remaining.is_empty() {
            // Safety: take at least one item to avoid infinite loop
            current_group.push(remaining.remove(0));
        }
        
        if !current_group.is_empty() {
            groups.push(current_group);
        }
    }
    
    Ok(groups)
}

/// Check if two resource access patterns conflict
pub fn resources_conflict<R1, R2>(resources1: &[R1], resources2: &[R2]) -> bool
where
    R1: ResourceAccess,
    R2: ResourceAccess,
{
    for r1 in resources1 {
        for r2 in resources2 {
            if r1.type_id() == r2.type_id() {
                // Same resource type - check if access patterns conflict
                match (r1.access(), r2.access()) {
                    (Access::Write, _) | (_, Access::Write) => return true, // Any write conflicts
                    (Access::Read, Access::Read) => continue, // Read-read is fine
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Resource;

    #[test]
    fn test_conflict_detection() {
        let write_resource = Resource::write::<u32>();
        let read_resource = Resource::read::<u32>();
        let other_resource = Resource::read::<i32>();
        
        // Write conflicts with read of same type
        assert!(resources_conflict(&[write_resource.clone()], &[read_resource.clone()]));
        
        // Read doesn't conflict with read of same type
        assert!(!resources_conflict(&[read_resource.clone()], &[read_resource.clone()]));
        
        // Different types don't conflict
        assert!(!resources_conflict(&[write_resource], &[other_resource]));
    }
    
    #[test]
    fn test_grouping() {
        struct TestTask {
            resources: Vec<Resource>,
        }
        
        let tasks = vec![
            TestTask { resources: vec![Resource::read::<u32>()] },
            TestTask { resources: vec![Resource::read::<u32>()] },
            TestTask { resources: vec![Resource::write::<u32>()] },
            TestTask { resources: vec![Resource::read::<i32>()] },
        ];
        
        let groups = group_by_resource_compatibility(&tasks, |task| &task.resources).unwrap();
        
        // Tasks 0, 1, 3 should be in first group (no conflicts)
        // Task 2 should be in second group (write conflicts)
        assert_eq!(groups.len(), 2);
        assert!(groups[0].contains(&0));
        assert!(groups[0].contains(&1));
        assert!(groups[0].contains(&3));
        assert!(groups[1].contains(&2));
    }
}
