use crate::errors::ContractError;
use crate::messages::{ExecuteMsg, InitMsg, QueryMsg};
use crate::storage::account_from_address;
use crate::traits::{
    DelegatingAccount, GatewayBondingAccount, MixnodeBondingAccount, VestingAccount,
};
use crate::vesting::{populate_vesting_periods, Account};
use config::defaults::{DEFAULT_MIXNET_CONTRACT_ADDRESS, DENOM};
use cosmwasm_std::{
    entry_point, to_binary, BankMsg, Coin, Deps, DepsMut, Env, MessageInfo, QueryResponse,
    Response, Timestamp, Uint128,
};
use mixnet_contract::{Gateway, IdentityKey, MixNode};

// We're using a 24 month vesting period with 3 months sub-periods.
// There are 8 three month periods in two years
// and duration of a single period is 30 days.
pub const NUM_VESTING_PERIODS: usize = 8;
pub const VESTING_PERIOD: u64 = 3 * 30 * 86400;
// Address of the account set to be contract admin
pub const ADMIN_ADDRESS: &str = "admin";

#[entry_point]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InitMsg,
) -> Result<Response, ContractError> {
    Ok(Response::default())
}

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
        ExecuteMsg::CreateAccount {
            owner_address,
            staking_address,
            start_time,
        } => try_create_periodic_vesting_account(
            &owner_address,
            staking_address,
            start_time,
            info,
            env,
            deps,
        ),
        ExecuteMsg::WithdrawVestedCoins { amount } => {
            try_withdraw_vested_coins(amount, env, info, deps)
        }
        ExecuteMsg::TrackUndelegation {
            owner,
            mix_identity,
            amount,
        } => try_track_undelegation(&owner, mix_identity, amount, info, deps),
        ExecuteMsg::BondMixnode {
            mix_node,
            owner_signature,
        } => try_bond_mixnode(mix_node, owner_signature, info, env, deps),
        ExecuteMsg::UnbondMixnode {} => try_unbond_mixnode(info, deps),
        ExecuteMsg::TrackUnbondMixnode { owner, amount } => {
            try_track_unbond_mixnode(&owner, amount, info, deps)
        }
        ExecuteMsg::BondGateway {
            gateway,
            owner_signature,
        } => try_bond_gateway(gateway, owner_signature, info, env, deps),
        ExecuteMsg::UnbondGateway {} => try_unbond_gateway(info, deps),
        ExecuteMsg::TrackUnbondGateway { owner, amount } => {
            try_track_unbond_gateway(&owner, amount, info, deps)
        }
        ExecuteMsg::TransferOwnership { to_address } => {
            try_transfer_ownership(to_address, info, deps)
        }
        ExecuteMsg::UpdateStakingAddress { to_address } => {
            try_update_staking_address(to_address, info, deps)
        }
    }
}

fn try_transfer_ownership(
    to_address: String,
    info: MessageInfo,
    deps: DepsMut,
) -> Result<Response, ContractError> {
    let address = info.sender.clone();
    let to_address = deps.api.addr_validate(&to_address)?;
    let mut account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;
    println!("{}", address);
    if address == account.owner_address() {
        account.transfer_ownership(&to_address, deps.storage)?;
        Ok(Response::default())
    } else {
        Err(ContractError::NotOwner(account.owner_address().to_string()))
    }
}

fn try_update_staking_address(
    to_address: Option<String>,
    info: MessageInfo,
    deps: DepsMut,
) -> Result<Response, ContractError> {
    let address = info.sender.clone();
    let to_address = to_address.and_then(|x| deps.api.addr_validate(&x).ok());
    let mut account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;
    if address == account.owner_address() {
        account.update_staking_address(to_address, deps.storage)?;
        Ok(Response::default())
    } else {
        Err(ContractError::NotOwner(account.owner_address().to_string()))
    }
}

pub fn try_bond_gateway(
    gateway: Gateway,
    owner_signature: String,
    info: MessageInfo,
    env: Env,
    deps: DepsMut,
) -> Result<Response, ContractError> {
    let pledge = validate_funds(&info.funds)?;
    let account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;
    account.try_bond_gateway(gateway, owner_signature, pledge, &env, deps.storage)
}

pub fn try_unbond_gateway(info: MessageInfo, deps: DepsMut) -> Result<Response, ContractError> {
    let account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;
    account.try_unbond_gateway(deps.storage)
}

pub fn try_track_unbond_gateway(
    owner: &str,
    amount: Coin,
    info: MessageInfo,
    deps: DepsMut,
) -> Result<Response, ContractError> {
    if info.sender != DEFAULT_MIXNET_CONTRACT_ADDRESS {
        return Err(ContractError::NotMixnetContract(info.sender));
    }
    let account = account_from_address(owner, deps.storage, deps.api)?;
    account.try_track_unbond_gateway(amount, deps.storage)?;
    Ok(Response::default())
}

pub fn try_bond_mixnode(
    mix_node: MixNode,
    owner_signature: String,
    info: MessageInfo,
    env: Env,
    deps: DepsMut,
) -> Result<Response, ContractError> {
    let pledge = validate_funds(&info.funds)?;
    let account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;
    account.try_bond_mixnode(mix_node, owner_signature, pledge, &env, deps.storage)
}

pub fn try_unbond_mixnode(info: MessageInfo, deps: DepsMut) -> Result<Response, ContractError> {
    let account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;
    account.try_unbond_mixnode(deps.storage)
}

pub fn try_track_unbond_mixnode(
    owner: &str,
    amount: Coin,
    info: MessageInfo,
    deps: DepsMut,
) -> Result<Response, ContractError> {
    if info.sender != DEFAULT_MIXNET_CONTRACT_ADDRESS {
        return Err(ContractError::NotMixnetContract(info.sender));
    }
    let account = account_from_address(owner, deps.storage, deps.api)?;
    account.try_track_unbond_mixnode(amount, deps.storage)?;
    Ok(Response::default())
}

