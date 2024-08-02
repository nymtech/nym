pub use crate::queries::*;
use crate::storage::{ADMIN, MIXNET_CONTRACT_ADDRESS, MIX_DENOM};
pub use crate::transactions::*;
use contracts_common::set_build_information;
use cosmwasm_std::{
    entry_point, to_binary, Addr, Coin, Deps, DepsMut, Env, MessageInfo, QueryResponse, Response,
    Uint128,
};
use vesting_contract_common::messages::{ExecuteMsg, InitMsg, MigrateMsg, QueryMsg};
use vesting_contract_common::{Account, VestingContractError};

// version info for migration info
const CONTRACT_NAME: &str = "crate:nym-vesting-contract";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const INITIAL_LOCKED_PLEDGE_CAP: Uint128 = Uint128::new(100_000_000_000);

// as of 01.02.23 the maximum number of delegations anyone has made towards particular mixnode is 12.
// thus restricting it to 25, which is more than double of that, doesn't seem too unreasonable.
// and is going to alleviate the issue of unbounded iteration in `remove_delegations_for_mix`
// that happens upon advancing the current epoch.
//
// However, do note it doesn't necessarily mean that upon reaching this limit it's impossible to perform
// further delegations (towards the same node)
// while this might not be the best workaround, you could remove the existing delegation
// (thus removing all separate entries from the storage, i.e. the `DELEGATIONS` map)
// and re-delegate it with the reclaimed amount (which will include all rewards)
// which will only result in a single key-value being stored.
pub const MAX_PER_MIX_DELEGATIONS: u32 = 25;

/// Instantiate the contract
#[entry_point]
pub fn instantiate(
    deps: DepsMut<'_>,
    _env: Env,
    info: MessageInfo,
    msg: InitMsg,
) -> Result<Response, VestingContractError> {
    // validate the received mixnet contract address
    let mixnet_contract_address = deps.api.addr_validate(&msg.mixnet_contract_address)?;

    // ADMIN is set to the address that instantiated the contract
    ADMIN.save(deps.storage, &info.sender)?;
    MIXNET_CONTRACT_ADDRESS.save(deps.storage, &mixnet_contract_address)?;
    MIX_DENOM.save(deps.storage, &msg.mix_denom)?;

    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    set_build_information!(deps.storage)?;

    Ok(Response::default())
}

#[entry_point]
pub fn migrate(
    deps: DepsMut<'_>,
    _env: Env,
    _msg: MigrateMsg,
) -> Result<Response, VestingContractError> {
    set_build_information!(deps.storage)?;
    cw2::ensure_from_older_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new())
}

