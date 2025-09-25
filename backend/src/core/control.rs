//! Time control system for simulation playback and debugging
//!
//! Provides high-level time controls (play/pause/step/speed) using instant crate
//! for precise timing, integrated with the existing simulation architecture.

use crate::core::time::{SimulationState, DeterministicFloat, det_f32};
use instant::{Duration, Instant};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, atomic::{AtomicBool, AtomicU32, Ordering}};
use thiserror::Error;
use tracing::{debug, info, warn};

/// Time control modes for simulation playback
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlaybackMode {
    /// Normal real-time playback
    Playing,
    /// Simulation is paused
    Paused,
    /// Single step mode - advance one tick then pause
    Stepping,
    /// Fast forward at specified multiplier
    FastForward,
    /// Slow motion at specified fraction
    SlowMotion,
}

/// Time control configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlConfig {
    /// Default playback speed multiplier
    pub default_speed: DeterministicFloat,
    /// Maximum allowed speed multiplier
    pub max_speed: DeterministicFloat,
    /// Minimum allowed speed fraction
    pub min_speed: DeterministicFloat,
    /// Enable frame stepping
    pub allow_stepping: bool,
    /// Auto-pause on errors
    pub auto_pause_on_error: bool,
}

impl Default for ControlConfig {
    fn default() -> Self {
        Self {
            default_speed: det_f32(1.0),
            max_speed: det_f32(16.0),
            min_speed: det_f32(0.0625), // 1/16 speed
            allow_stepping: true,
            auto_pause_on_error: true,
        }
    }
}

/// High-precision time control system
#[derive(Debug)]
pub struct TimeController {
    /// Current playback mode
    mode: Arc<RwLock<PlaybackMode>>,
    /// Speed multiplier
    speed: Arc<RwLock<DeterministicFloat>>,
    /// Configuration
    config: ControlConfig,
    /// Last update time (for delta calculations)
    last_update: Arc<RwLock<Instant>>,
    /// Accumulated time for fractional speeds
    accumulated_time: Arc<RwLock<Duration>>,
    /// Step request counter
    step_requested: Arc<AtomicU32>,
    /// Error state flag
    error_state: Arc<AtomicBool>,
    /// Statistics
    stats: Arc<RwLock<ControlStats>>,
}

impl TimeController {
    /// Create new time controller
    pub fn new() -> Self {
        Self::with_config(ControlConfig::default())
    }

    /// Create with custom configuration
    pub fn with_config(config: ControlConfig) -> Self {
        info!("Initializing time controller with config: {:?}", config);
        
        Self {
            mode: Arc::new(RwLock::new(PlaybackMode::Playing)),
            speed: Arc::new(RwLock::new(config.default_speed)),
            config,
            last_update: Arc::new(RwLock::new(Instant::now())),
            accumulated_time: Arc::new(RwLock::new(Duration::ZERO)),
            step_requested: Arc::new(AtomicU32::new(0)),
            error_state: Arc::new(AtomicBool::new(false)),
            stats: Arc::new(RwLock::new(ControlStats::default())),
        }
    }

    /// Play simulation at normal speed
    pub fn play(&self) -> Result<(), ControlError> {
        self.set_mode(PlaybackMode::Playing)?;
        info!("Simulation resumed");
        Ok(())
    }

    /// Pause simulation
    pub fn pause(&self) -> Result<(), ControlError> {
        self.set_mode(PlaybackMode::Paused)?;
        info!("Simulation paused");
        Ok(())
    }

    /// Toggle between play and pause
    pub fn toggle(&self) -> Result<PlaybackMode, ControlError> {
        let current = *self.mode.read();
        let new_mode = match current {
            PlaybackMode::Playing => PlaybackMode::Paused,
            PlaybackMode::Paused => PlaybackMode::Playing,
            _ => PlaybackMode::Playing,
        };
        
        self.set_mode(new_mode)?;
        Ok(new_mode)
    }

    /// Request single step (advance one tick then pause)
    pub fn step(&self) -> Result<(), ControlError> {
        if !self.config.allow_stepping {
            return Err(ControlError::SteppingDisabled);
        }

        self.step_requested.fetch_add(1, Ordering::Relaxed);
        self.set_mode(PlaybackMode::Stepping)?;
        
        debug!("Step requested");
        Ok(())
    }

