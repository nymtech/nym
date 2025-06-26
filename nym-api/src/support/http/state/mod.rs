// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::state::EcashState;
use crate::mixnet_contract_cache::cache::MixnetContractCache;
use crate::network::models::NetworkDetails;
use crate::node_describe_cache::cache::DescribedNodes;
use crate::node_status_api::handlers::unstable;
use crate::node_status_api::models::AxumErrorResponse;
use crate::node_status_api::NodeStatusCache;
use crate::status::ApiStatusState;
use crate::support::caching::cache::SharedCache;
use crate::support::caching::Cache;
use crate::support::http::state::chain_status::ChainStatusCache;
use crate::support::http::state::contract_details::ContractDetailsCache;
use crate::support::http::state::force_refresh::ForcedRefresh;
use crate::support::nyxd::Client;
use crate::support::storage;
use crate::unstable_routes::v1::account::cache::AddressInfoCache;
use crate::unstable_routes::v1::account::models::NyxAccountDetails;
use axum::extract::FromRef;
use nym_api_requests::models::{GatewayBondAnnotated, MixNodeBondAnnotated, NodeAnnotation};
use nym_mixnet_contract_common::NodeId;
use nym_topology::CachedEpochRewardedSet;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLockReadGuard;

pub(crate) mod chain_status;
pub(crate) mod contract_details;
pub(crate) mod force_refresh;

#[derive(Clone)]
pub(crate) struct AppState {
    // ideally this would have been made generic to make tests easier,
    // however, it'd be a way bigger change (I tried)
    /// Instance of a client used for interacting with the nyx chain.
    pub(crate) nyxd_client: Client,

    /// Holds information about the latest chain block it has queried.
    /// Note, it is not updated on every request. It follows the embedded ttl.
    pub(crate) chain_status_cache: ChainStatusCache,

    /// Holds mapping between a nyx address and tokens/delegations it holds
    pub(crate) address_info_cache: AddressInfoCache,

    /// Holds information on when nym-nodes requested an explicit request of their self-described data.
    /// It is used to prevent DoS by nodes constantly requesting the refresh.
    pub(crate) forced_refresh: ForcedRefresh,

    /// Holds cached state of the Nym Mixnet contract, e.g. bonded nym-nodes, rewarded set, current interval.
    pub(crate) mixnet_contract_cache: MixnetContractCache,

    /// Holds processed information on network nodes, i.e. performance, config scores, etc.
    // TODO: also perhaps redundant?
    pub(crate) node_status_cache: NodeStatusCache,

    /// Holds reference to the persistent storage of this nym-api.
    pub(crate) storage: storage::NymApiStorage,

    /// Holds information on the self-reported information of nodes, e.g. auxiliary keys they use,
    /// ports they announce, etc.
    pub(crate) described_nodes_cache: SharedCache<DescribedNodes>,

    /// Information about the current network this nym-api is connected to, e.g. contract addresses,
    /// endpoints, denominations.
    pub(crate) network_details: NetworkDetails,

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

impl FromRef<AppState> for MixnetContractCache {
    fn from_ref(app_state: &AppState) -> Self {
        app_state.mixnet_contract_cache.clone()
    }
}

impl AppState {
    pub(crate) fn nym_contract_cache(&self) -> &MixnetContractCache {
        &self.mixnet_contract_cache
    }

    pub(crate) fn node_status_cache(&self) -> &NodeStatusCache {
        &self.node_status_cache
    }

    pub(crate) fn network_details(&self) -> &NetworkDetails {
        &self.network_details
    }

    pub(crate) fn described_nodes_cache(&self) -> &SharedCache<DescribedNodes> {
        &self.described_nodes_cache
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
    pub(crate) async fn describe_nodes_cache_data(
        &self,
    ) -> Result<RwLockReadGuard<Cache<DescribedNodes>>, AxumErrorResponse> {
        Ok(self.described_nodes_cache().get().await?)
    }

    pub(crate) async fn rewarded_set(
        &self,
    ) -> Result<Cache<CachedEpochRewardedSet>, AxumErrorResponse> {
        Ok(self.nym_contract_cache().cached_rewarded_set().await?)
    }

    pub(crate) async fn node_annotations(
        &self,
    ) -> Result<RwLockReadGuard<Cache<HashMap<NodeId, NodeAnnotation>>>, AxumErrorResponse> {
        self.node_status_cache()
            .node_annotations()
            .await
            .ok_or_else(AxumErrorResponse::internal)
    }

    pub(crate) async fn legacy_mixnode_annotations(
        &self,
    ) -> Result<RwLockReadGuard<Cache<HashMap<NodeId, MixNodeBondAnnotated>>>, AxumErrorResponse>
    {
        self.node_status_cache()
            .annotated_legacy_mixnodes()
            .await
            .ok_or_else(AxumErrorResponse::internal)
    }

    pub(crate) async fn legacy_gateways_annotations(
        &self,
    ) -> Result<RwLockReadGuard<Cache<HashMap<NodeId, GatewayBondAnnotated>>>, AxumErrorResponse>
    {
        self.node_status_cache()
            .annotated_legacy_gateways()
            .await
            .ok_or_else(AxumErrorResponse::internal)
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
                        self.mixnet_contract_cache.clone(),
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
