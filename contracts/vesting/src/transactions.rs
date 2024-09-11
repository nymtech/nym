// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::contract::{ensure_staking_permission, validate_funds};
use crate::storage::{
    account_from_address, save_account, ADMIN, MIXNET_CONTRACT_ADDRESS, MIX_DENOM,
};
use crate::traits::{
    DelegatingAccount, GatewayBondingAccount, MixnodeBondingAccount, NodeFamilies, VestingAccount,
};
use crate::vesting::{populate_vesting_periods, StorableVestingAccountExt};
use contracts_common::signing::MessageSignature;
use cosmwasm_std::{coin, BankMsg, Coin, DepsMut, Env, MessageInfo, Response, Timestamp};
use mixnet_contract_common::families::FamilyHead;
use mixnet_contract_common::{
    Gateway, GatewayConfigUpdate, MixId, MixNode, MixNodeConfigUpdate, MixNodeCostParams,
};
use vesting_contract_common::events::{
    new_ownership_transfer_event, new_periodic_vesting_account_event,
    new_staking_address_update_event, new_track_gateway_unbond_event,
    new_track_migrate_mixnode_event, new_track_mixnode_pledge_decrease_event,
    new_track_mixnode_unbond_event, new_track_reward_event, new_track_undelegation_event,
    new_vested_coins_withdraw_event,
};
use vesting_contract_common::{Account, PledgeCap, VestingContractError, VestingSpecification};

pub fn try_create_family(
    info: MessageInfo,
    deps: DepsMut,
    label: String,
) -> Result<Response, VestingContractError> {
    let account = account_from_address(info.sender.as_ref(), deps.storage, deps.api)?;
    account.try_create_family(deps.storage, label)
}
pub fn try_join_family(
    info: MessageInfo,
    deps: DepsMut,
    join_permit: MessageSignature,
    family_head: FamilyHead,
) -> Result<Response, VestingContractError> {
    let account = account_from_address(info.sender.as_ref(), deps.storage, deps.api)?;
    account.try_join_family(deps.storage, join_permit, family_head)
}
pub fn try_leave_family(
    info: MessageInfo,
    deps: DepsMut,
    family_head: FamilyHead,
) -> Result<Response, VestingContractError> {
    let account = account_from_address(info.sender.as_ref(), deps.storage, deps.api)?;
    account.try_leave_family(deps.storage, family_head)
}
pub fn try_kick_family_member(
    info: MessageInfo,
    deps: DepsMut,
    member: String,
) -> Result<Response, VestingContractError> {
    let account = account_from_address(info.sender.as_ref(), deps.storage, deps.api)?;
    account.try_head_kick_member(deps.storage, &member)
}

