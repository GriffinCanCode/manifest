//! Hot reload manager following existing manager patterns
//!
//! Single responsibility: watch files and trigger reloads via handlers.

use super::types::{ReloadError, ReloadResult, ReloadEvent, FileInfo, FileType, ReloadHandler};
use crossbeam::channel::{unbounded, Receiver, Sender};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::{Mutex, RwLock};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};
use tracing::{debug, error, info, warn};

/// Statistics about the reload manager
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct ReloadStats {
    pub files_watched: usize,
    pub handlers_registered: usize,
    pub reloads_attempted: u64,
    pub reloads_successful: u64,
    pub is_running: bool,
}

/// Hot reload manager
pub struct ReloadManager {
    /// File watcher
    _watcher: RecommendedWatcher,
    /// Files being tracked
    files: Arc<RwLock<HashMap<PathBuf, FileInfo>>>,
    /// Registered handlers  
    handlers: Arc<Mutex<Vec<Box<dyn ReloadHandler>>>>,
    /// Event channels
    file_events: Receiver<notify::Result<Event>>,
    reload_events: (Sender<ReloadEvent>, Receiver<ReloadEvent>),
    /// Control state
    running: Arc<AtomicBool>,
    /// Statistics
    stats: Arc<Mutex<ReloadStats>>,
}

impl std::fmt::Debug for ReloadManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let files_count = self.files.read().len();
        let handlers_count = self.handlers.lock().len();
        let is_running = self.running.load(std::sync::atomic::Ordering::Relaxed);
        
        f.debug_struct("ReloadManager")
            .field("files_tracked", &files_count)
            .field("handlers_registered", &handlers_count)
            .field("is_running", &is_running)
            .finish_non_exhaustive()
    }
}

impl ReloadManager {
    /// Create a new reload manager
    pub fn new() -> ReloadResult<Self> {
        let (file_sender, file_events) = unbounded();
        let reload_events = unbounded();

        let watcher = RecommendedWatcher::new(
            move |res: notify::Result<Event>| {
                if let Err(e) = file_sender.send(res) {
                    error!("Failed to send file event: {}", e);
                }
            },
            notify::Config::default(),
        )
        .map_err(|e| ReloadError::WatchFailed {
            reason: e.to_string(),
        })?;

        Ok(Self {
            _watcher: watcher,
            files: Arc::new(RwLock::new(HashMap::new())),
            handlers: Arc::new(Mutex::new(Vec::new())),
            file_events,
            reload_events,
            running: Arc::new(AtomicBool::new(false)),
            stats: Arc::new(Mutex::new(ReloadStats::default())),
        })
    }

    /// Start the reload manager
    pub fn start(&self) -> ReloadResult<()> {
        if self.running.swap(true, Ordering::Relaxed) {
            warn!("ReloadManager already running");
            return Ok(());
        }

        let files = self.files.clone();
        let handlers = self.handlers.clone();
        let file_events = self.file_events.clone();
        let reload_sender = self.reload_events.0.clone();
        let running = self.running.clone();
        let stats = self.stats.clone();

        thread::spawn(move || {
            info!("🔥 ReloadManager started");

            while running.load(Ordering::Relaxed) {
                if let Ok(event_result) = file_events.recv_timeout(Duration::from_millis(100)) {
                    match event_result {
                        Ok(event) => {
                            Self::handle_file_event(
                                event,
                                &files,
                                &handlers,
                                &reload_sender,
                                &stats,
                            );
                        }
                        Err(e) => {
                            error!("File watcher error: {}", e);
                            let _ = reload_sender.send(ReloadEvent::Failed {
                                path: PathBuf::from("unknown"),
                                error: e.to_string(),
                            });
                        }
                    }
                }
            }

            info!("🔥 ReloadManager stopped");
        });

        self.stats.lock().is_running = true;
        Ok(())
    }

