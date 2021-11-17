use crate::errors::ContractError;
use crate::messages::{ExecuteMsg, InitMsg, QueryMsg};
use crate::storage::{get_account, get_account_balance, set_account_balance};
use crate::vesting::{DelegationAccount, PeriodicVestingAccount, VestingAccount, populate_vesting_periods};
use config::defaults::DENOM;
use cosmwasm_std::{
    entry_point, to_binary, Coin, Deps, DepsMut, Env, MessageInfo, QueryResponse, Response, BankMsg,
    Timestamp, attr, Uint128
};
use mixnet_contract::IdentityKey;

pub const NUM_VESTING_PERIODS: u64 = 8;
pub const VESTING_PERIOD: u64 = 3 * 30 * 86400;
pub const ADMIN_ADDRESS: &str = "admin";

/// Instantiate the contract.
///
/// `deps` contains Storage, API and Querier
/// `env` contains block, message and contract info
/// `msg` is the contract initialization message, sort of like a constructor call.
#[entry_point]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
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
        ExecuteMsg::DelegateToMixnode {
            mix_identity,
            delegate_addr,
            amount,
        } => try_delegate_to_mixnode(mix_identity, delegate_addr, amount, info, env, deps),
        ExecuteMsg::UndelegateFromMixnode {
            mix_identity,
            delegate_addr,
        } => try_undelegate_from_mixnode(mix_identity, delegate_addr, info, deps),
        ExecuteMsg::CreatePeriodicVestingAccount {
            address,
            coin,
            start_time,
        } => try_create_periodic_vesting_account(address, coin, start_time, info, env, deps),
        ExecuteMsg::WithdrawVestedCoins { amount } => try_withdraw_vested_coins(amount, env, info, deps),
    }
}

fn try_withdraw_vested_coins(amount: Coin, env: Env, info: MessageInfo, deps: DepsMut) -> Result<Response, ContractError> {
    let address = info.sender;
    if let Some(account) = get_account(deps.storage, &address) {
        let spendable_coins = account.spendable_coins(None, &env, deps.storage);
        if amount.amount < spendable_coins.amount {
            if let Some(balance)= get_account_balance(deps.storage, &address) {
                let new_balance = balance.u128().saturating_sub(amount.amount.u128());
                set_account_balance(deps.storage, &address, Uint128(new_balance))?;
            } else {
                return Err(ContractError::NoBalanceForAddress(address.as_str().to_string()));
            }

            let messages = vec![BankMsg::Send {
                to_address: address.as_str().to_string(),
                amount: vec!(amount.clone()),
            }.into()];
            
            let attributes = vec![attr("action", "whitdraw")];

            return Ok(Response {
                submessages: Vec::new(),
                messages,
                attributes,
                data: None,
            })
        } else {
            return Err(ContractError::InsufficientSpendable(address.as_str().to_string(), spendable_coins.amount.u128()));
        }
        
    } else {
        return Err(ContractError::NoAccountForAddress(address.as_str().to_string()));
    }
    Ok(Response::default())
    
}

fn try_delegate_to_mixnode(
    mix_identity: IdentityKey,
    delegate_addr: String,
    amount: Coin,
    info: MessageInfo,
    env: Env,
    deps: DepsMut,
) -> Result<Response, ContractError> {
    if info.sender != ADMIN_ADDRESS {
        return Err(ContractError::NotAdmin(info.sender.as_str().to_string()));
    }
    let address = deps.api.addr_validate(&delegate_addr)?;
    if let Some(account) = get_account(deps.storage, &address) {
        account.try_delegate_to_mixnode(mix_identity, amount, &env, deps.storage, Some(deps.querier))?;
    }
    Ok(Response::default())
}

fn try_undelegate_from_mixnode(
    mix_identity: IdentityKey,
    delegate_addr: String,
    info: MessageInfo,
    deps: DepsMut,
) -> Result<Response, ContractError> {
    if info.sender != ADMIN_ADDRESS {
        return Err(ContractError::NotAdmin(info.sender.as_str().to_string()));
    }
    let address = deps.api.addr_validate(&delegate_addr)?;
    if let Some(account) = get_account(deps.storage, &address) {
        account.try_undelegate_from_mixnode(mix_identity, deps.storage, Some(deps.querier))?;
    }
    Ok(Response::default())
}