/// Update locked_pledge_cap, the hard cap for staking/bonding with unvested tokens.
///
/// Callable by ADMIN only, see [instantiate].
pub fn try_update_locked_pledge_cap(
    address: String,
    cap: PledgeCap,
    info: MessageInfo,
    deps: DepsMut,
) -> Result<Response, VestingContractError> {
    if info.sender != ADMIN.load(deps.storage)? {
        return Err(VestingContractError::NotAdmin(
            info.sender.as_str().to_string(),
        ));
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
) -> Result<Response, VestingContractError> {
    let account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;
    account.try_update_mixnode_config(new_config, deps.storage)
}

pub fn try_update_gateway_config(
    new_config: GatewayConfigUpdate,
    info: MessageInfo,
    deps: DepsMut,
) -> Result<Response, VestingContractError> {
    let account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;
    account.try_update_gateway_config(new_config, deps.storage)
}

pub fn try_update_mixnode_cost_params(
    new_costs: MixNodeCostParams,
    info: MessageInfo,
    deps: DepsMut,
) -> Result<Response, VestingContractError> {
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
) -> Result<Response, VestingContractError> {
    if info.sender != ADMIN.load(deps.storage)? {
        return Err(VestingContractError::NotAdmin(
            info.sender.as_str().to_string(),
        ));
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
) -> Result<Response, VestingContractError> {
    let mix_denom = MIX_DENOM.load(deps.storage)?;
    if amount.denom != mix_denom {
        return Err(VestingContractError::WrongDenom(amount.denom, mix_denom));
    }

    let address = info.sender;
    let account = account_from_address(address.as_str(), deps.storage, deps.api)?;
    if address != account.owner_address() {
        return Err(VestingContractError::NotOwner(
            account.owner_address().to_string(),
        ));
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
        Err(VestingContractError::InsufficientSpendable(
            account.owner_address().as_str().to_string(),
            spendable_coins.amount.u128(),
        ))
    }
}

/// Transfer ownership of the entire vesting account.
pub fn try_transfer_ownership(
    to_address: String,
    info: MessageInfo,
    deps: DepsMut<'_>,
) -> Result<Response, VestingContractError> {
    let address = info.sender;
    let to_address = deps.api.addr_validate(&to_address)?;
    let mut account = account_from_address(address.as_str(), deps.storage, deps.api)?;
    if address == account.owner_address() {
        account.transfer_ownership(&to_address, deps.storage)?;
        Ok(Response::new().add_event(new_ownership_transfer_event(&address, &to_address)))
    } else {
        Err(VestingContractError::NotOwner(
            account.owner_address().to_string(),
        ))
    }
}

/// Set or update staking address for a vesting account.
pub fn try_update_staking_address(
    to_address: Option<String>,
    info: MessageInfo,
    deps: DepsMut<'_>,
) -> Result<Response, VestingContractError> {
    if let Some(ref to_address) = to_address {
        if account_from_address(to_address, deps.storage, deps.api).is_ok() {
            // do not allow setting staking address to an existing account's address
            return Err(VestingContractError::StakingAccountExists(
                to_address.to_string(),
            ));
        }
    }

    let address = info.sender;
    let to_address = to_address.and_then(|x| deps.api.addr_validate(&x).ok());
    let mut account = account_from_address(address.as_str(), deps.storage, deps.api)?;
    if address == account.owner_address() {
        let old = account.staking_address().cloned();
        account.update_staking_address(to_address.clone(), deps.storage)?;
        Ok(Response::new().add_event(new_staking_address_update_event(&old, &to_address)))
    } else {
        Err(VestingContractError::NotOwner(
            account.owner_address().to_string(),
        ))
    }
}

/// Bond a gateway, sends [mixnet_contract_common::ExecuteMsg::BondGatewayOnBehalf] to [crate::storage::MIXNET_CONTRACT_ADDRESS].
pub fn try_bond_gateway(
    gateway: Gateway,
    owner_signature: MessageSignature,
    amount: Coin,
    info: MessageInfo,
    env: Env,
    deps: DepsMut<'_>,
) -> Result<Response, VestingContractError> {
    let mix_denom = MIX_DENOM.load(deps.storage)?;
    let pledge = validate_funds(&[amount], mix_denom)?;
    let account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;
    account.try_bond_gateway(gateway, owner_signature, pledge, &env, deps.storage)
}

/// Unbond a gateway, sends [mixnet_contract_common::ExecuteMsg::UnbondGatewayOnBehalf] to [crate::storage::MIXNET_CONTRACT_ADDRESS].
pub fn try_unbond_gateway(
    info: MessageInfo,
    deps: DepsMut<'_>,
) -> Result<Response, VestingContractError> {
    let account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;
    account.try_unbond_gateway(deps.storage)
}

/// Track gateway unbonding, invoked by the mixnet contract after succesful unbonding, message containes coins returned including any accrued rewards.
pub fn try_track_unbond_gateway(
    owner: &str,
    amount: Coin,
    info: MessageInfo,
    deps: DepsMut<'_>,
) -> Result<Response, VestingContractError> {
    if info.sender != MIXNET_CONTRACT_ADDRESS.load(deps.storage)? {
        return Err(VestingContractError::NotMixnetContract(info.sender));
    }
    let account = account_from_address(owner, deps.storage, deps.api)?;
    account.try_track_unbond_gateway(amount, deps.storage)?;
    Ok(Response::new().add_event(new_track_gateway_unbond_event()))
}

/// Track vesting mixnode being converted into the usage of liquid tokens. invoked by the mixnet contract after successful migration.
pub fn try_track_migrate_mixnode(
    owner: &str,
    info: MessageInfo,
    deps: DepsMut<'_>,
) -> Result<Response, VestingContractError> {
    if info.sender != MIXNET_CONTRACT_ADDRESS.load(deps.storage)? {
        return Err(VestingContractError::NotMixnetContract(info.sender));
    }
    let account = account_from_address(owner, deps.storage, deps.api)?;
    account.try_track_migrated_mixnode(deps.storage)?;
    Ok(Response::new().add_event(new_track_migrate_mixnode_event()))
}

/// Track vesting delegation being converted into the usage of liquid tokens. invoked by the mixnet contract after successful migration.
pub fn try_track_migrate_delegation(
    owner: &str,
    mix_id: MixId,
    info: MessageInfo,
    deps: DepsMut<'_>,
) -> Result<Response, VestingContractError> {
    if info.sender != MIXNET_CONTRACT_ADDRESS.load(deps.storage)? {
        return Err(VestingContractError::NotMixnetContract(info.sender));
    }
    let account = account_from_address(owner, deps.storage, deps.api)?;
    account.track_migrated_delegation(mix_id, deps.storage)?;
    Ok(Response::new().add_event(new_track_migrate_mixnode_event()))
}

/// Bond a mixnode, sends [mixnet_contract_common::ExecuteMsg::BondMixnodeOnBehalf] to [crate::storage::MIXNET_CONTRACT_ADDRESS].
pub fn try_bond_mixnode(
    mix_node: MixNode,
    cost_params: MixNodeCostParams,
    owner_signature: MessageSignature,
    amount: Coin,
    info: MessageInfo,
    env: Env,
    deps: DepsMut<'_>,
) -> Result<Response, VestingContractError> {
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
) -> Result<Response, VestingContractError> {
    let mix_denom = MIX_DENOM.load(deps.storage)?;
    let additional_pledge = validate_funds(&[amount], mix_denom)?;

    let account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;
    account.try_pledge_additional_tokens(additional_pledge, &env, deps.storage)
}

pub fn try_decrease_pledge(
    deps: DepsMut<'_>,
    info: MessageInfo,
    amount: Coin,
) -> Result<Response, VestingContractError> {
    let mix_denom = MIX_DENOM.load(deps.storage)?;
    // perform basic validation - is it correct demon, is it non-zero, etc.
    let decrease = validate_funds(&[amount], mix_denom)?;

    let account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;
    account.try_decrease_mixnode_pledge(decrease, deps.storage)
}

/// Unbond a mixnode, sends [mixnet_contract_common::ExecuteMsg::UnbondMixnodeOnBehalf] to [crate::storage::MIXNET_CONTRACT_ADDRESS].
pub fn try_unbond_mixnode(
    info: MessageInfo,
    deps: DepsMut<'_>,
) -> Result<Response, VestingContractError> {
    let account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;
    account.try_unbond_mixnode(deps.storage)
}

/// Track mixnode unbonding, invoked by the mixnet contract after succesful unbonding, message containes coins returned including any accrued rewards.
pub fn try_track_unbond_mixnode(
    owner: &str,
    amount: Coin,
    info: MessageInfo,
    deps: DepsMut<'_>,
) -> Result<Response, VestingContractError> {
    if info.sender != MIXNET_CONTRACT_ADDRESS.load(deps.storage)? {
        return Err(VestingContractError::NotMixnetContract(info.sender));
    }
    let account = account_from_address(owner, deps.storage, deps.api)?;
    account.try_track_unbond_mixnode(amount, deps.storage)?;
    Ok(Response::new().add_event(new_track_mixnode_unbond_event()))
}

/// Tracks decreasing mixnode pledge. Invoked by the mixnet contract after successful event reconciliation.
/// A separate BankMsg containing the specified amount was sent in the same transaction.
pub fn try_track_decrease_mixnode_pledge(
    owner: &str,
    amount: Coin,
    info: MessageInfo,
    deps: DepsMut<'_>,
) -> Result<Response, VestingContractError> {
    if info.sender != MIXNET_CONTRACT_ADDRESS.load(deps.storage)? {
        return Err(VestingContractError::NotMixnetContract(info.sender));
    }
    let account = account_from_address(owner, deps.storage, deps.api)?;
    account.try_track_decrease_mixnode_pledge(amount, deps.storage)?;
    Ok(Response::new().add_event(new_track_mixnode_pledge_decrease_event()))
}

/// Track reward collection, invoked by the mixnert contract after sucessful reward compounding or claiming
pub fn try_track_reward(
    deps: DepsMut<'_>,
    info: MessageInfo,
    amount: Coin,
    address: &str,
) -> Result<Response, VestingContractError> {
    if info.sender != MIXNET_CONTRACT_ADDRESS.load(deps.storage)? {
        return Err(VestingContractError::NotMixnetContract(info.sender));
    }
    let account = account_from_address(address, deps.storage, deps.api)?;
    account.track_reward(amount, deps.storage)?;
    Ok(Response::new().add_event(new_track_reward_event()))
}

/// Track undelegation, invoked by the mixnet contract after sucessful undelegation, message contains coins returned with any accrued rewards.
pub fn try_track_undelegation(
    address: &str,
    mix_id: MixId,
    amount: Coin,
    info: MessageInfo,
    deps: DepsMut<'_>,
) -> Result<Response, VestingContractError> {
    if info.sender != MIXNET_CONTRACT_ADDRESS.load(deps.storage)? {
        return Err(VestingContractError::NotMixnetContract(info.sender));
    }
    let account = account_from_address(address, deps.storage, deps.api)?;

    account.track_undelegation(mix_id, amount, deps.storage)?;
    Ok(Response::new().add_event(new_track_undelegation_event()))
}

/// Delegate to mixnode, sends [mixnet_contract_common::ExecuteMsg::DelegateToMixnodeOnBehalf] to [crate::storage::MIXNET_CONTRACT_ADDRESS]..
pub fn try_delegate_to_mixnode(
    mix_id: MixId,
    amount: Coin,
    on_behalf_of: Option<String>,
    info: MessageInfo,
    env: Env,
    deps: DepsMut<'_>,
) -> Result<Response, VestingContractError> {
    // TODO
    // as of 01.02.23
    // thus restricting it to 25, which is more than double of that, doesn't seem too unreasonable.

    // while this might not be the best workaround, if user wishes to delegate more tokens towards the same node
    // they could remove the existing delegation (thus removing all separate entries from the storage)
    // and re-delegate it with the reclaimed amount (which will include all rewards).

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
pub fn try_claim_operator_reward(
    deps: DepsMut<'_>,
    info: MessageInfo,
) -> Result<Response, VestingContractError> {
    let account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;
    account.try_claim_operator_reward(deps.storage)
}

/// Claims delegator reward, sends [mixnet_contract_common::ExecuteMsg::ClaimDelegatorRewardOnBehalf] to [crate::storage::MIXNET_CONTRACT_ADDRESS].
pub fn try_claim_delegator_reward(
    deps: DepsMut<'_>,
    info: MessageInfo,
    mix_id: MixId,
) -> Result<Response, VestingContractError> {
    let account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;

    account.try_claim_delegator_reward(mix_id, deps.storage)
}

/// Undelegates from a mixnode, sends [mixnet_contract_common::ExecuteMsg::UndelegateFromMixnodeOnBehalf] to [crate::storage::MIXNET_CONTRACT_ADDRESS].
pub fn try_undelegate_from_mixnode(
    mix_id: MixId,
    on_behalf_of: Option<String>,
    info: MessageInfo,
    deps: DepsMut<'_>,
) -> Result<Response, VestingContractError> {
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
pub fn try_create_periodic_vesting_account(
    owner_address: &str,
    staking_address: Option<String>,
    vesting_spec: Option<VestingSpecification>,
    cap: Option<PledgeCap>,
    info: MessageInfo,
    env: Env,
    deps: DepsMut<'_>,
) -> Result<Response, VestingContractError> {
    if info.sender != ADMIN.load(deps.storage)? {
        return Err(VestingContractError::NotAdmin(
            info.sender.as_str().to_string(),
        ));
    }

    let mix_denom = MIX_DENOM.load(deps.storage)?;

    let account_exists = account_from_address(owner_address, deps.storage, deps.api).is_ok();
    if account_exists {
        return Err(VestingContractError::AccountAlreadyExists(
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
            return Err(VestingContractError::StakingAccountAlreadyExists(
                staking_address,
            ));
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

    Account::save_new(
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
