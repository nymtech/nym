// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::contract::NymEcashContract;
use crate::helpers::{
    create_batch_redemption_proposal, create_blacklist_proposal, Config, ProposalId,
};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_binary, Addr, Coin, Decimal, Deps, Storage, SubMsg, Uint128};
use cw3::ProposalResponse;
use nym_ecash_contract_common::EcashContractError;
use nym_multisig_contract_common::msg::QueryMsg as MultisigQueryMsg;
use nym_network_defaults::TICKETBOOK_SIZE;
use sylvia::ctx::ExecCtx;

#[cw_serde]
pub(crate) struct Invariants {
    pub(crate) ticket_book_size: u64,
}

impl NymEcashContract {
    pub(crate) fn get_ticketbook_size(
        &self,
        storage: &dyn Storage,
    ) -> Result<u64, EcashContractError> {
        let invariants = self.expected_invariants.load(storage)?;
        if invariants.ticket_book_size != TICKETBOOK_SIZE {
            return Err(EcashContractError::TicketBookSizeChanged {
                at_init: invariants.ticket_book_size,
                current: TICKETBOOK_SIZE,
            });
        }
        Ok(TICKETBOOK_SIZE)
    }

    pub(crate) fn tickets_redemption_amount(
        &self,
        storage: &dyn Storage,
        config: &Config,
        number_of_tickets: u16,
    ) -> Result<Coin, EcashContractError> {
        let deposit_amount = config.deposit_amount.amount;
        let ticketbook_size = Uint128::new(self.get_ticketbook_size(storage)? as u128);
        let tickets = Uint128::new(number_of_tickets as u128);

        // how many tickets from a ticketbook you redeemed
        let book_ratio = Decimal::from_ratio(tickets, ticketbook_size);

        // return = ticketbook_price * (tickets / ticketbook_size)
        let return_amount = deposit_amount.mul_floor(book_ratio);

        Ok(Coin {
            denom: config.deposit_amount.denom.clone(),
            amount: return_amount,
        })
    }

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

    // temporarily dead
    #[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::tests::TestSetupSimple;

