// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::contract::NymEcashContract;
use crate::helpers::{create_batch_redemption_proposal, create_blacklist_proposal, ProposalId};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_binary, Addr, Coin, Deps, SubMsg};
use cw3::ProposalResponse;
use nym_ecash_contract_common::EcashContractError;
use nym_multisig_contract_common::msg::QueryMsg as MultisigQueryMsg;
use sylvia::types::ExecCtx;

#[cw_serde]
pub(crate) struct Invariants {
    pub(crate) ticket_book_value: Coin,
    pub(crate) ticket_value: Coin,
}

impl NymEcashContract<'_> {
    fn must_get_multisig_addr(&self, deps: Deps) -> Result<Addr, EcashContractError> {
        // SAFETY: multisig admin MUST always be set on initialisation,
        // if the call fails, we're in some weird UB land
        #[allow(clippy::expect_used)]
        Ok(self
            .multisig
            .get(deps)?
            .expect("multisig admin must always be set on initialisation"))
    }

    pub(crate) fn create_redemption_proposal(
        &self,
        ctx: ExecCtx,
        commitment_bs58: String,
        number_of_tickets: u16,
    ) -> Result<SubMsg, EcashContractError> {
        let multisig_addr = self.must_get_multisig_addr(ctx.deps.as_ref())?;

        create_batch_redemption_proposal(
            commitment_bs58,
            ctx.info.sender.into_string(),
            number_of_tickets,
            ctx.env.contract.address.into_string(),
            multisig_addr.into_string(),
        )
        .map_err(Into::into)
    }

    pub(crate) fn create_blacklist_proposal(
        &self,
        ctx: ExecCtx,
        public_key: String,
    ) -> Result<SubMsg, EcashContractError> {
        let multisig_addr = self.must_get_multisig_addr(ctx.deps.as_ref())?;

        create_blacklist_proposal(
            public_key,
            ctx.env.contract.address.into_string(),
            multisig_addr.into_string(),
        )
        .map_err(Into::into)
    }

    pub(crate) fn query_multisig_proposal(
        &self,
        deps: Deps,
        proposal_id: ProposalId,
    ) -> Result<ProposalResponse, EcashContractError> {
        let msg = MultisigQueryMsg::Proposal { proposal_id };
        let multisig_addr = self.must_get_multisig_addr(deps)?;

        let proposal_response: ProposalResponse = deps.querier.query(
            &cosmwasm_std::QueryRequest::Wasm(cosmwasm_std::WasmQuery::Smart {
                contract_addr: multisig_addr.to_string(),
                msg: to_binary(&msg)?,
            }),
        )?;
        Ok(proposal_response)
    }
}
