// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::collect_paged;
use crate::nyxd::contract_traits::{
    collect_pages_at_height, AttestationPage, NymContractsProvider,
};
use crate::nyxd::error::NyxdError;
use crate::nyxd::{CosmWasmClient, Height};
use async_trait::async_trait;
use cosmrs::AccountId;
use serde::Deserialize;

pub use nym_performance_contract_common::{
    msg::QueryMsg as PerformanceQueryMsg, types::NetworkMonitorResponse, DigestResponse, EpochId,
    EpochMeasurementsPagedResponse, EpochNodeMeasurements, EpochNodePerformance,
    EpochPerformancePagedResponse, EpochWeights, FullHistoricalPerformancePagedResponse,
    HistoricalPerformance, KindMedians, LastKnownEpochResponse, LastSubmission, LatestPerformance,
    LatestPerformancePagedResponse, NetworkMonitorInformation, NetworkMonitorsPagedResponse,
    NodeId, NodeMeasurements, NodeMeasurementsResponse, NodePerformance,
    NodePerformancePagedResponse, NodePerformanceResponse, ResolvedMedians, RetiredNetworkMonitor,
    RetiredNetworkMonitorsPagedResponse, RewardingInputsResponse, RewardingScoreResponse, Weights,
    WeightsResponse,
};

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait PerformanceQueryClient {
    async fn query_performance_contract<T>(
        &self,
        query: PerformanceQueryMsg,
    ) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>;

    async fn admin(&self) -> Result<cw_controllers::AdminResponse, NyxdError> {
        self.query_performance_contract(PerformanceQueryMsg::Admin {})
            .await
    }

    async fn get_node_performance(
        &self,
        epoch_id: EpochId,
        node_id: NodeId,
    ) -> Result<NodePerformanceResponse, NyxdError> {
        self.query_performance_contract(PerformanceQueryMsg::NodePerformance { epoch_id, node_id })
            .await
    }

    async fn get_node_performance_paged(
        &self,
        node_id: NodeId,
        start_after: Option<EpochId>,
        limit: Option<u32>,
    ) -> Result<NodePerformancePagedResponse, NyxdError> {
        self.query_performance_contract(PerformanceQueryMsg::NodePerformancePaged {
            node_id,
            start_after,
            limit,
        })
        .await
    }

    async fn get_node_measurements(
        &self,
        epoch_id: EpochId,
        node_id: NodeId,
    ) -> Result<NodeMeasurementsResponse, NyxdError> {
        self.query_performance_contract(PerformanceQueryMsg::NodeMeasurements { epoch_id, node_id })
            .await
    }

    async fn get_epoch_measurements_paged(
        &self,
        epoch_id: EpochId,
        start_after: Option<NodeId>,
        limit: Option<u32>,
    ) -> Result<EpochMeasurementsPagedResponse, NyxdError> {
        self.query_performance_contract(PerformanceQueryMsg::EpochMeasurementsPaged {
            epoch_id,
            start_after,
            limit,
        })
        .await
    }

    async fn get_epoch_performance_paged(
        &self,
        epoch_id: EpochId,
        start_after: Option<NodeId>,
        limit: Option<u32>,
    ) -> Result<EpochPerformancePagedResponse, NyxdError> {
        self.query_performance_contract(PerformanceQueryMsg::EpochPerformancePaged {
            epoch_id,
            start_after,
            limit,
        })
        .await
    }

    async fn get_full_historical_performance_paged(
        &self,
        start_after: Option<(EpochId, NodeId)>,
        limit: Option<u32>,
    ) -> Result<FullHistoricalPerformancePagedResponse, NyxdError> {
        self.query_performance_contract(PerformanceQueryMsg::FullHistoricalPerformancePaged {
            start_after,
            limit,
        })
        .await
    }

    /// Every node's latest entry, exactly as the digest commits it, in ascending node order.
    /// A verifying caller pins every page and the digest read to one block height.
    async fn get_latest_performance_paged(
        &self,
        start_after: Option<NodeId>,
        limit: Option<u32>,
    ) -> Result<LatestPerformancePagedResponse, NyxdError> {
        self.query_performance_contract(PerformanceQueryMsg::LatestPerformancePaged {
            start_after,
            limit,
        })
        .await
    }

    /// The 32-byte collapse of the digest accumulator. Unproven: a client that needs a proof
    /// reads the raw digest key instead.
    async fn get_performance_digest(&self) -> Result<DigestResponse, NyxdError> {
        self.query_performance_contract(PerformanceQueryMsg::Digest {})
            .await
    }

    /// Everything rewarding uses for the node in the epoch: the resolved medians, the epoch they
    /// came from, the weights in force and the score.
    async fn get_rewarding_inputs(
        &self,
        epoch_id: EpochId,
        node_id: NodeId,
    ) -> Result<RewardingInputsResponse, NyxdError> {
        self.query_performance_contract(PerformanceQueryMsg::RewardingInputs { epoch_id, node_id })
            .await
    }

    /// Only the score rewarding uses for the node in the epoch.
    async fn get_rewarding_score(
        &self,
        epoch_id: EpochId,
        node_id: NodeId,
    ) -> Result<RewardingScoreResponse, NyxdError> {
        self.query_performance_contract(PerformanceQueryMsg::RewardingScore { epoch_id, node_id })
            .await
    }

    async fn get_last_known_epoch(
        &self,
        node_id: NodeId,
    ) -> Result<LastKnownEpochResponse, NyxdError> {
        self.query_performance_contract(PerformanceQueryMsg::LastKnownEpoch { node_id })
            .await
    }

    async fn get_weights_at(&self, epoch_id: EpochId) -> Result<WeightsResponse, NyxdError> {
        self.query_performance_contract(PerformanceQueryMsg::WeightsAt { epoch_id })
            .await
    }

    async fn get_current_weights(&self) -> Result<WeightsResponse, NyxdError> {
        self.query_performance_contract(PerformanceQueryMsg::CurrentWeights {})
            .await
    }

    async fn get_network_monitor(
        &self,
        address: &AccountId,
    ) -> Result<NetworkMonitorResponse, NyxdError> {
        self.query_performance_contract(PerformanceQueryMsg::NetworkMonitor {
            address: address.to_string(),
        })
        .await
    }

    async fn get_network_monitors_paged(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Result<NetworkMonitorsPagedResponse, NyxdError> {
        self.query_performance_contract(PerformanceQueryMsg::NetworkMonitorsPaged {
            start_after,
            limit,
        })
        .await
    }

    async fn get_retired_network_monitors_paged(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Result<RetiredNetworkMonitorsPagedResponse, NyxdError> {
        self.query_performance_contract(PerformanceQueryMsg::RetiredNetworkMonitorsPaged {
            start_after,
            limit,
        })
        .await
    }

    async fn get_last_submission(&self) -> Result<LastSubmission, NyxdError> {
        self.query_performance_contract(PerformanceQueryMsg::LastSubmittedMeasurement {})
            .await
    }
}

// extension trait to the query client to deal with the paged queries
// (it didn't feel appropriate to combine it with the existing trait
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait PagedPerformanceQueryClient: PerformanceQueryClient {
    async fn get_all_node_performance(
        &self,
        node_id: NodeId,
    ) -> Result<Vec<EpochNodePerformance>, NyxdError> {
        collect_paged!(self, get_node_performance_paged, performance, node_id)
    }

    async fn get_all_epoch_measurements(
        &self,
        epoch_id: EpochId,
    ) -> Result<Vec<NodeMeasurements>, NyxdError> {
        collect_paged!(self, get_epoch_measurements_paged, measurements, epoch_id)
    }

    async fn get_all_epoch_performance(
        &self,
        epoch_id: EpochId,
    ) -> Result<Vec<NodePerformance>, NyxdError> {
        collect_paged!(self, get_epoch_performance_paged, performance, epoch_id)
    }

    async fn get_all_full_historical_performance(
        &self,
    ) -> Result<Vec<HistoricalPerformance>, NyxdError> {
        collect_paged!(self, get_full_historical_performance_paged, performance)
    }

    /// Pins no height, so never verify it against a digest: use
    /// [`PinnedPerformanceQueryClient::get_all_latest_performance_at_height`].
    async fn get_all_latest_performance(&self) -> Result<Vec<LatestPerformance>, NyxdError> {
        collect_paged!(self, get_latest_performance_paged, performance)
    }

    async fn get_all_network_monitors(&self) -> Result<Vec<NetworkMonitorInformation>, NyxdError> {
        collect_paged!(self, get_network_monitors_paged, info)
    }

    async fn get_all_retired_network_monitors(
        &self,
    ) -> Result<Vec<RetiredNetworkMonitor>, NyxdError> {
        collect_paged!(self, get_retired_network_monitors_paged, info)
    }
}

#[async_trait]
impl<T> PagedPerformanceQueryClient for T where T: PerformanceQueryClient {}

/// Height-pinned reads, for callers that compare what they read against a digest proven at
/// the same height, or read alongside it.
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait PinnedPerformanceQueryClient {
    /// Every node's latest entry at exactly `height`.
    ///
    /// Unlike [`PagedPerformanceQueryClient::get_all_latest_performance`], every page is
    /// requested at the same height, so a submission landing mid-enumeration cannot duplicate
    /// or drop an entry. That is what makes the result safe to fold into an accumulator and
    /// compare against the digest proven at `height`.
    async fn get_all_latest_performance_at_height(
        &self,
        height: Height,
    ) -> Result<Vec<LatestPerformance>, NyxdError>;

    /// Every node's performance in `epoch_id`, as of `height`.
    async fn get_all_epoch_performance_at_height(
        &self,
        epoch_id: EpochId,
        height: Height,
    ) -> Result<Vec<NodePerformance>, NyxdError>;

    /// The weights in force at `height`.
    async fn get_current_weights_at_height(
        &self,
        height: Height,
    ) -> Result<WeightsResponse, NyxdError>;

    /// The last submission as of `height`.
    async fn get_last_submission_at_height(
        &self,
        height: Height,
    ) -> Result<LastSubmission, NyxdError>;
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C> PinnedPerformanceQueryClient for C
where
    C: CosmWasmClient + NymContractsProvider + Send + Sync,
{
    async fn get_all_latest_performance_at_height(
        &self,
        height: Height,
    ) -> Result<Vec<LatestPerformance>, NyxdError> {
        collect_pages_at_height::<LatestPerformancePagedResponse, _>(self, height, ()).await
    }

    async fn get_all_epoch_performance_at_height(
        &self,
        epoch_id: EpochId,
        height: Height,
    ) -> Result<Vec<NodePerformance>, NyxdError> {
        collect_pages_at_height::<EpochPerformancePagedResponse, _>(self, height, epoch_id).await
    }

    async fn get_current_weights_at_height(
        &self,
        height: Height,
    ) -> Result<WeightsResponse, NyxdError> {
        let contract_address = self
            .performance_contract_address()
            .ok_or_else(|| NyxdError::unavailable_contract_address("performance contract"))?;

        self.query_contract_smart_at_height(
            contract_address,
            &PerformanceQueryMsg::CurrentWeights {},
            Some(height),
        )
        .await
    }

    async fn get_last_submission_at_height(
        &self,
        height: Height,
    ) -> Result<LastSubmission, NyxdError> {
        let contract_address = self
            .performance_contract_address()
            .ok_or_else(|| NyxdError::unavailable_contract_address("performance contract"))?;

        self.query_contract_smart_at_height(
            contract_address,
            &PerformanceQueryMsg::LastSubmittedMeasurement {},
            Some(height),
        )
        .await
    }
}

impl AttestationPage for LatestPerformancePagedResponse {
    type QueryMsg = PerformanceQueryMsg;
    type QueryArgs = ();
    type Record = LatestPerformance;
    type Cursor = NodeId;

    const CONTRACT: &'static str = "performance contract";

    fn contract_address<C: NymContractsProvider>(client: &C) -> Option<&AccountId> {
        client.performance_contract_address()
    }

    fn query_msg(_args: &(), start_after: Option<NodeId>) -> PerformanceQueryMsg {
        PerformanceQueryMsg::LatestPerformancePaged {
            start_after,
            limit: None,
        }
    }

    fn into_parts(self) -> (Vec<LatestPerformance>, Option<NodeId>) {
        (self.performance, self.start_next_after)
    }
}

impl AttestationPage for EpochPerformancePagedResponse {
    type QueryMsg = PerformanceQueryMsg;
    type QueryArgs = EpochId;
    type Record = NodePerformance;
    type Cursor = NodeId;

    const CONTRACT: &'static str = "performance contract";

    fn contract_address<C: NymContractsProvider>(client: &C) -> Option<&AccountId> {
        client.performance_contract_address()
    }

    fn query_msg(epoch_id: &EpochId, start_after: Option<NodeId>) -> PerformanceQueryMsg {
        PerformanceQueryMsg::EpochPerformancePaged {
            epoch_id: *epoch_id,
            start_after,
            limit: None,
        }
    }

    fn into_parts(self) -> (Vec<NodePerformance>, Option<NodeId>) {
        (self.performance, self.start_next_after)
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C> PerformanceQueryClient for C
where
    C: CosmWasmClient + NymContractsProvider + Send + Sync,
{
    async fn query_performance_contract<T>(
        &self,
        query: PerformanceQueryMsg,
    ) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>,
    {
        let performance_contract_address = &self
            .performance_contract_address()
            .ok_or_else(|| NyxdError::unavailable_contract_address("performance contract"))?;
        self.query_contract_smart(performance_contract_address, &query)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nyxd::contract_traits::tests::IgnoreValue;
    use nym_performance_contract_common::QueryMsg;

    // it's enough that this compiles and clippy is happy about it
    #[allow(dead_code)]
    fn all_query_variants_are_covered<C: PerformanceQueryClient + Send + Sync>(
        client: C,
        msg: PerformanceQueryMsg,
    ) {
        match msg {
            PerformanceQueryMsg::Admin {} => client.admin().ignore(),
            PerformanceQueryMsg::NodePerformance { epoch_id, node_id } => {
                client.get_node_performance(epoch_id, node_id).ignore()
            }
            PerformanceQueryMsg::NodePerformancePaged {
                node_id,
                start_after,
                limit,
            } => client
                .get_node_performance_paged(node_id, start_after, limit)
                .ignore(),
            PerformanceQueryMsg::NodeMeasurements { epoch_id, node_id } => {
                client.get_node_measurements(epoch_id, node_id).ignore()
            }
            PerformanceQueryMsg::EpochMeasurementsPaged {
                epoch_id,
                start_after,
                limit,
            } => client
                .get_epoch_measurements_paged(epoch_id, start_after, limit)
                .ignore(),
            PerformanceQueryMsg::EpochPerformancePaged {
                epoch_id,
                start_after,
                limit,
            } => client
                .get_epoch_performance_paged(epoch_id, start_after, limit)
                .ignore(),
            PerformanceQueryMsg::FullHistoricalPerformancePaged { start_after, limit } => client
                .get_full_historical_performance_paged(start_after, limit)
                .ignore(),
            PerformanceQueryMsg::LatestPerformancePaged { start_after, limit } => client
                .get_latest_performance_paged(start_after, limit)
                .ignore(),
            PerformanceQueryMsg::Digest {} => client.get_performance_digest().ignore(),
            PerformanceQueryMsg::RewardingInputs { epoch_id, node_id } => {
                client.get_rewarding_inputs(epoch_id, node_id).ignore()
            }
            PerformanceQueryMsg::RewardingScore { epoch_id, node_id } => {
                client.get_rewarding_score(epoch_id, node_id).ignore()
            }
            PerformanceQueryMsg::LastKnownEpoch { node_id } => {
                client.get_last_known_epoch(node_id).ignore()
            }
            PerformanceQueryMsg::WeightsAt { epoch_id } => client.get_weights_at(epoch_id).ignore(),
            PerformanceQueryMsg::CurrentWeights {} => client.get_current_weights().ignore(),
            PerformanceQueryMsg::NetworkMonitor { address } => client
                .get_network_monitor(&address.parse().unwrap())
                .ignore(),
            PerformanceQueryMsg::NetworkMonitorsPaged { start_after, limit } => client
                .get_network_monitors_paged(start_after, limit)
                .ignore(),
            PerformanceQueryMsg::RetiredNetworkMonitorsPaged { start_after, limit } => client
                .get_retired_network_monitors_paged(start_after, limit)
                .ignore(),
            QueryMsg::LastSubmittedMeasurement {} => client.get_last_submission().ignore(),
        };
    }
}
