// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::ContractError;
use crate::storage::{
    config, config_read, decr_reward_pool, mix_delegations, mixnodes, mixnodes_read,
    rewarded_mixnodes, rewarded_mixnodes_read,
};
use crate::transactions::{MAX_REWARDING_DURATION_IN_BLOCKS, MINIMUM_BLOCK_AGE_FOR_REWARDING};
use cosmwasm_std::{attr, DepsMut, Env, MessageInfo, Response, StdResult, Storage, Uint128};
use mixnet_contract::mixnode::{DelegatorRewardParams, NodeRewardParams};
use mixnet_contract::{
    IdentityKey, IdentityKeyRef, PendingDelegatorRewarding, RewardingResult, RewardingStatus,
    MIXNODE_DELEGATORS_PAGE_LIMIT,
};

#[derive(Debug)]
struct MixDelegationRewardingResult {
    total_rewarded: Uint128,
    start_next: Option<String>,
}

// Note: this function is designed to work with only a single validator entity distributing rewards
// The main purpose of this function is to update `latest_rewarding_interval_nonce` which
// will trigger a different seed selection for the pseudorandom generation of the "demanded" set of mixnodes.
pub(crate) fn try_begin_mixnode_rewarding(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    rewarding_interval_nonce: u32,
) -> Result<Response, ContractError> {
    let mut state = config_read(deps.storage).load()?;

    // check if this is executed by the permitted validator, if not reject the transaction
    if info.sender != state.rewarding_validator_address {
        return Err(ContractError::Unauthorized);
    }

    // check whether sufficient number of blocks already elapsed since the previous rewarding happened
    // (this implies the validator responsible for rewarding in the previous interval did not call
    // `try_finish_mixnode_rewarding` - perhaps they crashed or something. Regardless of the reason
    // it shouldn't prevent anyone from distributing rewards in the following interval)
    // Do note, however, that calling `try_finish_mixnode_rewarding` is crucial as otherwise the
    // "demanded" set won't get updated on the validator API side
    if state.rewarding_in_progress
        && state.rewarding_interval_starting_block + MAX_REWARDING_DURATION_IN_BLOCKS
            > env.block.height
    {
        return Err(ContractError::RewardingInProgress);
    }

    // make sure the validator is in sync with the contract state
    if rewarding_interval_nonce != state.latest_rewarding_interval_nonce + 1 {
        return Err(ContractError::InvalidRewardingIntervalNonce {
            received: rewarding_interval_nonce,
            expected: state.latest_rewarding_interval_nonce + 1,
        });
    }

    state.rewarding_interval_starting_block = env.block.height;
    state.latest_rewarding_interval_nonce = rewarding_interval_nonce;
    state.rewarding_in_progress = true;

    config(deps.storage).save(&state)?;

    let mut response = Response::new();
    response.add_attribute(
        "rewarding interval nonce",
        rewarding_interval_nonce.to_string(),
    );
    Ok(response)
}

fn reward_mix_delegators_v2(
    storage: &mut dyn Storage,
    mix_identity: IdentityKeyRef,
    start: Option<String>,
    params: DelegatorRewardParams,
) -> StdResult<MixDelegationRewardingResult> {
    // TODO: some checks to make sure stuff is not TOO stale.

    let chunk_size = MIXNODE_DELEGATORS_PAGE_LIMIT;
    let start_value = start.as_ref().map(|addr| addr.as_bytes());

    let mut delegations = mix_delegations(storage, mix_identity);

    let mut total_rewarded = Uint128::zero();
    let mut items = 0;

    // keep track of the last iterated address so that we'd known what is the starting point
    // of the next call (if required)
    let mut start_next = None;

    // I really hate this intermediate allocation, but I don't think there's a nice
    // way around it as we need to have immutable borrow into the bucket to retrieve delegation
    // itself and then we need a mutable one to insert an updated one back
    let mut rewarded_delegations = Vec::new();

    // get `chunk_size` + 1 of delegations
    // we get the additional one to know the optional starting point of the next call
    // TODO: optimization for the future: we're reading 1 additional item than what's strictly
    // required for this transaction thus slightly increasing the gas costs.
    // however this makes the logic slightly simpler (I hope)
    // Note: we can't just return last key of `chunk_size` with appended 0 byte as that
    // would not be a valid utf8 string
    for delegation in delegations
        .range(start_value, None, cosmwasm_std::Order::Ascending)
        .take(chunk_size + 1)
    {
        items += 1;

        let (delegator_address, mut delegation) = delegation?;

        if items == chunk_size + 1 {
            // we shouldn't process this data, it's for the next call
            start_next = Some(String::from_utf8(delegator_address)?);
            break;
        } else {
            // and for each of them increase the stake proportionally to the reward
            // if at least `MINIMUM_BLOCK_AGE_FOR_REWARDING` blocks have been created
            // since they delegated
            if delegation.block_height + MINIMUM_BLOCK_AGE_FOR_REWARDING
                <= params.node_reward_params().reward_blockstamp()
            {
                let reward = params.determine_delegation_reward(delegation.amount);
                delegation.amount += Uint128(reward);
                total_rewarded += Uint128(reward);

                rewarded_delegations.push((delegator_address, delegation));
            }
        }
    }

    // finally save all delegation data back into the bucket
    for rewarded_delegation in rewarded_delegations {
        delegations.save(&rewarded_delegation.0, &rewarded_delegation.1)?;
    }

    Ok(MixDelegationRewardingResult {
        total_rewarded,
        start_next,
    })
}

/// Checks whether under the current context, any rewarding-related functionalities can be called.
/// The following must be true:
/// - the call has originated from the address of the authorised rewarding validator,
/// - the rewarding procedure has been initialised and has not concluded yet,
/// - the call has been made with the nonce corresponding to the current rewarding procedure,
///
/// # Arguments
///
/// * `storage`: reference (kinda) to the underlying storage pool of the contract used to read the current state
/// * `info`: contains the essential info for authorization, such as identity of the call
/// * `rewarding_interval_nonce`: nonce of the rewarding procedure sent alongside the call
fn verify_rewarding_state(
    storage: &dyn Storage,
    info: MessageInfo,
    rewarding_interval_nonce: u32,
) -> Result<(), ContractError> {
    let state = config_read(storage).load()?;

    // check if this is executed by the permitted validator, if not reject the transaction
    if info.sender != state.rewarding_validator_address {
        return Err(ContractError::Unauthorized);
    }

    // check if rewarding is currently in progress, if not reject the transaction
    if !state.rewarding_in_progress {
        return Err(ContractError::RewardingNotInProgress);
    }

    // make sure the transaction is sent for the correct rewarding interval
    // (guard ourselves against somebody trying to send stale results;
    // realistically it's never going to happen in a single rewarding validator case
    // but this check is not expensive (since we already had to read the state),
    // so we might as well)
    if rewarding_interval_nonce != state.latest_rewarding_interval_nonce {
        Err(ContractError::InvalidRewardingIntervalNonce {
            received: rewarding_interval_nonce,
            expected: state.latest_rewarding_interval_nonce,
        })
    } else {
        Ok(())
    }
}

