// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::{BLACKLIST_PROPOSAL_REPLY_ID, REDEMPTION_PROPOSAL_REPLY_ID};
use crate::contract::helpers::Invariants;
use crate::deposit::DepositStorage;
use crate::deposit_stats::DepositStatsStorage;
use crate::helpers::{
    BlacklistKey, Config, MultisigReply, BLACKLIST_PAGE_DEFAULT_LIMIT, BLACKLIST_PAGE_MAX_LIMIT,
    CONTRACT_NAME, CONTRACT_VERSION, DEPOSITS_PAGE_DEFAULT_LIMIT, DEPOSITS_PAGE_MAX_LIMIT,
};
use cosmwasm_std::{coin, Addr, BankMsg, Coin, DepsMut, Event, Order, Reply, Response, StdResult};
use cw4::Cw4Contract;
use cw_controllers::Admin;
use cw_storage_plus::{Bound, Item, Map};
use nym_contracts_common::set_build_information;
use nym_ecash_contract_common::blacklist::{
    BlacklistedAccount, BlacklistedAccountResponse, Blacklisting, PagedBlacklistedAccountResponse,
};
use nym_ecash_contract_common::counters::PoolCounters;
use nym_ecash_contract_common::deposit::{
    DepositData, DepositResponse, LatestDepositResponse, PagedDepositsResponse,
};
use nym_ecash_contract_common::deposit_statistics::DepositsStatistics;
use nym_ecash_contract_common::events::{
    DEPOSITED_FUNDS_EVENT_TYPE, DEPOSIT_ID, PROPOSAL_ID_ATTRIBUTE_NAME,
};
use nym_ecash_contract_common::msg::WhitelistedDeposit;
use nym_ecash_contract_common::reduced_deposit::{WhitelistedAccount, WhitelistedAccountsResponse};
use nym_ecash_contract_common::EcashContractError;
use nym_network_defaults::TICKETBOOK_SIZE;
use sylvia::ctx::{ExecCtx, InstantiateCtx, MigrateCtx, QueryCtx};
use sylvia::{contract, entry_points};

mod helpers;

mod queued_migrations;
#[cfg(test)]
mod test;

pub struct NymEcashContract {
    pub(crate) contract_admin: Admin,
    pub(crate) multisig: Admin,
    pub(crate) config: Item<Config>,
    pub(crate) pool_counters: Item<PoolCounters>,
    pub(crate) expected_invariants: Item<Invariants>,

    /// Information about the performed deposits
    pub(crate) deposit_stats: DepositStatsStorage,

    /// Map of approved addresses that are allowed to perform deposits using a reduced amount
    /// as specified by the saved value.
    pub(crate) reduced_deposits: Map<Addr, Coin>,

    pub(crate) blacklist: Map<BlacklistKey, Blacklisting>,

    pub(crate) deposits: DepositStorage,
}

#[entry_points]
#[contract]
#[sv::error(EcashContractError)]
impl NymEcashContract {
    #[allow(clippy::new_without_default)]
    pub const fn new() -> Self {
        Self {
            contract_admin: Admin::new("contract_admin"),
            multisig: Admin::new("multisig"),
            config: Item::new("config"),
            pool_counters: Item::new("pool_counters"),
            expected_invariants: Item::new("expected_invariants"),
            deposit_stats: DepositStatsStorage::new(),
            reduced_deposits: Map::new("reduced_deposits"),
            blacklist: Map::new("blacklist"),
            deposits: DepositStorage::new(),
        }
    }

