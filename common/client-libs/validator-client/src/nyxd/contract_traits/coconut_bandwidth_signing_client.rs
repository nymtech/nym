// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::contract_traits::NymContractsProvider;
use crate::nyxd::cosmwasm_client::types::ExecuteResult;
use crate::nyxd::error::NyxdError;
use crate::nyxd::{Coin, Fee, SigningCosmWasmClient};
use crate::signing::signer::OfflineSigner;
use async_trait::async_trait;
use nym_coconut_bandwidth_contract_common::spend_credential::SpendCredentialData;
use nym_coconut_bandwidth_contract_common::{
    deposit::DepositData, msg::ExecuteMsg as CoconutBandwidthExecuteMsg,
};

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait CoconutBandwidthSigningClient {
    async fn execute_coconut_bandwidth_contract(
        &self,
        fee: Option<Fee>,
        msg: CoconutBandwidthExecuteMsg,
        memo: String,
        funds: Vec<Coin>,
    ) -> Result<ExecuteResult, NyxdError>;

    async fn deposit(
        &self,
        amount: Coin,
        info: String,
        verification_key: String,
        encryption_key: String,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let req = CoconutBandwidthExecuteMsg::DepositFunds {
            data: DepositData::new(info, verification_key, encryption_key),
        };
        self.execute_coconut_bandwidth_contract(
            fee,
            req,
            "CoconutBandwidth::Deposit".to_string(),
            vec![amount],
        )
        .await
    }

    async fn spend_credential(
        &self,
        funds: Coin,
        blinded_serial_number: String,
        gateway_cosmos_address: String,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let req = CoconutBandwidthExecuteMsg::SpendCredential {
            data: SpendCredentialData::new(
                funds.into(),
                blinded_serial_number,
                gateway_cosmos_address,
            ),
        };
        self.execute_coconut_bandwidth_contract(
            fee,
            req,
            "CoconutBandwidth::SpendCredential".to_string(),
            vec![],
        )
        .await
    }

    async fn release_funds(
        &self,
        amount: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_coconut_bandwidth_contract(
            fee,
            CoconutBandwidthExecuteMsg::ReleaseFunds {
                funds: amount.into(),
            },
            "CoconutBandwidth::ReleaseFunds".to_string(),
            vec![],
        )
        .await
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C> CoconutBandwidthSigningClient for C
where
    C: SigningCosmWasmClient + NymContractsProvider + Sync,
    NyxdError: From<<Self as OfflineSigner>::Error>,
{
    async fn execute_coconut_bandwidth_contract(
        &self,
        fee: Option<Fee>,
        msg: CoconutBandwidthExecuteMsg,
        memo: String,
        funds: Vec<Coin>,
    ) -> Result<ExecuteResult, NyxdError> {
        let coconut_bandwidth_contract_address = self
            .coconut_bandwidth_contract_address()
            .ok_or_else(|| NyxdError::unavailable_contract_address("coconut bandwidth contract"))?;

        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier())));
        let signer_address = &self.signer_addresses()?[0];

        self.execute(
            signer_address,
            coconut_bandwidth_contract_address,
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
    use crate::nyxd::contract_traits::tests::{mock_coin, IgnoreValue};

    // it's enough that this compiles and clippy is happy about it
    #[allow(dead_code)]
    fn all_execute_variants_are_covered<C: CoconutBandwidthSigningClient + Send + Sync>(
        client: C,
        msg: CoconutBandwidthExecuteMsg,
    ) {
        match msg {
            CoconutBandwidthExecuteMsg::DepositFunds { data } => client
                .deposit(
                    mock_coin(),
                    data.deposit_info().to_string(),
                    data.identity_key().to_string(),
                    data.encryption_key().to_string(),
                    None,
                )
                .ignore(),
            CoconutBandwidthExecuteMsg::SpendCredential { data } => client
                .spend_credential(
                    mock_coin(),
                    data.blinded_serial_number().to_string(),
                    data.gateway_cosmos_address().to_string(),
                    None,
                )
                .ignore(),
            CoconutBandwidthExecuteMsg::ReleaseFunds { funds } => {
                client.release_funds(funds.into(), None).ignore()
            }
        };
    }
}
