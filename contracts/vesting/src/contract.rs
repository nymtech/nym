use crate::errors::ContractError;
use crate::queued_migrations::migrate_to_v2_mixnet_contract;
use crate::storage::{
    account_from_address, locked_pledge_cap, update_locked_pledge_cap, BlockTimestampSecs, ADMIN,
    DELEGATIONS, MIXNET_CONTRACT_ADDRESS, MIX_DENOM,
};
use crate::traits::{
    DelegatingAccount, GatewayBondingAccount, MixnodeBondingAccount, VestingAccount,
};
use crate::vesting::{populate_vesting_periods, Account};
use cosmwasm_std::{
    coin, entry_point, to_binary, BankMsg, Coin, Deps, DepsMut, Env, MessageInfo, Order,
    QueryResponse, Response, StdResult, Timestamp, Uint128,
};
use cw_storage_plus::Bound;
use mixnet_contract_common::mixnode::{MixNodeConfigUpdate, MixNodeCostParams};
use mixnet_contract_common::{Gateway, MixNode, NodeId};
use vesting_contract_common::events::{
    new_ownership_transfer_event, new_periodic_vesting_account_event,
    new_staking_address_update_event, new_track_gateway_unbond_event,
    new_track_mixnode_unbond_event, new_track_reward_event, new_track_undelegation_event,
    new_vested_coins_withdraw_event,
};
use vesting_contract_common::messages::{
    ExecuteMsg, InitMsg, MigrateMsg, QueryMsg, VestingSpecification,
};
use vesting_contract_common::{
    AllDelegationsResponse, DelegationTimesResponse, OriginalVestingResponse, Period, PledgeData,
    VestingDelegation,
};

pub const INITIAL_LOCKED_PLEDGE_CAP: Uint128 = Uint128::new(100_000_000_000);

/// Instantiate the contract
#[entry_point]
pub fn instantiate(
    deps: DepsMut<'_>,
    _env: Env,
    info: MessageInfo,
    msg: InitMsg,
) -> Result<Response, ContractError> {
    //! ADMIN is set to the address that instantiated the contract
    ADMIN.save(deps.storage, &info.sender.to_string())?;
    MIXNET_CONTRACT_ADDRESS.save(deps.storage, &msg.mixnet_contract_address)?;
    MIX_DENOM.save(deps.storage, &msg.mix_denom)?;
    Ok(Response::default())
}

#[entry_point]
pub fn migrate(deps: DepsMut<'_>, _env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    migrate_to_v2_mixnet_contract(deps, msg)
}

