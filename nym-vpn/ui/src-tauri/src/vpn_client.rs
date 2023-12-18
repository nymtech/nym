use crate::states::{app::ConnectionState, SharedAppState};
use anyhow::Result;
use futures::channel::oneshot::Receiver as OneshotReceiver;
use futures::StreamExt;
use nym_vpn_lib::gateway_client::{Config as GatewayClientConfig, EntryPoint, ExitPoint};
use nym_vpn_lib::nym_config::OptionalSet;
use nym_vpn_lib::{NymVpn, NymVpnExitError, NymVpnExitStatusMessage, StatusReceiver};
use tauri::Manager;
use time::OffsetDateTime;
use tracing::{debug, error, info, instrument, trace};

pub const EVENT_CONNECTION_STATE: &str = "connection-state";
pub const EVENT_CONNECTION_PROGRESS: &str = "connection-progress";

#[derive(Clone, serde::Serialize)]
pub enum ConnectProgressMsg {
    Initializing,
    InitDone,
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

fn handle_vpn_exit_error(e: Box<dyn std::error::Error + Send + Sync>) -> String {
    match e.downcast::<Box<NymVpnExitError>>() {
        Ok(e) => {
            // TODO The double boxing here is unexpected, we should look into that
            match **e {
                NymVpnExitError::Generic { reason } => reason.to_string(),
                NymVpnExitError::FailedToResetFirewallPolicy { reason } => reason.to_string(),
                NymVpnExitError::FailedToResetDnsMonitor { reason } => reason.to_string(),
            }
        }
        Err(e) => format!("unknown error: {e}"),
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
                debug!("received vpn exit message: {res:?}");
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
                    NymVpnExitStatusMessage::Failed(e) => {
                        let error = handle_vpn_exit_error(e);
                        debug!(
                            "vpn failed, sending event [{}]: disconnected",
                            EVENT_CONNECTION_STATE
                        );
                        app.emit_all(
                            EVENT_CONNECTION_STATE,
                            ConnectionEventPayload::new(
                                ConnectionState::Disconnected,
                                Some(error),
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
    app: tauri::AppHandle,
    app_state: SharedAppState,
    mut status_rx: StatusReceiver,
) -> Result<()> {
    tokio::spawn(async move {
        while let Some(msg) = status_rx.next().await {
            info!("received vpn status message: {msg:?}");
            if "Ready" == msg.to_string().as_str() {
                info!("vpn connection has been established");
                let now = OffsetDateTime::now_utc();
                {
                    let mut state = app_state.lock().await;
                    trace!("update connection state [Connected]");
                    state.state = ConnectionState::Connected;
                    state.connection_start_time = Some(now);
                }
                debug!("sending event [{}]: Connected", EVENT_CONNECTION_STATE);
                app.emit_all(
                    EVENT_CONNECTION_STATE,
                    ConnectionEventPayload::new(
                        ConnectionState::Connected,
                        None,
                        Some(now.unix_timestamp()),
                    ),
                )
                .ok();
            }
        }
        info!("vpn status listener has exited");
    });
    Ok(())
}

fn setup_gateway_client_config(private_key: Option<&str>) -> GatewayClientConfig {
    let mut config = GatewayClientConfig::default()
        // Read in the environment variable NYM_API if it exists
        .with_optional_env(GatewayClientConfig::with_custom_api_url, None, "NYM_API");
    info!("Using nym-api: {}", config.api_url());

    if let Some(key) = private_key {
        config = config.with_local_private_key(key.into());
    }
    config
}

#[instrument(skip_all)]
pub fn create_vpn_config(entry_point: EntryPoint, exit_point: ExitPoint) -> NymVpn {
    let mut nym_vpn = NymVpn::new(entry_point, exit_point);
    nym_vpn.gateway_config = setup_gateway_client_config(None);
    nym_vpn
}
