// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client_check::check_client;
use futures::stream::{FuturesUnordered, StreamExt};
use nym_network_defaults::NymNetworkDetails;
use nym_validator_client::nyxd::contract_traits::{DkgQueryClient, PagedDkgQueryClient};
use nym_validator_client::QueryHttpRpcNyxdClient;
use std::collections::HashMap;
use url::Url;

pub use crate::status::SignerResult;
pub use error::SignerCheckError;

pub mod chain_status;
mod client_check;
pub mod dealer_information;
pub mod error;
pub mod signing_status;
pub mod status;

pub async fn check_signers(
    rpc_endpoint: Url,
    // details such as denoms, prefixes, etc.
    network_details: NymNetworkDetails,
) -> Result<Vec<SignerResult>, SignerCheckError> {
    // 1. create nyx client instance
    let client = QueryHttpRpcNyxdClient::connect_with_network_details(
        rpc_endpoint.as_str(),
        network_details,
    )
    .map_err(SignerCheckError::invalid_nyxd_connection_details)?;

    check_signers_with_client(&client).await
}

pub async fn check_signers_with_client<C>(client: &C) -> Result<Vec<SignerResult>, SignerCheckError>
where
    C: DkgQueryClient + Sync,
{
    // 2. retrieve current dkg epoch
    let dkg_epoch = client
        .get_current_epoch()
        .await
        .map_err(SignerCheckError::dkg_contract_query_failure)?;

    // 3. retrieve information on current DKG dealers (i.e. eligible signers)
    let dealers = client
        .get_all_current_dealers()
        .await
        .map_err(SignerCheckError::dkg_contract_query_failure)?;

    // 4. retrieve their published keys (if available)
    let shares: HashMap<_, _> = client
        .get_all_verification_key_shares(dkg_epoch.epoch_id)
        .await
        .map_err(SignerCheckError::dkg_contract_query_failure)?
        .into_iter()
        .map(|share| (share.node_index, share))
        .collect();

    // 5. for each dealer attempt to perform the checks
    let results = dealers
        .into_iter()
        .map(|d| {
            let share = shares.get(&d.assigned_index);
            check_client(d, dkg_epoch.epoch_id, share)
        })
        .collect::<FuturesUnordered<_>>()
        .collect::<Vec<_>>()
        .await;

    Ok(results)
}
