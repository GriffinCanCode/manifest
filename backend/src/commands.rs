use serde::{Deserialize, Serialize};
use tauri::State;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub turn: u32,
    pub player_name: String,
    pub civilization: String,
    pub is_paused: bool,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            turn: 1,
            player_name: "Player".to_string(),
            civilization: "Ancient Empire".to_string(),
            is_paused: false,
        }
    }
}

#[tauri::command]
pub fn greet(name: &str) -> String {
    info!("Greeting called for: {}", name);
    format!("Hello, {}! Welcome to Manifest.", name)
}

#[tauri::command]
pub async fn get_game_state() -> Result<GameState, String> {
    info!("Getting game state");
    // For now, return a default state
    // Later this will interact with the actual game engine
    Ok(GameState::default())
}

#[tauri::command]
pub async fn initialize_game(player_name: String, civilization: String) -> Result<GameState, String> {
    info!("Initializing game for player: {} with civilization: {}", player_name, civilization);
    
    // Initialize game state
    let game_state = GameState {
        turn: 1,
        player_name,
        civilization,
        is_paused: false,
    };
    
    Ok(game_state)
}

#[tauri::command]
pub async fn save_game(save_name: String) -> Result<String, String> {
    info!("Saving game as: {}", save_name);
    // TODO: Implement actual save functionality
    Ok(format!("Game saved as: {}", save_name))
}

#[tauri::command]
pub async fn load_game(save_name: String) -> Result<GameState, String> {
    info!("Loading game: {}", save_name);
    // TODO: Implement actual load functionality
    Ok(GameState::default())
}
