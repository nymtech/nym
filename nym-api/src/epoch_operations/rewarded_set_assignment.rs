// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::epoch_operations::error::RewardingError;
use crate::epoch_operations::helpers::stake_to_f64;
use crate::EpochAdvancer;
use cosmwasm_std::Decimal;
use nym_mixnet_contract_common::reward_params::{Performance, RewardedSetParams};
use nym_mixnet_contract_common::{EpochState, NodeId, NymNodeDetails, RewardedSet};
use rand::prelude::SliceRandom;
use rand::rngs::OsRng;
use std::collections::HashSet;
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone, PartialEq)]
enum AvailableRole {
    // legacy mixnodes + nym-nodes in mixing mode
    Mix,

    // legacy gateways + nym-nodes in entry or exit mode
    EntryGateway,

    // nym-nodes in exit mode
    ExitGateway,
}

#[derive(Debug, Clone)]
struct NodeWithStakeAndPerformance {
    node_id: NodeId,
    available_roles: Vec<AvailableRole>,
    total_stake: Decimal,
    performance: Performance,
}

impl NodeWithStakeAndPerformance {
    fn to_selection_weight(&self) -> f64 {
        let scaled_performance = match self.performance.checked_pow(20) {
            Ok(perf) => perf,
            Err(overflow) => {
                warn!("the node's performance ({}) has overflow while scaling it by the factor of 20: {overflow}. Setting it to 0 instead.", self.performance);
                return 0.;
            }
        };

        let scaled_stake = self.total_stake * scaled_performance;
        stake_to_f64(scaled_stake)
    }

    fn can_operate_mixnode(&self) -> bool {
        self.available_roles.contains(&AvailableRole::Mix)
    }

    fn can_operate_entry_gateway(&self) -> bool {
        self.available_roles.contains(&AvailableRole::EntryGateway)
    }

    fn can_operate_exit_gateway(&self) -> bool {
        self.available_roles.contains(&AvailableRole::ExitGateway)
    }
}

impl EpochAdvancer {
    fn determine_rewarded_set(
        &self,
        nodes: Vec<NodeWithStakeAndPerformance>,
        spec: RewardedSetParams,
    ) -> Result<RewardedSet, RewardingError> {
        if nodes.is_empty() {
            warn!("there are no nodes for assignment!");
            return Ok(RewardedSet::default());
        }

        let mut rng = OsRng;

        // generate list of nodes and their relatively weight (by total stake scaled by performance)
        let all_choices = nodes
            .into_iter()
            .map(|node| {
                let weight = node.to_selection_weight();
                (node, weight)
            })
            .collect::<Vec<_>>();

        // 1. determine entry gateways
        let entry_eligible = all_choices
            .iter()
            .filter(|node| node.0.can_operate_entry_gateway())
            .collect::<Vec<_>>();
        let entry_gateways = entry_eligible
            .choose_multiple_weighted(&mut rng, spec.entry_gateways as usize, |item| item.1)?
            .map(|node| node.0.node_id)
            .collect::<HashSet<_>>();

        // 2. determine exit gateways
        let exit_eligible = all_choices
            .iter()
            .filter(|node| {
                node.0.can_operate_exit_gateway() && !entry_gateways.contains(&node.0.node_id)
            })
            .collect::<Vec<_>>();
        let exit_gateways = exit_eligible
            .choose_multiple_weighted(&mut rng, spec.exit_gateways as usize, |item| item.1)?
            .map(|node| node.0.node_id)
            .collect::<HashSet<_>>();

        // 3. determine mixnodes
        let mix_eligible = all_choices
            .iter()
            .filter(|node| {
                node.0.can_operate_mixnode()
                    && !exit_gateways.contains(&node.0.node_id)
                    && !entry_gateways.contains(&node.0.node_id)
            })
            .collect::<Vec<_>>();
        let mixnodes = mix_eligible
            .choose_multiple_weighted(&mut rng, spec.mixnodes as usize, |item| item.1)?
            .map(|node| node.0.node_id)
            .collect::<HashSet<_>>();

        // 4. determine standby
        let standby_eligible = all_choices
            .iter()
            .filter(|node| {
                !exit_gateways.contains(&node.0.node_id)
                    && !entry_gateways.contains(&node.0.node_id)
                    && !mixnodes.contains(&node.0.node_id)
            })
            .collect::<Vec<_>>();
        let standby = standby_eligible
            .choose_multiple_weighted(&mut rng, spec.standby as usize, |item| item.1)?
            .map(|node| node.0.node_id)
            .collect::<Vec<_>>();

        // 5. split mixnodes into the layers: just shuffle the selected nodes and select every 3rd into each layer
        let mut mixnodes_vec = mixnodes.into_iter().collect::<Vec<_>>();
        mixnodes_vec.shuffle(&mut rng);

        let mut layer1 = Vec::new();
        let mut layer2 = Vec::new();
        let mut layer3 = Vec::new();

        #[allow(clippy::panic)]
        for (i, mix) in mixnodes_vec.iter().enumerate() {
            match i % 3 {
                0 => layer1.push(*mix),
                1 => layer2.push(*mix),
                2 => layer3.push(*mix),
                n => panic!("we have broken maths! somehow {i} % 3 == {n}!"),
            }
        }

        if entry_gateways.len() != spec.entry_gateways as usize {
            warn!(
                "we didn't manage to select {} entry gateways. we only got {}",
                spec.entry_gateways,
                entry_gateways.len()
            )
        }

        if exit_gateways.len() != spec.exit_gateways as usize {
            warn!(
                "we didn't manage to select {} exit gateways. we only got {}",
                spec.exit_gateways,
                exit_gateways.len()
            )
        }

        if mixnodes_vec.len() != spec.mixnodes as usize {
            warn!(
                "we didn't manage to select {} mixnodes. we only got {}",
                spec.mixnodes,
                mixnodes_vec.len()
            )
        }

        if standby.len() != spec.standby as usize {
            warn!(
                "we didn't manage to select {} standby nodes. we only got {}",
                spec.standby,
                standby.len()
            )
        }

        let mut rewarded_set = RewardedSet {
            entry_gateways: entry_gateways.into_iter().collect(),
            exit_gateways: exit_gateways.into_iter().collect(),
            layer1,
            layer2,
            layer3,
            standby,
        };

        // make sure to sort the rewarded set values
        rewarded_set.entry_gateways.sort();
        rewarded_set.exit_gateways.sort();
        rewarded_set.layer1.sort();
        rewarded_set.layer2.sort();
        rewarded_set.layer3.sort();
        rewarded_set.standby.sort();

        Ok(rewarded_set)
    }

