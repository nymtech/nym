// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage::{self, DELEGATOR_REWARD_CLAIMED_HEIGHT};
use crate::constants;
use crate::delegations::storage as delegations_storage;
use crate::delegations::transactions::{_try_delegate_to_mixnode, try_delegate_to_mixnode};
use crate::error::ContractError;
use crate::interval::storage::{self as interval_storage};
use crate::mixnet_contract_settings::storage as mixnet_params_storage;
use crate::mixnodes::storage::mixnodes;
use crate::mixnodes::storage::{self as mixnodes_storage, StoredMixnodeBond};
use crate::rewards::helpers;
use config::defaults::DENOM;
use cosmwasm_std::{
    Addr, Api, Coin, DepsMut, Env, MessageInfo, Order, Response, StdResult, Storage, Uint128,
};
use cw_storage_plus::{Bound, PrimaryKey};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::events::{
    new_compound_delegator_reward_event, new_compound_operator_reward_event,
    new_mix_delegators_rewarding_event, new_mix_operator_rewarding_event,
    new_not_found_mix_operator_rewarding_event, new_too_fresh_bond_mix_operator_rewarding_event,
    new_zero_uptime_mix_operator_rewarding_event,
};
use mixnet_contract_common::mixnode::{DelegatorRewardParams, StoredNodeRewardResult};
use mixnet_contract_common::reward_params::{
    IntervalRewardParams, NodeEpochRewards, NodeRewardParams, RewardParams,
};
use mixnet_contract_common::{
    Delegation, IdentityKey, RewardingStatus, MIXNODE_DELEGATORS_PAGE_LIMIT,
};

// FIXME: Clean up
#[allow(unused_imports)]
use mixnet_contract_common::RewardingResult;

// #[derive(Debug)]
// struct MixDelegationRewardingResult {
//     total_rewarded: Uint128,
//     start_next: Option<Addr>,
// }

pub fn try_compound_operator_reward_on_behalf(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    owner: String,
) -> Result<Response, ContractError> {
    let proxy = deps.api.addr_validate(info.sender.as_str())?;
    let owner = deps.api.addr_validate(&owner)?;

    let reward =
        _try_compound_operator_reward(deps.storage, env.block.height, &owner, Some(proxy))?;

    Ok(Response::new().add_event(new_compound_operator_reward_event(&owner, reward)))
}

pub fn try_compound_operator_reward(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let owner = deps.api.addr_validate(info.sender.as_str())?;
    let reward = _try_compound_operator_reward(deps.storage, env.block.height, &owner, None)?;

    Ok(Response::new().add_event(new_compound_operator_reward_event(&owner, reward)))
}

pub fn _try_compound_operator_reward(
    storage: &mut dyn Storage,
    block_height: u64,
    owner: &Addr,
    proxy: Option<Addr>,
) -> Result<Uint128, ContractError> {
    let bond = match mixnodes().idx.owner.item(storage, owner.to_owned())? {
        Some(record) => record.1,
        None => {
            // Return if bond does not exist
            return Ok(Uint128::zero());
        }
    };

    if bond.proxy != proxy {
        // Return if proxy is not the same as the bond proxy
        return Ok(Uint128::zero());
    }

    let mut updated_bond = bond.clone();
    let reward = calculate_operator_reward(storage, owner, &bond)?;
    updated_bond.accumulated_rewards -= reward;
    updated_bond.pledge_amount.amount += reward;
    mixnodes().replace(
        storage,
        bond.identity(),
        Some(&updated_bond),
        Some(&bond),
        block_height,
    )?;

    DELEGATOR_REWARD_CLAIMED_HEIGHT.save(
        storage,
        (bond.identity().to_string(), owner.to_string()),
        &block_height,
    )?;

    Ok(reward)
}

fn calculate_operator_reward(
    storage: &dyn Storage,
    owner: &Addr,
    bond: &StoredMixnodeBond,
) -> Result<Uint128, ContractError> {
    let last_claimed_height = storage::OPERATOR_REWARD_CLAIMED_HEIGHT
        .load(storage, (owner.to_string(), bond.identity().to_string()))
        .unwrap_or(0);

    let accumulated_rewards = mixnodes()
        .changelog()
        .prefix(&bond.identity().to_string())
        .keys(storage, None, None, Order::Ascending)
        .filter_map(|height| height.ok())
        .filter(|height| last_claimed_height <= *height)
        .fold(
            Ok(Uint128::zero()),
            |acc, height| -> Result<Uint128, ContractError> {
                let accumulated_reward = acc?;
                if let Some(bond) =
                    mixnodes().may_load_at_height(storage, bond.identity().as_str(), height)?
                {
                    if let Some(epoch_rewards) = bond.epoch_rewards {
                        // Compound rewards from previous heights
                        let reward_at_height = epoch_rewards.delegation_reward(
                            bond.pledge_amount().amount + accumulated_reward,
                            bond.profit_margin(),
                        )?;
                        return Ok(accumulated_reward + reward_at_height);
                    }
                };
                Ok(accumulated_reward)
            },
        )?;
    Ok(accumulated_rewards)
}

// TODO:
// Add reward collection to undeleagation

// calculate_delegator_reward
// - figure out when is the last time reward go claimed
// - calculate current rewards
// compound_delegator_reward
// - decrease node accumulated rewards
// - increase delegation

pub fn try_compound_delegator_reward_on_behalf(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    owner: String,
    mix_identity: IdentityKey,
) -> Result<Response, ContractError> {
    let proxy = deps.api.addr_validate(info.sender.as_str())?;
    let owner = deps.api.addr_validate(&owner)?;
    let reward = _try_compound_delegator_reward(
        env.block.height,
        deps.api,
        deps.storage,
        owner.as_str(),
        &mix_identity,
        Some(proxy.clone()),
    )?;

    Ok(
        Response::new().add_event(new_compound_delegator_reward_event(
            &owner,
            &Some(proxy),
            reward,
            &mix_identity,
        )),
    )
}

