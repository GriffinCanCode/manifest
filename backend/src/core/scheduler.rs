//! General-purpose parallel task scheduler using Rayon
//!
//! Provides thread-safe, parallel execution of tasks with dependencies,
//! batching, and resource management. Designed to be used across the
//! entire system - ECS, AI, procedural generation, etc.

use crossbeam::channel::{unbounded, Receiver, Sender};
use parking_lot::{Mutex, RwLock};
use rayon::{ThreadPool, ThreadPoolBuilder};
use std::{
    any::TypeId,
    collections::HashMap,
    fmt::Debug,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Instant,
};
use bevy_ecs::system::Resource as BevyResource;
use thiserror::Error;

/// Errors that can occur during scheduling and execution
#[derive(Error, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SchedulerError {
    #[error("Task failed to execute: {0}")]
    TaskFailed(String),
    #[error("Dependency cycle detected")]
    CyclicDependency,
    #[error("Resource conflict: {resource}")]
    ResourceConflict { resource: String },
    #[error("Thread pool error: {0}")]
    ThreadPool(String),
}

/// Unique identifier for tasks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(u64);

impl TaskId {
    fn new() -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed) as u64)
    }
}

/// Resource access type for dependency tracking
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Access {
    Read,
    Write,
}

/// Resource requirement for tasks
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Resource {
    pub type_id: TypeId,
    pub name: String,
    pub access: Access,
}

impl Resource {
    pub fn read<T: 'static>() -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            name: std::any::type_name::<T>().to_string(),
            access: Access::Read,
        }
    }

    pub fn write<T: 'static>() -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            name: std::any::type_name::<T>().to_string(),
            access: Access::Write,
        }
    }
}

/// Task metadata and execution info
struct TaskInfo {
    id: TaskId,
    name: String,
    dependencies: Vec<TaskId>,
    resources: Vec<Resource>,
    task: Box<dyn FnOnce() -> Result<(), SchedulerError> + Send + Sync>,
    priority: i32,
}

/// Scheduling stage for organizing task execution
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum Stage {
    PreUpdate,
    Update,
    PostUpdate,
    Gameplay,
    WorldGeneration,
    Late,
    Cleanup,
}

/// Batch of tasks to execute together
pub struct TaskBatch {
    stage: Stage,
    tasks: Vec<TaskInfo>,
}

impl TaskBatch {
    pub fn new(stage: Stage) -> Self {
        Self {
            stage,
            tasks: Vec::new(),
        }
    }

    /// Add a task to this batch
    pub fn add_task<F>(&mut self, name: impl Into<String>, task: F) -> TaskId
    where
        F: FnOnce() -> Result<(), SchedulerError> + Send + Sync + 'static,
    {
        let id = TaskId::new();
        self.tasks.push(TaskInfo {
            id,
            name: name.into(),
            dependencies: Vec::new(),
            resources: Vec::new(),
            task: Box::new(task),
            priority: 0,
        });
        id
    }

    /// Add a task with resource requirements
    pub fn add_task_with_resources<F>(
        &mut self,
        name: impl Into<String>,
        resources: Vec<Resource>,
        task: F,
    ) -> TaskId
    where
        F: FnOnce() -> Result<(), SchedulerError> + Send + Sync + 'static,
    {
        let id = TaskId::new();
        self.tasks.push(TaskInfo {
            id,
            name: name.into(),
            dependencies: Vec::new(),
            resources,
            task: Box::new(task),
            priority: 0,
        });
        id
    }
}

/// Thread-safe parallel task scheduler
#[derive(BevyResource)]
pub struct Scheduler {
    thread_pool: ThreadPool,
    batches: Arc<RwLock<HashMap<Stage, TaskBatch>>>,
    active_tasks: Arc<AtomicUsize>,
    total_tasks: Arc<AtomicUsize>,
    error_sender: Sender<SchedulerError>,
    error_receiver: Receiver<SchedulerError>,
    metrics: Arc<Mutex<SchedulerMetrics>>,
}

impl Debug for Scheduler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Scheduler")
            .field("thread_count", &self.thread_pool.current_num_threads())
            .field("active_tasks", &self.active_tasks.load(Ordering::Relaxed))
            .field("total_tasks", &self.total_tasks.load(Ordering::Relaxed))
            .field("batches_count", &self.batches.read().len())
            .finish()
    }
}

