// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub use crate::nymd::cosmwasm_client::signing_client::SigningCosmWasmClient;
use crate::nymd::cosmwasm_client::types::ExecuteResult;
use crate::nymd::error::NymdError;
use crate::nymd::{Coin, Fee, NymdClient};
use coconut_bandwidth_contract_common::{deposit::DepositData, msg::ExecuteMsg};

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
    ) -> Result<ExecuteResult, NymdError>;
}

#[async_trait]
impl<C: SigningCosmWasmClient + Sync + Send> CoconutBandwidthSigningClient for NymdClient<C> {
    async fn deposit(
        &self,
        amount: Coin,
        info: String,
        verification_key: String,
        encryption_key: String,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError> {
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
}
