use chrono::{DateTime, Utc};
use std::time::{Duration, Instant};

pub struct GameTimer {
    last_tick: Instant,
    target_fps: u32,
    frame_time: Duration,
    accumulated_time: Duration,
}

impl Default for GameTimer {
    fn default() -> Self {
        Self::new(60) // 60 FPS default
    }
}

impl GameTimer {
    pub fn new(target_fps: u32) -> Self {
        let frame_time = Duration::from_nanos(1_000_000_000 / target_fps as u64);
        Self {
            last_tick: Instant::now(),
            target_fps,
            frame_time,
            accumulated_time: Duration::ZERO,
        }
    }

    pub fn tick(&mut self) -> bool {
        let now = Instant::now();
        let delta = now - self.last_tick;
        self.last_tick = now;
        self.accumulated_time += delta;

        if self.accumulated_time >= self.frame_time {
            self.accumulated_time -= self.frame_time;
            true
        } else {
            false
        }
    }

    pub fn get_fps(&self) -> u32 {
        self.target_fps
    }

    pub fn set_fps(&mut self, fps: u32) {
        self.target_fps = fps;
        self.frame_time = Duration::from_nanos(1_000_000_000 / fps as u64);
    }
}

pub fn get_current_time() -> DateTime<Utc> {
    Utc::now()
}
