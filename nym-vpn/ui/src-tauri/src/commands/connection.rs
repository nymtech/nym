use std::time::Duration;

use tauri::{Manager, State};
use time::OffsetDateTime;
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
        ConnectionEventPayload::new(ConnectionState::Connecting, None, None),
    )
    .ok();

    // TODO fake some delay to establish connection
    let app_state_cloned = state.inner().clone();
    let task = tokio::spawn(async move {
        app.emit_all(
            EVENT_CONNECTION_PROGRESS,
            ProgressEventPayload {
                message: "Connecting to the network…".to_string(),
            },
        )
        .ok();
        sleep(Duration::from_millis(300)).await;
        app.emit_all(
            EVENT_CONNECTION_PROGRESS,
            ProgressEventPayload {
                message: "Fetching nodes and gateways…".to_string(),
            },
        )
        .ok();
        sleep(Duration::from_millis(400)).await;
        app.emit_all(
            EVENT_CONNECTION_PROGRESS,
            ProgressEventPayload {
                message: "Done".to_string(),
            },
        )
        .ok();
        sleep(Duration::from_millis(200)).await;
        trace!("connected");
        let now = OffsetDateTime::now_utc();
        let mut state = app_state_cloned.lock().await;
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
        ConnectionEventPayload::new(ConnectionState::Disconnecting, None, None),
    )
    .ok();

    // TODO fake some delay to confirm disconnection
    let app_state_cloned = state.inner().clone();
    let task = tokio::spawn(async move {
        sleep(Duration::from_secs(1)).await;
        trace!("disconnected");

        let mut state = app_state_cloned.lock().await;
        state.state = ConnectionState::Disconnected;
        state.connection_start_time = None;

        debug!("sending event [{}]: disconnected", EVENT_CONNECTION_STATE);
        app.emit_all(
            EVENT_CONNECTION_STATE,
            ConnectionEventPayload::new(ConnectionState::Disconnected, None, None),
        )
        .ok();
    });

    let _ = task.await;

    let app_state = state.lock().await;
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
