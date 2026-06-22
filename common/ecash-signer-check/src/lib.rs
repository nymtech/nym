// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client_check::check_client;
use futures::stream;
use futures::stream::StreamExt;
use nym_ecash_signer_check_types::status::{SignerResult, Status};
use nym_network_defaults::NymNetworkDetails;
use nym_validator_client::QueryHttpRpcNyxdClient;
use nym_validator_client::ecash::models::EcashSignerStatusResponse;
use nym_validator_client::models::{
    ChainBlocksStatusResponse, ChainStatusResponse, SignerInformationResponse,
};
use nym_validator_client::nyxd::contract_traits::dkg_query_client::{
    ContractVKShare, DealerDetails, Epoch,
};
use nym_validator_client::nyxd::contract_traits::{DkgQueryClient, PagedDkgQueryClient};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

pub use error::SignerCheckError;

mod client_check;
pub mod error;

pub type TypedSignerResult = SignerResult<
    SignerInformationResponse,
    EcashSignerStatusResponse,
    ChainStatusResponse,
    ChainBlocksStatusResponse,
>;
pub type LocalChainStatus = Status<ChainStatusResponse, ChainBlocksStatusResponse>;
pub type SigningStatus = Status<SignerInformationResponse, EcashSignerStatusResponse>;

#[derive(Serialize, Deserialize)]
pub struct SignersTestResult {
    pub threshold: Option<u64>,
    pub results: Vec<TypedSignerResult>,
}

pub async fn check_signers(
    rpc_endpoint: Url,
    // details such as denoms, prefixes, etc.
    network_details: NymNetworkDetails,
) -> Result<SignersTestResult, SignerCheckError> {
    // 1. create nyx client instance
    let client = QueryHttpRpcNyxdClient::connect_with_network_details(
        rpc_endpoint.as_str(),
        network_details,
    )
    .map_err(SignerCheckError::invalid_nyxd_connection_details)?;

    check_signers_with_client(&client).await
}

pub struct DkgDetails {
    pub dkg_epoch: Epoch,
    pub threshold: Option<u64>,
    pub network_dealers: Vec<DealerDetails>,
    pub submitted_shared: HashMap<u64, ContractVKShare>,
}

pub async fn check_signers_with_client<C>(client: &C) -> Result<SignersTestResult, SignerCheckError>
where
    C: DkgQueryClient,
{
    let dkg_details = dkg_details_with_client(client).await?;
    check_known_dealers(dkg_details, None).await
}

pub async fn dkg_details_with_client<C>(client: &C) -> Result<DkgDetails, SignerCheckError>
where
    C: DkgQueryClient,
{
    // 2. retrieve current dkg epoch
    let dkg_epoch = client
        .get_current_epoch()
        .await
        .map_err(SignerCheckError::dkg_contract_query_failure)?;

    // 3. retrieve the dkg threshold as reference point
    let threshold = client
        .get_epoch_threshold(dkg_epoch.epoch_id)
        .await
        .map_err(SignerCheckError::dkg_contract_query_failure)?;

    // 4. retrieve information on current DKG dealers (i.e. eligible signers)
    let dealers = client
        .get_all_current_dealers()
        .await
        .map_err(SignerCheckError::dkg_contract_query_failure)?;

    // 5. retrieve their published keys (if available)
    let shares: HashMap<_, _> = client
        .get_all_verification_key_shares(dkg_epoch.epoch_id)
        .await
        .map_err(SignerCheckError::dkg_contract_query_failure)?
        .into_iter()
        .map(|share| (share.node_index, share))
        .collect();

    Ok(DkgDetails {
        dkg_epoch,
        threshold,
        network_dealers: dealers,
        submitted_shared: shares,
    })
}

pub async fn check_known_dealers(
    dkg_details: DkgDetails,
    concurrency: impl Into<Option<usize>>,
) -> Result<SignersTestResult, SignerCheckError> {
    // 6. for each dealer attempt to perform the checks
    let epoch_id = dkg_details.dkg_epoch.epoch_id;
    let submitted = dkg_details.submitted_shared;
    let dealers = dkg_details.network_dealers.len();

    let tasks = dkg_details.network_dealers.into_iter().map(move |d| {
        let share = submitted.get(&d.assigned_index).cloned();
        check_client(d, epoch_id, share)
    });

    let limit = concurrency.into().filter(|&n| n > 0).unwrap_or(dealers);

    let results = stream::iter(tasks).buffer_unordered(limit).collect().await;

    Ok(SignersTestResult {
        threshold: dkg_details.threshold,
        results,
    })
}