fn try_create_periodic_vesting_account(
    address: String,
    coin: Coin,
    start_time: Option<u64>,
    info: MessageInfo,
    env: Env,
    deps: DepsMut,
) -> Result<Response, ContractError> {
    if info.sender != ADMIN_ADDRESS {
        return Err(ContractError::NotAdmin(info.sender.as_str().to_string()));
    }
    let address = deps.api.addr_validate(&address)?;
    let start_time = start_time.unwrap_or_else(|| env.block.time.seconds());
    let periods = populate_vesting_periods(start_time, NUM_VESTING_PERIODS);
    PeriodicVestingAccount::new(
        address,
        coin,
        Timestamp::from_seconds(start_time),
        periods,
        deps.storage,
    )?;
    Ok(Response::default())
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<QueryResponse, ContractError> {
    let query_res = match msg {
        QueryMsg::LockedCoins {
            vesting_account_address,
            block_time,
        } => to_binary(&try_get_locked_coins(
            vesting_account_address,
            block_time,
            env,
            deps,
        )?),
        QueryMsg::SpendableCoins {
            vesting_account_address,
            block_time,
        } => to_binary(&try_get_spendable_coins(
            vesting_account_address,
            block_time,
            env,
            deps,
        )?),
        QueryMsg::GetVestedCoins {
            vesting_account_address,
            block_time,
        } => to_binary(&try_get_vested_coins(
            vesting_account_address,
            block_time,
            env,
            deps,
        )?),
        QueryMsg::GetVestingCoins {
            vesting_account_address,
            block_time,
        } => to_binary(&try_get_vesting_coins(
            vesting_account_address,
            block_time,
            env,
            deps,
        )?),
        QueryMsg::GetStartTime {
            vesting_account_address,
        } => to_binary(&try_get_start_time(vesting_account_address, env, deps)?),
        QueryMsg::GetEndTime {
            vesting_account_address,
        } => to_binary(&try_get_end_time(vesting_account_address, env, deps)?),
        QueryMsg::GetOriginalVesting {
            vesting_account_address,
        } => to_binary(&try_get_original_vesting(
            vesting_account_address,
            env,
            deps,
        )?),
        QueryMsg::GetDelegatedFree {
            block_time,
            vesting_account_address,
        } => to_binary(&try_get_delegated_free(block_time, vesting_account_address, env, deps)?),
        QueryMsg::GetDelegatedVesting {
            block_time,
            vesting_account_address,
        } => to_binary(&try_get_delegated_vesting(
            block_time,
            vesting_account_address,
            env,
            deps,
        )?),
    };

    Ok(query_res?)
}

fn try_get_locked_coins(
    vesting_account_address: String,
    block_time: Option<Timestamp>,
    env: Env,
    deps: Deps,
) -> Result<Coin, ContractError> {
    let address = deps.api.addr_validate(&vesting_account_address)?;
    if let Some(account) = get_account(deps.storage, &address) {
        Ok(account.locked_coins(block_time, &env, deps.storage))
    } else {
        Err(ContractError::NoAccountForAddress(vesting_account_address))
    }
}

fn try_get_spendable_coins(
    vesting_account_address: String,
    block_time: Option<Timestamp>,
    env: Env,
    deps: Deps,
) -> Result<Coin, ContractError> {
    let address = deps.api.addr_validate(&vesting_account_address)?;
    if let Some(account) = get_account(deps.storage, &address) {
        Ok(account.spendable_coins(block_time, &env, deps.storage))
    } else {
        Err(ContractError::NoAccountForAddress(vesting_account_address))
    }
}

fn try_get_vested_coins(
    vesting_account_address: String,
    block_time: Option<Timestamp>,
    env: Env,
    deps: Deps,
) -> Result<Coin, ContractError> {
    let address = deps.api.addr_validate(&vesting_account_address)?;
    if let Some(account) = get_account(deps.storage, &address) {
        Ok(account.get_vested_coins(block_time, &env))
    } else {
        Err(ContractError::NoAccountForAddress(vesting_account_address))
    }
}

fn try_get_vesting_coins(
    vesting_account_address: String,
    block_time: Option<Timestamp>,
    env: Env,
    deps: Deps,
) -> Result<Coin, ContractError> {
    let address = deps.api.addr_validate(&vesting_account_address)?;
    if let Some(account) = get_account(deps.storage, &address) {
        Ok(account.get_vesting_coins(block_time, &env))
    } else {
        Err(ContractError::NoAccountForAddress(vesting_account_address))
    }
}

fn try_get_start_time(
    vesting_account_address: String,
    env: Env,
    deps: Deps,
) -> Result<Timestamp, ContractError> {
    let address = deps.api.addr_validate(&vesting_account_address)?;
    if let Some(account) = get_account(deps.storage, &address) {
        Ok(account.get_start_time())
    } else {
        Err(ContractError::NoAccountForAddress(vesting_account_address))
    }
}

fn try_get_end_time(
    vesting_account_address: String,
    env: Env,
    deps: Deps,
) -> Result<Timestamp, ContractError> {
    let address = deps.api.addr_validate(&vesting_account_address)?;
    if let Some(account) = get_account(deps.storage, &address) {
        Ok(account.get_end_time())
    } else {
        Err(ContractError::NoAccountForAddress(vesting_account_address))
    }
}

fn try_get_original_vesting(
    vesting_account_address: String,
    env: Env,
    deps: Deps,
) -> Result<Coin, ContractError> {
    let address = deps.api.addr_validate(&vesting_account_address)?;
    if let Some(account) = get_account(deps.storage, &address) {
        Ok(account.get_original_vesting())
    } else {
        Err(ContractError::NoAccountForAddress(vesting_account_address))
    }
}

fn try_get_delegated_free(
    block_time: Option<Timestamp>,
    vesting_account_address: String,
    env: Env,
    deps: Deps,
) -> Result<Coin, ContractError> {
    let address = deps.api.addr_validate(&vesting_account_address)?;
    if let Some(account) = get_account(deps.storage, &address) {
        Ok(account.get_delegated_free(block_time, &env, deps.storage))
    } else {
        Err(ContractError::NoAccountForAddress(vesting_account_address))
    }
}

fn try_get_delegated_vesting(
    block_time: Option<Timestamp>,
    vesting_account_address: String,
    env: Env,
    deps: Deps,
) -> Result<Coin, ContractError> {
    let address = deps.api.addr_validate(&vesting_account_address)?;
    if let Some(account) = get_account(deps.storage, &address) {
        Ok(account.get_delegated_vesting(block_time, &env, deps.storage))
    } else {
        Err(ContractError::NoAccountForAddress(vesting_account_address))
    }
}
