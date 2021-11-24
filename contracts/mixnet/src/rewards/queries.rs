// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::mixnet_contract_settings::storage as mixnet_params_storage;
use crate::mixnodes::storage as mixnodes_storage;
use cosmwasm_std::Uint128;
use cosmwasm_std::{Deps, StdResult};
use mixnet_contract::{IdentityKey, MixnodeRewardingStatusResponse};

pub(crate) fn query_reward_pool(deps: Deps) -> Uint128 {
    storage::reward_pool_value(deps.storage)
}

pub(crate) fn query_circulating_supply(deps: Deps) -> Uint128 {
    storage::circulating_supply(deps.storage)
}

pub(crate) fn query_rewarding_status(
    deps: Deps,
    mix_identity: IdentityKey,
    rewarding_interval_nonce: u32,
) -> StdResult<MixnodeRewardingStatusResponse> {
    let status = mixnodes_storage::rewarded_mixnodes_read(deps.storage, rewarding_interval_nonce)
        .may_load(mix_identity.as_bytes())?;

    Ok(MixnodeRewardingStatusResponse { status })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::state::State;
    use crate::storage::{config, gateways, mix_delegations};
    use crate::support::tests::helpers;
    use crate::support::tests::helpers::{
        good_gateway_bond, raw_delegation_fixture, test_helpers::good_mixnode_bond,
    };
    use crate::transactions;
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::{Addr, Storage};
    use mixnet_contract::{Gateway, MixNode, RawDelegationData};

    pub fn store_n_mix_delegations(n: u32, storage: &mut dyn Storage, node_identity: &IdentityKey) {
        for i in 0..n {
            let address = format!("address{}", i);
            mix_delegations(storage, node_identity)
                .save(address.as_bytes(), &raw_delegation_fixture(42))
                .unwrap();
        }
    }

    #[cfg(test)]
    mod querying_for_rewarding_status {
        use super::*;
        use crate::support::tests::helpers::{add_mixnode, node_rewarding_params_fixture};
        use crate::transactions::{
            try_add_mixnode, try_begin_mixnode_rewarding, try_delegate_to_mixnode,
            try_finish_mixnode_rewarding, try_reward_mixnode_v2,
            try_reward_next_mixnode_delegators_v2, MINIMUM_BLOCK_AGE_FOR_REWARDING,
        };
        use mixnet_contract::{RewardingResult, RewardingStatus, MIXNODE_DELEGATORS_PAGE_LIMIT};

        #[test]
        fn returns_empty_status_for_unrewarded_nodes() {
            let mut deps = test_helpers::init_contract();
            let env = mock_env();
            let current_state = config_read(deps.as_mut().storage).load().unwrap();
            let rewarding_validator_address = current_state.rewarding_validator_address;

            let node_identity =
                add_mixnode("bob", test_helpers::good_mixnode_bond(), deps.as_mut());

            assert!(
                query_rewarding_status(deps.as_ref(), node_identity.clone(), 1)
                    .unwrap()
                    .status
                    .is_none()
            );

            // node was rewarded but for different epoch
            let info = mock_info(rewarding_validator_address.as_ref(), &[]);
            try_begin_mixnode_rewarding(deps.as_mut(), env.clone(), info.clone(), 1).unwrap();
            try_reward_mixnode_v2(
                deps.as_mut(),
                env.clone(),
                info.clone(),
                node_identity.clone(),
                node_rewarding_params_fixture(100),
                1,
            )
            .unwrap();
            try_finish_mixnode_rewarding(deps.as_mut(), info.clone(), 1).unwrap();

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
            let current_state = config_read(deps.as_mut().storage).load().unwrap();
            let rewarding_validator_address = current_state.rewarding_validator_address;

            let node_identity = "bobsnode".to_string();
            try_add_mixnode(
                deps.as_mut(),
                env.clone(),
                mock_info("bob", &test_helpers::good_mixnode_bond()),
                MixNode {
                    identity_key: node_identity.clone(),
                    ..helpers::mix_node_fixture()
                },
            )
            .unwrap();

            env.block.height += MINIMUM_BLOCK_AGE_FOR_REWARDING;

            let info = mock_info(rewarding_validator_address.as_ref(), &[]);
            try_begin_mixnode_rewarding(deps.as_mut(), env.clone(), info.clone(), 1).unwrap();
            try_reward_mixnode_v2(
                deps.as_mut(),
                env.clone(),
                info.clone(),
                node_identity.clone(),
                node_rewarding_params_fixture(100),
                1,
            )
            .unwrap();
            try_finish_mixnode_rewarding(deps.as_mut(), info.clone(), 1).unwrap();

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

            let node_identity = "alicesnode".to_string();
            try_add_mixnode(
                deps.as_mut(),
                env.clone(),
                mock_info("alice", &test_helpers::good_mixnode_bond()),
                MixNode {
                    identity_key: node_identity.clone(),
                    ..helpers::mix_node_fixture()
                },
            )
            .unwrap();

            for i in 0..MIXNODE_DELEGATORS_PAGE_LIMIT + 123 {
                try_delegate_to_mixnode(
                    deps.as_mut(),
                    env.clone(),
                    mock_info(
                        &*format!("delegator{:04}", i),
                        &vec![coin(200_000000, DENOM)],
                    ),
                    node_identity.clone(),
                )
                .unwrap();
            }

            env.block.height += MINIMUM_BLOCK_AGE_FOR_REWARDING;

            let info = mock_info(rewarding_validator_address.as_ref(), &[]);
            try_begin_mixnode_rewarding(deps.as_mut(), env.clone(), info.clone(), 2).unwrap();

            try_reward_mixnode_v2(
                deps.as_mut(),
                env.clone(),
                info.clone(),
                node_identity.clone(),
                node_rewarding_params_fixture(100),
                2,
            )
            .unwrap();

            // rewards all pending
            try_reward_next_mixnode_delegators_v2(
                deps.as_mut(),
                info.clone(),
                node_identity.to_string(),
                2,
            )
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
            let current_state = config_read(deps.as_mut().storage).load().unwrap();
            let rewarding_validator_address = current_state.rewarding_validator_address;

            let node_identity = "bobsnode".to_string();
            try_add_mixnode(
                deps.as_mut(),
                env.clone(),
                mock_info("bob", &test_helpers::good_mixnode_bond()),
                MixNode {
                    identity_key: node_identity.clone(),
                    ..helpers::mix_node_fixture()
                },
            )
            .unwrap();

            for i in 0..MIXNODE_DELEGATORS_PAGE_LIMIT + 123 {
                try_delegate_to_mixnode(
                    deps.as_mut(),
                    env.clone(),
                    mock_info(
                        &*format!("delegator{:04}", i),
                        &vec![coin(200_000000, DENOM)],
                    ),
                    node_identity.clone(),
                )
                .unwrap();
            }

            env.block.height += MINIMUM_BLOCK_AGE_FOR_REWARDING;

            let info = mock_info(rewarding_validator_address.as_ref(), &[]);
            try_begin_mixnode_rewarding(deps.as_mut(), env.clone(), info.clone(), 1).unwrap();

            try_reward_mixnode_v2(
                deps.as_mut(),
                env.clone(),
                info,
                node_identity.clone(),
                node_rewarding_params_fixture(100),
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
