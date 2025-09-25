//! Deterministic simulation time and RNG systems
//!
//! Provides precise timing control and deterministic random number generation
//! for reproducible game simulations across different platforms.

use chrono::{DateTime, Utc};
use ordered_float::OrderedFloat;
use parking_lot::Mutex;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use bevy_ecs::prelude::Resource;
use serde::{Deserialize, Serialize};
use spin_sleep::SpinSleeper;
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tracing::{debug, warn};
use crate::core::zig_ffi::{det_add_f32, det_mul_f32, det_div_f32, det_sqrt_f32};

/// Deterministic float type for cross-platform consistency
pub type DeterministicFloat = OrderedFloat<f32>;

/// Deterministic double precision float
pub type DeterministicDouble = OrderedFloat<f64>;

/// Create a deterministic float value
pub fn det_f32(value: f32) -> DeterministicFloat {
    OrderedFloat(value)
}

/// Create a deterministic double value  
pub fn det_f64(value: f64) -> DeterministicDouble {
    OrderedFloat(value)
}

/// Deterministic floating point operations using Zig SIMD optimizations
pub mod det_math {
    use super::*;
    
    /// Deterministic addition using Zig optimizations
    pub fn add(a: f32, b: f32) -> f32 {
        det_add_f32(a, b)
    }
    
    /// Deterministic multiplication using Zig optimizations
    pub fn mul(a: f32, b: f32) -> f32 {
        det_mul_f32(a, b)
    }
    
    /// Deterministic division using Zig optimizations
    pub fn div(a: f32, b: f32) -> f32 {
        det_div_f32(a, b)
    }
    
    /// Deterministic square root using Zig optimizations
    pub fn sqrt(a: f32) -> f32 {
        det_sqrt_f32(a)
    }
    
    /// Deterministic linear interpolation
    pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
        add(mul(a, sub(1.0, t)), mul(b, t))
    }
    
    /// Deterministic subtraction
    pub fn sub(a: f32, b: f32) -> f32 {
        add(a, -b)
    }
    
    /// Deterministic distance calculation
    pub fn distance(x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
        let dx = sub(x2, x1);
        let dy = sub(y2, y1);
        sqrt(add(mul(dx, dx), mul(dy, dy)))
    }
    
    /// Deterministic clamp operation  
    pub fn clamp(value: f32, min_val: f32, max_val: f32) -> f32 {
        if value < min_val { min_val }
        else if value > max_val { max_val }
        else { value }
    }
}

/// Global deterministic RNG state
#[derive(Debug, Clone)]
pub struct DeterministicRng {
    rng: ChaCha8Rng,
    initial_seed: u64,
}

impl DeterministicRng {
    /// Create a new deterministic RNG with given seed
    pub fn new(seed: u64) -> Self {
        Self {
            rng: ChaCha8Rng::seed_from_u64(seed),
            initial_seed: seed,
        }
    }

    /// Generate random f32 in range [0.0, 1.0)
    pub fn gen_f32(&mut self) -> DeterministicFloat {
        use rand::Rng;
        det_f32(self.rng.gen())
    }

    /// Generate random f64 in range [0.0, 1.0)
    pub fn gen_f64(&mut self) -> DeterministicDouble {
        use rand::Rng;
        det_f64(self.rng.gen())
    }

    /// Generate random u32
    pub fn gen_u32(&mut self) -> u32 {
        use rand::Rng;
        self.rng.gen()
    }

    /// Generate random u64
    pub fn gen_u64(&mut self) -> u64 {
        use rand::Rng;
        self.rng.gen()
    }

    /// Get initial seed for save/load
    pub fn seed(&self) -> u64 {
        self.initial_seed
    }

    /// Reset RNG to initial state
    pub fn reset(&mut self) {
        self.rng = ChaCha8Rng::seed_from_u64(self.initial_seed);
    }
}

/// Fixed timestep configuration for deterministic simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixedTimestepConfig {
    /// Target updates per second
    pub target_ups: u32,
    /// Maximum frame time before slow-motion
    pub max_frame_time: Duration,
    /// Enable high-precision timing with spin_sleep
    pub use_spin_sleep: bool,
}