pub fn try_compound_delegator_reward(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    mix_identity: IdentityKey,
) -> Result<Response, ContractError> {
    let owner = deps.api.addr_validate(info.sender.as_str())?;
    let reward = _try_compound_delegator_reward(
        env.block.height,
        deps.api,
        deps.storage,
        owner.as_str(),
        &mix_identity,
        None,
    )?;

    Ok(
        Response::new().add_event(new_compound_delegator_reward_event(
            &info.sender,
            &None,
            reward,
            &mix_identity,
        )),
    )
}

pub fn _try_compound_delegator_reward(
    block_height: u64,
    api: &dyn Api,
    storage: &mut dyn Storage,
    owner_address: &str,
    mix_identity: &str,
    proxy: Option<Addr>,
) -> Result<Uint128, ContractError> {
    let reward = calculate_delegator_reward(storage, owner_address, mix_identity)?;
    match _try_delegate_to_mixnode(
        storage,
        api,
        block_height,
        mix_identity,
        owner_address,
        Coin {
            amount: reward,
            denom: DENOM.to_string(),
        },
        proxy,
    ) {
        // Node exists all is well, life goes on
        Ok(_) => {
            if let Some(mut bond) = mixnodes().may_load(storage, mix_identity)? {
                bond.accumulated_rewards -= reward;
                mixnodes().save(storage, mix_identity, &bond, block_height)?;
            }
        }
        // Its fine to fail here as it means there is no bond to delegate to, we'll just return the reward to the caller
        Err(_) => {}
    };

    DELEGATOR_REWARD_CLAIMED_HEIGHT.save(
        storage,
        (mix_identity.to_string(), owner_address.to_string()),
        &block_height,
    )?;

    Ok(reward)
}

// TODO: Test
// + last_reward_claimed_height is updated
// + last_reward_claimed height is correctly used
fn calculate_delegator_reward(
    storage: &dyn Storage,
    owner_address: &str,
    mix_identity: &str,
) -> Result<Uint128, ContractError> {
    let last_claimed_height = storage::DELEGATOR_REWARD_CLAIMED_HEIGHT
        .load(
            storage,
            (owner_address.to_string(), mix_identity.to_string()),
        )
        .unwrap_or(0);

    // Get delegations newer then last_claimed_height, it would be nice to also fold this into the iteration bellow but it should be ok for now, as
    // I doubt folks refresh their delegations often
    let delegations = delegations_storage::delegations()
        .prefix((mix_identity.to_string(), owner_address.as_bytes().to_vec()))
        .range(storage, None, None, Order::Descending)
        .filter_map(|record| record.ok())
        .filter(|(height, _)| last_claimed_height <= *height)
        .map(|(_, delegation)| delegation)
        .collect::<Vec<Delegation>>();

    // This is a bit gnarly, but we want to avoid loading all heights, the loading mixnodes, so we're doing it all in the iterator
    let accumulated_rewards = mixnodes()
        .changelog()
        .prefix(mix_identity)
        .keys(storage, None, None, Order::Ascending)
        .filter_map(|height| height.ok())
        .filter(|height| last_claimed_height <= *height)
        .fold(
            Ok(Uint128::zero()),
            |acc, height| -> Result<Uint128, ContractError> {
                let accumulated_reward = acc?;
                let delegation_at_height = delegations
                    .iter()
                    .filter(|d| height <= d.block_height)
                    .fold(Uint128::zero(), |total, delegation| {
                        total + delegation.amount.amount
                    });
                if delegation_at_height != Uint128::zero() {
                    if let Some(bond) =
                        mixnodes().may_load_at_height(storage, mix_identity, height)?
                    {
                        if let Some(epoch_rewards) = bond.epoch_rewards {
                            // Compound rewards from previous heights
                            let reward_at_height = epoch_rewards.delegation_reward(
                                delegation_at_height + accumulated_reward,
                                bond.profit_margin(),
                            )?;
                            return Ok(accumulated_reward + reward_at_height);
                        }
                    }
                };
                Ok(accumulated_reward)
            },
        )?;

    Ok(accumulated_rewards)
}

/// Checks whether under the current context, any rewarding-related functionalities can be called.
/// The following must be true:
/// - the call has originated from the address of the authorised rewarding validator,
/// - the call has been made with the nonce corresponding to the current rewarding procedure,
///
/// # Arguments
///
/// * `storage`: reference (kinda) to the underlying storage pool of the contract used to read the current state
/// * `info`: contains the essential info for authorization, such as identity of the call
/// * `interval_id`: expected id of the current interval sent alongside the call
fn verify_rewarding_state(
    storage: &dyn Storage,
    info: MessageInfo,
    interval_id: u32,
) -> Result<(), ContractError> {
    let state = mixnet_params_storage::CONTRACT_STATE.load(storage)?;

    // check if this is executed by the permitted validator, if not reject the transaction
    if info.sender != state.rewarding_validator_address {
        return Err(ContractError::Unauthorized);
    }

    let current_interval = interval_storage::current_interval(storage)?;

    // make sure the transaction is sent for the correct interval
    // (guard ourselves against somebody trying to send stale results;
    // realistically it's never going to happen in a single rewarding validator case
    if interval_id != current_interval.id() {
        Err(ContractError::InvalidIntervalId {
            received: interval_id,
            expected: current_interval.id(),
        })
    } else {
        Ok(())
    }
}

// fn reward_mix_delegators(
//     storage: &mut dyn Storage,
//     mix_identity: IdentityKey,
//     start: Option<Addr>,
//     params: DelegatorRewardParams,
// ) -> StdResult<MixDelegationRewardingResult> {
//     // TODO: some checks to make sure stuff is not TOO stale.

//     let chunk_size = MIXNODE_DELEGATORS_PAGE_LIMIT;

//     let start_value =
//         start.map(|start| Bound::Inclusive((mix_identity.clone(), start).joined_key()));

//     let delegations = delegations_storage::delegations();

