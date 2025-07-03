// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::chain_status::LocalChainStatus;
use crate::client_check::check_client;
use crate::signing_status::SigningStatus;
use futures::stream::{FuturesUnordered, StreamExt};
use nym_network_defaults::NymNetworkDetails;
use nym_validator_client::nyxd::contract_traits::dkg_query_client::ContractVKShare;
use nym_validator_client::nyxd::contract_traits::{DkgQueryClient, PagedDkgQueryClient};
use nym_validator_client::QueryHttpRpcNyxdClient;
use url::Url;

pub use error::SignerCheckError;

mod chain_status;
mod client_check;
pub mod error;
pub(crate) mod signing_status;

#[derive(Debug)]
pub struct SignerInformation {
    pub announce_address: String,
    pub owner_address: String,
    pub node_index: u64,
}

impl From<&ContractVKShare> for SignerInformation {
    fn from(share: &ContractVKShare) -> Self {
        SignerInformation {
            announce_address: share.announce_address.clone(),
            owner_address: share.owner.to_string(),
            node_index: share.node_index,
        }
    }
}

#[derive(Debug)]
pub struct SignerResult {
    pub information: SignerInformation,
    pub status: SignerStatus,
}

#[derive(Debug)]
pub enum SignerStatus {
    Unreachable,
    ProvidedInvalidDetails,
    Tested { result: SignerTestResult },
}

impl SignerStatus {
    pub fn with_signer_information(self, information: SignerInformation) -> SignerResult {
        SignerResult {
            status: self,
            information,
        }
    }
}

#[derive(Debug)]
pub struct SignerTestResult {
    pub reported_version: String,
    pub signing_status: SigningStatus,
    pub local_chain_status: LocalChainStatus,
}

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

    // 2. retrieve current dkg epoch
    let dkg_epoch = client
        .get_current_epoch()
        .await
        .map_err(SignerCheckError::dkg_contract_query_failure)?;

    // 3. retrieve list of all current dealers (signers)
    let shares = client
        .get_all_verification_key_shares(dkg_epoch.epoch_id)
        .await
        .map_err(SignerCheckError::dkg_contract_query_failure)?;

    // 4. for each share, attempt to check corresponding signer
    let results = shares
        .into_iter()
        .map(check_client)
        .collect::<FuturesUnordered<_>>()
        .collect::<Vec<_>>()
        .await;

    Ok(results)
}
