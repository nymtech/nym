// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use nym_service_provider_directory_common::{
    msg::ExecuteMsg as SpExecuteMsg, NymAddress, ServiceId, ServiceType,
};

use crate::nyxd::{
    coin::Coin, cosmwasm_client::types::ExecuteResult, error::NyxdError, Fee, NyxdClient,
    SigningCosmWasmClient,
};

#[async_trait]
pub trait SpDirectorySigningClient {
    async fn exectute_service_provider_directory_contract(
        &self,
        fee: Option<Fee>,
        msg: SpExecuteMsg,
        funds: Vec<Coin>,
    ) -> Result<ExecuteResult, NyxdError>;

    async fn announce_service_provider(
        &self,
        nym_address: NymAddress,
        service_type: ServiceType,
        deposit: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.exectute_service_provider_directory_contract(
            fee,
            SpExecuteMsg::Announce {
                nym_address,
                service_type,
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
        self.exectute_service_provider_directory_contract(
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
        self.exectute_service_provider_directory_contract(
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
        self.exectute_service_provider_directory_contract(
            fee,
            SpExecuteMsg::UpdateDepositRequired {
                deposit_required: deposit_required.into(),
            },
            vec![],
        )
        .await
    }
}

#[async_trait]
impl<C> SpDirectorySigningClient for NyxdClient<C>
where
    C: SigningCosmWasmClient + Sync + Send,
{
    async fn exectute_service_provider_directory_contract(
        &self,
        fee: Option<Fee>,
        msg: SpExecuteMsg,
        funds: Vec<Coin>,
    ) -> Result<ExecuteResult, NyxdError> {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        let memo = msg.default_memo();
        self.client
            .execute(
                self.address(),
                self.service_provider_contract_address(),
                &msg,
                fee,
                memo,
                funds,
            )
            .await
    }
}