    #[sv::msg(instantiate)]
    pub fn instantiate(
        &self,
        mut ctx: InstantiateCtx,
        holding_account: String,
        multisig_addr: String,
        group_addr: String,
        deposit_amount: Coin,
    ) -> Result<Response, EcashContractError> {
        // all counters, deposits, etc. always use and always will use the same denom
        let zero_coin = coin(0, &deposit_amount.denom);

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
                total_deposited: zero_coin.clone(),
                total_redeemed: zero_coin.clone(),
            },
        )?;

        self.config.save(
            ctx.deps.storage,
            &Config {
                group_addr,
                holding_account,
                deposit_amount,
            },
        )?;

        self.deposit_stats
            .deposits_with_default_price
            .save(ctx.deps.storage, &0)?;

        self.deposit_stats
            .deposits_with_default_price_amounts
            .save(ctx.deps.storage, &zero_coin)?;

        cw2::set_contract_version(ctx.deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
        set_build_information!(ctx.deps.storage)?;

        Ok(Response::default())
    }

    /*==================
    ======QUERIES=======
    ==================*/
    #[sv::msg(query)]
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

    #[sv::msg(query)]
    pub fn get_blacklisted_account(
        &self,
        ctx: QueryCtx,
        public_key: String,
    ) -> StdResult<BlacklistedAccountResponse> {
        let account = self.blacklist.may_load(ctx.deps.storage, public_key)?;
        Ok(BlacklistedAccountResponse::new(account))
    }

    #[sv::msg(query)]
    pub fn get_default_deposit_amount(&self, ctx: QueryCtx) -> StdResult<Coin> {
        let deposit_amount = self.config.load(ctx.deps.storage)?.deposit_amount;

        Ok(deposit_amount)
    }

    #[sv::msg(query)]
    pub fn get_reduced_deposit_amount(
        &self,
        ctx: QueryCtx,
        address: String,
    ) -> StdResult<Option<Coin>> {
        let address = ctx.deps.api.addr_validate(&address)?;
        let deposit_amount = self.reduced_deposits.may_load(ctx.deps.storage, address)?;

        Ok(deposit_amount)
    }

    #[sv::msg(query)]
    pub fn get_all_whitelisted_accounts(
        &self,
        ctx: QueryCtx,
    ) -> StdResult<WhitelistedAccountsResponse> {
        let whitelisted_accounts = self
            .reduced_deposits
            .range(ctx.deps.storage, None, None, Order::Ascending)
            .map(|item| {
                let (address, deposit) = item?;
                Ok(WhitelistedAccount { address, deposit })
            })
            .collect::<StdResult<Vec<_>>>()?;

        Ok(WhitelistedAccountsResponse {
            whitelisted_accounts,
        })
    }

    #[sv::msg(query)]
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

    #[sv::msg(query)]
    pub fn get_latest_deposit(
        &self,
        ctx: QueryCtx,
    ) -> Result<LatestDepositResponse, EcashContractError> {
        let Some(latest_id) = self.deposits.latest_deposit(ctx.deps.storage)? else {
            return Ok(LatestDepositResponse::default());
        };

        let maybe_deposit = self.deposits.try_load_by_id(ctx.deps.storage, latest_id)?;

        Ok(LatestDepositResponse {
            deposit: maybe_deposit.map(|deposit| DepositData {
                id: latest_id,
                deposit,
            }),
        })
    }

    #[sv::msg(query)]
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

    #[sv::msg(query)]
    pub fn get_deposits_statistics(
        &self,
        ctx: QueryCtx,
    ) -> Result<DepositsStatistics, EcashContractError> {
        let storage = ctx.deps.storage;
        let denom = &self.config.load(storage)?.deposit_amount.denom;

        let total_deposits_made = self.deposits.total_deposits_made(storage)?;
        let total_deposited = self.pool_counters.load(storage)?.total_deposited;

        let total_deposits_made_with_default_price = self
            .deposit_stats
            .get_total_deposits_made_with_default_price(storage)?;
        let total_deposited_with_default_price = self
            .deposit_stats
            .get_total_deposited_with_default_price(storage, denom)?;

        let custom = self
            .deposit_stats
            .get_custom_price_deposits(storage, denom)?;

        Ok(DepositsStatistics {
            total_deposits_made,
            total_deposited,
            total_deposits_made_with_default_price,
            total_deposited_with_default_price,
            total_deposits_made_with_custom_price: custom.total_count,
            total_deposited_with_custom_price: custom.total_amount,
            deposits_made_with_custom_price: custom.per_account_count,
            deposited_with_custom_price: custom.per_account_amount,
        })
    }

    /*=====================
    ======EXECUTIONS=======
    =====================*/

    #[sv::msg(exec)]
    pub fn deposit_ticket_book_funds(
        &self,
        ctx: ExecCtx,
        identity_key: String,
    ) -> Result<Response, EcashContractError> {
        // if the sender is in the reduced deposit map, require that amount
        // otherwise fallback to the default
        let (required_deposit, is_reduced) = if let Some(reduced) = self
            .reduced_deposits
            .may_load(ctx.deps.storage, ctx.info.sender.clone())?
        {
            (reduced, true)
        } else {
            (self.config.load(ctx.deps.storage)?.deposit_amount, false)
        };

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

        // update running stats

        // global total needed when migrating to the nym pool contract
        self.pool_counters
            .update(ctx.deps.storage, |mut counters| -> StdResult<_> {
                counters.total_deposited.amount += submitted;
                Ok(counters)
            })?;

        // per-price stats
        if is_reduced {
            self.deposit_stats.new_reduced_deposit(
                ctx.deps.storage,
                &ctx.info.sender,
                &required_deposit,
            )?;
        } else {
            self.deposit_stats
                .new_default_deposit(ctx.deps.storage, &required_deposit)?;
        }

        let deposit_id = self.deposits.save_deposit(ctx.deps.storage, identity_key)?;

        Ok(Response::new()
            .add_event(
                Event::new(DEPOSITED_FUNDS_EVENT_TYPE)
                    .add_attribute(DEPOSIT_ID, deposit_id.to_string()),
            )
            .set_data(deposit_id.to_be_bytes()))
    }

    #[sv::msg(exec)]
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

    #[sv::msg(exec)]
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
        let to_return = self.tickets_redemption_amount(ctx.deps.storage, &config, n)?;
        if to_return.amount.is_zero() {
            return Ok(Response::new());
        }

        self.pool_counters
            .update(ctx.deps.storage, |mut counters| -> StdResult<_> {
                counters.total_redeemed.amount += to_return.amount;
                Ok(counters)
            })?;

        Ok(Response::new().add_message(BankMsg::Send {
            to_address: config.holding_account.to_string(),
            amount: vec![to_return],
        }))
    }

    #[sv::msg(exec)]
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

    #[sv::msg(exec)]
    pub fn update_default_deposit_value(
        &self,
        ctx: ExecCtx,
        new_deposit: Coin,
    ) -> Result<Response, EcashContractError> {
        // only current admin can do that
        self.contract_admin
            .assert_admin(ctx.deps.as_ref(), &ctx.info.sender)?;

        let ticket_book_size = self.get_ticketbook_size(ctx.deps.storage)?;
        if new_deposit.amount < cosmwasm_std::Uint128::from(ticket_book_size) {
            return Err(EcashContractError::DepositBelowTicketBookSize {
                amount: new_deposit.amount,
                ticket_book_size,
            });
        }

        let deposit_str = new_deposit.to_string();
        self.config
            .update(ctx.deps.storage, |mut cfg| -> StdResult<_> {
                cfg.deposit_amount = new_deposit;
                Ok(cfg)
            })?;
        Ok(Response::new().add_attribute("updated_deposit", deposit_str))
    }

    pub(crate) fn add_reduced_deposit_address(
        &self,
        deps: DepsMut,
        address: Addr,
        deposit: &Coin,
    ) -> Result<(), EcashContractError> {
        // the reduced price must be strictly less than the default to avoid
        // accidentally misconfiguring an address to pay more than everyone else
        let default = self.config.load(deps.storage)?.deposit_amount;
        if deposit.denom != default.denom {
            return Err(EcashContractError::InvalidReducedDepositDenom {
                expected: default.denom,
                got: deposit.denom.clone(),
            });
        }
        if deposit.amount >= default.amount {
            return Err(EcashContractError::ReducedDepositNotReduced {
                reduced: deposit.amount,
                default: default.amount,
            });
        }

        let ticket_book_size = self.get_ticketbook_size(deps.storage)?;
        if deposit.amount < cosmwasm_std::Uint128::from(ticket_book_size) {
            return Err(EcashContractError::DepositBelowTicketBookSize {
                amount: deposit.amount,
                ticket_book_size,
            });
        }

        self.reduced_deposits.save(deps.storage, address, deposit)?;
        Ok(())
    }

    #[sv::msg(exec)]
    pub fn set_reduced_deposit_price(
        &self,
        ctx: ExecCtx,
        address: String,
        deposit: Coin,
    ) -> Result<Response, EcashContractError> {
        self.contract_admin
            .assert_admin(ctx.deps.as_ref(), &ctx.info.sender)?;

        let addr = ctx.deps.api.addr_validate(&address)?;
        self.add_reduced_deposit_address(ctx.deps, addr.clone(), &deposit)?;

        Ok(Response::new()
            .add_attribute("action", "set_reduced_deposit_price")
            .add_attribute("address", address)
            .add_attribute("deposit", deposit.to_string()))
    }

    /// Removes the reduced deposit price for a given address, reverting them to
    /// the default deposit amount. This is safe to call even if the address has
    /// already deposited at the reduced price — their next deposit will simply
    /// use the default price. Historical deposit statistics are not affected.
    #[sv::msg(exec)]
    pub fn remove_reduced_deposit_price(
        &self,
        ctx: ExecCtx,
        address: String,
    ) -> Result<Response, EcashContractError> {
        self.contract_admin
            .assert_admin(ctx.deps.as_ref(), &ctx.info.sender)?;

        let addr = ctx.deps.api.addr_validate(&address)?;

        if !self.reduced_deposits.has(ctx.deps.storage, addr.clone()) {
            return Err(EcashContractError::NoReducedDepositPrice { address });
        }

        self.reduced_deposits.remove(ctx.deps.storage, addr);

        Ok(Response::new()
            .add_attribute("action", "remove_reduced_deposit_price")
            .add_attribute("address", address))
    }

    #[sv::msg(exec)]
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

    #[sv::msg(exec)]
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
    #[sv::msg(reply)]
    #[allow(deprecated)]
    pub fn reply(
        &self,
        ctx: sylvia::types::ReplyCtx,
        msg: Reply,
    ) -> Result<Response, EcashContractError> {
        match msg.id {
            n if n == BLACKLIST_PROPOSAL_REPLY_ID => self.handle_blacklist_proposal_reply(ctx, msg),
            n if n == REDEMPTION_PROPOSAL_REPLY_ID => {
                self.handle_redemption_proposal_reply(ctx, msg)
            }
            other => Err(EcashContractError::InvalidReplyId { id: other }),
        }
    }

    #[allow(deprecated)]
    fn handle_blacklist_proposal_reply(
        &self,
        ctx: sylvia::types::ReplyCtx,
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

    #[allow(deprecated)]
    fn handle_redemption_proposal_reply(
        &self,
        _ctx: sylvia::types::ReplyCtx,
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
    #[sv::msg(migrate)]
    pub fn migrate(
        &self,
        ctx: MigrateCtx,
        initial_whitelist: Vec<WhitelistedDeposit>,
    ) -> Result<Response, EcashContractError> {
        set_build_information!(ctx.deps.storage)?;
        cw2::ensure_from_older_version(ctx.deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

        queued_migrations::add_tiered_pricing(ctx.deps, initial_whitelist)?;

        Ok(Response::new())
    }
}
