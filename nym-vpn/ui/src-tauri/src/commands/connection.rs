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
struct Payload {
    state: ConnectionState,
}

#[instrument]
#[tauri::command]
pub async fn get_connection_state(
    state: State<'_, SharedAppState>,
) -> Result<ConnectionState, CommandError> {
    debug!("get_connection_state");
    let app_state = state.lock().await;
    Ok(app_state.state)
}

#[tauri::command]
pub async fn connect(
    app: tauri::AppHandle,
    state: State<'_, SharedAppState>,
) -> Result<ConnectionState, CommandError> {
    debug!("connect");
    let app_state = state.lock().await;
    let ConnectionState::Disconnected = app_state.state else {
        return Err(CommandError::CallerError(format!(
            "cannot connect from state {:?}",
            app_state.state
        )));
    };

    // switch to "Connecting" state
    let app_state_cloned = state.inner().clone();
    app_state_cloned.lock().await.state = ConnectionState::Connecting;
    app.emit_all(
        EVENT_CONNECTION,
        Payload {
            state: ConnectionState::Connecting,
        },
    )
    .ok();

    // TODO fake some delay to establish connection
    let task = tokio::spawn(async move {
        sleep(Duration::from_secs(2)).await;
        trace!("connected");
        app_state_cloned.lock().await.state = ConnectionState::Connected;
        app.emit_all(
            EVENT_CONNECTION,
            Payload {
                state: ConnectionState::Connected,
            },
        )
        .ok();
    });

    let _ = task.await;

    Ok(app_state.state)
}

#[instrument]
#[tauri::command]
pub async fn disconnect(
    app: tauri::AppHandle,
    state: State<'_, SharedAppState>,
) -> Result<ConnectionState, CommandError> {
    debug!("disconnect");
    let app_state = state.lock().await;
    let ConnectionState::Connected = app_state.state else {
        return Err(CommandError::CallerError(format!(
            "cannot disconnect from state {:?}",
            app_state.state
        )));
    };

    // switch to "Disconnecting" state
    let app_state_cloned = state.inner().clone();
    app_state_cloned.lock().await.state = ConnectionState::Disconnecting;
    app.emit_all(
        EVENT_CONNECTION,
        Payload {
            state: ConnectionState::Disconnecting,
        },
    )
    .ok();

    // TODO fake some delay to confirm disconnection
    let task = tokio::spawn(async move {
        sleep(Duration::from_secs(2)).await;
        trace!("disconnected");
        app_state_cloned.lock().await.state = ConnectionState::Disconnected;
        app.emit_all(
            EVENT_CONNECTION,
            Payload {
                state: ConnectionState::Disconnected,
            },
        )
        .ok();
    });

    let _ = task.await;

    Ok(app_state.state)
}
