use crate::error::BackendError;
use crate::models::DirectoryService;

static SERVICE_PROVIDER_WELLKNOWN_URL: &str =
    "https://nymtech.net/.wellknown/connect/service-providers.json";

#[tauri::command]
pub async fn get_services() -> Result<Vec<DirectoryService>, BackendError> {
    let res = reqwest::get(SERVICE_PROVIDER_WELLKNOWN_URL)
        .await?
        .json::<Vec<DirectoryService>>()
        .await?;
    Ok(res)
}
