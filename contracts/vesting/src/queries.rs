// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::storage;
use crate::storage::{account_from_address, BlockTimestampSecs, ACCOUNTS, DELEGATIONS, MIX_DENOM};
use crate::traits::VestingAccount;
use crate::vesting::StorableVestingAccountExt;
use contracts_common::{get_build_information, ContractBuildInformation};
use cosmwasm_std::{Coin, Deps, Env, Order, StdResult, Timestamp, Uint128};
use cw_storage_plus::Bound;
use mixnet_contract_common::NodeId;
use vesting_contract_common::{
    Account, AccountVestingCoins, AccountsResponse, AllDelegationsResponse, BaseVestingAccountInfo,
    DelegationTimesResponse, OriginalVestingResponse, Period, PledgeData, VestingCoinsResponse,
    VestingContractError, VestingDelegation,
};

/// Get current vesting period for a given [crate::vesting::Account].
pub fn try_get_current_vesting_period(
    address: &str,
    deps: Deps<'_>,
    env: Env,
) -> Result<Period, VestingContractError> {
    let account = account_from_address(address, deps.storage, deps.api)?;
    account.get_current_vesting_period(env.block.time)
}

/// Loads mixnode bond from vesting contract storage.
pub fn try_get_mixnode(
    address: &str,
    deps: Deps<'_>,
) -> Result<Option<PledgeData>, VestingContractError> {
    let account = account_from_address(address, deps.storage, deps.api)?;
    account.load_mixnode_pledge(deps.storage)
}

/// Loads gateway bond from vesting contract storage.
pub fn try_get_gateway(
    address: &str,
    deps: Deps<'_>,
) -> Result<Option<PledgeData>, VestingContractError> {
    let account = account_from_address(address, deps.storage, deps.api)?;
    account.load_gateway_pledge(deps.storage)
}

pub fn try_get_account(address: &str, deps: Deps<'_>) -> Result<Account, VestingContractError> {
    account_from_address(address, deps.storage, deps.api)
}

/// Gets build information of this contract.
pub fn get_contract_version() -> ContractBuildInformation {
    get_build_information!()
}

pub fn try_get_all_accounts(
    deps: Deps<'_>,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<AccountsResponse, VestingContractError> {
    let limit = limit.unwrap_or(150).min(250) as usize;

    let start = start_after
        .map(|raw| deps.api.addr_validate(&raw).map(Bound::exclusive))
        .transpose()?;

    let accounts = ACCOUNTS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| {
            res.map(|(_, account)| BaseVestingAccountInfo {
                account_id: account.storage_key(),
                owner: account.owner_address,
            })
        })
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = accounts.last().map(|acc| acc.owner.clone());

    Ok(AccountsResponse {
        accounts,
        start_next_after,
    })
}

pub fn try_get_all_accounts_vesting_coins(
    deps: Deps<'_>,
    env: Env,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<VestingCoinsResponse, VestingContractError> {
    let limit = limit.unwrap_or(150).min(250) as usize;

    let start = start_after
        .map(|raw| deps.api.addr_validate(&raw).map(Bound::exclusive))
        .transpose()?;

    let accounts = ACCOUNTS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| {
            res.map(|(_, account)| {
                account
                    .get_vesting_coins(None, &env, deps.storage)
                    .map(|still_vesting| AccountVestingCoins {
                        account_id: account.storage_key(),
                        owner: account.owner_address,
                        still_vesting,
                    })
            })
        })
        .collect::<StdResult<Result<Vec<_>, _>>>()??;

    let start_next_after = accounts.last().map(|acc| acc.owner.clone());

    Ok(VestingCoinsResponse {
        accounts,
        start_next_after,
    })
}

/// Gets currently locked coins, see [crate::traits::VestingAccount::locked_coins]
pub fn try_get_locked_coins(
    vesting_account_address: &str,
    block_time: Option<Timestamp>,
    env: Env,
    deps: Deps<'_>,
) -> Result<Coin, VestingContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    account.locked_coins(block_time, &env, deps.storage)
}

/// Returns currently locked coins, see [crate::traits::VestingAccount::spendable_coins]
pub fn try_get_spendable_coins(
    vesting_account_address: &str,
    block_time: Option<Timestamp>,
    env: Env,
    deps: Deps<'_>,
) -> Result<Coin, VestingContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    account.spendable_coins(block_time, &env, deps.storage)
}

/// Returns coins that have vested, see [crate::traits::VestingAccount::get_vested_coins]
pub fn try_get_vested_coins(
    vesting_account_address: &str,
    block_time: Option<Timestamp>,
    env: Env,
    deps: Deps<'_>,
) -> Result<Coin, VestingContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    account.get_vested_coins(block_time, &env, deps.storage)
}

/// Returns coins that are vesting, see [crate::traits::VestingAccount::get_vesting_coins]
pub fn try_get_vesting_coins(
    vesting_account_address: &str,
    block_time: Option<Timestamp>,
    env: Env,
    deps: Deps<'_>,
) -> Result<Coin, VestingContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    account.get_vesting_coins(block_time, &env, deps.storage)
}

/// See [crate::traits::VestingAccount::get_start_time]
pub fn try_get_start_time(
    vesting_account_address: &str,
    deps: Deps<'_>,
) -> Result<Timestamp, VestingContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    Ok(account.get_start_time())
}

