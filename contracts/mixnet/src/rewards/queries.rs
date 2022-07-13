// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::delegations::storage as delegations_storage;
use crate::mixnodes;
use cosmwasm_std::{Deps, StdResult};
use mixnet_contract_common::mixnode::MixNodeDetails;
use mixnet_contract_common::reward_params::RewardingParams;
use mixnet_contract_common::rewarding::PendingRewardResponse;
use mixnet_contract_common::{Delegation, NodeId};

pub(crate) fn query_rewarding_params(deps: Deps<'_>) -> StdResult<RewardingParams> {
    storage::REWARDING_PARAMS.load(deps.storage)
}

fn pending_operator_reward(mix_details: Option<MixNodeDetails>) -> PendingRewardResponse {
    match mix_details {
        Some(mix_rewarding) => PendingRewardResponse {
            amount_staked: Some(mix_rewarding.original_pledge().clone()),
            amount_earned: Some(mix_rewarding.pending_operator_reward()),
            mixnode_still_bonded: true,
        },
        None => PendingRewardResponse::default(),
    }
}

pub fn query_pending_operator_reward(
    deps: Deps,
    owner: String,
) -> StdResult<PendingRewardResponse> {
    let owner_address = deps.api.addr_validate(&owner)?;
    // in order to determine operator's reward we need to know its original pledge and thus
    // we have to load the entire thing
    let mix_details = mixnodes::helpers::get_mixnode_details_by_owner(deps.storage, owner_address)?;
    Ok(pending_operator_reward(mix_details))
}

pub fn query_pending_mixnode_operator_reward(
    deps: Deps,
    mix_id: NodeId,
) -> StdResult<PendingRewardResponse> {
    // in order to determine operator's reward we need to know its original pledge and thus
    // we have to load the entire thing
    let mix_details = mixnodes::helpers::get_mixnode_details_by_id(deps.storage, mix_id)?;
    Ok(pending_operator_reward(mix_details))
}

pub fn query_pending_delegator_reward(
    deps: Deps,
    owner: String,
    mix_id: NodeId,
    proxy: Option<String>,
) -> StdResult<PendingRewardResponse> {
    let owner_address = deps.api.addr_validate(&owner)?;
    let proxy = proxy
        .map(|proxy| deps.api.addr_validate(&proxy))
        .transpose()?;

    let mix_rewarding = match storage::MIXNODE_REWARDING.may_load(deps.storage, mix_id)? {
        Some(mix_rewarding) => mix_rewarding,
        None => return Ok(PendingRewardResponse::default()),
    };

    let storage_key = Delegation::generate_storage_key(mix_id, &owner_address, proxy.as_ref());
    let delegation = match delegations_storage::delegations().may_load(deps.storage, storage_key)? {
        Some(delegation) => delegation,
        None => return Ok(PendingRewardResponse::default()),
    };

    let delegator_reward = mix_rewarding.pending_delegator_reward(&delegation);

    Ok(PendingRewardResponse {
        amount_staked: Some(delegation.amount),
        amount_earned: Some(delegator_reward),
        mixnode_still_bonded: mix_rewarding.still_bonded(),
    })
}

