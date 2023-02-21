use itertools::Itertools;

use crate::error::Result;
use crate::models::{DirectoryService, HarbourMasterService, PagedResult};

static SERVICE_PROVIDER_WELLKNOWN_URL: &str =
    "https://nymtech.net/.wellknown/connect/service-providers.json";

static HARBOUR_MASTER_URL: &str = "https://harbourmaster.nymtech.net/v1/services/?size=100";

#[tauri::command]
pub async fn get_services() -> Result<Vec<DirectoryService>> {
    log::trace!("Fetching services");
    let res = reqwest::get(SERVICE_PROVIDER_WELLKNOWN_URL)
        .await?
        .json::<Vec<DirectoryService>>()
        .await?;
    log::trace!("Received: {:#?}", res);

    // TODO: get paged
    log::trace!("Fetching active services");
    let active_services = reqwest::get(HARBOUR_MASTER_URL)
        .await?
        .json::<PagedResult<HarbourMasterService>>()
        .await?;
    log::trace!("Active: {:#?}", active_services);

    let mut filtered: Vec<DirectoryService> = vec![];

    for service in &res {
        let items: _ = service
            .items
            .clone()
            .into_iter()
            .filter(|sp| {
                active_services
                    .items
                    .iter()
                    .any(|active| active.service_provider_client_id == sp.address)
            })
            .collect_vec();
        log::trace!("service = {} has {} items", service.id, items.len());
        filtered.push(DirectoryService {
            id: service.id.clone(),
            description: service.description.clone(),
            items,
        })
    }

    Ok(filtered)
}