#[entry_point]
pub fn execute(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateLockedPledgeCap { amount } => {
            try_update_locked_pledge_cap(amount, info, deps)
        }
        ExecuteMsg::TrackReward { amount, address } => {
            try_track_reward(deps, info, amount, &address)
        }
        ExecuteMsg::ClaimOperatorReward {} => try_claim_operator_reward(deps, info),
        ExecuteMsg::ClaimDelegatorReward { mix_id } => {
            try_claim_delegator_reward(deps, info, mix_id)
        }
        ExecuteMsg::UpdateMixnodeConfig { new_config } => {
            try_update_mixnode_config(new_config, info, deps)
        }
        ExecuteMsg::UpdateMixnodeCostParams { new_costs } => {
            try_update_mixnode_cost_params(new_costs, info, deps)
        }
        ExecuteMsg::UpdateMixnetAddress { address } => {
            try_update_mixnet_address(address, info, deps)
        }
        ExecuteMsg::DelegateToMixnode { mix_id, amount } => {
            try_delegate_to_mixnode(mix_id, amount, info, env, deps)
        }
        ExecuteMsg::UndelegateFromMixnode { mix_id } => {
            try_undelegate_from_mixnode(mix_id, info, deps)
        }
        ExecuteMsg::CreateAccount {
            owner_address,
            staking_address,
            vesting_spec,
        } => try_create_periodic_vesting_account(
            &owner_address,
            staking_address,
            vesting_spec,
            info,
            env,
            deps,
        ),
        ExecuteMsg::WithdrawVestedCoins { amount } => {
            try_withdraw_vested_coins(amount, env, info, deps)
        }
        ExecuteMsg::TrackUndelegation {
            owner,
            mix_id,
            amount,
        } => try_track_undelegation(&owner, mix_id, amount, info, deps),
        ExecuteMsg::BondMixnode {
            mix_node,
            cost_params,
            owner_signature,
            amount,
        } => try_bond_mixnode(
            mix_node,
            cost_params,
            owner_signature,
            amount,
            info,
            env,
            deps,
        ),
        ExecuteMsg::UnbondMixnode {} => try_unbond_mixnode(info, deps),
        ExecuteMsg::TrackUnbondMixnode { owner, amount } => {
            try_track_unbond_mixnode(&owner, amount, info, deps)
        }
        ExecuteMsg::BondGateway {
            gateway,
            owner_signature,
            amount,
        } => try_bond_gateway(gateway, owner_signature, amount, info, env, deps),
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

/// Update locked_pledge_cap, the hard cap for staking/bonding with unvested tokens.
///
/// Callable by ADMIN only, see [instantiate].
pub fn try_update_locked_pledge_cap(
    amount: Uint128,
    info: MessageInfo,
    deps: DepsMut,
) -> Result<Response, ContractError> {
    if info.sender != ADMIN.load(deps.storage)? {
        return Err(ContractError::NotAdmin(info.sender.as_str().to_string()));
    }
    update_locked_pledge_cap(amount, deps.storage)?;
    Ok(Response::default())
}

/// Update config for a mixnode bonded with vesting account, sends [mixnet_contract_common::ExecuteMsg::UpdateMixnodeConfig] to [crate::storage::MIXNET_CONTRACT_ADDRESS].
pub fn try_update_mixnode_config(
    new_config: MixNodeConfigUpdate,
    info: MessageInfo,
    deps: DepsMut,
) -> Result<Response, ContractError> {
    let account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;
    account.try_update_mixnode_config(new_config, deps.storage)
}

pub fn try_update_mixnode_cost_params(
    new_costs: MixNodeCostParams,
    info: MessageInfo,
    deps: DepsMut,
) -> Result<Response, ContractError> {
    let account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;
    account.try_update_mixnode_cost_params(new_costs, deps.storage)
}

/// Updates mixnet contract address, for cases when a new mixnet contract is deployed.
///
/// Callable by ADMIN only, see [instantiate].
pub fn try_update_mixnet_address(
    address: String,
    info: MessageInfo,
    deps: DepsMut<'_>,
) -> Result<Response, ContractError> {
    if info.sender != ADMIN.load(deps.storage)? {
        return Err(ContractError::NotAdmin(info.sender.as_str().to_string()));
    }
    MIXNET_CONTRACT_ADDRESS.save(deps.storage, &address)?;
    Ok(Response::default())
}

/// Withdraw already vested coins.
pub fn try_withdraw_vested_coins(
    amount: Coin,
    env: Env,
    info: MessageInfo,
    deps: DepsMut<'_>,
) -> Result<Response, ContractError> {
    let mix_denom = MIX_DENOM.load(deps.storage)?;
    if amount.denom != mix_denom {
        return Err(ContractError::WrongDenom(amount.denom, mix_denom));
    }

    let address = info.sender.clone();
    let account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;
    if address != account.owner_address() {
        return Err(ContractError::NotOwner(account.owner_address().to_string()));
    }
    let spendable_coins = account.spendable_coins(None, &env, deps.storage)?;
    if amount.amount <= spendable_coins.amount {
        let new_balance = account.withdraw(&amount, deps.storage)?;

        let send_tokens = BankMsg::Send {
            to_address: account.owner_address().as_str().to_string(),
            amount: vec![amount.clone()],
        };

        Ok(Response::new()
            .add_message(send_tokens)
            .add_event(new_vested_coins_withdraw_event(
                &address,
                &amount,
                &coin(new_balance, &amount.denom),
            )))
    } else {
        Err(ContractError::InsufficientSpendable(
            account.owner_address().as_str().to_string(),
            spendable_coins.amount.u128(),
        ))
    }
}

/// Transfer ownership of the entire vesting account.
fn try_transfer_ownership(
    to_address: String,
    info: MessageInfo,
    deps: DepsMut<'_>,
) -> Result<Response, ContractError> {
    let address = info.sender.clone();
    let to_address = deps.api.addr_validate(&to_address)?;
    let mut account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;
    if address == account.owner_address() {
        account.transfer_ownership(&to_address, deps.storage)?;
        Ok(Response::new().add_event(new_ownership_transfer_event(&address, &to_address)))
    } else {
        Err(ContractError::NotOwner(account.owner_address().to_string()))
    }
}

/// Set or update staking address for a vesting account.
fn try_update_staking_address(
    to_address: Option<String>,
    info: MessageInfo,
    deps: DepsMut<'_>,
) -> Result<Response, ContractError> {
    let address = info.sender.clone();
    let to_address = to_address.and_then(|x| deps.api.addr_validate(&x).ok());
    let mut account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;
    if address == account.owner_address() {
        let old = account.staking_address().cloned();
        account.update_staking_address(to_address.clone(), deps.storage)?;
        Ok(Response::new().add_event(new_staking_address_update_event(&old, &to_address)))
    } else {
        Err(ContractError::NotOwner(account.owner_address().to_string()))
    }
}

/// Bond a gateway, sends [mixnet_contract_common::ExecuteMsg::BondGatewayOnBehalf] to [crate::storage::MIXNET_CONTRACT_ADDRESS].
pub fn try_bond_gateway(
    gateway: Gateway,
    owner_signature: String,
    amount: Coin,
    info: MessageInfo,
    env: Env,
    deps: DepsMut<'_>,
) -> Result<Response, ContractError> {
    let mix_denom = MIX_DENOM.load(deps.storage)?;
    let pledge = validate_funds(&[amount], mix_denom)?;
    let account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;
    account.try_bond_gateway(gateway, owner_signature, pledge, &env, deps.storage)
}

/// Unbond a gateway, sends [mixnet_contract_common::ExecuteMsg::UnbondGatewayOnBehalf] to [crate::storage::MIXNET_CONTRACT_ADDRESS].
pub fn try_unbond_gateway(info: MessageInfo, deps: DepsMut<'_>) -> Result<Response, ContractError> {
    let account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;
    account.try_unbond_gateway(deps.storage)
}

/// Track gateway unbonding, invoked by the mixnet contract after succesful unbonding, message containes coins returned including any accrued rewards.
pub fn try_track_unbond_gateway(
    owner: &str,
    amount: Coin,
    info: MessageInfo,
    deps: DepsMut<'_>,
) -> Result<Response, ContractError> {
    if info.sender != MIXNET_CONTRACT_ADDRESS.load(deps.storage)? {
        return Err(ContractError::NotMixnetContract(info.sender));
    }
    let account = account_from_address(owner, deps.storage, deps.api)?;
    account.try_track_unbond_gateway(amount, deps.storage)?;
    Ok(Response::new().add_event(new_track_gateway_unbond_event()))
}

/// Bond a mixnode, sends [mixnet_contract_common::ExecuteMsg::BondMixnodeOnBehalf] to [crate::storage::MIXNET_CONTRACT_ADDRESS].
pub fn try_bond_mixnode(
    mix_node: MixNode,
    cost_params: MixNodeCostParams,
    owner_signature: String,
    amount: Coin,
    info: MessageInfo,
    env: Env,
    deps: DepsMut<'_>,
) -> Result<Response, ContractError> {
    let mix_denom = MIX_DENOM.load(deps.storage)?;
    let pledge = validate_funds(&[amount], mix_denom)?;
    let account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;
    account.try_bond_mixnode(
        mix_node,
        cost_params,
        owner_signature,
        pledge,
        &env,
        deps.storage,
    )
}

/// Unbond a mixnode, sends [mixnet_contract_common::ExecuteMsg::UnbondMixnodeOnBehalf] to [crate::storage::MIXNET_CONTRACT_ADDRESS].
pub fn try_unbond_mixnode(info: MessageInfo, deps: DepsMut<'_>) -> Result<Response, ContractError> {
    let account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;
    account.try_unbond_mixnode(deps.storage)
}

/// Track mixnode unbonding, invoked by the mixnet contract after succesful unbonding, message containes coins returned including any accrued rewards.
pub fn try_track_unbond_mixnode(
    owner: &str,
    amount: Coin,
    info: MessageInfo,
    deps: DepsMut<'_>,
) -> Result<Response, ContractError> {
    if info.sender != MIXNET_CONTRACT_ADDRESS.load(deps.storage)? {
        return Err(ContractError::NotMixnetContract(info.sender));
    }
    let account = account_from_address(owner, deps.storage, deps.api)?;
    account.try_track_unbond_mixnode(amount, deps.storage)?;
    Ok(Response::new().add_event(new_track_mixnode_unbond_event()))
}

/// Track reward collection, invoked by the mixnert contract after sucessful reward compounding or claiming
fn try_track_reward(
    deps: DepsMut<'_>,
    info: MessageInfo,
    amount: Coin,
    address: &str,
) -> Result<Response, ContractError> {
    if info.sender != MIXNET_CONTRACT_ADDRESS.load(deps.storage)? {
        return Err(ContractError::NotMixnetContract(info.sender));
    }
    let account = account_from_address(address, deps.storage, deps.api)?;
    account.track_reward(amount, deps.storage)?;
    Ok(Response::new().add_event(new_track_reward_event()))
}

/// Track undelegation, invoked by the mixnet contract after sucessful undelegation, message contains coins returned with any accrued rewards.
fn try_track_undelegation(
    address: &str,
    mix_id: NodeId,
    amount: Coin,
    info: MessageInfo,
    deps: DepsMut<'_>,
) -> Result<Response, ContractError> {
    if info.sender != MIXNET_CONTRACT_ADDRESS.load(deps.storage)? {
        return Err(ContractError::NotMixnetContract(info.sender));
    }
    let account = account_from_address(address, deps.storage, deps.api)?;

    account.track_undelegation(mix_id, amount, deps.storage)?;
    Ok(Response::new().add_event(new_track_undelegation_event()))
}

/// Delegate to mixnode, sends [mixnet_contract_common::ExecuteMsg::DelegateToMixnodeOnBehalf] to [crate::storage::MIXNET_CONTRACT_ADDRESS]..
fn try_delegate_to_mixnode(
    mix_id: NodeId,
    amount: Coin,
    info: MessageInfo,
    env: Env,
    deps: DepsMut<'_>,
) -> Result<Response, ContractError> {
    let mix_denom = MIX_DENOM.load(deps.storage)?;
    let amount = validate_funds(&[amount], mix_denom)?;
    let account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;

    account.try_delegate_to_mixnode(mix_id, amount, &env, deps.storage)
}

/// Claims operator reward, sends [mixnet_contract_common::ExecuteMsg::ClaimOperatorRewardOnBehalf] to [crate::storage::MIXNET_CONTRACT_ADDRESS].
fn try_claim_operator_reward(
    deps: DepsMut<'_>,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;
    account.try_claim_operator_reward(deps.storage)
}

/// Claims delegator reward, sends [mixnet_contract_common::ExecuteMsg::ClaimDelegatorRewardOnBehalf] to [crate::storage::MIXNET_CONTRACT_ADDRESS].
fn try_claim_delegator_reward(
    deps: DepsMut<'_>,
    info: MessageInfo,
    mix_id: NodeId,
) -> Result<Response, ContractError> {
    let account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;

    account.try_claim_delegator_reward(mix_id, deps.storage)
}

/// Undelegates from a mixnode, sends [mixnet_contract_common::ExecuteMsg::UndelegateFromMixnodeOnBehalf] to [crate::storage::MIXNET_CONTRACT_ADDRESS].
fn try_undelegate_from_mixnode(
    mix_id: NodeId,
    info: MessageInfo,
    deps: DepsMut<'_>,
) -> Result<Response, ContractError> {
    let account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;

    account.try_undelegate_from_mixnode(mix_id, deps.storage)
}

/// Creates a new periodic vesting account, and deposits funds to vest into the contract.
///
/// Callable by ADMIN only, see [instantiate].
fn try_create_periodic_vesting_account(
    owner_address: &str,
    staking_address: Option<String>,
    vesting_spec: Option<VestingSpecification>,
    info: MessageInfo,
    env: Env,
    deps: DepsMut<'_>,
) -> Result<Response, ContractError> {
    if info.sender != ADMIN.load(deps.storage)? {
        return Err(ContractError::NotAdmin(info.sender.as_str().to_string()));
    }
    let mix_denom = MIX_DENOM.load(deps.storage)?;

    let account_exists = account_from_address(owner_address, deps.storage, deps.api).is_ok();
    if account_exists {
        return Err(ContractError::AccountAlreadyExists(
            owner_address.to_string(),
        ));
    }

    let vesting_spec = vesting_spec.unwrap_or_default();

    let coin = validate_funds(&info.funds, mix_denom)?;

    let owner_address = deps.api.addr_validate(owner_address)?;
    let staking_address = if let Some(staking_address) = staking_address {
        Some(deps.api.addr_validate(&staking_address)?)
    } else {
        None
    };
    let start_time = vesting_spec
        .start_time()
        .unwrap_or_else(|| env.block.time.seconds());

    let periods = populate_vesting_periods(start_time, vesting_spec);

    let start_time = Timestamp::from_seconds(start_time);

    let response = Response::new();

    Account::new(
        owner_address.clone(),
        staking_address.clone(),
        coin.clone(),
        start_time,
        periods,
        deps.storage,
    )?;

    Ok(response.add_event(new_periodic_vesting_account_event(
        &owner_address,
        &coin,
        &staking_address,
        start_time,
    )))
}

#[entry_point]
pub fn query(deps: Deps<'_>, env: Env, msg: QueryMsg) -> Result<QueryResponse, ContractError> {
    let query_res = match msg {
        QueryMsg::GetLockedPledgeCap {} => to_binary(&get_locked_pledge_cap(deps)),
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
        QueryMsg::GetAccount { address } => to_binary(&try_get_account(&address, deps)?),
        QueryMsg::GetMixnode { address } => to_binary(&try_get_mixnode(&address, deps)?),
        QueryMsg::GetGateway { address } => to_binary(&try_get_gateway(&address, deps)?),
        QueryMsg::GetCurrentVestingPeriod { address } => {
            to_binary(&try_get_current_vesting_period(&address, deps, env)?)
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

/// Get locked_pledge_cap, the hard cap for staking/bonding with unvested tokens.
pub fn get_locked_pledge_cap(deps: Deps<'_>) -> Uint128 {
    locked_pledge_cap(deps.storage)
}

/// Get current vesting period for a given [crate::vesting::Account].
pub fn try_get_current_vesting_period(
    address: &str,
    deps: Deps<'_>,
    env: Env,
) -> Result<Period, ContractError> {
    let account = account_from_address(address, deps.storage, deps.api)?;
    Ok(account.get_current_vesting_period(env.block.time))
}

/// Loads mixnode bond from vesting contract storage.
pub fn try_get_mixnode(address: &str, deps: Deps<'_>) -> Result<Option<PledgeData>, ContractError> {
    let account = account_from_address(address, deps.storage, deps.api)?;
    account.load_mixnode_pledge(deps.storage)
}

/// Loads gateway bond from vesting contract storage.
pub fn try_get_gateway(address: &str, deps: Deps<'_>) -> Result<Option<PledgeData>, ContractError> {
    let account = account_from_address(address, deps.storage, deps.api)?;
    account.load_gateway_pledge(deps.storage)
}

pub fn try_get_account(address: &str, deps: Deps<'_>) -> Result<Account, ContractError> {
    account_from_address(address, deps.storage, deps.api)
}

/// Gets currently locked coins, see [crate::traits::VestingAccount::locked_coins]
pub fn try_get_locked_coins(
    vesting_account_address: &str,
    block_time: Option<Timestamp>,
    env: Env,
    deps: Deps<'_>,
) -> Result<Coin, ContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    account.locked_coins(block_time, &env, deps.storage)
}

/// Returns currently locked coins, see [crate::traits::VestingAccount::spendable_coins]
pub fn try_get_spendable_coins(
    vesting_account_address: &str,
    block_time: Option<Timestamp>,
    env: Env,
    deps: Deps<'_>,
) -> Result<Coin, ContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    account.spendable_coins(block_time, &env, deps.storage)
}

/// Returns coins that have vested, see [crate::traits::VestingAccount::get_vested_coins]
pub fn try_get_vested_coins(
    vesting_account_address: &str,
    block_time: Option<Timestamp>,
    env: Env,
    deps: Deps<'_>,
) -> Result<Coin, ContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    account.get_vested_coins(block_time, &env, deps.storage)
}

/// Returns coins that are vesting, see [crate::traits::VestingAccount::get_vesting_coins]
pub fn try_get_vesting_coins(
    vesting_account_address: &str,
    block_time: Option<Timestamp>,
    env: Env,
    deps: Deps<'_>,
) -> Result<Coin, ContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    account.get_vesting_coins(block_time, &env, deps.storage)
}

