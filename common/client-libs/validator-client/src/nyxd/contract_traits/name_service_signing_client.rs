// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use nym_contracts_common::signing::MessageSignature;
use nym_name_service_common::{msg::ExecuteMsg as NameExecuteMsg, NameDetails, NameId, NymName};

use crate::nyxd::contract_traits::NymContractsProvider;
use crate::nyxd::{
    coin::Coin, cosmwasm_client::types::ExecuteResult, error::NyxdError, Fee, SigningCosmWasmClient,
};
use crate::signing::signer::OfflineSigner;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait NameServiceSigningClient {
    async fn execute_name_service_contract(
        &self,
        fee: Option<Fee>,
        msg: NameExecuteMsg,
        funds: Vec<Coin>,
    ) -> Result<ExecuteResult, NyxdError>;

    async fn register_name(
        &self,
        name: NameDetails,
        owner_signature: MessageSignature,
        deposit: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_name_service_contract(
            fee,
            NameExecuteMsg::Register {
                name,
                owner_signature,
            },
            vec![deposit],
        )
        .await
    }

    async fn delete_name_by_id(
        &self,
        name_id: NameId,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_name_service_contract(fee, NameExecuteMsg::DeleteId { name_id }, vec![])
            .await
    }

    async fn delete_service_provider_by_name(
        &self,
        name: NymName,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_name_service_contract(fee, NameExecuteMsg::DeleteName { name }, vec![])
            .await
    }

    async fn update_deposit_required(
        &self,
        deposit_required: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_name_service_contract(
            fee,
            NameExecuteMsg::UpdateDepositRequired {
                deposit_required: deposit_required.into(),
            },
            vec![],
        )
        .await
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C> NameServiceSigningClient for C
where
    C: SigningCosmWasmClient + NymContractsProvider + Sync,
    NyxdError: From<<Self as OfflineSigner>::Error>,
{
    async fn execute_name_service_contract(
        &self,
        fee: Option<Fee>,
        msg: NameExecuteMsg,
        funds: Vec<Coin>,
    ) -> Result<ExecuteResult, NyxdError> {
        let name_service_contract_address = &self
            .name_service_contract_address()
            .ok_or_else(|| NyxdError::unavailable_contract_address("name service contract"))?;

        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier())));
        let memo = msg.default_memo();

        let signer_address = &self.signer_addresses()?[0];
        self.execute(
            signer_address,
            name_service_contract_address,
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
    fn all_execute_variants_are_covered<C: NameServiceSigningClient + Send + Sync>(
        client: C,
        msg: NameExecuteMsg,
    ) {
        match msg {
            NameExecuteMsg::Register {
                name,
                owner_signature,
            } => client
                .register_name(name, owner_signature, mock_coin(), None)
                .ignore(),
            NameExecuteMsg::DeleteId { name_id } => {
                client.delete_name_by_id(name_id, None).ignore()
            }
            NameExecuteMsg::DeleteName { name } => {
                client.delete_service_provider_by_name(name, None).ignore()
            }
            NameExecuteMsg::UpdateDepositRequired { deposit_required } => client
                .update_deposit_required(deposit_required.into(), None)
                .ignore(),
        };
    }
}
