use futures::SinkExt;
use nym_vpn_lib::gateway_client::{EntryPoint, ExitPoint};
use nym_vpn_lib::{NymVpnCtrlMessage, NymVpnHandle};
use tauri::{Manager, State};
use tracing::{debug, error, info, instrument, trace};

use crate::{
    error::{CmdError, CmdErrorSource},
    states::{
        app::{ConnectionState, VpnMode},
        SharedAppConfig, SharedAppData, SharedAppState,
    },
    vpn_client::{
        create_vpn_config, spawn_exit_listener, spawn_status_listener, ConnectProgressMsg,
        ConnectionEventPayload, ProgressEventPayload, EVENT_CONNECTION_PROGRESS,
        EVENT_CONNECTION_STATE,
    },
};

const DEFAULT_NODE_LOCATION: &str = "DE";

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
    config_store: State<'_, SharedAppConfig>,
) -> Result<ConnectionState, CmdError> {
    debug!("connect");
    {
        let mut app_state = state.lock().await;
        if app_state.state != ConnectionState::Disconnected {
            return Err(CmdError::new(
                CmdErrorSource::CallerError,
                format!("cannot connect from state {:?}", app_state.state),
            ));
        };

        // switch to "Connecting" state
        trace!("update connection state [Connecting]");
        app_state.state = ConnectionState::Connecting;
    }

    debug!("sending event [{}]: Connecting", EVENT_CONNECTION_STATE);
    app.emit_all(
        EVENT_CONNECTION_STATE,
        ConnectionEventPayload::new(ConnectionState::Connecting, None, None),
    )
    .ok();

    trace!(
        "sending event [{}]: Initializing",
        EVENT_CONNECTION_PROGRESS
    );
    app.emit_all(
        EVENT_CONNECTION_PROGRESS,
        ProgressEventPayload {
            key: ConnectProgressMsg::Initializing,
        },
    )
    .ok();

    let app_state = state.lock().await;

    let entry_point = match app_state.entry_node_location {
        Some(ref entry_node_location) => {
            debug!("entry node location set, using: {}", entry_node_location.code);
            EntryPoint::Location(entry_node_location.code.clone())
        }
        _ => {
            debug!(
                "entry node location not set, using default: {}",
                DEFAULT_NODE_LOCATION
            );
            EntryPoint::Location(DEFAULT_NODE_LOCATION.into())
        }
    };
    let exit_point = match app_state.exit_node_location {
        Some(ref exit_node_location) => {
            debug!("exit node location set, using: {}", exit_node_location.code);
            ExitPoint::Location(exit_node_location.code.clone())
        }
        _ => {
            debug!(
                "exit node location not set, using default: {}",
                DEFAULT_NODE_LOCATION
            );
            ExitPoint::Location(DEFAULT_NODE_LOCATION.into())
        }
    };

    let mut vpn_config = create_vpn_config(entry_point, exit_point);
    if let VpnMode::TwoHop = app_state.vpn_mode {
        info!("2-hop mode enabled");
        vpn_config.enable_two_hop = true;
    } else {
        info!("5-hop mode enabled");
    }
    // vpn_config.disable_routing = true;
    // !! release app_state mutex
    drop(app_state);

    // spawn the VPN client and start a new connection
    let NymVpnHandle {
        vpn_ctrl_tx,
        vpn_status_rx,
        vpn_exit_rx,
    } = nym_vpn_lib::spawn_nym_vpn(vpn_config).map_err(|e| {
        let err_message = format!("fail to initialize Nym VPN client: {}", e);
        error!(err_message);
        debug!("sending event [{}]: Disconnected", EVENT_CONNECTION_STATE);
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
    info!("nym vpn client spawned");
    trace!("sending event [{}]: InitDone", EVENT_CONNECTION_PROGRESS);
    app.emit_all(
        EVENT_CONNECTION_PROGRESS,
        ProgressEventPayload {
            key: ConnectProgressMsg::InitDone,
        },
    )
    .ok();

    // Start exit message listener
    // This will listen for the (single) exit message from the VPN client and update the UI accordingly
    debug!("starting exit listener");
    spawn_exit_listener(app.clone(), state.inner().clone(), vpn_exit_rx)
        .await
        .ok();

    // Start the VPN status listener
    // This will listen for status messages from the VPN client and update the UI accordingly
    debug!("starting status listener");
    spawn_status_listener(app, state.inner().clone(), vpn_status_rx)
        .await
        .ok();

    // Store the vpn control tx in the app state, which will be used to send control messages to
    // the running background VPN task, such as to disconnect.
    trace!("added vpn_ctrl_tx to app state");
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
    let mut app_state = state.lock().await;
    if app_state.state != ConnectionState::Connected {
        return Err(CmdError::new(
            CmdErrorSource::CallerError,
            format!("cannot disconnect from state {:?}", app_state.state),
        ));
    };

    // switch to "Disconnecting" state
    trace!("update connection state [Disconnecting]");
    app_state.state = ConnectionState::Disconnecting;

    debug!("sending event [{}]: Disconnecting", EVENT_CONNECTION_STATE);
    app.emit_all(
        EVENT_CONNECTION_STATE,
        ConnectionEventPayload::new(ConnectionState::Disconnecting, None, None),
    )
    .ok();

    let Some(ref mut vpn_tx) = app_state.vpn_ctrl_tx else {
        trace!("update connection state [Disconnected]");
        app_state.state = ConnectionState::Disconnected;
        app_state.connection_start_time = None;
        debug!("sending event [{}]: Disconnected", EVENT_CONNECTION_STATE);
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
    debug!("sending Stop message to VPN client");
    vpn_tx.send(NymVpnCtrlMessage::Stop).await.map_err(|e| {
        let err_message = format!("failed to send Stop message to VPN client: {}", e);
        error!(err_message);
        debug!("sending event [{}]: Disconnected", EVENT_CONNECTION_STATE);
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
    debug!("Stop message sent");

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
