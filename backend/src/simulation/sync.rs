//! Tick synchronization system using tokio::time::interval
//!
//! Provides precise timing control and synchronization for multiplayer
//! determinism and consistent frame pacing across different systems.

use std::{
    sync::{
        atomic::{AtomicU64, AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use thiserror::Error;
use tokio::time::{interval, sleep_until, Interval, MissedTickBehavior};
use tracing::{debug, info, warn};
use parking_lot::{Mutex, RwLock};

/// Tick synchronization modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncMode {
    /// Run as fast as possible (no sync)
    Unlimited,
    /// Fixed tick rate (deterministic)
    FixedRate,
    /// Adaptive rate based on performance
    Adaptive,
    /// Synchronized to external source (multiplayer)
    External,
}

/// Synchronization configuration
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// Target ticks per second
    pub target_tps: u64,
    /// Maximum allowed tick time
    pub max_tick_duration: Duration,
    /// Sync mode
    pub mode: SyncMode,
    /// Enable catch-up for missed ticks
    pub catch_up: bool,
    /// Maximum catch-up ticks
    pub max_catch_up: u64,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            target_tps: 60,
            max_tick_duration: Duration::from_millis(50), // 20 TPS minimum
            mode: SyncMode::FixedRate,
            catch_up: true,
            max_catch_up: 5,
        }
    }
}

/// Tick synchronizer for deterministic timing
#[derive(Debug)]
pub struct TickSynchronizer {
    /// Configuration
    config: SyncConfig,
    /// Current tick counter
    current_tick: Arc<AtomicU64>,
    /// Target tick time
    tick_duration: Duration,
    /// Tokio interval timer
    interval: Option<Interval>,
    /// Synchronization statistics
    stats: Arc<RwLock<SyncStats>>,
    /// Running state
    running: Arc<AtomicBool>,
    /// Last tick time
    last_tick_time: Arc<Mutex<Instant>>,
    /// Accumulated tick debt for catch-up
    tick_debt: Arc<AtomicU64>,
    /// External synchronization receiver channel
    external_sync_receiver: Option<tokio::sync::mpsc::UnboundedReceiver<u64>>,
    /// External synchronization sender channel for setup
    external_sync_sender: Option<tokio::sync::mpsc::UnboundedSender<u64>>,
}

impl TickSynchronizer {
    /// Create new tick synchronizer
    pub fn new() -> Self {
        let config = SyncConfig::default();
        let tick_duration = Duration::from_nanos(1_000_000_000 / config.target_tps);
        
        Self {
            config,
            current_tick: Arc::new(AtomicU64::new(0)),
            tick_duration,
            interval: None,
            stats: Arc::new(RwLock::new(SyncStats::default())),
            running: Arc::new(AtomicBool::new(false)),
            last_tick_time: Arc::new(Mutex::new(Instant::now())),
            tick_debt: Arc::new(AtomicU64::new(0)),
            external_sync_receiver: None,
            external_sync_sender: None,
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: SyncConfig) -> Self {
        let tick_duration = Duration::from_nanos(1_000_000_000 / config.target_tps);
        
        Self {
            tick_duration,
            config,
            current_tick: Arc::new(AtomicU64::new(0)),
            interval: None,
            stats: Arc::new(RwLock::new(SyncStats::default())),
            running: Arc::new(AtomicBool::new(false)),
            last_tick_time: Arc::new(Mutex::new(Instant::now())),
            tick_debt: Arc::new(AtomicU64::new(0)),
            external_sync_receiver: None,
            external_sync_sender: None,
        }
    }

    /// Start synchronization
    pub fn start(&mut self) -> Result<(), SyncError> {
        if self.running.load(Ordering::Relaxed) {
            return Err(SyncError::AlreadyRunning);
        }

        match self.config.mode {
            SyncMode::FixedRate => {
                let mut timer = interval(self.tick_duration);
                timer.set_missed_tick_behavior(if self.config.catch_up {
                    MissedTickBehavior::Burst
                } else {
                    MissedTickBehavior::Skip
                });
                self.interval = Some(timer);
            }
            SyncMode::Adaptive => {
                // Start with fixed rate, will adjust dynamically
                let mut timer = interval(self.tick_duration);
                timer.set_missed_tick_behavior(MissedTickBehavior::Delay);
                self.interval = Some(timer);
            }
            _ => {
                // No interval needed for unlimited or external sync
            }
        }

        self.running.store(true, Ordering::Relaxed);
        *self.last_tick_time.lock() = Instant::now();
        
        info!("Started tick synchronizer: {:?} at {} TPS", self.config.mode, self.config.target_tps);
        Ok(())
    }

    /// Stop synchronization
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        self.interval = None;
        info!("Stopped tick synchronizer");
    }

