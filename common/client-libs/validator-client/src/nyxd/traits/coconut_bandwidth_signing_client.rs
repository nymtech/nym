// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub use crate::nyxd::cosmwasm_client::signing_client::SigningCosmWasmClient;
use crate::nyxd::cosmwasm_client::types::ExecuteResult;
use crate::nyxd::error::NyxdError;
use crate::nyxd::{Coin, Fee, NyxdClient};
use nym_coconut_bandwidth_contract_common::spend_credential::SpendCredentialData;
use nym_coconut_bandwidth_contract_common::{deposit::DepositData, msg::ExecuteMsg};

use async_trait::async_trait;

#[async_trait]
pub trait CoconutBandwidthSigningClient {
    async fn deposit(
        &self,
        amount: Coin,
        info: String,
        verification_key: String,
        encryption_key: String,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError>;
    async fn spend_credential(
        &self,
        funds: Coin,
        blinded_serial_number: String,
        gateway_cosmos_address: String,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError>;
}

#[async_trait]
impl<C: SigningCosmWasmClient + Sync + Send> CoconutBandwidthSigningClient for NyxdClient<C> {
    async fn deposit(
        &self,
        amount: Coin,
        info: String,
        verification_key: String,
        encryption_key: String,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        let req = ExecuteMsg::DepositFunds {
            data: DepositData::new(info.to_string(), verification_key, encryption_key),
        };
        self.client
            .execute(
                self.address(),
                self.coconut_bandwidth_contract_address(),
                &req,
                fee,
                "CoconutBandwidth::Deposit",
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
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        let req = ExecuteMsg::SpendCredential {
            data: SpendCredentialData::new(
                funds.into(),
                blinded_serial_number,
                gateway_cosmos_address,
            ),
        };
        self.client
            .execute(
                self.address(),
                self.coconut_bandwidth_contract_address(),
                &req,
                fee,
                "CoconutBandwidth::SpendCredential",
                vec![],
            )
            .await
    }
}
