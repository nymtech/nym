use crate::errors::ContractError;
use crate::messages::{ExecuteMsg, InitMsg, QueryMsg};
use crate::vesting::VestingPeriod;
use cosmwasm_std::{
    entry_point, to_binary, Addr, Coin, Deps, DepsMut, Env, MessageInfo, QueryResponse, Response,
    Timestamp,
};
use mixnet_contract::IdentityKey;

/// Instantiate the contract.
///
/// `deps` contains Storage, API and Querier
/// `env` contains block, message and contract info
/// `msg` is the contract initialization message, sort of like a constructor call.
#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    _msg: InitMsg,
) -> Result<Response, ContractError> {
    Ok(Response::default())
}

/// Handle an incoming message
#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::DelegateToMixnode { mix_identity } => {
            try_delegate_to_mixnode(mix_identity, env, deps)
        }
        ExecuteMsg::UndelegateFromMixnode { mix_identity } => {
            try_undelegate_from_mixnode(mix_identity, env, deps)
        }
        ExecuteMsg::CreatePeriodicVestingAccount {
            address,
            coins,
            start_time,
            periods,
        } => try_create_periodic_vesting_account(address, coins, start_time, periods, env, deps),
    }
}

fn try_delegate_to_mixnode(
    mix_identity: IdentityKey,
    env: Env,
    deps: DepsMut,
) -> Result<Response, ContractError> {
    unimplemented!()
}

fn try_undelegate_from_mixnode(
    mix_identity: IdentityKey,
    env: Env,
    deps: DepsMut,
) -> Result<Response, ContractError> {
    unimplemented!()
}

fn try_create_periodic_vesting_account(
    address: Addr,
    coins: Vec<Coin>,
    start_time: Option<u64>,
    periods: Option<Vec<VestingPeriod>>,
    env: Env,
    deps: DepsMut,
) -> Result<Response, ContractError> {
    let start_time = start_time.unwrap_or_else(|| env.block.time.seconds());
    let periods = periods.unwrap_or_else(|| vec![VestingPeriod::default(); 8]);
    let account = crate::vesting::PeriodicVestingAccount::new(address, coins, start_time, periods);
    unimplemented!()
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<QueryResponse, ContractError> {
    let query_res = match msg {
        QueryMsg::LockedCoins { block_time } => {
            to_binary(&try_get_locked_coins(block_time, env, deps)?)
        }
        QueryMsg::SpendableCoins { block_time } => {
            to_binary(&try_get_spendable_coins(block_time, env, deps)?)
        }
        QueryMsg::GetVestedCoins { block_time } => {
            to_binary(&try_get_vested_coins(block_time, env, deps)?)
        }
        QueryMsg::GetVestingCoins { block_time } => {
            to_binary(&try_get_vesting_coins(block_time, env, deps)?)
        }
        QueryMsg::GetStartTime {} => to_binary(&try_get_start_time(env, deps)?),
        QueryMsg::GetEndTime {} => to_binary(&try_get_end_time(env, deps)?),
        QueryMsg::GetOriginalVesting {} => to_binary(&try_get_original_vesting(env, deps)?),
        QueryMsg::GetDelegatedFree {} => to_binary(&try_get_delegated_free(env, deps)?),
        QueryMsg::GetDelegatedVesting {} => to_binary(&try_get_delegated_vesting(env, deps)?),
    };

    Ok(query_res?)
}

fn try_get_locked_coins(
    block_time: Option<Timestamp>,
    env: Env,
    deps: Deps,
) -> Result<Vec<Coin>, ContractError> {
    let block_time = block_time.unwrap_or_else(|| env.block.time);
    unimplemented!()
}

fn try_get_spendable_coins(
    block_time: Option<Timestamp>,
    env: Env,
    deps: Deps,
) -> Result<Vec<Coin>, ContractError> {
    let block_time = block_time.unwrap_or_else(|| env.block.time);
    unimplemented!()
}

fn try_get_vested_coins(
    block_time: Option<Timestamp>,
    env: Env,
    deps: Deps,
) -> Result<Vec<Coin>, ContractError> {
    let block_time = block_time.unwrap_or_else(|| env.block.time);
    unimplemented!()
}

fn try_get_vesting_coins(
    block_time: Option<Timestamp>,
    env: Env,
    deps: Deps,
) -> Result<Vec<Coin>, ContractError> {
    let block_time = block_time.unwrap_or_else(|| env.block.time);
    unimplemented!()
}

fn try_get_start_time(env: Env, deps: Deps) -> Result<Option<Timestamp>, ContractError> {
    unimplemented!()
}

fn try_get_end_time(env: Env, deps: Deps) -> Result<Option<Timestamp>, ContractError> {
    unimplemented!()
}

fn try_get_original_vesting(env: Env, deps: Deps) -> Result<Vec<Coin>, ContractError> {
    unimplemented!()
}

fn try_get_delegated_free(env: Env, deps: Deps) -> Result<Vec<Coin>, ContractError> {
    unimplemented!()
}

fn try_get_delegated_vesting(env: Env, deps: Deps) -> Result<Vec<Coin>, ContractError> {
    unimplemented!()
}
