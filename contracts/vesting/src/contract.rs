use crate::errors::ContractError;
use crate::storage::{
    account_from_address, save_account, BlockTimestampSecs, ACCOUNTS, ADMIN, DELEGATIONS,
    MIXNET_CONTRACT_ADDRESS, MIX_DENOM,
};
use crate::traits::{
    DelegatingAccount, GatewayBondingAccount, MixnodeBondingAccount, NodeFamilies, VestingAccount,
};
use crate::vesting::{populate_vesting_periods, Account};
use contracts_common::ContractBuildInformation;
use cosmwasm_std::{
    coin, entry_point, to_binary, Addr, BankMsg, Coin, Deps, DepsMut, Env, MessageInfo, Order,
    QueryResponse, Response, StdResult, Timestamp, Uint128,
};
use cw_storage_plus::Bound;
use mixnet_contract_common::mixnode::{MixNodeConfigUpdate, MixNodeCostParams};
use mixnet_contract_common::{Gateway, MixId, MixNode};
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
    AccountVestingCoins, AccountsResponse, AllDelegationsResponse, BaseVestingAccountInfo,
    DelegationTimesResponse, OriginalVestingResponse, Period, PledgeCap, PledgeData,
    VestingCoinsResponse, VestingDelegation,
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
    // validate the received mixnet contract address
    let mixnet_contract_address = deps.api.addr_validate(&msg.mixnet_contract_address)?;

    // ADMIN is set to the address that instantiated the contract
    ADMIN.save(deps.storage, &info.sender)?;
    MIXNET_CONTRACT_ADDRESS.save(deps.storage, &mixnet_contract_address)?;
    MIX_DENOM.save(deps.storage, &msg.mix_denom)?;
    Ok(Response::default())
}

#[entry_point]
pub fn migrate(_deps: DepsMut<'_>, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Response::new())
}

