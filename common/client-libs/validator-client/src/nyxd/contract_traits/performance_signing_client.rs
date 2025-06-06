// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::coin::Coin;
use crate::nyxd::contract_traits::NymContractsProvider;
use crate::nyxd::cosmwasm_client::types::ExecuteResult;
use crate::nyxd::error::NyxdError;
use crate::nyxd::{Fee, SigningCosmWasmClient};
use crate::signing::signer::OfflineSigner;
use async_trait::async_trait;
use nym_performance_contract_common::{types::NodeId, ExecuteMsg as PerformanceExecuteMsg};

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

    // it's enough that this compiles and clippy is happy about it
    #[allow(dead_code)]
    fn all_execute_variants_are_covered<C: PerformanceSigningClient + Send + Sync>(
        client: C,
        msg: PerformanceExecuteMsg,
    ) {
        match msg {
            PerformanceExecuteMsg::UpdateAdmin { .. } => {}
            PerformanceExecuteMsg::Submit { .. } => {}
            PerformanceExecuteMsg::BatchSubmit { .. } => {}
            PerformanceExecuteMsg::AuthoriseNetworkMonitor { .. } => {}
            PerformanceExecuteMsg::RetireNetworkMonitor { .. } => {}
        };
    }
}
