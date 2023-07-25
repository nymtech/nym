use crate::{
    config::PrivacyLevel,
    error::Result,
    models::{
        DirectoryService, DirectoryServiceProvider, Gateway, HarbourMasterService, PagedResult,
    },
    state::State,
};
use itertools::Itertools;
use nym_api_requests::models::GatewayBondAnnotated;
use nym_contracts_common::types::Percent;
use std::sync::Arc;
use tokio::sync::RwLock;

static SERVICE_PROVIDER_WELLKNOWN_URL: &str =
    "https://nymtech.net/.wellknown/connect/service-providers.json";

// List of network-requesters running with medium toggle enabled, for testing
static SERVICE_PROVIDER_WELLKNOWN_URL_MEDIUM: &str =
    "https://nymtech.net/.wellknown/connect/service-providers-medium.json";

// Harbour master is used to periodically keep track of which network-requesters are online
static HARBOUR_MASTER_URL: &str = "https://harbourmaster.nymtech.net/v1/services/?size=100";

// We only consider network requesters with a routing score above this threshold
const SERVICE_ROUTING_SCORE_THRESHOLD: f32 = 0.9;

static GATEWAYS_DETAILED_URL: &str =
    "https://validator.nymtech.net/api/v1/status/gateways/detailed";

// Only use gateways with a performnnce score above this
const GATEWAY_PERFORMANCE_SCORE_THRESHOLD: u64 = 90;

#[tauri::command]
pub async fn get_services(
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Vec<DirectoryServiceProvider>> {
    let guard = state.read().await;
    let privacy_level = guard.get_user_data().privacy_level.unwrap_or_default();

    log::trace!("Fetching services");
    let all_services_with_category = fetch_services(&privacy_level).await?;
    log::trace!("Received: {:#?}", all_services_with_category);

    // Flatten all services into a single vector (get rid of categories)
    // We currently don't care about categories, but we might in the future...
    let all_services = all_services_with_category
        .into_iter()
        .flat_map(|sp| sp.items)
        .collect_vec();

    // Early return if we're running with medium toggle enabled
    if let PrivacyLevel::Medium = privacy_level {
        return Ok(all_services);
    }

    // TODO: get paged
    log::trace!("Fetching active services");
    let active_services = fetch_active_services().await?;
    log::trace!("Active: {:#?}", active_services);

    if active_services.items.is_empty() {
        log::warn!("No active services found! Using all services instead as fallback");
        return Ok(all_services);
    }

    log::trace!("Filter out inactive");
    let filtered_services = filter_out_inactive_services(&all_services, active_services);
    log::trace!("After filtering: {:#?}", filtered_services);

    if filtered_services.is_empty() {
        log::warn!(
            "After filtering, no active services found! Using all services instead as fallback"
        );
        return Ok(all_services);
    }

    Ok(filtered_services)
}

async fn fetch_services(privacy_level: &PrivacyLevel) -> Result<Vec<DirectoryService>> {
    let services_url = match privacy_level {
        PrivacyLevel::Medium => SERVICE_PROVIDER_WELLKNOWN_URL_MEDIUM,
        _ => SERVICE_PROVIDER_WELLKNOWN_URL,
    };

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

fn filter_out_inactive_services(
    all_services: &[DirectoryServiceProvider],
    active_services: PagedResult<HarbourMasterService>,
) -> Vec<DirectoryServiceProvider> {
    all_services
        .iter()
        .filter(|sp| {
            active_services.items.iter().any(|active| {
                active.service_provider_client_id == sp.address
                    && active.routing_score > SERVICE_ROUTING_SCORE_THRESHOLD
            })
        })
        .cloned()
        .collect()
}

async fn fetch_gateways() -> Result<Vec<GatewayBondAnnotated>> {
    Ok(reqwest::get(GATEWAYS_DETAILED_URL)
        .await?
        .json::<Vec<GatewayBondAnnotated>>()
        .await?)
}

#[tauri::command]
pub async fn get_gateways() -> Result<Vec<Gateway>> {
    log::trace!("Fetching gateways");
    let all_gateways = fetch_gateways().await?;
    log::trace!("Received: {:#?}", all_gateways);

    let filtered_gateways = all_gateways
        .iter()
        .filter(|g| {
            g.node_performance.most_recent
                > Percent::from_percentage_value(GATEWAY_PERFORMANCE_SCORE_THRESHOLD).unwrap()
        })
        .map(|g| Gateway {
            identity: g.identity().clone(),
        })
        .collect_vec();
    log::trace!("Filtered: {:#?}", filtered_gateways);

    if filtered_gateways.is_empty() {
        log::warn!("No gateways with high enough performance score found! Using all gateways instead as fallback");
        return Ok(all_gateways
            .iter()
            .map(|g| Gateway {
                identity: g.identity().clone(),
            })
            .collect_vec());
    }

    Ok(filtered_gateways)
}
