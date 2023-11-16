use nymvpn_types::{location::Location, vpn_session::VpnStatus};

use crate::error::Error;

#[tauri::command]
pub async fn connect(location: Location) -> Result<VpnStatus, Error> {
    let mut client = nymvpn_controller::new_grpc_client()
        .await
        .map_err(|_| Error::DaemonIsOffline)?;

    Ok(client
        .connect_vpn(nymvpn_controller::proto::Location::from(location))
        .await?
        .into_inner()
        .into())
}

#[tauri::command]
pub async fn disconnect() -> Result<VpnStatus, Error> {
    let mut client = nymvpn_controller::new_grpc_client()
        .await
        .map_err(|_| Error::DaemonIsOffline)?;

    Ok(client.disconnect_vpn(()).await?.into_inner().into())
}

#[tauri::command]
pub async fn get_vpn_status() -> Result<VpnStatus, Error> {
    let mut client = nymvpn_controller::new_grpc_client()
        .await
        .map_err(|_| Error::DaemonIsOffline)?;

    Ok(client.get_vpn_status(()).await?.into_inner().into())
}