//
// #[cfg(test)]
// pub(crate) mod tests {
//     use super::*;
//     use crate::mixnet_contract_settings::storage as mixnet_params_storage;
//     use crate::support::tests;
//     use crate::support::tests::test_helpers;
//     use cosmwasm_std::testing::{mock_env, mock_info};
//
//     #[cfg(test)]
//     mod querying_for_rewarding_status {
//         use super::*;
//         use crate::constants;
//         use crate::delegations::transactions::try_delegate_to_mixnode;
//         use crate::interval::storage::{save_epoch, save_epoch_reward_params};
//         use crate::rewards::transactions::try_reward_mixnode;
//         use config::defaults::MIX_DENOM;
//         use cosmwasm_std::{coin, Addr};
//         use mixnet_contract_common::{
//             Interval, RewardingResult, RewardingStatus, MIXNODE_DELEGATORS_PAGE_LIMIT,
//         };
//
//         #[test]
//         fn returns_empty_status_for_unrewarded_nodes() {
//             let mut deps = test_helpers::init_contract();
//             let env = mock_env();
//             let current_state = mixnet_params_storage::CONTRACT_STATE
//                 .load(deps.as_mut().storage)
//                 .unwrap();
//             let rewarding_validator_address = current_state.rewarding_validator_address;
//
//             let node_identity = test_helpers::add_mixnode(
//                 "bob",
//                 tests::fixtures::good_mixnode_pledge(),
//                 deps.as_mut(),
//             );
//
//             assert!(
//                 query_rewarding_status(deps.as_ref(), node_identity.clone(), 1)
//                     .unwrap()
//                     .status
//                     .is_none()
//             );
//
//             // node was rewarded but for different interval
//             let info = mock_info(rewarding_validator_address.as_ref(), &[]);
//             try_reward_mixnode(
//                 deps.as_mut(),
//                 env,
//                 info.clone(),
//                 node_identity.clone(),
//                 tests::fixtures::node_reward_params_fixture(100),
//             )
//             .unwrap();
//
//             assert!(query_rewarding_status(deps.as_ref(), node_identity, 2)
//                 .unwrap()
//                 .status
//                 .is_none());
//         }
//
//         #[test]
//
//         fn returns_complete_status_for_fully_rewarded_node() {
//             // with single page
//             let mut deps = test_helpers::init_contract();
//             let mut env = mock_env();
//             let current_state = mixnet_params_storage::CONTRACT_STATE
//                 .load(deps.as_mut().storage)
//                 .unwrap();
//             let rewarding_validator_address = current_state.rewarding_validator_address;
//
//             let node_owner: Addr = Addr::unchecked("bob");
//             let node_identity = test_helpers::add_mixnode(
//                 node_owner.as_str(),
//                 tests::fixtures::good_mixnode_pledge(),
//                 deps.as_mut(),
//             );
//
//             env.block.height += constants::MINIMUM_BLOCK_AGE_FOR_REWARDING;
//
//             let info = mock_info(rewarding_validator_address.as_ref(), &[]);
//
//             let epoch = Interval::init_epoch(env.clone());
//             save_epoch(&mut deps.storage, &epoch).unwrap();
//             save_epoch_reward_params(epoch.id(), &mut deps.storage).unwrap();
//
//             try_reward_mixnode(
//                 deps.as_mut(),
//                 env.clone(),
//                 info.clone(),
//                 node_identity.clone(),
//                 tests::fixtures::node_reward_params_fixture(100),
//             )
//             .unwrap();
//
//             let res = query_rewarding_status(deps.as_ref(), node_identity, 0).unwrap();
//             assert!(matches!(res.status, Some(RewardingStatus::Complete(..))));
//
//             match res.status.unwrap() {
//                 RewardingStatus::Complete(result) => {
//                     assert_ne!(RewardingResult::default().node_reward, result.node_reward);
//                 }
//                 _ => unreachable!(),
//             }
//
//             // with multiple pages
//             let node_owner: Addr = Addr::unchecked("alice");
//             let node_identity = test_helpers::add_mixnode(
//                 node_owner.as_str(),
//                 tests::fixtures::good_mixnode_pledge(),
//                 deps.as_mut(),
//             );
//
//             for i in 0..MIXNODE_DELEGATORS_PAGE_LIMIT + 123 {
//                 try_delegate_to_mixnode(
//                     deps.as_mut(),
//                     env.clone(),
//                     mock_info(
//                         &*format!("delegator{:04}", i),
//                         &[coin(200_000000, MIX_DENOM.base)],
//                     ),
//                     node_identity.clone(),
//                 )
//                 .unwrap();
//             }
//
//             env.block.height += constants::MINIMUM_BLOCK_AGE_FOR_REWARDING;
//             test_helpers::update_env_and_progress_interval(&mut env, deps.as_mut().storage);
//
//             let info = mock_info(rewarding_validator_address.as_ref(), &[]);
//
//             try_reward_mixnode(
//                 deps.as_mut(),
//                 env,
//                 info,
//                 node_identity.clone(),
//                 tests::fixtures::node_reward_params_fixture(100),
//             )
//             .unwrap();
//
//             // rewards all pending
//             // try_reward_next_mixnode_delegators(deps.as_mut(), info, node_identity.to_string(), 1)
//             //     .unwrap();
//
//             let res = query_rewarding_status(deps.as_ref(), node_identity, 1).unwrap();
//             assert!(matches!(res.status, Some(RewardingStatus::Complete(..))));
//
//             match res.status.unwrap() {
//                 RewardingStatus::Complete(result) => {
//                     assert_ne!(RewardingResult::default().node_reward, result.node_reward);
//                 }
//                 _ => unreachable!(),
//             }
//         }
//     }
// }
