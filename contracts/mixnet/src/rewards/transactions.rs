// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::delegations::storage as delegations_storage;
use crate::error::ContractError;
use crate::mixnet_contract_settings::storage as mixnet_params_storage;
use crate::mixnodes::storage as mixnodes_storage;
use crate::rewards::helpers;
use cosmwasm_std::{Addr, DepsMut, Env, MessageInfo, Response, StdResult, Storage, Uint128};
use cw_storage_plus::{Bound, PrimaryKey};
use mixnet_contract::mixnode::{DelegatorRewardParams, NodeRewardParams};
use mixnet_contract::{
    IdentityKey, RewardingResult, RewardingStatus, MIXNODE_DELEGATORS_PAGE_LIMIT,
};

#[derive(Debug)]
struct MixDelegationRewardingResult {
    total_rewarded: Uint128,
    start_next: Option<Addr>,
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
    let state = mixnet_params_storage::CONTRACT_STATE.load(storage)?;

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

// Note: this function is designed to work with only a single validator entity distributing rewards
// The main purpose of this function is to update `latest_rewarding_interval_nonce` which
// will trigger a different seed selection for the pseudorandom generation of the "demanded" set of mixnodes.
pub(crate) fn try_begin_mixnode_rewarding(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    rewarding_interval_nonce: u32,
) -> Result<Response, ContractError> {
    let mut state = mixnet_params_storage::CONTRACT_STATE.load(deps.storage)?;

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
        && state.rewarding_interval_starting_block + storage::MAX_REWARDING_DURATION_IN_BLOCKS
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

    mixnet_params_storage::CONTRACT_STATE.save(deps.storage, &state)?;

    Ok(Response::new().add_attribute(
        "rewarding interval nonce",
        rewarding_interval_nonce.to_string(),
    ))
}

fn reward_mix_delegators(
    storage: &mut dyn Storage,
    mix_identity: IdentityKey,
    start: Option<Addr>,
    params: DelegatorRewardParams,
) -> StdResult<MixDelegationRewardingResult> {
    // TODO: some checks to make sure stuff is not TOO stale.

    let chunk_size = MIXNODE_DELEGATORS_PAGE_LIMIT;

    let start_value =
        start.map(|start| Bound::Inclusive((mix_identity.clone(), start).joined_key()));

    let delegations = delegations_storage::delegations();

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
        .idx
        .mixnode
        .prefix(mix_identity)
        .range(storage, start_value, None, cosmwasm_std::Order::Ascending)
        .take(chunk_size + 1)
    {
        items += 1;

        let (_pk, mut delegation) = delegation?;

        if items == chunk_size + 1 {
            // we shouldn't process this data, it's for the next call
            start_next = Some(delegation.owner());
            break;
        } else {
            // and for each of them increase the stake proportionally to the reward
            // if at least `MINIMUM_BLOCK_AGE_FOR_REWARDING` blocks have been created
            // since they delegated
            if delegation.block_height + storage::MINIMUM_BLOCK_AGE_FOR_REWARDING
                <= params.node_reward_params().reward_blockstamp()
            {
                let reward = params.determine_delegation_reward(delegation.amount.amount);
                delegation.increment_amount(Uint128::new(reward), None);
                total_rewarded += Uint128::new(reward);

                rewarded_delegations.push(delegation);
            }
        }
    }

    // finally save all delegation data back into the storage
    for rewarded_delegation in rewarded_delegations {
        let storage_key = rewarded_delegation.storage_key().joined_key();
        delegations.save(storage, storage_key, &rewarded_delegation)?;
    }

    Ok(MixDelegationRewardingResult {
        total_rewarded,
        start_next,
    })
}

pub(crate) fn try_reward_next_mixnode_delegators(
    deps: DepsMut,
    info: MessageInfo,
    mix_identity: IdentityKey,
    rewarding_interval_nonce: u32,
) -> Result<Response, ContractError> {
    verify_rewarding_state(deps.storage, info, rewarding_interval_nonce)?;

    match storage::REWARDING_STATUS.may_load(
        deps.storage,
        (rewarding_interval_nonce.into(), mix_identity.clone()),
    )? {
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
            let delegation_rewarding_result = reward_mix_delegators(
                deps.storage,
                mix_identity.clone(),
                Some(next_page_info.next_start),
                next_page_info.rewarding_params,
            )?;

            helpers::update_post_rewarding_storage(
                deps.storage,
                &mix_identity,
                Uint128::zero(),
                delegation_rewarding_result.total_rewarded,
            )?;

            let mut rewarding_results = next_page_info.running_results;
            rewarding_results.total_delegator_reward += delegation_rewarding_result.total_rewarded;

            let round_increase = delegation_rewarding_result.total_rewarded.to_string();
            let more_delegators = delegation_rewarding_result.start_next.is_some();

            helpers::update_rewarding_status(
                deps.storage,
                rewarding_interval_nonce,
                mix_identity,
                rewarding_results,
                delegation_rewarding_result.start_next,
                next_page_info.rewarding_params,
            )?;

            Ok(Response::new()
                .add_attribute("current round delegation increase", round_increase)
                .add_attribute("more delegators to reward", more_delegators.to_string()))
        }
    }
}

