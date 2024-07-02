// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::contract::helpers::Invariants;
use crate::deposit::DepositStorage;
use crate::helpers::{
    BlacklistKey, Config, MultisigReply, BLACKLIST_PAGE_DEFAULT_LIMIT, BLACKLIST_PAGE_MAX_LIMIT,
    CONTRACT_NAME, CONTRACT_VERSION, DEPOSITS_PAGE_DEFAULT_LIMIT, DEPOSITS_PAGE_MAX_LIMIT,
};
use cosmwasm_std::{BankMsg, Coin, Decimal, Event, Order, Reply, Response, StdResult, Uint128};
use cw4::Cw4Contract;
use cw_controllers::Admin;
use cw_storage_plus::{Bound, Item, Map};
use nym_contracts_common::set_build_information;
use nym_ecash_contract_common::blacklist::{
    BlacklistedAccount, BlacklistedAccountResponse, Blacklisting, PagedBlacklistedAccountResponse,
};
use nym_ecash_contract_common::deposit::{DepositData, DepositResponse, PagedDepositsResponse};
use nym_ecash_contract_common::events::{
    BLACKLIST_PROPOSAL_REPLY_ID, DEPOSITED_FUNDS_EVENT_TYPE, DEPOSIT_ID,
    PROPOSAL_ID_ATTRIBUTE_NAME, REDEMPTION_PROPOSAL_REPLY_ID, TICKET_BOOK_VALUE, TICKET_VALUE,
};
use nym_ecash_contract_common::EcashContractError;
use sylvia::types::{ExecCtx, InstantiateCtx, MigrateCtx, QueryCtx, ReplyCtx};
use sylvia::{contract, entry_points};

mod helpers;

#[cfg(test)]
mod test;

pub struct NymEcashContract<'a> {
    pub(crate) multisig: Admin<'a>,
    pub(crate) config: Item<'a, Config>,
    pub(crate) expected_invariants: Item<'a, Invariants>,

    pub(crate) blacklist: Map<'a, BlacklistKey, Blacklisting>,

    pub(crate) deposits: DepositStorage<'a>,
}

