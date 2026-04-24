// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::contract_traits::NymContractsProvider;
use crate::nyxd::cosmwasm_client::types::ExecuteResult;
use crate::nyxd::error::NyxdError;
use crate::nyxd::{Coin, Fee, SigningCosmWasmClient};
use crate::signing::signer::OfflineSigner;
use async_trait::async_trait;
use nym_network_monitors_contract_common::ExecuteMsg as NetworkMonitorsExecuteMsg;
use std::net::SocketAddr;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait NetworkMonitorsSigningClient {
    async fn execute_network_monitors_contract(
        &self,
        fee: Option<Fee>,
        msg: NetworkMonitorsExecuteMsg,
        memo: String,
        funds: Vec<Coin>,
    ) -> Result<ExecuteResult, NyxdError>;

    async fn update_admin(
        &self,
        admin: String,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let msg = NetworkMonitorsExecuteMsg::UpdateAdmin { admin };
        self.execute_network_monitors_contract(
            fee,
            msg,
            "NetworkMonitorsExecuteMsg::UpdateAdmin".into(),
            vec![],
        )
        .await
    }

    async fn authorise_network_monitor_orchestrator(
        &self,
        address: String,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let msg = NetworkMonitorsExecuteMsg::AuthoriseNetworkMonitorOrchestrator { address };
        self.execute_network_monitors_contract(
            fee,
            msg,
            "NetworkMonitorsExecuteMsg::AuthoriseNetworkMonitorOrchestrator".into(),
            vec![],
        )
        .await
    }

    async fn revoke_network_monitor_orchestrator(
        &self,
        address: String,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let msg = NetworkMonitorsExecuteMsg::RevokeNetworkMonitorOrchestrator { address };
        self.execute_network_monitors_contract(
            fee,
            msg,
            "NetworkMonitorsExecuteMsg::RevokeNetworkMonitorOrchestrator".into(),
            vec![],
        )
        .await
    }

    async fn authorise_network_monitor(
        &self,
        mixnet_address: SocketAddr,
        bs58_x25519_noise: String,
        noise_version: u8,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let msg = NetworkMonitorsExecuteMsg::AuthoriseNetworkMonitor {
            mixnet_address,
            bs58_x25519_noise,
            noise_version,
        };
        self.execute_network_monitors_contract(
            fee,
            msg,
            "NetworkMonitorsExecuteMsg::AuthoriseNetworkMonitor".into(),
            vec![],
        )
        .await
    }

    async fn revoke_network_monitor(
        &self,
        address: SocketAddr,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let msg = NetworkMonitorsExecuteMsg::RevokeNetworkMonitor { address };
        self.execute_network_monitors_contract(
            fee,
            msg,
            "NetworkMonitorsExecuteMsg::RevokeNetworkMonitor".into(),
            vec![],
        )
        .await
    }

    async fn revoke_all_network_monitors(
        &self,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let msg = NetworkMonitorsExecuteMsg::RevokeAllNetworkMonitors;
        self.execute_network_monitors_contract(
            fee,
            msg,
            "NetworkMonitorsExecuteMsg::RevokeAllNetworkMonitors".into(),
            vec![],
        )
        .await
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C> NetworkMonitorsSigningClient for C
where
    C: SigningCosmWasmClient + NymContractsProvider + Sync,
    NyxdError: From<<Self as OfflineSigner>::Error>,
{
    async fn execute_network_monitors_contract(
        &self,
        fee: Option<Fee>,
        msg: NetworkMonitorsExecuteMsg,
        memo: String,
        funds: Vec<Coin>,
    ) -> Result<ExecuteResult, NyxdError> {
        let contract_address = &self
            .network_monitors_contract_address()
            .ok_or_else(|| NyxdError::unavailable_contract_address("network monitors contract"))?;

        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier())));

        let signer_address = &self.signer_addresses()[0];
        self.execute(signer_address, contract_address, &msg, fee, memo, funds)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nyxd::contract_traits::tests::IgnoreValue;
    use nym_network_monitors_contract_common::ExecuteMsg;

    // it's enough that this compiles and clippy is happy about it
    #[allow(dead_code)]
    fn all_execute_variants_are_covered<C: NetworkMonitorsSigningClient + Send + Sync>(
        client: C,
        msg: NetworkMonitorsExecuteMsg,
    ) {
        match msg {
            NetworkMonitorsExecuteMsg::UpdateAdmin { admin } => {
                client.update_admin(admin, None).ignore()
            }
            ExecuteMsg::AuthoriseNetworkMonitorOrchestrator { address } => client
                .authorise_network_monitor_orchestrator(address, None)
                .ignore(),
            ExecuteMsg::RevokeNetworkMonitorOrchestrator { address } => client
                .revoke_network_monitor_orchestrator(address, None)
                .ignore(),
            ExecuteMsg::AuthoriseNetworkMonitor {
                mixnet_address: address,
                bs58_x25519_noise,
                noise_version,
            } => client
                .authorise_network_monitor(address, bs58_x25519_noise, noise_version, None)
                .ignore(),
            ExecuteMsg::RevokeNetworkMonitor { address } => {
                client.revoke_network_monitor(address, None).ignore()
            }
            ExecuteMsg::RevokeAllNetworkMonitors => {
                client.revoke_all_network_monitors(None).ignore()
            }
        };
    }
}
