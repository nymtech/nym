// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// // will return details of either nym-node, legacy mixnode or legacy gateway
// pub fn query_owned_node() {
//     todo!()
// }

pub(crate) mod rewards {
    use crate::mixnodes::helpers::{get_mixnode_details_by_id, get_mixnode_details_by_owner};
    use crate::nodes::helpers::{get_node_details_by_id, get_node_details_by_owner};
    use cosmwasm_std::{Addr, Deps, StdResult};
    use mixnet_contract_common::{NodeId, PendingRewardResponse};

    #[allow(deprecated)]
    pub(crate) fn pending_operator_reward(
        deps: Deps<'_>,
        operator: Addr,
    ) -> StdResult<PendingRewardResponse> {
        // check if owns mixnode or nymnode and query accordingly
        if let Some(nym_node_details) = get_node_details_by_owner(deps.storage, operator.clone())? {
            Ok(PendingRewardResponse {
                amount_staked: Some(nym_node_details.original_pledge().clone()),
                amount_earned: Some(nym_node_details.pending_operator_reward()),
                amount_earned_detailed: Some(nym_node_details.pending_detailed_operator_reward()?),
                mixnode_still_fully_bonded: !nym_node_details.is_unbonding(),
                node_still_fully_bonded: !nym_node_details.is_unbonding(),
            })
        } else if let Some(legacy_mixnode_details) =
            get_mixnode_details_by_owner(deps.storage, operator)?
        {
            Ok(PendingRewardResponse {
                amount_staked: Some(legacy_mixnode_details.original_pledge().clone()),
                amount_earned: Some(legacy_mixnode_details.pending_operator_reward()),
                amount_earned_detailed: Some(
                    legacy_mixnode_details.pending_detailed_operator_reward()?,
                ),
                mixnode_still_fully_bonded: !legacy_mixnode_details.is_unbonding(),
                node_still_fully_bonded: !legacy_mixnode_details.is_unbonding(),
            })
        } else {
            Ok(PendingRewardResponse::default())
        }
    }

    #[allow(deprecated)]
    pub(crate) fn pending_operator_reward_by_id(
        deps: Deps<'_>,
        node_id: NodeId,
    ) -> StdResult<PendingRewardResponse> {
        // check if owns mixnode or nymnode and query accordingly
        if let Some(nym_node_details) = get_node_details_by_id(deps.storage, node_id)? {
            Ok(PendingRewardResponse {
                amount_staked: Some(nym_node_details.original_pledge().clone()),
                amount_earned: Some(nym_node_details.pending_operator_reward()),
                amount_earned_detailed: Some(nym_node_details.pending_detailed_operator_reward()?),
                mixnode_still_fully_bonded: !nym_node_details.is_unbonding(),
                node_still_fully_bonded: !nym_node_details.is_unbonding(),
            })
        } else if let Some(legacy_mixnode_details) =
            get_mixnode_details_by_id(deps.storage, node_id)?
        {
            Ok(PendingRewardResponse {
                amount_staked: Some(legacy_mixnode_details.original_pledge().clone()),
                amount_earned: Some(legacy_mixnode_details.pending_operator_reward()),
                amount_earned_detailed: Some(
                    legacy_mixnode_details.pending_detailed_operator_reward()?,
                ),
                mixnode_still_fully_bonded: !legacy_mixnode_details.is_unbonding(),
                node_still_fully_bonded: !legacy_mixnode_details.is_unbonding(),
            })
        } else {
            Ok(PendingRewardResponse::default())
        }
    }
}
