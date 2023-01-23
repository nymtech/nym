use crate::error::Result;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
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

#[derive(Serialize, Deserialize, Debug)]
struct ConnectionSuccess {
    status: String,
}

#[tauri::command]
pub async fn run_health_check() -> bool {
    log::info!("Running network health check");
    match crate::operations::http::socks5_get::<_, ConnectionSuccess>(HEALTH_CHECK_URL).await {
        Ok(res) if res.status == "ok" => {
            log::error!("Healthcheck success!");
            true
        }
        Ok(res) => {
            log::error!("Healthcheck failed with status: {}", res.status);
            false
        }
        Err(err) => {
            log::error!("Healthcheck failed: {err}");
            false
        }
    }
}
