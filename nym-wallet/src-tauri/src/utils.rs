use crate::error::BackendError;
use crate::nymd_client;
use crate::state::State;
use nym_wallet_types::app::AppEnv;
use std::sync::Arc;
use tokio::sync::RwLock;

fn get_env_as_option(key: &str) -> Option<String> {
  match ::std::env::var(key) {
    Ok(res) => Some(res),
    Err(_e) => None,
  }
}

#[tauri::command]
pub fn get_env() -> AppEnv {
  AppEnv {
    ADMIN_ADDRESS: get_env_as_option("ADMIN_ADDRESS"),
    SHOW_TERMINAL: get_env_as_option("SHOW_TERMINAL"),
  }
}

#[tauri::command]
pub async fn owns_mixnode(
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<bool, BackendError> {
  Ok(
    nymd_client!(state)
      .owns_mixnode(nymd_client!(state).address())
      .await?
      .is_some(),
  )
}

#[tauri::command]
pub async fn owns_gateway(
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<bool, BackendError> {
  Ok(
    nymd_client!(state)
      .owns_gateway(nymd_client!(state).address())
      .await?
      .is_some(),
  )
}
