// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub use crate::nyxd::cosmwasm_client::signing_client::SigningCosmWasmClient;
use crate::nyxd::cosmwasm_client::types::ExecuteResult;
use crate::nyxd::error::NyxdError;
use crate::nyxd::{Fee, NyxdClient};

use cw3::Vote;
use nym_coconut_bandwidth_contract_common::msg::ExecuteMsg as CoconutBandwidthExecuteMsg;
use nym_multisig_contract_common::msg::ExecuteMsg;

use async_trait::async_trait;
use cosmwasm_std::{to_binary, Coin, CosmosMsg, WasmMsg};

#[async_trait]
pub trait MultisigSigningClient {
    async fn propose_release_funds(
        &self,
        title: String,
        blinded_serial_number: String,
        voucher_value: u128,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError>;

    async fn vote_proposal(
        &self,
        proposal_id: u64,
        yes: bool,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError>;

    async fn execute_proposal(
        &self,
        proposal_id: u64,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError>;
}

#[async_trait]
impl<C: SigningCosmWasmClient + Sync + Send + Clone> MultisigSigningClient for NyxdClient<C> {
    async fn propose_release_funds(
        &self,
        title: String,
        blinded_serial_number: String,
        voucher_value: u128,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        let release_funds_req = CoconutBandwidthExecuteMsg::ReleaseFunds {
            funds: Coin::new(
                voucher_value,
                self.config.chain_details.mix_denom.base.clone(),
            ),
        };
        let release_funds_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.coconut_bandwidth_contract_address().to_string(),
            msg: to_binary(&release_funds_req)?,
            funds: vec![],
        });
        let req = ExecuteMsg::Propose {
            title,
            description: blinded_serial_number,
            msgs: vec![release_funds_msg],
            latest: None,
        };
        self.client
            .execute(
                self.address(),
                self.multisig_contract_address(),
                &req,
                fee,
                "Multisig::Propose::Execute::ReleaseFunds",
                vec![],
            )
            .await
    }

    async fn vote_proposal(
        &self,
        proposal_id: u64,
        vote_yes: bool,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        let vote = if vote_yes { Vote::Yes } else { Vote::No };
        let req = ExecuteMsg::Vote { proposal_id, vote };
        self.client
            .execute(
                self.address(),
                self.multisig_contract_address(),
                &req,
                fee,
                "Multisig::Vote",
                vec![],
            )
            .await
    }

    async fn execute_proposal(
        &self,
        proposal_id: u64,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        let req = ExecuteMsg::Execute { proposal_id };
        self.client
            .execute(
                self.address(),
                self.multisig_contract_address(),
                &req,
                fee,
                "Multisig::Execute",
                vec![],
            )
            .await
    }
}