    #[test]
    fn ticket_redemption_amount() -> anyhow::Result<()> {
        // make sure the ticketbook size hasn't changed so that our tests are still valid
        assert_eq!(TICKETBOOK_SIZE, 50);

        // ticketbook price of 100nym
        let test = TestSetupSimple::new().with_deposit_amount(100_000_000);
        let res =
            test.contract()
                .tickets_redemption_amount(test.deps().storage, &test.config(), 1)?;
        assert_eq!(res.amount.u128(), 2_000_000);
        let res =
            test.contract()
                .tickets_redemption_amount(test.deps().storage, &test.config(), 2)?;
        assert_eq!(res.amount.u128(), 4_000_000);
        let res =
            test.contract()
                .tickets_redemption_amount(test.deps().storage, &test.config(), 5)?;
        assert_eq!(res.amount.u128(), 10_000_000);
        let res =
            test.contract()
                .tickets_redemption_amount(test.deps().storage, &test.config(), 10)?;
        assert_eq!(res.amount.u128(), 20_000_000);
        let res =
            test.contract()
                .tickets_redemption_amount(test.deps().storage, &test.config(), 30)?;
        assert_eq!(res.amount.u128(), 60_000_000);
        let res =
            test.contract()
                .tickets_redemption_amount(test.deps().storage, &test.config(), 50)?;
        assert_eq!(res.amount.u128(), 100_000_000);
        let res =
            test.contract()
                .tickets_redemption_amount(test.deps().storage, &test.config(), 123)?;
        assert_eq!(res.amount.u128(), 246_000_000);

        // ticketbook price of 1.5unym per ticket
        let test = TestSetupSimple::new().with_deposit_amount(75);
        let res =
            test.contract()
                .tickets_redemption_amount(test.deps().storage, &test.config(), 1)?;
        assert_eq!(res.amount.u128(), 1);
        let res =
            test.contract()
                .tickets_redemption_amount(test.deps().storage, &test.config(), 2)?;
        assert_eq!(res.amount.u128(), 3);
        let res =
            test.contract()
                .tickets_redemption_amount(test.deps().storage, &test.config(), 5)?;
        assert_eq!(res.amount.u128(), 7);
        let res =
            test.contract()
                .tickets_redemption_amount(test.deps().storage, &test.config(), 10)?;
        assert_eq!(res.amount.u128(), 15);
        let res =
            test.contract()
                .tickets_redemption_amount(test.deps().storage, &test.config(), 30)?;
        assert_eq!(res.amount.u128(), 45);
        let res =
            test.contract()
                .tickets_redemption_amount(test.deps().storage, &test.config(), 50)?;
        assert_eq!(res.amount.u128(), 75);
        let res =
            test.contract()
                .tickets_redemption_amount(test.deps().storage, &test.config(), 123)?;
        assert_eq!(res.amount.u128(), 184);

        // ticketbook price of 1unym per ticket
        let test = TestSetupSimple::new().with_deposit_amount(50);
        let res =
            test.contract()
                .tickets_redemption_amount(test.deps().storage, &test.config(), 1)?;
        assert_eq!(res.amount.u128(), 1);
        let res =
            test.contract()
                .tickets_redemption_amount(test.deps().storage, &test.config(), 2)?;
        assert_eq!(res.amount.u128(), 2);
        let res =
            test.contract()
                .tickets_redemption_amount(test.deps().storage, &test.config(), 5)?;
        assert_eq!(res.amount.u128(), 5);
        let res =
            test.contract()
                .tickets_redemption_amount(test.deps().storage, &test.config(), 10)?;
        assert_eq!(res.amount.u128(), 10);
        let res =
            test.contract()
                .tickets_redemption_amount(test.deps().storage, &test.config(), 30)?;
        assert_eq!(res.amount.u128(), 30);
        let res =
            test.contract()
                .tickets_redemption_amount(test.deps().storage, &test.config(), 50)?;
        assert_eq!(res.amount.u128(), 50);
        let res =
            test.contract()
                .tickets_redemption_amount(test.deps().storage, &test.config(), 123)?;
        assert_eq!(res.amount.u128(), 123);

        // ticketbook price of 1unym in total
        let test = TestSetupSimple::new().with_deposit_amount(1);
        let res =
            test.contract()
                .tickets_redemption_amount(test.deps().storage, &test.config(), 1)?;
        assert_eq!(res.amount.u128(), 0);
        let res =
            test.contract()
                .tickets_redemption_amount(test.deps().storage, &test.config(), 2)?;
        assert_eq!(res.amount.u128(), 0);
        let res =
            test.contract()
                .tickets_redemption_amount(test.deps().storage, &test.config(), 5)?;
        assert_eq!(res.amount.u128(), 0);
        let res =
            test.contract()
                .tickets_redemption_amount(test.deps().storage, &test.config(), 10)?;
        assert_eq!(res.amount.u128(), 0);
        let res =
            test.contract()
                .tickets_redemption_amount(test.deps().storage, &test.config(), 30)?;
        assert_eq!(res.amount.u128(), 0);
        let res =
            test.contract()
                .tickets_redemption_amount(test.deps().storage, &test.config(), 50)?;
        assert_eq!(res.amount.u128(), 1);
        let res =
            test.contract()
                .tickets_redemption_amount(test.deps().storage, &test.config(), 123)?;
        assert_eq!(res.amount.u128(), 2);

        // ticketbook price of 0unym
        let test = TestSetupSimple::new().with_deposit_amount(0);
        let res =
            test.contract()
                .tickets_redemption_amount(test.deps().storage, &test.config(), 1)?;
        assert_eq!(res.amount.u128(), 0);
        let res =
            test.contract()
                .tickets_redemption_amount(test.deps().storage, &test.config(), 2)?;
        assert_eq!(res.amount.u128(), 0);
        let res =
            test.contract()
                .tickets_redemption_amount(test.deps().storage, &test.config(), 5)?;
        assert_eq!(res.amount.u128(), 0);
        let res =
            test.contract()
                .tickets_redemption_amount(test.deps().storage, &test.config(), 10)?;
        assert_eq!(res.amount.u128(), 0);
        let res =
            test.contract()
                .tickets_redemption_amount(test.deps().storage, &test.config(), 30)?;
        assert_eq!(res.amount.u128(), 0);
        let res =
            test.contract()
                .tickets_redemption_amount(test.deps().storage, &test.config(), 50)?;
        assert_eq!(res.amount.u128(), 0);
        let res =
            test.contract()
                .tickets_redemption_amount(test.deps().storage, &test.config(), 123)?;
        assert_eq!(res.amount.u128(), 0);

        Ok(())
    }
}
