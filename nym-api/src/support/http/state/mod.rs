// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::state::EcashState;
use crate::network::models::NetworkDetailsV2;
use crate::node_status_api::handlers::unstable;
use crate::node_status_api::models::AxumErrorResponse;
use crate::signers_cache::cache::SignersCacheData;
use crate::status::ApiStatusState;
use crate::support::attested_snapshots::data::RetainedHeight;
use crate::support::attested_snapshots::state::ContractSnapshots;
use crate::support::caching::cache::SharedCache;
use crate::support::http::state::chain_status::ChainStatusCache;
use crate::support::http::state::contract_details::ContractDetailsCache;
use crate::support::nyxd::Client;
use crate::support::storage;
use crate::unstable_routes::v1::account::cache::AddressInfoCache;
use crate::unstable_routes::v1::account::models::NyxAccountDetails;
use axum::extract::FromRef;
use nym_crypto::asymmetric::ed25519;
use std::sync::Arc;
use tokio::sync::RwLockReadGuard;

pub(crate) mod chain_status;
pub(crate) mod contract_details;
pub(crate) mod helpers;
#[cfg(test)]
pub(crate) mod test_helpers;

#[derive(Clone)]
pub(crate) struct AppState {
    /// Identity of this nym-api instance.
    pub(crate) identity_keypair: Arc<ed25519::KeyPair>,

    // ideally this would have been made generic to make tests easier,
    // however, it'd be a way bigger change (I tried)
    /// Instance of a client used for interacting with the nyx chain.
    pub(crate) nyxd_client: Client,

    /// Holds information about the latest chain block it has queried.
    /// Note, it is not updated on every request. It follows the embedded ttl.
    pub(crate) chain_status_cache: ChainStatusCache,

    /// Holds cached state of the statuses of all [ecash] signers on the network -
    /// their perceived chain statuses and signing capabilities.
    pub(crate) ecash_signers_cache: SharedCache<SignersCacheData>,

    /// Holds mapping between a nyx address and tokens/delegations it holds
    pub(crate) address_info_cache: AddressInfoCache,

    /// Holds reference to the persistent storage of this nym-api.
    pub(crate) storage: storage::NymApiStorage,

    /// Information about the current network this nym-api is connected to, e.g. contract addresses,
    /// endpoints, denominations.
    pub(crate) network_details: NetworkDetailsV2,

    /// A simple in-memory cache of node information mapping their database id to their node-ids
    /// and public keys. Useful (I guess?) for returning information about test routes.
    // TODO: do we need it?
    pub(crate) node_info_cache: unstable::NodeInfoCache,

    /// Cache containing data (build info, versions, etc.) on all nym smart contracts on the network
    pub(crate) contract_info_cache: ContractDetailsCache,

    /// Information about this nym-api, i.e. its public key, startup time, etc.
    pub(crate) api_status: ApiStatusState,

    // todo: refactor it into inner: Arc<EcashStateInner>
    /// Cache holding data required by the ecash credentials - static signatures, merkle trees, etc.
    pub(crate) ecash_state: Arc<EcashState>,

    /// The retained snapshot heights every node, epoch and family route reads from.
    pub(crate) contract_snapshots: ContractSnapshots,
}

impl FromRef<AppState> for ApiStatusState {
    fn from_ref(app_state: &AppState) -> Self {
        app_state.api_status.clone()
    }
}

impl FromRef<AppState> for Arc<EcashState> {
    fn from_ref(app_state: &AppState) -> Self {
        app_state.ecash_state.clone()
    }
}

impl FromRef<AppState> for SharedCache<SignersCacheData> {
    fn from_ref(app_state: &AppState) -> Self {
        app_state.ecash_signers_cache.clone()
    }
}

impl AppState {
    pub(crate) fn private_signing_key(&self) -> &ed25519::PrivateKey {
        // even though we have to go through ecash state, the key is always available
        // (moving it would involve some refactoring that's not worth it now)
        self.ecash_state.local.identity_keypair.private_key()
    }

    pub(crate) fn network_details(&self) -> &NetworkDetailsV2 {
        &self.network_details
    }

    pub(crate) fn storage(&self) -> &storage::NymApiStorage {
        &self.storage
    }

    pub(crate) fn node_info_cache(&self) -> &unstable::NodeInfoCache {
        &self.node_info_cache
    }
}

// handler helpers to easily get data or return error response
impl AppState {
    /// The latest settled snapshot; read it once per response so that it never mixes two heights.
    pub(crate) async fn latest_snapshot(
        &self,
    ) -> Result<RwLockReadGuard<'_, RetainedHeight>, AxumErrorResponse> {
        self.contract_snapshots
            .latest_entry()
            .await
            .ok_or_else(|| AxumErrorResponse::service_unavailable("no snapshot has settled yet"))
    }

    pub(crate) async fn get_address_info(
        self,
        account_id: nym_validator_client::nyxd::AccountId,
    ) -> Result<NyxAccountDetails, AxumErrorResponse> {
        let address = account_id.to_string();
        match self.address_info_cache.get(&address).await {
            Some(guard) => {
                tracing::trace!("Fetching from cache...");
                let read_lock = guard.read().await;
                Ok(read_lock.clone())
            }
            None => {
                tracing::trace!("No cache for {}, refreshing data...", &address);

                let address_info = self
                    .address_info_cache
                    .collect_balances(
                        self.nyxd_client.clone(),
                        self.contract_snapshots.clone(),
                        self.network_details()
                            .network
                            .chain_details
                            .mix_denom
                            .base
                            .to_owned(),
                        &address,
                        account_id,
                    )
                    .await?;

                self.address_info_cache
                    .upsert_address_info(&address, address_info.clone())
                    .await;

                Ok(address_info)
            }
        }
    }
}
