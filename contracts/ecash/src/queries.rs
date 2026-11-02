// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Read-only query handlers.

use crate::helpers::{
    BLACKLIST_PAGE_DEFAULT_LIMIT, BLACKLIST_PAGE_MAX_LIMIT, DEPOSITS_PAGE_DEFAULT_LIMIT,
    DEPOSITS_PAGE_MAX_LIMIT,
};
use crate::storage::ECASH_CONTRACT_STORAGE;
use cosmwasm_std::{Coin, Deps, Order, StdResult};
use cw_storage_plus::Bound;
use nym_ecash_contract_common::blacklist::{
    BlacklistedAccount, BlacklistedAccountResponse, PagedBlacklistedAccountResponse,
};
use nym_ecash_contract_common::deposit::{
    DepositData, DepositResponse, LatestDepositResponse, PagedDepositsResponse,
};
use nym_ecash_contract_common::deposit_statistics::DepositsStatistics;
use nym_ecash_contract_common::reduced_deposit::{WhitelistedAccount, WhitelistedAccountsResponse};
use nym_ecash_contract_common::EcashContractError;

/// Paginated listing of blacklist entries. Always empty today - see the
/// stubbed blacklist requirement in the spec.
pub(crate) fn query_blacklist_paged(
    deps: Deps,
    limit: Option<u32>,
    start_after: Option<String>,
) -> StdResult<PagedBlacklistedAccountResponse> {
    let limit = limit
        .unwrap_or(BLACKLIST_PAGE_DEFAULT_LIMIT)
        .min(BLACKLIST_PAGE_MAX_LIMIT) as usize;

    let start = start_after.as_deref().map(Bound::exclusive);

    let nodes = ECASH_CONTRACT_STORAGE
        .blacklist
        .range(deps.storage, start, None, Order::Ascending)
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

/// Single-key blacklist lookup. Always returns `None` on a freshly deployed
/// contract because the blacklist execute surface is stubbed.
pub(crate) fn query_blacklisted_account(
    deps: Deps,
    public_key: String,
) -> StdResult<BlacklistedAccountResponse> {
    let account = ECASH_CONTRACT_STORAGE
        .blacklist
        .may_load(deps.storage, public_key)?;
    Ok(BlacklistedAccountResponse::new(account))
}

/// Default per-deposit price (`Config::deposit_amount`).
pub(crate) fn query_default_deposit_amount(deps: Deps) -> StdResult<Coin> {
    let deposit_amount = ECASH_CONTRACT_STORAGE
        .config
        .load(deps.storage)?
        .deposit_amount;

    Ok(deposit_amount)
}

/// Per-address reduced deposit price override. `None` for any
/// non-whitelisted sender.
pub(crate) fn query_reduced_deposit_amount(deps: Deps, address: String) -> StdResult<Option<Coin>> {
    let address = deps.api.addr_validate(&address)?;
    let deposit_amount = ECASH_CONTRACT_STORAGE
        .reduced_deposits
        .may_load(deps.storage, address)?;

    Ok(deposit_amount)
}

/// Enumerate every reduced-deposit whitelist entry. Unpaginated - the
/// whitelist is expected to stay small.
pub(crate) fn query_all_whitelisted_accounts(deps: Deps) -> StdResult<WhitelistedAccountsResponse> {
    let whitelisted_accounts = ECASH_CONTRACT_STORAGE
        .reduced_deposits
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| {
            let (address, deposit) = item?;
            Ok(WhitelistedAccount { address, deposit })
        })
        .collect::<StdResult<Vec<_>>>()?;

    Ok(WhitelistedAccountsResponse {
        whitelisted_accounts,
    })
}

/// Look up a deposit by id. Returns `{ id, deposit: None }` when the id
/// has not yet been assigned.
pub(crate) fn query_deposit(
    deps: Deps,
    deposit_id: u32,
) -> Result<DepositResponse, EcashContractError> {
    Ok(DepositResponse {
        id: deposit_id,
        deposit: ECASH_CONTRACT_STORAGE
            .deposits
            .try_load_by_id(deps.storage, deposit_id)?,
    })
}

/// Most recently assigned deposit, or `{ deposit: None }` on a fresh
/// contract. See `DepositStorage::latest_deposit`.
pub(crate) fn query_latest_deposit(
    deps: Deps,
) -> Result<LatestDepositResponse, EcashContractError> {
    let Some(latest_id) = ECASH_CONTRACT_STORAGE
        .deposits
        .latest_deposit(deps.storage)?
    else {
        return Ok(LatestDepositResponse::default());
    };

    let maybe_deposit = ECASH_CONTRACT_STORAGE
        .deposits
        .try_load_by_id(deps.storage, latest_id)?;

    Ok(LatestDepositResponse {
        deposit: maybe_deposit.map(|deposit| DepositData {
            id: latest_id,
            deposit,
        }),
    })
}

/// Paginated listing of deposits in ascending id order. Defaults to a
/// limit of 50, clamped at 100.
pub(crate) fn query_deposits_paged(
    deps: Deps,
    limit: Option<u32>,
    start_after: Option<u32>,
) -> StdResult<PagedDepositsResponse> {
    let limit = limit
        .unwrap_or(DEPOSITS_PAGE_DEFAULT_LIMIT)
        .min(DEPOSITS_PAGE_MAX_LIMIT) as usize;

    let start = start_after.map(Bound::exclusive);

    let deposits = ECASH_CONTRACT_STORAGE
        .deposits
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| res.map(Into::into))
        .collect::<StdResult<Vec<DepositData>>>()?;

    let start_next_after = deposits.last().map(|deposit| deposit.id);

    Ok(PagedDepositsResponse {
        deposits,
        start_next_after,
    })
}

/// Aggregate statistics - global totals plus per-account custom-price
/// breakdowns. Single read pass over `PoolCounters` and the stats storage.
pub(crate) fn query_deposits_statistics(
    deps: Deps,
) -> Result<DepositsStatistics, EcashContractError> {
    let storage = deps.storage;
    let denom = &ECASH_CONTRACT_STORAGE
        .config
        .load(storage)?
        .deposit_amount
        .denom;

    let total_deposits_made = ECASH_CONTRACT_STORAGE
        .deposits
        .total_deposits_made(storage)?;
    let total_deposited = ECASH_CONTRACT_STORAGE
        .pool_counters
        .load(storage)?
        .total_deposited;

    let total_deposits_made_with_default_price = ECASH_CONTRACT_STORAGE
        .deposit_stats
        .get_total_deposits_made_with_default_price(storage)?;
    let total_deposited_with_default_price = ECASH_CONTRACT_STORAGE
        .deposit_stats
        .get_total_deposited_with_default_price(storage, denom)?;

    let custom = ECASH_CONTRACT_STORAGE
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
