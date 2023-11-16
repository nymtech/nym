use tauri::AppHandle;
use nymvpn_controller::proto::SignInRequest;

use crate::{error::Error, state::AppState};

#[tauri::command]
pub async fn sign_in(
    email: String,
    password: String,
    app_handle: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), Error> {
    let mut client = nymvpn_controller::new_grpc_client()
        .await
        .map_err(|_| Error::DaemonIsOffline)?;

    let req = SignInRequest { email, password };

    let _ = client.account_sign_in(req).await?;

    {
        //start event forwarder
        let mut guard = state.lock().await;
        guard.start_event_forwarder(app_handle).await;
    }

    Ok(())
}

#[tauri::command]
pub async fn sign_out(state: tauri::State<'_, AppState>) -> Result<(), Error> {
    let mut client = nymvpn_controller::new_grpc_client()
        .await
        .map_err(|_| Error::DaemonIsOffline)?;

    let _ = client.account_sign_out(()).await?;

    {
        // stop event forwarder
        let mut guard = state.lock().await;
        guard.stop_event_forwarder().await;
    }

    Ok(())
}

#[tauri::command]
pub async fn is_signed_in(
    app_handle: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<bool, Error> {
    let mut client = nymvpn_controller::new_grpc_client()
        .await
        .map_err(|_| Error::DaemonIsOffline)?;

    let is_authenticated = client.is_authenticated(()).await.map(|r| r.into_inner())?;

    {
        // start event forwarder if needed
        let mut guard = state.lock().await;
        if is_authenticated {
            guard.start_event_forwarder(app_handle).await;
        } else {
            guard.stop_event_forwarder().await;
        }
    }

    Ok(is_authenticated)
}