/// Performance metrics for the scheduler
#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct SchedulerMetrics {
    pub tasks_executed: u64,
    /// Total execution time in milliseconds
    pub total_execution_time_ms: u64,
    /// Average task time in milliseconds  
    pub average_task_time_ms: u64,
    pub parallel_efficiency: f64,
    /// Last frame time in milliseconds
    pub last_frame_time_ms: u64,
}

impl Scheduler {
    /// Create a new scheduler with specified thread count
    pub fn new(threads: Option<usize>) -> Result<Self, SchedulerError> {
        let pool = match threads {
            Some(n) => ThreadPoolBuilder::new()
                .num_threads(n)
                .thread_name(|i| format!("manifest-worker-{}", i))
                .build()
                .map_err(|e| SchedulerError::ThreadPool(e.to_string()))?,
            None => ThreadPoolBuilder::new()
                .thread_name(|i| format!("manifest-worker-{}", i))
                .build()
                .map_err(|e| SchedulerError::ThreadPool(e.to_string()))?,
        };

        let (error_sender, error_receiver) = unbounded();

        Ok(Self {
            thread_pool: pool,
            batches: Arc::new(RwLock::new(HashMap::new())),
            active_tasks: Arc::new(AtomicUsize::new(0)),
            total_tasks: Arc::new(AtomicUsize::new(0)),
            error_sender,
            error_receiver,
            metrics: Arc::new(Mutex::new(SchedulerMetrics::default())),
        })
    }

    /// Add a batch of tasks for a specific stage
    pub fn add_batch(&self, batch: TaskBatch) {
        let mut batches = self.batches.write();
        batches.insert(batch.stage.clone(), batch);
    }

    /// Execute all tasks in a specific stage
    pub fn run_stage(&self, stage: Stage) -> Result<(), Vec<SchedulerError>> {
        let start_time = Instant::now();
        let batch = {
            let batches = self.batches.read();
            match batches.get(&stage) {
                Some(batch) => {
                    // Extract tasks safely - we'll need to reconstruct the batch
                    let task_count = batch.tasks.len();
                    self.total_tasks.store(task_count, Ordering::Relaxed);
                    task_count
                }
                None => return Ok(()), // No tasks for this stage
            }
        };

        if batch == 0 {
            return Ok(());
        }

        // Collect errors during execution
        let errors = Arc::new(Mutex::new(Vec::new()));
        let completed = Arc::new(AtomicUsize::new(0));

        // Extract tasks from batch for parallel execution
        let all_tasks = {
            let mut batches = self.batches.write();
            if let Some(batch) = batches.get_mut(&stage) {
                std::mem::take(&mut batch.tasks)
            } else {
                Vec::new()
            }
        };

        if all_tasks.is_empty() {
            return Ok(());
        }

        // Group tasks for parallel execution
        let task_groups = self.group_tasks_by_compatibility(all_tasks).map_err(|e| vec![e])?;
        
        // Execute each group of compatible tasks in parallel
        for group in task_groups {
            let errors_clone = errors.clone();
            let completed_clone = completed.clone();
            let active_tasks = self.active_tasks.clone();
            
            self.thread_pool.scope(|scope| {
                for task_info in group {
                    let errors_ref = errors_clone.clone();
                    let completed_ref = completed_clone.clone();
                    let active_ref = active_tasks.clone();
                    
                    scope.spawn(move |_| {
                        active_ref.fetch_add(1, Ordering::Relaxed);
                        
                        // Execute the actual task
                        let result = (task_info.task)();
                        
                        if let Err(e) = result {
                            errors_ref.lock().push(e);
                        }
                        
                        completed_ref.fetch_add(1, Ordering::Relaxed);
                        active_ref.fetch_sub(1, Ordering::Relaxed);
                    });
                }
            });
        }

        // Wait for completion and collect metrics
        let execution_time = start_time.elapsed();
        {
            let mut metrics = self.metrics.lock();
            metrics.tasks_executed += completed.load(Ordering::Relaxed) as u64;
            metrics.last_frame_time_ms = execution_time.as_millis() as u64;
            metrics.total_execution_time_ms += execution_time.as_millis() as u64;
            
            if metrics.tasks_executed > 0 {
                metrics.average_task_time_ms = 
                    metrics.total_execution_time_ms / metrics.tasks_executed;
            }
        }

        let final_errors = errors.lock().clone();
        if final_errors.is_empty() {
            Ok(())
        } else {
            Err(final_errors)
        }
    }

