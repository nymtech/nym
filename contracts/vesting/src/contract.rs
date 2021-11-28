use crate::errors::ContractError;
use crate::messages::{ExecuteMsg, InitMsg, QueryMsg};
use crate::storage::{get_account, get_account_balance, set_account_balance};
use crate::vesting::{
    populate_vesting_periods, BondingAccount, DelegationAccount, PeriodicVestingAccount,
    VestingAccount,
};
use config::defaults::{DEFAULT_MIXNET_CONTRACT_ADDRESS, DENOM};
use cosmwasm_std::{
    entry_point, to_binary, Addr, BankMsg, Coin, Deps, DepsMut, Env, MessageInfo, QueryResponse,
    Response, Timestamp, Uint128,
};
use mixnet_contract::{IdentityKey, MixNode};

pub const NUM_VESTING_PERIODS: usize = 8;
pub const VESTING_PERIOD: u64 = 3 * 30 * 86400;
pub const ADMIN_ADDRESS: &str = "admin";

// TODO: Validate vesting/vested withdraw mathematics
// TODO: Try and get to the bottom of multilevel bucket vs vector and performance of the whole storage thing

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
        ExecuteMsg::DelegateToMixnode { mix_identity } => {
            try_delegate_to_mixnode(mix_identity, info, env, deps)
        }
        ExecuteMsg::UndelegateFromMixnode { mix_identity } => {
            try_undelegate_from_mixnode(mix_identity, info, deps)
        }
        ExecuteMsg::CreatePeriodicVestingAccount {
            address,
            start_time,
        } => try_create_periodic_vesting_account(address, start_time, info, env, deps),
        ExecuteMsg::WithdrawVestedCoins { amount } => {
            try_withdraw_vested_coins(amount, env, info, deps)
        }
        ExecuteMsg::TrackUndelegation {
            owner,
            mix_identity,
            amount,
        } => try_track_undelegation(owner, mix_identity, amount, info, deps),
        ExecuteMsg::BondMixnode { mix_node } => try_bond_mixnode(mix_node, info, env, deps),
        ExecuteMsg::UnbondMixnode {} => try_unbond_mixnode(info, deps),
        ExecuteMsg::TrackUnbond { owner, amount } => try_track_unbond(owner, amount, deps),
    }
}

pub fn try_bond_mixnode(
    mix_node: MixNode,
    info: MessageInfo,
    env: Env,
    deps: DepsMut,
) -> Result<Response, ContractError> {
    let owner = deps.api.addr_validate(info.sender.as_str())?;
    let bond = validate_funds(&info.funds)?;
    if let Some(account) = get_account(deps.storage, &owner) {
        account.try_bond_mixnode(mix_node, bond, &env, deps.storage)
    } else {
        Err(ContractError::NoAccountForAddress(
            owner.as_str().to_string(),
        ))
    }
}

pub fn try_unbond_mixnode(info: MessageInfo, deps: DepsMut) -> Result<Response, ContractError> {
    let owner = deps.api.addr_validate(info.sender.as_str())?;
    if let Some(account) = get_account(deps.storage, &owner) {
        account.try_unbond_mixnode()
    } else {
        Err(ContractError::NoAccountForAddress(
            owner.as_str().to_string(),
        ))
    }
}

pub fn try_track_unbond(
    owner: Addr,
    amount: Coin,
    deps: DepsMut,
) -> Result<Response, ContractError> {
    let owner = deps.api.addr_validate(owner.as_str())?;
    if let Some(account) = get_account(deps.storage, &owner) {
        account.track_unbond(amount, deps.storage)?;
        Ok(Response::default())
    } else {
        Err(ContractError::NoAccountForAddress(
            owner.as_str().to_string(),
        ))
    }
}