    /// Wait for next tick
    pub async fn wait_for_tick(&mut self) -> Result<u64, SyncError> {
        if !self.running.load(Ordering::Relaxed) {
            return Err(SyncError::NotRunning);
        }

        let tick_start = Instant::now();
        let current_tick = match self.config.mode {
            SyncMode::Unlimited => {
                // No waiting, just increment tick
                self.current_tick.fetch_add(1, Ordering::Relaxed) + 1
            }
            SyncMode::FixedRate => {
                self.wait_fixed_rate().await?
            }
            SyncMode::Adaptive => {
                self.wait_adaptive().await?
            }
            SyncMode::External => {
                // Wait for external synchronization signal
                self.wait_external().await?
            }
        };

        // Update statistics
        let tick_time = tick_start.elapsed();
        {
            let mut stats = self.stats.write();
            stats.update(tick_time);
            
            if tick_time > self.config.max_tick_duration {
                stats.slow_ticks += 1;
                warn!("Slow tick detected: {:?} > {:?}", tick_time, self.config.max_tick_duration);
            }
        }

        *self.last_tick_time.lock() = tick_start;
        debug!("Tick {} completed in {:?}", current_tick, tick_time);
        
        Ok(current_tick)
    }

    /// Get current tick number
    pub fn current_tick(&self) -> u64 {
        self.current_tick.load(Ordering::Relaxed)
    }

    /// Check if synchronizer is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    /// Get synchronization statistics
    pub fn stats(&self) -> SyncStats {
        self.stats.read().clone()
    }

    /// Update configuration
    pub fn update_config(&mut self, config: SyncConfig) -> Result<(), SyncError> {
        if self.running.load(Ordering::Relaxed) {
            return Err(SyncError::CannotUpdateWhileRunning);
        }

        self.config = config;
        self.tick_duration = Duration::from_nanos(1_000_000_000 / self.config.target_tps);
        
        info!("Updated sync config: {:?}", self.config);
        Ok(())
    }

    /// Set tick manually (for external sync)
    pub fn set_tick(&self, tick: u64) {
        self.current_tick.store(tick, Ordering::Relaxed);
        debug!("Manually set tick to {}", tick);
    }

    /// Calculate time until next tick
    pub fn time_until_next_tick(&self) -> Duration {
        let last_tick = *self.last_tick_time.lock();
        let elapsed = last_tick.elapsed();
        
        if elapsed >= self.tick_duration {
            Duration::ZERO
        } else {
            self.tick_duration - elapsed
        }
    }

    async fn wait_fixed_rate(&mut self) -> Result<u64, SyncError> {
        if let Some(ref mut timer) = self.interval {
            timer.tick().await;
            Ok(self.current_tick.fetch_add(1, Ordering::Relaxed) + 1)
        } else {
            Err(SyncError::IntervalNotInitialized)
        }
    }

    async fn wait_adaptive(&mut self) -> Result<u64, SyncError> {
        if let Some(ref mut timer) = self.interval {
            timer.tick().await;
            
            // Adjust tick rate based on performance
            let stats = self.stats.read();
            if stats.average_tick_time() > self.tick_duration * 9 / 10 {
                // If we're running close to the limit, slow down slightly
                drop(stats);
                self.adapt_tick_rate(0.95).await;
            } else if stats.average_tick_time() < self.tick_duration / 2 {
                // If we have plenty of headroom, speed up slightly
                drop(stats);
                self.adapt_tick_rate(1.05).await;
            }
            
            Ok(self.current_tick.fetch_add(1, Ordering::Relaxed) + 1)
        } else {
            Err(SyncError::IntervalNotInitialized)
        }
    }

