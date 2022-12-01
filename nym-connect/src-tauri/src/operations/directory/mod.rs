use crate::error::Result;
use crate::models::DirectoryService;

//lets host our own service file locally
static SERVICE_PROVIDER_WELLKNOWN_URL: &str = "http://localhost:3000/service-providers.json";

#[tauri::command]
pub async fn get_services() -> Result<Vec<DirectoryService>> {
    log::trace!("Fetching services");
    let res = reqwest::get(SERVICE_PROVIDER_WELLKNOWN_URL)
        .await?
        .json::<Vec<DirectoryService>>()
        .await?;
    log::trace!("Received: {:#?}", res);
    Ok(res)
}