//     let mut total_rewarded = Uint128::zero();
//     let mut items = 0;

//     // keep track of the last iterated address so that we'd known what is the starting point
//     // of the next call (if required)
//     let mut start_next = None;

//     // I really hate this intermediate allocation, but I don't think there's a nice
//     // way around it as we need to have immutable borrow into the bucket to retrieve delegation
//     // itself and then we need a mutable one to insert an updated one back
//     let mut rewarded_delegations = Vec::new();

//     // get `chunk_size` + 1 of delegations
//     // we get the additional one to know the optional starting point of the next call
//     // TODO: optimization for the future: we're reading 1 additional item than what's strictly
//     // required for this transaction thus slightly increasing the gas costs.
//     // however this makes the logic slightly simpler (I hope)
//     // Note: we can't just return last key of `chunk_size` with appended 0 byte as that
//     // would not be a valid utf8 string
//     for delegation in delegations
//         .idx
//         .mixnode
//         .prefix(mix_identity)
//         .range(storage, start_value, None, cosmwasm_std::Order::Ascending)
//         .take(chunk_size + 1)
//     {
//         items += 1;

//         let (_pk, mut delegation) = delegation?;

//         if items == chunk_size + 1 {
//             // we shouldn't process this data, it's for the next call
//             start_next = Some(delegation.owner());
//             break;
//         } else {
//             // and for each of them increase the stake proportionally to the reward
//             // if at least `MINIMUM_BLOCK_AGE_FOR_REWARDING` blocks have been created
//             // since they delegated
//             if delegation.block_height + constants::MINIMUM_BLOCK_AGE_FOR_REWARDING
//                 <= params.node_reward_params().reward_blockstamp()
//             {
//                 let reward = params.determine_delegation_reward(delegation.amount.amount);
//                 delegation.increment_amount(Uint128::new(reward), None);
//                 total_rewarded += Uint128::new(reward);

//                 rewarded_delegations.push(delegation);
//             }
//         }
//     }

//     // finally save all delegation data back into the storage
//     for rewarded_delegation in rewarded_delegations {
//         let storage_key = rewarded_delegation.storage_key().joined_key();
//         delegations.save(storage, storage_key, &rewarded_delegation)?;
//     }

//     Ok(MixDelegationRewardingResult {
//         total_rewarded,
//         start_next,
//     })
// }

// FIXME: Delete after refactoring done
// pub(crate) fn try_reward_next_mixnode_delegators(
//     deps: DepsMut<'_>,
//     info: MessageInfo,
//     mix_identity: IdentityKey,
//     interval_id: u32,
// ) -> Result<Response, ContractError> {
//     verify_rewarding_state(deps.storage, info, interval_id)?;

//     match storage::REWARDING_STATUS.may_load(deps.storage, (interval_id, mix_identity.clone()))? {
//         None => {
//             // we haven't called 'regular' try_reward_mixnode, i.e. the operator itself
//             // was not rewarded yet
//             Err(ContractError::MixnodeOperatorNotRewarded {
//                 identity: mix_identity,
//             })
//         }
//         Some(RewardingStatus::Complete(_)) => {
//             // rewarding of this mixnode operator and all of its delegators has already been completed
//             Err(ContractError::MixnodeAlreadyRewarded {
//                 identity: mix_identity,
//             })
//         }
//         Some(RewardingStatus::PendingNextDelegatorPage(next_page_info)) => {
//             let delegation_rewarding_result = reward_mix_delegators(
//                 deps.storage,
//                 mix_identity.clone(),
//                 Some(next_page_info.next_start),
//                 next_page_info.rewarding_params,
//             )?;

//             helpers::update_post_rewarding_storage(
//                 deps.storage,
//                 &mix_identity,
//                 Uint128::zero(),
//                 delegation_rewarding_result.total_rewarded,
//             )?;

//             let mut rewarding_results = next_page_info.running_results;
//             rewarding_results.total_delegator_reward += delegation_rewarding_result.total_rewarded;

//             let round_increase = delegation_rewarding_result.total_rewarded;
//             let more_delegators = delegation_rewarding_result.start_next.is_some();

//             helpers::update_rewarding_status(
//                 deps.storage,
//                 interval_id,
//                 mix_identity.clone(),
//                 rewarding_results,
//                 delegation_rewarding_result.start_next,
//                 next_page_info.rewarding_params,
//             )?;

//             Ok(
//                 Response::new().add_event(new_mix_delegators_rewarding_event(
//                     interval_id,
//                     &mix_identity,
//                     round_increase,
//                     more_delegators,
//                 )),
//             )
//         }
//     }
// }

