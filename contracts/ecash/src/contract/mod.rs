// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::{BLACKLIST_PROPOSAL_REPLY_ID, REDEMPTION_PROPOSAL_REPLY_ID};
use crate::contract::helpers::Invariants;
use crate::deposit::DepositStorage;
use crate::helpers::{
    BlacklistKey, Config, MultisigReply, BLACKLIST_PAGE_DEFAULT_LIMIT, BLACKLIST_PAGE_MAX_LIMIT,
    CONTRACT_NAME, CONTRACT_VERSION, DEPOSITS_PAGE_DEFAULT_LIMIT, DEPOSITS_PAGE_MAX_LIMIT,
};
use cosmwasm_std::{
    coin, BankMsg, Coin, Decimal, Event, Order, Reply, Response, StdResult, Uint128,
};
use cw4::Cw4Contract;
use cw_controllers::Admin;
use cw_storage_plus::{Bound, Item, Map};
use nym_contracts_common::set_build_information;
use nym_ecash_contract_common::blacklist::{
    BlacklistedAccount, BlacklistedAccountResponse, Blacklisting, PagedBlacklistedAccountResponse,
};
use nym_ecash_contract_common::counters::PoolCounters;
use nym_ecash_contract_common::deposit::{DepositData, DepositResponse, PagedDepositsResponse};
use nym_ecash_contract_common::events::{
    DEPOSITED_FUNDS_EVENT_TYPE, DEPOSIT_ID, PROPOSAL_ID_ATTRIBUTE_NAME,
};
use nym_ecash_contract_common::EcashContractError;
use nym_network_defaults::TICKETBOOK_SIZE;
use sylvia::types::{ExecCtx, InstantiateCtx, MigrateCtx, QueryCtx, ReplyCtx};
use sylvia::{contract, entry_points};

mod helpers;

#[cfg(test)]
mod test;