/// See [crate::traits::VestingAccount::get_end_time]
pub fn try_get_end_time(
    vesting_account_address: &str,
    deps: Deps<'_>,
) -> Result<Timestamp, VestingContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    Ok(account.get_end_time())
}

/// See [crate::traits::VestingAccount::get_original_vesting]
pub fn try_get_original_vesting(
    vesting_account_address: &str,
    deps: Deps<'_>,
) -> Result<OriginalVestingResponse, VestingContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    account.get_original_vesting()
}

pub fn try_get_historical_vesting_staking_reward(
    vesting_account_address: &str,
    deps: Deps<'_>,
) -> Result<Coin, VestingContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    account.get_historical_vested_staking_rewards(deps.storage)
}

pub fn try_get_spendable_vested_coins(
    vesting_account_address: &str,
    deps: Deps<'_>,
    env: Env,
) -> Result<Coin, VestingContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    account.spendable_vested_coins(None, &env, deps.storage)
}

pub fn try_get_spendable_reward_coins(
    vesting_account_address: &str,
    deps: Deps<'_>,
    env: Env,
) -> Result<Coin, VestingContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    account.spendable_reward_coins(None, &env, deps.storage)
}

pub fn try_get_delegated_coins(
    vesting_account_address: &str,
    deps: Deps<'_>,
) -> Result<Coin, VestingContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    let denom = MIX_DENOM.load(deps.storage)?;
    let amount = account.total_delegations(deps.storage)?;
    Ok(Coin { denom, amount })
}

pub fn try_get_pledged_coins(
    vesting_account_address: &str,
    deps: Deps<'_>,
) -> Result<Coin, VestingContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    let denom = MIX_DENOM.load(deps.storage)?;
    let amount = account.total_pledged(deps.storage)?;
    Ok(Coin { denom, amount })
}

pub fn try_get_staked_coins(
    vesting_account_address: &str,
    deps: Deps<'_>,
) -> Result<Coin, VestingContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    let denom = MIX_DENOM.load(deps.storage)?;
    let amount = account.total_staked(deps.storage)?;
    Ok(Coin { denom, amount })
}

pub fn try_get_withdrawn_coins(
    vesting_account_address: &str,
    deps: Deps<'_>,
) -> Result<Coin, VestingContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    let denom = MIX_DENOM.load(deps.storage)?;
    let amount = account.load_withdrawn(deps.storage)?;
    Ok(Coin { denom, amount })
}

/// Returns timestamps at which delegations were made
pub fn try_get_delegation_times(
    deps: Deps<'_>,
    vesting_account_address: &str,
    mix_id: NodeId,
) -> Result<DelegationTimesResponse, VestingContractError> {
    let owner = deps.api.addr_validate(vesting_account_address)?;
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;

    let delegation_timestamps =
        storage::load_delegation_timestamps((account.storage_key(), mix_id), deps.storage)?;

    Ok(DelegationTimesResponse {
        owner,
        account_id: account.storage_key(),
        mix_id,
        delegation_timestamps,
    })
}

pub fn try_get_all_delegations(
    deps: Deps<'_>,
    start_after: Option<(u32, NodeId, BlockTimestampSecs)>,
    limit: Option<u32>,
) -> Result<AllDelegationsResponse, VestingContractError> {
    let limit = limit.unwrap_or(100).min(200) as usize;

    let start = start_after.map(Bound::exclusive);
    let delegations = DELEGATIONS
        .range(deps.storage, start, None, Order::Ascending)
        .map(|kv| {
            kv.map(
                |((account_id, mix_id, block_timestamp), amount)| VestingDelegation {
                    account_id,
                    mix_id,
                    block_timestamp,
                    amount,
                },
            )
        })
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = if delegations.len() < limit {
        None
    } else {
        delegations
            .last()
            .map(|delegation| delegation.storage_key())
    };

    Ok(AllDelegationsResponse {
        delegations,
        start_next_after,
    })
}

pub fn try_get_delegation(
    deps: Deps<'_>,
    vesting_account_address: &str,
    mix_id: NodeId,
    block_timestamp_secs: BlockTimestampSecs,
) -> Result<VestingDelegation, VestingContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;

    let storage_key = (account.storage_key(), mix_id, block_timestamp_secs);
    let delegation_amount = DELEGATIONS.load(deps.storage, storage_key)?;

    Ok(VestingDelegation {
        account_id: account.storage_key(),
        mix_id,
        block_timestamp: block_timestamp_secs,
        amount: delegation_amount,
    })
}

pub fn try_get_delegation_amount(
    deps: Deps<'_>,
    vesting_account_address: &str,
    mix_id: NodeId,
) -> Result<Coin, VestingContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;

    let amount = DELEGATIONS
        .prefix((account.storage_key(), mix_id))
        .range(deps.storage, None, None, Order::Ascending)
        .map(|kv_res| kv_res.map(|kv| kv.1))
        .try_fold(Uint128::zero(), |acc, x_res| x_res.map(|x| x + acc))?;
    let denom = MIX_DENOM.load(deps.storage)?;

    Ok(Coin { denom, amount })
}
