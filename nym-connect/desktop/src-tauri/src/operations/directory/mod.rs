use itertools::Itertools;

use crate::error::Result;
use crate::models::{
    DirectoryService, DirectoryServiceProvider, HarbourMasterService, PagedResult,
};
use crate::state::is_medium_enabled;
use nym_api_requests::models::GatewayBondAnnotated;
use nym_contracts_common::types::Percent;

static SERVICE_PROVIDER_WELLKNOWN_URL: &str =
    "https://nymtech.net/.wellknown/connect/service-providers.json";

// List of network-requesters running with medium toggle enabled, for testing
static SERVICE_PROVIDER_WELLKNOWN_URL_MEDIUM: &str =
    "https://nymtech.net/.wellknown/connect/service-providers-medium.json";

static HARBOUR_MASTER_URL: &str = "https://harbourmaster.nymtech.net/v1/services/?size=100";

static GATEWAYS_DETAILED_URL: &str =
    "https://validator.nymtech.net/api/v1/status/gateways/detailed";

fn get_services_url() -> &'static str {
    if is_medium_enabled() {
        return SERVICE_PROVIDER_WELLKNOWN_URL_MEDIUM;
    }
    SERVICE_PROVIDER_WELLKNOWN_URL
}

#[tauri::command]
pub async fn get_services() -> Result<Vec<DirectoryServiceProvider>> {
    log::trace!("Fetching services");
    let all_services = fetch_services().await?;
    log::trace!("Received: {:#?}", all_services);

    // Early return if we're running with medium toggle enabled
    if is_medium_enabled() {
        return Ok(all_services.into_iter().flat_map(|sp| sp.items).collect());
    }

    // TODO: get paged
    log::trace!("Fetching active services");
    let active_services = fetch_active_services().await?;
    log::trace!("Active: {:#?}", active_services);

    let filtered_services = filter_out_inactive(all_services, active_services);

    log::trace!("Fetching gateways");
    let gateway_res = get_gateways_detailed().await?;
    log::trace!("Received: {:#?}", gateway_res);

    // Use only services that are active AND have a performance of >= 90%
    let filtered_services_with_good_gateway =
        filter_out_poor_gateways(filtered_services, gateway_res);

    Ok(filtered_services_with_good_gateway)
}

fn filter_out_inactive(
    services_res: Vec<DirectoryService>,
    active_services: PagedResult<HarbourMasterService>,
) -> Vec<DirectoryService> {
    let mut filtered: Vec<DirectoryService> = vec![];
    for service_type in &services_res {
        let items = service_type
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
        log::trace!("service = {} has {} items", service_type.id, items.len());
        filtered.push(DirectoryService {
            id: service_type.id.clone(),
            description: service_type.description.clone(),
            items,
        })
    }
    filtered
}

fn filter_out_poor_gateways(
    services: Vec<DirectoryService>,
    gateway_res: Vec<GatewayBondAnnotated>,
) -> Vec<DirectoryServiceProvider> {
    let perf_threshold = Percent::from_percentage_value(90).unwrap();
    services
        .into_iter()
        .flat_map(|sp| sp.items)
        .filter(|sp| {
            gateway_res.iter().any(|gateway| {
                gateway.gateway_bond.gateway.identity_key == sp.gateway
                    && gateway.performance >= perf_threshold
            })
        })
        .collect()
}

async fn fetch_services() -> Result<Vec<DirectoryService>> {
    let services_url = get_services_url();
    let services_res = reqwest::get(services_url)
        .await?
        .json::<Vec<DirectoryService>>()
        .await?;
    Ok(services_res)
}

async fn fetch_active_services() -> Result<PagedResult<HarbourMasterService>> {
    let active_services = reqwest::get(HARBOUR_MASTER_URL)
        .await?
        .json::<PagedResult<HarbourMasterService>>()
        .await?;
    Ok(active_services)
}

#[tauri::command]
pub async fn get_gateways_detailed() -> Result<Vec<GatewayBondAnnotated>> {
    let res = reqwest::get(GATEWAYS_DETAILED_URL)
        .await?
        .json::<Vec<GatewayBondAnnotated>>()
        .await?;
    Ok(res)
}
