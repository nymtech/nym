use tracing::{debug, instrument};

pub mod connection;
pub mod settings;

#[instrument]
#[tauri::command]
pub fn greet(name: &str) -> String {
    debug!("greet");
    format!("Hello, {}! You've been greeted from Rust!", name)
}