impl Default for FixedTimestepConfig {
    fn default() -> Self {
        Self {
            target_ups: 60,
            max_frame_time: Duration::from_millis(50), // 20 FPS minimum
            use_spin_sleep: true,
        }
    }
}

/// Deterministic simulation timer with fixed timestep
#[derive(Debug)]
pub struct DeterministicTimer {
    /// Current simulation tick
    tick: AtomicU64,
    /// Fixed timestep duration
    timestep: Duration,
    /// Accumulated real time
    accumulated_time: Duration,
    /// Last real time measurement
    last_time: Instant,
    /// Spin sleep helper for precision
    sleeper: Option<SpinSleeper>,
    /// Configuration
    config: FixedTimestepConfig,
}

impl Clone for DeterministicTimer {
    fn clone(&self) -> Self {
        Self {
            tick: AtomicU64::new(self.tick.load(Ordering::Relaxed)),
            timestep: self.timestep,
            accumulated_time: self.accumulated_time,
            last_time: Instant::now(), // Reset time for new instance
            sleeper: None, // Create new sleeper
            config: self.config.clone(),
        }
    }
}

impl DeterministicTimer {
    /// Create a new deterministic timer
    pub fn new(config: FixedTimestepConfig) -> Self {
        let timestep = Duration::from_nanos(1_000_000_000 / config.target_ups as u64);
        
        let sleeper = if config.use_spin_sleep {
            Some(SpinSleeper::default())
        } else {
            None
        };

        Self {
            tick: AtomicU64::new(0),
            timestep,
            accumulated_time: Duration::ZERO,
            last_time: Instant::now(),
            sleeper,
            config,
        }
    }

    /// Update timer and return number of simulation steps to execute
    pub fn update(&mut self) -> u32 {
        let now = Instant::now();
        let delta = now - self.last_time;
        self.last_time = now;

        // Clamp delta to prevent spiral of death
        let clamped_delta = delta.min(self.config.max_frame_time);
        if delta > self.config.max_frame_time {
            warn!(
                "Frame time {} exceeded maximum {}, clamping to prevent instability",
                delta.as_millis(),
                self.config.max_frame_time.as_millis()
            );
        }

        self.accumulated_time += clamped_delta;

        let mut steps = 0;
        while self.accumulated_time >= self.timestep {
            self.accumulated_time -= self.timestep;
            self.tick.fetch_add(1, Ordering::Relaxed);
            steps += 1;
        }

        // Use spin sleep for precise timing

        steps
    }

    /// Wait until next timestep with high precision
    pub fn wait_for_next_step(&self) {
        if let Some(ref sleeper) = self.sleeper {
            sleeper.sleep(self.timestep);
        } else {
            std::thread::sleep(self.timestep);
        }
    }

    /// Get current simulation tick
    pub fn tick(&self) -> u64 {
        self.tick.load(Ordering::Relaxed)
    }

    /// Get timestep duration
    pub fn timestep(&self) -> Duration {
        self.timestep
    }

    /// Get timestep as deterministic float seconds
    pub fn timestep_f32(&self) -> DeterministicFloat {
        det_f32(self.timestep.as_secs_f32())
    }

    /// Get current target UPS
    pub fn target_ups(&self) -> u32 {
        self.config.target_ups
    }

    /// Reset timer to initial state
    pub fn reset(&mut self) {
        self.tick.store(0, Ordering::Relaxed);
        self.accumulated_time = Duration::ZERO;
        self.last_time = Instant::now();
    }

    /// Set new target UPS
    pub fn set_target_ups(&mut self, ups: u32) {
        self.config.target_ups = ups;
        self.timestep = Duration::from_nanos(1_000_000_000 / ups as u64);
    }
}

/// Thread-safe global simulation state
#[derive(Debug, Resource)]
pub struct SimulationState {
    /// Deterministic timer
    pub timer: Arc<Mutex<DeterministicTimer>>,
    /// Global deterministic RNG
    pub rng: Arc<Mutex<DeterministicRng>>,
    /// Initial seed for reproducibility
    pub initial_seed: u64,
}

