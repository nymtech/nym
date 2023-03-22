use itertools::Itertools;

use crate::error::Result;
use crate::models::{
    DirectoryService, DirectoryServiceProvider, HarbourMasterService, PagedResult,
};
use nym_api_requests::models::GatewayBondAnnotated;
use nym_contracts_common::types::Percent;

static SERVICE_PROVIDER_WELLKNOWN_URL: &str =
    "http://178.79.159.92/service-providers.json";
static HARBOUR_MASTER_URL: &str = "http://178.79.159.92/harbourmaster.json";
static GATEWAYS_DETAILED_URL: &str =
    "https://qwerty-validator-api.qa.nymte.ch/api/v1/status/gateways/detailed";

#[tauri::command]
pub async fn get_services() -> Result<Vec<DirectoryServiceProvider>> {
    log::trace!("Fetching services");
    let services_res = reqwest::get(SERVICE_PROVIDER_WELLKNOWN_URL)
        .await?
        .json::<Vec<DirectoryService>>()
        .await?;
    log::trace!("Received: {:#?}", services_res);

    log::trace!("Fetching gateways");
    let gateway_res = reqwest::get(GATEWAYS_DETAILED_URL)
        .await?
        .json::<Vec<GatewayBondAnnotated>>()
        .await?;
    log::trace!("Received: {:#?}", gateway_res);

    // TODO: get paged
    log::trace!("Fetching active services");
    let active_services = reqwest::get(HARBOUR_MASTER_URL)
        .await?
        .json::<PagedResult<HarbourMasterService>>()
        .await?;
    log::trace!("Active: {:#?}", active_services);

    let mut filtered: Vec<DirectoryService> = vec![];

    for service in &services_res {
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

    let perf_threshold = Percent::from_percentage_value(90).unwrap();

    // Use only services that are active AND have a performance of >= 90%
    let services_with_good_performance: Vec<DirectoryServiceProvider> = filtered
        .iter_mut()
        .fold(vec![], |mut acc, sp| {
            acc.append(&mut sp.items);
            acc
        })
        .into_iter()
        .filter(|sp| {
            gateway_res.iter().any(|gateway| {
                gateway.gateway_bond.gateway.identity_key == sp.gateway
                    && gateway.performance >= perf_threshold
            })
        })
        .collect();

    Ok(services_with_good_performance)
}

#[tauri::command]
pub async fn get_gateways_detailed() -> Result<Vec<GatewayBondAnnotated>> {
    let res = reqwest::get(GATEWAYS_DETAILED_URL)
        .await?
        .json::<Vec<GatewayBondAnnotated>>()
        .await?;
    Ok(res)
}
