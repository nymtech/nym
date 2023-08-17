// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::contract_traits::NymContractsProvider;
use crate::nyxd::{
    coin::Coin, cosmwasm_client::types::ExecuteResult, error::NyxdError, Fee, SigningCosmWasmClient,
};
use crate::signing::signer::OfflineSigner;
use async_trait::async_trait;
use nym_contracts_common::signing::MessageSignature;
use nym_service_provider_directory_common::{
    msg::ExecuteMsg as SpExecuteMsg, NymAddress, ServiceDetails, ServiceId,
};

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait SpDirectorySigningClient {
    async fn execute_service_provider_directory_contract(
        &self,
        fee: Option<Fee>,
        msg: SpExecuteMsg,
        funds: Vec<Coin>,
    ) -> Result<ExecuteResult, NyxdError>;

    async fn announce_service_provider(
        &self,
        service: ServiceDetails,
        owner_signature: MessageSignature,
        deposit: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_service_provider_directory_contract(
            fee,
            SpExecuteMsg::Announce {
                service,
                owner_signature,
            },
            vec![deposit],
        )
        .await
    }

    async fn delete_service_provider_by_id(
        &self,
        service_id: ServiceId,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_service_provider_directory_contract(
            fee,
            SpExecuteMsg::DeleteId { service_id },
            vec![],
        )
        .await
    }

    async fn delete_service_provider_by_nym_address(
        &self,
        nym_address: NymAddress,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_service_provider_directory_contract(
            fee,
            SpExecuteMsg::DeleteNymAddress { nym_address },
            vec![],
        )
        .await
    }

    async fn update_deposit_required(
        &self,
        deposit_required: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_service_provider_directory_contract(
            fee,
            SpExecuteMsg::UpdateDepositRequired {
                deposit_required: deposit_required.into(),
            },
            vec![],
        )
        .await
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C> SpDirectorySigningClient for C
where
    C: SigningCosmWasmClient + NymContractsProvider + Sync,
    NyxdError: From<<Self as OfflineSigner>::Error>,
{
    async fn execute_service_provider_directory_contract(
        &self,
        fee: Option<Fee>,
        msg: SpExecuteMsg,
        funds: Vec<Coin>,
    ) -> Result<ExecuteResult, NyxdError> {
        let sp_directory_contract_address =
            &self.service_provider_contract_address().ok_or_else(|| {
                NyxdError::unavailable_contract_address("service provider directory contract")
            })?;

        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier())));
        let memo = msg.default_memo();

        let signer_address = &self.signer_addresses()?[0];
        self.execute(
            signer_address,
            sp_directory_contract_address,
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
    fn all_execute_variants_are_covered<C: SpDirectorySigningClient + Send + Sync>(
        client: C,
        msg: SpExecuteMsg,
    ) {
        match msg {
            SpExecuteMsg::Announce {
                service,
                owner_signature,
            } => client
                .announce_service_provider(service, owner_signature, mock_coin(), None)
                .ignore(),
            SpExecuteMsg::DeleteId { service_id } => client
                .delete_service_provider_by_id(service_id, None)
                .ignore(),
            SpExecuteMsg::DeleteNymAddress { nym_address } => client
                .delete_service_provider_by_nym_address(nym_address, None)
                .ignore(),
            SpExecuteMsg::UpdateDepositRequired { deposit_required } => client
                .update_deposit_required(deposit_required.into(), None)
                .ignore(),
        };
    }
}
