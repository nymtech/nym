// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use cosmwasm_std::Uint128;
use cosmwasm_std::{Deps, StdResult};
use mixnet_contract_common::{IdentityKey, MixnodeRewardingStatusResponse};

pub(crate) fn query_reward_pool(deps: Deps) -> StdResult<Uint128> {
    storage::REWARD_POOL.load(deps.storage)
}

pub(crate) fn query_circulating_supply(deps: Deps) -> StdResult<Uint128> {
    storage::circulating_supply(deps.storage)
}

pub(crate) fn query_rewarding_status(
    deps: Deps,
    mix_identity: IdentityKey,
    rewarding_interval_nonce: u32,
) -> StdResult<MixnodeRewardingStatusResponse> {
    let status = storage::REWARDING_STATUS
        .may_load(deps.storage, (rewarding_interval_nonce, mix_identity))?;

    Ok(MixnodeRewardingStatusResponse { status })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::mixnet_contract_settings::storage as mixnet_params_storage;
    use crate::support::tests;
    use crate::support::tests::test_helpers;
    use cosmwasm_std::testing::{mock_env, mock_info};

    #[cfg(test)]
    mod querying_for_rewarding_status {
        use super::storage;
        use super::*;
        use crate::delegations::transactions::try_delegate_to_mixnode;
        use crate::rewards::transactions::{
            try_begin_mixnode_rewarding, try_finish_mixnode_rewarding, try_reward_mixnode,
            try_reward_next_mixnode_delegators,
        };
        use config::defaults::DENOM;
        use cosmwasm_std::{coin, Addr};
        use mixnet_contract_common::{
            RewardingResult, RewardingStatus, MIXNODE_DELEGATORS_PAGE_LIMIT,
        };

        #[test]
        fn returns_empty_status_for_unrewarded_nodes() {
            let mut deps = test_helpers::init_contract();
            let env = mock_env();
            let current_state = mixnet_params_storage::CONTRACT_STATE
                .load(deps.as_mut().storage)
                .unwrap();
            let rewarding_validator_address = current_state.rewarding_validator_address;

            let node_identity = test_helpers::add_mixnode(
                "bob",
                tests::fixtures::good_mixnode_pledge(),
                deps.as_mut(),
            );

            assert!(
                query_rewarding_status(deps.as_ref(), node_identity.clone(), 1)
                    .unwrap()
                    .status
                    .is_none()
            );

            // node was rewarded but for different epoch
            let info = mock_info(rewarding_validator_address.as_ref(), &[]);
            try_begin_mixnode_rewarding(deps.as_mut(), env.clone(), info.clone(), 1).unwrap();
            try_reward_mixnode(
                deps.as_mut(),
                env,
                info.clone(),
                node_identity.clone(),
                tests::fixtures::node_rewarding_params_fixture(100),
                1,
            )
            .unwrap();
            try_finish_mixnode_rewarding(deps.as_mut(), info, 1).unwrap();

            assert!(query_rewarding_status(deps.as_ref(), node_identity, 2)
                .unwrap()
                .status
                .is_none());
        }

        #[test]
        fn returns_complete_status_for_fully_rewarded_node() {
            // with single page
            let mut deps = test_helpers::init_contract();
            let mut env = mock_env();
            let current_state = mixnet_params_storage::CONTRACT_STATE
                .load(deps.as_mut().storage)
                .unwrap();
            let rewarding_validator_address = current_state.rewarding_validator_address;

            let node_owner: Addr = Addr::unchecked("bob");
            let node_identity = test_helpers::add_mixnode(
                node_owner.as_str(),
                tests::fixtures::good_mixnode_pledge(),
                deps.as_mut(),
            );

            env.block.height += storage::MINIMUM_BLOCK_AGE_FOR_REWARDING;

            let info = mock_info(rewarding_validator_address.as_ref(), &[]);
            try_begin_mixnode_rewarding(deps.as_mut(), env.clone(), info.clone(), 1).unwrap();
            try_reward_mixnode(
                deps.as_mut(),
                env.clone(),
                info.clone(),
                node_identity.clone(),
                tests::fixtures::node_rewarding_params_fixture(100),
                1,
            )
            .unwrap();
            try_finish_mixnode_rewarding(deps.as_mut(), info, 1).unwrap();

            let res = query_rewarding_status(deps.as_ref(), node_identity, 1).unwrap();
            assert!(matches!(res.status, Some(RewardingStatus::Complete(..))));

            match res.status.unwrap() {
                RewardingStatus::Complete(result) => {
                    assert_ne!(
                        RewardingResult::default().operator_reward,
                        result.operator_reward
                    );
                    assert_eq!(
                        RewardingResult::default().total_delegator_reward,
                        result.total_delegator_reward
                    );
                }
                _ => unreachable!(),
            }

            // with multiple pages
            let node_owner: Addr = Addr::unchecked("alice");
            let node_identity = test_helpers::add_mixnode(
                node_owner.as_str(),
                tests::fixtures::good_mixnode_pledge(),
                deps.as_mut(),
            );

            for i in 0..MIXNODE_DELEGATORS_PAGE_LIMIT + 123 {
                try_delegate_to_mixnode(
                    deps.as_mut(),
                    env.clone(),
                    mock_info(&*format!("delegator{:04}", i), &[coin(200_000000, DENOM)]),
                    node_identity.clone(),
                )
                .unwrap();
            }

            env.block.height += storage::MINIMUM_BLOCK_AGE_FOR_REWARDING;

            let info = mock_info(rewarding_validator_address.as_ref(), &[]);
            try_begin_mixnode_rewarding(deps.as_mut(), env.clone(), info.clone(), 2).unwrap();

            try_reward_mixnode(
                deps.as_mut(),
                env,
                info.clone(),
                node_identity.clone(),
                tests::fixtures::node_rewarding_params_fixture(100),
                2,
            )
            .unwrap();

            // rewards all pending
            try_reward_next_mixnode_delegators(deps.as_mut(), info, node_identity.to_string(), 2)
                .unwrap();

            let res = query_rewarding_status(deps.as_ref(), node_identity, 2).unwrap();
            assert!(matches!(res.status, Some(RewardingStatus::Complete(..))));

            match res.status.unwrap() {
                RewardingStatus::Complete(result) => {
                    assert_ne!(
                        RewardingResult::default().operator_reward,
                        result.operator_reward
                    );
                    assert_ne!(
                        RewardingResult::default().total_delegator_reward,
                        result.total_delegator_reward
                    );
                }
                _ => unreachable!(),
            }
        }

        #[test]
        fn returns_pending_next_delegator_page_status_when_there_are_more_delegators_to_reward() {
            let mut deps = test_helpers::init_contract();
            let mut env = mock_env();
            let current_state = mixnet_params_storage::CONTRACT_STATE
                .load(deps.as_mut().storage)
                .unwrap();
            let rewarding_validator_address = current_state.rewarding_validator_address;

            let node_owner: Addr = Addr::unchecked("bob");
            let node_identity = test_helpers::add_mixnode(
                node_owner.as_str(),
                tests::fixtures::good_mixnode_pledge(),
                deps.as_mut(),
            );

            for i in 0..MIXNODE_DELEGATORS_PAGE_LIMIT + 123 {
                try_delegate_to_mixnode(
                    deps.as_mut(),
                    env.clone(),
                    mock_info(&*format!("delegator{:04}", i), &[coin(200_000000, DENOM)]),
                    node_identity.clone(),
                )
                .unwrap();
            }

            env.block.height += storage::MINIMUM_BLOCK_AGE_FOR_REWARDING;

            let info = mock_info(rewarding_validator_address.as_ref(), &[]);
            try_begin_mixnode_rewarding(deps.as_mut(), env.clone(), info.clone(), 1).unwrap();

            try_reward_mixnode(
                deps.as_mut(),
                env,
                info,
                node_identity.clone(),
                tests::fixtures::node_rewarding_params_fixture(100),
                1,
            )
            .unwrap();

            let res = query_rewarding_status(deps.as_ref(), node_identity, 1).unwrap();
            assert!(matches!(
                res.status,
                Some(RewardingStatus::PendingNextDelegatorPage(..))
            ));

            match res.status.unwrap() {
                RewardingStatus::PendingNextDelegatorPage(result) => {
                    assert_ne!(
                        RewardingResult::default().operator_reward,
                        result.running_results.operator_reward
                    );
                    assert_ne!(
                        RewardingResult::default().total_delegator_reward,
                        result.running_results.total_delegator_reward
                    );
                    assert_eq!(
                        &*format!("delegator{:04}", MIXNODE_DELEGATORS_PAGE_LIMIT),
                        result.next_start
                    );
                }
                _ => unreachable!(),
            }
        }
    }
}
