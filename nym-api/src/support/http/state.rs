// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::circulating_supply_api::cache::CirculatingSupplyCache;
use crate::network::models::NetworkDetails;
use crate::node_describe_cache::DescribedNodes;
use crate::node_status_api::handlers::unstable;
use crate::node_status_api::models::AxumErrorResponse;
use crate::node_status_api::NodeStatusCache;
use crate::nym_contract_cache::cache::{CachedRewardedSet, NymContractCache};
use crate::support::caching::cache::SharedCache;
use crate::support::caching::Cache;
use crate::support::storage;
use nym_api_requests::models::{GatewayBondAnnotated, MixNodeBondAnnotated, NodeAnnotation};
use nym_mixnet_contract_common::NodeId;
use nym_task::TaskManager;
use std::collections::HashMap;
use tokio::sync::RwLockReadGuard;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

pub(crate) const TASK_MANAGER_TIMEOUT_S: u64 = 10;

/// Shutdown goes 2 directions:
/// 1. signal background tasks to gracefully finish
/// 2. signal server itself
///
/// These are done through separate shutdown handles. Of course, shut down server
/// AFTER you have shut down BG tasks (or past their grace period).
pub(crate) struct ShutdownHandles {
    task_manager: TaskManager,
    axum_shutdown_button: ShutdownAxum,
    /// Tokio JoinHandle for axum server's task
    axum_join_handle: AxumJoinHandle,
}

impl ShutdownHandles {
    /// Cancellation token is given to Axum server constructor. When the token
    /// receives a shutdown signal, Axum server will shut down gracefully.
    pub(crate) fn new(
        task_manager: TaskManager,
        axum_server_handle: AxumJoinHandle,
        shutdown_button: CancellationToken,
    ) -> Self {
        Self {
            task_manager,
            axum_shutdown_button: ShutdownAxum(shutdown_button.clone()),
            axum_join_handle: axum_server_handle,
        }
    }

    pub(crate) fn task_manager_mut(&mut self) -> &mut TaskManager {
        &mut self.task_manager
    }

    /// Signal server to shut down, then return join handle to its
    /// `tokio` task
    ///
    /// https://tikv.github.io/doc/tokio/task/struct.JoinHandle.html
    #[must_use]
    pub(crate) fn shutdown_axum(self) -> AxumJoinHandle {
        self.axum_shutdown_button.0.cancel();
        self.axum_join_handle
    }
}

struct ShutdownAxum(CancellationToken);

type AxumJoinHandle = JoinHandle<std::io::Result<()>>;

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) nym_contract_cache: NymContractCache,
    pub(crate) node_status_cache: NodeStatusCache,
    pub(crate) circulating_supply_cache: CirculatingSupplyCache,
    pub(crate) storage: storage::NymApiStorage,
    pub(crate) described_nodes_cache: SharedCache<DescribedNodes>,
    pub(crate) network_details: NetworkDetails,
    pub(crate) node_info_cache: unstable::NodeInfoCache,
}

impl AppState {
    pub(crate) fn nym_contract_cache(&self) -> &NymContractCache {
        &self.nym_contract_cache
    }

    pub(crate) fn node_status_cache(&self) -> &NodeStatusCache {
        &self.node_status_cache
    }

    pub(crate) fn circulating_supply_cache(&self) -> &CirculatingSupplyCache {
        &self.circulating_supply_cache
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
    ) -> Result<RwLockReadGuard<Cache<CachedRewardedSet>>, AxumErrorResponse> {
        self.nym_contract_cache()
            .rewarded_set()
            .await
            .ok_or_else(AxumErrorResponse::internal)
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
}
