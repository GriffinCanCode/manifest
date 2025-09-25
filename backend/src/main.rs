// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{Builder, WebviewUrl, WebviewWindowBuilder};
use tracing::{info, Level};
use tracing_subscriber;

mod commands;
mod core;

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("Starting Manifest");

    Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            info!("Setting up Tauri application");
            
            // Create the main game window
            let _main_window = WebviewWindowBuilder::new(
                app,
                "main",
                WebviewUrl::App("index.html".into())
            )
            .title("Manifest")
            .resizable(true)
            .maximized(true)
            .min_inner_size(1200.0, 800.0)
            .center()
            .build()?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::greet,
            commands::get_game_state,
            commands::initialize_game
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