#[entry_points]
#[contract]
#[error(EcashContractError)]
impl NymEcashContract<'_> {
    #[allow(clippy::new_without_default)]
    pub const fn new() -> Self {
        Self {
            multisig: Admin::new("multisig"),
            config: Item::new("config"),
            expected_invariants: Item::new("expected_invariants"),
            blacklist: Map::new("blacklist"),
            deposits: DepositStorage::new(),
        }
    }

    #[msg(instantiate)]
    pub fn instantiate(
        &self,
        mut ctx: InstantiateCtx,
        holding_account: String,
        multisig_addr: String,
        group_addr: String,
        mix_denom: String,
    ) -> Result<Response, EcashContractError> {
        let multisig_addr = ctx.deps.api.addr_validate(&multisig_addr)?;
        let holding_account = ctx.deps.api.addr_validate(&holding_account)?;
        let group_addr = Cw4Contract(ctx.deps.api.addr_validate(&group_addr).map_err(|_| {
            EcashContractError::InvalidGroup {
                addr: group_addr.clone(),
            }
        })?);

        self.multisig
            .set(ctx.deps.branch(), Some(multisig_addr.clone()))?;

        self.expected_invariants.save(
            ctx.deps.storage,
            &Invariants {
                ticket_book_value: Coin::new(TICKET_BOOK_VALUE, &mix_denom),
                ticket_value: Coin::new(TICKET_VALUE, &mix_denom),
            },
        )?;

        let cfg = Config {
            group_addr,
            mix_denom,
            holding_account,

            redemption_gateway_share: Decimal::percent(5),
        };
        self.config.save(ctx.deps.storage, &cfg)?;

        cw2::set_contract_version(ctx.deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
        set_build_information!(ctx.deps.storage)?;

        Ok(Response::default())
    }

    /*==================
    ======QUERIES=======
    ==================*/
    #[msg(query)]
    pub fn get_blacklist_paged(
        &self,
        ctx: QueryCtx,
        limit: Option<u32>,
        start_after: Option<String>,
    ) -> StdResult<PagedBlacklistedAccountResponse> {
        let limit = limit
            .unwrap_or(BLACKLIST_PAGE_DEFAULT_LIMIT)
            .min(BLACKLIST_PAGE_MAX_LIMIT) as usize;

        let start = start_after.as_deref().map(Bound::exclusive);

        let nodes = self
            .blacklist
            .range(ctx.deps.storage, start, None, Order::Ascending)
            .take(limit)
            .map(|res| res.map(Into::into))
            .collect::<StdResult<Vec<BlacklistedAccount>>>()?;

        let start_next_after = nodes
            .last()
            .map(|account: &BlacklistedAccount| account.public_key().to_string());

        Ok(PagedBlacklistedAccountResponse::new(
            nodes,
            limit,
            start_next_after,
        ))
    }

    #[msg(query)]
    pub fn get_blacklisted_account(
        &self,
        ctx: QueryCtx,
        public_key: String,
    ) -> StdResult<BlacklistedAccountResponse> {
        let account = self.blacklist.may_load(ctx.deps.storage, public_key)?;
        Ok(BlacklistedAccountResponse::new(account))
    }

    #[msg(query)]
    pub fn get_required_deposit_amount(&self, ctx: QueryCtx) -> Result<Coin, EcashContractError> {
        let mix_denom = self.config.load(ctx.deps.storage)?.mix_denom;
        let expected_deposit = self
            .expected_invariants
            .load(ctx.deps.storage)?
            .ticket_book_value;
        let current = Coin::new(TICKET_BOOK_VALUE, mix_denom);
        if expected_deposit != current {
            return Err(EcashContractError::DepositAmountChanged {
                at_init: expected_deposit,
                current,
            });
        }

        Ok(current)
    }

    #[msg(query)]
    pub fn get_deposit(
        &self,
        ctx: QueryCtx,
        deposit_id: u32,
    ) -> Result<DepositResponse, EcashContractError> {
        Ok(DepositResponse {
            id: deposit_id,
            deposit: self.deposits.try_load_by_id(ctx.deps.storage, deposit_id)?,
        })
    }

    #[msg(query)]
    pub fn get_deposits_paged(
        &self,
        ctx: QueryCtx,
        limit: Option<u32>,
        start_after: Option<u32>,
    ) -> StdResult<PagedDepositsResponse> {
        let limit = limit
            .unwrap_or(DEPOSITS_PAGE_DEFAULT_LIMIT)
            .min(DEPOSITS_PAGE_MAX_LIMIT) as usize;

        let start = start_after.map(Bound::exclusive);

        let deposits = self
            .deposits
            .range(ctx.deps.storage, start, None, Order::Ascending)
            .take(limit)
            .map(|res| res.map(Into::into))
            .collect::<StdResult<Vec<DepositData>>>()?;

        let start_next_after = deposits.last().map(|deposit| deposit.id);

        Ok(PagedDepositsResponse {
            deposits,
            start_next_after,
        })
    }

    /*=====================
    ======EXECUTIONS=======
    =====================*/

    #[msg(exec)]
    pub fn deposit_ticket_book_funds(
        &self,
        ctx: ExecCtx,
        identity_key: String,
    ) -> Result<Response, EcashContractError> {
        let mix_denom = self.config.load(ctx.deps.storage)?.mix_denom;
        let voucher_value = cw_utils::must_pay(&ctx.info, &mix_denom)?;
        let amount = voucher_value.u128();

        let expected_deposit = self
            .expected_invariants
            .load(ctx.deps.storage)?
            .ticket_book_value;
        if expected_deposit.amount.u128() != TICKET_BOOK_VALUE {
            return Err(EcashContractError::DepositAmountChanged {
                at_init: expected_deposit,
                current: Coin::new(TICKET_BOOK_VALUE, mix_denom),
            });
        }

        if amount != TICKET_BOOK_VALUE {
            return Err(EcashContractError::WrongAmount {
                received: amount,
                amount: TICKET_BOOK_VALUE,
            });
        }

        let deposit_id = self.deposits.save_deposit(ctx.deps.storage, identity_key)?;

        Ok(Response::new()
            .add_event(
                Event::new(DEPOSITED_FUNDS_EVENT_TYPE)
                    .add_attribute(DEPOSIT_ID, deposit_id.to_string()),
            )
            .set_data(deposit_id.to_be_bytes()))
    }

    #[msg(exec)]
    pub fn request_redemption(
        &self,
        ctx: ExecCtx,
        commitment_bs58: String,
        number_of_tickets: u16,
    ) -> Result<Response, EcashContractError> {
        // basic validation of commitment to make sure it's a valid sha256 digest
        let Ok(digest) = bs58::decode(&commitment_bs58).into_vec() else {
            return Err(EcashContractError::MalformedRedemptionCommitment);
        };
        if digest.len() != 32 {
            return Err(EcashContractError::MalformedRedemptionCommitment);
        }

        let msg = self.create_redemption_proposal(ctx, commitment_bs58, number_of_tickets)?;
        Ok(Response::new().add_submessage(msg))
    }

    #[msg(exec)]
    pub fn redeem_tickets(
        &self,
        ctx: ExecCtx,
        n: u16,
        gw: String,
    ) -> Result<Response, EcashContractError> {
        // only a mutlisig proposal can do that
        self.multisig
            .assert_admin(ctx.deps.as_ref(), &ctx.info.sender)?;

        let config = self.config.load(ctx.deps.storage)?;
        let denom = &config.mix_denom;

        let expected_ticket = self
            .expected_invariants
            .load(ctx.deps.storage)?
            .ticket_value;
        let current = Coin::new(TICKET_VALUE, denom);
        if expected_ticket != current {
            return Err(EcashContractError::TicketValueChanged {
                at_init: expected_ticket,
                current,
            });
        }

        // TODO: we need unit tests for this
        let return_amount = Uint128::new(TICKET_VALUE * n as u128);
        let gw_share = config.redemption_gateway_share * return_amount;
        let holding_share = return_amount - gw_share;

        Ok(Response::new()
            .add_message(BankMsg::Send {
                to_address: gw,
                amount: vec![Coin {
                    denom: denom.to_owned(),
                    amount: gw_share,
                }],
            })
            .add_message(BankMsg::Send {
                to_address: config.holding_account.to_string(),
                amount: vec![Coin {
                    denom: denom.to_owned(),
                    amount: holding_share,
                }],
            }))
    }

    #[msg(exec)]
    pub fn propose_to_blacklist(
        &self,
        ctx: ExecCtx,
        public_key: String,
    ) -> Result<Response, EcashContractError> {
        let cfg = self.config.load(ctx.deps.storage)?;
        cfg.group_addr
            .is_voting_member(&ctx.deps.querier, &ctx.info.sender, ctx.env.block.height)?
            .ok_or(EcashContractError::Unauthorized)?;

        if let Some(blacklisted) = self
            .blacklist
            .may_load(ctx.deps.storage, public_key.clone())?
        {
            // return existing proposal id
            Ok(Response::new().add_attribute(
                PROPOSAL_ID_ATTRIBUTE_NAME,
                blacklisted.proposal_id.to_string(),
            ))
        } else {
            let msg = self.create_blacklist_proposal(ctx, public_key)?;
            Ok(Response::new().add_submessage(msg))
        }
    }

    #[msg(exec)]
    pub fn add_to_blacklist(
        &self,
        ctx: ExecCtx,
        public_key: String,
    ) -> Result<Response, EcashContractError> {
        //Only by multisig contract, actually add public key to blacklist
        self.multisig
            .assert_admin(ctx.deps.as_ref(), &ctx.info.sender)?;

        let mut blacklisting = self.blacklist.load(ctx.deps.storage, public_key.clone())?;
        blacklisting.finalized_at_height = Some(ctx.env.block.height);
        self.blacklist
            .save(ctx.deps.storage, public_key.clone(), &blacklisting)?;

        Ok(Response::new())
    }

    /*=====================
    =========REPLY=========
    =====================*/
    #[msg(reply)]
    pub fn reply(&self, ctx: ReplyCtx, msg: Reply) -> Result<Response, EcashContractError> {
        match msg.id {
            n if n == BLACKLIST_PROPOSAL_REPLY_ID => self.handle_blacklist_proposal_reply(ctx, msg),
            n if n == REDEMPTION_PROPOSAL_REPLY_ID => {
                self.handle_redemption_proposal_reply(ctx, msg)
            }
            other => Err(EcashContractError::InvalidReplyId { id: other }),
        }
    }

    fn handle_blacklist_proposal_reply(
        &self,
        ctx: ReplyCtx,
        msg: Reply,
    ) -> Result<Response, EcashContractError> {
        let proposal_id = msg.multisig_proposal_id()?;

        let proposal = self.query_multisig_proposal(ctx.deps.as_ref(), proposal_id)?;
        let public_key = proposal.description;
        self.blacklist.save(
            ctx.deps.storage,
            public_key,
            &Blacklisting::new(proposal_id),
        )?;

        // TODO: that `BLACKLIST_PROPOSAL_ID` might be redundant since it should be available from cw3 event
        Ok(Response::new().add_attribute(PROPOSAL_ID_ATTRIBUTE_NAME, proposal_id.to_string()))
    }

    fn handle_redemption_proposal_reply(
        &self,
        _ctx: ReplyCtx,
        msg: Reply,
    ) -> Result<Response, EcashContractError> {
        let proposal_id = msg.multisig_proposal_id()?;

        // emit the proposal_id in the response data for easy client access and to make sure it can't be tampered with
        // (since it's included as part of block hash)

        Ok(Response::new().set_data(proposal_id.to_be_bytes()))
    }

    /*=====================
    =======MIGRATION=======
    =====================*/
    #[msg(migrate)]
    pub fn migrate(&self, ctx: MigrateCtx) -> Result<Response, EcashContractError> {
        set_build_information!(ctx.deps.storage)?;
        cw2::ensure_from_older_version(ctx.deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

        Ok(Response::new())
    }
}