impl SimulationState {
    /// Create new simulation state with given seed
    pub fn new(seed: u64, config: Option<FixedTimestepConfig>) -> Self {
        let config = config.unwrap_or_default();
        let timer = Arc::new(Mutex::new(DeterministicTimer::new(config)));
        let rng = Arc::new(Mutex::new(DeterministicRng::new(seed)));

        debug!("Initialized deterministic simulation with seed: {}", seed);

        Self {
            timer,
            rng,
            initial_seed: seed,
        }
    }

    /// Update simulation and return number of steps
    pub fn update(&self) -> u32 {
        self.timer.lock().update()
    }

    /// Get current tick
    pub fn tick(&self) -> u64 {
        self.timer.lock().tick()
    }

    /// Generate random f32
    pub fn gen_f32(&self) -> DeterministicFloat {
        self.rng.lock().gen_f32()
    }

    /// Generate random f64  
    pub fn gen_f64(&self) -> DeterministicDouble {
        self.rng.lock().gen_f64()
    }

    /// Generate random u32
    pub fn gen_u32(&self) -> u32 {
        self.rng.lock().gen_u32()
    }

    /// Generate random u64
    pub fn gen_u64(&self) -> u64 {
        self.rng.lock().gen_u64()
    }

    /// Reset simulation to initial state
    pub fn reset(&self) {
        self.timer.lock().reset();
        self.rng.lock().reset();
        debug!("Reset simulation to initial state");
    }

    /// Get simulation state for save/load
    pub fn state(&self) -> SimulationSnapshot {
        let timer = self.timer.lock();
        let rng = self.rng.lock();
        
        SimulationSnapshot {
            tick: timer.tick(),
            seed: rng.seed(),
            timestep_nanos: timer.timestep().as_nanos() as u64,
        }
    }

    /// Restore from snapshot
    pub fn restore(&self, snapshot: &SimulationSnapshot) {
        // Note: This would require serializing ChaCha8Rng state
        // For now, we can only restore from initial seed
        warn!("Full RNG state restoration not implemented - resetting to initial seed");
        self.reset();
        debug!("Restored simulation from snapshot at tick {}", snapshot.tick);
    }
}

/// Serializable simulation snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationSnapshot {
    pub tick: u64,
    pub seed: u64,
    pub timestep_nanos: u64,
}

/// Get current real-world time
pub fn get_current_time() -> DateTime<Utc> {
    Utc::now()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_deterministic_rng() {
        let mut rng1 = DeterministicRng::new(42);
        let mut rng2 = DeterministicRng::new(42);
        
        // Same seed should produce same sequence
        assert_eq!(rng1.gen_u32(), rng2.gen_u32());
        assert_eq!(rng1.gen_f32(), rng2.gen_f32());
    }

    #[test]
    fn test_deterministic_timer() {
        let config = FixedTimestepConfig {
            target_ups: 10,
            use_spin_sleep: false, // Disable for testing
            ..Default::default()
        };
        
        let mut timer = DeterministicTimer::new(config);
        assert_eq!(timer.tick(), 0);
        
        // Simulate passage of time
        thread::sleep(Duration::from_millis(150)); // 1.5 steps
        let steps = timer.update();
        
        assert!(steps >= 1);
        assert!(timer.tick() >= 1);
    }

    #[test]
    fn test_simulation_state() {
        let sim = SimulationState::new(123, None);
        assert_eq!(sim.tick(), 0);
        
        // Test RNG determinism
        let val1 = sim.gen_u32();
        sim.reset();
        let val2 = sim.gen_u32();
        assert_eq!(val1, val2);
    }

    #[test]
    fn test_deterministic_float() {
        let a = det_f32(1.0);
        let b = det_f32(2.0);
        let c = a + b;
        
        assert_eq!(c, det_f32(3.0));
        assert!(a < b);
    }
}

