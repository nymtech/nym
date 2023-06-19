// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use nym_contracts_common::signing::MessageSignature;
use nym_name_service_common::{msg::ExecuteMsg as NameExecuteMsg, NameDetails, NameId, NymName};

use crate::nyxd::{
    coin::Coin, cosmwasm_client::types::ExecuteResult, error::NyxdError, Fee, NyxdClient,
    SigningCosmWasmClient,
};

#[async_trait]
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

#[async_trait]
impl<C> NameServiceSigningClient for NyxdClient<C>
where
    C: SigningCosmWasmClient + Sync + Send,
{
    async fn execute_name_service_contract(
        &self,
        fee: Option<Fee>,
        msg: NameExecuteMsg,
        funds: Vec<Coin>,
    ) -> Result<ExecuteResult, NyxdError> {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        let memo = msg.default_memo();
        self.client
            .execute(
                self.address(),
                self.name_service_contract_address().ok_or(
                    NyxdError::NoContractAddressAvailable("name service contract".to_string()),
                )?,
                &msg,
                fee,
                memo,
                funds,
            )
            .await
    }
}
