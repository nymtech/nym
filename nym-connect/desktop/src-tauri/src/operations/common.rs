use crate::config::PrivacyLevel;
use crate::error::Result;
use crate::{config::UserData, state::State};
use std::env;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tauri::command]
pub async fn get_env(variable: String) -> Option<String> {
    let var = env::var(&variable).ok();
    log::trace!("get_env {variable} {:?}", var);

    var
}

#[tauri::command]
pub async fn get_user_data(state: tauri::State<'_, Arc<RwLock<State>>>) -> Result<UserData> {
    let guard = state.read().await;
    Ok(guard.get_user_data().clone())
}

#[tauri::command]
pub async fn set_monitoring(
    enabled: bool,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<()> {
    let mut guard = state.write().await;
    guard.set_monitoring(enabled)
}

#[tauri::command]
pub async fn set_privacy_level(
    privacy_level: PrivacyLevel,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<()> {
    let mut guard = state.write().await;
    guard.set_privacy_level(privacy_level)
}
