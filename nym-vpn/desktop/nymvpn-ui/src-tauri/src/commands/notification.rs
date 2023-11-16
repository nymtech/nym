use nymvpn_types::notification::Notification;

use crate::error::Error;

#[tauri::command]
pub async fn notifications() -> Result<Vec<Notification>, Error> {
    let mut client = nymvpn_controller::new_grpc_client()
        .await
        .map_err(|_| Error::DaemonIsOffline)?;

    Ok(client
        .get_notifications(())
        .await?
        .into_inner()
        .try_into()
        .unwrap())
}

#[tauri::command]
pub async fn ack_notification(id: String) -> Result<(), Error> {
    let mut client = nymvpn_controller::new_grpc_client()
        .await
        .map_err(|_| Error::DaemonIsOffline)?;

    Ok(client.ack_notification(id).await?.into_inner())
}
