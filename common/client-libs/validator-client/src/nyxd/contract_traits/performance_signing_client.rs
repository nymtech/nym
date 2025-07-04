// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::coin::Coin;
use crate::nyxd::contract_traits::NymContractsProvider;
use crate::nyxd::cosmwasm_client::types::ExecuteResult;
use crate::nyxd::cosmwasm_client::ContractResponseData;
use crate::nyxd::error::NyxdError;
use crate::nyxd::{Fee, SigningCosmWasmClient};
use crate::signing::signer::OfflineSigner;
use async_trait::async_trait;
use nym_performance_contract_common::{
    EpochId, ExecuteMsg as PerformanceExecuteMsg, NodeId, NodePerformance,
    RemoveEpochMeasurementsResponse,
};

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait PerformanceSigningClient {
    async fn execute_performance_contract(
        &self,
        fee: Option<Fee>,
        msg: PerformanceExecuteMsg,
        memo: String,
        funds: Vec<Coin>,
    ) -> Result<ExecuteResult, NyxdError>;

    async fn update_admin(
        &self,
        admin: String,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_performance_contract(
            fee,
            PerformanceExecuteMsg::UpdateAdmin { admin },
            "PerformanceContract::UpdateAdmin".to_string(),
            vec![],
        )
        .await
    }

    async fn submit_performance(
        &self,
        epoch: EpochId,
        data: NodePerformance,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_performance_contract(
            fee,
            PerformanceExecuteMsg::Submit { epoch, data },
            "PerformanceContract::Submit".to_string(),
            vec![],
        )
        .await
    }

    async fn batch_submit_performance(
        &self,
        epoch: EpochId,
        data: Vec<NodePerformance>,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_performance_contract(
            fee,
            PerformanceExecuteMsg::BatchSubmit { epoch, data },
            "PerformanceContract::BatchSubmit".to_string(),
            vec![],
        )
        .await
    }

    async fn authorise_network_monitor(
        &self,
        address: String,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_performance_contract(
            fee,
            PerformanceExecuteMsg::AuthoriseNetworkMonitor { address },
            "PerformanceContract::AuthoriseNetworkMonitor".to_string(),
            vec![],
        )
        .await
    }

    async fn retire_network_monitor(
        &self,
        address: String,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_performance_contract(
            fee,
            PerformanceExecuteMsg::RetireNetworkMonitor { address },
            "PerformanceContract::RetireNetworkMonitor".to_string(),
            vec![],
        )
        .await
    }

    async fn remove_node_measurements(
        &self,
        epoch_id: EpochId,
        node_id: NodeId,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_performance_contract(
            fee,
            PerformanceExecuteMsg::RemoveNodeMeasurements { epoch_id, node_id },
            "PerformanceContract::RemoveNodeMeasurements".to_string(),
            vec![],
        )
        .await
    }

    async fn partial_remove_epoch_measurements(
        &self,
        epoch_id: EpochId,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_performance_contract(
            fee,
            PerformanceExecuteMsg::RemoveEpochMeasurements { epoch_id },
            "PerformanceContract::RemoveEpochMeasurements".to_string(),
            vec![],
        )
        .await
    }

    async fn remove_epoch_measurements(
        &self,
        epoch_id: EpochId,
        fee: Option<Fee>,
    ) -> Result<(), NyxdError> {
        loop {
            let execute_res = self
                .partial_remove_epoch_measurements(epoch_id, fee.clone())
                .await?;
            let response = execute_res
                .parse_singleton_json_contract_response::<RemoveEpochMeasurementsResponse>()?;
            if !response.additional_entries_to_remove_remaining {
                break;
            }
        }
        Ok(())
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C> PerformanceSigningClient for C
where
    C: SigningCosmWasmClient + NymContractsProvider + Sync,
    NyxdError: From<<Self as OfflineSigner>::Error>,
{
    async fn execute_performance_contract(
        &self,
        fee: Option<Fee>,
        msg: PerformanceExecuteMsg,
        memo: String,
        funds: Vec<Coin>,
    ) -> Result<ExecuteResult, NyxdError> {
        let performance_contract_address = &self
            .performance_contract_address()
            .ok_or_else(|| NyxdError::unavailable_contract_address("performance contract"))?;

        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier())));

        let signer_address = &self.signer_addresses()?[0];
        self.execute(
            signer_address,
            performance_contract_address,
            &msg,
            fee,
            memo,
            funds,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nyxd::contract_traits::tests::IgnoreValue;
    use nym_performance_contract_common::ExecuteMsg;

    // it's enough that this compiles and clippy is happy about it
    #[allow(dead_code)]
    fn all_execute_variants_are_covered<C: PerformanceSigningClient + Send + Sync>(
        client: C,
        msg: PerformanceExecuteMsg,
    ) {
        match msg {
            PerformanceExecuteMsg::UpdateAdmin { admin } => {
                client.update_admin(admin, None).ignore()
            }
            PerformanceExecuteMsg::Submit { epoch, data } => {
                client.submit_performance(epoch, data, None).ignore()
            }
            PerformanceExecuteMsg::BatchSubmit { epoch, data } => {
                client.batch_submit_performance(epoch, data, None).ignore()
            }
            PerformanceExecuteMsg::AuthoriseNetworkMonitor { address } => {
                client.authorise_network_monitor(address, None).ignore()
            }
            PerformanceExecuteMsg::RetireNetworkMonitor { address } => {
                client.retire_network_monitor(address, None).ignore()
            }
            ExecuteMsg::RemoveNodeMeasurements { epoch_id, node_id } => client
                .remove_node_measurements(epoch_id, node_id, None)
                .ignore(),
            ExecuteMsg::RemoveEpochMeasurements { epoch_id } => client
                .partial_remove_epoch_measurements(epoch_id, None)
                .ignore(),
        };
    }
}