pub(crate) fn try_reward_next_mixnode_delegators_v2(
    deps: DepsMut,
    info: MessageInfo,
    mix_identity: IdentityKey,
    rewarding_interval_nonce: u32,
) -> Result<Response, ContractError> {
    verify_rewarding_state(deps.storage, info, rewarding_interval_nonce)?;

    match rewarded_mixnodes_read(deps.storage, rewarding_interval_nonce)
        .may_load(mix_identity.as_bytes())?
    {
        None => {
            // we haven't called 'regular' try_reward_mixnode, i.e. the operator itself
            // was not rewarded yet
            Err(ContractError::MixnodeOperatorNotRewarded {
                identity: mix_identity,
            })
        }
        Some(RewardingStatus::Complete(_)) => {
            // rewarding of this mixnode operator and all of its delegators has already been completed
            Err(ContractError::MixnodeAlreadyRewarded {
                identity: mix_identity,
            })
        }
        Some(RewardingStatus::PendingNextDelegatorPage(next_page_info)) => {
            let delegation_rewarding_result = reward_mix_delegators_v2(
                deps.storage,
                &mix_identity,
                Some(next_page_info.next_start),
                next_page_info.rewarding_params,
            )?;

            // read current bond to update the memoised total delegation field
            let mut mixnodes = mixnodes(deps.storage);
            if let Some(mut current_bond) = mixnodes.may_load(mix_identity.as_bytes())? {
                // if the node unbonded, we don't have to worry about it.
                current_bond.total_delegation.amount += delegation_rewarding_result.total_rewarded;
                mixnodes.save(mix_identity.as_bytes(), &current_bond)?;
            }

            decr_reward_pool(delegation_rewarding_result.total_rewarded, deps.storage)?;

            let mut rewarding_results = next_page_info.running_results;
            rewarding_results.total_delegator_reward += delegation_rewarding_result.total_rewarded;

            let mut attributes = vec![(
                "current round delegation increase",
                delegation_rewarding_result.total_rewarded.to_string(),
            )];

            if let Some(next_start) = delegation_rewarding_result.start_next {
                attributes.push(("more delegators to reward", "true".to_owned()));

                rewarded_mixnodes(deps.storage, rewarding_interval_nonce).save(
                    mix_identity.as_bytes(),
                    &RewardingStatus::PendingNextDelegatorPage(PendingDelegatorRewarding {
                        running_results: rewarding_results,
                        next_start,
                        rewarding_params: next_page_info.rewarding_params,
                    }),
                )?;
            } else {
                attributes.push(("more delegators to reward", "false".to_owned()));

                rewarded_mixnodes(deps.storage, rewarding_interval_nonce).save(
                    mix_identity.as_bytes(),
                    &RewardingStatus::Complete(rewarding_results),
                )?;
            }

            let mut response = Response::new();
            // it looks kinda ugly now, but the API for this is vastly improved in cosmwasm 1.0
            for attribute in attributes {
                response.add_attribute(attribute.0, attribute.1)
            }

            Ok(response)
        }
    }
}

