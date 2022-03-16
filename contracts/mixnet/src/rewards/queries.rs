// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use cosmwasm_std::Uint128;
use cosmwasm_std::{Deps, StdResult};
use mixnet_contract_common::{IdentityKey, MixnodeRewardingStatusResponse};

pub(crate) fn query_reward_pool(deps: Deps<'_>) -> StdResult<Uint128> {
    storage::REWARD_POOL.load(deps.storage)
}

pub(crate) fn query_circulating_supply(deps: Deps<'_>) -> StdResult<Uint128> {
    storage::circulating_supply(deps.storage)
}

pub(crate) fn query_rewarding_status(
    deps: Deps<'_>,
    mix_identity: IdentityKey,
    interval_id: u32,
) -> StdResult<MixnodeRewardingStatusResponse> {
    let status = storage::REWARDING_STATUS.may_load(deps.storage, (interval_id, mix_identity))?;

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
        use super::*;
        use crate::delegations::transactions::try_delegate_to_mixnode;
        use crate::interval::storage::{save_epoch, save_epoch_reward_params};
        use crate::rewards::transactions::try_reward_mixnode;
        use crate::{constants};
        use config::defaults::DENOM;
        use cosmwasm_std::{coin, Addr};
        use mixnet_contract_common::{
            RewardingResult, RewardingStatus, MIXNODE_DELEGATORS_PAGE_LIMIT, Interval,
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

            // node was rewarded but for different interval
            let info = mock_info(rewarding_validator_address.as_ref(), &[]);
            try_reward_mixnode(
                deps.as_mut(),
                env,
                info,
                node_identity.clone(),
                tests::fixtures::node_reward_params_fixture(100),
                0,
            )
            .unwrap();

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

            env.block.height += constants::MINIMUM_BLOCK_AGE_FOR_REWARDING;

            let info = mock_info(rewarding_validator_address.as_ref(), &[]);

            let epoch = Interval::init_epoch(env.clone());
            save_epoch(&mut deps.storage, &epoch).unwrap();
            save_epoch_reward_params(epoch.id(), &mut deps.storage).unwrap();

            try_reward_mixnode(
                deps.as_mut(),
                env.clone(),
                info,
                node_identity.clone(),
                tests::fixtures::node_reward_params_fixture(100),
                0,
            )
            .unwrap();

            let res = query_rewarding_status(deps.as_ref(), node_identity, 0).unwrap();
            assert!(matches!(res.status, Some(RewardingStatus::Complete(..))));

            match res.status.unwrap() {
                RewardingStatus::Complete(result) => {
                    assert_ne!(RewardingResult::default().node_reward, result.node_reward);
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

            env.block.height += constants::MINIMUM_BLOCK_AGE_FOR_REWARDING;
            test_helpers::update_env_and_progress_interval(&mut env, deps.as_mut().storage);

            let info = mock_info(rewarding_validator_address.as_ref(), &[]);

            try_reward_mixnode(
                deps.as_mut(),
                env,
                info.clone(),
                node_identity.clone(),
                tests::fixtures::node_reward_params_fixture(100),
                1,
            )
            .unwrap();

            // rewards all pending
            // try_reward_next_mixnode_delegators(deps.as_mut(), info, node_identity.to_string(), 1)
            //     .unwrap();

            let res = query_rewarding_status(deps.as_ref(), node_identity, 1).unwrap();
            assert!(matches!(res.status, Some(RewardingStatus::Complete(..))));

            match res.status.unwrap() {
                RewardingStatus::Complete(result) => {
                    assert_ne!(RewardingResult::default().node_reward, result.node_reward);
                }
                _ => unreachable!(),
            }
        }
    }
}