pub struct NymEcashContract<'a> {
    pub(crate) contract_admin: Admin<'a>,
    pub(crate) multisig: Admin<'a>,
    pub(crate) config: Item<'a, Config>,
    pub(crate) pool_counters: Item<'a, PoolCounters>,
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
            contract_admin: Admin::new("contract_admin"),
            multisig: Admin::new("multisig"),
            config: Item::new("config"),
            pool_counters: Item::new("pool_counters"),
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
        deposit_amount: Coin,
    ) -> Result<Response, EcashContractError> {
        let multisig_addr = ctx.deps.api.addr_validate(&multisig_addr)?;
        let holding_account = ctx.deps.api.addr_validate(&holding_account)?;
        let group_addr = Cw4Contract(ctx.deps.api.addr_validate(&group_addr).map_err(|_| {
            EcashContractError::InvalidGroup {
                addr: group_addr.clone(),
            }
        })?);

        // by default the sender becomes the admin
        self.contract_admin
            .set(ctx.deps.branch(), Some(ctx.info.sender))?;
        self.multisig
            .set(ctx.deps.branch(), Some(multisig_addr.clone()))?;

        self.expected_invariants.save(
            ctx.deps.storage,
            &Invariants {
                ticket_book_size: TICKETBOOK_SIZE,
            },
        )?;

        self.pool_counters.save(
            ctx.deps.storage,
            &PoolCounters {
                total_deposited: coin(0, &deposit_amount.denom),
                total_redeemed: coin(0, &deposit_amount.denom),
            },
        )?;

        self.config.save(
            ctx.deps.storage,
            &Config {
                group_addr,
                holding_account,
                redemption_gateway_share: Decimal::percent(5),
                deposit_amount,
            },
        )?;

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
    pub fn get_required_deposit_amount(&self, ctx: QueryCtx) -> StdResult<Coin> {
        let deposit_amount = self.config.load(ctx.deps.storage)?.deposit_amount;

        Ok(deposit_amount)
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
        let required_deposit = self.config.load(ctx.deps.storage)?.deposit_amount;

        let submitted = cw_utils::must_pay(&ctx.info, &required_deposit.denom)?;

        if submitted != required_deposit.amount {
            let mut funds = ctx.info.funds;
            return Err(EcashContractError::WrongAmount {
                // SAFETY: the call to `must_pay` ensured a single coin has been sent
                #[allow(clippy::unwrap_used)]
                received: funds.pop().unwrap(),
                amount: required_deposit,
            });
        }

        self.pool_counters
            .update(ctx.deps.storage, |mut counters| -> StdResult<_> {
                counters.total_deposited.amount += submitted;
                Ok(counters)
            })?;

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
        // preserve the gateway argument so that upon scraping the chain and going through transactions,
        // one could see which gateway attempted to redeem it.
        // in the long run it will be needed to determine work factor.
        let _ = gw;

        // only a mutlisig proposal can do that
        self.multisig
            .assert_admin(ctx.deps.as_ref(), &ctx.info.sender)?;

        let config = self.config.load(ctx.deps.storage)?;

        // TODO: we need unit tests for this
        let deposit_amount = config.deposit_amount.amount;
        let ticketbook_size = Uint128::new(self.get_ticketbook_size(ctx.deps.storage)? as u128);
        let tickets = Uint128::new(n as u128);

        // how many tickets from a ticketbook you redeemed
        let book_ratio = Decimal::from_ratio(tickets, ticketbook_size);

        // return = ticketbook_price * (tickets / ticketbook_size)
        let return_amount = book_ratio * deposit_amount;

        self.pool_counters
            .update(ctx.deps.storage, |mut counters| -> StdResult<_> {
                counters.total_redeemed.amount += return_amount;
                Ok(counters)
            })?;

        Ok(Response::new().add_message(BankMsg::Send {
            to_address: config.holding_account.to_string(),
            amount: vec![Coin {
                denom: config.deposit_amount.denom,
                amount: return_amount,
            }],
        }))
    }

    #[msg(exec)]
    pub fn update_admin(
        &self,
        ctx: ExecCtx,
        admin: String,
    ) -> Result<Response, EcashContractError> {
        let new_admin = ctx.deps.api.addr_validate(&admin)?;

        // note: the below performs validation to ensure the sender IS the current admin
        Ok(self
            .contract_admin
            .execute_update_admin(ctx.deps, ctx.info, Some(new_admin))?)
    }

    #[msg(exec)]
    pub fn update_deposit_value(
        &self,
        ctx: ExecCtx,
        new_deposit: Coin,
    ) -> Result<Response, EcashContractError> {
        // only current admin can do that
        self.contract_admin
            .assert_admin(ctx.deps.as_ref(), &ctx.info.sender)?;

        let deposit_str = new_deposit.to_string();
        self.config
            .update(ctx.deps.storage, |mut cfg| -> StdResult<_> {
                cfg.deposit_amount = new_deposit;
                Ok(cfg)
            })?;
        Ok(Response::new().add_attribute("updated_deposit", deposit_str))
    }

    #[msg(exec)]
    pub fn propose_to_blacklist(
        &self,
        ctx: ExecCtx,
        public_key: String,
    ) -> Result<Response, EcashContractError> {
        let _ = ctx;
        let _ = public_key;
        Err(EcashContractError::UnimplementedBlacklisting)
        // let cfg = self.config.load(ctx.deps.storage)?;
        // cfg.group_addr
        //     .is_voting_member(&ctx.deps.querier, &ctx.info.sender, ctx.env.block.height)?
        //     .ok_or(EcashContractError::Unauthorized)?;
        //
        // if let Some(blacklisted) = self
        //     .blacklist
        //     .may_load(ctx.deps.storage, public_key.clone())?
        // {
        //     // return existing proposal id
        //     Ok(Response::new().add_attribute(
        //         PROPOSAL_ID_ATTRIBUTE_NAME,
        //         blacklisted.proposal_id.to_string(),
        //     ))
        // } else {
        //     let msg = self.create_blacklist_proposal(ctx, public_key)?;
        //     Ok(Response::new().add_submessage(msg))
        // }
    }

    #[msg(exec)]
    pub fn add_to_blacklist(
        &self,
        ctx: ExecCtx,
        public_key: String,
    ) -> Result<Response, EcashContractError> {
        let _ = ctx;
        let _ = public_key;
        Err(EcashContractError::UnimplementedBlacklisting)
        // //Only by multisig contract, actually add public key to blacklist
        // self.multisig
        //     .assert_admin(ctx.deps.as_ref(), &ctx.info.sender)?;
        //
        // let mut blacklisting = self.blacklist.load(ctx.deps.storage, public_key.clone())?;
        // blacklisting.finalized_at_height = Some(ctx.env.block.height);
        // self.blacklist
        //     .save(ctx.deps.storage, public_key.clone(), &blacklisting)?;
        //
        // Ok(Response::new())
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
