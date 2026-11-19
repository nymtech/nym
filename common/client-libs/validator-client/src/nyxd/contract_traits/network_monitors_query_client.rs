// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::collect_paged;
use crate::nyxd::contract_traits::{check_pinned_read_progress, NymContractsProvider};
use crate::nyxd::error::NyxdError;
use crate::nyxd::{CosmWasmClient, Height};
use async_trait::async_trait;
use nym_network_monitors_contract_common::{
    AuthorisedNetworkMonitor, AuthorisedNetworkMonitorOrchestratorsResponse,
    AuthorisedNetworkMonitorsPagedResponse, QueryMsg as NetworkMonitorsQueryMsg,
};
use serde::Deserialize;
use std::net::SocketAddr;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait NetworkMonitorsQueryClient {
    async fn query_network_monitors_contract<T>(
        &self,
        query: NetworkMonitorsQueryMsg,
    ) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>;

    async fn get_admin(&self) -> Result<cw_controllers::AdminResponse, NyxdError> {
        self.query_network_monitors_contract(NetworkMonitorsQueryMsg::Admin {})
            .await
    }

    async fn get_network_monitor_orchestrators(
        &self,
    ) -> Result<AuthorisedNetworkMonitorOrchestratorsResponse, NyxdError> {
        self.query_network_monitors_contract(
            NetworkMonitorsQueryMsg::NetworkMonitorOrchestrators {},
        )
        .await
    }

    async fn get_network_monitor_agents_paged(
        &self,
        start_next_after: Option<SocketAddr>,
        limit: Option<u32>,
    ) -> Result<AuthorisedNetworkMonitorsPagedResponse, NyxdError> {
        self.query_network_monitors_contract(NetworkMonitorsQueryMsg::NetworkMonitorAgents {
            start_next_after,
            limit,
        })
        .await
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait PagedNetworkMonitorsQueryClient: NetworkMonitorsQueryClient {
    async fn get_all_network_monitor_agents(
        &self,
    ) -> Result<Vec<AuthorisedNetworkMonitor>, NyxdError> {
        collect_paged!(self, get_network_monitor_agents_paged, authorised)
    }
}

#[async_trait]
impl<T> PagedNetworkMonitorsQueryClient for T where T: NetworkMonitorsQueryClient {}

/// Height-pinned reads, for callers that need the agent set exactly as it stood at one height.
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait PinnedNetworkMonitorsQueryClient {
    /// Every authorised agent at exactly `height`.
    ///
    /// Unlike [`PagedNetworkMonitorsQueryClient::get_all_network_monitor_agents`], every page is
    /// requested at the same height, so an authorisation or revocation landing mid-enumeration
    /// cannot duplicate or drop an agent.
    async fn get_all_network_monitor_agents_at_height(
        &self,
        height: Height,
    ) -> Result<Vec<AuthorisedNetworkMonitor>, NyxdError>;
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C> PinnedNetworkMonitorsQueryClient for C
where
    C: CosmWasmClient + NymContractsProvider + Send + Sync,
{
    async fn get_all_network_monitor_agents_at_height(
        &self,
        height: Height,
    ) -> Result<Vec<AuthorisedNetworkMonitor>, NyxdError> {
        let contract_address = self
            .network_monitors_contract_address()
            .ok_or_else(|| NyxdError::unavailable_contract_address("network monitors contract"))?;

        let mut agents = Vec::new();
        let mut start_next_after = None;
        loop {
            let page: AuthorisedNetworkMonitorsPagedResponse = self
                .query_contract_smart_at_height(
                    contract_address,
                    &NetworkMonitorsQueryMsg::NetworkMonitorAgents {
                        start_next_after,
                        limit: None,
                    },
                    Some(height),
                )
                .await?;

            agents.extend(page.authorised);
            match page.start_next_after {
                Some(cursor) => {
                    check_pinned_read_progress(
                        "network monitors contract",
                        start_next_after.as_ref(),
                        &cursor,
                        agents.len(),
                    )?;
                    start_next_after = Some(cursor)
                }
                None => break,
            }
        }
        Ok(agents)
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C> NetworkMonitorsQueryClient for C
where
    C: CosmWasmClient + NymContractsProvider + Send + Sync,
{
    async fn query_network_monitors_contract<T>(
        &self,
        query: NetworkMonitorsQueryMsg,
    ) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>,
    {
        let contract_address = &self
            .network_monitors_contract_address()
            .ok_or_else(|| NyxdError::unavailable_contract_address("network monitors contract"))?;
        self.query_contract_smart(contract_address, &query).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nyxd::contract_traits::tests::IgnoreValue;

    // it's enough that this compiles and clippy is happy about it
    #[allow(dead_code)]
    fn all_query_variants_are_covered<C: NetworkMonitorsQueryClient + Send + Sync>(
        client: C,
        msg: NetworkMonitorsQueryMsg,
    ) {
        match msg {
            NetworkMonitorsQueryMsg::Admin {} => client.get_admin().ignore(),
            NetworkMonitorsQueryMsg::NetworkMonitorOrchestrators {} => {
                client.get_network_monitor_orchestrators().ignore()
            }
            NetworkMonitorsQueryMsg::NetworkMonitorAgents { .. } => {
                client.get_network_monitor_agents_paged(None, None).ignore()
            }
        };
    }
}