pub fn try_withdraw_vested_coins(
    amount: Coin,
    env: Env,
    info: MessageInfo,
    deps: DepsMut,
) -> Result<Response, ContractError> {
    let address = info.sender;
    if let Some(account) = get_account(deps.storage, &address) {
        let spendable_coins = account.spendable_coins(None, &env, deps.storage)?;
        if amount.amount < spendable_coins.amount {
            if let Some(balance) = get_account_balance(deps.storage, &address) {
                let new_balance = balance.u128().saturating_sub(amount.amount.u128());
                set_account_balance(deps.storage, &address, Uint128::new(new_balance))?;
            } else {
                return Err(ContractError::NoBalanceForAddress(
                    address.as_str().to_string(),
                ));
            }

            let send_tokens = BankMsg::Send {
                to_address: address.as_str().to_string(),
                amount: vec![amount],
            };

            Ok(Response::new()
                .add_attribute("action", "whitdraw")
                .add_message(send_tokens))
        } else {
            Err(ContractError::InsufficientSpendable(
                address.as_str().to_string(),
                spendable_coins.amount.u128(),
            ))
        }
    } else {
        return Err(ContractError::NoAccountForAddress(
            address.as_str().to_string(),
        ));
    }
}

fn try_track_undelegation(
    address: Addr,
    mix_identity: IdentityKey,
    amount: Coin,
    info: MessageInfo,
    deps: DepsMut,
) -> Result<Response, ContractError> {
    if info.sender != DEFAULT_MIXNET_CONTRACT_ADDRESS {
        return Err(ContractError::NotMixnetContract(info.sender));
    }
    let adddress = deps.api.addr_validate(address.as_str())?;
    if let Some(account) = get_account(deps.storage, &adddress) {
        account.track_undelegation(mix_identity, amount, deps.storage)?;
        Ok(Response::default())
    } else {
        Err(ContractError::NoAccountForAddress(
            address.as_str().to_string(),
        ))
    }
}

fn try_delegate_to_mixnode(
    mix_identity: IdentityKey,
    info: MessageInfo,
    env: Env,
    deps: DepsMut,
) -> Result<Response, ContractError> {
    let delegate_addr = info.sender;
    let amount = validate_funds(&info.funds)?;
    let address = deps.api.addr_validate(delegate_addr.as_str())?;
    if let Some(account) = get_account(deps.storage, &address) {
        account.try_delegate_to_mixnode(mix_identity, amount, &env, deps.storage)
    } else {
        Err(ContractError::NoAccountForAddress(
            address.as_str().to_string(),
        ))
    }
}

fn try_undelegate_from_mixnode(
    mix_identity: IdentityKey,
    info: MessageInfo,
    deps: DepsMut,
) -> Result<Response, ContractError> {
    let delegate_addr = info.sender;
    let address = deps.api.addr_validate(delegate_addr.as_str())?;
    if let Some(account) = get_account(deps.storage, &address) {
        account.try_undelegate_from_mixnode(mix_identity)
    } else {
        Err(ContractError::NoAccountForAddress(
            address.as_str().to_string(),
        ))
    }
}

fn try_create_periodic_vesting_account(
    address: String,
    start_time: Option<u64>,
    info: MessageInfo,
    env: Env,
    deps: DepsMut,
) -> Result<Response, ContractError> {
    if info.sender != ADMIN_ADDRESS {
        return Err(ContractError::NotAdmin(info.sender.as_str().to_string()));
    }
    let coin = validate_funds(&info.funds)?;
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
        } => to_binary(&try_get_start_time(vesting_account_address, deps)?),
        QueryMsg::GetEndTime {
            vesting_account_address,
        } => to_binary(&try_get_end_time(vesting_account_address, deps)?),
        QueryMsg::GetOriginalVesting {
            vesting_account_address,
        } => to_binary(&try_get_original_vesting(vesting_account_address, deps)?),
        QueryMsg::GetDelegatedFree {
            block_time,
            vesting_account_address,
        } => to_binary(&try_get_delegated_free(
            block_time,
            vesting_account_address,
            env,
            deps,
        )?),
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

pub fn try_get_locked_coins(
    vesting_account_address: String,
    block_time: Option<Timestamp>,
    env: Env,
    deps: Deps,
) -> Result<Coin, ContractError> {
    let address = deps.api.addr_validate(&vesting_account_address)?;
    if let Some(account) = get_account(deps.storage, &address) {
        Ok(account.locked_coins(block_time, &env, deps.storage)?)
    } else {
        Err(ContractError::NoAccountForAddress(vesting_account_address))
    }
}

