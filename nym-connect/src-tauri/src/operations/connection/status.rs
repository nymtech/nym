use crate::error::Result;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::models::ConnectionStatusKind;
use crate::state::State;

static HEALTH_CHECK_URL: &str = "https://nymtech.net/.wellknown/connect/healthcheck.json";

#[tauri::command]
pub async fn get_connection_status(
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<ConnectionStatusKind> {
    let state = state.read().await;
    Ok(state.get_status())
}

#[tauri::command]
pub async fn run_health_check() -> bool {
    log::trace!("Running network health check");
    match crate::operations::http::socks5_get::<_, serde_json::Value>(HEALTH_CHECK_URL).await {
        Ok(_) => {
            log::info!("Healthcheck success!");
            true
        }
        Err(err) => {
            log::error!("Healthcheck failed: {err}");
            false
        }
    }
}
