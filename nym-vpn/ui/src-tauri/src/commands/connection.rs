use std::time::Duration;

use tauri::{Manager, State};
use tokio::time::sleep;
use tracing::{debug, instrument, trace};

use crate::{
    error::CommandError,
    states::{app::ConnectionState, SharedAppState},
};

const EVENT_CONNECTION: &str = "connection-state";

#[derive(Clone, serde::Serialize)]
struct EventPayload {
    state: ConnectionState,
}

#[instrument(skip_all)]
#[tauri::command]
pub async fn get_connection_state(
    state: State<'_, SharedAppState>,
) -> Result<ConnectionState, CommandError> {
    debug!("get_connection_state");
    let app_state = state.lock().await;
    Ok(app_state.state)
}

#[instrument(skip_all)]
#[tauri::command]
pub async fn connect(
    app: tauri::AppHandle,
    state: State<'_, SharedAppState>,
) -> Result<ConnectionState, CommandError> {
    debug!("connect");
    let mut app_state = state.lock().await;
    let ConnectionState::Disconnected = app_state.state else {
        return Err(CommandError::CallerError(format!(
            "cannot connect from state {:?}",
            app_state.state
        )));
    };

    // switch to "Connecting" state
    app_state.state = ConnectionState::Connecting;
    // unlock the mutex
    drop(app_state);
    app.emit_all(
        EVENT_CONNECTION,
        EventPayload {
            state: ConnectionState::Connecting,
        },
    )
    .ok();

    // TODO fake some delay to establish connection
    let app_state_cloned = state.inner().clone();
    let task = tokio::spawn(async move {
        sleep(Duration::from_secs(2)).await;
        trace!("connected");
        app_state_cloned.lock().await.state = ConnectionState::Connected;
        debug!("sending event [{}]: connected", EVENT_CONNECTION);
        app.emit_all(
            EVENT_CONNECTION,
            EventPayload {
                state: ConnectionState::Connected,
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
) -> Result<ConnectionState, CommandError> {
    debug!("disconnect");
    let mut app_state = state.lock().await;
    let ConnectionState::Connected = app_state.state else {
        return Err(CommandError::CallerError(format!(
            "cannot disconnect from state {:?}",
            app_state.state
        )));
    };

    // switch to "Disconnecting" state
    app_state.state = ConnectionState::Disconnecting;
    // unlock the mutex
    drop(app_state);
    app.emit_all(
        EVENT_CONNECTION,
        EventPayload {
            state: ConnectionState::Disconnecting,
        },
    )
    .ok();

    // TODO fake some delay to confirm disconnection
    let app_state_cloned = state.inner().clone();
    let task = tokio::spawn(async move {
        sleep(Duration::from_secs(2)).await;
        trace!("disconnected");
        app_state_cloned.lock().await.state = ConnectionState::Disconnected;
        debug!("sending event [{}]: disconnected", EVENT_CONNECTION);
        app.emit_all(
            EVENT_CONNECTION,
            EventPayload {
                state: ConnectionState::Disconnected,
            },
        )
        .ok();
    });

    let _ = task.await;

    let app_state = state.lock().await;
    Ok(app_state.state)
}