    async fn attach_performance_to_eligible_nodes(
        &self,
        nym_nodes: &[NymNodeDetails],
    ) -> Vec<NodeWithStakeAndPerformance> {
        let mut with_performance = Vec::new();

        // SAFETY: the cache MUST HAVE been initialised before now
        #[allow(clippy::unwrap_used)]
        let described_cache = self.described_cache.get().await.unwrap();

        let Some(status_cache) = self.status_cache.node_annotations().await else {
            warn!("there are no node annotations available");
            return Vec::new();
        };

        for nym_node in nym_nodes {
            let node_id = nym_node.node_id();
            let total_stake = nym_node.total_stake();

            let Some(self_described) = described_cache.get_description(&node_id) else {
                continue;
            };

            let Some(annotation) = status_cache.get(&node_id) else {
                debug!("couldn't find annotation for nym-node {node_id}");
                continue;
            };

            let performance = annotation.detailed_performance.to_rewarding_performance();
            debug!("nym-node {node_id}: stake: {total_stake}, performance: {performance}");

            let mut available_roles = Vec::new();
            if self_described.declared_role.mixnode {
                available_roles.push(AvailableRole::Mix)
            }
            if self_described.declared_role.entry {
                available_roles.push(AvailableRole::EntryGateway)
            }
            if self_described.declared_role.can_operate_exit_gateway() {
                available_roles.push(AvailableRole::ExitGateway)
            }

            if available_roles.is_empty() {
                warn!("nym-node {node_id} can't operate under any mode!");
                continue;
            }

            with_performance.push(NodeWithStakeAndPerformance {
                node_id: nym_node.node_id(),
                available_roles,
                total_stake,
                performance,
            })
        }

        with_performance
    }

    pub(super) async fn update_rewarded_set_and_advance_epoch(
        &self,
        nym_nodes: &[NymNodeDetails],
    ) -> Result<(), RewardingError> {
        let epoch_status = self.nyxd_client.get_current_epoch_status().await?;
        match epoch_status.state {
            EpochState::RoleAssignment { next } => {
                // with how the nym-api is currently coded, this should never happen as we're always
                // assigning roles to ALL nodes at once, but who knows what we might decide to do in the future...
                if !next.is_first() {
                    return Err(RewardingError::MidRoleAssignment { next });
                }

                info!("attempting to assign the rewarded set for the upcoming epoch...");
                let nodes_with_performance =
                    self.attach_performance_to_eligible_nodes(nym_nodes).await;

                if let Err(err) = self
                    ._update_rewarded_set_and_advance_epoch(nodes_with_performance)
                    .await
                {
                    error!("FAILED to assign the rewarded set... - {err}");
                    Err(err)
                } else {
                    info!("Advanced the epoch and updated the rewarded set... SUCCESS");
                    Ok(())
                }
            }
            state => {
                // hard error, this shouldn't have happened!
                error!("tried to perform node rewarded set assignment while in {state} state!");
                Err(RewardingError::InvalidEpochState {
                    current_state: state,
                    operation: "assigning rewarded set".to_string(),
                })
            }
        }
    }

    async fn _update_rewarded_set_and_advance_epoch(
        &self,
        all_nodes: Vec<NodeWithStakeAndPerformance>,
    ) -> Result<(), RewardingError> {
        // we grab rewarding parameters here as they might have gotten updated when performing epoch actions
        let rewarding_parameters = self.nyxd_client.get_current_rewarding_parameters().await?;

        debug!("Rewarding parameters: {rewarding_parameters:?}");

        let new_rewarded_set =
            self.determine_rewarded_set(all_nodes, rewarding_parameters.rewarded_set)?;

        debug!("New rewarded set: {:?}", new_rewarded_set);

        self.nyxd_client
            .send_role_assignment_messages(new_rewarded_set)
            .await?;
        Ok(())
    }
}
