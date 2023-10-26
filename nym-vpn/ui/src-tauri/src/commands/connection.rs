use std::time::Duration;

use tauri::State;
use tokio::time::sleep;
use tracing::{debug, instrument, trace};

use crate::{
    error::CommandError,
    states::app::{ConnectionState, SharedAppState},
};

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
pub async fn connect(state: State<'_, SharedAppState>) -> Result<ConnectionState, CommandError> {
    debug!("connect");
    let mut app_state = state.lock().await;
    let ConnectionState::Disconnected = app_state.state else {
        return Err(CommandError::CallerError(format!(
            "cannot connect from state {:?}",
            app_state.state
        )));
    };

    // TODO fake some delay to establish connection
    let app_state_cloned = state.inner().clone();
    let task = tokio::spawn(async move {
        sleep(Duration::from_secs(2)).await;
        trace!("connected");
        app_state_cloned.lock().await.state = ConnectionState::Connected;
    });

    let _ = task.await;

    app_state.state = ConnectionState::Connecting;
    Ok(app_state.state)
}

#[instrument]
#[tauri::command]
pub async fn disconnect(state: State<'_, SharedAppState>) -> Result<ConnectionState, CommandError> {
    debug!("disconnect");
    let mut app_state = state.lock().await;
    let ConnectionState::Connected = app_state.state else {
        return Err(CommandError::CallerError(format!(
            "cannot disconnect from state {:?}",
            app_state.state
        )));
    };

    app_state.state = ConnectionState::Disconnecting;
    Ok(app_state.state)
}
