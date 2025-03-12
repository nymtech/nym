// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::models::SignerStatus;
use anyhow::bail;
use nym_network_defaults::NymNetworkDetails;
use nym_validator_client::nyxd::contract_traits::dkg_query_client::DealerDetails;
use nym_validator_client::nyxd::contract_traits::PagedDkgQueryClient;
use nym_validator_client::nyxd::Config;
use nym_validator_client::QueryHttpRpcNyxdClient;
use tracing::info;

async fn get_query_client() -> anyhow::Result<QueryHttpRpcNyxdClient> {
    let network = NymNetworkDetails::new_from_env();

    let Some(endpoint_info) = network.endpoints.first() else {
        bail!("no known rpc endpoints available")
    };

    let config = Config::try_from_nym_network_details(&network)?;
    Ok(QueryHttpRpcNyxdClient::connect(
        config,
        endpoint_info.nyxd_url.as_str(),
    )?)
}

pub(crate) async fn get_known_dealers() -> anyhow::Result<Vec<DealerDetails>> {
    let client = get_query_client().await?;
    Ok(client.get_all_current_dealers().await?)
}

pub(crate) async fn get_signer_status(raw_api_endpoint: &str) -> SignerStatus {
    info!("attempting to get signer status of {raw_api_endpoint}...");
    let mut status = SignerStatus::new(raw_api_endpoint.to_string());

    status.try_update_api_version().await;
    status.try_update_rpc_status().await;
    status
}