pub fn try_get_spendable_coins(
    vesting_account_address: String,
    block_time: Option<Timestamp>,
    env: Env,
    deps: Deps,
) -> Result<Coin, ContractError> {
    let address = deps.api.addr_validate(&vesting_account_address)?;
    if let Some(account) = get_account(deps.storage, &address) {
        Ok(account.spendable_coins(block_time, &env, deps.storage)?)
    } else {
        Err(ContractError::NoAccountForAddress(vesting_account_address))
    }
}

pub fn try_get_vested_coins(
    vesting_account_address: String,
    block_time: Option<Timestamp>,
    env: Env,
    deps: Deps,
) -> Result<Coin, ContractError> {
    let address = deps.api.addr_validate(&vesting_account_address)?;
    if let Some(account) = get_account(deps.storage, &address) {
        Ok(account.get_vested_coins(block_time, &env)?)
    } else {
        Err(ContractError::NoAccountForAddress(vesting_account_address))
    }
}

pub fn try_get_vesting_coins(
    vesting_account_address: String,
    block_time: Option<Timestamp>,
    env: Env,
    deps: Deps,
) -> Result<Coin, ContractError> {
    let address = deps.api.addr_validate(&vesting_account_address)?;
    if let Some(account) = get_account(deps.storage, &address) {
        Ok(account.get_vesting_coins(block_time, &env)?)
    } else {
        Err(ContractError::NoAccountForAddress(vesting_account_address))
    }
}

pub fn try_get_start_time(
    vesting_account_address: String,
    deps: Deps,
) -> Result<Timestamp, ContractError> {
    let address = deps.api.addr_validate(&vesting_account_address)?;
    if let Some(account) = get_account(deps.storage, &address) {
        Ok(account.get_start_time())
    } else {
        Err(ContractError::NoAccountForAddress(vesting_account_address))
    }
}

pub fn try_get_end_time(
    vesting_account_address: String,
    deps: Deps,
) -> Result<Timestamp, ContractError> {
    let address = deps.api.addr_validate(&vesting_account_address)?;
    if let Some(account) = get_account(deps.storage, &address) {
        Ok(account.get_end_time())
    } else {
        Err(ContractError::NoAccountForAddress(vesting_account_address))
    }
}

pub fn try_get_original_vesting(
    vesting_account_address: String,
    deps: Deps,
) -> Result<Coin, ContractError> {
    let address = deps.api.addr_validate(&vesting_account_address)?;
    if let Some(account) = get_account(deps.storage, &address) {
        Ok(account.get_original_vesting())
    } else {
        Err(ContractError::NoAccountForAddress(vesting_account_address))
    }
}

pub fn try_get_delegated_free(
    block_time: Option<Timestamp>,
    vesting_account_address: String,
    env: Env,
    deps: Deps,
) -> Result<Coin, ContractError> {
    let address = deps.api.addr_validate(&vesting_account_address)?;
    if let Some(account) = get_account(deps.storage, &address) {
        Ok(account.get_delegated_free(block_time, &env, deps.storage)?)
    } else {
        Err(ContractError::NoAccountForAddress(vesting_account_address))
    }
}

pub fn try_get_delegated_vesting(
    block_time: Option<Timestamp>,
    vesting_account_address: String,
    env: Env,
    deps: Deps,
) -> Result<Coin, ContractError> {
    let address = deps.api.addr_validate(&vesting_account_address)?;
    if let Some(account) = get_account(deps.storage, &address) {
        Ok(account.get_delegated_vesting(block_time, &env, deps.storage)?)
    } else {
        Err(ContractError::NoAccountForAddress(vesting_account_address))
    }
}

fn validate_funds(funds: &[Coin]) -> Result<Coin, ContractError> {
    if funds.is_empty() || funds[0].amount.is_zero() {
        return Err(ContractError::EmptyFunds);
    }

    if funds.len() > 1 {
        return Err(ContractError::MultipleDenoms);
    }

    if funds[0].denom != DENOM {
        return Err(ContractError::WrongDenom(
            funds[0].denom.clone(),
            DENOM.to_string(),
        ));
    }

    Ok(funds[0].clone())
}
