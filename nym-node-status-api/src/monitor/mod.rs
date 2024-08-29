use crate::db::{DbPool, Storage};
use crate::error::NodeStatusApiResult;
use nym_explorer_client::ExplorerClient;
use nym_network_defaults::NymNetworkDetails;
use nym_validator_client::client::NymApiClientExt;
use nym_validator_client::NymApiClient;
use tokio::task::JoinHandle;
use tokio::time::Duration;

const REFRESH_DELAY: Duration = Duration::from_secs(60 * 5);
const FAILURE_RETRY_DELAY: Duration = Duration::from_secs(60);

pub(crate) fn spawn_in_background(storage: Storage) -> JoinHandle<()> {
    tokio::spawn(async move {
        let db_pool = storage.pool().await;
        let network_defaults = nym_network_defaults::NymNetworkDetails::new_from_env();

        loop {
            tracing::info!("Refreshing node info...");

            if let Err(e) = run(db_pool, &network_defaults).await {
                tracing::error!(
                    "Monitor run failed: {e}, retrying in {}s...",
                    FAILURE_RETRY_DELAY.as_secs()
                );
                tokio::time::sleep(FAILURE_RETRY_DELAY).await;
            } else {
                tokio::time::sleep(REFRESH_DELAY).await;
            }
        }
    })
}

async fn run(db_pool: &DbPool, network_details: &NymNetworkDetails) -> NodeStatusApiResult<()> {
    let default_api_url = network_details
        .endpoints
        .first()
        .expect("rust sdk mainnet default incorrectly configured")
        .api_url
        .clone()
        .expect("rust sdk mainnet default missing api_url")
        .parse()
        .expect("rust sdk mainnet default api_url not parseable");
    let default_explorer_url = network_details.explorer_api.clone().map(|url| {
        url.parse()
            .expect("rust sdk mainnet default explorer url not parseable")
    });

    let default_explorer_url =
        default_explorer_url.expect("explorer url missing in network config");
    let explorer_client = ExplorerClient::new(default_explorer_url)?;
    let explorer_gateways = explorer_client.get_gateways().await?;
    tracing::debug!("explorer_gateways:\n{}", explorer_gateways.len());

    let api_client = NymApiClient::new(default_api_url);
    let gateways = api_client.get_cached_described_gateways().await?;
    tracing::debug!("Gateways:\n{}", gateways.len());
    tracing::debug!("example gateway:\n{:#?}", gateways.first());
    let mixnodes = api_client.get_cached_mixnodes().await?;
    tracing::debug!("Mixnodes:\n{}", mixnodes.len());
    // tracing::debug!("example mixnode:\n{:#?}", mixnodes.first());

    let mixnodes_described = api_client.nym_api.get_mixnodes_described().await?;
    tracing::debug!("Mixnodes described:\n{}", mixnodes_described.len());
    // tracing::debug!("Mixnodes described example:\n{:#?}", mixnodes_described.first());
    let gateways_blacklisted = api_client.nym_api.get_gateways_blacklisted().await?;
    tracing::debug!("gateways_blacklisted:\n{}", gateways_blacklisted.len());
    let mixnodes_blacklisted = api_client.nym_api.get_mixnodes_blacklisted().await?;
    tracing::debug!("mixnodes_blacklisted:\n{}", mixnodes_blacklisted.len());
    // TODO left over here

    Ok(())
}
