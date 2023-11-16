use std::sync::Arc;

use tauri::AppHandle;
use nymvpn_types::location::Location;

use crate::event_forwarder::EventForwarderHandler;

pub type AppState = Arc<tokio::sync::Mutex<UiState>>;

#[derive(Debug, Default)]
pub struct UiState {
    pub event_fwd_handler: Option<EventForwarderHandler>,
    pub locations: Vec<Location>,
}

impl UiState {
    pub async fn start_event_forwarder(&mut self, app_handle: AppHandle) {
        if self.event_fwd_handler.is_none() {
            log::info!("starting event forwarder");
            let event_fwd_handler = EventForwarderHandler::start(app_handle).await;
            self.event_fwd_handler = Some(event_fwd_handler);
        }
    }

    pub async fn stop_event_forwarder(&mut self) {
        if let Some(event_fwd_handler) = self.event_fwd_handler.take() {
            log::info!("stopping event forwarder");
            drop(event_fwd_handler);
        }
    }
}
