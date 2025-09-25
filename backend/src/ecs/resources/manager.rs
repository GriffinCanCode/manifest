//! Thread-safe resource manager using parking_lot::RwLock
//!
//! Provides controlled access to global ECS resources with optimal
//! read/write performance and deadlock prevention.

use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use crate::core::hashing::{collections, FastHashMap};

/// Errors that can occur during resource operations
#[derive(Error, Debug)]
pub enum ResourceError {
    #[error("Resource of type {0} not found")]
    NotFound(String),
    #[error("Failed to cast resource to type {0}")]
    CastFailed(String),
    #[error("Resource of type {0} already exists")]
    AlreadyExists(String),
}

/// Thread-safe container for a single resource
#[derive(Clone)]
struct ResourceContainer {
    data: Arc<RwLock<Box<dyn Any + Send + Sync>>>,
    type_name: String,
}

impl ResourceContainer {
    fn new<T: Any + Send + Sync + 'static>(resource: T) -> Self {
        Self {
            data: Arc::new(RwLock::new(Box::new(resource))),
            type_name: std::any::type_name::<T>().to_string(),
        }
    }
    
    fn read<T: Any + 'static>(&self) -> Result<ResourceReadGuard<T>, ResourceError> {
        let guard = self.data.read();
        ResourceReadGuard::new(guard, &self.type_name)
    }
    
    fn write<T: Any + 'static>(&self) -> Result<ResourceWriteGuard<T>, ResourceError> {
        let guard = self.data.write();
        ResourceWriteGuard::new(guard, &self.type_name)
    }
}

/// Read-only guard for accessing resources safely
pub struct ResourceReadGuard<'a, T: 'static> {
    _guard: RwLockReadGuard<'a, Box<dyn Any + Send + Sync>>,
    data: *const T,
}

impl<'a, T: 'static> ResourceReadGuard<'a, T> {
    fn new(
        guard: RwLockReadGuard<'a, Box<dyn Any + Send + Sync>>,
        type_name: &str,
    ) -> Result<Self, ResourceError> {
        let data_ptr = guard
            .downcast_ref::<T>()
            .ok_or_else(|| ResourceError::CastFailed(type_name.to_string()))?
            as *const T;
            
        Ok(Self {
            _guard: guard,
            data: data_ptr,
        })
    }
}

impl<T: 'static> std::ops::Deref for ResourceReadGuard<'_, T> {
    type Target = T;
    
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.data }
    }
}

unsafe impl<T: Send + Sync + 'static> Send for ResourceReadGuard<'_, T> {}
unsafe impl<T: Send + Sync + 'static> Sync for ResourceReadGuard<'_, T> {}

/// Write guard for modifying resources safely
pub struct ResourceWriteGuard<'a, T: 'static> {
    _guard: RwLockWriteGuard<'a, Box<dyn Any + Send + Sync>>,
    data: *mut T,
}

impl<'a, T: 'static> ResourceWriteGuard<'a, T> {
    fn new(
        mut guard: RwLockWriteGuard<'a, Box<dyn Any + Send + Sync>>,
        type_name: &str,
    ) -> Result<Self, ResourceError> {
        let data_ptr = guard
            .downcast_mut::<T>()
            .ok_or_else(|| ResourceError::CastFailed(type_name.to_string()))?
            as *mut T;
            
        Ok(Self {
            _guard: guard,
            data: data_ptr,
        })
    }
}

impl<T: 'static> std::ops::Deref for ResourceWriteGuard<'_, T> {
    type Target = T;
    
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.data }
    }
}

impl<T: 'static> std::ops::DerefMut for ResourceWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.data }
    }
}

unsafe impl<T: Send + Sync + 'static> Send for ResourceWriteGuard<'_, T> {}
unsafe impl<T: Send + Sync + 'static> Sync for ResourceWriteGuard<'_, T> {}

/// Thread-safe resource manager
pub struct ResourceManager {
    resources: RwLock<FastHashMap<TypeId, ResourceContainer>>,
}

impl ResourceManager {
    /// Create a new empty resource manager
    pub fn new() -> Self {
        Self {
            resources: RwLock::new(collections::fast_hash_map()),
        }
    }
    
    /// Insert a resource, returning error if it already exists
    pub fn insert<T: Any + Send + Sync + 'static>(&self, resource: T) -> Result<(), ResourceError> {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();
        
        let mut resources = self.resources.write();
        
        if resources.contains_key(&type_id) {
            return Err(ResourceError::AlreadyExists(type_name.to_string()));
        }
        
