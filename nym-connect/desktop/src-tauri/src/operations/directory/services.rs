use crate::{
    config::PrivacyLevel,
    error::{BackendError, Result},
    models::{DirectoryService, DirectoryServiceProvider, HarbourMasterService, PagedResult},
    state::State,
};
use itertools::Itertools;
use nym_config::defaults::var_names::NETWORK_NAME;
use std::sync::Arc;
use tap::TapFallible;
use tokio::sync::RwLock;

use super::WELLKNOWN_DIR;

static SERVICE_PROVIDER_URL_PATH: &str = "connect/service-providers.json";

// List of network-requesters running with medium toggle enabled, for testing
static SERVICE_PROVIDER_MEDIUM_URL_PATH: &str = "connect/service-providers-medium.json";

// Harbour master is used to periodically keep track of which network-requesters are online
static HARBOUR_MASTER_URL: &str = "https://harbourmaster.nymtech.net/v1/services/?size=100";

// We only consider network requesters with a routing score above this threshold
const SERVICE_ROUTING_SCORE_THRESHOLD: f32 = 0.9;

// Fetch all the services from the directory (currently hardcoded, but in the future it could be a
// contract).
async fn fetch_services(privacy_level: &PrivacyLevel) -> Result<Vec<DirectoryService>> {
    let services_url = match privacy_level {
        PrivacyLevel::Medium => SERVICE_PROVIDER_MEDIUM_URL_PATH,
        _ => SERVICE_PROVIDER_URL_PATH,
    };

    let network_name = std::env::var(NETWORK_NAME)?;
    let url = format!("{}/{}/{}", WELLKNOWN_DIR, network_name, services_url);
    let services_res = reqwest::get(url)
        .await?
        .json::<Vec<DirectoryService>>()
        .await?;
    if services_res.is_empty() {
        log::error!("No services found in directory!");
        Err(BackendError::NoServicesFoundInDirectory)
    } else {
        Ok(services_res)
    }
}

// Fetch all the active services from harbour master
async fn query_active_services() -> Result<PagedResult<HarbourMasterService>> {
    let active_services = reqwest::get(HARBOUR_MASTER_URL)
        .await?
        .json::<PagedResult<HarbourMasterService>>()
        .await?;
    if active_services.items.is_empty() {
        log::error!("No active services found!");
        Err(BackendError::NoActiveServicesFound)
    } else {
        Ok(active_services)
    }
}

fn filter_out_inactive_services(
    all_services: &[DirectoryServiceProvider],
    active_services: PagedResult<HarbourMasterService>,
) -> Result<Vec<DirectoryServiceProvider>> {
    let services: Vec<_> = all_services
        .iter()
        .filter(|sp| {
            active_services.items.iter().any(|active| {
                active.service_provider_client_id == sp.address
                    && active.routing_score > SERVICE_ROUTING_SCORE_THRESHOLD
            })
        })
        .cloned()
        .collect();
    if services.is_empty() {
        Err(BackendError::NoServicesFoundInDirectory)
    } else {
        Ok(services)
    }
}

#[tauri::command]
pub async fn get_services(
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Vec<DirectoryServiceProvider>> {
    let guard = state.read().await;
    let privacy_level = guard.get_user_data().privacy_level.unwrap_or_default();

    log::trace!("Fetching services");
    let all_services_with_category = fetch_services(&privacy_level).await?;

    // Flatten all services into a single vector (get rid of categories)
    // We currently don't care about categories, but we might in the future...
    let all_services = all_services_with_category
        .into_iter()
        .flat_map(|sp| sp.items)
        .collect_vec();
    log::debug!("Received {} services", all_services.len());
    log::trace!("Received: {:#?}", all_services);

    // Early return if we're running with medium toggle enabled
    if let PrivacyLevel::Medium = privacy_level {
        return Ok(all_services);
    }

    // If there is a failure getting the active services, just return all of them
    // TODO: get paged
    log::trace!("Fetching active services");
    let Ok(active_services) = query_active_services().await else {
        log::warn!("Using all services instead as fallback");
        return Ok(all_services);
    };
    log::debug!(
        "Received {} active services from harbourmaster",
        active_services.items.len()
    );
    log::trace!("Active: {:#?}", active_services);

    // From the list of all services, filter out the ones that are inactive
    log::trace!("Filter out inactive and low performance");
    let filtered_services = filter_out_inactive_services(&all_services, active_services);

    // If there is a failure filtering out inactive services, just return all of them
    filtered_services
        .tap_ok(|services| {
            log::debug!(
                "After filtering out inactive and low performance: {}",
                services.len()
            );
            log::trace!("After filtering: {:#?}", services);
        })
        .or_else(|_| {
            // If for some reason harbourmaster is done, we want things to still sort of work.
            log::warn!(
                "After filtering, no active services found! Using all services instead as fallback"
            );
            Ok(all_services)
        })
}
