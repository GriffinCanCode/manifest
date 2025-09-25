use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreGameState {
    pub turn: u32,
    pub tick: u64,
    pub speed: GameSpeed,
    pub is_paused: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameSpeed {
    Paused,
    Slow,
    Normal,
    Fast,
    VeryFast,
}

impl Default for CoreGameState {
    fn default() -> Self {
        Self {
            turn: 1,
            tick: 0,
            speed: GameSpeed::Normal,
            is_paused: false,
        }
    }
}

impl CoreGameState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn advance_turn(&mut self) {
        self.turn += 1;
    }

    pub fn advance_tick(&mut self) {
        self.tick += 1;
    }

    pub fn pause(&mut self) {
        self.is_paused = true;
    }

    pub fn resume(&mut self) {
        self.is_paused = false;
    }

    pub fn set_speed(&mut self, speed: GameSpeed) {
        self.speed = speed;
    }
}