    /// Group tasks that can run in parallel (no resource conflicts) - reference version
    fn group_compatible_tasks<'a>(&self, tasks: &'a [TaskInfo]) -> Result<Vec<Vec<&'a TaskInfo>>, SchedulerError> {
        // Use shared conflict detection to get indices, then map to references
        let indices = crate::core::conflict_detection::group_by_resource_compatibility(
            tasks,
            |task| &task.resources
        )?;
        
        let groups = indices
            .into_iter()
            .map(|group_indices| {
                group_indices.into_iter().map(|i| &tasks[i]).collect()
            })
            .collect();
        
        Ok(groups)
    }

    /// Group tasks by compatibility (owned version for actual execution)
    fn group_tasks_by_compatibility(&self, tasks: Vec<TaskInfo>) -> Result<Vec<Vec<TaskInfo>>, SchedulerError> {
        // Use shared conflict detection to get indices, then map to owned tasks
        let indices = crate::core::conflict_detection::group_by_resource_compatibility(
            &tasks,
            |task| &task.resources
        )?;
        
        let mut task_vec = tasks;
        let mut groups = Vec::new();
        
        // Process groups in reverse order to avoid index invalidation
        for group_indices in indices.into_iter().rev() {
            let mut group = Vec::new();
            // Extract tasks in reverse order to avoid index issues
            for i in group_indices.into_iter().rev() {
                if i < task_vec.len() {
                    group.push(task_vec.remove(i));
                }
            }
            group.reverse(); // Restore original order
            if !group.is_empty() {
                groups.push(group);
            }
        }
        
        groups.reverse(); // Restore original group order
        Ok(groups)
    }

    /// Get current scheduler metrics
    pub fn metrics(&self) -> SchedulerMetrics {
        self.metrics.lock().clone()
    }

    /// Get number of active tasks
    pub fn active_count(&self) -> usize {
        self.active_tasks.load(Ordering::Relaxed)
    }

    /// Check if scheduler is currently running tasks
    pub fn is_busy(&self) -> bool {
        self.active_count() > 0
    }

    /// Clear all batches (for testing/reset)
    pub fn clear(&self) {
        self.batches.write().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheduler_creation() {
        let scheduler = Scheduler::new(Some(4)).unwrap();
        assert_eq!(scheduler.active_count(), 0);
        assert!(!scheduler.is_busy());
    }

    #[test]
    fn task_batch_creation() {
        let mut batch = TaskBatch::new(Stage::Update);
        let _task_id = batch.add_task("test_task", || Ok(()));
        assert_eq!(batch.tasks.len(), 1);
    }

    #[test]
    fn resource_conflict_detection() {
        let scheduler = Scheduler::new(Some(2)).unwrap();
        
        // Create tasks with conflicting resources
        let task1 = TaskInfo {
            id: TaskId::new(),
            name: "task1".to_string(),
            dependencies: Vec::new(),
            resources: vec![Resource::write::<u32>()],
            task: Box::new(|| Ok(())),
            priority: 0,
        };
        
        let task2 = TaskInfo {
            id: TaskId::new(),
            name: "task2".to_string(),
            dependencies: Vec::new(),
            resources: vec![Resource::read::<u32>()],
            task: Box::new(|| Ok(())),
            priority: 0,
        };
        
        let tasks = [task1, task2];
        let groups = scheduler.group_compatible_tasks(&tasks).unwrap();
        assert_eq!(groups.len(), 2); // Should be in separate groups due to conflict
    }

    #[test]
    fn parallel_compatible_tasks() {
        let scheduler = Scheduler::new(Some(2)).unwrap();
        
        let task1 = TaskInfo {
            id: TaskId::new(),
            name: "task1".to_string(),
            dependencies: Vec::new(),
            resources: vec![Resource::read::<u32>()],
            task: Box::new(|| Ok(())),
            priority: 0,
        };
        
        let task2 = TaskInfo {
            id: TaskId::new(),
            name: "task2".to_string(),
            dependencies: Vec::new(),
            resources: vec![Resource::read::<u32>()],
            task: Box::new(|| Ok(())),
            priority: 0,
        };
        
        let tasks = [task1, task2];
        let groups = scheduler.group_compatible_tasks(&tasks).unwrap();
        assert_eq!(groups.len(), 1); // Should be in same group - both read-only
        assert_eq!(groups[0].len(), 2);
    }

    #[test]
    fn metrics_tracking() {
        let scheduler = Scheduler::new(Some(2)).unwrap();
        let metrics = scheduler.metrics();
        assert_eq!(metrics.tasks_executed, 0);
        assert_eq!(metrics.total_execution_time_ms, 0);
    }
}
