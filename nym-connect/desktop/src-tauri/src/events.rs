use std::sync::Arc;

use nym_client_core::error::ClientCoreStatusMessage;
use nym_task::manager::TaskStatus;
use tauri::async_runtime::RwLock;

use crate::{
    state::{GatewayConnectivity, State},
    tasks,
};

#[derive(Clone, serde::Serialize)]
struct Payload {
    title: String,
    message: String,
}

impl Payload {
    fn new(title: String, message: String) -> Self {
        Self { title, message }
    }
}

pub fn emit_event(event: &str, title: &str, msg: &str, window: &tauri::Window<tauri::Wry>) {
    if let Err(err) = window.emit(event, Payload::new(title.into(), msg.into())) {
        log::error!("Failed to emit tauri event: {err}");
    }
}

pub fn emit_status_event<T: ToString>(event: &str, msg: &T, window: &tauri::Window<tauri::Wry>) {
    if let Err(err) = window.emit(event, Payload::new("SOCKS5 update".into(), msg.to_string())) {
        log::error!("Failed to emit tauri event: {err}");
    }
}

pub async fn handle_task_status(
    task_status: &TaskStatus,
    state: &Arc<RwLock<State>>,
    window: &tauri::Window,
) {
    match task_status {
        TaskStatus::Ready | TaskStatus::ReadyWithGateway(_) => {
            {
                let mut state_w = state.write().await;
                state_w.mark_connected(window);
            }

            emit_status_event("socks5-connected-event", task_status, window);
            tasks::start_connection_check(state.clone(), window.clone());
        }
    }
}

pub async fn handle_client_status_message(
    client_status_message: &ClientCoreStatusMessage,
    state: &Arc<RwLock<State>>,
    window: &tauri::Window,
) {
    // TODO: use this instead once we change on the frontend too
    let _event_name = match client_status_message {
        ClientCoreStatusMessage::GatewayIsSlow | ClientCoreStatusMessage::GatewayIsVerySlow => {
            "socks5-gateway-status"
        }
    };

    if let Ok(connectivity) = GatewayConnectivity::try_from(client_status_message) {
        state.write().await.set_gateway_connectivity(connectivity);
    }

    emit_status_event("socks5-status-event", client_status_message, window);
}
