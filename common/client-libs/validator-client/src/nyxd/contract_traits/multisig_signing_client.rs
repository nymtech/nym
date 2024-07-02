// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::contract_traits::NymContractsProvider;
use crate::nyxd::cosmwasm_client::types::ExecuteResult;
use crate::nyxd::error::NyxdError;
use crate::nyxd::{Coin, Fee, SigningCosmWasmClient};
use crate::signing::signer::OfflineSigner;
use async_trait::async_trait;
use cosmwasm_std::{to_binary, CosmosMsg, WasmMsg};
use cw3::Vote;
use cw4::{MemberChangedHookMsg, MemberDiff};
use nym_coconut_bandwidth_contract_common::msg::ExecuteMsg as CoconutBandwidthExecuteMsg;
use nym_multisig_contract_common::msg::ExecuteMsg as MultisigExecuteMsg;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait MultisigSigningClient: NymContractsProvider {
    async fn execute_multisig_contract(
        &self,
        fee: Option<Fee>,
        msg: MultisigExecuteMsg,
        memo: String,
        funds: Vec<Coin>,
    ) -> Result<ExecuteResult, NyxdError>;

    async fn propose_release_funds(
        &self,
        title: String,
        blinded_serial_number: String,
        voucher_value: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let ecash_contract_address = self
            .ecash_contract_address()
            .ok_or_else(|| NyxdError::unavailable_contract_address("coconut bandwidth contract"))?;

        let release_funds_req = CoconutBandwidthExecuteMsg::ReleaseFunds {
            funds: voucher_value.into(),
        };
        let release_funds_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: ecash_contract_address.to_string(),
            msg: to_binary(&release_funds_req)?,
            funds: vec![],
        });
        let req = MultisigExecuteMsg::Propose {
            title,
            description: blinded_serial_number,
            msgs: vec![release_funds_msg],
            latest: None,
        };
        self.execute_multisig_contract(
            fee,
            req,
            "Multisig::Propose::Execute::ReleaseFunds".to_string(),
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
        let vote = if vote_yes { Vote::Yes } else { Vote::No };
        let req = MultisigExecuteMsg::Vote { proposal_id, vote };
        self.execute_multisig_contract(fee, req, "Multisig::Vote".to_string(), vec![])
            .await
    }

    // alternative variant to vote_proposal that lets you to abstain and veto a proposal
    async fn vote(
        &self,
        proposal_id: u64,
        vote: Vote,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_multisig_contract(
            fee,
            MultisigExecuteMsg::Vote { proposal_id, vote },
            "Multisig::Vote".to_string(),
            vec![],
        )
        .await
    }

    async fn execute_proposal(
        &self,
        proposal_id: u64,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let req = MultisigExecuteMsg::Execute { proposal_id };
        self.execute_multisig_contract(fee, req, "Multisig::Execute".to_string(), vec![])
            .await
    }

    async fn close_proposal(
        &self,
        proposal_id: u64,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_multisig_contract(
            fee,
            MultisigExecuteMsg::Close { proposal_id },
            "Multisig::Close".to_string(),
            vec![],
        )
        .await
    }

    async fn changed_member_hook(
        &self,
        member_diff: Vec<MemberDiff>,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_multisig_contract(
            fee,
            MultisigExecuteMsg::MemberChangedHook(MemberChangedHookMsg::new(member_diff)),
            "Multisig::MemberChangedHook".to_string(),
            vec![],
        )
        .await
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C> MultisigSigningClient for C
where
    C: SigningCosmWasmClient + NymContractsProvider + Sync,
    NyxdError: From<<Self as OfflineSigner>::Error>,
{
    async fn execute_multisig_contract(
        &self,
        fee: Option<Fee>,
        msg: MultisigExecuteMsg,
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
    use crate::nyxd::contract_traits::tests::{mock_coin, IgnoreValue};

    // it's enough that this compiles and clippy is happy about it
    #[allow(dead_code)]
    fn all_execute_variants_are_covered<C: MultisigSigningClient + Send + Sync>(
        client: C,
        msg: MultisigExecuteMsg,
    ) {
        match msg {
            MultisigExecuteMsg::Propose {
                title, description, ..
            } => client
                .propose_release_funds(title, description, mock_coin(), None)
                .ignore(),
            MultisigExecuteMsg::Vote { proposal_id, vote } => {
                client.vote(proposal_id, vote, None).ignore()
            }
            MultisigExecuteMsg::Execute { proposal_id } => {
                client.execute_proposal(proposal_id, None).ignore()
            }
            MultisigExecuteMsg::Close { proposal_id } => {
                client.close_proposal(proposal_id, None).ignore()
            }
            MultisigExecuteMsg::MemberChangedHook(hook_msg) => {
                client.changed_member_hook(hook_msg.diffs, None).ignore()
            }
        };
    }
}
