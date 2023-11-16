use nymvpn_types::location::Location;

use crate::{error::Error, state::AppState};

#[tauri::command]
pub async fn locations(state: tauri::State<'_, AppState>) -> Result<Vec<Location>, Error> {
    let mut guard = state.lock().await;

    if guard.locations.is_empty() {
        let mut client = nymvpn_controller::new_grpc_client()
            .await
            .map_err(|_| Error::DaemonIsOffline)?;
        // cache locations
        guard.locations = client.get_locations(()).await?.into_inner().into();
    }

    Ok(guard.locations.clone())
}

#[tauri::command]
pub async fn recent_locations() -> Result<Vec<Location>, Error> {
    let mut client = nymvpn_controller::new_grpc_client()
        .await
        .map_err(|_| Error::DaemonIsOffline)?;

    Ok(client.recent_locations(()).await?.into_inner().into())
}