// Note: if any changes are made to this function or anything it is calling down the stack,
// for example delegation reward distribution, the gas limits must be retested and both
// validator-api/src/rewarding/mod.rs::{MIXNODE_REWARD_OP_BASE_GAS_LIMIT, PER_MIXNODE_DELEGATION_GAS_INCREASE}
// must be updated appropriately.
pub(crate) fn try_reward_mixnode(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    mix_identity: IdentityKey,
    params: NodeRewardParams,
    rewarding_interval_nonce: u32,
) -> Result<Response, ContractError> {
    verify_rewarding_state(deps.storage, info, rewarding_interval_nonce)?;

    // check if the mixnode hasn't been rewarded in this rewarding interval already
    match storage::REWARDING_STATUS.may_load(
        deps.storage,
        (rewarding_interval_nonce.into(), mix_identity.clone()),
    )? {
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
    let current_bond = match mixnodes_storage::read_full_mixnode_bond(deps.storage, &mix_identity)?
    {
        Some(bond) => bond,
        None => return Ok(Response::new().add_attribute("result", "bond not found")),
    };

    let mut node_reward = "0".to_string();
    let mut operator_reward = Uint128::zero();
    let mut total_delegation_increase = Uint128::zero();
    let mut more_delegators = false;

    // check if node is old enough for rewarding and if its uptime is non-zero
    if current_bond.block_height + storage::MINIMUM_BLOCK_AGE_FOR_REWARDING <= env.block.height
        && params.uptime() > 0
    {
        let mut node_reward_params = params;
        node_reward_params.set_reward_blockstamp(env.block.height);

        let operator_reward_result = current_bond.reward(&node_reward_params);
        node_reward = operator_reward_result.reward().to_string();

        // Omitting the price per packet function now, it follows that base operator reward is the node_reward
        operator_reward = Uint128::new(current_bond.operator_reward(&node_reward_params));

        let delegator_params = DelegatorRewardParams::new(&current_bond, node_reward_params);
        let delegation_rewarding_result =
            reward_mix_delegators(deps.storage, mix_identity.clone(), None, delegator_params)?;

        helpers::update_post_rewarding_storage(
            deps.storage,
            &mix_identity,
            operator_reward,
            delegation_rewarding_result.total_rewarded,
        )?;

        let rewarding_results = RewardingResult {
            operator_reward,
            total_delegator_reward: delegation_rewarding_result.total_rewarded,
        };

        total_delegation_increase = rewarding_results.total_delegator_reward;
        more_delegators = delegation_rewarding_result.start_next.is_some();

        helpers::update_rewarding_status(
            deps.storage,
            rewarding_interval_nonce,
            mix_identity,
            rewarding_results,
            delegation_rewarding_result.start_next,
            delegator_params,
        )?;
    } else {
        // node is not eligible for rewarding, so we're done immediately
        storage::REWARDING_STATUS.save(
            deps.storage,
            (rewarding_interval_nonce.into(), mix_identity),
            &RewardingStatus::Complete(Default::default()),
        )?;
    }

    Ok(Response::new()
        .add_attribute("node reward", node_reward)
        .add_attribute("operator reward", operator_reward)
        .add_attribute("total delegation increase", total_delegation_increase)
        .add_attribute("more delegators to reward", more_delegators.to_string()))
}

pub(crate) fn try_finish_mixnode_rewarding(
    deps: DepsMut,
    info: MessageInfo,
    rewarding_interval_nonce: u32,
) -> Result<Response, ContractError> {
    let mut state = mixnet_params_storage::CONTRACT_STATE.load(deps.storage)?;

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
    mixnet_params_storage::CONTRACT_STATE.save(deps.storage, &state)?;

    Ok(Response::new())
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::contract::DEFAULT_SYBIL_RESISTANCE_PERCENT;
    use crate::delegations::transactions::try_delegate_to_mixnode;
    use crate::error::ContractError;
    use crate::mixnet_contract_settings::storage as mixnet_params_storage;
    use crate::mixnodes::storage as mixnodes_storage;
    use crate::mixnodes::storage::StoredMixnodeBond;
    use crate::mixnodes::transactions::try_add_mixnode;
    use crate::rewards::transactions::{
        try_begin_mixnode_rewarding, try_finish_mixnode_rewarding, try_reward_mixnode,
    };
    use crate::support::tests::test_helpers;
    use crate::support::tests::test_helpers::{good_mixnode_bond, mix_node_fixture};
    use config::defaults::DENOM;
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::Coin;
    use cosmwasm_std::{attr, Order};
    use cosmwasm_std::{coin, Addr, Uint128};
    use mixnet_contract::mixnode::NodeRewardParams;
    use mixnet_contract::{Delegation, MixNode};
    use mixnet_contract::{IdentityKey, Layer};

    #[cfg(test)]
    mod beginning_mixnode_rewarding {
        use super::*;
        use crate::rewards::transactions::try_begin_mixnode_rewarding;
        use crate::support::tests::test_helpers;
        use cosmwasm_std::testing::mock_env;

        #[test]
        fn can_only_be_called_by_specified_validator_address() {
            let mut deps = test_helpers::init_contract();
            let env = mock_env();
            let current_state = mixnet_params_storage::CONTRACT_STATE
                .load(deps.as_mut().storage)
                .unwrap();
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
            let mut deps = test_helpers::init_contract();
            let env = mock_env();
            let current_state = mixnet_params_storage::CONTRACT_STATE
                .load(deps.as_mut().storage)
                .unwrap();
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
            let mut deps = test_helpers::init_contract();
            let env = mock_env();
            let current_state = mixnet_params_storage::CONTRACT_STATE
                .load(deps.as_mut().storage)
                .unwrap();
            let rewarding_validator_address = current_state.rewarding_validator_address;

            try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            )
            .unwrap();

            let mut new_env = env.clone();

            new_env.block.height = env.block.height + storage::MAX_REWARDING_DURATION_IN_BLOCKS;

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
            let mut deps = test_helpers::init_contract();
            let env = mock_env();
            let mut current_state = mixnet_params_storage::CONTRACT_STATE
                .load(deps.as_mut().storage)
                .unwrap();
            current_state.latest_rewarding_interval_nonce = 42;
            mixnet_params_storage::CONTRACT_STATE
                .save(deps.as_mut().storage, &current_state)
                .unwrap();

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
            let mut deps = test_helpers::init_contract();
            let env = mock_env();
            let start_state = mixnet_params_storage::CONTRACT_STATE
                .load(deps.as_mut().storage)
                .unwrap();
            let rewarding_validator_address = start_state.rewarding_validator_address;

            try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            )
            .unwrap();

            let new_state = mixnet_params_storage::CONTRACT_STATE
                .load(deps.as_mut().storage)
                .unwrap();
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
        use crate::rewards::transactions::{
            try_begin_mixnode_rewarding, try_finish_mixnode_rewarding,
        };
        use crate::support::tests::test_helpers;

        #[test]
        fn can_only_be_called_by_specified_validator_address() {
            let mut deps = test_helpers::init_contract();
            let env = mock_env();
            let current_state = mixnet_params_storage::CONTRACT_STATE
                .load(deps.as_mut().storage)
                .unwrap();
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
            let mut deps = test_helpers::init_contract();
            let current_state = mixnet_params_storage::CONTRACT_STATE
                .load(deps.as_mut().storage)
                .unwrap();
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
            let mut deps = test_helpers::init_contract();
            let env = mock_env();
            let mut current_state = mixnet_params_storage::CONTRACT_STATE
                .load(deps.as_mut().storage)
                .unwrap();
            current_state.latest_rewarding_interval_nonce = 42;
            mixnet_params_storage::CONTRACT_STATE
                .save(deps.as_mut().storage, &current_state)
                .unwrap();

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
            let mut deps = test_helpers::init_contract();
            let env = mock_env();
            let current_state = mixnet_params_storage::CONTRACT_STATE
                .load(deps.as_mut().storage)
                .unwrap();
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

            let new_state = mixnet_params_storage::CONTRACT_STATE
                .load(deps.as_mut().storage)
                .unwrap();
            assert!(!new_state.rewarding_in_progress);
        }
    }

    #[test]
    fn rewarding_mixnodes_outside_rewarding_period() {
        let mut deps = test_helpers::init_contract();
        let env = mock_env();
        let current_state = mixnet_params_storage::CONTRACT_STATE
            .load(deps.as_mut().storage)
            .unwrap();
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
                ..test_helpers::mix_node_fixture()
            },
        )
        .unwrap();

        let info = mock_info(rewarding_validator_address.as_ref(), &[]);
        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            node_identity.clone(),
            test_helpers::node_rewarding_params_fixture(100),
            1,
        );
        assert_eq!(Err(ContractError::RewardingNotInProgress), res);

        try_begin_mixnode_rewarding(deps.as_mut(), env.clone(), info.clone(), 1).unwrap();

        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info,
            node_identity.clone(),
            test_helpers::node_rewarding_params_fixture(100),
            1,
        );
        assert!(res.is_ok())
    }

    #[test]
    fn rewarding_mixnodes_with_incorrect_rewarding_nonce() {
        let mut deps = test_helpers::init_contract();
        let env = mock_env();
        let current_state = mixnet_params_storage::CONTRACT_STATE
            .load(deps.as_mut().storage)
            .unwrap();
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
                ..test_helpers::mix_node_fixture()
            },
        )
        .unwrap();

        let info = mock_info(rewarding_validator_address.as_ref(), &[]);
        try_begin_mixnode_rewarding(deps.as_mut(), env.clone(), info.clone(), 1).unwrap();
        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            node_identity.clone(),
            test_helpers::node_rewarding_params_fixture(100),
            0,
        );
        assert_eq!(
            Err(ContractError::InvalidRewardingIntervalNonce {
                received: 0,
                expected: 1
            }),
            res
        );

        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            node_identity.clone(),
            test_helpers::node_rewarding_params_fixture(100),
            2,
        );
        assert_eq!(
            Err(ContractError::InvalidRewardingIntervalNonce {
                received: 2,
                expected: 1
            }),
            res
        );

        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info,
            node_identity.clone(),
            test_helpers::node_rewarding_params_fixture(100),
            1,
        );
        assert!(res.is_ok())
    }

    #[test]
    fn attempting_rewarding_mixnode_multiple_times_per_interval() {
        let mut deps = test_helpers::init_contract();
        let env = mock_env();
        let current_state = mixnet_params_storage::CONTRACT_STATE
            .load(deps.as_mut().storage)
            .unwrap();
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
                ..test_helpers::mix_node_fixture()
            },
        )
        .unwrap();

        let info = mock_info(rewarding_validator_address.as_ref(), &[]);
        try_begin_mixnode_rewarding(deps.as_mut(), env.clone(), info.clone(), 1).unwrap();

        // first reward goes through just fine
        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            node_identity.clone(),
            test_helpers::node_rewarding_params_fixture(100),
            1,
        );
        assert!(res.is_ok());

        // but the other one fails
        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            node_identity.clone(),
            test_helpers::node_rewarding_params_fixture(100),
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

        let res = try_reward_mixnode(
            deps.as_mut(),
            env,
            info,
            node_identity.clone(),
            test_helpers::node_rewarding_params_fixture(100),
            2,
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
            bond_amount: coin(initial_bond, DENOM),
            owner: node_owner.clone(),
            layer: Layer::One,
            block_height: env.block.height,
            mix_node: MixNode {
                identity_key: node_identity.clone(),
                ..mix_node_fixture()
            },
            profit_margin_percent: Some(10),
        };

        mixnodes_storage::mixnodes()
            .save(deps.as_mut().storage, &node_identity, &mixnode_bond)
            .unwrap();
        mixnodes_storage::TOTAL_DELEGATION
            .save(
                deps.as_mut().storage,
                &node_identity,
                &Uint128::new(initial_delegation),
            )
            .unwrap();

        // delegation happens later, but not later enough
        env.block.height += storage::MINIMUM_BLOCK_AGE_FOR_REWARDING - 1;

        delegations_storage::delegations()
            .save(
                deps.as_mut().storage,
                (node_identity.clone(), "delegator").joined_key(),
                &Delegation::new(
                    Addr::unchecked("delegator"),
                    node_identity.clone(),
                    coin(initial_delegation, DENOM),
                    env.block.height,
                ),
            )
            .unwrap();

        let info = mock_info(rewarding_validator_address.as_ref(), &[]);
        try_begin_mixnode_rewarding(deps.as_mut(), env.clone(), info.clone(), 1).unwrap();
        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            node_identity.clone(),
            test_helpers::node_rewarding_params_fixture(100),
            1,
        )
        .unwrap();
        try_finish_mixnode_rewarding(deps.as_mut(), info, 1).unwrap();

        assert_eq!(
            initial_bond,
            test_helpers::read_mixnode_bond_amount(deps.as_ref().storage, &node_identity)
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

        assert_eq!(res.attributes[0], attr("node reward", "0"));
        assert_eq!(res.attributes[1], attr("operator reward", "0"));
        assert_eq!(res.attributes[2], attr("total delegation increase", "0"));
        assert_eq!(
            res.attributes[3],
            attr("more delegators to reward", false.to_string())
        );

        // reward can happen now, but only for bonded node
        env.block.height += 1;

        let info = mock_info(rewarding_validator_address.as_ref(), &[]);
        try_begin_mixnode_rewarding(deps.as_mut(), env.clone(), info.clone(), 2).unwrap();
        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            node_identity.clone(),
            test_helpers::node_rewarding_params_fixture(100),
            2,
        )
        .unwrap();
        try_finish_mixnode_rewarding(deps.as_mut(), info, 2).unwrap();

        assert!(
            test_helpers::read_mixnode_bond_amount(deps.as_ref().storage, &node_identity)
                .unwrap()
                .u128()
                > initial_bond
        );
        assert_eq!(
            initial_delegation,
            mixnodes_storage::TOTAL_DELEGATION
                .load(deps.as_ref().storage, &node_identity)
                .unwrap()
                .u128()
        );

        assert_ne!(res.attributes[0], attr("node reward", "0"));
        assert_ne!(res.attributes[1], attr("operator reward", "0"));
        assert_eq!(res.attributes[2], attr("total delegation increase", "0"));
        assert_eq!(
            res.attributes[3],
            attr("more delegators to reward", false.to_string())
        );

        // reward happens now, both for node owner and delegators
        env.block.height += storage::MINIMUM_BLOCK_AGE_FOR_REWARDING - 1;

        let bond_before_rewarding =
            test_helpers::read_mixnode_bond_amount(deps.as_ref().storage, &node_identity)
                .unwrap()
                .u128();

        let info = mock_info(rewarding_validator_address.as_ref(), &[]);
        try_begin_mixnode_rewarding(deps.as_mut(), env.clone(), info.clone(), 3).unwrap();
        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            node_identity.clone(),
            test_helpers::node_rewarding_params_fixture(100),
            3,
        )
        .unwrap();
        try_finish_mixnode_rewarding(deps.as_mut(), info, 3).unwrap();

        assert!(
            test_helpers::read_mixnode_bond_amount(deps.as_ref().storage, &node_identity)
                .unwrap()
                .u128()
                > bond_before_rewarding
        );
        assert!(
            mixnodes_storage::TOTAL_DELEGATION
                .load(deps.as_ref().storage, &node_identity)
                .unwrap()
                .u128()
                > initial_delegation
        );

        assert_ne!(res.attributes[0], attr("node reward", "0"));
        assert_ne!(res.attributes[1], attr("operator reward", "0"));
        assert_ne!(res.attributes[2], attr("total delegation increase", "0"));
        assert_eq!(
            res.attributes[3],
            attr("more delegators to reward", false.to_string())
        );
    }

    #[test]
    fn test_tokenomics_rewarding() {
        use crate::contract::{EPOCH_REWARD_PERCENT, INITIAL_REWARD_POOL};

        type U128 = fixed::types::U75F53;

        let mut deps = test_helpers::init_contract();
        let mut env = mock_env();
        let current_state = mixnet_params_storage::CONTRACT_STATE
            .load(deps.as_ref().storage)
            .unwrap();
        let rewarding_validator_address = current_state.rewarding_validator_address;
        let period_reward_pool = (INITIAL_REWARD_POOL / 100) * EPOCH_REWARD_PERCENT as u128;
        assert_eq!(period_reward_pool, 5_000_000_000_000);
        let k = 200; // Imagining our active set size is 200
        let circulating_supply = storage::circulating_supply(&deps.storage).unwrap().u128();
        assert_eq!(circulating_supply, 750_000_000_000_000u128);
        // mut_reward_pool(deps.as_mut().storage)
        //     .save(&Uint128::new(period_reward_pool))
        //     .unwrap();

        try_add_mixnode(
            deps.as_mut(),
            mock_env(),
            mock_info(
                "alice",
                &[Coin {
                    denom: DENOM.to_string(),
                    amount: Uint128::new(10_000_000_000),
                }],
            ),
            MixNode {
                identity_key: "alice".to_string(),
                ..test_helpers::mix_node_fixture()
            },
        )
        .unwrap();

        try_delegate_to_mixnode(
            deps.as_mut(),
            mock_env(),
            mock_info("d1", &[coin(8000_000000, DENOM)]),
            "alice".to_string(),
        )
        .unwrap();

        try_delegate_to_mixnode(
            deps.as_mut(),
            mock_env(),
            mock_info("d2", &[coin(2000_000000, DENOM)]),
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

        env.block.height += 2 * storage::MINIMUM_BLOCK_AGE_FOR_REWARDING;

        let mix_1 = mixnodes_storage::read_full_mixnode_bond(&deps.storage, "alice")
            .unwrap()
            .unwrap();
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

        let mix1_delegator1_reward = mix_1.reward_delegation(Uint128::new(8000_000000), &params);

        let mix1_delegator2_reward = mix_1.reward_delegation(Uint128::new(2000_000000), &params);

        assert_eq!(mix1_operator_profit, U128::from_num(74455384));
        assert_eq!(mix1_delegator1_reward, U128::from_num(22552615));
        assert_eq!(mix1_delegator2_reward, U128::from_num(5638153));

        let pre_reward_bond = test_helpers::read_mixnode_bond_amount(&deps.storage, "alice")
            .unwrap()
            .u128();
        assert_eq!(pre_reward_bond, 10_000_000_000);

        let pre_reward_delegation = mixnodes_storage::TOTAL_DELEGATION
            .load(&deps.storage, "alice")
            .unwrap()
            .u128();
        assert_eq!(pre_reward_delegation, 10_000_000_000);

        try_reward_mixnode(deps.as_mut(), env, info, "alice".to_string(), params, 1).unwrap();

        assert_eq!(
            test_helpers::read_mixnode_bond_amount(&deps.storage, "alice")
                .unwrap()
                .u128(),
            U128::from_num(pre_reward_bond) + U128::from_num(mix1_operator_profit)
        );
        assert_eq!(
            mixnodes_storage::TOTAL_DELEGATION
                .load(&deps.storage, "alice")
                .unwrap()
                .u128(),
            pre_reward_delegation + mix1_delegator1_reward + mix1_delegator2_reward
        );

        assert_eq!(
            storage::REWARD_POOL.load(&deps.storage).unwrap().u128(),
            U128::from_num(INITIAL_REWARD_POOL)
                - (U128::from_num(mix1_operator_profit)
                    + U128::from_num(mix1_delegator1_reward)
                    + U128::from_num(mix1_delegator2_reward))
        );

        // it's all correctly saved
        match storage::REWARDING_STATUS
            .load(deps.as_ref().storage, (1.into(), "alice".into()))
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

            let mut deps = test_helpers::init_contract();
            let mut env = mock_env();
            let current_state = mixnet_params_storage::CONTRACT_STATE
                .load(deps.as_mut().storage)
                .unwrap();
            let rewarding_validator_address = current_state.rewarding_validator_address;

            let mix_bond = Uint128::new(10000_000_000);
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
                    ..test_helpers::mix_node_fixture()
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

            env.block.height += storage::MINIMUM_BLOCK_AGE_FOR_REWARDING;

            let info = mock_info(rewarding_validator_address.as_ref(), &[]);
            try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            )
            .unwrap();

            let res = try_reward_mixnode(
                deps.as_mut(),
                env,
                info,
                "10delegators".to_string(),
                test_helpers::node_rewarding_params_fixture(100),
                1,
            )
            .unwrap();
            assert_eq!(
                res.attributes[3],
                attr("more delegators to reward", false.to_string())
            );

            try_finish_mixnode_rewarding(
                deps.as_mut(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            )
            .unwrap();

            for i in 0..10 {
                let delegation = test_helpers::read_delegation(
                    &deps.storage,
                    "10delegators",
                    format!("delegator{}", i),
                )
                .unwrap();

                assert!(delegation.amount.amount > Uint128::new(delegation_value));
            }
        }

        #[test]
        fn with_full_page_limit() {
            use super::*;

            let mut deps = test_helpers::init_contract();
            let mut env = mock_env();
            let current_state = mixnet_params_storage::CONTRACT_STATE
                .load(deps.as_mut().storage)
                .unwrap();
            let rewarding_validator_address = current_state.rewarding_validator_address;

            let mix_bond = Uint128::new(10000_000_000);
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
                    ..test_helpers::mix_node_fixture()
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

            env.block.height += storage::MINIMUM_BLOCK_AGE_FOR_REWARDING;

            let info = mock_info(rewarding_validator_address.as_ref(), &[]);
            try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            )
            .unwrap();

            let res = try_reward_mixnode(
                deps.as_mut(),
                env,
                info,
                "MIXNODE_DELEGATORS_PAGE_LIMIT_delegators".to_string(),
                test_helpers::node_rewarding_params_fixture(100),
                1,
            )
            .unwrap();
            assert_eq!(
                res.attributes[3],
                attr("more delegators to reward", false.to_string())
            );

            try_finish_mixnode_rewarding(
                deps.as_mut(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            )
            .unwrap();

            for i in 0..MIXNODE_DELEGATORS_PAGE_LIMIT {
                let delegation = test_helpers::read_delegation(
                    &deps.storage,
                    "MIXNODE_DELEGATORS_PAGE_LIMIT_delegators",
                    format!("delegator{}", i),
                )
                .unwrap();

                assert!(delegation.amount.amount > Uint128::new(delegation_value));
            }
        }

        #[test]
        fn with_more_than_full_page_limit() {
            use super::*;

            let mut deps = test_helpers::init_contract();
            let mut env = mock_env();
            let current_state = mixnet_params_storage::CONTRACT_STATE
                .load(deps.as_mut().storage)
                .unwrap();
            let rewarding_validator_address = current_state.rewarding_validator_address;

            let mix_bond = Uint128::new(10000_000_000);
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
                    ..test_helpers::mix_node_fixture()
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

            env.block.height += storage::MINIMUM_BLOCK_AGE_FOR_REWARDING;

            let info = mock_info(rewarding_validator_address.as_ref(), &[]);
            try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            )
            .unwrap();

            let res = try_reward_mixnode(
                deps.as_mut(),
                env,
                info,
                "MIXNODE_DELEGATORS_PAGE_LIMIT+1_delegators".to_string(),
                test_helpers::node_rewarding_params_fixture(100),
                1,
            )
            .unwrap();
            assert_eq!(
                res.attributes[3],
                attr("more delegators to reward", true.to_string())
            );

            try_finish_mixnode_rewarding(
                deps.as_mut(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            )
            .unwrap();

            for i in 0..MIXNODE_DELEGATORS_PAGE_LIMIT {
                let delegation = test_helpers::read_delegation(
                    &deps.storage,
                    "MIXNODE_DELEGATORS_PAGE_LIMIT+1_delegators",
                    format!("delegator{:04}", i),
                )
                .unwrap();

                assert!(delegation.amount.amount > Uint128::new(delegation_value));
            }

            let delegation = test_helpers::read_delegation(
                &deps.storage,
                "MIXNODE_DELEGATORS_PAGE_LIMIT+1_delegators",
                format!("delegator{:04}", MIXNODE_DELEGATORS_PAGE_LIMIT),
            )
            .unwrap();

            assert_eq!(delegation.amount.amount, Uint128::new(delegation_value));
        }
    }

    #[test]
    fn rewarding_mix_delegators_return_consistent_results() {
        // with single page
        let mut deps = test_helpers::init_contract();
        let mut env = mock_env();

        let node_identity = "bobsnode".to_string();
        try_add_mixnode(
            deps.as_mut(),
            env.clone(),
            mock_info("bob", &[coin(10000_000_000, DENOM)]),
            MixNode {
                identity_key: node_identity.clone(),
                ..test_helpers::mix_node_fixture()
            },
        )
        .unwrap();
        let bond = mixnodes_storage::read_full_mixnode_bond(deps.as_ref().storage, &*node_identity)
            .unwrap()
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

        env.block.height += storage::MINIMUM_BLOCK_AGE_FOR_REWARDING + 1;
        let mut node_rewarding_params = test_helpers::node_rewarding_params_fixture(100);
        node_rewarding_params.set_reward_blockstamp(env.block.height);

        let params = DelegatorRewardParams::new(&bond, node_rewarding_params);
        let res = reward_mix_delegators(deps.as_mut().storage, node_identity.clone(), None, params)
            .unwrap();

        let mut actual_reward = Uint128::new(0);
        for delegation in delegations_storage::delegations()
            .idx
            .mixnode
            .prefix(node_identity.clone())
            .range(deps.as_ref().storage, None, None, Order::Ascending)
        {
            actual_reward +=
                Uint128::new(delegation.unwrap().1.amount.amount.u128() - base_delegation);
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
                ..test_helpers::mix_node_fixture()
            },
        )
        .unwrap();
        let bond = mixnodes_storage::read_full_mixnode_bond(deps.as_ref().storage, &*node_identity)
            .unwrap()
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

        env.block.height += storage::MINIMUM_BLOCK_AGE_FOR_REWARDING + 1;
        let mut node_rewarding_params = test_helpers::node_rewarding_params_fixture(100);
        node_rewarding_params.set_reward_blockstamp(env.block.height);

        let params = DelegatorRewardParams::new(&bond, node_rewarding_params);
        let res = reward_mix_delegators(deps.as_mut().storage, node_identity.clone(), None, params)
            .unwrap();

        let mut actual_reward = Uint128::new(0);
        for delegation in delegations_storage::delegations()
            .idx
            .mixnode
            .prefix(node_identity.clone())
            .range(deps.as_ref().storage, None, None, Order::Ascending)
        {
            let (primary_key, delegation) = delegation.unwrap();
            let delegator_reward = Uint128::new(delegation.amount.amount.u128() - base_delegation);
            actual_reward += delegator_reward;

            // we start from index 2 as first 2 bytes are used to indicate length of first part
            // of the composite key
            let id_delegator = String::from_utf8_lossy(&primary_key[2..]);

            let delegator_id: usize = id_delegator
                .strip_prefix(&node_identity)
                .unwrap()
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

        let res2 = reward_mix_delegators(
            deps.as_mut().storage,
            node_identity.clone(),
            res.start_next.clone(),
            params,
        )
        .unwrap();

        let start = res.start_next.unwrap();
        let mut actual_reward = Uint128::new(0);

        let start = Bound::Inclusive((node_identity.clone(), start).joined_key());
        for delegation in delegations_storage::delegations()
            .idx
            .mixnode
            .prefix(node_identity.clone())
            .range(deps.as_ref().storage, Some(start), None, Order::Ascending)
        {
            actual_reward +=
                Uint128::new(delegation.unwrap().1.amount.amount.u128() - base_delegation);
        }

        assert_eq!(actual_reward, res2.total_rewarded);
        assert!(res2.start_next.is_none());
    }

    #[cfg(test)]
    mod delegator_rewarding_tx {
        use super::*;

        #[test]
        fn can_only_be_called_by_specified_validator_address() {
            let mut deps = test_helpers::init_contract();

            let res = try_reward_next_mixnode_delegators(
                deps.as_mut(),
                mock_info("not-the-approved-validator", &[]),
                "alice's mixnode".to_string(),
                1,
            );
            assert_eq!(Err(ContractError::Unauthorized), res);
        }

        #[test]
        fn cannot_be_called_if_rewarding_is_not_in_progress() {
            let mut deps = test_helpers::init_contract();
            let current_state = mixnet_params_storage::CONTRACT_STATE
                .load(deps.as_mut().storage)
                .unwrap();
            let rewarding_validator_address = current_state.rewarding_validator_address;

            let res = try_reward_next_mixnode_delegators(
                deps.as_mut(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                "alice's mixnode".to_string(),
                1,
            );

            assert_eq!(Err(ContractError::RewardingNotInProgress), res);
        }

        #[test]
        fn cannot_be_called_if_mixnodes_operator_wasnt_rewarded() {
            let mut deps = test_helpers::init_contract();
            let env = mock_env();
            let current_state = mixnet_params_storage::CONTRACT_STATE
                .load(deps.as_mut().storage)
                .unwrap();
            let rewarding_validator_address = current_state.rewarding_validator_address;

            try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            )
            .unwrap();

            let res = try_reward_next_mixnode_delegators(
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
            let mut deps = test_helpers::init_contract();
            let mut env = mock_env();
            let current_state = mixnet_params_storage::CONTRACT_STATE
                .load(deps.as_mut().storage)
                .unwrap();
            let rewarding_validator_address = current_state.rewarding_validator_address;

            try_add_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info(
                    "alice",
                    &vec![Coin {
                        denom: DENOM.to_string(),
                        amount: Uint128::new(10000_000_000),
                    }],
                ),
                MixNode {
                    identity_key: "alice".to_string(),
                    ..test_helpers::mix_node_fixture()
                },
            )
            .unwrap();

            env.block.height += storage::MINIMUM_BLOCK_AGE_FOR_REWARDING;

            try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            )
            .unwrap();

            try_reward_mixnode(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                "alice".to_string(),
                test_helpers::node_rewarding_params_fixture(100),
                1,
            )
            .unwrap();

            let res = try_reward_next_mixnode_delegators(
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
                        amount: Uint128::new(10000_000_000),
                    }],
                ),
                MixNode {
                    identity_key: "bob".to_string(),
                    ..test_helpers::mix_node_fixture()
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

            env.block.height += storage::MINIMUM_BLOCK_AGE_FOR_REWARDING;

            let info = mock_info(rewarding_validator_address.as_ref(), &[]);
            try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                2,
            )
            .unwrap();

            try_reward_mixnode(
                deps.as_mut(),
                env.clone(),
                info,
                "bob".to_string(),
                test_helpers::node_rewarding_params_fixture(100),
                2,
            )
            .unwrap();

            // rewards all pending
            try_reward_next_mixnode_delegators(
                deps.as_mut(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                "bob".to_string(),
                2,
            )
            .unwrap();

            let res = try_reward_next_mixnode_delegators(
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
            let mut deps = test_helpers::init_contract();
            let mut env = mock_env();
            let current_state = mixnet_params_storage::CONTRACT_STATE
                .load(deps.as_mut().storage)
                .unwrap();
            let rewarding_validator_address = current_state.rewarding_validator_address;

            let mix_bond = Uint128::new(10000_000_000);
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
                    ..test_helpers::mix_node_fixture()
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

            env.block.height += storage::MINIMUM_BLOCK_AGE_FOR_REWARDING;

            let info = mock_info(rewarding_validator_address.as_ref(), &[]);
            try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            )
            .unwrap();

            try_reward_mixnode(
                deps.as_mut(),
                env.clone(),
                info,
                "alice".to_string(),
                test_helpers::node_rewarding_params_fixture(100),
                1,
            )
            .unwrap();

            // we have 3 pages in total, so we have to call this twice
            try_reward_next_mixnode_delegators(
                deps.as_mut(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                "alice".to_string(),
                1,
            )
            .unwrap();
            try_reward_next_mixnode_delegators(
                deps.as_mut(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                "alice".to_string(),
                1,
            )
            .unwrap();

            let expected = delegations_storage::delegations()
                .load(
                    deps.as_ref().storage,
                    ("alice", "delegator0001").joined_key(),
                )
                .unwrap()
                .amount;

            for i in 0..total_delegators {
                // everyone was rewarded (and the same amount, because they all delegated the same amount)
                let delegation = test_helpers::read_delegation(
                    &deps.storage,
                    "alice",
                    format!("delegator{:04}", i),
                )
                .unwrap();

                assert!(delegation.amount.amount > Uint128::new(delegation_value));
                assert_eq!(expected, delegation.amount)
            }
        }

        #[test]
        fn ignores_delegators_that_updated_their_pledge_in_the_meantime() {
            // setup: bond > page limit delegators, reward operator + first batch
            let mut deps = test_helpers::init_contract();
            let mut env = mock_env();
            let current_state = mixnet_params_storage::CONTRACT_STATE
                .load(deps.as_mut().storage)
                .unwrap();
            let rewarding_validator_address = current_state.rewarding_validator_address;

            let mix_bond = Uint128::new(10000_000_000);
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
                    ..test_helpers::mix_node_fixture()
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

            env.block.height += storage::MINIMUM_BLOCK_AGE_FOR_REWARDING;

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

            try_reward_mixnode(
                deps.as_mut(),
                env.clone(),
                info,
                "alice".to_string(),
                test_helpers::node_rewarding_params_fixture(100),
                1,
            )
            .unwrap();

            // we have 3 pages in total, so we have to call this twice
            try_reward_next_mixnode_delegators(
                deps.as_mut(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                "alice".to_string(),
                1,
            )
            .unwrap();

            let expected = test_helpers::read_delegation(&deps.storage, "alice", "delegator0001")
                .unwrap()
                .amount;

            for i in 0..total_delegators {
                // everyone was rewarded (and the same amount, because they all delegated the same amount)
                let delegation = test_helpers::read_delegation(
                    &deps.storage,
                    "alice",
                    format!("delegator{:04}", i),
                )
                .unwrap();

                if i == 123 || i == 123 + MIXNODE_DELEGATORS_PAGE_LIMIT {
                    assert_eq!(delegation.amount.amount, Uint128::new(2 * delegation_value))
                } else {
                    assert!(delegation.amount.amount > Uint128::new(delegation_value));
                    assert_eq!(expected, delegation.amount)
                }
            }
        }
    }
}