/// See [crate::traits::VestingAccount::get_start_time]
pub fn try_get_start_time(
    vesting_account_address: &str,
    deps: Deps<'_>,
) -> Result<Timestamp, ContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    Ok(account.get_start_time())
}

/// See [crate::traits::VestingAccount::get_end_time]
pub fn try_get_end_time(
    vesting_account_address: &str,
    deps: Deps<'_>,
) -> Result<Timestamp, ContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    Ok(account.get_end_time())
}

/// See [crate::traits::VestingAccount::get_original_vesting]
pub fn try_get_original_vesting(
    vesting_account_address: &str,
    deps: Deps<'_>,
) -> Result<OriginalVestingResponse, ContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    Ok(account.get_original_vesting())
}

/// See [crate::traits::VestingAccount::get_delegated_free]
pub fn try_get_delegated_free(
    block_time: Option<Timestamp>,
    vesting_account_address: &str,
    env: Env,
    deps: Deps<'_>,
) -> Result<Coin, ContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    account.get_delegated_free(block_time, &env, deps.storage)
}

/// See [crate::traits::VestingAccount::get_delegated_vesting]
pub fn try_get_delegated_vesting(
    block_time: Option<Timestamp>,
    vesting_account_address: &str,
    env: Env,
    deps: Deps<'_>,
) -> Result<Coin, ContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    account.get_delegated_vesting(block_time, &env, deps.storage)
}

/// Returns timestamps at which delegations were made
pub fn try_get_delegation_times(
    deps: Deps<'_>,
    vesting_account_address: &str,
    mix_id: NodeId,
) -> Result<DelegationTimesResponse, ContractError> {
    let owner = deps.api.addr_validate(vesting_account_address)?;
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;

    let delegation_timestamps = DELEGATIONS
        .prefix((account.storage_key(), mix_id))
        .keys(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;

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
) -> Result<AllDelegationsResponse, ContractError> {
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

fn validate_funds(funds: &[Coin], mix_denom: String) -> Result<Coin, ContractError> {
    if funds.is_empty() || funds[0].amount.is_zero() {
        return Err(ContractError::EmptyFunds);
    }

    if funds.len() > 1 {
        return Err(ContractError::MultipleDenoms);
    }

    if funds[0].denom != mix_denom {
        return Err(ContractError::WrongDenom(funds[0].denom.clone(), mix_denom));
    }

    Ok(funds[0].clone())
}