// Note: if any changes are made to this function or anything it is calling down the stack,
// for example delegation reward distribution, the gas limits must be retested and both
// validator-api/src/rewarding/mod.rs::{MIXNODE_REWARD_OP_BASE_GAS_LIMIT, PER_MIXNODE_DELEGATION_GAS_INCREASE}
// must be updated appropriately.
pub(crate) fn try_reward_mixnode_v2(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    mix_identity: IdentityKey,
    params: NodeRewardParams,
    rewarding_interval_nonce: u32,
) -> Result<Response, ContractError> {
    verify_rewarding_state(deps.storage, info, rewarding_interval_nonce)?;

    match rewarded_mixnodes_read(deps.storage, rewarding_interval_nonce)
        .may_load(mix_identity.as_bytes())?
    {
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

    // check if the mixnode hasn't been rewarded in this rewarding interval already
    if rewarded_mixnodes_read(deps.storage, rewarding_interval_nonce)
        .may_load(mix_identity.as_bytes())?
        .is_some()
    {
        return Err(ContractError::MixnodeAlreadyRewarded {
            identity: mix_identity,
        });
    }

    // check if the bond even exists
    let mut current_bond = match mixnodes_read(deps.storage).load(mix_identity.as_bytes()) {
        Ok(bond) => bond,
        Err(_) => {
            return Ok(Response {
                attributes: vec![attr("result", "bond not found")],
                ..Default::default()
            });
        }
    };

    // in cosmwasm 1.0 all attributes have to be T: Into<String> anyway
    let mut node_reward = "0".to_string();
    let mut operator_reward = Uint128::zero();
    let mut total_delegation_increase = Uint128::zero();
    let mut more_delegators = false;

    if current_bond.block_height + MINIMUM_BLOCK_AGE_FOR_REWARDING <= env.block.height {
        let mut node_reward_params = params;
        node_reward_params.set_reward_blockstamp(env.block.height);

        let operator_reward_result = current_bond.reward(&node_reward_params);
        node_reward = operator_reward_result.reward().to_string();

        // Omitting the price per packet function now, it follows that base operator reward is the node_reward
        operator_reward = Uint128(current_bond.operator_reward(&node_reward_params));

        let delegator_params = DelegatorRewardParams::new(&current_bond, node_reward_params);

        let delegation_rewarding_result =
            reward_mix_delegators_v2(deps.storage, &mix_identity, None, delegator_params)?;

        current_bond.bond_amount.amount += operator_reward;
        current_bond.total_delegation.amount += delegation_rewarding_result.total_rewarded;
        mixnodes(deps.storage).save(mix_identity.as_bytes(), &current_bond)?;
        decr_reward_pool(
            operator_reward + delegation_rewarding_result.total_rewarded,
            deps.storage,
        )?;

        let rewarding_results = RewardingResult {
            operator_reward,
            total_delegator_reward: delegation_rewarding_result.total_rewarded,
        };

        total_delegation_increase = rewarding_results.total_delegator_reward;

        if let Some(next_start) = delegation_rewarding_result.start_next {
            more_delegators = true;

            rewarded_mixnodes(deps.storage, rewarding_interval_nonce)
                .save(
                    mix_identity.as_bytes(),
                    &RewardingStatus::PendingNextDelegatorPage(PendingDelegatorRewarding {
                        running_results: rewarding_results,
                        next_start,
                        rewarding_params: delegator_params,
                    }),
                )
                .expect("blows up here");
        } else {
            rewarded_mixnodes(deps.storage, rewarding_interval_nonce).save(
                mix_identity.as_bytes(),
                &RewardingStatus::Complete(rewarding_results),
            )?;
        }
    } else {
        // node is not eligible for rewarding, so we're done immediately
        rewarded_mixnodes(deps.storage, rewarding_interval_nonce).save(
            mix_identity.as_bytes(),
            &RewardingStatus::Complete(Default::default()),
        )?;
    }

    Ok(Response {
        submessages: vec![],
        messages: vec![],
        attributes: vec![
            attr("node reward", node_reward),
            attr("operator reward", operator_reward),
            attr("total delegation increase", total_delegation_increase),
            attr("more delegators to reward", more_delegators),
        ],
        data: None,
    })
}

pub(crate) fn try_finish_mixnode_rewarding(
    deps: DepsMut,
    info: MessageInfo,
    rewarding_interval_nonce: u32,
) -> Result<Response, ContractError> {
    let mut state = config_read(deps.storage).load()?;

    // check if this is executed by the permitted validator, if not reject the transaction
    if info.sender != state.rewarding_validator_address {
        return Err(ContractError::Unauthorized);
    }

    if !state.rewarding_in_progress {
        return Err(ContractError::RewardingNotInProgress);
    }

    // make sure the validator is in sync with the contract state
    if rewarding_interval_nonce != state.latest_rewarding_interval_nonce {
        return Err(ContractError::InvalidRewardingIntervalNonce {
            received: rewarding_interval_nonce,
            expected: state.latest_rewarding_interval_nonce,
        });
    }

    state.rewarding_in_progress = false;
    config(deps.storage).save(&state)?;

    Ok(Response::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::DEFAULT_SYBIL_RESISTANCE_PERCENT;
    use crate::storage::{
        circulating_supply, mix_delegations_read, read_mixnode_bond, read_mixnode_delegation,
        reward_pool_value,
    };
    use crate::support::tests::helpers;
    use crate::support::tests::helpers::{
        good_mixnode_bond, mix_node_fixture, node_rewarding_params_fixture,
    };
    use crate::transactions::{try_add_mixnode, try_delegate_to_mixnode};
    use config::defaults::DENOM;
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::{coin, Addr, Coin, Order};
    use mixnet_contract::{Layer, MixNode, MixNodeBond, RawDelegationData};

    #[cfg(test)]
    mod beginning_mixnode_rewarding {
        use super::*;

        #[test]
        fn can_only_be_called_by_specified_validator_address() {
            let mut deps = helpers::init_contract();
            let env = mock_env();
            let current_state = config_read(deps.as_mut().storage).load().unwrap();
            let rewarding_validator_address = current_state.rewarding_validator_address;

            let res = try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info("not-the-approved-validator", &[]),
                1,
            );
            assert_eq!(Err(ContractError::Unauthorized), res);

            let res = try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            );
            assert!(res.is_ok())
        }

        #[test]
        fn cannot_be_called_if_rewarding_is_already_in_progress_with_little_day() {
            let mut deps = helpers::init_contract();
            let env = mock_env();
            let current_state = config_read(deps.as_mut().storage).load().unwrap();
            let rewarding_validator_address = current_state.rewarding_validator_address;

            try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            )
            .unwrap();

            let res = try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                2,
            );
            assert_eq!(Err(ContractError::RewardingInProgress), res);
        }

        #[test]
        fn can_be_called_if_rewarding_is_in_progress_if_sufficient_number_of_blocks_elapsed() {
            let mut deps = helpers::init_contract();
            let env = mock_env();
            let current_state = config_read(deps.as_mut().storage).load().unwrap();
            let rewarding_validator_address = current_state.rewarding_validator_address;

            try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            )
            .unwrap();

            let mut new_env = env.clone();

            new_env.block.height = env.block.height + MAX_REWARDING_DURATION_IN_BLOCKS;

            let res = try_begin_mixnode_rewarding(
                deps.as_mut(),
                new_env,
                mock_info(rewarding_validator_address.as_ref(), &[]),
                2,
            );
            assert!(res.is_ok());
        }

        #[test]
        fn provided_nonce_must_be_equal_the_current_plus_one() {
            let mut deps = helpers::init_contract();
            let env = mock_env();
            let mut current_state = config_read(deps.as_mut().storage).load().unwrap();
            current_state.latest_rewarding_interval_nonce = 42;
            config(deps.as_mut().storage).save(&current_state).unwrap();

            let rewarding_validator_address = current_state.rewarding_validator_address;

            let res = try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                11,
            );
            assert_eq!(
                Err(ContractError::InvalidRewardingIntervalNonce {
                    received: 11,
                    expected: 43
                }),
                res
            );

            let res = try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                44,
            );
            assert_eq!(
                Err(ContractError::InvalidRewardingIntervalNonce {
                    received: 44,
                    expected: 43
                }),
                res
            );

            let res = try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                42,
            );
            assert_eq!(
                Err(ContractError::InvalidRewardingIntervalNonce {
                    received: 42,
                    expected: 43
                }),
                res
            );

            let res = try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                43,
            );
            assert!(res.is_ok())
        }

        #[test]
        fn updates_contract_state() {
            let mut deps = helpers::init_contract();
            let env = mock_env();
            let start_state = config_read(deps.as_mut().storage).load().unwrap();
            let rewarding_validator_address = start_state.rewarding_validator_address;

            try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            )
            .unwrap();

            let new_state = config_read(deps.as_mut().storage).load().unwrap();
            assert!(new_state.rewarding_in_progress);
            assert_eq!(
                new_state.rewarding_interval_starting_block,
                env.block.height
            );
            assert_eq!(
                start_state.latest_rewarding_interval_nonce + 1,
                new_state.latest_rewarding_interval_nonce
            );
        }
    }

    #[cfg(test)]
    mod finishing_mixnode_rewarding {
        use super::*;

        #[test]
        fn can_only_be_called_by_specified_validator_address() {
            let mut deps = helpers::init_contract();
            let env = mock_env();
            let current_state = config_read(deps.as_mut().storage).load().unwrap();
            let rewarding_validator_address = current_state.rewarding_validator_address;

            try_begin_mixnode_rewarding(
                deps.as_mut(),
                env,
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            )
            .unwrap();

            let res = try_finish_mixnode_rewarding(
                deps.as_mut(),
                mock_info("not-the-approved-validator", &[]),
                1,
            );
            assert_eq!(Err(ContractError::Unauthorized), res);

            let res = try_finish_mixnode_rewarding(
                deps.as_mut(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            );
            assert!(res.is_ok())
        }

        #[test]
        fn cannot_be_called_if_rewarding_is_not_in_progress() {
            let mut deps = helpers::init_contract();
            let current_state = config_read(deps.as_mut().storage).load().unwrap();
            let rewarding_validator_address = current_state.rewarding_validator_address;

            let res = try_finish_mixnode_rewarding(
                deps.as_mut(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                0,
            );
            assert_eq!(Err(ContractError::RewardingNotInProgress), res);
        }

        #[test]
        fn provided_nonce_must_be_equal_the_current_one() {
            let mut deps = helpers::init_contract();
            let env = mock_env();
            let mut current_state = config_read(deps.as_mut().storage).load().unwrap();
            current_state.latest_rewarding_interval_nonce = 42;
            config(deps.as_mut().storage).save(&current_state).unwrap();

            let rewarding_validator_address = current_state.rewarding_validator_address;

            try_begin_mixnode_rewarding(
                deps.as_mut(),
                env,
                mock_info(rewarding_validator_address.as_ref(), &[]),
                43,
            )
            .unwrap();

            let res = try_finish_mixnode_rewarding(
                deps.as_mut(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                11,
            );
            assert_eq!(
                Err(ContractError::InvalidRewardingIntervalNonce {
                    received: 11,
                    expected: 43
                }),
                res
            );

            let res = try_finish_mixnode_rewarding(
                deps.as_mut(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                44,
            );
            assert_eq!(
                Err(ContractError::InvalidRewardingIntervalNonce {
                    received: 44,
                    expected: 43
                }),
                res
            );

            let res = try_finish_mixnode_rewarding(
                deps.as_mut(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                42,
            );
            assert_eq!(
                Err(ContractError::InvalidRewardingIntervalNonce {
                    received: 42,
                    expected: 43
                }),
                res
            );

            let res = try_finish_mixnode_rewarding(
                deps.as_mut(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                43,
            );
            assert!(res.is_ok())
        }

        #[test]
        fn updates_contract_state() {
            let mut deps = helpers::init_contract();
            let env = mock_env();
            let current_state = config_read(deps.as_mut().storage).load().unwrap();
            let rewarding_validator_address = current_state.rewarding_validator_address;

            try_begin_mixnode_rewarding(
                deps.as_mut(),
                env,
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            )
            .unwrap();

            try_finish_mixnode_rewarding(
                deps.as_mut(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            )
            .unwrap();

            let new_state = config_read(deps.as_mut().storage).load().unwrap();
            assert!(!new_state.rewarding_in_progress);
        }
    }

    #[test]
    fn rewarding_mixnodes_outside_rewarding_period() {
        let mut deps = helpers::init_contract();
        let env = mock_env();
        let current_state = config_read(deps.as_mut().storage).load().unwrap();
        let rewarding_validator_address = current_state.rewarding_validator_address;

        // bond the node
        let node_owner: Addr = Addr::unchecked("node-owner");
        let node_identity: IdentityKey = "nodeidentity".into();

        try_add_mixnode(
            deps.as_mut(),
            env.clone(),
            mock_info(node_owner.as_ref(), &good_mixnode_bond()),
            MixNode {
                identity_key: node_identity.to_string(),
                ..helpers::mix_node_fixture()
            },
        )
        .unwrap();

        let info = mock_info(rewarding_validator_address.as_ref(), &[]);
        let res = try_reward_mixnode_v2(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            node_identity.clone(),
            node_rewarding_params_fixture(100),
            1,
        );
        assert_eq!(Err(ContractError::RewardingNotInProgress), res);

        try_begin_mixnode_rewarding(deps.as_mut(), env.clone(), info.clone(), 1).unwrap();

        let res = try_reward_mixnode_v2(
            deps.as_mut(),
            env.clone(),
            info,
            node_identity.clone(),
            node_rewarding_params_fixture(100),
            1,
        );
        assert!(res.is_ok())
    }

    #[test]
    fn rewarding_mixnodes_with_incorrect_rewarding_nonce() {
        let mut deps = helpers::init_contract();
        let env = mock_env();
        let current_state = config_read(deps.as_mut().storage).load().unwrap();
        let rewarding_validator_address = current_state.rewarding_validator_address;

        // bond the node
        let node_owner: Addr = Addr::unchecked("node-owner");
        let node_identity: IdentityKey = "nodeidentity".into();

        try_add_mixnode(
            deps.as_mut(),
            env.clone(),
            mock_info(node_owner.as_ref(), &good_mixnode_bond()),
            MixNode {
                identity_key: node_identity.to_string(),
                ..helpers::mix_node_fixture()
            },
        )
        .unwrap();

        let info = mock_info(rewarding_validator_address.as_ref(), &[]);
        try_begin_mixnode_rewarding(deps.as_mut(), env.clone(), info.clone(), 1).unwrap();
        let res = try_reward_mixnode_v2(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            node_identity.clone(),
            node_rewarding_params_fixture(100),
            0,
        );
        assert_eq!(
            Err(ContractError::InvalidRewardingIntervalNonce {
                received: 0,
                expected: 1
            }),
            res
        );

        let res = try_reward_mixnode_v2(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            node_identity.clone(),
            node_rewarding_params_fixture(100),
            2,
        );
        assert_eq!(
            Err(ContractError::InvalidRewardingIntervalNonce {
                received: 2,
                expected: 1
            }),
            res
        );

        let res = try_reward_mixnode_v2(
            deps.as_mut(),
            env.clone(),
            info,
            node_identity.clone(),
            node_rewarding_params_fixture(100),
            1,
        );
        assert!(res.is_ok())
    }

    #[test]
    fn attempting_rewarding_mixnode_multiple_times_per_interval() {
        let mut deps = helpers::init_contract();
        let env = mock_env();
        let current_state = config_read(deps.as_mut().storage).load().unwrap();
        let rewarding_validator_address = current_state.rewarding_validator_address;

        // bond the node
        let node_owner: Addr = Addr::unchecked("node-owner");
        let node_identity: IdentityKey = "nodeidentity".into();

        try_add_mixnode(
            deps.as_mut(),
            env.clone(),
            mock_info(node_owner.as_ref(), &good_mixnode_bond()),
            MixNode {
                identity_key: node_identity.to_string(),
                ..helpers::mix_node_fixture()
            },
        )
        .unwrap();

        let info = mock_info(rewarding_validator_address.as_ref(), &[]);
        try_begin_mixnode_rewarding(deps.as_mut(), env.clone(), info.clone(), 1).unwrap();

        // first reward goes through just fine
        let res = try_reward_mixnode_v2(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            node_identity.clone(),
            node_rewarding_params_fixture(100),
            1,
        );
        assert!(res.is_ok());

        // but the other one fails
        let res = try_reward_mixnode_v2(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            node_identity.clone(),
            node_rewarding_params_fixture(100),
            1,
        );
        assert_eq!(
            Err(ContractError::MixnodeAlreadyRewarded {
                identity: node_identity.clone()
            }),
            res
        );

        // but rewarding the same node in the following interval is fine again
        try_finish_mixnode_rewarding(deps.as_mut(), info.clone(), 1).unwrap();
        try_begin_mixnode_rewarding(deps.as_mut(), env.clone(), info.clone(), 2).unwrap();

        let res = try_reward_mixnode_v2(
            deps.as_mut(),
            env,
            info,
            node_identity.clone(),
            node_rewarding_params_fixture(100),
            2,
        );
        assert!(res.is_ok());
    }

    #[test]
    fn rewarding_mixnode_blockstamp_based() {
        let mut deps = helpers::init_contract();
        let mut env = mock_env();
        let current_state = config_read(deps.as_mut().storage).load().unwrap();
        let rewarding_validator_address = current_state.rewarding_validator_address;

        let node_owner: Addr = Addr::unchecked("node-owner");
        let node_identity: IdentityKey = "nodeidentity".into();

        let initial_bond = 10000_000000;
        let initial_delegation = 20000_000000;
        let mixnode_bond = MixNodeBond {
            bond_amount: coin(initial_bond, DENOM),
            total_delegation: coin(initial_delegation, DENOM),
            owner: node_owner.clone(),
            layer: Layer::One,
            block_height: env.block.height,
            mix_node: MixNode {
                identity_key: node_identity.clone(),
                ..mix_node_fixture()
            },
            profit_margin_percent: Some(10),
        };

        mixnodes(deps.as_mut().storage)
            .save(node_identity.as_bytes(), &mixnode_bond)
            .unwrap();

        // delegation happens later, but not later enough
        env.block.height += MINIMUM_BLOCK_AGE_FOR_REWARDING - 1;

        mix_delegations(&mut deps.storage, &node_identity)
            .save(
                b"delegator",
                &RawDelegationData::new(initial_delegation.into(), env.block.height),
            )
            .unwrap();

        // no reward is due
        let info = mock_info(rewarding_validator_address.as_ref(), &[]);
        try_begin_mixnode_rewarding(deps.as_mut(), env.clone(), info.clone(), 1).unwrap();
        let res = try_reward_mixnode_v2(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            node_identity.clone(),
            node_rewarding_params_fixture(100),
            1,
        )
        .unwrap();
        try_finish_mixnode_rewarding(deps.as_mut(), info, 1).unwrap();

        assert_eq!(
            initial_bond,
            read_mixnode_bond(deps.as_ref().storage, node_identity.as_bytes())
                .unwrap()
                .u128()
        );
        assert_eq!(
            initial_delegation,
            read_mixnode_delegation(deps.as_ref().storage, node_identity.as_bytes())
                .unwrap()
                .u128()
        );

        assert_eq!(res.attributes[0], attr("node reward", "0"));
        assert_eq!(res.attributes[1], attr("operator reward", "0"));
        assert_eq!(res.attributes[2], attr("total delegation increase", "0"));
        assert_eq!(res.attributes[3], attr("more delegators to reward", false));

        // reward can happen now, but only for bonded node
        env.block.height += 1;

        let info = mock_info(rewarding_validator_address.as_ref(), &[]);
        try_begin_mixnode_rewarding(deps.as_mut(), env.clone(), info.clone(), 2).unwrap();
        let res = try_reward_mixnode_v2(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            node_identity.clone(),
            node_rewarding_params_fixture(100),
            2,
        )
        .unwrap();
        try_finish_mixnode_rewarding(deps.as_mut(), info, 2).unwrap();

        assert!(
            read_mixnode_bond(deps.as_ref().storage, node_identity.as_bytes())
                .unwrap()
                .u128()
                > initial_bond
        );
        assert_eq!(
            initial_delegation,
            read_mixnode_delegation(deps.as_ref().storage, node_identity.as_bytes())
                .unwrap()
                .u128()
        );

        assert_ne!(res.attributes[0], attr("node reward", "0"));
        assert_ne!(res.attributes[1], attr("operator reward", "0"));
        assert_eq!(res.attributes[2], attr("total delegation increase", "0"));
        assert_eq!(res.attributes[3], attr("more delegators to reward", false));

        // reward happens now, both for node owner and delegators
        env.block.height += MINIMUM_BLOCK_AGE_FOR_REWARDING - 1;

        let bond_before_rewarding =
            read_mixnode_bond(deps.as_ref().storage, node_identity.as_bytes())
                .unwrap()
                .u128();

        let info = mock_info(rewarding_validator_address.as_ref(), &[]);
        try_begin_mixnode_rewarding(deps.as_mut(), env.clone(), info.clone(), 3).unwrap();
        let res = try_reward_mixnode_v2(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            node_identity.clone(),
            node_rewarding_params_fixture(100),
            3,
        )
        .unwrap();
        try_finish_mixnode_rewarding(deps.as_mut(), info, 3).unwrap();

        assert!(
            read_mixnode_bond(deps.as_ref().storage, node_identity.as_bytes())
                .unwrap()
                .u128()
                > bond_before_rewarding
        );
        assert!(
            read_mixnode_delegation(deps.as_ref().storage, node_identity.as_bytes())
                .unwrap()
                .u128()
                > initial_delegation
        );

        assert_ne!(res.attributes[0], attr("node reward", "0"));
        assert_ne!(res.attributes[1], attr("operator reward", "0"));
        assert_ne!(res.attributes[2], attr("total delegation increase", "0"));
        assert_eq!(res.attributes[3], attr("more delegators to reward", false));
    }

    #[test]
    fn test_tokenomics_rewarding() {
        use crate::contract::{EPOCH_REWARD_PERCENT, INITIAL_REWARD_POOL};

        type U128 = fixed::types::U75F53;

        let mut deps = helpers::init_contract();
        let mut env = mock_env();
        let current_state = config(deps.as_mut().storage).load().unwrap();
        let rewarding_validator_address = current_state.rewarding_validator_address;
        let period_reward_pool = (INITIAL_REWARD_POOL / 100) * EPOCH_REWARD_PERCENT as u128;
        assert_eq!(period_reward_pool, 5_000_000_000_000);
        let k = 200; // Imagining our active set size is 200
        let circulating_supply = circulating_supply(&deps.storage).u128();
        assert_eq!(circulating_supply, 750_000_000_000_000u128);
        // mut_reward_pool(deps.as_mut().storage)
        //     .save(&Uint128(period_reward_pool))
        //     .unwrap();

        try_add_mixnode(
            deps.as_mut(),
            mock_env(),
            mock_info(
                "alice",
                &vec![Coin {
                    denom: DENOM.to_string(),
                    amount: Uint128(10000_000_000),
                }],
            ),
            MixNode {
                identity_key: "alice".to_string(),
                ..helpers::mix_node_fixture()
            },
        )
        .unwrap();

        try_delegate_to_mixnode(
            deps.as_mut(),
            mock_env(),
            mock_info("d1", &vec![coin(8000_000000, DENOM)]),
            "alice".to_string(),
        )
        .unwrap();

        try_delegate_to_mixnode(
            deps.as_mut(),
            mock_env(),
            mock_info("d2", &vec![coin(2000_000000, DENOM)]),
            "alice".to_string(),
        )
        .unwrap();

        let info = mock_info(rewarding_validator_address.as_ref(), &[]);
        try_begin_mixnode_rewarding(
            deps.as_mut(),
            env.clone(),
            mock_info(rewarding_validator_address.as_ref(), &[]),
            1,
        )
        .unwrap();

        env.block.height += 2 * MINIMUM_BLOCK_AGE_FOR_REWARDING;

        let mix_1 = mixnodes_read(&deps.storage).load(b"alice").unwrap();
        let mix_1_uptime = 100;

        let mut params = NodeRewardParams::new(
            period_reward_pool,
            k,
            0,
            circulating_supply,
            mix_1_uptime,
            DEFAULT_SYBIL_RESISTANCE_PERCENT,
        );

        params.set_reward_blockstamp(env.block.height);

        assert_eq!(params.performance(), 1);

        let mix_1_reward_result = mix_1.reward(&params);

        assert_eq!(
            mix_1_reward_result.sigma(),
            U128::from_num(0.0000266666666666)
        );
        assert_eq!(
            mix_1_reward_result.lambda(),
            U128::from_num(0.0000133333333333)
        );
        assert_eq!(mix_1_reward_result.reward().int(), 102646153);

        let mix1_operator_profit = mix_1.operator_reward(&params);

        let mix1_delegator1_reward = mix_1.reward_delegation(Uint128(8000_000000), &params);

        let mix1_delegator2_reward = mix_1.reward_delegation(Uint128(2000_000000), &params);

        assert_eq!(mix1_operator_profit, U128::from_num(74455384));
        assert_eq!(mix1_delegator1_reward, U128::from_num(22552615));
        assert_eq!(mix1_delegator2_reward, U128::from_num(5638153));

        let pre_reward_bond = read_mixnode_bond(&deps.storage, b"alice").unwrap().u128();
        assert_eq!(pre_reward_bond, 10000_000_000);

        let pre_reward_delegation = read_mixnode_delegation(&deps.storage, b"alice")
            .unwrap()
            .u128();
        assert_eq!(pre_reward_delegation, 10000_000_000);

        try_reward_mixnode_v2(deps.as_mut(), env, info, "alice".to_string(), params, 1).unwrap();

        assert_eq!(
            read_mixnode_bond(&deps.storage, b"alice").unwrap().u128(),
            U128::from_num(pre_reward_bond) + U128::from_num(mix1_operator_profit)
        );
        assert_eq!(
            read_mixnode_delegation(&deps.storage, b"alice")
                .unwrap()
                .u128(),
            pre_reward_delegation + mix1_delegator1_reward + mix1_delegator2_reward
        );

        assert_eq!(
            reward_pool_value(&deps.storage).u128(),
            U128::from_num(INITIAL_REWARD_POOL)
                - (U128::from_num(mix1_operator_profit)
                    + U128::from_num(mix1_delegator1_reward)
                    + U128::from_num(mix1_delegator2_reward))
        );

        // it's all correctly saved
        match rewarded_mixnodes_read(&deps.storage, 1)
            .load(b"alice")
            .unwrap()
        {
            RewardingStatus::Complete(result) => assert_eq!(
                RewardingResult {
                    operator_reward: Uint128::new(mix1_operator_profit),
                    total_delegator_reward: Uint128::new(
                        mix1_delegator1_reward + mix1_delegator2_reward
                    )
                },
                result
            ),
            _ => unreachable!(),
        }
    }

    #[cfg(test)]
    mod mixnode_rewarding_distributes_rewards_to_up_to_one_page_of_delegators {

        #[test]
        fn with_10_delegations() {
            use super::*;

            let mut deps = helpers::init_contract();
            let mut env = mock_env();
            let current_state = config(deps.as_mut().storage).load().unwrap();
            let rewarding_validator_address = current_state.rewarding_validator_address;

            let mix_bond = Uint128(10000_000_000);
            let delegation_value = 2000_000000;
            try_add_mixnode(
                deps.as_mut(),
                env.clone(),
                mock_info(
                    "10delegators",
                    &vec![Coin {
                        denom: DENOM.to_string(),
                        amount: mix_bond,
                    }],
                ),
                MixNode {
                    identity_key: "10delegators".to_string(),
                    ..helpers::mix_node_fixture()
                },
            )
            .unwrap();

            for i in 0..10 {
                try_delegate_to_mixnode(
                    deps.as_mut(),
                    env.clone(),
                    mock_info(
                        &*format!("delegator{}", i),
                        &vec![coin(delegation_value, DENOM)],
                    ),
                    "10delegators".to_string(),
                )
                .unwrap();
            }

            env.block.height += MINIMUM_BLOCK_AGE_FOR_REWARDING;

            let info = mock_info(rewarding_validator_address.as_ref(), &[]);
            try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            )
            .unwrap();

            let res = try_reward_mixnode_v2(
                deps.as_mut(),
                env,
                info,
                "10delegators".to_string(),
                node_rewarding_params_fixture(100),
                1,
            )
            .unwrap();
            assert_eq!(res.attributes[3], attr("more delegators to reward", false));

            try_finish_mixnode_rewarding(
                deps.as_mut(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            )
            .unwrap();

            for i in 0..10 {
                let delegation = mix_delegations_read(deps.as_ref().storage, "10delegators")
                    .load(format!("delegator{}", i).as_bytes())
                    .unwrap();
                assert!(delegation.amount > Uint128(delegation_value));
            }
        }

        #[test]
        fn with_full_page_limit() {
            use super::*;

            let mut deps = helpers::init_contract();
            let mut env = mock_env();
            let current_state = config(deps.as_mut().storage).load().unwrap();
            let rewarding_validator_address = current_state.rewarding_validator_address;

            let mix_bond = Uint128(10000_000_000);
            let delegation_value = 2000_000000;
            try_add_mixnode(
                deps.as_mut(),
                env.clone(),
                mock_info(
                    "MIXNODE_DELEGATORS_PAGE_LIMIT_delegators",
                    &vec![Coin {
                        denom: DENOM.to_string(),
                        amount: mix_bond,
                    }],
                ),
                MixNode {
                    identity_key: "MIXNODE_DELEGATORS_PAGE_LIMIT_delegators".to_string(),
                    ..helpers::mix_node_fixture()
                },
            )
            .unwrap();

            for i in 0..MIXNODE_DELEGATORS_PAGE_LIMIT {
                try_delegate_to_mixnode(
                    deps.as_mut(),
                    env.clone(),
                    mock_info(
                        &*format!("delegator{}", i),
                        &vec![coin(delegation_value, DENOM)],
                    ),
                    "MIXNODE_DELEGATORS_PAGE_LIMIT_delegators".to_string(),
                )
                .unwrap();
            }

            env.block.height += MINIMUM_BLOCK_AGE_FOR_REWARDING;

            let info = mock_info(rewarding_validator_address.as_ref(), &[]);
            try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            )
            .unwrap();

            let res = try_reward_mixnode_v2(
                deps.as_mut(),
                env,
                info,
                "MIXNODE_DELEGATORS_PAGE_LIMIT_delegators".to_string(),
                node_rewarding_params_fixture(100),
                1,
            )
            .unwrap();
            assert_eq!(res.attributes[3], attr("more delegators to reward", false));

            try_finish_mixnode_rewarding(
                deps.as_mut(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            )
            .unwrap();

            for i in 0..MIXNODE_DELEGATORS_PAGE_LIMIT {
                let delegation = mix_delegations_read(
                    deps.as_ref().storage,
                    "MIXNODE_DELEGATORS_PAGE_LIMIT_delegators",
                )
                .load(format!("delegator{}", i).as_bytes())
                .unwrap();
                assert!(delegation.amount > Uint128(delegation_value));
            }
        }

        #[test]
        fn with_more_than_full_page_limit() {
            use super::*;

            let mut deps = helpers::init_contract();
            let mut env = mock_env();
            let current_state = config(deps.as_mut().storage).load().unwrap();
            let rewarding_validator_address = current_state.rewarding_validator_address;

            let mix_bond = Uint128(10000_000_000);
            let delegation_value = 2000_000000;
            try_add_mixnode(
                deps.as_mut(),
                env.clone(),
                mock_info(
                    "MIXNODE_DELEGATORS_PAGE_LIMIT+1_delegators",
                    &vec![Coin {
                        denom: DENOM.to_string(),
                        amount: mix_bond,
                    }],
                ),
                MixNode {
                    identity_key: "MIXNODE_DELEGATORS_PAGE_LIMIT+1_delegators".to_string(),
                    ..helpers::mix_node_fixture()
                },
            )
            .unwrap();

            for i in 0..MIXNODE_DELEGATORS_PAGE_LIMIT + 1 {
                try_delegate_to_mixnode(
                    deps.as_mut(),
                    env.clone(),
                    mock_info(
                        &*format!("delegator{:04}", i),
                        &vec![coin(delegation_value, DENOM)],
                    ),
                    "MIXNODE_DELEGATORS_PAGE_LIMIT+1_delegators".to_string(),
                )
                .unwrap();
            }

            env.block.height += MINIMUM_BLOCK_AGE_FOR_REWARDING;

            let info = mock_info(rewarding_validator_address.as_ref(), &[]);
            try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            )
            .unwrap();

            let res = try_reward_mixnode_v2(
                deps.as_mut(),
                env,
                info,
                "MIXNODE_DELEGATORS_PAGE_LIMIT+1_delegators".to_string(),
                node_rewarding_params_fixture(100),
                1,
            )
            .unwrap();
            assert_eq!(res.attributes[3], attr("more delegators to reward", true));

            try_finish_mixnode_rewarding(
                deps.as_mut(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            )
            .unwrap();

            for i in 0..MIXNODE_DELEGATORS_PAGE_LIMIT {
                let delegation = mix_delegations_read(
                    deps.as_ref().storage,
                    "MIXNODE_DELEGATORS_PAGE_LIMIT+1_delegators",
                )
                .load(format!("delegator{:04}", i).as_bytes())
                .unwrap();
                assert!(delegation.amount > Uint128(delegation_value));
            }

            // and the one on the next page should have been unrewarded
            let delegation = mix_delegations_read(
                deps.as_ref().storage,
                "MIXNODE_DELEGATORS_PAGE_LIMIT+1_delegators",
            )
            .load(format!("delegator{:04}", MIXNODE_DELEGATORS_PAGE_LIMIT).as_bytes())
            .unwrap();
            assert_eq!(delegation.amount, Uint128(delegation_value));
        }
    }

    #[test]
    fn rewarding_mix_delegators_return_consistent_results() {
        // with single page
        let mut deps = helpers::init_contract();
        let mut env = mock_env();

        let node_identity = "bobsnode".to_string();
        try_add_mixnode(
            deps.as_mut(),
            env.clone(),
            mock_info("bob", &[coin(10000_000_000, DENOM)]),
            MixNode {
                identity_key: node_identity.clone(),
                ..helpers::mix_node_fixture()
            },
        )
        .unwrap();
        let bond = mixnodes_read(deps.as_ref().storage)
            .load(node_identity.as_bytes())
            .unwrap();

        let base_delegation = 200_000000;
        let delegations = 123;

        for i in 0..delegations {
            try_delegate_to_mixnode(
                deps.as_mut(),
                env.clone(),
                mock_info(
                    &*format!("delegator{:04}", i),
                    &vec![coin(base_delegation, DENOM)],
                ),
                node_identity.clone(),
            )
            .unwrap();
        }

        env.block.height += MINIMUM_BLOCK_AGE_FOR_REWARDING + 1;
        let mut node_rewarding_params = node_rewarding_params_fixture(100);
        node_rewarding_params.set_reward_blockstamp(env.block.height);

        let params = DelegatorRewardParams::new(&bond, node_rewarding_params);
        let res =
            reward_mix_delegators_v2(deps.as_mut().storage, &node_identity, None, params).unwrap();

        let mut actual_reward = Uint128::new(0);
        for delegation in mix_delegations_read(deps.as_ref().storage, &node_identity).range(
            None,
            None,
            Order::Ascending,
        ) {
            actual_reward += Uint128(delegation.unwrap().1.amount.u128() - base_delegation);
        }

        // sanity check to make sure we actually gave out any rewards
        assert_ne!(actual_reward, Uint128::zero());

        assert_eq!(actual_reward, res.total_rewarded);
        assert!(res.start_next.is_none());

        // with paging
        let node_identity = "alicesnode".to_string();
        try_add_mixnode(
            deps.as_mut(),
            env.clone(),
            mock_info("alice", &[coin(10000_000_000, DENOM)]),
            MixNode {
                identity_key: node_identity.clone(),
                ..helpers::mix_node_fixture()
            },
        )
        .unwrap();
        let bond = mixnodes_read(deps.as_ref().storage)
            .load(node_identity.as_bytes())
            .unwrap();

        let base_delegation = 200_000000;
        let delegations = MIXNODE_DELEGATORS_PAGE_LIMIT + 123;

        for i in 0..delegations {
            try_delegate_to_mixnode(
                deps.as_mut(),
                env.clone(),
                mock_info(
                    &*format!("delegator{:04}", i),
                    &vec![coin(base_delegation, DENOM)],
                ),
                node_identity.clone(),
            )
            .unwrap();
        }

        env.block.height += MINIMUM_BLOCK_AGE_FOR_REWARDING + 1;
        let mut node_rewarding_params = node_rewarding_params_fixture(100);
        node_rewarding_params.set_reward_blockstamp(env.block.height);

        let params = DelegatorRewardParams::new(&bond, node_rewarding_params);
        let res =
            reward_mix_delegators_v2(deps.as_mut().storage, &node_identity, None, params).unwrap();

        let mut actual_reward = Uint128::new(0);
        for delegation in mix_delegations_read(deps.as_ref().storage, &node_identity).range(
            None,
            None,
            Order::Ascending,
        ) {
            let (delegator, delegation) = delegation.unwrap();
            let delegator_reward = Uint128(delegation.amount.u128() - base_delegation);
            actual_reward += delegator_reward;

            let delegator = String::from_utf8(delegator).unwrap();
            let delegator_id: usize = delegator
                .strip_prefix("delegator")
                .unwrap()
                .parse()
                .unwrap();
            if delegator_id >= MIXNODE_DELEGATORS_PAGE_LIMIT {
                // if they were in next page, they shouldn't have gotten anything!
                assert_eq!(Uint128::zero(), delegator_reward)
            } else {
                assert_ne!(Uint128::zero(), delegator_reward);
            }
        }

        assert_eq!(actual_reward, res.total_rewarded);
        let expected_next_page_start = format!("delegator{:04}", MIXNODE_DELEGATORS_PAGE_LIMIT);
        assert_eq!(expected_next_page_start, res.start_next.clone().unwrap());

        let res2 = reward_mix_delegators_v2(
            deps.as_mut().storage,
            &node_identity,
            res.start_next.clone(),
            params,
        )
        .unwrap();

        let start = res.start_next.unwrap();
        let start_bytes = start.as_bytes();
        let mut actual_reward = Uint128::new(0);
        for delegation in mix_delegations_read(deps.as_ref().storage, &node_identity).range(
            Some(start_bytes),
            None,
            Order::Ascending,
        ) {
            actual_reward += Uint128(delegation.unwrap().1.amount.u128() - base_delegation);
        }

        assert_eq!(actual_reward, res2.total_rewarded);
        assert!(res2.start_next.is_none());
    }

    #[cfg(test)]
    mod delegator_rewarding_tx {
        use super::*;
        use crate::storage::mix_delegations_read;

        #[test]
        fn can_only_be_called_by_specified_validator_address() {
            let mut deps = helpers::init_contract();

            let res = try_reward_next_mixnode_delegators_v2(
                deps.as_mut(),
                mock_info("not-the-approved-validator", &[]),
                "alice's mixnode".to_string(),
                1,
            );
            assert_eq!(Err(ContractError::Unauthorized), res);
        }

        #[test]
        fn cannot_be_called_if_rewarding_is_not_in_progress() {
            let mut deps = helpers::init_contract();
            let current_state = config_read(deps.as_mut().storage).load().unwrap();
            let rewarding_validator_address = current_state.rewarding_validator_address;

            let res = try_reward_next_mixnode_delegators_v2(
                deps.as_mut(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                "alice's mixnode".to_string(),
                1,
            );

            assert_eq!(Err(ContractError::RewardingNotInProgress), res);
        }

        #[test]
        fn cannot_be_called_if_mixnodes_operator_wasnt_rewarded() {
            let mut deps = helpers::init_contract();
            let env = mock_env();
            let current_state = config_read(deps.as_mut().storage).load().unwrap();
            let rewarding_validator_address = current_state.rewarding_validator_address;

            try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            )
            .unwrap();

            let res = try_reward_next_mixnode_delegators_v2(
                deps.as_mut(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                "alice's mixnode".to_string(),
                1,
            );

            assert_eq!(
                Err(ContractError::MixnodeOperatorNotRewarded {
                    identity: "alice's mixnode".to_string()
                }),
                res
            )
        }

        #[test]
        fn cannot_be_called_if_mixnode_is_fully_rewarded() {
            // everything was done in a single reward call
            let mut deps = helpers::init_contract();
            let mut env = mock_env();
            let current_state = config_read(deps.as_mut().storage).load().unwrap();
            let rewarding_validator_address = current_state.rewarding_validator_address;

            try_add_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info(
                    "alice",
                    &vec![Coin {
                        denom: DENOM.to_string(),
                        amount: Uint128(10000_000_000),
                    }],
                ),
                MixNode {
                    identity_key: "alice".to_string(),
                    ..helpers::mix_node_fixture()
                },
            )
            .unwrap();

            env.block.height += MINIMUM_BLOCK_AGE_FOR_REWARDING;

            try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            )
            .unwrap();

            try_reward_mixnode_v2(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                "alice".to_string(),
                node_rewarding_params_fixture(100),
                1,
            )
            .unwrap();

            let res = try_reward_next_mixnode_delegators_v2(
                deps.as_mut(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                "alice".to_string(),
                1,
            );

            assert_eq!(
                Err(ContractError::MixnodeAlreadyRewarded {
                    identity: "alice".to_string()
                }),
                res
            );

            try_finish_mixnode_rewarding(
                deps.as_mut(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            )
            .unwrap();

            // there was another page of delegators, but they were already dealt with
            try_add_mixnode(
                deps.as_mut(),
                env.clone(),
                mock_info(
                    "bob",
                    &vec![Coin {
                        denom: DENOM.to_string(),
                        amount: Uint128(10000_000_000),
                    }],
                ),
                MixNode {
                    identity_key: "bob".to_string(),
                    ..helpers::mix_node_fixture()
                },
            )
            .unwrap();

            for i in 0..MIXNODE_DELEGATORS_PAGE_LIMIT + 1 {
                try_delegate_to_mixnode(
                    deps.as_mut(),
                    env.clone(),
                    mock_info(
                        &*format!("delegator{:04}", i),
                        &vec![coin(2000_000000, DENOM)],
                    ),
                    "bob".to_string(),
                )
                .unwrap();
            }

            env.block.height += MINIMUM_BLOCK_AGE_FOR_REWARDING;

            let info = mock_info(rewarding_validator_address.as_ref(), &[]);
            try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                2,
            )
            .unwrap();

            try_reward_mixnode_v2(
                deps.as_mut(),
                env.clone(),
                info,
                "bob".to_string(),
                node_rewarding_params_fixture(100),
                2,
            )
            .unwrap();

            // rewards all pending
            try_reward_next_mixnode_delegators_v2(
                deps.as_mut(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                "bob".to_string(),
                2,
            )
            .unwrap();

            let res = try_reward_next_mixnode_delegators_v2(
                deps.as_mut(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                "bob".to_string(),
                2,
            );

            assert_eq!(
                Err(ContractError::MixnodeAlreadyRewarded {
                    identity: "bob".to_string()
                }),
                res
            );

            try_finish_mixnode_rewarding(
                deps.as_mut(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                2,
            )
            .unwrap();
        }

        #[test]
        fn rewards_all_delegators_on_the_next_page() {
            // setup: bond > page limit delegators, reward operator + first batch
            let mut deps = helpers::init_contract();
            let mut env = mock_env();
            let current_state = config_read(deps.as_mut().storage).load().unwrap();
            let rewarding_validator_address = current_state.rewarding_validator_address;

            let mix_bond = Uint128(10000_000_000);
            let delegation_value = 2000_000000;

            let total_delegators = 2 * MIXNODE_DELEGATORS_PAGE_LIMIT + 123;

            try_add_mixnode(
                deps.as_mut(),
                env.clone(),
                mock_info(
                    "alice",
                    &vec![Coin {
                        denom: DENOM.to_string(),
                        amount: mix_bond,
                    }],
                ),
                MixNode {
                    identity_key: "alice".to_string(),
                    ..helpers::mix_node_fixture()
                },
            )
            .unwrap();

            for i in 0..total_delegators {
                try_delegate_to_mixnode(
                    deps.as_mut(),
                    env.clone(),
                    mock_info(
                        &*format!("delegator{:04}", i),
                        &vec![coin(delegation_value, DENOM)],
                    ),
                    "alice".to_string(),
                )
                .unwrap();
            }

            env.block.height += MINIMUM_BLOCK_AGE_FOR_REWARDING;

            let info = mock_info(rewarding_validator_address.as_ref(), &[]);
            try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            )
            .unwrap();

            try_reward_mixnode_v2(
                deps.as_mut(),
                env.clone(),
                info,
                "alice".to_string(),
                node_rewarding_params_fixture(100),
                1,
            )
            .unwrap();

            // we have 3 pages in total, so we have to call this twice
            try_reward_next_mixnode_delegators_v2(
                deps.as_mut(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                "alice".to_string(),
                1,
            )
            .unwrap();
            try_reward_next_mixnode_delegators_v2(
                deps.as_mut(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                "alice".to_string(),
                1,
            )
            .unwrap();

            let expected = mix_delegations_read(deps.as_ref().storage, "alice")
                .load("delegator0001".as_bytes())
                .unwrap()
                .amount;

            for i in 0..total_delegators {
                // everyone was rewarded (and the same amount, because they all delegated the same amount)
                let delegation = mix_delegations_read(deps.as_ref().storage, "alice")
                    .load(format!("delegator{:04}", i).as_bytes())
                    .unwrap();
                assert!(delegation.amount > Uint128(delegation_value));
                assert_eq!(expected, delegation.amount)
            }
        }

        #[test]
        fn ignores_delegators_that_updated_their_pledge_in_the_meantime() {
            // setup: bond > page limit delegators, reward operator + first batch
            let mut deps = helpers::init_contract();
            let mut env = mock_env();
            let current_state = config_read(deps.as_mut().storage).load().unwrap();
            let rewarding_validator_address = current_state.rewarding_validator_address;

            let mix_bond = Uint128(10000_000_000);
            let delegation_value = 2000_000000;

            let total_delegators = MIXNODE_DELEGATORS_PAGE_LIMIT + 123;

            try_add_mixnode(
                deps.as_mut(),
                env.clone(),
                mock_info(
                    "alice",
                    &vec![Coin {
                        denom: DENOM.to_string(),
                        amount: mix_bond,
                    }],
                ),
                MixNode {
                    identity_key: "alice".to_string(),
                    ..helpers::mix_node_fixture()
                },
            )
            .unwrap();

            for i in 0..total_delegators {
                try_delegate_to_mixnode(
                    deps.as_mut(),
                    env.clone(),
                    mock_info(
                        &*format!("delegator{:04}", i),
                        &vec![coin(delegation_value, DENOM)],
                    ),
                    "alice".to_string(),
                )
                .unwrap();
            }

            env.block.height += MINIMUM_BLOCK_AGE_FOR_REWARDING;

            // update some delegations (on 'main' page and the secondary call)
            try_delegate_to_mixnode(
                deps.as_mut(),
                env.clone(),
                mock_info("delegator0123", &vec![coin(delegation_value, DENOM)]),
                "alice".to_string(),
            )
            .unwrap();

            try_delegate_to_mixnode(
                deps.as_mut(),
                env.clone(),
                mock_info(
                    &*format!("delegator{:04}", 123 + MIXNODE_DELEGATORS_PAGE_LIMIT),
                    &vec![coin(delegation_value, DENOM)],
                ),
                "alice".to_string(),
            )
            .unwrap();

            env.block.height += 123;

            let info = mock_info(rewarding_validator_address.as_ref(), &[]);
            try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            )
            .unwrap();

            try_reward_mixnode_v2(
                deps.as_mut(),
                env.clone(),
                info,
                "alice".to_string(),
                node_rewarding_params_fixture(100),
                1,
            )
            .unwrap();

            // we have 3 pages in total, so we have to call this twice
            try_reward_next_mixnode_delegators_v2(
                deps.as_mut(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                "alice".to_string(),
                1,
            )
            .unwrap();

            let expected = mix_delegations_read(deps.as_ref().storage, "alice")
                .load("delegator0001".as_bytes())
                .unwrap()
                .amount;

            for i in 0..total_delegators {
                // everyone was rewarded (and the same amount, because they all delegated the same amount)
                let delegation = mix_delegations_read(deps.as_ref().storage, "alice")
                    .load(format!("delegator{:04}", i).as_bytes())
                    .unwrap();

                if i == 123 || i == 123 + MIXNODE_DELEGATORS_PAGE_LIMIT {
                    assert_eq!(delegation.amount, Uint128(2 * delegation_value))
                } else {
                    assert!(delegation.amount > Uint128(delegation_value));
                    assert_eq!(expected, delegation.amount)
                }
            }
        }
    }
}