pub(crate) fn try_reward_mixnode(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    mix_identity: IdentityKey,
    params: RewardParams,
    interval_id: u32,
) -> Result<Response, ContractError> {
    verify_rewarding_state(deps.storage, info, interval_id)?;

    // check if the mixnode hasn't been rewarded in this rewarding interval already
    match storage::REWARDING_STATUS.may_load(deps.storage, (interval_id, mix_identity.clone()))? {
        None => (),
        Some(RewardingStatus::Complete(_)) => {
            return Err(ContractError::MixnodeAlreadyRewarded {
                identity: mix_identity,
            })
        }
        Some(RewardingStatus::PendingNextDelegatorPage(_)) => {
            return Err(ContractError::DelegatorsPendingReward {
                identity: mix_identity,
            })
        }
    }

    // check if the bond even exists
    let mut current_bond =
        match mixnodes_storage::read_full_mixnode_bond(deps.storage, &mix_identity)? {
            Some(bond) => bond,
            None => {
                return Ok(
                    Response::new().add_event(new_not_found_mix_operator_rewarding_event(
                        interval_id,
                        &mix_identity,
                    )),
                )
            }
        };

    // check if node is old enough for rewarding
    if current_bond.block_height + constants::MINIMUM_BLOCK_AGE_FOR_REWARDING > env.block.height {
        storage::REWARDING_STATUS.save(
            deps.storage,
            (interval_id, mix_identity.clone()),
            &RewardingStatus::Complete(Default::default()),
        )?;

        return Ok(
            Response::new().add_event(new_too_fresh_bond_mix_operator_rewarding_event(
                interval_id,
                &mix_identity,
            )),
        );
    }

    let node_pledge = current_bond.pledge_amount.amount;
    let node_delegation = current_bond.total_delegation.amount;

    // check if it has non-zero uptime
    if params.uptime() == 0 {
        storage::REWARDING_STATUS.save(
            deps.storage,
            (interval_id, mix_identity.clone()),
            &RewardingStatus::Complete(Default::default()),
        )?;

        return Ok(
            Response::new().add_event(new_zero_uptime_mix_operator_rewarding_event(
                interval_id,
                &mix_identity,
            )),
        );
    }

    let mut node_reward_params = params;
    node_reward_params.set_reward_blockstamp(env.block.height);

    let node_reward_result = current_bond.reward(&node_reward_params);
    let stored_node_result: StoredNodeRewardResult = node_reward_result.try_into()?;

    current_bond.accumulated_rewards += stored_node_result.reward();
    let mut stored_bond: StoredMixnodeBond = current_bond.into();
    // technically we don't have to set the total_delegation bucket, but it makes things easier
    // in different places that we can guarantee that if node exists, so does the data behind the total delegation
    let identity = stored_bond.identity();
    crate::mixnodes::storage::mixnodes().save(
        deps.storage,
        identity,
        &stored_bond,
        env.block.height,
    )?;

    stored_bond.epoch_rewards = Some(NodeEpochRewards::new(
        node_reward_params,
        stored_node_result,
    ));

    // Take rewards out of the rewarding pool
    storage::decr_reward_pool(deps.storage, stored_node_result.reward())?;

    // FIXME: Delete commented code, after introducing lazy reward claims
    // INFO: No eager rewarding for operators and delegators anymore
    // Omitting the price per packet function now, it follows that base operator reward is the node_reward
    // let operator_reward = Uint128::new(current_bond.operator_reward(&node_reward_params));

    // let delegator_params = DelegatorRewardParams::new(&current_bond, node_reward_params);
    // let delegation_rewarding_result =
    //     reward_mix_delegators(deps.storage, mix_identity.clone(), None, delegator_params)?;

    // helpers::update_post_rewarding_storage(
    //     deps.storage,
    //     &mix_identity,
    //     operator_reward,
    //     delegation_rewarding_result.total_rewarded,
    // )?;

    let rewarding_result = RewardingResult {
        node_reward: stored_node_result.reward(),
    };
    // let total_delegator_reward = rewarding_results.total_delegator_reward;
    // let further_delegations = delegation_rewarding_result.start_next.is_some();

    helpers::update_rewarding_status(
        deps.storage,
        interval_id,
        mix_identity.clone(),
        rewarding_result,
    )?;

    Ok(Response::new().add_event(new_mix_operator_rewarding_event(
        interval_id,
        &mix_identity,
        node_reward_result,
        node_pledge,
        node_delegation,
    )))
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::constants::SYBIL_RESISTANCE_PERCENT;
    use crate::delegations::transactions::try_delegate_to_mixnode;
    use crate::error::ContractError;
    use crate::mixnet_contract_settings::storage as mixnet_params_storage;
    use crate::mixnodes::storage as mixnodes_storage;
    use crate::mixnodes::storage::StoredMixnodeBond;
    use crate::rewards::transactions::try_reward_mixnode;
    use crate::support::tests;
    use crate::support::tests::test_helpers;
    use az::CheckedCast;
    use config::defaults::DENOM;
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::Coin;
    use cosmwasm_std::Order;
    use cosmwasm_std::{coin, coins, Addr, Uint128};
    use mixnet_contract_common::events::{
        must_find_attribute, BOND_TOO_FRESH_VALUE, DISTRIBUTED_DELEGATION_REWARDS_KEY,
        FURTHER_DELEGATIONS_TO_REWARD_KEY, NO_REWARD_REASON_KEY, OPERATOR_REWARDING_EVENT_TYPE,
        OPERATOR_REWARD_KEY, TOTAL_MIXNODE_REWARD_KEY,
    };
    use mixnet_contract_common::reward_params::{
        IntervalRewardParams, NodeRewardParams, RewardParams,
    };
    use mixnet_contract_common::{Delegation, IdentityKey, Layer, MixNode};

    #[test]
    fn rewarding_mixnodes_with_incorrect_interval_id() {
        let mut deps = test_helpers::init_contract();
        let mut env = mock_env();
        let current_state = mixnet_params_storage::CONTRACT_STATE
            .load(deps.as_mut().storage)
            .unwrap();
        let rewarding_validator_address = current_state.rewarding_validator_address;

        // bond the node
        let node_owner: Addr = Addr::unchecked("node-owner");
        let node_identity = test_helpers::add_mixnode(
            node_owner.as_str(),
            tests::fixtures::good_mixnode_pledge(),
            deps.as_mut(),
        );

        let info = mock_info(rewarding_validator_address.as_ref(), &[]);

        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            node_identity.clone(),
            tests::fixtures::rewarding_params_fixture(100),
            1,
        );
        assert_eq!(
            Err(ContractError::InvalidIntervalId {
                received: 1,
                expected: 0
            }),
            res
        );

        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            node_identity.clone(),
            tests::fixtures::rewarding_params_fixture(100),
            2,
        );
        assert_eq!(
            Err(ContractError::InvalidIntervalId {
                received: 2,
                expected: 0
            }),
            res
        );

        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            node_identity.clone(),
            tests::fixtures::rewarding_params_fixture(100),
            0,
        );
        assert!(res.is_ok());

        test_helpers::update_env_and_progress_interval(&mut env, deps.as_mut().storage);

        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info,
            node_identity,
            tests::fixtures::rewarding_params_fixture(100),
            1,
        );
        assert!(res.is_ok());
    }

    #[test]
    fn attempting_rewarding_mixnode_multiple_times_per_interval() {
        let mut deps = test_helpers::init_contract();
        let mut env = mock_env();
        let current_state = mixnet_params_storage::CONTRACT_STATE
            .load(deps.as_mut().storage)
            .unwrap();
        let rewarding_validator_address = current_state.rewarding_validator_address;

        // bond the node
        let node_owner: Addr = Addr::unchecked("node-owner");
        let node_identity = test_helpers::add_mixnode(
            node_owner.as_str(),
            tests::fixtures::good_mixnode_pledge(),
            deps.as_mut(),
        );

        let info = mock_info(rewarding_validator_address.as_ref(), &[]);

        // first reward goes through just fine
        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            node_identity.clone(),
            tests::fixtures::rewarding_params_fixture(100),
            0,
        );
        assert!(res.is_ok());

        // but the other one fails
        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            node_identity.clone(),
            tests::fixtures::rewarding_params_fixture(100),
            0,
        );
        assert_eq!(
            Err(ContractError::MixnodeAlreadyRewarded {
                identity: node_identity.clone()
            }),
            res
        );

        // but rewarding the same node in the following interval is fine again
        test_helpers::update_env_and_progress_interval(&mut env, deps.as_mut().storage);

        let res = try_reward_mixnode(
            deps.as_mut(),
            env,
            info,
            node_identity,
            tests::fixtures::rewarding_params_fixture(100),
            1,
        );
        assert!(res.is_ok());
    }

    #[test]
    fn rewarding_mixnode_blockstamp_based() {
        let mut deps = test_helpers::init_contract();
        let mut env = mock_env();
        let current_state = mixnet_params_storage::CONTRACT_STATE
            .load(deps.as_mut().storage)
            .unwrap();
        let rewarding_validator_address = current_state.rewarding_validator_address;

        let node_owner: Addr = Addr::unchecked("node-owner");
        let node_identity: IdentityKey = "nodeidentity".into();

        let initial_bond = 10000_000000;
        let initial_delegation = 20000_000000;
        let mixnode_bond = StoredMixnodeBond {
            pledge_amount: coin(initial_bond, DENOM),
            owner: node_owner,
            layer: Layer::One,
            block_height: env.block.height,
            mix_node: MixNode {
                identity_key: node_identity.clone(),
                ..tests::fixtures::mix_node_fixture()
            },
            proxy: None,
            accumulated_rewards: Uint128::zero(),
            epoch_rewards: None,
        };

        mixnodes_storage::mixnodes()
            .save(
                deps.as_mut().storage,
                &node_identity,
                &mixnode_bond,
                env.block.height,
            )
            .unwrap();
        mixnodes_storage::TOTAL_DELEGATION
            .save(
                deps.as_mut().storage,
                &node_identity,
                &Uint128::new(initial_delegation),
            )
            .unwrap();

        // delegation happens later, but not later enough
        env.block.height += constants::MINIMUM_BLOCK_AGE_FOR_REWARDING - 1;

        delegations_storage::delegations()
            .save(
                deps.as_mut().storage,
                (node_identity.clone(), "delegator".into(), env.block.height),
                &Delegation::new(
                    Addr::unchecked("delegator"),
                    node_identity.clone(),
                    coin(initial_delegation, DENOM),
                    env.block.height,
                    None,
                ),
            )
            .unwrap();

        let info = mock_info(rewarding_validator_address.as_ref(), &[]);

        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            node_identity.clone(),
            tests::fixtures::rewarding_params_fixture(100),
            0,
        )
        .unwrap();

        assert_eq!(
            initial_bond,
            test_helpers::read_mixnode_pledge_amount(deps.as_ref().storage, &node_identity)
                .unwrap()
                .u128()
        );
        assert_eq!(
            initial_delegation,
            mixnodes_storage::TOTAL_DELEGATION
                .load(deps.as_ref().storage, &node_identity)
                .unwrap()
                .u128()
        );
        assert_eq!(1, res.events.len());
        assert_eq!(OPERATOR_REWARDING_EVENT_TYPE, res.events[0].ty);
        assert_eq!(
            BOND_TOO_FRESH_VALUE,
            must_find_attribute(&res.events[0], NO_REWARD_REASON_KEY)
        );

        // reward can happen now, but only for bonded node
        env.block.height += 1;
        test_helpers::update_env_and_progress_interval(&mut env, deps.as_mut().storage);

        let info = mock_info(rewarding_validator_address.as_ref(), &[]);
        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            node_identity.clone(),
            tests::fixtures::rewarding_params_fixture(100),
            1,
        )
        .unwrap();

        let mixnode = crate::mixnodes::storage::mixnodes()
            .load(&deps.storage, &node_identity)
            .unwrap();

        assert!(mixnode.accumulated_rewards > Uint128::zero(),);
        assert_eq!(
            initial_delegation,
            mixnodes_storage::TOTAL_DELEGATION
                .load(deps.as_ref().storage, &node_identity)
                .unwrap()
                .u128()
        );

        assert_eq!(1, res.events.len());
        let event = &res.events[0];
        assert_eq!(OPERATOR_REWARDING_EVENT_TYPE, event.ty);
        // assert_ne!("0", must_find_attribute(event, TOTAL_MIXNODE_REWARD_KEY));
        // assert_ne!("0", must_find_attribute(event, OPERATOR_REWARD_KEY));
        // assert_eq!(
        //     "0",
        //     must_find_attribute(event, DISTRIBUTED_DELEGATION_REWARDS_KEY)
        // );
        // assert_eq!(
        //     false.to_string(),
        //     must_find_attribute(event, FURTHER_DELEGATIONS_TO_REWARD_KEY)
        // );

        // reward happens now, both for node owner and delegators
        env.block.height += constants::MINIMUM_BLOCK_AGE_FOR_REWARDING - 1;
        test_helpers::update_env_and_progress_interval(&mut env, deps.as_mut().storage);

        let pledge_before_rewarding =
            test_helpers::read_mixnode_pledge_amount(deps.as_ref().storage, &node_identity)
                .unwrap()
                .u128();

        let info = mock_info(rewarding_validator_address.as_ref(), &[]);
        let res = try_reward_mixnode(
            deps.as_mut(),
            env,
            info.clone(),
            node_identity.clone(),
            tests::fixtures::rewarding_params_fixture(100),
            2,
        )
        .unwrap();

        // We are in a lazy system, rewarding will not increase pledge or delegations
        assert_eq!(
            test_helpers::read_mixnode_pledge_amount(deps.as_ref().storage, &node_identity)
                .unwrap()
                .u128(),
            pledge_before_rewarding
        );
        assert_eq!(
            mixnodes_storage::TOTAL_DELEGATION
                .load(deps.as_ref().storage, &node_identity)
                .unwrap()
                .u128(),
            initial_delegation
        );

        assert_eq!(1, res.events.len());
        let event = &res.events[0];
        assert_eq!(OPERATOR_REWARDING_EVENT_TYPE, event.ty);
        // assert_ne!("0", must_find_attribute(event, TOTAL_MIXNODE_REWARD_KEY));
        // assert_ne!("0", must_find_attribute(event, OPERATOR_REWARD_KEY));
        // assert_ne!(
        //     "0",
        //     must_find_attribute(event, DISTRIBUTED_DELEGATION_REWARDS_KEY)
        // );
        // assert_eq!(
        //     false.to_string(),
        //     must_find_attribute(event, FURTHER_DELEGATIONS_TO_REWARD_KEY)
        // );
    }

    #[test]
    fn test_tokenomics_rewarding() {
        use crate::constants::INTERVAL_REWARD_PERCENT;
        use crate::contract::INITIAL_REWARD_POOL;

        type U128 = fixed::types::U75F53;

        let mut deps = test_helpers::init_contract();
        let mut env = mock_env();
        let current_state = mixnet_params_storage::CONTRACT_STATE
            .load(deps.as_ref().storage)
            .unwrap();
        let rewarding_validator_address = current_state.rewarding_validator_address;
        let period_reward_pool = (INITIAL_REWARD_POOL / 100) * INTERVAL_REWARD_PERCENT as u128;
        assert_eq!(period_reward_pool, 5_000_000_000_000);
        let rewarded_set_size = 200; // Imagining our reward set size is 200
        let active_set_size = 100;
        let active_set_work_factor = 10;
        let circulating_supply = storage::circulating_supply(&deps.storage).unwrap().u128();
        assert_eq!(circulating_supply, 750_000_000_000_000u128);

        let node_owner: Addr = Addr::unchecked("alice");
        let node_identity = test_helpers::add_mixnode(
            node_owner.as_str(),
            coins(10_000_000_000, DENOM),
            deps.as_mut(),
        );

        try_delegate_to_mixnode(
            deps.as_mut(),
            mock_env(),
            mock_info("alice_d1", &[coin(8000_000000, DENOM)]),
            node_identity.clone(),
        )
        .unwrap();

        try_delegate_to_mixnode(
            deps.as_mut(),
            mock_env(),
            mock_info("alice_d2", &[coin(2000_000000, DENOM)]),
            node_identity.clone(),
        )
        .unwrap();

        crate::delegations::transactions::_try_reconcile_all_delegation_events(
            &mut deps.storage,
            &deps.api,
        )
        .unwrap();

        let info = mock_info(rewarding_validator_address.as_ref(), &[]);
        env.block.height += 2 * constants::MINIMUM_BLOCK_AGE_FOR_REWARDING;

        let mix_1 = mixnodes_storage::read_full_mixnode_bond(&deps.storage, &node_identity)
            .unwrap()
            .unwrap();
        let mix_1_uptime = 100;

        let interval_reward_params = IntervalRewardParams::new(
            period_reward_pool,
            rewarded_set_size,
            active_set_size,
            circulating_supply,
            SYBIL_RESISTANCE_PERCENT,
            active_set_work_factor,
        );

        let node_reward_params = NodeRewardParams::new(0, mix_1_uptime, true);

        let mut params = RewardParams::new(interval_reward_params, node_reward_params);

        params.set_reward_blockstamp(env.block.height);

        assert_eq!(params.performance(), U128::from_num(1u32));

        let mix_1_reward_result = mix_1.reward(&params);

        assert_eq!(
            mix_1_reward_result.sigma(),
            U128::from_num(0.0000266666666666f64)
        );
        assert_eq!(
            mix_1_reward_result.lambda(),
            U128::from_num(0.0000133333333333f64)
        );
        assert_eq!(mix_1_reward_result.reward().int(), 186562237u128);

        let mix1_operator_reward = mix_1.operator_reward(&params);

        let mix1_delegator1_reward = mix_1.reward_delegation(Uint128::new(8000_000000), &params);

        let mix1_delegator2_reward = mix_1.reward_delegation(Uint128::new(2000_000000), &params);

        assert_eq!(mix1_operator_reward, 120609230);
        assert_eq!(mix1_delegator1_reward, 52762405);
        assert_eq!(mix1_delegator2_reward, 13190601);

        assert_eq!(
            mix1_operator_reward + mix1_delegator1_reward + mix1_delegator2_reward + 1,
            mix_1_reward_result.reward().int()
        );

        let pre_reward_bond =
            test_helpers::read_mixnode_pledge_amount(&deps.storage, &node_identity)
                .unwrap()
                .u128();
        assert_eq!(pre_reward_bond, 10_000_000_000);

        let pre_reward_delegation = mixnodes_storage::TOTAL_DELEGATION
            .load(&deps.storage, &node_identity)
            .unwrap()
            .u128();
        assert_eq!(pre_reward_delegation, 10_000_000_000);

        try_reward_mixnode(deps.as_mut(), env, info, node_identity.clone(), params, 0).unwrap();

        let mixnode = crate::mixnodes::storage::mixnodes()
            .load(&deps.storage, &node_identity)
            .unwrap();

        assert_eq!(
            test_helpers::read_mixnode_pledge_amount(&deps.storage, &node_identity)
                .unwrap()
                .u128()
                + mixnode.accumulated_rewards.u128(),
            pre_reward_bond
                + mix1_operator_reward
                + mix1_delegator1_reward
                + mix1_delegator2_reward
                + 1 // There is a rounding error here it seems
        );
        // assert_eq!(
        //     mixnodes_storage::TOTAL_DELEGATION
        //         .load(&deps.storage, &node_identity)
        //         .unwrap()
        //         .u128(),
        //     pre_reward_delegation + mix1_delegator1_reward + mix1_delegator2_reward
        // );

        assert_eq!(
            storage::REWARD_POOL.load(&deps.storage).unwrap().u128(),
            INITIAL_REWARD_POOL
                - (mix1_operator_reward + mix1_delegator1_reward + mix1_delegator2_reward)
                - 1 // Same rounding error, its 1 ucoin, it will manifest/correct when the rewards are claimed
        );

        // it's all correctly saved
        match storage::REWARDING_STATUS
            .load(deps.as_ref().storage, (0u32, node_identity))
            .unwrap()
        {
            RewardingStatus::Complete(result) => assert_eq!(
                RewardingResult {
                    node_reward: Uint128::new(mix_1_reward_result.reward().checked_cast().unwrap()),
                },
                result
            ),
            _ => unreachable!(),
        }
    }

    // #[test]
    // fn rewarding_mix_delegators_return_consistent_results() {
    //     // with single page
    //     let mut deps = test_helpers::init_contract();
    //     let mut env = mock_env();

    //     let node_owner: Addr = Addr::unchecked("bob");

    //     #[allow(clippy::inconsistent_digit_grouping)]
    //     let node_identity = test_helpers::add_mixnode(
    //         node_owner.as_str(),
    //         coins(10000_000_000, DENOM),
    //         deps.as_mut(),
    //     );

    //     let bond = mixnodes_storage::read_full_mixnode_bond(deps.as_ref().storage, &*node_identity)
    //         .unwrap()
    //         .unwrap();

    //     let base_delegation = 200_000000;
    //     let delegations = 123;

    //     for i in 0..delegations {
    //         try_delegate_to_mixnode(
    //             deps.as_mut(),
    //             env.clone(),
    //             mock_info(
    //                 &*format!("delegator{:04}", i),
    //                 &[coin(base_delegation, DENOM)],
    //             ),
    //             node_identity.clone(),
    //         )
    //         .unwrap();
    //     }

    //     env.block.height += constants::MINIMUM_BLOCK_AGE_FOR_REWARDING + 1;
    //     let mut node_rewarding_params = tests::fixtures::rewarding_params_fixture(100);
    //     node_rewarding_params.set_reward_blockstamp(env.block.height);

    //     let params = DelegatorRewardParams::new(&bond, node_rewarding_params);
    //     let res = reward_mix_delegators(deps.as_mut().storage, node_identity.clone(), None, params)
    //         .unwrap();

    //     let mut actual_reward = Uint128::new(0);
    //     for delegation in delegations_storage::delegations()
    //         .idx
    //         .mixnode
    //         .prefix(node_identity)
    //         .range(deps.as_ref().storage, None, None, Order::Ascending)
    //     {
    //         actual_reward +=
    //             Uint128::new(delegation.unwrap().1.amount.amount.u128() - base_delegation);
    //     }

    //     // sanity check to make sure we actually gave out any rewards
    //     assert_ne!(actual_reward, Uint128::zero());

    //     assert_eq!(actual_reward, res.total_rewarded);
    //     assert!(res.start_next.is_none());

    //     // with paging
    //     let node_owner: Addr = Addr::unchecked("alice");

    //     #[allow(clippy::inconsistent_digit_grouping)]
    //     let node_identity = test_helpers::add_mixnode(
    //         node_owner.as_str(),
    //         coins(10000_000_000, DENOM),
    //         deps.as_mut(),
    //     );

    //     let bond = mixnodes_storage::read_full_mixnode_bond(deps.as_ref().storage, &*node_identity)
    //         .unwrap()
    //         .unwrap();

    //     let base_delegation = 200_000000;
    //     let delegations = MIXNODE_DELEGATORS_PAGE_LIMIT + 123;

    //     for i in 0..delegations {
    //         try_delegate_to_mixnode(
    //             deps.as_mut(),
    //             env.clone(),
    //             mock_info(
    //                 &*format!("delegator{:04}", i),
    //                 &[coin(base_delegation, DENOM)],
    //             ),
    //             node_identity.clone(),
    //         )
    //         .unwrap();
    //     }

    //     env.block.height += constants::MINIMUM_BLOCK_AGE_FOR_REWARDING + 1;
    //     let mut node_rewarding_params = tests::fixtures::rewarding_params_fixture(100);
    //     node_rewarding_params.set_reward_blockstamp(env.block.height);

    //     let params = DelegatorRewardParams::new(&bond, node_rewarding_params);
    //     let res = reward_mix_delegators(deps.as_mut().storage, node_identity.clone(), None, params)
    //         .unwrap();

    //     let mut actual_reward = Uint128::new(0);
    //     for delegation in delegations_storage::delegations()
    //         .idx
    //         .mixnode
    //         .prefix(node_identity.clone())
    //         .range_raw(deps.as_ref().storage, None, None, Order::Ascending)
    //     {
    //         let (primary_key, delegation) = delegation.unwrap();
    //         let delegator_reward = Uint128::new(delegation.amount.amount.u128() - base_delegation);
    //         actual_reward += delegator_reward;

    //         // we start from index 2 as first 2 bytes are used to indicate length of first part
    //         // of the composite key
    //         let id_delegator = String::from_utf8_lossy(&primary_key[2..]);

    //         let delegator_id: usize = id_delegator
    //             .strip_prefix(&node_identity)
    //             .unwrap()
    //             .strip_prefix("delegator")
    //             .unwrap()
    //             .parse()
    //             .unwrap();
    //         if delegator_id >= MIXNODE_DELEGATORS_PAGE_LIMIT {
    //             // if they were in next page, they shouldn't have gotten anything!
    //             assert_eq!(Uint128::zero(), delegator_reward)
    //         } else {
    //             assert_ne!(Uint128::zero(), delegator_reward);
    //         }
    //     }

    //     assert_eq!(actual_reward, res.total_rewarded);
    //     let expected_next_page_start = format!("delegator{:04}", MIXNODE_DELEGATORS_PAGE_LIMIT);
    //     assert_eq!(expected_next_page_start, res.start_next.clone().unwrap());

    //     let res2 = reward_mix_delegators(
    //         deps.as_mut().storage,
    //         node_identity.clone(),
    //         res.start_next.clone(),
    //         params,
    //     )
    //     .unwrap();

    //     let start = res.start_next.unwrap();
    //     let mut actual_reward = Uint128::new(0);

    //     let start = Bound::Inclusive((node_identity.clone(), start).joined_key());
    //     for delegation in delegations_storage::delegations()
    //         .idx
    //         .mixnode
    //         .prefix(node_identity)
    //         .range(deps.as_ref().storage, Some(start), None, Order::Ascending)
    //     {
    //         actual_reward +=
    //             Uint128::new(delegation.unwrap().1.amount.amount.u128() - base_delegation);
    //     }

    //     assert_eq!(actual_reward, res2.total_rewarded);
    //     assert!(res2.start_next.is_none());
    // }

    #[cfg(test)]
    mod delegator_rewarding_tx {
        use super::*;

        // #[test]
        // fn can_only_be_called_by_specified_validator_address() {
        //     let mut deps = test_helpers::init_contract();

        //     let res = try_reward_next_mixnode_delegators(
        //         deps.as_mut(),
        //         mock_info("not-the-approved-validator", &[]),
        //         "alice's mixnode".to_string(),
        //         1,
        //     );
        //     assert_eq!(Err(ContractError::Unauthorized), res);
        // }

        // #[test]
        // fn cannot_be_called_if_mixnodes_operator_wasnt_rewarded() {
        //     let mut deps = test_helpers::init_contract();
        //     let current_state = mixnet_params_storage::CONTRACT_STATE
        //         .load(deps.as_mut().storage)
        //         .unwrap();
        //     let rewarding_validator_address = current_state.rewarding_validator_address;

        //     let res = try_reward_next_mixnode_delegators(
        //         deps.as_mut(),
        //         mock_info(rewarding_validator_address.as_ref(), &[]),
        //         "alice's mixnode".to_string(),
        //         0,
        //     );

        //     assert_eq!(
        //         Err(ContractError::MixnodeOperatorNotRewarded {
        //             identity: "alice's mixnode".to_string()
        //         }),
        //         res
        //     )
        // }

        #[test]
        fn cannot_be_called_if_mixnode_is_fully_rewarded() {
            // everything was done in a single reward call
            let mut deps = test_helpers::init_contract();
            let mut env = mock_env();
            let current_state = mixnet_params_storage::CONTRACT_STATE
                .load(deps.as_mut().storage)
                .unwrap();
            let rewarding_validator_address = current_state.rewarding_validator_address;

            let node_owner: Addr = Addr::unchecked("alice");

            #[allow(clippy::inconsistent_digit_grouping)]
            let node_identity = test_helpers::add_mixnode(
                node_owner.as_str(),
                coins(10000_000_000, DENOM),
                deps.as_mut(),
            );

            env.block.height += constants::MINIMUM_BLOCK_AGE_FOR_REWARDING;

            try_reward_mixnode(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                node_identity.clone(),
                tests::fixtures::rewarding_params_fixture(100),
                0,
            )
            .unwrap();

            // let res = try_reward_next_mixnode_delegators(
            //     deps.as_mut(),
            //     mock_info(rewarding_validator_address.as_ref(), &[]),
            //     node_identity.clone(),
            //     0,
            // );

            // assert_eq!(
            //     Err(ContractError::MixnodeAlreadyRewarded {
            //         identity: node_identity
            //     }),
            //     res
            // );

            // there was another page of delegators, but they were already dealt with
            let node_owner: Addr = Addr::unchecked("bob");

            #[allow(clippy::inconsistent_digit_grouping)]
            let node_identity = test_helpers::add_mixnode(
                node_owner.as_str(),
                coins(10000_000_000, DENOM),
                deps.as_mut(),
            );

            for i in 0..MIXNODE_DELEGATORS_PAGE_LIMIT + 1 {
                try_delegate_to_mixnode(
                    deps.as_mut(),
                    env.clone(),
                    mock_info(&*format!("delegator{:04}", i), &[coin(2000_000000, DENOM)]),
                    node_identity.clone(),
                )
                .unwrap();
            }

            env.block.height += constants::MINIMUM_BLOCK_AGE_FOR_REWARDING;
            test_helpers::update_env_and_progress_interval(&mut env, deps.as_mut().storage);

            let info = mock_info(rewarding_validator_address.as_ref(), &[]);
            try_reward_mixnode(
                deps.as_mut(),
                env,
                info,
                node_identity.clone(),
                tests::fixtures::rewarding_params_fixture(100),
                1,
            )
            .unwrap();

            // rewards all pending
            // try_reward_next_mixnode_delegators(
            //     deps.as_mut(),
            //     mock_info(rewarding_validator_address.as_ref(), &[]),
            //     node_identity.clone(),
            //     1,
            // )
            // .unwrap();

            // let res = try_reward_next_mixnode_delegators(
            //     deps.as_mut(),
            //     mock_info(rewarding_validator_address.as_ref(), &[]),
            //     node_identity.clone(),
            //     1,
            // );

            // assert_eq!(
            //     Err(ContractError::MixnodeAlreadyRewarded {
            //         identity: node_identity
            //     }),
            //     res
            // );
        }
    }
}