    /// Stop the reload manager
    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
        self.stats.lock().is_running = false;
    }

    /// Watch a file for changes
    pub fn watch_file(&mut self, path: PathBuf, file_type: FileType) -> ReloadResult<()> {
        let file_info = FileInfo::new(path.clone(), file_type)?;

        self._watcher
            .watch(&path, RecursiveMode::NonRecursive)
            .map_err(|e| ReloadError::WatchFailed {
                reason: e.to_string(),
            })?;

        self.files.write().insert(path.clone(), file_info);
        self.stats.lock().files_watched += 1;

        debug!("📁 Watching file: {}", path.display());
        Ok(())
    }

    /// Register a handler for specific file types
    pub fn add_handler(&self, handler: Box<dyn ReloadHandler>) {
        debug!("➕ Added handler: {}", handler.name());
        self.handlers.lock().push(handler);
        self.stats.lock().handlers_registered += 1;
    }

    /// Get the next reload event (non-blocking)
    pub fn poll_event(&self) -> Option<ReloadEvent> {
        self.reload_events.1.try_recv().ok()
    }

    /// Get all pending events
    pub fn poll_events(&self) -> Vec<ReloadEvent> {
        let mut events = Vec::new();
        while let Some(event) = self.poll_event() {
            events.push(event);
        }
        events
    }

    /// Get statistics
    pub fn stats(&self) -> ReloadStats {
        self.stats.lock().clone()
    }

    /// Get read access to handlers for inspection
    pub fn get_handlers(&self) -> parking_lot::MutexGuard<Vec<Box<dyn ReloadHandler>>> {
        self.handlers.lock()
    }

    /// Get a specific handler by name (debug builds only)
    #[cfg(debug_assertions)]
    pub fn get_handler(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        let handlers = self.handlers.lock();
        for handler in handlers.iter() {
            if handler.name() == name {
                // This is a workaround since we can't directly return a reference
                // In practice, you'd need to restructure this to return a reference
                // or use a different approach for accessing specific handlers
                break;
            }
        }
        None
    }

    /// Execute a function with access to handlers (debug builds only)
    #[cfg(debug_assertions)]
    pub fn with_handlers<T>(&self, f: impl FnOnce(&[Box<dyn ReloadHandler>]) -> T) -> T {
        let handlers = self.handlers.lock();
        f(&*handlers)
    }

    /// Handle file system events
    fn handle_file_event(
        event: Event,
        files: &Arc<RwLock<HashMap<PathBuf, FileInfo>>>,
        handlers: &Arc<Mutex<Vec<Box<dyn ReloadHandler>>>>,
        sender: &Sender<ReloadEvent>,
        stats: &Arc<Mutex<ReloadStats>>,
    ) {
        if !matches!(event.kind, EventKind::Modify(_)) {
            return;
        }

        for path in event.paths {
            // Check if we're tracking this file and if it changed
            let should_reload = {
                let mut files_lock = files.write();
                if let Some(file_info) = files_lock.get_mut(&path) {
                    file_info.has_changed()
                } else {
                    false
                }
            };

            if should_reload {
                let _ = sender.send(ReloadEvent::FileChanged { path: path.clone() });

                // Find handler and reload
                Self::trigger_reload(&path, handlers, sender, stats);
            }
        }
    }

    /// Trigger reload using appropriate handler
    fn trigger_reload(
        path: &PathBuf,
        handlers: &Arc<Mutex<Vec<Box<dyn ReloadHandler>>>>,
        sender: &Sender<ReloadEvent>,
        stats: &Arc<Mutex<ReloadStats>>,
    ) {
        let mut handlers_lock = handlers.lock();
        
        for handler in handlers_lock.iter_mut() {
            if handler.handles(path) {
                stats.lock().reloads_attempted += 1;
                
                let handler_name = handler.name();
                match handler.reload(path) {
                    Ok(()) => {
                        info!("🔄 Reloaded {} using {}", path.display(), handler_name);
                        stats.lock().reloads_successful += 1;
                        let _ = sender.send(ReloadEvent::Reloaded {
                            path: path.clone(),
                            handler: handler_name.to_string(),
                        });
                    }
                    Err(e) => {
                        error!("❌ Failed to reload {} using {}: {}", path.display(), handler_name, e);
                        let _ = sender.send(ReloadEvent::Failed {
                            path: path.clone(),
                            error: e.to_string(),
                        });
                    }
                }
                break;
            }
        }
    }
}

impl Drop for ReloadManager {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{atomic::AtomicUsize, Arc};
    use tempfile::TempDir;

    struct TestHandler {
        name: &'static str,
        counter: Arc<AtomicUsize>,
        extension: &'static str,
    }

    impl TestHandler {
        fn new(name: &'static str, extension: &'static str) -> (Self, Arc<AtomicUsize>) {
            let counter = Arc::new(AtomicUsize::new(0));
            (
                Self {
                    name,
                    counter: counter.clone(),
                    extension,
                },
                counter,
            )
        }
    }

    impl ReloadHandler for TestHandler {
        fn name(&self) -> &'static str {
            self.name
        }

        fn handles(&self, path: &PathBuf) -> bool {
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext == self.extension)
                .unwrap_or(false)
        }

        fn reload(&mut self, _path: &PathBuf) -> ReloadResult<()> {
            self.counter.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }

    #[test]
    fn manager_creation() {
        let manager = ReloadManager::new();
        assert!(manager.is_ok());

        let stats = manager.unwrap().stats();
        assert_eq!(stats.files_watched, 0);
        assert_eq!(stats.handlers_registered, 0);
        assert!(!stats.is_running);
    }

    #[test]
    fn file_watching() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.lua");
        std::fs::write(&test_file, "print('test')").unwrap();

        let mut manager = ReloadManager::new().unwrap();
        let result = manager.watch_file(test_file, FileType::Lua);
        assert!(result.is_ok());

        let stats = manager.stats();
        assert_eq!(stats.files_watched, 1);
    }

    #[test]
    fn handler_registration() {
        let manager = ReloadManager::new().unwrap();
        let (handler, _counter) = TestHandler::new("test", "lua");

        manager.add_handler(Box::new(handler));

        let stats = manager.stats();
        assert_eq!(stats.handlers_registered, 1);
    }
}