#[entry_point]
pub fn execute(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, VestingContractError> {
    match msg {
        ExecuteMsg::TrackReward { amount, address } => {
            try_track_reward(deps, info, amount, &address)
        }
        ExecuteMsg::UpdateMixnetAddress { address } => {
            try_update_mixnet_address(address, info, deps)
        }
        ExecuteMsg::WithdrawVestedCoins { amount } => {
            try_withdraw_vested_coins(amount, env, info, deps)
        }
        ExecuteMsg::TrackUndelegation {
            owner,
            mix_id,
            amount,
        } => try_track_undelegation(&owner, mix_id, amount, info, deps),
        ExecuteMsg::TrackUnbondMixnode { owner, amount } => {
            try_track_unbond_mixnode(&owner, amount, info, deps)
        }
        ExecuteMsg::TrackDecreasePledge { owner, amount } => {
            try_track_decrease_mixnode_pledge(&owner, amount, info, deps)
        }
        ExecuteMsg::TrackUnbondGateway { owner, amount } => {
            try_track_unbond_gateway(&owner, amount, info, deps)
        }
        ExecuteMsg::TrackMigratedMixnode { owner } => try_track_migrate_mixnode(&owner, info, deps),
        ExecuteMsg::TrackMigratedDelegation { owner, mix_id } => {
            try_track_migrate_delegation(&owner, mix_id, info, deps)
        }
        _ => Err(VestingContractError::Other {
            message: "the contract has been disabled".to_string(),
        }),
    }
}

#[entry_point]
pub fn query(
    deps: Deps<'_>,
    env: Env,
    msg: QueryMsg,
) -> Result<QueryResponse, VestingContractError> {
    let query_res = match msg {
        QueryMsg::GetContractVersion {} => to_binary(&get_contract_version()),
        QueryMsg::GetCW2ContractVersion {} => to_binary(&cw2::get_contract_version(deps.storage)?),
        QueryMsg::GetAccountsPaged {
            start_next_after,
            limit,
        } => to_binary(&try_get_all_accounts(deps, start_next_after, limit)?),
        QueryMsg::GetAccountsVestingCoinsPaged {
            start_next_after,
            limit,
        } => to_binary(&try_get_all_accounts_vesting_coins(
            deps,
            env,
            start_next_after,
            limit,
        )?),
        QueryMsg::LockedCoins {
            vesting_account_address,
            block_time,
        } => to_binary(&try_get_locked_coins(
            &vesting_account_address,
            block_time,
            env,
            deps,
        )?),
        QueryMsg::SpendableCoins {
            vesting_account_address,
            block_time,
        } => to_binary(&try_get_spendable_coins(
            &vesting_account_address,
            block_time,
            env,
            deps,
        )?),
        QueryMsg::GetVestedCoins {
            vesting_account_address,
            block_time,
        } => to_binary(&try_get_vested_coins(
            &vesting_account_address,
            block_time,
            env,
            deps,
        )?),
        QueryMsg::GetVestingCoins {
            vesting_account_address,
            block_time,
        } => to_binary(&try_get_vesting_coins(
            &vesting_account_address,
            block_time,
            env,
            deps,
        )?),
        QueryMsg::GetStartTime {
            vesting_account_address,
        } => to_binary(&try_get_start_time(&vesting_account_address, deps)?),
        QueryMsg::GetEndTime {
            vesting_account_address,
        } => to_binary(&try_get_end_time(&vesting_account_address, deps)?),
        QueryMsg::GetOriginalVesting {
            vesting_account_address,
        } => to_binary(&try_get_original_vesting(&vesting_account_address, deps)?),
        QueryMsg::GetHistoricalVestingStakingReward {
            vesting_account_address,
        } => to_binary(&try_get_historical_vesting_staking_reward(
            &vesting_account_address,
            deps,
        )?),
        QueryMsg::GetSpendableVestedCoins {
            vesting_account_address,
        } => to_binary(&try_get_spendable_vested_coins(
            &vesting_account_address,
            deps,
            env,
        )?),
        QueryMsg::GetSpendableRewardCoins {
            vesting_account_address,
        } => to_binary(&try_get_spendable_reward_coins(
            &vesting_account_address,
            deps,
            env,
        )?),
        QueryMsg::GetDelegatedCoins {
            vesting_account_address,
        } => to_binary(&try_get_delegated_coins(&vesting_account_address, deps)?),
        QueryMsg::GetPledgedCoins {
            vesting_account_address,
        } => to_binary(&try_get_pledged_coins(&vesting_account_address, deps)?),
        QueryMsg::GetStakedCoins {
            vesting_account_address,
        } => to_binary(&try_get_staked_coins(&vesting_account_address, deps)?),
        QueryMsg::GetWithdrawnCoins {
            vesting_account_address,
        } => to_binary(&try_get_withdrawn_coins(&vesting_account_address, deps)?),
        QueryMsg::GetAccount { address } => to_binary(&try_get_account(&address, deps)?),
        QueryMsg::GetMixnode { address } => to_binary(&try_get_mixnode(&address, deps)?),
        QueryMsg::GetGateway { address } => to_binary(&try_get_gateway(&address, deps)?),
        QueryMsg::GetCurrentVestingPeriod { address } => {
            to_binary(&try_get_current_vesting_period(&address, deps, env)?)
        }
        QueryMsg::GetDelegation {
            address,
            mix_id,
            block_timestamp_secs,
        } => to_binary(&try_get_delegation(
            deps,
            &address,
            mix_id,
            block_timestamp_secs,
        )?),
        QueryMsg::GetTotalDelegationAmount { address, mix_id } => {
            to_binary(&try_get_delegation_amount(deps, &address, mix_id)?)
        }
        QueryMsg::GetDelegationTimes { address, mix_id } => {
            to_binary(&try_get_delegation_times(deps, &address, mix_id)?)
        }
        QueryMsg::GetAllDelegations { start_after, limit } => {
            to_binary(&try_get_all_delegations(deps, start_after, limit)?)
        }
    };

    Ok(query_res?)
}

pub(crate) fn validate_funds(
    funds: &[Coin],
    mix_denom: String,
) -> Result<Coin, VestingContractError> {
    if funds.is_empty() || funds[0].amount.is_zero() {
        return Err(VestingContractError::EmptyFunds);
    }

    if funds.len() > 1 {
        return Err(VestingContractError::MultipleDenoms);
    }

    if funds[0].denom != mix_denom {
        return Err(VestingContractError::WrongDenom(
            funds[0].denom.clone(),
            mix_denom,
        ));
    }

    Ok(funds[0].clone())
}

pub(crate) fn ensure_staking_permission(
    addr: &Addr,
    account: &Account,
) -> Result<(), VestingContractError> {
    if let Some(staking_address) = account.staking_address() {
        if staking_address == addr {
            return Ok(());
        }
    }
    Err(VestingContractError::InvalidStakingAccount {
        address: addr.clone(),
        for_account: account.owner_address(),
    })
}
