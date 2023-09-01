use crate::{
    config::PrivacyLevel,
    error::{BackendError, Result},
    models::{
        DirectoryService, DirectoryServiceProvider, Gateway, HarbourMasterService, PagedResult,
    },
    state::State,
};
use itertools::Itertools;
use nym_api_requests::models::GatewayBondAnnotated;
use nym_bin_common::version_checker::is_minor_version_compatible;
use nym_config::defaults::var_names::{NETWORK_NAME, NYM_API};
use nym_contracts_common::types::Percent;
use nym_topology::gateway;
use nym_validator_client::nym_api::Client as ApiClient;
use std::str::FromStr;
use std::sync::Arc;
use tap::TapFallible;
use tokio::sync::RwLock;
use url::Url;

pub(crate) static WELLKNOWN_DIR: &str = "https://nymtech.net/.wellknown";

static SERVICE_PROVIDER_URL_PATH: &str = "connect/service-providers.json";

// List of network-requesters running with medium toggle enabled, for testing
static SERVICE_PROVIDER_MEDIUM_URL_PATH: &str = "connect/service-providers-medium.json";

// Harbour master is used to periodically keep track of which network-requesters are online
static HARBOUR_MASTER_URL: &str = "https://harbourmaster.nymtech.net/v1/services/?size=100";

// We only consider network requesters with a routing score above this threshold
const SERVICE_ROUTING_SCORE_THRESHOLD: f32 = 0.9;

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

async fn fetch_all_gateways() -> Result<Vec<GatewayBondAnnotated>> {
    let api_client = ApiClient::new(Url::from_str(&std::env::var(NYM_API)?)?);
    let gateways = api_client.get_gateways_detailed().await?;
    if gateways.is_empty() {
        Err(BackendError::NoGatewaysFoundInDirectory)
    } else {
        Ok(gateways)
    }
}

async fn fetch_only_compatible_gateways() -> Result<Vec<GatewayBondAnnotated>> {
    let gateways = fetch_all_gateways().await?;
    let our_version = env!("CARGO_PKG_VERSION");
    log::debug!(
        "Our version that we use to filter compatible gateways: {}",
        our_version
    );
    let gateways: Vec<_> = gateways
        .into_iter()
        .filter(|g| is_minor_version_compatible(&g.gateway_bond.gateway.version, our_version))
        .collect();
    if gateways.is_empty() {
        Err(BackendError::NoVersionCompatibleGatewaysFound(
            our_version.to_string(),
        ))
    } else {
        Ok(gateways)
    }
}

fn filter_out_low_performance_gateways(
    gateways: Vec<GatewayBondAnnotated>,
) -> Result<Vec<GatewayBondAnnotated>> {
    let mut filtered_gateways: Vec<_> = gateways
        .iter()
        .filter(|g| {
            g.node_performance.most_recent
                > Percent::from_percentage_value(GATEWAY_PERFORMANCE_SCORE_THRESHOLD).unwrap()
        })
        .cloned()
        .collect();

    // Sometimes the most_recent is zero for all gateways (bug in nym-api?)
    if filtered_gateways.is_empty() {
        log::warn!(
            "No gateways with recent performance score above threshold found! Using \
            last hour performance scores instead as fallback"
        );
        filtered_gateways = gateways
            .into_iter()
            .filter(|g| {
                g.node_performance.last_hour
                    > Percent::from_percentage_value(GATEWAY_PERFORMANCE_SCORE_THRESHOLD).unwrap()
            })
            .collect();
    }

    if filtered_gateways.is_empty() {
        log::error!("No gateways found! (with high enough performance score)");
        Err(BackendError::NoGatewayWithAcceptablePerformanceFound)
    } else {
        Ok(filtered_gateways)
    }
}

async fn select_gateway_by_latency(gateways: Vec<GatewayBondAnnotated>) -> Result<gateway::Node> {
    let gateways_as_nodes: Vec<gateway::Node> = gateways
        .into_iter()
        .filter_map(|g| g.gateway_bond.try_into().ok())
        .collect();

    let mut rng = rand_07::rngs::OsRng;
    let selected_gateway =
        nym_client_core::init::helpers::choose_gateway_by_latency(&mut rng, &gateways_as_nodes)
            .await?;
    Ok(selected_gateway)
}

// Get all gateways satisfying the performance threshold.
#[tauri::command]
pub async fn get_gateways() -> Result<Vec<Gateway>> {
    log::trace!("Fetching gateways");
    let all_gateways = fetch_only_compatible_gateways().await?;
    log::debug!("Received {} gateways", all_gateways.len());
    log::trace!("Received: {:#?}", all_gateways);

    let gateways_filtered = filter_out_low_performance_gateways(all_gateways.clone())?
        .into_iter()
        .map(|g| Gateway {
            identity: g.identity().clone(),
        })
        .collect_vec();
    log::debug!(
        "After filtering out low-performance gateways: {}",
        gateways_filtered.len()
    );
    log::trace!(
        "Filtered: [\n\t{}\n]",
        gateways_filtered.iter().join(",\n\t")
    );

    Ok(gateways_filtered)
}

// Lookup and select a single gateway with low latency.
#[tauri::command]
pub async fn get_gateway_with_low_latency() -> Result<Gateway> {
    log::trace!("Fetching gateways with low latency");
    let all_gateways = fetch_only_compatible_gateways().await?;
    log::debug!("Received {} gateways", all_gateways.len());
    log::trace!("Received: {:#?}", all_gateways);

    let gateways_filtered = filter_out_low_performance_gateways(all_gateways)?;
    let selected_gateway = select_gateway_by_latency(gateways_filtered).await?;
    log::debug!("Selected gateway: {}", selected_gateway);
    Ok(Gateway {
        identity: selected_gateway.identity().to_base58_string(),
    })
}

// From a given list of gateways, select the one with low latency.
#[tauri::command]
pub async fn select_gateway_with_low_latency_from_list(gateways: Vec<Gateway>) -> Result<Gateway> {
    log::debug!("Selecting a gateway with low latency");
    let gateways = gateways.into_iter().map(|g| g.identity).collect_vec();
    let all_gateways = fetch_only_compatible_gateways().await?;
    let gateways_union_set: Vec<GatewayBondAnnotated> = all_gateways
        .into_iter()
        .filter(|g| gateways.contains(g.identity()))
        .collect();
    let gateways_filtered = filter_out_low_performance_gateways(gateways_union_set)?;
    let selected_gateway = select_gateway_by_latency(gateways_filtered).await?;
    log::debug!("Selected gateway: {}", selected_gateway);
    Ok(Gateway {
        identity: selected_gateway.identity().to_base58_string(),
    })
}