#[entry_point]
pub fn execute(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CreateFamily {
            owner_signature,
            label,
        } => try_create_family(info, deps, owner_signature, label),
        ExecuteMsg::JoinFamily {
            signature,
            family_head,
        } => try_join_family(info, deps, signature, family_head),
        ExecuteMsg::LeaveFamily {
            signature,
            family_head,
        } => try_leave_family(info, deps, signature, family_head),
        ExecuteMsg::KickFamilyMember { signature, member } => {
            try_kick_family_member(info, deps, signature, member)
        }
        ExecuteMsg::UpdateLockedPledgeCap { address, cap } => {
            try_update_locked_pledge_cap(address, cap, info, deps)
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
        ExecuteMsg::DelegateToMixnode {
            mix_id,
            amount,
            on_behalf_of,
        } => try_delegate_to_mixnode(mix_id, amount, on_behalf_of, info, env, deps),
        ExecuteMsg::UndelegateFromMixnode {
            mix_id,
            on_behalf_of,
        } => try_undelegate_from_mixnode(mix_id, on_behalf_of, info, deps),
        ExecuteMsg::CreateAccount {
            owner_address,
            staking_address,
            vesting_spec,
            cap,
        } => try_create_periodic_vesting_account(
            &owner_address,
            staking_address,
            vesting_spec,
            cap,
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
        ExecuteMsg::PledgeMore { amount } => try_pledge_more(deps, env, info, amount),
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

pub fn try_create_family(
    info: MessageInfo,
    deps: DepsMut,
    owner_signature: String,
    label: String,
) -> Result<Response, ContractError> {
    let account = account_from_address(info.sender.as_ref(), deps.storage, deps.api)?;
    account.try_create_family(deps.storage, owner_signature, label)
}
pub fn try_join_family(
    info: MessageInfo,
    deps: DepsMut,
    signature: String,
    family_head: String,
) -> Result<Response, ContractError> {
    let account = account_from_address(info.sender.as_ref(), deps.storage, deps.api)?;
    account.try_join_family(deps.storage, signature, &family_head)
}
pub fn try_leave_family(
    info: MessageInfo,
    deps: DepsMut,
    signature: String,
    family_head: String,
) -> Result<Response, ContractError> {
    let account = account_from_address(info.sender.as_ref(), deps.storage, deps.api)?;
    account.try_leave_family(deps.storage, signature, &family_head)
}
pub fn try_kick_family_member(
    info: MessageInfo,
    deps: DepsMut,
    signature: String,
    member: String,
) -> Result<Response, ContractError> {
    let account = account_from_address(info.sender.as_ref(), deps.storage, deps.api)?;
    account.try_head_kick_member(deps.storage, signature, &member)
}

/// Update locked_pledge_cap, the hard cap for staking/bonding with unvested tokens.
///
/// Callable by ADMIN only, see [instantiate].
pub fn try_update_locked_pledge_cap(
    address: String,
    cap: PledgeCap,
    info: MessageInfo,
    deps: DepsMut,
) -> Result<Response, ContractError> {
    if info.sender != ADMIN.load(deps.storage)? {
        return Err(ContractError::NotAdmin(info.sender.as_str().to_string()));
    }
    let mut account = account_from_address(&address, deps.storage, deps.api)?;

    account.pledge_cap = Some(cap);
    save_account(&account, deps.storage)?;
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
    let mixnet_contract_address = deps.api.addr_validate(&address)?;

    MIXNET_CONTRACT_ADDRESS.save(deps.storage, &mixnet_contract_address)?;
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
    if let Some(ref to_address) = to_address {
        if account_from_address(to_address, deps.storage, deps.api).is_ok() {
            // do not allow setting staking address to an existing account's address
            return Err(ContractError::StakingAccountExists(to_address.to_string()));
        }
    }

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

pub fn try_pledge_more(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    amount: Coin,
) -> Result<Response, ContractError> {
    let mix_denom = MIX_DENOM.load(deps.storage)?;
    let additional_pledge = validate_funds(&[amount], mix_denom)?;

    let account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;
    account.try_pledge_additional_tokens(additional_pledge, &env, deps.storage)
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
    mix_id: MixId,
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
    mix_id: MixId,
    amount: Coin,
    on_behalf_of: Option<String>,
    info: MessageInfo,
    env: Env,
    deps: DepsMut<'_>,
) -> Result<Response, ContractError> {
    let mix_denom = MIX_DENOM.load(deps.storage)?;
    let amount = validate_funds(&[amount], mix_denom)?;

    let account = match on_behalf_of {
        Some(account_owner) => {
            let account = account_from_address(&account_owner, deps.storage, deps.api)?;
            ensure_staking_permission(&info.sender, &account)?;
            account
        }
        // you're the owner, you can do what you want
        None => account_from_address(info.sender.as_str(), deps.storage, deps.api)?,
    };

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
    mix_id: MixId,
) -> Result<Response, ContractError> {
    let account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;

    account.try_claim_delegator_reward(mix_id, deps.storage)
}

/// Undelegates from a mixnode, sends [mixnet_contract_common::ExecuteMsg::UndelegateFromMixnodeOnBehalf] to [crate::storage::MIXNET_CONTRACT_ADDRESS].
fn try_undelegate_from_mixnode(
    mix_id: MixId,
    on_behalf_of: Option<String>,
    info: MessageInfo,
    deps: DepsMut<'_>,
) -> Result<Response, ContractError> {
    let account = match on_behalf_of {
        Some(account_owner) => {
            let account = account_from_address(&account_owner, deps.storage, deps.api)?;
            ensure_staking_permission(&info.sender, &account)?;
            account
        }
        // you're the owner, you can do what you want
        None => account_from_address(info.sender.as_str(), deps.storage, deps.api)?,
    };

    account.try_undelegate_from_mixnode(mix_id, deps.storage)
}

/// Creates a new periodic vesting account, and deposits funds to vest into the contract.
///
/// Callable by ADMIN only, see [instantiate].
pub(crate) fn try_create_periodic_vesting_account(
    owner_address: &str,
    staking_address: Option<String>,
    vesting_spec: Option<VestingSpecification>,
    cap: Option<PledgeCap>,
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
        let staking_account_exists =
            account_from_address(&staking_address, deps.storage, deps.api).is_ok();
        if staking_account_exists {
            return Err(ContractError::StakingAccountAlreadyExists(staking_address));
        }
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
        cap,
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
        QueryMsg::GetContractVersion {} => to_binary(&get_contract_version()),
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
        QueryMsg::GetDelegationTimes { address, mix_id } => {
            to_binary(&try_get_delegation_times(deps, &address, mix_id)?)
        }
        QueryMsg::GetAllDelegations { start_after, limit } => {
            to_binary(&try_get_all_delegations(deps, start_after, limit)?)
        }
    };

    Ok(query_res?)
}

/// Get current vesting period for a given [crate::vesting::Account].
pub fn try_get_current_vesting_period(
    address: &str,
    deps: Deps<'_>,
    env: Env,
) -> Result<Period, ContractError> {
    let account = account_from_address(address, deps.storage, deps.api)?;
    account.get_current_vesting_period(env.block.time)
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

/// Gets build information of this contract.
pub fn get_contract_version() -> ContractBuildInformation {
    // as per docs
    // env! macro will expand to the value of the named environment variable at
    // compile time, yielding an expression of type `&'static str`
    ContractBuildInformation {
        build_timestamp: env!("VERGEN_BUILD_TIMESTAMP").to_string(),
        build_version: env!("VERGEN_BUILD_SEMVER").to_string(),
        commit_sha: option_env!("VERGEN_GIT_SHA").unwrap_or("NONE").to_string(),
        commit_timestamp: option_env!("VERGEN_GIT_COMMIT_TIMESTAMP")
            .unwrap_or("NONE")
            .to_string(),
        commit_branch: option_env!("VERGEN_GIT_BRANCH")
            .unwrap_or("NONE")
            .to_string(),
        rustc_version: env!("VERGEN_RUSTC_SEMVER").to_string(),
    }
}

pub fn try_get_all_accounts(
    deps: Deps<'_>,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<AccountsResponse, ContractError> {
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
) -> Result<VestingCoinsResponse, ContractError> {
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
    account.get_original_vesting()
}

pub fn try_get_historical_vesting_staking_reward(
    vesting_account_address: &str,
    deps: Deps<'_>,
) -> Result<Coin, ContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    account.get_historical_vested_staking_rewards(deps.storage)
}

pub fn try_get_spendable_vested_coins(
    vesting_account_address: &str,
    deps: Deps<'_>,
    env: Env,
) -> Result<Coin, ContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    account.spendable_vested_coins(None, &env, deps.storage)
}

pub fn try_get_spendable_reward_coins(
    vesting_account_address: &str,
    deps: Deps<'_>,
    env: Env,
) -> Result<Coin, ContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    account.spendable_reward_coins(None, &env, deps.storage)
}

pub fn try_get_delegated_coins(
    vesting_account_address: &str,
    deps: Deps<'_>,
) -> Result<Coin, ContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    let denom = MIX_DENOM.load(deps.storage)?;
    let amount = account.total_delegations(deps.storage)?;
    Ok(Coin { denom, amount })
}

