use futures::{SinkExt, StreamExt};
use nym_vpn_lib::{NymVpnCtrlMessage, NymVpnExitStatusMessage, NymVpnHandle};
use tauri::{Manager, State};
use time::OffsetDateTime;
use tracing::{debug, error, info, instrument};

use crate::{
    error::{CmdError, CmdErrorSource},
    states::{
        app::{ConnectionState, VpnMode},
        NymVPNState, SharedAppData, SharedAppState,
    },
};

const EVENT_CONNECTION_STATE: &str = "connection-state";
const EVENT_CONNECTION_PROGRESS: &str = "connection-progress";

#[derive(Clone, serde::Serialize)]
struct ConnectionEventPayload {
    state: ConnectionState,
    error: Option<String>,
    start_time: Option<i64>, // unix timestamp in seconds
}

impl ConnectionEventPayload {
    fn new(state: ConnectionState, error: Option<String>, start_time: Option<i64>) -> Self {
        Self {
            state,
            error,
            start_time,
        }
    }
}

#[derive(Clone, serde::Serialize)]
struct ProgressEventPayload {
    message: String,
}

#[instrument(skip_all)]
#[tauri::command]
pub async fn get_connection_state(
    state: State<'_, SharedAppState>,
) -> Result<ConnectionState, CmdError> {
    debug!("get_connection_state");
    let app_state = state.lock().await;
    Ok(app_state.state)
}