        resources.insert(type_id, ResourceContainer::new(resource));
        Ok(())
    }
    
    /// Insert or replace a resource
    pub fn insert_or_replace<T: Any + Send + Sync + 'static>(&self, resource: T) {
        let type_id = TypeId::of::<T>();
        let mut resources = self.resources.write();
        resources.insert(type_id, ResourceContainer::new(resource));
    }
    
    /// Get read-only access to a resource
    pub fn read<T: Any + Send + Sync + 'static>(&self) -> Result<ResourceReadGuard<T>, ResourceError> {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();
        
        let container = {
            let resources = self.resources.read();
            resources
                .get(&type_id)
                .ok_or_else(|| ResourceError::NotFound(type_name.to_string()))?
                .clone()
        };
            
        container.read::<T>()
    }
    
    /// Get write access to a resource
    pub fn write<T: Any + Send + Sync + 'static>(&self) -> Result<ResourceWriteGuard<T>, ResourceError> {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();
        
        let container = {
            let resources = self.resources.read();
            resources
                .get(&type_id)
                .ok_or_else(|| ResourceError::NotFound(type_name.to_string()))?
                .clone()
        };
            
        container.write::<T>()
    }
    
    /// Check if a resource exists
    pub fn contains<T: Any>(&self) -> bool {
        let type_id = TypeId::of::<T>();
        let resources = self.resources.read();
        resources.contains_key(&type_id)
    }
    
    /// Remove a resource, returning it if it existed
    pub fn remove<T: Any + Send + Sync + 'static>(&self) -> Result<T, ResourceError> {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();
        
        let mut resources = self.resources.write();
        let container = resources
            .remove(&type_id)
            .ok_or_else(|| ResourceError::NotFound(type_name.to_string()))?;
            
        // Extract the resource from the container
        // Since we use Arc, we need to try to unwrap it
        let data = Arc::try_unwrap(container.data)
            .map_err(|_| ResourceError::CastFailed("Resource still in use".to_string()))?
            .into_inner();
        let resource = *data
            .downcast::<T>()
            .map_err(|_| ResourceError::CastFailed(type_name.to_string()))?;
            
        Ok(resource)
    }
    
    /// Get the number of stored resources
    pub fn len(&self) -> usize {
        self.resources.read().len()
    }
    
    /// Check if the manager is empty
    pub fn is_empty(&self) -> bool {
        self.resources.read().is_empty()
    }
    
    /// Clear all resources
    pub fn clear(&self) {
        self.resources.write().clear();
    }
}

impl Default for ResourceManager {
    fn default() -> Self {
        Self::new()
    }
}

// Thread safety markers
unsafe impl Send for ResourceManager {}
unsafe impl Sync for ResourceManager {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;
    
    #[derive(Debug, PartialEq)]
    struct TestResource {
        value: i32,
    }
    
    #[derive(Debug, PartialEq)]
    struct AnotherResource {
        name: String,
    }
    
    #[test]
    fn insert_and_read() {
        let manager = ResourceManager::new();
        let resource = TestResource { value: 42 };
        
        manager.insert(resource).unwrap();
        
        let read_guard = manager.read::<TestResource>().unwrap();
        assert_eq!(read_guard.value, 42);
    }
    
    #[test]
    fn insert_duplicate_fails() {
        let manager = ResourceManager::new();
        
        manager.insert(TestResource { value: 1 }).unwrap();
        let result = manager.insert(TestResource { value: 2 });
        
        assert!(matches!(result, Err(ResourceError::AlreadyExists(_))));
    }
    
    #[test]
    fn insert_or_replace() {
        let manager = ResourceManager::new();
        
        manager.insert_or_replace(TestResource { value: 1 });
        manager.insert_or_replace(TestResource { value: 2 });
        
        let read_guard = manager.read::<TestResource>().unwrap();
        assert_eq!(read_guard.value, 2);
    }
    
    #[test]
    fn write_access() {
        let manager = ResourceManager::new();
        manager.insert(TestResource { value: 10 }).unwrap();
        
        {
            let mut write_guard = manager.write::<TestResource>().unwrap();
            write_guard.value = 20;
        }
        
        let read_guard = manager.read::<TestResource>().unwrap();
        assert_eq!(read_guard.value, 20);
    }
    
    #[test]
    fn concurrent_reads() {
        let manager = Arc::new(ResourceManager::new());
        manager.insert(TestResource { value: 100 }).unwrap();
        
        let mut handles = vec![];
        
        // Spawn multiple readers
        for _ in 0..10 {
            let manager_clone = Arc::clone(&manager);
            let handle = thread::spawn(move || {
                let read_guard = manager_clone.read::<TestResource>().unwrap();
                read_guard.value
            });
            handles.push(handle);
        }
        
        // All reads should succeed
        for handle in handles {
            let value = handle.join().unwrap();
            assert_eq!(value, 100);
        }
    }
    
    #[test]
    fn multiple_resource_types() {
        let manager = ResourceManager::new();
        
        manager.insert(TestResource { value: 42 }).unwrap();
        manager.insert(AnotherResource { name: "test".to_string() }).unwrap();
        
        let test_read = manager.read::<TestResource>().unwrap();
        let another_read = manager.read::<AnotherResource>().unwrap();
        
        assert_eq!(test_read.value, 42);
        assert_eq!(another_read.name, "test");
    }
    
    #[test]
    fn resource_not_found() {
        let manager = ResourceManager::new();
        let result = manager.read::<TestResource>();
        
        assert!(matches!(result, Err(ResourceError::NotFound(_))));
    }
    
    #[test]
    fn contains_check() {
        let manager = ResourceManager::new();
        
        assert!(!manager.contains::<TestResource>());
        manager.insert(TestResource { value: 1 }).unwrap();
        assert!(manager.contains::<TestResource>());
    }
    
    #[test]
    fn remove_resource() {
        let manager = ResourceManager::new();
        let original = TestResource { value: 99 };
        
        manager.insert(original).unwrap();
        assert!(manager.contains::<TestResource>());
        
        let removed = manager.remove::<TestResource>().unwrap();
        assert_eq!(removed.value, 99);
        assert!(!manager.contains::<TestResource>());
    }
    
    #[test]
    fn clear_all() {
        let manager = ResourceManager::new();
        
        manager.insert(TestResource { value: 1 }).unwrap();
        manager.insert(AnotherResource { name: "test".to_string() }).unwrap();
        
        assert_eq!(manager.len(), 2);
        
        manager.clear();
        
        assert_eq!(manager.len(), 0);
        assert!(manager.is_empty());
    }
}