pub fn try_get_pledged_coins(
    vesting_account_address: &str,
    deps: Deps<'_>,
) -> Result<Coin, ContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    let denom = MIX_DENOM.load(deps.storage)?;
    let amount = account.total_pledged(deps.storage)?;
    Ok(Coin { denom, amount })
}

pub fn try_get_staked_coins(
    vesting_account_address: &str,
    deps: Deps<'_>,
) -> Result<Coin, ContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    let denom = MIX_DENOM.load(deps.storage)?;
    let amount = account.total_staked(deps.storage)?;
    Ok(Coin { denom, amount })
}

pub fn try_get_withdrawn_coins(
    vesting_account_address: &str,
    deps: Deps<'_>,
) -> Result<Coin, ContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    let denom = MIX_DENOM.load(deps.storage)?;
    let amount = account.load_withdrawn(deps.storage)?;
    Ok(Coin { denom, amount })
}

/// Returns timestamps at which delegations were made
pub fn try_get_delegation_times(
    deps: Deps<'_>,
    vesting_account_address: &str,
    mix_id: MixId,
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
    start_after: Option<(u32, MixId, BlockTimestampSecs)>,
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

fn ensure_staking_permission(addr: &Addr, account: &Account) -> Result<(), ContractError> {
    if let Some(staking_address) = account.staking_address() {
        if staking_address == addr {
            return Ok(());
        }
    }
    Err(ContractError::InvalidStakingAccount {
        address: addr.clone(),
        for_account: account.owner_address(),
    })
}
