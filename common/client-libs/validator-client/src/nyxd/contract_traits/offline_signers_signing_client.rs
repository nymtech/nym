// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::contract_traits::NymContractsProvider;
use crate::nyxd::cosmwasm_client::types::ExecuteResult;
use crate::nyxd::error::NyxdError;
use crate::nyxd::{Coin, Fee, SigningCosmWasmClient};
use crate::signing::signer::OfflineSigner;
use async_trait::async_trait;
use cosmrs::AccountId;
use nym_offline_signers_contract_common::msg::ExecuteMsg as OfflineSignersExecuteMsg;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait OfflineSignersSigningClient {
    async fn execute_offline_signers_contract(
        &self,
        fee: Option<Fee>,
        msg: OfflineSignersExecuteMsg,
        memo: String,
        funds: Vec<Coin>,
    ) -> Result<ExecuteResult, NyxdError>;

    async fn update_admin(
        &self,
        admin: String,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_offline_signers_contract(
            fee,
            OfflineSignersExecuteMsg::UpdateAdmin { admin },
            "OfflineSignersContract::UpdateAdmin".to_string(),
            vec![],
        )
        .await
    }
    async fn propose_or_vote(
        &self,
        signer: AccountId,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_offline_signers_contract(
            fee,
            OfflineSignersExecuteMsg::ProposeOrVote {
                signer: signer.to_string(),
            },
            "OfflineSignersContract::ProposeOrVote".to_string(),
            vec![],
        )
        .await
    }

    async fn reset_offline_status(&self, fee: Option<Fee>) -> Result<ExecuteResult, NyxdError> {
        self.execute_offline_signers_contract(
            fee,
            OfflineSignersExecuteMsg::ResetOfflineStatus {},
            "OfflineSignersContract::ResetOfflineStatus".to_string(),
            vec![],
        )
        .await
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C> OfflineSignersSigningClient for C
where
    C: SigningCosmWasmClient + NymContractsProvider + Sync,
    NyxdError: From<<Self as OfflineSigner>::Error>,
{
    async fn execute_offline_signers_contract(
        &self,
        fee: Option<Fee>,
        msg: OfflineSignersExecuteMsg,
        memo: String,
        funds: Vec<Coin>,
    ) -> Result<ExecuteResult, NyxdError> {
        let multisig_contract_address = self
            .multisig_contract_address()
            .ok_or_else(|| NyxdError::unavailable_contract_address("multisig contract"))?;

        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier())));

        let signer_address = &self.signer_addresses()?[0];
        self.execute(
            signer_address,
            multisig_contract_address,
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
    fn all_execute_variants_are_covered<C: OfflineSignersSigningClient + Send + Sync>(
        client: C,
        msg: OfflineSignersExecuteMsg,
    ) {
        match msg {
            OfflineSignersExecuteMsg::UpdateAdmin { admin } => {
                client.update_admin(admin, None).ignore()
            }
            OfflineSignersExecuteMsg::ProposeOrVote { signer } => client
                .propose_or_vote(signer.parse().unwrap(), None)
                .ignore(),
            OfflineSignersExecuteMsg::ResetOfflineStatus {} => {
                client.reset_offline_status(None).ignore()
            }
        };
    }
}
