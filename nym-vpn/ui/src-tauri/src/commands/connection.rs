use std::time::Duration;

use tauri::{Manager, State};
use tokio::time::sleep;
use tracing::{debug, instrument, trace};

use crate::{
    error::{CmdError, CmdErrorSource},
    states::{app::ConnectionState, SharedAppState},
};

const EVENT_CONNECTION_STATE: &str = "connection-state";
const EVENT_CONNECTION_PROGRESS: &str = "connection-progress";

#[derive(Clone, serde::Serialize)]
struct ConnectionEventPayload {
    state: ConnectionState,
    error: Option<String>,
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
) -> Result<ConnectionState, CmdError> {
    debug!("connect");
    let mut app_state = state.lock().await;
    let ConnectionState::Disconnected = app_state.state else {
        return Err(CmdError::new(
            CmdErrorSource::CallerError,
            format!("cannot connect from state {:?}", app_state.state),
        ));
    };

    // switch to "Connecting" state
    app_state.state = ConnectionState::Connecting;
    // unlock the mutex
    drop(app_state);
    app.emit_all(
        EVENT_CONNECTION_STATE,
        ConnectionEventPayload {
            state: ConnectionState::Connecting,
            error: None,
        },
    )
    .ok();

    // TODO fake some delay to establish connection
    let app_state_cloned = state.inner().clone();
    let task = tokio::spawn(async move {
        app.emit_all(
            EVENT_CONNECTION_PROGRESS,
            ProgressEventPayload {
                message: "Connecting to the network...".to_string(),
            },
        )
        .ok();
        sleep(Duration::from_secs(2)).await;
        trace!("connected");
        app_state_cloned.lock().await.state = ConnectionState::Connected;
        debug!("sending event [{}]: connected", EVENT_CONNECTION_STATE);
        app.emit_all(
            EVENT_CONNECTION_STATE,
            ConnectionEventPayload {
                state: ConnectionState::Connected,
                error: None,
            },
        )
        .ok();
    });

    let _ = task.await;

    let app_state = state.lock().await;
    Ok(app_state.state)
}

#[instrument(skip_all)]
#[tauri::command]
pub async fn disconnect(
    app: tauri::AppHandle,
    state: State<'_, SharedAppState>,
) -> Result<ConnectionState, CmdError> {
    debug!("disconnect");
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
    drop(app_state);
    app.emit_all(
        EVENT_CONNECTION_STATE,
        ConnectionEventPayload {
            state: ConnectionState::Disconnecting,
            error: None,
        },
    )
    .ok();

    // TODO fake some delay to confirm disconnection
    let app_state_cloned = state.inner().clone();
    let task = tokio::spawn(async move {
        sleep(Duration::from_secs(2)).await;
        trace!("disconnected");
        app_state_cloned.lock().await.state = ConnectionState::Disconnected;
        debug!("sending event [{}]: disconnected", EVENT_CONNECTION_STATE);
        app.emit_all(
            EVENT_CONNECTION_STATE,
            ConnectionEventPayload {
                state: ConnectionState::Disconnected,
                error: None,
            },
        )
        .ok();
    });

    let _ = task.await;

    let app_state = state.lock().await;
    Ok(app_state.state)
}
