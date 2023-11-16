use log::info;
use semver::Version;
use nymvpn_config::config;

#[tauri::command]
pub async fn current_app_version() -> String {
    config().version().into()
}

#[tauri::command]
pub async fn update_available() -> bool {
    //todo: cache available version
    let current_version = config().version();
    if let Ok(mut client) = nymvpn_controller::new_grpc_client().await {
        if let Ok(latest_version) = client.latest_app_version(()).await {
            let latest_version = latest_version.into_inner();
            info!("current {current_version}, latest {latest_version}");
            if let (Ok(current_version), Ok(latest_version)) = (
                Version::parse(current_version),
                Version::parse(&latest_version),
            ) {
                return latest_version > current_version;
            }
        }
    };

    false
}
