// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::collect_paged;
use crate::nyxd::contract_traits::NymContractsProvider;
use crate::nyxd::error::NyxdError;
use crate::nyxd::CosmWasmClient;
use async_trait::async_trait;
use cosmrs::AccountId;
use serde::Deserialize;

pub use nym_performance_contract_common::{
    msg::QueryMsg as PerformanceQueryMsg, types::NetworkMonitorResponse, EpochId,
    EpochMeasurementsPagedResponse, EpochNodePerformance, EpochPerformancePagedResponse,
    FullHistoricalPerformancePagedResponse, HistoricalPerformance, LastSubmission,
    NetworkMonitorInformation, NetworkMonitorsPagedResponse, NodeId, NodeMeasurement,
    NodeMeasurementsResponse, NodePerformance, NodePerformancePagedResponse,
    NodePerformanceResponse, RetiredNetworkMonitor, RetiredNetworkMonitorsPagedResponse,
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
        node_id: NodeId,
    ) -> Result<Vec<NodeMeasurement>, NyxdError> {
        collect_paged!(self, get_epoch_measurements_paged, measurements, node_id)
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