    async fn wait_external(&mut self) -> Result<u64, SyncError> {
        // In external sync mode, we wait for an external system to set the tick
        // This could be from network synchronization, master server, or other coordination
        
        // Check if we have external tick events to process
        if let Some(external_tick) = self.check_external_tick_events().await? {
            // External system provided a specific tick number
            self.current_tick.store(external_tick, Ordering::Relaxed);
            return Ok(external_tick);
        }
        
        // Wait for external tick signal or timeout
        let timeout_duration = self.tick_duration * 3; // Allow some slack for network delays
        let wait_start = tokio::time::Instant::now();
        
        loop {
            // Check for external tick signal every 1ms
            tokio::time::sleep(Duration::from_millis(1)).await;
            
            // Check if external tick was set by another thread/system
            let current_external_tick = self.current_tick.load(Ordering::Relaxed);
            let expected_tick = current_external_tick + 1;
            
            // Check if external system has advanced the tick
            if self.has_external_tick_advanced(expected_tick) {
                self.current_tick.store(expected_tick, Ordering::Relaxed);
                return Ok(expected_tick);
            }
            
            // Check for timeout
            if wait_start.elapsed() > timeout_duration {
                warn!("External sync timeout after {:?}, falling back to local increment", timeout_duration);
                // Fall back to local tick increment to prevent hanging
                let new_tick = self.current_tick.fetch_add(1, Ordering::Relaxed) + 1;
                
                // Update stats to track external sync failures
                {
                    let mut stats = self.stats.write();
                    stats.external_sync_failures += 1;
                }
                
                return Ok(new_tick);
            }
        }
    }
    
    /// Check for external tick events from network or other sources
    async fn check_external_tick_events(&mut self) -> Result<Option<u64>, SyncError> {
        // In a real implementation, this would:
        // 1. Check network messages for tick synchronization
        // 2. Read from shared memory or message queues
        // 3. Communicate with master server or coordinator
        // 4. Process multiplayer synchronization events
        
        // For now, simulate checking for external events
        // This would be replaced with actual network/IPC communication
        
        // Check if external synchronizer has set a specific tick
        if let Some(sync_channel) = &mut self.external_sync_receiver {
            match sync_channel.try_recv() {
                Ok(external_tick) => {
                    debug!("Received external tick: {}", external_tick);
                    return Ok(Some(external_tick));
                },
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                    // No external tick available
                },
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                    return Err(SyncError::ExternalSyncDisconnected);
                }
            }
        }
        
        Ok(None)
    }
    
    /// Check if external system has signaled tick advancement
    fn has_external_tick_advanced(&self, expected_tick: u64) -> bool {
        // In a real implementation, this would check:
        // 1. Network synchronization flags
        // 2. Shared memory tick counters
        // 3. Master server tick broadcasts
        // 4. Peer-to-peer tick consensus
        
        // For now, simulate external tick advancement detection
        // This could check a file, network endpoint, or shared resource
        
        // Simple simulation: check if external tick file exists or network signal received
        // In practice, this would be much more sophisticated
        false // Placeholder - would check actual external sync state
    }

    async fn adapt_tick_rate(&mut self, factor: f64) {
        let new_duration = Duration::from_nanos(
            (self.tick_duration.as_nanos() as f64 / factor) as u64
        );
        
        // Clamp to reasonable bounds
        let min_duration = Duration::from_millis(1);  // 1000 TPS max
        let max_duration = Duration::from_millis(100); // 10 TPS min
        
        self.tick_duration = new_duration.max(min_duration).min(max_duration);
        
        // Update the interval
        let mut timer = interval(self.tick_duration);
        timer.set_missed_tick_behavior(MissedTickBehavior::Delay);
        self.interval = Some(timer);
        
        debug!("Adapted tick duration to {:?}", self.tick_duration);
    }
    
    /// Set up external synchronization channel
    pub fn setup_external_sync(&mut self) -> tokio::sync::mpsc::UnboundedSender<u64> {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        self.external_sync_receiver = Some(receiver);
        self.external_sync_sender = Some(sender.clone());
        
        info!("External synchronization channel established");
        sender
    }
    
    /// Send external tick signal
    pub fn send_external_tick(&self, tick: u64) -> Result<(), SyncError> {
        if let Some(sender) = &self.external_sync_sender {
            sender.send(tick).map_err(|_| SyncError::ExternalSyncDisconnected)?;
            debug!("Sent external tick signal: {}", tick);
            Ok(())
        } else {
            Err(SyncError::ExternalSyncDisconnected)
        }
    }
    
    /// Check if external sync is available
    pub fn has_external_sync(&self) -> bool {
        self.external_sync_receiver.is_some() && self.external_sync_sender.is_some()
    }
    
    /// Get external sync failure count
    pub fn external_sync_failures(&self) -> u64 {
        self.stats.read().external_sync_failures
    }
}