    /// Set playback speed multiplier
    pub fn set_speed(&self, speed: f32) -> Result<(), ControlError> {
        let speed = det_f32(speed);
        
        if speed < self.config.min_speed {
            return Err(ControlError::SpeedTooSlow(speed.into_inner()));
        }
        if speed > self.config.max_speed {
            return Err(ControlError::SpeedTooFast(speed.into_inner()));
        }

        *self.speed.write() = speed;
        
        // Update mode based on speed
        let mode = if speed > det_f32(1.0) {
            PlaybackMode::FastForward
        } else if speed < det_f32(1.0) {
            PlaybackMode::SlowMotion
        } else {
            PlaybackMode::Playing
        };
        
        self.set_mode(mode)?;
        info!("Speed set to {}x", speed.into_inner());
        Ok(())
    }

    /// Get current playback mode
    pub fn mode(&self) -> PlaybackMode {
        *self.mode.read()
    }

    /// Get current speed multiplier
    pub fn speed(&self) -> f32 {
        self.speed.read().into_inner()
    }

    /// Check if simulation should advance this frame
    pub fn should_advance(&self, simulation: &SimulationState) -> bool {
        // Check error state
        if self.error_state.load(Ordering::Relaxed) && self.config.auto_pause_on_error {
            return false;
        }

        let mode = *self.mode.read();
        match mode {
            PlaybackMode::Paused => false,
            PlaybackMode::Playing | PlaybackMode::FastForward | PlaybackMode::SlowMotion => {
                self.check_speed_advance()
            }
            PlaybackMode::Stepping => {
                let steps = self.step_requested.load(Ordering::Relaxed);
                if steps > 0 {
                    self.step_requested.fetch_sub(1, Ordering::Relaxed);
                    // Auto-pause after step
                    let _ = self.set_mode(PlaybackMode::Paused);
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Update controller state and return effective time delta
    pub fn update(&self) -> DeterministicFloat {
        let now = Instant::now();
        let mut last = self.last_update.write();
        let delta = now - *last;
        *last = now;

        let speed = *self.speed.read();
        let effective_delta = det_f32(delta.as_secs_f32() * speed.into_inner());

        // Update statistics
        {
            let mut stats = self.stats.write();
            stats.update(delta, speed);
        }

        effective_delta
    }

    /// Set error state (auto-pauses if configured)
    pub fn set_error(&self, error: bool) {
        self.error_state.store(error, Ordering::Relaxed);
        
        if error && self.config.auto_pause_on_error {
            let _ = self.pause();
            warn!("Auto-paused due to error state");
        }
    }

    /// Get control statistics
    pub fn stats(&self) -> ControlStats {
        self.stats.read().clone()
    }

    /// Reset controller to default state
    pub fn reset(&self) {
        *self.mode.write() = PlaybackMode::Playing;
        *self.speed.write() = self.config.default_speed;
        *self.last_update.write() = Instant::now();
        *self.accumulated_time.write() = Duration::ZERO;
        self.step_requested.store(0, Ordering::Relaxed);
        self.error_state.store(false, Ordering::Relaxed);
        *self.stats.write() = ControlStats::default();
        
        info!("Time controller reset to defaults");
    }

    fn set_mode(&self, mode: PlaybackMode) -> Result<(), ControlError> {
        let old_mode = *self.mode.read();
        *self.mode.write() = mode;
        
        debug!("Mode changed from {:?} to {:?}", old_mode, mode);
        Ok(())
    }

    fn check_speed_advance(&self) -> bool {
        let speed = *self.speed.read();
        
        // For speeds >= 1.0, always advance
        if speed >= det_f32(1.0) {
            return true;
        }
        
        // For fractional speeds, accumulate time and advance when threshold is met
        let now = Instant::now();
        let last = *self.last_update.read();
        let delta = now - last;
        
        let mut accumulated = self.accumulated_time.write();
        *accumulated += delta;
        
        let threshold = Duration::from_secs_f32(1.0 / speed.into_inner());
        if *accumulated >= threshold {
            *accumulated -= threshold;
            true
        } else {
            false
        }
    }
}

/// Time controller statistics
#[derive(Debug, Clone, Default)]
pub struct ControlStats {
    /// Total update calls
    pub total_updates: u64,
    /// Total real time elapsed
    pub real_time_elapsed: Duration,
    /// Total simulation time elapsed
    pub sim_time_elapsed: Duration,
    /// Current effective FPS
    pub effective_fps: f32,
    /// Average speed multiplier
    pub average_speed: f32,
    /// Mode change count
    pub mode_changes: u32,
}

impl ControlStats {
    fn update(&mut self, delta: Duration, speed: DeterministicFloat) {
        self.total_updates += 1;
        self.real_time_elapsed += delta;
        self.sim_time_elapsed += Duration::from_secs_f32(delta.as_secs_f32() * speed.into_inner());
        
        // Calculate effective FPS (simulation time / real time)
        if self.real_time_elapsed > Duration::ZERO {
            self.effective_fps = self.sim_time_elapsed.as_secs_f32() / self.real_time_elapsed.as_secs_f32();
        }
        
        // Update average speed
        let total_speed = self.average_speed * (self.total_updates - 1) as f32 + speed.into_inner();
        self.average_speed = total_speed / self.total_updates as f32;
    }
}

/// Time control errors
#[derive(Error, Debug)]
pub enum ControlError {
    #[error("Speed {0} is too slow (minimum: {1})")]
    SpeedTooSlow(f32),
    #[error("Speed {0} is too fast (maximum: {1})")]  
    SpeedTooFast(f32),
    #[error("Frame stepping is disabled")]
    SteppingDisabled,
    #[error("Cannot perform operation in current mode")]
    InvalidMode,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_controller_creation() {
        let controller = TimeController::new();
        assert_eq!(controller.mode(), PlaybackMode::Playing);
        assert_eq!(controller.speed(), 1.0);
    }

    #[test]
    fn test_play_pause_toggle() {
        let controller = TimeController::new();
        
        controller.pause().unwrap();
        assert_eq!(controller.mode(), PlaybackMode::Paused);
        
        controller.play().unwrap();
        assert_eq!(controller.mode(), PlaybackMode::Playing);
        
        let mode = controller.toggle().unwrap();
        assert_eq!(mode, PlaybackMode::Paused);
    }

    #[test]
    fn test_speed_control() {
        let controller = TimeController::new();
        
        controller.set_speed(2.0).unwrap();
        assert_eq!(controller.speed(), 2.0);
        assert_eq!(controller.mode(), PlaybackMode::FastForward);
        
        controller.set_speed(0.5).unwrap();
        assert_eq!(controller.speed(), 0.5);
        assert_eq!(controller.mode(), PlaybackMode::SlowMotion);
    }

    #[test]
    fn test_step_mode() {
        let controller = TimeController::new();
        let sim = SimulationState::new(42, None);
        
        controller.step().unwrap();
        assert_eq!(controller.mode(), PlaybackMode::Stepping);
        
        // First call should advance
        assert!(controller.should_advance(&sim));
        // Second call should not (auto-paused)
        assert!(!controller.should_advance(&sim));
    }

    #[test]
    fn test_speed_validation() {
        let controller = TimeController::new();
        
        // Test speed limits
        assert!(controller.set_speed(0.01).is_err()); // Too slow
        assert!(controller.set_speed(32.0).is_err());  // Too fast
        assert!(controller.set_speed(2.0).is_ok());    // Valid
    }

    #[test]
    fn test_fractional_speed_advance() {
        let controller = TimeController::new();
        let sim = SimulationState::new(42, None);
        
        controller.set_speed(0.25).unwrap(); // Quarter speed
        
        // Should not advance on every call at quarter speed
        // (This test is timing-sensitive and may be flaky)
        sleep(Duration::from_millis(10));
        let advance1 = controller.should_advance(&sim);
        let advance2 = controller.should_advance(&sim);
        
        // At least one should be false for quarter speed
        assert!(!(advance1 && advance2));
    }
}
