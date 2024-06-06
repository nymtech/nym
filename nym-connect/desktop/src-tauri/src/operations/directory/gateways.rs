use crate::{
    error::{BackendError, Result},
    models::Gateway,
};
use itertools::Itertools;
use nym_api_requests::models::GatewayBondAnnotated;
use nym_bin_common::version_checker::is_minor_version_compatible;
use nym_config::defaults::var_names::NYM_API;
use nym_contracts_common::types::Percent;
use nym_topology::gateway;
use nym_validator_client::client::NymApiClientExt;
use nym_validator_client::nym_api::Client as ApiClient;
use std::str::FromStr;
use url::Url;

// Only use gateways with a performnnce score above this
const GATEWAY_PERFORMANCE_SCORE_THRESHOLD: u64 = 90;

async fn fetch_all_gateways() -> Result<Vec<GatewayBondAnnotated>> {
    let api_client = ApiClient::new(Url::from_str(&std::env::var(NYM_API)?)?, None);
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
        Err(BackendError::NoGatewaysWithAcceptablePerformanceFound)
    } else {
        Ok(filtered_gateways)
    }
}

async fn select_gateway_by_latency(gateways: Vec<GatewayBondAnnotated>) -> Result<gateway::Node> {
    let gateways_as_nodes: Vec<gateway::Node> = gateways
        .into_iter()
        .filter_map(|g| g.gateway_bond.try_into().ok())
        .collect();

    let mut rng = rand::rngs::OsRng;
    let selected_gateway = nym_client_core::init::helpers::choose_gateway_by_latency(
        &mut rng,
        &gateways_as_nodes,
        false,
    )
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

    let gateways_filtered = filter_out_low_performance_gateways(all_gateways)?
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