/// Synchronization statistics
#[derive(Debug, Clone, Default)]
pub struct SyncStats {
    /// Total ticks processed
    pub total_ticks: u64,
    /// Total time spent in ticks
    pub total_tick_time: Duration,
    /// Number of slow ticks
    pub slow_ticks: u64,
    /// Minimum tick time
    pub min_tick_time: Duration,
    /// Maximum tick time
    pub max_tick_time: Duration,
    /// Current TPS
    pub current_tps: f64,
    /// Start time for TPS calculation
    pub start_time: Option<Instant>,
    /// External synchronization failures
    pub external_sync_failures: u64,
}

impl SyncStats {
    /// Update statistics with new tick time
    pub fn update(&mut self, tick_time: Duration) {
        if self.start_time.is_none() {
            self.start_time = Some(Instant::now());
        }

        self.total_ticks += 1;
        self.total_tick_time += tick_time;

        if self.total_ticks == 1 {
            self.min_tick_time = tick_time;
            self.max_tick_time = tick_time;
        } else {
            self.min_tick_time = self.min_tick_time.min(tick_time);
            self.max_tick_time = self.max_tick_time.max(tick_time);
        }

        // Update current TPS
        if let Some(start) = self.start_time {
            let elapsed = start.elapsed();
            if elapsed > Duration::ZERO {
                self.current_tps = self.total_ticks as f64 / elapsed.as_secs_f64();
            }
        }
    }

    /// Get average tick time
    pub fn average_tick_time(&self) -> Duration {
        if self.total_ticks > 0 {
            self.total_tick_time / self.total_ticks as u32
        } else {
            Duration::ZERO
        }
    }

    /// Get slow tick percentage
    pub fn slow_tick_percentage(&self) -> f64 {
        if self.total_ticks > 0 {
            self.slow_ticks as f64 / self.total_ticks as f64 * 100.0
        } else {
            0.0
        }
    }
}

/// Synchronization errors
#[derive(Error, Debug)]
pub enum SyncError {
    #[error("Synchronizer is already running")]
    AlreadyRunning,
    #[error("Synchronizer is not running")]
    NotRunning,
    #[error("Cannot update configuration while running")]
    CannotUpdateWhileRunning,
    #[error("Interval timer not initialized")]
    IntervalNotInitialized,
    #[error("External sync timeout")]
    ExternalSyncTimeout,
    #[error("External synchronization channel disconnected")]
    ExternalSyncDisconnected,
}

#[cfg(test)]
mod tests {
    use super::*;
    // Note: These functions require test-util feature
    // use tokio::time::{pause, resume, advance};

    #[tokio::test]
    async fn test_tick_synchronizer_creation() {
        let sync = TickSynchronizer::new();
        assert!(!sync.is_running());
        assert_eq!(sync.current_tick(), 0);
    }

    #[tokio::test]
    async fn test_fixed_rate_sync() {
        let mut sync = TickSynchronizer::with_config(SyncConfig {
            target_tps: 10, // 10 TPS for faster testing
            mode: SyncMode::FixedRate,
            ..Default::default()
        });

        sync.start().unwrap();
        assert!(sync.is_running());

        // Use tokio time control for deterministic testing
        pause();
        
        let tick1 = sync.wait_for_tick().await.unwrap();
        assert_eq!(tick1, 1);
        
        advance(Duration::from_millis(100)).await;
        let tick2 = sync.wait_for_tick().await.unwrap();
        assert_eq!(tick2, 2);

        resume();
        sync.stop();
    }

    #[test]
    fn test_sync_config() {
        let config = SyncConfig {
            target_tps: 120,
            mode: SyncMode::Adaptive,
            catch_up: false,
            max_catch_up: 10,
            ..Default::default()
        };

        assert_eq!(config.target_tps, 120);
        assert_eq!(config.mode, SyncMode::Adaptive);
        assert!(!config.catch_up);
    }

    #[test]
    fn test_sync_stats() {
        let mut stats = SyncStats::default();
        
        stats.update(Duration::from_millis(10));
        stats.update(Duration::from_millis(20));
        stats.update(Duration::from_millis(15));
        
        assert_eq!(stats.total_ticks, 3);
        assert_eq!(stats.average_tick_time(), Duration::from_millis(15));
        assert_eq!(stats.min_tick_time, Duration::from_millis(10));
        assert_eq!(stats.max_tick_time, Duration::from_millis(20));
    }
}
