use crate::fs::config::AppConfig;
use crate::states::{app::ConnectionState, SharedAppState};
use anyhow::Result;
use futures::channel::oneshot::Receiver as OneshotReceiver;
use futures::{channel::mpsc::Receiver, StreamExt};
use nym_vpn_lib::gateway_client::Config as GatewayClientConfig;
use nym_vpn_lib::nym_config::OptionalSet;
use nym_vpn_lib::{NymVPN, NymVpnExitStatusMessage, NymVpnStatusMessage};
use tauri::Manager;
use tracing::{debug, error, info, instrument};

pub const EVENT_CONNECTION_STATE: &str = "connection-state";
pub const EVENT_CONNECTION_PROGRESS: &str = "connection-progress";

#[derive(Clone, serde::Serialize)]
pub enum ConnectProgressMsg {
    Initializing,
    Done,
}

#[derive(Clone, serde::Serialize)]
pub struct ProgressEventPayload {
    pub key: ConnectProgressMsg,
}

#[derive(Clone, serde::Serialize)]
pub struct ConnectionEventPayload {
    state: ConnectionState,
    error: Option<String>,
    start_time: Option<i64>, // unix timestamp in seconds
}

impl ConnectionEventPayload {
    pub fn new(state: ConnectionState, error: Option<String>, start_time: Option<i64>) -> Self {
        Self {
            state,
            error,
            start_time,
        }
    }
}

#[instrument(skip_all)]
pub async fn spawn_exit_listener(
    app: tauri::AppHandle,
    app_state: SharedAppState,
    exit_rx: OneshotReceiver<NymVpnExitStatusMessage>,
) -> Result<()> {
    tokio::spawn(async move {
        match exit_rx.await {
            Ok(res) => {
                info!("received vpn exit message: {res:?}");
                match res {
                    NymVpnExitStatusMessage::Stopped => {
                        info!("vpn connection stopped");
                        debug!(
                            "vpn stopped, sending event [{}]: disconnected",
                            EVENT_CONNECTION_STATE
                        );
                        app.emit_all(
                            EVENT_CONNECTION_STATE,
                            ConnectionEventPayload::new(ConnectionState::Disconnected, None, None),
                        )
                        .ok();
                    }
                    NymVpnExitStatusMessage::Failed => {
                        info!("vpn connection failed");
                        debug!(
                            "vpn failed, sending event [{}]: disconnected",
                            EVENT_CONNECTION_STATE
                        );
                        app.emit_all(
                            EVENT_CONNECTION_STATE,
                            ConnectionEventPayload::new(
                                ConnectionState::Disconnected,
                                Some("vpn connection failed".to_string()),
                                None,
                            ),
                        )
                        .ok();
                    }
                }
            }
            Err(e) => {
                error!("vpn_exit_rx failed to receive exit message: {}", e);
                app.emit_all(
                    EVENT_CONNECTION_STATE,
                    ConnectionEventPayload::new(
                        ConnectionState::Disconnected,
                        Some("exit channel with vpn client has been closed".to_string()),
                        None,
                    ),
                )
                .ok();
            }
        }
        // update the connection state
        let mut state = app_state.lock().await;
        state.state = ConnectionState::Disconnected;
        state.connection_start_time = None;
        info!("vpn exit listener has exited");
    });
    Ok(())
}

#[instrument(skip_all)]
pub async fn spawn_status_listener(
    // app: tauri::AppHandle,
    // app_state: SharedAppState,
    mut status_rx: Receiver<NymVpnStatusMessage>,
) -> Result<()> {
    tokio::spawn(async move {
        while let Some(msg) = status_rx.next().await {
            info!("received vpn status message: {msg:?}");
            match msg {
                nym_vpn_lib::NymVpnStatusMessage::Slow => todo!(),
            }
        }
        info!("vpn status listener has exited");
    });
    Ok(())
}

fn setup_gateway_client_config(private_key: Option<&str>, nym_api: &str) -> GatewayClientConfig {
    let mut config = GatewayClientConfig::default()
        // .with_custom_api_url(nym_config::defaults::mainnet::NYM_API.parse().unwrap())
        .with_custom_api_url(nym_api.parse().unwrap())
        // Read in the environment variable NYM_API if it exists
        .with_optional_env(GatewayClientConfig::with_custom_api_url, None, "NYM_API");
    info!("Using nym-api: {}", config.api_url());

    if let Some(key) = private_key {
        config = config.with_local_private_key(key.into());
    }
    config
}

#[instrument(skip_all)]
pub fn create_vpn_config(app_config: &AppConfig) -> NymVPN {
    let mut nym_vpn = NymVPN::new(&app_config.entry_gateway, &app_config.exit_router);
    nym_vpn.gateway_config = setup_gateway_client_config(None, &app_config.nym_api);
    nym_vpn
}
