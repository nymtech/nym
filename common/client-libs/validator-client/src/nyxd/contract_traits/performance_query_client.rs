// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::contract_traits::NymContractsProvider;
use crate::nyxd::error::NyxdError;
use crate::nyxd::CosmWasmClient;
use async_trait::async_trait;
pub use nym_performance_contract_common::{
    msg::QueryMsg as PerformanceQueryMsg, types::NetworkMonitorResponse,
};
use serde::Deserialize;

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
}

// extension trait to the query client to deal with the paged queries
// (it didn't feel appropriate to combine it with the existing trait
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait PagedPerformanceQueryClient: PerformanceQueryClient {
    //
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

    // it's enough that this compiles and clippy is happy about it
    #[allow(dead_code)]
    fn all_query_variants_are_covered<C: PerformanceQueryClient + Send + Sync>(
        client: C,
        msg: PerformanceQueryMsg,
    ) {
        match msg {
            PerformanceQueryMsg::Admin {} => client.admin().ignore(),
            PerformanceQueryMsg::NodePerformance { .. } => {}
            PerformanceQueryMsg::NodePerformancePaged { .. } => {}
            PerformanceQueryMsg::NodeMeasurements { .. } => {}
            PerformanceQueryMsg::EpochMeasurementsPaged { .. } => {}
            PerformanceQueryMsg::EpochPerformancePaged { .. } => {}
            PerformanceQueryMsg::FullHistoricalPerformancePaged { .. } => {}
            PerformanceQueryMsg::NetworkMonitor { .. } => {}
            PerformanceQueryMsg::NetworkMonitorsPaged { .. } => {}
            PerformanceQueryMsg::RetiredNetworkMonitorsPaged { .. } => {}
        }
    }
}