#[instrument(skip_all)]
#[tauri::command]
pub async fn connect(
    app: tauri::AppHandle,
    state: State<'_, SharedAppState>,
    nymvpn_state: State<'_, NymVPNState>,
) -> Result<ConnectionState, CmdError> {
    debug!("connect");
    {
        let mut app_state = state.lock().await;
        let ConnectionState::Disconnected = app_state.state else {
            return Err(CmdError::new(
                CmdErrorSource::CallerError,
                format!("cannot connect from state {:?}", app_state.state),
            ));
        };

        // switch to "Connecting" state
        app_state.state = ConnectionState::Connecting;
    }
    app.emit_all(
        EVENT_CONNECTION_STATE,
        ConnectionEventPayload::new(ConnectionState::Connecting, None, None),
    )
    .ok();

    app.emit_all(
        EVENT_CONNECTION_PROGRESS,
        ProgressEventPayload {
            message: "Initializing Nym VPN client…".to_string(),
        },
    )
    .ok();

    let nymvpn_state_cloned = nymvpn_state.inner().clone();
    let local_nymvpn = nymvpn_state_cloned.lock().await.clone();

    // spawn the VPN client and start a new connection
    let NymVpnHandle {
        vpn_ctrl_tx,
        mut vpn_status_rx,
        vpn_exit_rx,
    } = nym_vpn_lib::spawn_nym_vpn(local_nymvpn).map_err(|e| {
        let err_message = format!("fail to initialize Nym VPN client: {}", e);
        error!(err_message);
        app.emit_all(
            EVENT_CONNECTION_STATE,
            ConnectionEventPayload::new(
                ConnectionState::Disconnected,
                Some(err_message.clone()),
                None,
            ),
        )
        .ok();
        CmdError::new(CmdErrorSource::InternalError, err_message)
    })?;

    app.emit_all(
        EVENT_CONNECTION_PROGRESS,
        ProgressEventPayload {
            message: "Done".to_string(),
        },
    )
    .ok();

    // update the connection state
    {
        let mut state = state.lock().await;
        let now = OffsetDateTime::now_utc();
        state.state = ConnectionState::Connected;
        state.connection_start_time = Some(now);
        debug!("sending event [{}]: connected", EVENT_CONNECTION_STATE);
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

    // Start exit message listener
    // This will listen for the (single) exit message from the VPN client and update the UI accordingly
    let local_app_state = state.inner().clone();
    tokio::spawn(async move {
        match vpn_exit_rx.await {
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
        let mut state = local_app_state.lock().await;
        state.state = ConnectionState::Disconnected;
        state.connection_start_time = None;
        info!("vpn exit listener has exited");
    });

    // Start the VPN status listener
    // This will listen for status messages from the VPN client and update the UI accordingly
    tokio::spawn(async move {
        while let Some(msg) = vpn_status_rx.next().await {
            info!("received vpn status message: {msg:?}");
            match msg {
                nym_vpn_lib::NymVpnStatusMessage::Slow => todo!(),
            }
        }
        info!("vpn status listener has exited");
    });

    // Store the vpn control tx in the app state, which will be used to send control messages to
    // the running background VPN task, such as to disconnect.
    let mut state = state.lock().await;
    state.vpn_ctrl_tx = Some(vpn_ctrl_tx);

    Ok(state.state)
}

#[instrument(skip_all)]
#[tauri::command]
pub async fn disconnect(
    app: tauri::AppHandle,
    state: State<'_, SharedAppState>,
) -> Result<ConnectionState, CmdError> {
    debug!("disconnect");
    {
        let mut app_state = state.lock().await;
        let ConnectionState::Connected = app_state.state else {
            return Err(CmdError::new(
                CmdErrorSource::CallerError,
                format!("cannot disconnect from state {:?}", app_state.state),
            ));
        };

        // switch to "Disconnecting" state
        app_state.state = ConnectionState::Disconnecting;
        // unlock the mutex
    }
    app.emit_all(
        EVENT_CONNECTION_STATE,
        ConnectionEventPayload::new(ConnectionState::Disconnecting, None, None),
    )
    .ok();

    let mut app_state = state.lock().await;
    let Some(ref mut vpn_tx) = app_state.vpn_ctrl_tx else {
        app_state.state = ConnectionState::Disconnected;
        app_state.connection_start_time = None;
        app.emit_all(
            EVENT_CONNECTION_STATE,
            ConnectionEventPayload::new(
                ConnectionState::Disconnected,
                Some("vpn handle has not been initialized".to_string()),
                None,
            ),
        )
        .ok();
        return Err(CmdError::new(
            CmdErrorSource::InternalError,
            "vpn handle has not been initialized".to_string(),
        ));
    };

    // send Stop message to the VPN client
    // TODO handle error case properly
    vpn_tx.send(NymVpnCtrlMessage::Stop).await.map_err(|e| {
        error!("failed to send Stop message to VPN client: {}", e);
        CmdError::new(
            CmdErrorSource::InternalError,
            "failed to send Stop message to VPN client".into(),
        )
    })?;

    app_state.state = ConnectionState::Disconnected;
    app_state.connection_start_time = None;

    debug!("sending event [{}]: disconnected", EVENT_CONNECTION_STATE);
    app.emit_all(
        EVENT_CONNECTION_STATE,
        ConnectionEventPayload::new(ConnectionState::Disconnected, None, None),
    )
    .ok();

    Ok(app_state.state)
}

#[instrument(skip_all)]
#[tauri::command]
pub async fn get_connection_start_time(
    state: State<'_, SharedAppState>,
) -> Result<Option<i64>, CmdError> {
    debug!("get_connection_start_time");
    let app_state = state.lock().await;
    Ok(app_state.connection_start_time.map(|t| t.unix_timestamp()))
}

#[instrument(skip(app_state, data_state))]
#[tauri::command]
pub async fn set_vpn_mode(
    app_state: State<'_, SharedAppState>,
    data_state: State<'_, SharedAppData>,
    mode: VpnMode,
) -> Result<(), CmdError> {
    debug!("set_vpn_mode");

    let mut state = app_state.lock().await;

    if let ConnectionState::Disconnected = state.state {
    } else {
        let err_message = format!("cannot change vpn mode from state {:?}", state.state);
        error!(err_message);
        return Err(CmdError::new(CmdErrorSource::CallerError, err_message));
    }
    state.vpn_mode = mode.clone();

    // save the selected mode to disk
    let mut app_data_store = data_state.lock().await;
    let mut app_data = app_data_store
        .read()
        .await
        .map_err(|e| CmdError::new(CmdErrorSource::InternalError, e.to_string()))?;
    app_data.vpn_mode = Some(mode);
    app_data_store.data = app_data;
    app_data_store
        .write()
        .await
        .map_err(|e| CmdError::new(CmdErrorSource::InternalError, e.to_string()))?;
    Ok(())
}
