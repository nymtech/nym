// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub use crate::nymd::cosmwasm_client::signing_client::SigningCosmWasmClient;
use crate::nymd::cosmwasm_client::types::ExecuteResult;
use crate::nymd::error::NymdError;
use crate::nymd::fee::helpers::Operation;
use crate::nymd::NymdClient;
use coconut_bandwidth_contract_common::msg::ExecuteMsg as CoconutBandwidthExecuteMsg;
use coconut_interface::{Base58, Credential};
use multisig_contract_common::msg::ExecuteMsg;

use async_trait::async_trait;
use cosmwasm_std::{to_binary, Coin, CosmosMsg, WasmMsg};
use network_defaults::DEFAULT_NETWORK;

#[async_trait]
pub trait MultisigSigningClient {
    async fn propose_release_funds(
        &self,
        credential: &Credential,
    ) -> Result<ExecuteResult, NymdError>;
}

#[async_trait]
impl<C: SigningCosmWasmClient + Sync + Send> MultisigSigningClient for NymdClient<C> {
    async fn propose_release_funds(
        &self,
        credential: &Credential,
    ) -> Result<ExecuteResult, NymdError> {
        let fee = self.operation_fee(Operation::BandwidthProposal);
        let release_funds_req = CoconutBandwidthExecuteMsg::ReleaseFunds {
            funds: Coin::new(credential.voucher_value() as u128, DEFAULT_NETWORK.denom()),
        };
        let release_funds_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.coconut_bandwidth_contract_address().to_string(),
            msg: to_binary(&release_funds_req)?,
            funds: vec![],
        });
        let req = ExecuteMsg::Propose {
            title: String::from("Bandwidth consumption request"),
            description: credential.to_bs58(),
            msgs: vec![release_funds_msg],
            latest: None,
        };
        self.client
            .execute(
                self.address(),
                self.multisig_contract_address(),
                &req,
                fee,
                "Multisig::Propose",
                vec![],
            )
            .await
    }
}