pub fn try_withdraw_vested_coins(
    amount: Coin,
    env: Env,
    info: MessageInfo,
    deps: DepsMut,
) -> Result<Response, ContractError> {
    let address = info.sender.clone();
    let account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;
    if address != account.owner_address() {
        return Err(ContractError::NotOwner(account.owner_address().to_string()));
    }
    let spendable_coins = account.spendable_coins(None, &env, deps.storage)?;
    if amount.amount <= spendable_coins.amount {
        let new_balance = account
            .load_balance(deps.storage)?
            .u128()
            .saturating_sub(amount.amount.u128());
        account.save_balance(Uint128::new(new_balance), deps.storage)?;

        let send_tokens = BankMsg::Send {
            to_address: account.owner_address().as_str().to_string(),
            amount: vec![amount],
        };

        Ok(Response::new()
            .add_attribute("action", "whitdraw")
            .add_message(send_tokens))
    } else {
        Err(ContractError::InsufficientSpendable(
            account.owner_address().as_str().to_string(),
            spendable_coins.amount.u128(),
        ))
    }
}

fn try_track_undelegation(
    address: &str,
    mix_identity: IdentityKey,
    amount: Coin,
    info: MessageInfo,
    deps: DepsMut,
) -> Result<Response, ContractError> {
    if info.sender != DEFAULT_MIXNET_CONTRACT_ADDRESS {
        return Err(ContractError::NotMixnetContract(info.sender));
    }
    let account = account_from_address(address, deps.storage, deps.api)?;
    account.track_undelegation(mix_identity, amount, deps.storage)?;
    Ok(Response::default())
}

fn try_delegate_to_mixnode(
    mix_identity: IdentityKey,
    info: MessageInfo,
    env: Env,
    deps: DepsMut,
) -> Result<Response, ContractError> {
    let amount = validate_funds(&info.funds)?;
    let account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;
    account.try_delegate_to_mixnode(mix_identity, amount, &env, deps.storage)
}

fn try_undelegate_from_mixnode(
    mix_identity: IdentityKey,
    info: MessageInfo,
    deps: DepsMut,
) -> Result<Response, ContractError> {
    let account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;
    account.try_undelegate_from_mixnode(mix_identity, deps.storage)
}

fn try_create_periodic_vesting_account(
    owner_address: &str,
    staking_address: Option<String>,
    start_time: Option<u64>,
    info: MessageInfo,
    env: Env,
    deps: DepsMut,
) -> Result<Response, ContractError> {
    if info.sender != ADMIN_ADDRESS {
        return Err(ContractError::NotAdmin(info.sender.as_str().to_string()));
    }
    let coin = validate_funds(&info.funds)?;
    let owner_address = deps.api.addr_validate(owner_address)?;
    let staking_address = if let Some(staking_address) = staking_address {
        Some(deps.api.addr_validate(&staking_address)?)
    } else {
        None
    };
    let start_time = start_time.unwrap_or_else(|| env.block.time.seconds());
    let periods = populate_vesting_periods(start_time, NUM_VESTING_PERIODS);
    Account::new(
        owner_address,
        staking_address,
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
        QueryMsg::GetDelegatedFree {
            block_time,
            vesting_account_address,
        } => to_binary(&try_get_delegated_free(
            block_time,
            &vesting_account_address,
            env,
            deps,
        )?),
        QueryMsg::GetDelegatedVesting {
            block_time,
            vesting_account_address,
        } => to_binary(&try_get_delegated_vesting(
            block_time,
            &vesting_account_address,
            env,
            deps,
        )?),
    };

    Ok(query_res?)
}

pub fn try_get_locked_coins(
    vesting_account_address: &str,
    block_time: Option<Timestamp>,
    env: Env,
    deps: Deps,
) -> Result<Coin, ContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    account.locked_coins(block_time, &env, deps.storage)
}

pub fn try_get_spendable_coins(
    vesting_account_address: &str,
    block_time: Option<Timestamp>,
    env: Env,
    deps: Deps,
) -> Result<Coin, ContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    account.spendable_coins(block_time, &env, deps.storage)
}

pub fn try_get_vested_coins(
    vesting_account_address: &str,
    block_time: Option<Timestamp>,
    env: Env,
    deps: Deps,
) -> Result<Coin, ContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    account.get_vested_coins(block_time, &env)
}

pub fn try_get_vesting_coins(
    vesting_account_address: &str,
    block_time: Option<Timestamp>,
    env: Env,
    deps: Deps,
) -> Result<Coin, ContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    account.get_vesting_coins(block_time, &env)
}

pub fn try_get_start_time(
    vesting_account_address: &str,
    deps: Deps,
) -> Result<Timestamp, ContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    Ok(account.get_start_time())
}

pub fn try_get_end_time(
    vesting_account_address: &str,
    deps: Deps,
) -> Result<Timestamp, ContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    Ok(account.get_end_time())
}

pub fn try_get_original_vesting(
    vesting_account_address: &str,
    deps: Deps,
) -> Result<Coin, ContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    Ok(account.get_original_vesting())
}

pub fn try_get_delegated_free(
    block_time: Option<Timestamp>,
    vesting_account_address: &str,
    env: Env,
    deps: Deps,
) -> Result<Coin, ContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    account.get_delegated_free(block_time, &env, deps.storage)
}

pub fn try_get_delegated_vesting(
    block_time: Option<Timestamp>,
    vesting_account_address: &str,
    env: Env,
    deps: Deps,
) -> Result<Coin, ContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    account.get_delegated_vesting(block_time, &env, deps.storage)
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
